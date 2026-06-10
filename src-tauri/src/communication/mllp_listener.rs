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
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::communication::mllp::{
    decode_with_label, encode_with_label, MLLP_END_1, MLLP_END_2, MLLP_START,
};

// 10 MiB read cap matches the rest of the parser story.
const MAX_MSG_BYTES: usize = 10 * 1024 * 1024;

/// User-tunable knobs applied to the listening socket and to every
/// connection it accepts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenerConfig {
    /// TCP port to bind. Default 2575 in the UI.
    pub port: u16,
    /// Bind address. `"0.0.0.0"` to accept connections from any interface,
    /// `"127.0.0.1"` to restrict to localhost (safer on dev machines).
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
    /// Empty string falls back to UTF-8-with-Latin-1-failover.
    #[serde(default)]
    pub encoding: String,
}

impl Default for ListenerConfig {
    fn default() -> Self {
        Self {
            port: 2575,
            bind_address: "0.0.0.0".into(),
            auto_ack: true,
            ack_code: "AA".into(),
            read_timeout_secs: 30,
            encoding: String::new(),
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
    /// Charset the payload was decoded with ("UTF-8" when unset).
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
        let task = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer)) => {
                        let app2 = app_for_task.clone();
                        let cfg2 = cfg.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, peer.to_string(), cfg2, app2.clone()).await {
                                let _ = app2.emit("mllp:listen_error", e);
                            }
                        });
                    }
                    Err(e) => {
                        let _ = app_for_task.emit("mllp:listen_error", format!("accept failed: {}", e));
                        break;
                    }
                }
            }
        });

        let mut inner = self.0.lock().await;
        inner.handle = Some(task);
        inner.config = Some(config);
        drop(inner);
        Ok(self.status().await)
    }

    pub async fn stop(&self) -> ListenerStatus {
        self.stop_internal().await;
        self.status().await
    }

    async fn stop_internal(&self) {
        let mut inner = self.0.lock().await;
        if let Some(handle) = inner.handle.take() {
            handle.abort();
        }
        inner.config = None;
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    source_addr: String,
    cfg: ListenerConfig,
    app: AppHandle,
) -> Result<(), String> {
    // Read until MLLP terminator (FS CR), capped at MAX_MSG_BYTES.
    let mut buf: Vec<u8> = Vec::with_capacity(8 * 1024);
    let mut chunk = [0u8; 8 * 1024];

    let read_status = tokio::time::timeout(Duration::from_secs(cfg.read_timeout_secs), async {
        loop {
            match stream.read(&mut chunk).await {
                Ok(0) => return Ok::<bool, std::io::Error>(true),  // EOF
                Ok(n) => {
                    buf.extend_from_slice(&chunk[..n]);
                    let len = buf.len();
                    if len >= 2 && buf[len - 2] == MLLP_END_1 && buf[len - 1] == MLLP_END_2 {
                        return Ok(false); // got terminator
                    }
                    if len > MAX_MSG_BYTES {
                        return Ok(false); // cap hit; treat as terminator-less
                    }
                }
                Err(e) => return Err(e),
            }
        }
    })
    .await;

    match read_status {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => return Err(format!("read failed: {}", e)),
        Err(_) => return Err(format!("connection from {} idle past {}s", source_addr, cfg.read_timeout_secs)),
    }

    // Strip MLLP framing bytes manually so we can decode the payload with
    // the user-selected charset. Empty payload after stripping → reject.
    let mut start_idx = 0usize;
    let mut end_idx = buf.len();
    if !buf.is_empty() && buf[0] == MLLP_START { start_idx = 1; }
    if end_idx > start_idx && buf[end_idx - 1] == MLLP_END_2 { end_idx -= 1; }
    if end_idx > start_idx && buf[end_idx - 1] == MLLP_END_1 { end_idx -= 1; }
    if start_idx >= end_idx {
        return Err("could not unframe MLLP payload".into());
    }
    let payload_bytes = end_idx - start_idx;
    let content = decode_with_label(&buf[start_idx..end_idx], &cfg.encoding);

    // ACK code actually written back, surfaced to the frontend console.
    let mut ack_sent: Option<String> = None;
    if cfg.auto_ack {
        use crate::parser::hl7::ack;
        let control_id = ack::extract_message_control_id(&content).unwrap_or_default();
        let sending_app = ack::extract_sending_app(&content).unwrap_or_default();
        let ack_msg = ack::generate_ack(&cfg.ack_code, &control_id, "BridgeLab", &sending_app, None);
        // Re-encode the ACK with the same charset so the peer doesn't see
        // mojibake on its side.
        let payload = encode_with_label(&ack_msg, &cfg.encoding);
        let mut framed = Vec::with_capacity(payload.len() + 3);
        framed.push(MLLP_START);
        framed.extend_from_slice(&payload);
        framed.push(MLLP_END_1);
        framed.push(MLLP_END_2);
        if stream.write_all(&framed).await.is_ok() {
            ack_sent = Some(cfg.ack_code.clone());
        }
    }

    let _ = stream.shutdown().await;

    let _ = app.emit("mllp:received", ReceivedEvent {
        content,
        source_addr,
        received_at: chrono::Utc::now().to_rfc3339(),
        bytes: payload_bytes,
        ack_code: ack_sent,
        encoding: if cfg.encoding.is_empty() { "UTF-8".into() } else { cfg.encoding.clone() },
    });
    Ok(())
}
