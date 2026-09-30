//! Persistent MLLP listener.
//!
//! Unlike `mllp::receive_one` (single-shot, returns after one message), this
//! module owns a long-running task that keeps accepting connections until
//! explicitly stopped. Each received message is emitted to the frontend via
//! the Tauri event bus (`mllp:received`); errors hit `mllp:listen_error`.
//!
//! State is held in `ListenerState` and `tauri::Manager`-managed so the
//! frontend can issue start/stop without losing the handle between IPC calls.

use std::sync::Arc;
use std::time::Duration;

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
    /// past this are dropped — the listener itself stays up.
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
        match &inner.config {
            Some(c) if inner.handle.is_some() => ListenerStatus {
                running: true,
                port: Some(c.port),
                bind_address: Some(c.bind_address.clone()),
            },
            _ => ListenerStatus { running: false, port: None, bind_address: None },
        }
    }

    pub async fn start(&self, app: AppHandle, config: ListenerConfig) -> Result<ListenerStatus, String> {
        // Stop any prior listener so port-change / config-change is one IPC call.
        self.stop_internal().await;

        let bind = format!("{}:{}", config.bind_address, config.port);
        let listener = TcpListener::bind(&bind)
            .await
            .map_err(|e| format!("Failed to bind to {}: {}", bind, e))?;

        let cfg = config.clone();
        let app_for_task = app.clone();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            let serve = move |stream: TcpStream, peer: String| {
                let app2 = app_for_task.clone();
                let cfg2 = cfg.clone();
                async move {
                    if let Err(e) = handle_connection(stream, peer, cfg2, app2.clone()).await {
                        let _ = app2.emit("mllp:listen_error", e);
                    }
                }
            };
            if let Err(e) = accept_loop(listener, shutdown_rx, serve).await {
                let _ = app.emit("mllp:listen_error", e);
            }
        });

        let mut inner = self.0.lock().await;
        inner.handle = Some(task);
        inner.shutdown = Some(shutdown_tx);
        inner.config = Some(config);
        drop(inner);
        Ok(self.status().await)
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
    }
}

/// Accept connections, each served by `serve` in a task of its own, until
/// `shutdown` fires (or its sender is dropped); then abort every
/// connection still open and wait for them to end. An accept error ends
/// the loop the same way and is returned.
async fn accept_loop<F, Fut>(listener: TcpListener, mut shutdown: oneshot::Receiver<()>, serve: F) -> Result<(), String>
where
    F: Fn(TcpStream, String) -> Fut,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let mut connections = JoinSet::new();
    let result = loop {
        tokio::select! {
            _ = &mut shutdown => break Ok(()),
            accepted = listener.accept() => match accepted {
                Ok((stream, peer)) => {
                    connections.spawn(serve(stream, peer.to_string()));
                }
                Err(e) => break Err(format!("accept failed: {}", e)),
            },
            // Reap finished connections so the set does not grow.
            Some(_) = connections.join_next(), if !connections.is_empty() => {}
        }
    };
    drop(listener);
    connections.shutdown().await;
    result
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

/// Take every complete frame out of `buf`, leaving a partial one in place.
/// Bytes before a VT are not MLLP and are dropped (their count is added
/// to `junk`).
fn take_frames(buf: &mut Vec<u8>, junk: &mut usize) -> Vec<Frame> {
    let mut out = Vec::new();
    loop {
        let Some(start) = buf.iter().position(|&b| b == MLLP_START) else {
            *junk += buf.len();
            buf.clear();
            break;
        };
        if start > 0 {
            *junk += start;
            buf.drain(..start);
        }
        match buf.windows(2).position(|w| w == [MLLP_END_1, MLLP_END_2]) {
            // The whole frame arrived in one read, and it is too large:
            // refused like one still streaming.
            Some(end) if end - 1 > MAX_MSG_BYTES => {
                let head = buf[1..4096].to_vec();
                buf.clear();
                out.push(Frame::TooLarge(head));
                break;
            }
            Some(end) => {
                let frame: Vec<u8> = buf[1..end].to_vec();
                buf.drain(..end + 2);
                out.push(Frame::Message(frame));
            }
            None if buf.len() > MAX_MSG_BYTES => {
                let head = buf[1..buf.len().min(4096)].to_vec();
                buf.clear();
                out.push(Frame::TooLarge(head));
                break;
            }
            None => break,
        }
    }
    out
}

/// Serve one connection: every framed message on it is received and
/// acknowledged in turn, until the peer closes it or it stays idle for the
/// read timeout. Interface engines keep MLLP connections open; the
/// listener used to take one message and hang up.
async fn handle_connection(
    mut stream: TcpStream,
    source_addr: String,
    cfg: ListenerConfig,
    app: AppHandle,
) -> Result<(), String> {
    let mut buf: Vec<u8> = Vec::with_capacity(8 * 1024);
    let mut chunk = [0u8; 8 * 1024];
    let mut junk = 0usize;
    let mut received = 0usize;

    loop {
        let read = tokio::time::timeout(Duration::from_secs(cfg.read_timeout_secs), stream.read(&mut chunk)).await;
        let n = match read {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(format!("read failed: {}", e)),
            // Quiet between messages is normal; quiet inside one is not.
            Err(_) if buf.is_empty() && received > 0 => break,
            Err(_) => {
                return Err(format!("connection from {} idle past {}s", source_addr, cfg.read_timeout_secs))
            }
        };
        buf.extend_from_slice(&chunk[..n]);
        for frame in take_frames(&mut buf, &mut junk) {
            match frame {
                Frame::Message(bytes) => {
                    received += 1;
                    handle_message(&mut stream, &bytes, &source_addr, &cfg, &app).await;
                }
                Frame::TooLarge(head) => {
                    // Refused, not truncated: an AA for the first 10 MiB
                    // told the sender a message it lost part of had
                    // arrived. The stream cannot be resynchronised, so
                    // the connection ends here.
                    let (content, label) = decode(&head, &cfg);
                    let limit = MAX_MSG_BYTES / (1024 * 1024);
                    if cfg.auto_ack {
                        let text = format!("Message larger than {} MiB refused", limit);
                        let _ = write_ack(&mut stream, &content, "AR", Some(&text), &label).await;
                    }
                    let _ = stream.shutdown().await;
                    return Err(format!(
                        "message from {} refused: larger than {} MiB (answered AR)",
                        source_addr, limit
                    ));
                }
            }
        }
    }

    let _ = stream.shutdown().await;
    if junk > 0 && received == 0 {
        return Err(format!(
            "{} sent {} bytes that are not an MLLP-framed message (no VT … FS CR); nothing was received",
            source_addr, junk
        ));
    }
    if !buf.is_empty() {
        return Err(format!("connection from {} closed in the middle of a message", source_addr));
    }
    Ok(())
}

/// Acknowledge one framed message and hand it to the console.
async fn handle_message(stream: &mut TcpStream, bytes: &[u8], source_addr: &str, cfg: &ListenerConfig, app: &AppHandle) {
    let (content, label) = decode(bytes, cfg);
    // A UTF-8 byte-order mark is not part of the message, and blank lines
    // before the header are not a reason to refuse it.
    let content = content.trim_start_matches('\u{FEFF}').to_string();
    let is_hl7 = matches!(content.trim_start().get(..3), Some("MSH" | "FHS" | "BHS"));

    let started = std::time::Instant::now();
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
        let _ = app.emit(
            "mllp:listen_error",
            format!("{} sent a frame that is not an HL7 message (answered AR)", source_addr),
        );
    }

    {
        use tauri::Manager;
        if let Some(c) = app.try_state::<crate::licensing::telemetry::UsageCounters>() {
            c.bump_mem("mllp_received");
        }
        // Received messages are logged in the history like sends.
        if let Some(db) = app.try_state::<crate::database::Database>() {
            use crate::communication::profiles::{history_text, HistoryEntry};
            let _ = db.add_history_entry(&HistoryEntry {
                id: uuid::Uuid::new_v4().to_string(),
                profile_name: source_addr.to_string(),
                profile_type: "mllp".into(),
                direction: "receive".into(),
                content_preview: String::new(),
                status: "OK".into(),
                response_time_ms: started.elapsed().as_millis() as u64,
                timestamp: chrono::Utc::now().to_rfc3339(),
                ack_code: ack_sent.clone(),
                target: source_addr.to_string(),
                size_bytes: bytes.len() as u64,
                request: history_text(&content),
                response: history_text(&ack_text),
            });
        }
    }

    let _ = app.emit("mllp:received", ReceivedEvent {
        content,
        source_addr: source_addr.to_string(),
        received_at: chrono::Utc::now().to_rfc3339(),
        bytes: bytes.len(),
        ack_code: ack_sent,
        encoding: if is_auto(&cfg.encoding) { format!("{} (auto)", label) } else { label },
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
        let task = tokio::spawn(accept_loop(listener, rx, serve));

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

    #[test]
    fn several_frames_in_one_read_are_split_and_a_partial_one_waits() {
        let mut buf = framed("MSH|1");
        buf.extend(framed("MSH|2"));
        buf.extend_from_slice(&[MLLP_START, b'M']);
        let mut junk = 0;
        let frames = take_frames(&mut buf, &mut junk);
        assert_eq!(frames, vec![Frame::Message(b"MSH|1".to_vec()), Frame::Message(b"MSH|2".to_vec())]);
        assert_eq!(buf, vec![MLLP_START, b'M']);
        assert_eq!(junk, 0);
    }

    #[test]
    fn bytes_outside_a_frame_are_counted_as_junk() {
        let mut buf = b"GET / HTTP/1.1\r\n".to_vec();
        buf.extend(framed("MSH|1"));
        let mut junk = 0;
        assert_eq!(take_frames(&mut buf, &mut junk), vec![Frame::Message(b"MSH|1".to_vec())]);
        assert_eq!(junk, 16);
    }

    #[test]
    fn an_endless_frame_is_refused_not_truncated() {
        let mut buf = vec![MLLP_START];
        buf.extend_from_slice(b"MSH|^~\\&|A|B|C|D|20240101||ADT^A01|E1|P|2.5\r");
        buf.resize(MAX_MSG_BYTES + 10, b'A');
        let mut junk = 0;
        let frames = take_frames(&mut buf, &mut junk);
        assert!(matches!(&frames[..], [Frame::TooLarge(head)] if head.starts_with(b"MSH|")));
        assert!(buf.is_empty());
    }

    /// A frame just over the limit whose end is already buffered is
    /// refused too, not accepted because its terminator came with it.
    #[test]
    fn a_complete_frame_over_the_limit_is_refused() {
        for extra in [1usize, 500, 4000] {
            let mut buf = vec![MLLP_START];
            buf.extend_from_slice(b"MSH|^~\\&|A|B|C|D|20240101||ADT^A01|E1|P|2.5\r");
            buf.resize(1 + MAX_MSG_BYTES + extra, b'A');
            buf.extend_from_slice(&[MLLP_END_1, MLLP_END_2]);
            let mut junk = 0;
            let frames = take_frames(&mut buf, &mut junk);
            assert!(matches!(&frames[..], [Frame::TooLarge(_)]), "+{extra}");
        }
        let mut buf = vec![MLLP_START];
        buf.resize(1 + MAX_MSG_BYTES, b'A');
        buf.extend_from_slice(&[MLLP_END_1, MLLP_END_2]);
        assert!(matches!(&take_frames(&mut buf, &mut 0)[..], [Frame::Message(m)] if m.len() == MAX_MSG_BYTES));
    }
}
