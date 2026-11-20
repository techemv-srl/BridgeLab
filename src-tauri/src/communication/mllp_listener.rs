//! Persistent MLLP listener.
//!
//! Unlike `mllp::receive_one` (single-shot, returns after one message), this
//! module owns a long-running task that keeps accepting connections until
//! explicitly stopped. Each received message is emitted to the frontend via
//! the Tauri event bus (`mllp:received`); errors hit `mllp:listen_error`,
//! and `mllp:listen_stopped` says the listener ended on its own.
//!
//! State is held in `ListenerState` and `tauri::Manager`-managed so the
//! frontend can issue start/stop without losing the handle between IPC calls.
//!
//! One peer must not be able to take the listener down or fill the
//! computer's memory, even when the user opened it to the network: the
//! connections served at once, the bytes of messages still arriving and the
//! time one message may take are all bounded (see [`MAX_CONNECTIONS`],
//! [`MAX_BUFFERED_BYTES`], [`MESSAGE_DEADLINE`]), and a failed accept() is
//! retried instead of ending the listener.

use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{oneshot, Mutex};
use tokio::task::{JoinHandle, JoinSet};

use crate::communication::mllp::{
    decode_auto, decode_with_label, encode_with_label, AUTO, MLLP_END_1, MLLP_END_2, MLLP_START,
};

// 10 MiB read cap matches the rest of the parser story.
const MAX_MSG_BYTES: usize = 10 * 1024 * 1024;

/// Connections served at once. A sender uses one connection and an
/// integration test a handful; past this a connection is accepted and
/// closed at once. Without a cap, a peer that opened connections and held
/// them used up the app's open files (about a thousand on Linux, 256 for an
/// app started from the macOS Finder), and accept() then failed for
/// everyone.
const MAX_CONNECTIONS: usize = 64;

/// Bytes of messages still arriving, across every connection at once: room
/// for six messages of the largest size at the same moment. Without it, 300
/// connections each sending 9 MB took the app to 2.4 GB. The message that
/// would go past it is refused with AR.
const MAX_BUFFERED_BYTES: usize = 64 * 1024 * 1024;

/// Longest one message may take to arrive, from its first byte to its last
/// (as long as the read timeout when that is set longer). The read timeout
/// alone starts again with every byte, so a peer sending one byte at a time
/// kept its connection, and the memory of its unfinished message, for ever.
/// Two minutes carries a 10 MB message over a link of under 1 Mbit/s.
const MESSAGE_DEADLINE: Duration = Duration::from_secs(120);

/// Waits before accepting again after accept() failed for want of
/// something (open files, memory): doubling from the first to the last.
const ACCEPT_RETRY_FIRST: Duration = Duration::from_millis(50);
const ACCEPT_RETRY_LAST: Duration = Duration::from_secs(1);

/// A buffer that held a large message is not kept that large once the
/// message is taken out of it: a connection that stays open would hold the
/// memory for as long as it lasts.
const KEPT_CAPACITY: usize = 64 * 1024;

/// The limits above, together, so the tests can use small ones.
#[derive(Debug, Clone, Copy)]
struct Limits {
    connections: usize,
    buffered_bytes: usize,
    message_deadline: Duration,
}

const LIMITS: Limits = Limits {
    connections: MAX_CONNECTIONS,
    buffered_bytes: MAX_BUFFERED_BYTES,
    message_deadline: MESSAGE_DEADLINE,
};

/// User-tunable knobs applied to the listening socket and to every
/// connection it accepts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenerConfig {
    /// TCP port to bind. Default 2575 in the UI.
    pub port: u16,
    /// Bind address. `"127.0.0.1"` (the default) accepts connections from
    /// this computer only; `"0.0.0.0"` from any interface, which the user
    /// has to choose explicitly — a listener must not be reachable from
    /// the hospital network just because someone clicked Start.
    pub bind_address: String,
    /// Whether to send back an HL7 ACK for every received message.
    pub auto_ack: bool,
    /// ACK code to use when auto-ACKing. `"AA"`, `"AE"`, or `"AR"` for
    /// testing how the upstream system handles each ack class.
    pub ack_code: String,
    /// Per-connection read timeout in seconds. Connections that go quiet
    /// past this are dropped — the listener itself stays up. A message
    /// that keeps arriving a byte at a time is dropped at
    /// [`MESSAGE_DEADLINE`] all the same.
    pub read_timeout_secs: u64,
    /// Character encoding label (encoding_rs spelling, e.g. "UTF-8",
    /// "ISO-8859-1", "windows-1252", "windows-1250", "windows-1251").
    /// `"auto"` (the default; an empty or unknown label too) decodes each
    /// message in the charset its MSH-18 declares, else UTF-8, else Latin-1.
    #[serde(default)]
    pub encoding: String,
}

impl Default for ListenerConfig {
    fn default() -> Self {
        Self {
            port: 2575,
            bind_address: "127.0.0.1".into(),
            auto_ack: true,
            ack_code: "AA".into(),
            read_timeout_secs: 30,
            encoding: AUTO.into(),
        }
    }
}

impl ListenerConfig {
    /// The settings as the listener uses them, or why they cannot be. An
    /// empty "Listen on" is this computer only, as the panel says of it
    /// (binding to "" failed); a read timeout of 0 would drop every
    /// connection before its first byte.
    fn checked(mut self) -> Result<Self, String> {
        // An IPv6 address may be typed in brackets, as it is written before
        // a port ("[::1]", "[fe80::1%3]"): they are not part of it, and
        // "[::1]" was bound as the text "[[::1]]:2575".
        let typed = self.bind_address.trim();
        let address = typed.strip_prefix('[').and_then(|a| a.strip_suffix(']'));
        let address = match address {
            Some(inner) if inner.is_empty() || inner.contains(['[', ']']) => None,
            Some(inner) => Some(inner),
            None if typed.contains(['[', ']']) => None,
            None if typed.is_empty() => Some("127.0.0.1"),
            None => Some(typed),
        };
        self.bind_address = match address {
            Some(address) => address.to_string(),
            None => {
                return Err(format!(
                    "Cannot listen on '{}': an IPv6 address goes in one pair of brackets, or in none (for example [::1] or ::1)",
                    typed
                ))
            }
        };
        if self.read_timeout_secs == 0 {
            return Err("The read timeout must be at least 1 second".into());
        }
        Ok(self)
    }

    /// Bind the listening socket. Port 0 asks the system for a free port:
    /// the settings then carry the port it gave, so the panel does not
    /// show ":0" for a listener that answers elsewhere.
    async fn bind(mut self) -> Result<(TcpListener, Self), String> {
        let (bound, shown) = match socket_address(&self.bind_address, self.port) {
            Ok(addr) => (TcpListener::bind(addr).await, addr.to_string()),
            // A host name ("localhost"), resolved by the system.
            Err(text) => (TcpListener::bind(text.as_str()).await, text),
        };
        let listener = bound.map_err(|e| format!("Failed to bind to {}: {}", shown, e))?;
        if let Ok(local) = listener.local_addr() {
            self.port = local.port();
        }
        Ok((listener, self))
    }
}

/// The socket address for listening on `address` (an IP address, with a
/// zone for a link-local IPv6 one: `fe80::1%3`, or `fe80::1%eth0` on Linux
/// and macOS) and `port`; Err with the `host:port` text for anything else,
/// a host name for the system to resolve. Written as text, an IPv6 address
/// needs brackets before the port, and with a zone it parses as neither an
/// address nor a host name: `fe80::1%3:2575` failed to bind.
fn socket_address(address: &str, port: u16) -> Result<SocketAddr, String> {
    use std::net::{IpAddr, Ipv6Addr, SocketAddrV6};
    if let Ok(ip) = address.parse::<IpAddr>() {
        return Ok(SocketAddr::new(ip, port));
    }
    if let Some((ip, zone)) = address.split_once('%') {
        if let (Ok(ip), Some(scope)) = (ip.parse::<Ipv6Addr>(), zone_index(zone)) {
            return Ok(SocketAddr::V6(SocketAddrV6::new(ip, port, 0, scope)));
        }
    }
    // Brackets let the error name the address as it was typed.
    Err(if address.contains(':') { format!("[{}]:{}", address, port) } else { format!("{}:{}", address, port) })
}

/// The interface index an IPv6 zone names: a number (`%3`, as Windows
/// shows it), or on Unix an interface name (`%eth0`, `%en0`).
fn zone_index(zone: &str) -> Option<u32> {
    if let Ok(index) = zone.parse::<u32>() {
        return Some(index);
    }
    #[cfg(unix)]
    {
        let name = std::ffi::CString::new(zone).ok()?;
        // SAFETY: `name` is a valid NUL-terminated string that outlives the call.
        let index = unsafe { libc::if_nametoindex(name.as_ptr()) };
        if index != 0 {
            return Some(index);
        }
    }
    None
}

/// Payload emitted on `mllp:received`.
#[derive(Debug, Clone, Serialize)]
pub struct ReceivedEvent {
    pub content: String,
    pub source_addr: String,
    pub received_at: String,
    /// Payload size after MLLP unframing, in bytes.
    pub bytes: usize,
    /// ACK code sent back ("AA"/"AE"/"AR"), or None when auto-ACK is off.
    pub ack_code: Option<String>,
    /// Charset the payload was decoded with; "(auto)" follows the label
    /// when the listener chose it.
    pub encoding: String,
}

/// Payload emitted on `mllp:connection_idle`: a connection that sent
/// nothing within the read timeout was closed. Not an error: an interface
/// engine may open its connection long before it has a message to send.
#[derive(Debug, Clone, Serialize)]
pub struct IdleEvent {
    pub source_addr: String,
    pub idle_secs: u64,
}

/// Snapshot of the current listener state, returned by status/start/stop so
/// the UI can stay in sync without subscribing to every transition.
#[derive(Debug, Clone, Serialize)]
pub struct ListenerStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub bind_address: Option<String>,
}

#[derive(Default)]
struct Inner {
    handle: Option<JoinHandle<()>>,
    /// Tells the accept loop to stop; it then closes every open connection.
    shutdown: Option<oneshot::Sender<()>>,
    config: Option<ListenerConfig>,
    /// Set by the listener's task when its accept loop ended on its own,
    /// before it tells the frontend: the frontend reads the status then,
    /// while the task is still finishing.
    ended: Option<Arc<AtomicBool>>,
}

/// State managed by Tauri. Wrapped in Arc<Mutex> so the IPC handlers can
/// `.await` on it across calls without holding a sync lock across awaits.
pub struct ListenerState(Arc<Mutex<Inner>>);

impl ListenerState {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Inner::default())))
    }

    pub async fn status(&self) -> ListenerStatus {
        let inner = self.0.lock().await;
        // A listener whose accept loop has ended accepts nothing, whatever
        // handle and settings are still kept: it used to read "running"
        // while every new connection was refused.
        let running = inner.handle.as_ref().is_some_and(|h| !h.is_finished())
            && !inner.ended.as_ref().is_some_and(|e| e.load(Ordering::SeqCst));
        match &inner.config {
            Some(c) if running => ListenerStatus {
                running: true,
                port: Some(c.port),
                bind_address: Some(c.bind_address.clone()),
            },
            _ => ListenerStatus { running: false, port: None, bind_address: None },
        }
    }

    pub async fn start(&self, app: AppHandle, config: ListenerConfig) -> Result<ListenerStatus, String> {
        let config = config.checked()?;
        // Stop any prior listener so port-change / config-change is one IPC
        // call, and so it gives up the port before it is bound again.
        self.stop_internal().await;
        let (listener, config) = config.bind().await?;
        Ok(self.start_on(listener, Arc::new(app), config, LIMITS).await)
    }

    /// Serve `listener` until stopped: [`ListenerState::start`] once the
    /// socket is bound, and what the tests drive with a listener and a
    /// sink of their own.
    async fn start_on<L: Incoming, S: Sink>(
        &self,
        listener: L,
        sink: Arc<S>,
        config: ListenerConfig,
        limits: Limits,
    ) -> ListenerStatus {
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let ended = Arc::new(AtomicBool::new(false));
        let task = {
            let (cfg, ended) = (config.clone(), ended.clone());
            tokio::spawn(async move {
                if let Err(reason) = serve_listener(listener, shutdown_rx, cfg, limits, sink.clone()).await {
                    ended.store(true, Ordering::SeqCst);
                    sink.on_stopped(reason);
                }
            })
        };

        let mut inner = self.0.lock().await;
        inner.handle = Some(task);
        inner.shutdown = Some(shutdown_tx);
        inner.config = Some(config);
        inner.ended = Some(ended);
        drop(inner);
        self.status().await
    }

    pub async fn stop(&self) -> ListenerStatus {
        self.stop_internal().await;
        self.status().await
    }

    /// Stop accepting and close every connection still open: a peer that
    /// keeps its connection must not go on being received and ACKed after
    /// Stop. Returns once the connections are closed.
    async fn stop_internal(&self) {
        let mut inner = self.0.lock().await;
        if let Some(tx) = inner.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(mut handle) = inner.handle.take() {
            if tokio::time::timeout(Duration::from_secs(5), &mut handle).await.is_err() {
                handle.abort();
            }
        }
        inner.config = None;
        inner.ended = None;
    }
}

/// One received message, as a connection hands it on.
struct Received {
    content: String,
    source_addr: String,
    /// Payload size after MLLP unframing, in bytes.
    bytes: usize,
    /// The ACK code written back; None with auto-ACK off.
    ack_code: Option<String>,
    /// The ACK written back, for the History.
    ack_text: String,
    /// The charset label as the console shows it.
    encoding: String,
    /// How long acknowledging took.
    elapsed_ms: u64,
}

/// Where a listener's outcomes go: the app's console, History and tabs, or
/// a recorder in the tests.
trait Sink: Send + Sync + 'static {
    /// A message was received (and acknowledged, with auto-ACK on).
    fn on_message(&self, message: Received);
    /// A red row in the console: a connection that failed or was refused,
    /// a message refused, an accept() that failed and is tried again.
    fn on_error(&self, text: String);
    /// An ordinary row in the console: a connection that sent nothing was
    /// closed after `idle_secs`, the read timeout.
    fn on_idle(&self, source_addr: String, idle_secs: u64);
    /// The listener ended on its own and accepts nothing any more.
    fn on_stopped(&self, reason: String);
}

impl Sink for AppHandle {
    fn on_message(&self, m: Received) {
        use tauri::Manager;
        if let Some(c) = self.try_state::<crate::licensing::telemetry::UsageCounters>() {
            c.bump_mem("mllp_received");
        }
        // Received messages are logged in the history like sends.
        if let Some(db) = self.try_state::<crate::database::Database>() {
            use crate::communication::profiles::{history_text, HistoryEntry};
            let _ = db.add_history_entry(&HistoryEntry {
                id: uuid::Uuid::new_v4().to_string(),
                profile_name: m.source_addr.clone(),
                profile_type: "mllp".into(),
                direction: "receive".into(),
                content_preview: String::new(),
                status: "OK".into(),
                response_time_ms: m.elapsed_ms,
                timestamp: chrono::Utc::now().to_rfc3339(),
                ack_code: m.ack_code.clone(),
                target: m.source_addr.clone(),
                size_bytes: m.bytes as u64,
                request: history_text(&m.content),
                response: history_text(&m.ack_text),
            });
        }
        let _ = self.emit("mllp:received", ReceivedEvent {
            content: m.content,
            source_addr: m.source_addr,
            received_at: chrono::Utc::now().to_rfc3339(),
            bytes: m.bytes,
            ack_code: m.ack_code,
            encoding: m.encoding,
        });
    }

    fn on_error(&self, text: String) {
        let _ = self.emit("mllp:listen_error", text);
    }

    fn on_idle(&self, source_addr: String, idle_secs: u64) {
        let _ = self.emit("mllp:connection_idle", IdleEvent { source_addr, idle_secs });
    }

    fn on_stopped(&self, reason: String) {
        let _ = self.emit("mllp:listen_stopped", reason);
    }
}

/// Where the accept loop takes its connections from: the listening socket,
/// or in the tests one that fails when told to.
trait Incoming: Send + Sync + 'static {
    fn accept(&self) -> impl Future<Output = io::Result<(TcpStream, SocketAddr)>> + Send;
}

impl Incoming for TcpListener {
    fn accept(&self) -> impl Future<Output = io::Result<(TcpStream, SocketAddr)>> + Send {
        TcpListener::accept(self)
    }
}

/// Accept connections on `listener` and serve each in a task of its own,
/// with `limits`, until `shutdown` fires. Err when accept() failed in a way
/// the listener cannot get past.
async fn serve_listener<L: Incoming, S: Sink>(
    listener: L,
    shutdown: oneshot::Receiver<()>,
    cfg: ListenerConfig,
    limits: Limits,
    sink: Arc<S>,
) -> Result<(), String> {
    let budget = Arc::new(Budget::new(limits.buffered_bytes));
    let report = {
        let sink = sink.clone();
        move |text: String| sink.on_error(text)
    };
    accept_loop(listener, shutdown, limits.connections, report, move |stream, peer| {
        let (cfg, budget, sink) = (cfg.clone(), budget.clone(), sink.clone());
        async move {
            if let Err(e) = handle_connection(stream, peer, &cfg, limits, Share::new(budget), &*sink).await {
                sink.on_error(e);
            }
        }
    })
    .await
}

/// What the accept loop does after accept() failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AcceptFailure {
    /// One connection went wrong before it was accepted (the peer reset
    /// it): accept the next one at once.
    Connection,
    /// The system is short of something, open files most often: wait a
    /// little and accept again. Connections that end give back what is
    /// missing.
    Resources,
    /// The listening socket itself cannot accept any more.
    Fatal,
}

/// accept() errors that mean the listening socket itself is unusable.
#[cfg(unix)]
const FATAL_ACCEPT_ERRORS: &[i32] = &[libc::EBADF, libc::ENOTSOCK, libc::EINVAL];
/// WSAEBADF, WSAEINVAL, WSAENOTSOCK, WSAEOPNOTSUPP and WSANOTINITIALISED.
#[cfg(windows)]
const FATAL_ACCEPT_ERRORS: &[i32] = &[10009, 10022, 10038, 10045, 10093];
#[cfg(not(any(unix, windows)))]
const FATAL_ACCEPT_ERRORS: &[i32] = &[];

fn accept_failure(e: &io::Error) -> AcceptFailure {
    if e.raw_os_error().is_some_and(|code| FATAL_ACCEPT_ERRORS.contains(&code)) {
        return AcceptFailure::Fatal;
    }
    use io::ErrorKind::*;
    match e.kind() {
        // Linux also hands a pending network error of the new connection
        // to accept(), to be treated as "try again".
        ConnectionAborted | ConnectionReset | ConnectionRefused | Interrupted | WouldBlock | TimedOut
        | NetworkDown | NetworkUnreachable | HostUnreachable => AcceptFailure::Connection,
        // Anything else, "too many open files" and "out of memory" among
        // them, can pass: ending the listener over it turned one bad moment
        // into a listener gone for good while the app said "Listening".
        _ => AcceptFailure::Resources,
    }
}

/// Accept connections, each served by `serve` in a task of its own, until
/// `shutdown` fires (or its sender is dropped); then abort every
/// connection still open and wait for them to end. At most
/// `max_connections` are served at once: a further one is accepted and
/// closed at once. A failed accept() is reported through `report` and
/// retried, unless the socket itself is unusable: that ends the loop the
/// same way and is returned.
async fn accept_loop<L, R, F, Fut>(
    listener: L,
    mut shutdown: oneshot::Receiver<()>,
    max_connections: usize,
    report: R,
    serve: F,
) -> Result<(), String>
where
    L: Incoming,
    R: Fn(String),
    F: Fn(TcpStream, String) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    let mut connections = JoinSet::new();
    // One console row per spell of trouble, not one per refused connection
    // or failed accept(): a flood would push every other row out of the
    // console. The cap is reported again once the connections have fallen
    // to half of it, a failed accept() once a connection was accepted.
    let mut full = false;
    let mut failing: Option<String> = None;
    let mut retry_in = ACCEPT_RETRY_FIRST;
    let mut pause: Option<Duration> = None;
    let result = loop {
        if let Some(wait) = pause.take() {
            tokio::select! {
                _ = &mut shutdown => break Ok(()),
                _ = tokio::time::sleep(wait) => {}
            }
        }
        tokio::select! {
            _ = &mut shutdown => break Ok(()),
            accepted = listener.accept() => match accepted {
                Ok((stream, peer)) => {
                    failing = None;
                    retry_in = ACCEPT_RETRY_FIRST;
                    // Connections that ended but were not reaped yet do not count.
                    while connections.try_join_next().is_some() {}
                    if connections.len() <= max_connections / 2 {
                        full = false;
                    }
                    if connections.len() >= max_connections {
                        drop(stream);
                        if !full {
                            full = true;
                            report(format!(
                                "connection from {} closed: {} connections are open, the most the listener serves at once; further ones are closed until some end",
                                peer, max_connections
                            ));
                        }
                    } else {
                        connections.spawn(serve(stream, peer.to_string()));
                    }
                }
                Err(e) => match accept_failure(&e) {
                    AcceptFailure::Fatal => break Err(format!("accept failed: {}", e)),
                    failure => {
                        let text = e.to_string();
                        if failing.as_deref() != Some(text.as_str()) {
                            report(format!("accept failed: {}; the listener goes on and tries again", text));
                            failing = Some(text);
                        }
                        if failure == AcceptFailure::Resources {
                            pause = Some(retry_in);
                            retry_in = (retry_in * 2).min(ACCEPT_RETRY_LAST);
                        }
                    }
                },
            },
            // Reap finished connections so the set does not grow.
            Some(_) = connections.join_next(), if !connections.is_empty() => {}
        }
    };
    drop(listener);
    connections.shutdown().await;
    result
}

/// Bytes of messages still arriving, held by all of a listener's
/// connections together, against a limit.
struct Budget {
    used: AtomicUsize,
    limit: usize,
}

impl Budget {
    fn new(limit: usize) -> Self {
        Self { used: AtomicUsize::new(0), limit }
    }

    /// Take `n` more bytes, if they fit.
    fn take(&self, n: usize) -> bool {
        self.used
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |used| {
                used.checked_add(n).filter(|&total| total <= self.limit)
            })
            .is_ok()
    }

    fn give_back(&self, n: usize) {
        self.used.fetch_sub(n, Ordering::SeqCst);
    }
}

/// One connection's part of the [`Budget`], given back when the connection
/// ends, however it ends (Stop aborts it).
struct Share {
    budget: Arc<Budget>,
    held: usize,
}

impl Share {
    fn new(budget: Arc<Budget>) -> Self {
        Self { budget, held: 0 }
    }

    /// Hold `n` bytes in all. False, still holding what it held, when the
    /// budget cannot cover them.
    fn resize(&mut self, n: usize) -> bool {
        if n > self.held {
            if !self.budget.take(n - self.held) {
                return false;
            }
        } else {
            self.budget.give_back(self.held - n);
        }
        self.held = n;
        true
    }
}

impl Drop for Share {
    fn drop(&mut self) {
        self.budget.give_back(self.held);
    }
}

/// What one MLLP frame holds, once taken out of the byte stream.
#[derive(Debug, PartialEq)]
enum Frame {
    /// The bytes between VT and FS CR.
    Message(Vec<u8>),
    /// A frame that grew past [`MAX_MSG_BYTES`] without its end: the
    /// first bytes are kept so the reply can name the control ID.
    TooLarge(Vec<u8>),
}

/// The bytes a connection has read and not yet taken out as frames.
#[derive(Default)]
struct FrameBuffer {
    buf: Vec<u8>,
    /// Bytes dropped because they were not inside a frame.
    junk: usize,
    /// How much of the frame being received has been searched for its
    /// end, so each read searches only the bytes it added: searching the
    /// whole frame again on every 8 KB read made one 9 MB message cost
    /// seconds of CPU.
    scanned: usize,
    /// Bytes searched for a frame end so far, for the test that keeps the
    /// search linear.
    #[cfg(test)]
    searched: usize,
}

impl FrameBuffer {
    fn extend(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    fn len(&self) -> usize {
        self.buf.len()
    }

    fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// The first bytes of the frame being received (after its VT), for the
    /// ACK that refuses it.
    fn head(&self) -> &[u8] {
        let start = memchr::memchr(MLLP_START, &self.buf).map_or(self.buf.len(), |i| i + 1);
        &self.buf[start..self.buf.len().min(start + 4095)]
    }

    fn reset(&mut self) {
        self.buf = Vec::new();
        self.scanned = 0;
    }

    /// Take every complete frame out, leaving a partial one in place.
    /// Bytes before a VT are not MLLP and are dropped (their count is added
    /// to `junk`).
    fn take(&mut self) -> Vec<Frame> {
        let mut out = Vec::new();
        loop {
            if self.scanned == 0 {
                let Some(start) = memchr::memchr(MLLP_START, &self.buf) else {
                    self.junk += self.buf.len();
                    self.buf.clear();
                    break;
                };
                if start > 0 {
                    self.junk += start;
                    self.buf.drain(..start);
                }
                self.scanned = 1;
            }
            // One byte back: the FS may have ended the last read and its CR
            // begun this one. The VT at 0 is never part of the end.
            let from = self.scanned.saturating_sub(1).max(1);
            #[cfg(test)]
            {
                self.searched += self.buf.len() - from;
            }
            let end = memchr::memmem::find(&self.buf[from..], &[MLLP_END_1, MLLP_END_2]).map(|i| from + i);
            match end {
                // The whole frame arrived in one read, and it is too large:
                // refused like one still streaming.
                Some(end) if end - 1 > MAX_MSG_BYTES => {
                    out.push(Frame::TooLarge(self.head().to_vec()));
                    self.reset();
                    break;
                }
                Some(end) => {
                    out.push(Frame::Message(self.buf[1..end].to_vec()));
                    self.buf.drain(..end + 2);
                    self.scanned = 0;
                }
                None if self.buf.len() > MAX_MSG_BYTES => {
                    out.push(Frame::TooLarge(self.head().to_vec()));
                    self.reset();
                    break;
                }
                None => {
                    self.scanned = self.buf.len();
                    break;
                }
            }
        }
        if self.buf.is_empty() && self.buf.capacity() > KEPT_CAPACITY {
            self.buf = Vec::new();
        }
        out
    }
}

/// Serve one connection: every framed message on it is received and
/// acknowledged in turn, until the peer closes it or it stays idle for the
/// read timeout. Interface engines keep MLLP connections open; the
/// listener used to take one message and hang up. What is still arriving
/// is held in `share` of the budget of every connection, and one message
/// may take no longer than the deadline of `limits` to arrive.
async fn handle_connection<S: Sink>(
    mut stream: TcpStream,
    source_addr: String,
    cfg: &ListenerConfig,
    limits: Limits,
    mut share: Share,
    sink: &S,
) -> Result<(), String> {
    let mut frames = FrameBuffer::default();
    let mut chunk = [0u8; 8 * 1024];
    let mut received = 0usize;
    let read_timeout = Duration::from_secs(cfg.read_timeout_secs);
    let deadline = limits.message_deadline.max(read_timeout);
    // When the bytes not yet part of a received message began to arrive: a
    // frame in progress, or bytes that are not MLLP at all.
    let mut arriving_since: Option<Instant> = None;
    // Closed by us after the read timeout, with no message on its way.
    let mut went_idle = false;

    loop {
        let wait = match arriving_since {
            Some(since) => match deadline.checked_sub(since.elapsed()) {
                Some(left) if !left.is_zero() => left.min(read_timeout),
                _ => {
                    let _ = stream.shutdown().await;
                    return Err(format!(
                        "connection from {} dropped: no complete message within {}s of its first byte",
                        source_addr,
                        deadline.as_secs()
                    ));
                }
            },
            None => read_timeout,
        };
        let n = match tokio::time::timeout(wait, stream.read(&mut chunk)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(format!("read failed: {}", e)),
            // The deadline came before the read timeout: said above.
            Err(_) if wait < read_timeout => continue,
            // Quiet with no message on its way is normal, before the first
            // message as between two: an engine may open its connection
            // long before it has anything to send. The connection is
            // closed to free its place, and that is no error.
            Err(_) if frames.is_empty() => {
                went_idle = true;
                break;
            }
            // Quiet inside a message is.
            Err(_) => {
                let _ = stream.shutdown().await;
                return Err(format!(
                    "connection from {} dropped: nothing more for {}s in the middle of a message",
                    source_addr, cfg.read_timeout_secs
                ));
            }
        };
        arriving_since.get_or_insert_with(Instant::now);
        frames.extend(&chunk[..n]);
        let mut completed = false;
        for frame in frames.take() {
            match frame {
                Frame::Message(bytes) => {
                    received += 1;
                    completed = true;
                    handle_message(&mut stream, &bytes, &source_addr, cfg, sink).await;
                }
                Frame::TooLarge(head) => {
                    // Refused, not truncated: an AA for the first 10 MiB
                    // told the sender a message it lost part of had
                    // arrived. The stream cannot be resynchronised, so
                    // the connection ends here.
                    let limit = MAX_MSG_BYTES / (1024 * 1024);
                    refuse(&mut stream, &head, cfg, &format!("Message larger than {} MiB refused", limit)).await;
                    return Err(format!(
                        "message from {} refused: larger than {} MiB (answered AR)",
                        source_addr, limit
                    ));
                }
            }
        }
        if completed {
            arriving_since = (!frames.is_empty()).then(Instant::now);
        }
        // What is still arriving counts against the budget of every
        // connection; the messages just handled are given back only now.
        if !share.resize(frames.len()) {
            refuse(&mut stream, frames.head(), cfg, "Message refused: the listener is receiving too much at once").await;
            return Err(format!(
                "message from {} refused: the messages arriving on all connections together passed {} MiB (answered AR)",
                source_addr,
                limits.buffered_bytes / (1024 * 1024)
            ));
        }
    }

    let _ = stream.shutdown().await;
    if frames.junk > 0 && received == 0 {
        return Err(format!(
            "{} sent {} bytes that are not an MLLP-framed message (no VT … FS CR); nothing was received",
            source_addr, frames.junk
        ));
    }
    if !frames.is_empty() {
        return Err(format!("connection from {} closed in the middle of a message", source_addr));
    }
    // One that never sent anything gets a row of its own, so a sender that
    // connects and waits is seen; after messages, their rows say enough.
    if went_idle && received == 0 {
        sink.on_idle(source_addr, cfg.read_timeout_secs);
    }
    Ok(())
}

/// Answer AR (with auto-ACK on) for the message `head` begins, and close
/// the connection: what follows on it can no longer be told apart into
/// messages.
async fn refuse(stream: &mut TcpStream, head: &[u8], cfg: &ListenerConfig, text: &str) {
    let (content, label) = decode(head, cfg);
    if cfg.auto_ack {
        let _ = write_ack(stream, &content, "AR", Some(text), &label).await;
    }
    let _ = stream.shutdown().await;
}

/// Acknowledge one framed message and hand it to the console.
async fn handle_message<S: Sink>(stream: &mut TcpStream, bytes: &[u8], source_addr: &str, cfg: &ListenerConfig, sink: &S) {
    let (content, label) = decode(bytes, cfg);
    // A UTF-8 byte-order mark is not part of the message, and blank lines
    // before the header are not a reason to refuse it.
    let content = content.trim_start_matches('\u{FEFF}').to_string();
    let is_hl7 = matches!(content.trim_start().get(..3), Some("MSH" | "FHS" | "BHS"));

    let started = Instant::now();
    let mut ack_sent: Option<String> = None;
    let mut ack_text = String::new();
    if cfg.auto_ack {
        // Something that is not HL7 gets AR whatever code is configured:
        // accepting it would tell the sender a message was delivered.
        let (code, text) = if is_hl7 { (cfg.ack_code.as_str(), None) } else { ("AR", Some("Not an HL7 message")) };
        if let Ok(ack) = write_ack(stream, &content, code, text, &label).await {
            ack_sent = Some(code.to_string());
            ack_text = ack;
        }
    }
    if !is_hl7 {
        sink.on_error(format!("{} sent a frame that is not an HL7 message (answered AR)", source_addr));
    }

    sink.on_message(Received {
        content,
        source_addr: source_addr.to_string(),
        bytes: bytes.len(),
        ack_code: ack_sent,
        ack_text,
        encoding: if is_auto(&cfg.encoding) { format!("{} (auto)", label) } else { label },
        elapsed_ms: started.elapsed().as_millis() as u64,
    });
}

fn is_auto(label: &str) -> bool {
    label.trim().is_empty() || label.trim().eq_ignore_ascii_case(AUTO)
}

/// The text of a received frame, and the charset label it was decoded
/// with: the configured one, or in automatic mode the one the frame
/// chose (MSH-18, else UTF-8, else Latin-1).
fn decode(bytes: &[u8], cfg: &ListenerConfig) -> (String, String) {
    if is_auto(&cfg.encoding) || !crate::parser::hl7::charset::is_known_label(&cfg.encoding) {
        decode_auto(bytes)
    } else {
        (decode_with_label(bytes, &cfg.encoding), cfg.encoding.trim().to_string())
    }
}

async fn write_ack(
    stream: &mut TcpStream,
    content: &str,
    code: &str,
    text: Option<&str>,
    encoding: &str,
) -> std::io::Result<String> {
    let ack_msg = crate::parser::hl7::ack::ack_for(content, code, text);
    // Re-encoded with the same charset so the peer doesn't see mojibake.
    let payload = encode_with_label(&ack_msg, encoding);
    let mut framed = Vec::with_capacity(payload.len() + 3);
    framed.push(MLLP_START);
    framed.extend_from_slice(&payload);
    framed.push(MLLP_END_1);
    framed.push(MLLP_END_2);
    stream.write_all(&framed).await?;
    Ok(ack_msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    /// A listener started with the defaults must not be reachable from
    /// other machines; the UI and the manual promise loopback.
    #[test]
    fn default_bind_is_loopback() {
        assert_eq!(ListenerConfig::default().bind_address, "127.0.0.1");
    }

    fn framed(msg: &str) -> Vec<u8> {
        let mut v = vec![MLLP_START];
        v.extend_from_slice(msg.as_bytes());
        v.extend_from_slice(&[MLLP_END_1, MLLP_END_2]);
        v
    }

    fn hl7(control_id: &str) -> String {
        format!("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|{}|P|2.5\rPID|1\r", control_id)
    }

    /// What a listener under test told the app.
    #[derive(Default)]
    struct Recorder {
        messages: StdMutex<Vec<String>>,
        errors: StdMutex<Vec<String>>,
        idle: StdMutex<Vec<(String, u64)>>,
        stopped: StdMutex<Option<String>>,
    }

    impl Recorder {
        fn errors(&self) -> Vec<String> {
            self.errors.lock().unwrap().clone()
        }
        fn messages(&self) -> usize {
            self.messages.lock().unwrap().len()
        }
    }

    impl Sink for Recorder {
        fn on_message(&self, m: Received) {
            self.messages.lock().unwrap().push(m.content);
        }
        fn on_error(&self, text: String) {
            self.errors.lock().unwrap().push(text);
        }
        fn on_idle(&self, source_addr: String, idle_secs: u64) {
            self.idle.lock().unwrap().push((source_addr, idle_secs));
        }
        fn on_stopped(&self, reason: String) {
            *self.stopped.lock().unwrap() = Some(reason);
        }
    }

    /// A listening socket whose accept() fails as told: `failures` times
    /// with `error()` and then normally, or for good once `fatal` is
    /// notified.
    struct Flaky {
        inner: TcpListener,
        failures: AtomicUsize,
        error: fn() -> io::Error,
        fatal: Arc<tokio::sync::Notify>,
    }

    impl Flaky {
        async fn new(failures: usize, error: fn() -> io::Error) -> Self {
            Self {
                inner: TcpListener::bind("127.0.0.1:0").await.unwrap(),
                failures: AtomicUsize::new(failures),
                error,
                fatal: Arc::new(tokio::sync::Notify::new()),
            }
        }
        fn addr(&self) -> SocketAddr {
            self.inner.local_addr().unwrap()
        }
    }

    impl Incoming for Flaky {
        fn accept(&self) -> impl Future<Output = io::Result<(TcpStream, SocketAddr)>> + Send {
            async move {
                if self.failures.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1)).is_ok() {
                    return Err((self.error)());
                }
                tokio::select! {
                    _ = self.fatal.notified() => Err(bad_socket()),
                    accepted = self.inner.accept() => accepted,
                }
            }
        }
    }

    #[cfg(unix)]
    fn too_many_open_files() -> io::Error {
        io::Error::from_raw_os_error(libc::EMFILE)
    }
    #[cfg(windows)]
    fn too_many_open_files() -> io::Error {
        io::Error::from_raw_os_error(10024) // WSAEMFILE
    }
    #[cfg(unix)]
    fn bad_socket() -> io::Error {
        io::Error::from_raw_os_error(libc::EBADF)
    }
    #[cfg(windows)]
    fn bad_socket() -> io::Error {
        io::Error::from_raw_os_error(10038) // WSAENOTSOCK
    }
    fn aborted() -> io::Error {
        io::Error::from(io::ErrorKind::ConnectionAborted)
    }

    const TEST_LIMITS: Limits = Limits { connections: 64, buffered_bytes: 1 << 20, message_deadline: Duration::from_secs(60) };

    fn test_config() -> ListenerConfig {
        ListenerConfig { read_timeout_secs: 5, ..ListenerConfig::default() }
    }

    /// A listener on `incoming` with the app's connection handling and
    /// `limits`, reporting to the returned recorder.
    fn spawn_listener<L: Incoming>(
        incoming: L,
        cfg: ListenerConfig,
        limits: Limits,
    ) -> (Arc<Recorder>, oneshot::Sender<()>, JoinHandle<Result<(), String>>) {
        let sink = Arc::new(Recorder::default());
        let (tx, rx) = oneshot::channel();
        let task = tokio::spawn(serve_listener(incoming, rx, cfg, limits, sink.clone()));
        (sink, tx, task)
    }

    /// Read one MLLP frame (an ACK) from `stream`.
    async fn read_frame(stream: &mut TcpStream) -> String {
        let mut got = Vec::new();
        let mut b = [0u8; 4096];
        tokio::time::timeout(Duration::from_secs(5), async {
            while !got.ends_with(&[MLLP_END_1, MLLP_END_2]) {
                let n = stream.read(&mut b).await.unwrap();
                assert!(n > 0, "closed before a whole frame: {:?}", String::from_utf8_lossy(&got));
                got.extend_from_slice(&b[..n]);
            }
        })
        .await
        .expect("no frame in time");
        String::from_utf8_lossy(&got).into_owned()
    }

    /// Whether the server closed `stream` (a read gives end of file or an
    /// error) within `within`.
    async fn closed_within(stream: &mut TcpStream, within: Duration) -> bool {
        let mut b = [0u8; 256];
        match tokio::time::timeout(within, stream.read(&mut b)).await {
            Ok(Ok(0)) | Ok(Err(_)) => true,
            Ok(Ok(_)) | Err(_) => false,
        }
    }

    async fn wait_until(what: &str, mut ready: impl FnMut() -> bool) {
        let give_up = Instant::now() + Duration::from_secs(10);
        while !ready() {
            assert!(Instant::now() < give_up, "timed out waiting for {what}");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    /// Stop closes the connections that are still open: a peer that keeps
    /// its connection cannot go on sending after the listener stopped.
    #[tokio::test]
    async fn stopping_closes_open_connections() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = oneshot::channel();
        // Echo each read back until the peer closes.
        let serve = |mut stream: TcpStream, _peer: String| async move {
            let mut b = [0u8; 64];
            while let Ok(n) = stream.read(&mut b).await {
                if n == 0 || stream.write_all(&b[..n]).await.is_err() {
                    break;
                }
            }
        };
        let task = tokio::spawn(accept_loop(listener, rx, 64, |_| {}, serve));

        let mut client = TcpStream::connect(addr).await.unwrap();
        client.write_all(b"one").await.unwrap();
        let mut b = [0u8; 8];
        let n = client.read(&mut b).await.unwrap();
        assert_eq!(&b[..n], b"one");

        tx.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), task).await.unwrap().unwrap().unwrap();
        // The connection is gone: the next exchange gets no echo.
        let _ = client.write_all(b"two").await;
        let read = tokio::time::timeout(Duration::from_secs(5), client.read(&mut b)).await.unwrap();
        assert!(matches!(read, Ok(0) | Err(_)), "{:?}", read);
        assert!(TcpStream::connect(addr).await.is_err(), "no new connections either");
    }

    /// Only an error of the listening socket itself ends the loop: running
    /// out of open files or memory passes, and so does a peer that reset
    /// its connection before it was accepted.
    #[test]
    fn only_an_unusable_socket_stops_accepting() {
        assert_eq!(accept_failure(&too_many_open_files()), AcceptFailure::Resources);
        assert_eq!(accept_failure(&io::Error::from(io::ErrorKind::OutOfMemory)), AcceptFailure::Resources);
        assert_eq!(accept_failure(&io::Error::other("something new")), AcceptFailure::Resources);
        assert_eq!(accept_failure(&aborted()), AcceptFailure::Connection);
        assert_eq!(accept_failure(&io::Error::from(io::ErrorKind::ConnectionReset)), AcceptFailure::Connection);
        assert_eq!(accept_failure(&bad_socket()), AcceptFailure::Fatal);
        #[cfg(unix)]
        for code in [libc::ENOTSOCK, libc::EINVAL] {
            assert_eq!(accept_failure(&io::Error::from_raw_os_error(code)), AcceptFailure::Fatal, "{code}");
        }
        #[cfg(unix)]
        for code in [libc::ENFILE, libc::ENOBUFS, libc::ENOMEM, libc::EPROTO, libc::EOPNOTSUPP] {
            assert_ne!(accept_failure(&io::Error::from_raw_os_error(code)), AcceptFailure::Fatal, "{code}");
        }
    }

    /// "Too many open files" used to end the accept loop and close every
    /// connection: the listener now says so once, waits a little and goes
    /// on accepting.
    #[tokio::test]
    async fn the_listener_survives_failed_accepts() {
        let incoming = Flaky::new(4, too_many_open_files).await;
        let addr = incoming.addr();
        let (sink, tx, task) = spawn_listener(incoming, test_config(), TEST_LIMITS);

        let mut client = TcpStream::connect(addr).await.unwrap();
        client.write_all(&framed(&hl7("AFTER1"))).await.unwrap();
        let ack = read_frame(&mut client).await;
        assert!(ack.contains("MSA|AA|AFTER1"), "{ack}");
        assert!(!task.is_finished());
        let errors = sink.errors();
        assert_eq!(errors.len(), 1, "one row for the spell, not one per attempt: {errors:?}");
        assert!(errors[0].contains("accept failed") && errors[0].contains("tries again"), "{errors:?}");

        tx.send(()).unwrap();
        assert_eq!(task.await.unwrap(), Ok(()));
    }

    #[tokio::test]
    async fn a_connection_reset_before_accept_is_skipped_at_once() {
        let incoming = Flaky::new(3, aborted).await;
        let addr = incoming.addr();
        let (sink, tx, task) = spawn_listener(incoming, test_config(), TEST_LIMITS);
        let mut client = TcpStream::connect(addr).await.unwrap();
        client.write_all(&framed(&hl7("AB1"))).await.unwrap();
        assert!(read_frame(&mut client).await.contains("MSA|AA|AB1"));
        assert_eq!(sink.errors().len(), 1, "{:?}", sink.errors());
        tx.send(()).unwrap();
        assert_eq!(task.await.unwrap(), Ok(()));
    }

    /// Past the cap a connection is accepted and closed at once, with one
    /// console row however many follow; once one ends, the next is served.
    #[tokio::test]
    async fn connections_past_the_cap_are_closed_at_once() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let limits = Limits { connections: 2, ..TEST_LIMITS };
        let (sink, tx, task) = spawn_listener(listener, test_config(), limits);

        let mut first = TcpStream::connect(addr).await.unwrap();
        let mut second = TcpStream::connect(addr).await.unwrap();
        // Both are served: each gets its ACK.
        first.write_all(&framed(&hl7("C1"))).await.unwrap();
        assert!(read_frame(&mut first).await.contains("MSA|AA|C1"));
        second.write_all(&framed(&hl7("C2"))).await.unwrap();
        assert!(read_frame(&mut second).await.contains("MSA|AA|C2"));

        for _ in 0..3 {
            let mut extra = TcpStream::connect(addr).await.unwrap();
            assert!(closed_within(&mut extra, Duration::from_secs(5)).await, "a connection past the cap stays open");
        }
        let full: Vec<String> = sink.errors().into_iter().filter(|e| e.contains("the most the listener serves")).collect();
        assert_eq!(full.len(), 1, "{:?}", sink.errors());

        // One ends: a new connection is served again (once the listener has
        // seen the first one close, hence the retries).
        drop(first);
        let give_up = Instant::now() + Duration::from_secs(10);
        loop {
            assert!(Instant::now() < give_up, "no connection served after one ended");
            let mut c = TcpStream::connect(addr).await.unwrap();
            let _ = c.write_all(&framed(&hl7("C3"))).await;
            let mut got = Vec::new();
            let mut b = [0u8; 1024];
            while let Ok(Ok(n)) = tokio::time::timeout(Duration::from_secs(2), c.read(&mut b)).await {
                if n == 0 {
                    break;
                }
                got.extend_from_slice(&b[..n]);
                if got.ends_with(&[MLLP_END_1, MLLP_END_2]) {
                    break;
                }
            }
            if String::from_utf8_lossy(&got).contains("MSA|AA|C3") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        drop(second);
        tx.send(()).unwrap();
        assert_eq!(task.await.unwrap(), Ok(()));
    }

    /// A peer that sends a byte now and then, each well within the read
    /// timeout, is dropped when its message has taken longer than the
    /// deadline: the read timeout alone let it hold the connection and its
    /// buffer for ever.
    #[tokio::test]
    async fn a_trickling_peer_is_dropped_at_the_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let limits = Limits { message_deadline: Duration::from_millis(1500), ..TEST_LIMITS };
        let cfg = ListenerConfig { read_timeout_secs: 1, ..ListenerConfig::default() };
        let (sink, tx, task) = spawn_listener(listener, cfg, limits);

        let mut client = TcpStream::connect(addr).await.unwrap();
        client.write_all(&[MLLP_START]).await.unwrap();
        let started = Instant::now();
        let mut dropped = false;
        for &b in hl7("SLOW").as_bytes().iter().cycle().take(40) {
            tokio::time::sleep(Duration::from_millis(200)).await;
            if client.write_all(&[b]).await.is_err() {
                dropped = true;
                break;
            }
            if !sink.errors().is_empty() {
                dropped = true;
                break;
            }
        }
        assert!(dropped, "still connected after {:?}", started.elapsed());
        assert!(started.elapsed() < Duration::from_secs(4), "{:?}", started.elapsed());
        wait_until("the console row", || !sink.errors().is_empty()).await;
        let errors = sink.errors();
        assert!(errors[0].contains("no complete message within 1s"), "{errors:?}");
        assert!(closed_within(&mut client, Duration::from_secs(5)).await);
        assert_eq!(sink.messages(), 0);
        tx.send(()).unwrap();
        assert_eq!(task.await.unwrap(), Ok(()));
    }

    /// A connection that opens and sends nothing is closed after the read
    /// timeout, to free its place, as an ordinary event: it used to be a
    /// red "idle" row although the sender had done nothing wrong. After a
    /// message it closes quietly, as before.
    #[tokio::test]
    async fn a_connection_that_waits_is_closed_without_an_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let cfg = ListenerConfig { read_timeout_secs: 1, ..ListenerConfig::default() };
        let (sink, tx, task) = spawn_listener(listener, cfg, TEST_LIMITS);

        let mut waiting = TcpStream::connect(addr).await.unwrap();
        let local = waiting.local_addr().unwrap().to_string();
        assert!(closed_within(&mut waiting, Duration::from_secs(5)).await);
        wait_until("the idle row", || !sink.idle.lock().unwrap().is_empty()).await;
        assert_eq!(sink.idle.lock().unwrap().clone(), vec![(local, 1)]);

        let mut sender = TcpStream::connect(addr).await.unwrap();
        sender.write_all(&framed(&hl7("ONE"))).await.unwrap();
        assert!(read_frame(&mut sender).await.contains("MSA|AA|ONE"));
        assert!(closed_within(&mut sender, Duration::from_secs(5)).await);
        // A message cut off by the same quiet is still an error.
        let mut cut = TcpStream::connect(addr).await.unwrap();
        cut.write_all(&framed(&hl7("CUT"))[..20]).await.unwrap();
        assert!(closed_within(&mut cut, Duration::from_secs(5)).await);
        wait_until("the error row", || !sink.errors().is_empty()).await;
        assert!(sink.errors()[0].contains("in the middle of a message"), "{:?}", sink.errors());
        assert_eq!(sink.errors().len(), 1);
        assert_eq!(sink.idle.lock().unwrap().len(), 1);
        tx.send(()).unwrap();
        assert_eq!(task.await.unwrap(), Ok(()));
    }

    /// The settings as the panel can send them: an empty address is this
    /// computer only, a read timeout of 0 is refused, and port 0 is bound
    /// to a real port that the status then shows.
    #[tokio::test]
    async fn listener_settings_are_checked_and_port_0_reports_its_port() {
        let cfg = ListenerConfig { bind_address: "  ".into(), port: 0, ..ListenerConfig::default() }.checked().unwrap();
        assert_eq!(cfg.bind_address, "127.0.0.1");
        let (listener, cfg) = cfg.bind().await.unwrap();
        assert_ne!(cfg.port, 0);
        assert_eq!(listener.local_addr().unwrap().port(), cfg.port);
        assert!(ListenerConfig { read_timeout_secs: 0, ..ListenerConfig::default() }.checked().is_err());
        // An IPv6 address is bound in brackets; skipped where IPv6 is off.
        if std::net::TcpListener::bind("[::1]:0").is_ok() {
            let cfg = ListenerConfig { bind_address: "::1".into(), port: 0, ..ListenerConfig::default() };
            assert!(cfg.checked().unwrap().bind().await.is_ok());
        }
    }

    /// An IPv6 address typed in brackets is read like the same address
    /// without them; brackets that do not pair up are refused, saying why.
    #[tokio::test]
    async fn an_ipv6_address_in_brackets_is_read_without_them() {
        let checked = |address: &str| ListenerConfig { bind_address: address.into(), port: 0, ..ListenerConfig::default() }.checked();
        let mut same = vec![("[::1]", "::1"), ("[fe80::1%3]", "fe80::1%3"), (" [::1] ", "::1")];
        if cfg!(target_os = "linux") {
            same.push(("[fe80::1%lo]", "fe80::1%lo"));
        }
        for (bracketed, plain) in same {
            let cfg = checked(bracketed).unwrap();
            assert_eq!(cfg.bind_address, plain, "{bracketed}");
            assert_eq!(socket_address(&cfg.bind_address, 2575), socket_address(plain, 2575), "{bracketed}");
            assert!(socket_address(&cfg.bind_address, 2575).is_ok(), "{bracketed}");
        }
        // And it binds, where IPv6 is on.
        if std::net::TcpListener::bind("[::1]:0").is_ok() {
            assert!(checked("[::1]").unwrap().bind().await.is_ok());
        }
        for bad in ["[[::1]]", "[::1", "::1]", "[]"] {
            let error = checked(bad).err().unwrap_or_default();
            assert!(error.contains(&format!("'{bad}'")) && error.contains("one pair of brackets"), "{bad}: {error}");
        }
    }

    /// A link-local IPv6 address with its zone is an address, not a host
    /// name: `fe80::1%3:2575` was what the listener tried to bind.
    #[test]
    fn a_scoped_ipv6_address_keeps_its_zone() {
        let addr = socket_address("fe80::1%3", 2575).unwrap();
        assert_eq!(addr, "[fe80::1%3]:2575".parse::<SocketAddr>().unwrap());
        match addr {
            SocketAddr::V6(v6) => assert_eq!(v6.scope_id(), 3),
            other => panic!("{other}"),
        }
        assert_eq!(socket_address("::1", 2575).unwrap().to_string(), "[::1]:2575");
        assert_eq!(socket_address("127.0.0.1", 2575).unwrap().to_string(), "127.0.0.1:2575");
        assert_eq!(socket_address("localhost", 2575), Err("localhost:2575".to_string()));
        // An unknown zone is not guessed: bind says what is wrong with it.
        assert_eq!(socket_address("fe80::1%no-such-if", 2575), Err("[fe80::1%no-such-if]:2575".to_string()));
        // An interface name (Linux and macOS): the loopback one always exists.
        #[cfg(target_os = "linux")]
        assert!(matches!(socket_address("fe80::1%lo", 2575), Ok(SocketAddr::V6(v6)) if v6.scope_id() > 0));
        #[cfg(target_os = "macos")]
        assert!(matches!(socket_address("fe80::1%lo0", 2575), Ok(SocketAddr::V6(v6)) if v6.scope_id() > 0));
    }

    /// Bytes that are not MLLP at all, trickled the same way, do not keep
    /// a connection either.
    #[tokio::test]
    async fn trickled_junk_is_dropped_at_the_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let limits = Limits { message_deadline: Duration::from_millis(1200), ..TEST_LIMITS };
        let cfg = ListenerConfig { read_timeout_secs: 1, ..ListenerConfig::default() };
        let (sink, tx, task) = spawn_listener(listener, cfg, limits);
        let mut client = TcpStream::connect(addr).await.unwrap();
        let started = Instant::now();
        while sink.errors().is_empty() && started.elapsed() < Duration::from_secs(5) {
            let _ = client.write_all(b"x").await;
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        assert!(started.elapsed() < Duration::from_secs(4), "{:?}", started.elapsed());
        assert!(sink.errors()[0].contains("no complete message"), "{:?}", sink.errors());
        tx.send(()).unwrap();
        assert_eq!(task.await.unwrap(), Ok(()));
    }

    /// Messages that arrive one after another on a connection, each in
    /// good time, are not held to the deadline as a whole.
    #[tokio::test]
    async fn steady_messages_on_one_connection_outlast_the_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let limits = Limits { message_deadline: Duration::from_millis(1000), ..TEST_LIMITS };
        let cfg = ListenerConfig { read_timeout_secs: 1, ..ListenerConfig::default() };
        let (sink, tx, task) = spawn_listener(listener, cfg, limits);
        let mut client = TcpStream::connect(addr).await.unwrap();
        for i in 0..8 {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let id = format!("S{i}");
            client.write_all(&framed(&hl7(&id))).await.unwrap();
            let ack = read_frame(&mut client).await;
            assert!(ack.contains(&format!("MSA|AA|{id}")), "{ack}");
        }
        assert!(sink.errors().is_empty(), "{:?}", sink.errors());
        assert_eq!(sink.messages(), 8);
        tx.send(()).unwrap();
        assert_eq!(task.await.unwrap(), Ok(()));
    }

    /// The messages still arriving on all connections together are held
    /// to the budget: the one that would go past it is refused with AR,
    /// and the others go on.
    #[tokio::test]
    async fn the_buffered_bytes_of_all_connections_are_bounded() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let limits = Limits { buffered_bytes: 64 * 1024, ..TEST_LIMITS };
        let (sink, tx, task) = spawn_listener(listener, test_config(), limits);

        // The first connection holds most of the budget with a message it
        // has not finished.
        let mut holder = TcpStream::connect(addr).await.unwrap();
        let mut big = vec![MLLP_START];
        big.extend_from_slice(hl7("HOLD").as_bytes());
        big.extend_from_slice(b"OBX|1|TX|||");
        big.resize(40 * 1024, b'A');
        holder.write_all(&big).await.unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;

        // The second fills what is left, and one byte more is refused. (The
        // byte that goes past it comes last, so the listener has read all
        // it was sent when it closes the connection: a close with unread
        // bytes resets it, and the AR could be lost.)
        let mut second = TcpStream::connect(addr).await.unwrap();
        let mut more = vec![MLLP_START];
        more.extend_from_slice(hl7("OVER").as_bytes());
        more.resize(24 * 1024, b'B');
        second.write_all(&more).await.unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(sink.errors().is_empty(), "the budget is not full yet: {:?}", sink.errors());
        second.write_all(b"B").await.unwrap();
        let ack = read_frame(&mut second).await;
        assert!(ack.contains("MSA|AR|OVER") && ack.contains("too much at once"), "{ack}");
        assert!(closed_within(&mut second, Duration::from_secs(5)).await);
        wait_until("the console row", || sink.errors().iter().any(|e| e.contains("all connections together"))).await;

        // The first finishes its message and is acknowledged.
        holder.write_all(&[MLLP_END_1, MLLP_END_2]).await.unwrap();
        assert!(read_frame(&mut holder).await.contains("MSA|AA|HOLD"));

        // With the budget given back, a message of the same size fits.
        let mut third = TcpStream::connect(addr).await.unwrap();
        let mut same = vec![MLLP_START];
        same.extend_from_slice(hl7("FITS").as_bytes());
        same.resize(30 * 1024, b'C');
        same.extend_from_slice(&[MLLP_END_1, MLLP_END_2]);
        third.write_all(&same).await.unwrap();
        assert!(read_frame(&mut third).await.contains("MSA|AA|FITS"));

        tx.send(()).unwrap();
        assert_eq!(task.await.unwrap(), Ok(()));
    }

    /// The app's limits leave ordinary test traffic alone: a few senders
    /// at once, each with hundreds of messages on its connection, large
    /// messages among them, and hundreds of one-message connections.
    #[tokio::test]
    async fn ordinary_traffic_is_untouched_by_the_limits() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (sink, tx, task) = spawn_listener(listener, ListenerConfig::default(), LIMITS);

        let senders: Vec<_> = (0..6)
            .map(|s| {
                tokio::spawn(async move {
                    let mut c = TcpStream::connect(addr).await.unwrap();
                    for i in 0..200 {
                        let id = format!("P{s}-{i}");
                        let mut msg = hl7(&id);
                        if i == 100 {
                            // One 3 MB message per sender, all about at once.
                            msg.push_str(&"A".repeat(3 * 1024 * 1024));
                        }
                        c.write_all(&framed(&msg)).await.unwrap();
                        let ack = read_frame(&mut c).await;
                        assert!(ack.contains(&format!("MSA|AA|{id}")), "{ack}");
                    }
                })
            })
            .collect();
        for s in senders {
            s.await.unwrap();
        }
        for i in 0..300 {
            let mut c = TcpStream::connect(addr).await.unwrap();
            let id = format!("Q{i}");
            c.write_all(&framed(&hl7(&id))).await.unwrap();
            assert!(read_frame(&mut c).await.contains(&format!("MSA|AA|{id}")));
        }
        assert!(sink.errors().is_empty(), "{:?}", sink.errors());
        assert_eq!(sink.messages(), 6 * 200 + 300);
        tx.send(()).unwrap();
        assert_eq!(task.await.unwrap(), Ok(()));
    }

    #[test]
    fn a_share_gives_its_bytes_back_however_the_connection_ends() {
        let budget = Arc::new(Budget::new(100));
        let mut a = Share::new(budget.clone());
        assert!(a.resize(60));
        let mut b = Share::new(budget.clone());
        assert!(!b.resize(50), "60 + 50 is past 100");
        assert!(b.resize(40));
        assert!(a.resize(10), "shrinking always works");
        assert!(b.resize(90));
        drop(a);
        drop(b);
        assert_eq!(budget.used.load(Ordering::SeqCst), 0);
    }

    /// When the accept loop ends on its own, the status says the listener
    /// stopped and the frontend is told why; it used to read "running"
    /// while every connection was refused.
    #[tokio::test]
    async fn the_status_reads_stopped_once_the_accept_loop_ends() {
        let state = ListenerState::new();
        let incoming = Flaky::new(0, too_many_open_files).await;
        let addr = incoming.addr();
        let fatal = incoming.fatal.clone();
        let sink = Arc::new(Recorder::default());
        let cfg = ListenerConfig { port: addr.port(), ..test_config() };
        let status = state.start_on(incoming, sink.clone(), cfg, TEST_LIMITS).await;
        assert!(status.running);
        assert_eq!(status.port, Some(addr.port()));

        fatal.notify_one();
        wait_until("the stopped event", || sink.stopped.lock().unwrap().is_some()).await;
        let reason = sink.stopped.lock().unwrap().clone().unwrap();
        assert!(reason.starts_with("accept failed"), "{reason}");
        // Read at once, as the frontend does when the event arrives.
        let status = state.status().await;
        assert!(!status.running && status.port.is_none(), "{status:?}");
        assert!(!state.stop().await.running);
    }

    /// The default decodes a Latin-1 message as Latin-1 (with or without
    /// MSH-18) and a UTF-8 one as UTF-8; an explicit choice is kept.
    #[test]
    fn automatic_encoding_handles_latin1_and_utf8_traffic() {
        let cfg = ListenerConfig::default();
        let latin1 = b"MSH|^~\\&|UP|HOSP|BL|LAB|20260101120000||ADT^A01|LAT1|P|2.5\rPID|1||123||M\xfcller^J\xf6rg\r";
        let (text, label) = decode(latin1, &cfg);
        assert!(text.contains("M\u{fc}ller^J\u{f6}rg"), "{text}");
        assert_eq!(label, "ISO-8859-1");

        let declared = b"MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5|||||POL|8859/2\rPID|1||1||\xa3\xf3d\xbc\r";
        let (text, label) = decode(declared, &cfg);
        assert!(text.contains("\u{141}\u{f3}d\u{17a}"), "{text}");
        assert_eq!(label, "ISO-8859-2");

        let utf8 = "\u{FEFF}MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||1||Müller\r";
        let (text, label) = decode(utf8.as_bytes(), &cfg);
        assert!(text.starts_with("MSH|") && text.contains("Müller"));
        assert_eq!(label, "UTF-8");

        let explicit = ListenerConfig { encoding: "windows-1252".into(), ..ListenerConfig::default() };
        assert_eq!(decode(b"MSH|^~\\&|\x80", &explicit), ("MSH|^~\\&|\u{20ac}".to_string(), "windows-1252".to_string()));
    }

    fn buffer(bytes: &[u8]) -> FrameBuffer {
        let mut f = FrameBuffer::default();
        f.extend(bytes);
        f
    }

    #[test]
    fn several_frames_in_one_read_are_split_and_a_partial_one_waits() {
        let mut bytes = framed("MSH|1");
        bytes.extend(framed("MSH|2"));
        bytes.extend_from_slice(&[MLLP_START, b'M']);
        let mut f = buffer(&bytes);
        let frames = f.take();
        assert_eq!(frames, vec![Frame::Message(b"MSH|1".to_vec()), Frame::Message(b"MSH|2".to_vec())]);
        assert_eq!(f.buf, vec![MLLP_START, b'M']);
        assert_eq!(f.junk, 0);
    }

    #[test]
    fn bytes_outside_a_frame_are_counted_as_junk() {
        let mut bytes = b"GET / HTTP/1.1\r\n".to_vec();
        bytes.extend(framed("MSH|1"));
        let mut f = buffer(&bytes);
        assert_eq!(f.take(), vec![Frame::Message(b"MSH|1".to_vec())]);
        assert_eq!(f.junk, 16);
    }

    /// A frame read in pieces is found whatever the pieces, its FS and CR
    /// in different reads included; each read searches only what it added.
    #[test]
    fn a_frame_split_across_reads_is_found_once_whole() {
        let whole = framed("MSH|^~\\&|split");
        for cut in 1..whole.len() {
            let mut f = buffer(&whole[..cut]);
            assert!(f.take().is_empty(), "cut at {cut}");
            assert_eq!(f.scanned, f.buf.len(), "cut at {cut}");
            f.extend(&whole[cut..]);
            assert_eq!(f.take(), vec![Frame::Message(b"MSH|^~\\&|split".to_vec())], "cut at {cut}");
            assert!(f.buf.is_empty() && f.scanned == 0);
        }
        // A frame end in the bytes before the VT is junk, not a frame.
        let mut bytes = vec![MLLP_END_1, MLLP_END_2];
        bytes.extend(framed("MSH|x"));
        assert_eq!(buffer(&bytes).take(), vec![Frame::Message(b"MSH|x".to_vec())]);
    }

    /// A message read 8 KB at a time is searched once through, not once
    /// per read.
    #[test]
    fn a_message_read_in_chunks_is_searched_once() {
        let mut whole = vec![MLLP_START];
        whole.resize(1024 * 1024, b'A');
        whole.extend_from_slice(&[MLLP_END_1, MLLP_END_2]);
        let mut f = FrameBuffer::default();
        let mut frames = Vec::new();
        for piece in whole.chunks(8 * 1024) {
            f.extend(piece);
            frames.extend(f.take());
        }
        assert_eq!(frames.len(), 1);
        assert!(f.searched < 2 * whole.len(), "{} bytes searched for a {} byte message", f.searched, whole.len());
    }

    #[test]
    fn a_large_message_leaves_no_large_buffer_behind() {
        let mut big = vec![MLLP_START];
        big.resize(2 * 1024 * 1024, b'A');
        let mut f = buffer(&big);
        assert!(f.take().is_empty());
        f.extend(&[MLLP_END_1, MLLP_END_2]);
        assert!(matches!(&f.take()[..], [Frame::Message(m)] if m.len() == 2 * 1024 * 1024 - 1));
        assert!(f.buf.capacity() <= KEPT_CAPACITY, "{}", f.buf.capacity());
    }

    #[test]
    fn an_endless_frame_is_refused_not_truncated() {
        let mut bytes = vec![MLLP_START];
        bytes.extend_from_slice(b"MSH|^~\\&|A|B|C|D|20240101||ADT^A01|E1|P|2.5\r");
        bytes.resize(MAX_MSG_BYTES + 10, b'A');
        let mut f = buffer(&bytes);
        let frames = f.take();
        assert!(matches!(&frames[..], [Frame::TooLarge(head)] if head.starts_with(b"MSH|")));
        assert!(f.buf.is_empty());
    }

    /// A frame just over the limit whose end is already buffered is
    /// refused too, not accepted because its terminator came with it.
    #[test]
    fn a_complete_frame_over_the_limit_is_refused() {
        for extra in [1usize, 500, 4000] {
            let mut bytes = vec![MLLP_START];
            bytes.extend_from_slice(b"MSH|^~\\&|A|B|C|D|20240101||ADT^A01|E1|P|2.5\r");
            bytes.resize(1 + MAX_MSG_BYTES + extra, b'A');
            bytes.extend_from_slice(&[MLLP_END_1, MLLP_END_2]);
            let frames = buffer(&bytes).take();
            assert!(matches!(&frames[..], [Frame::TooLarge(_)]), "+{extra}");
        }
        let mut bytes = vec![MLLP_START];
        bytes.resize(1 + MAX_MSG_BYTES, b'A');
        bytes.extend_from_slice(&[MLLP_END_1, MLLP_END_2]);
        assert!(matches!(&buffer(&bytes).take()[..], [Frame::Message(m)] if m.len() == MAX_MSG_BYTES));
    }
}
