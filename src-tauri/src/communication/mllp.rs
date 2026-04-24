use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

/// MLLP framing bytes
const MLLP_START: u8 = 0x0B; // VT (vertical tab)
const MLLP_END_1: u8 = 0x1C; // FS (file separator)
const MLLP_END_2: u8 = 0x0D; // CR (carriage return)

/// Result of an MLLP send operation.
#[derive(Debug, Clone, Serialize)]
pub struct MllpSendResult {
    pub success: bool,
    pub response: String,
    pub response_time_ms: u64,
    pub error: Option<String>,
}

/// Result of an MLLP receive operation (single message).
#[derive(Debug, Clone, Serialize)]
pub struct MllpReceivedMessage {
    pub content: String,
    pub source_addr: String,
    pub received_at: String,
}

/// Wrap a message in MLLP framing.
pub fn mllp_frame(message: &str) -> Vec<u8> {
    let mut framed = Vec::with_capacity(message.len() + 3);
    framed.push(MLLP_START);
    framed.extend_from_slice(message.as_bytes());
    framed.push(MLLP_END_1);
    framed.push(MLLP_END_2);
    framed
}

/// Remove MLLP framing from received data.
pub fn mllp_unframe(data: &[u8]) -> Option<String> {
    if data.is_empty() {
        return None;
    }

    let start = if data[0] == MLLP_START { 1 } else { 0 };
    let mut end = data.len();

    // Strip trailing MLLP_END_2 and MLLP_END_1
    if end > 0 && data[end - 1] == MLLP_END_2 {
        end -= 1;
    }
    if end > 0 && data[end - 1] == MLLP_END_1 {
        end -= 1;
    }

    if start >= end {
        return None;
    }

    String::from_utf8(data[start..end].to_vec()).ok()
}

/// Send an HL7 message via MLLP to a remote host.
pub async fn send(
    host: &str,
    port: u16,
    message: &str,
    timeout_secs: u64,
) -> MllpSendResult {
    let start = Instant::now();
    let addr = format!("{}:{}", host, port);

    let connect_result = tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        TcpStream::connect(&addr),
    )
    .await;

    let mut stream = match connect_result {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            return MllpSendResult {
                success: false,
                response: String::new(),
                response_time_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("Connection failed: {}", e)),
            };
        }
        Err(_) => {
            return MllpSendResult {
                success: false,
                response: String::new(),
                response_time_ms: start.elapsed().as_millis() as u64,
                error: Some("Connection timed out".into()),
            };
        }
    };

    // Send framed message
    let framed = mllp_frame(message);
    if let Err(e) = stream.write_all(&framed).await {
        return MllpSendResult {
            success: false,
            response: String::new(),
            response_time_ms: start.elapsed().as_millis() as u64,
            error: Some(format!("Send failed: {}", e)),
        };
    }

    // Read ACK by looping until we see the MLLP terminator (FS CR) or EOF.
    // A single read() may return a partial ACK if the peer flushes in two
    // TCP segments — leaving unread bytes in the kernel buffer. Dropping
    // the stream with unread bytes causes the kernel to send RST instead
    // of FIN, which surfaces on the peer as "Connection reset by peer"
    // after it has already sent the ACK.
    let mut response_bytes: Vec<u8> = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];

    let read_status = tokio::time::timeout(Duration::from_secs(timeout_secs), async {
        loop {
            match stream.read(&mut chunk).await {
                Ok(0) => return Ok::<(), std::io::Error>(()), // peer closed
                Ok(n) => {
                    response_bytes.extend_from_slice(&chunk[..n]);
                    let len = response_bytes.len();
                    if len >= 2
                        && response_bytes[len - 2] == MLLP_END_1
                        && response_bytes[len - 1] == MLLP_END_2
                    {
                        return Ok(());
                    }
                }
                Err(e) => return Err(e),
            }
        }
    })
    .await;

    // Gracefully half-close the write side before dropping the stream so the
    // peer sees FIN, not RST. This is the fix for the "socket error after
    // ACK" symptom reported by receivers like HAPI / Mirth.
    let _ = stream.shutdown().await;

    let response = mllp_unframe(&response_bytes).unwrap_or_default();
    let response_time_ms = start.elapsed().as_millis() as u64;

    match read_status {
        Ok(Ok(())) if !response_bytes.is_empty() => MllpSendResult {
            success: true,
            response,
            response_time_ms,
            error: None,
        },
        Ok(Ok(())) => MllpSendResult {
            success: false,
            response: String::new(),
            response_time_ms,
            error: Some("Empty response (connection closed by peer)".into()),
        },
        Ok(Err(e)) => MllpSendResult {
            success: false,
            response,
            response_time_ms,
            error: Some(format!("Read failed: {}", e)),
        },
        Err(_) => MllpSendResult {
            success: false,
            response,
            response_time_ms,
            error: Some("Response timed out".into()),
        },
    }
}

/// Listen for a single incoming MLLP message on the given port.
/// Returns the received message after accepting one connection.
pub async fn receive_one(
    port: u16,
    timeout_secs: u64,
    auto_ack: bool,
) -> Result<MllpReceivedMessage, String> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind to port {}: {}", port, e))?;

    let accept_result = tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        listener.accept(),
    )
    .await;

    let (mut stream, peer_addr) = match accept_result {
        Ok(Ok((s, a))) => (s, a),
        Ok(Err(e)) => return Err(format!("Accept failed: {}", e)),
        Err(_) => return Err(format!("No connection received within {} seconds", timeout_secs)),
    };

    // Read incoming MLLP message
    let mut buf = vec![0u8; 10 * 1024 * 1024]; // 10MB buffer
    let mut total = 0;

    loop {
        let read_result = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            stream.read(&mut buf[total..]),
        )
        .await;

        match read_result {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => {
                total += n;
                // Check if we got the MLLP end marker
                if total >= 2 && buf[total - 2] == MLLP_END_1 && buf[total - 1] == MLLP_END_2 {
                    break;
                }
            }
            Ok(Err(e)) => return Err(format!("Read error: {}", e)),
            Err(_) => break,
        }
    }

    let content = mllp_unframe(&buf[..total])
        .ok_or_else(|| "Failed to unframe MLLP message".to_string())?;

    // Send ACK if auto_ack is enabled
    if auto_ack {
        use crate::parser::hl7::ack;
        let control_id = ack::extract_message_control_id(&content).unwrap_or_default();
        let sending_app = ack::extract_sending_app(&content).unwrap_or_default();
        let ack_msg = ack::generate_ack("AA", &control_id, "BridgeLab", &sending_app, None);
        let ack_framed = mllp_frame(&ack_msg);
        let _ = stream.write_all(&ack_framed).await;
    }

    // Gracefully half-close write side so the client receives FIN, not RST,
    // after the ACK has been flushed. Mirror of the fix applied in send().
    let _ = stream.shutdown().await;

    let received_at = chrono::Utc::now().to_rfc3339();

    Ok(MllpReceivedMessage {
        content,
        source_addr: peer_addr.to_string(),
        received_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mllp_frame() {
        let msg = "MSH|^~\\&|test";
        let framed = mllp_frame(msg);
        assert_eq!(framed[0], MLLP_START);
        assert_eq!(framed[framed.len() - 2], MLLP_END_1);
        assert_eq!(framed[framed.len() - 1], MLLP_END_2);
        assert_eq!(framed.len(), msg.len() + 3);
    }

    #[test]
    fn test_mllp_unframe() {
        let msg = "MSH|^~\\&|test";
        let framed = mllp_frame(msg);
        let unframed = mllp_unframe(&framed).unwrap();
        assert_eq!(unframed, msg);
    }

    #[test]
    fn test_mllp_unframe_no_framing() {
        let msg = b"MSH|^~\\&|test";
        let unframed = mllp_unframe(msg).unwrap();
        assert_eq!(unframed, "MSH|^~\\&|test");
    }

    #[tokio::test]
    async fn test_mllp_roundtrip() {
        // Start a listener
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let msg = "MSH|^~\\&|Send|SF|Recv|RF|20230101||ADT^A01|MSG001|P|2.5\rPID|||12345";

        // Spawn listener task that echoes back
        let _handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let _n = stream.read(&mut buf).await.unwrap();
            // Echo the same data back as ACK
            let response = mllp_frame("MSH|^~\\&|Recv||Send||20230101||ACK|ACK001|P|2.5\rMSA|AA|MSG001");
            stream.write_all(&response).await.unwrap();
        });

        let result = send("127.0.0.1", port, msg, 5).await;
        assert!(result.success);
        assert!(result.response.contains("MSA|AA|MSG001"));

        _handle.await.unwrap();
    }

    /// BL-MLLP-09/10/11 (backend slice): start the real `receive_one` listener,
    /// send a framed HL7 message to it, and verify the listener returns the
    /// decoded payload AND writes back an AA ACK when auto_ack is enabled.
    #[tokio::test]
    async fn test_mllp_receive_one_with_auto_ack() {
        // Bind to an ephemeral port first, then hand it to receive_one.
        // receive_one binds 0.0.0.0:{port} itself, so we need to pick a free
        // port separately and then spawn the listener.
        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe); // release so receive_one can bind

        // Spawn the receiver under test
        let receiver = tokio::spawn(async move {
            receive_one(port, 5, true).await
        });

        // Give the listener a moment to start
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Client: framed send to localhost:port
        let msg = "MSH|^~\\&|Sender|Fac|Receiver|Fac|20260415120000||ADT^A01|UNIT001|P|2.5\rPID|||42";
        let result = send("127.0.0.1", port, msg, 5).await;
        assert!(result.success, "client send failed: {:?}", result.error);
        // auto_ack should have returned an AA ACK referencing our control id
        assert!(result.response.contains("MSA|AA"),
                "expected AA ACK, got: {}", result.response);
        assert!(result.response.contains("UNIT001"),
                "ACK should echo control id, got: {}", result.response);

        // Server side: should have captured the inbound message
        let received = receiver.await.unwrap().expect("receive_one errored");
        assert_eq!(received.content, msg);
        assert!(received.source_addr.starts_with("127.0.0.1:"));
        assert!(!received.received_at.is_empty());
    }

    /// Regression: ACK delivered in two TCP segments must be fully assembled
    /// by the client. Before the loop-read fix, the second segment was left
    /// in the kernel buffer and the response came back truncated (or empty,
    /// causing an MLLP unframe failure). Also exercises the graceful close
    /// path (shutdown before drop) — without it the test peer would observe
    /// a reset on its read side.
    #[tokio::test]
    async fn test_mllp_send_handles_split_ack() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        // Peer that writes the ACK in two bursts with a deliberate gap to
        // force the client through multiple read() calls.
        let _peer = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let _ = stream.read(&mut buf).await.unwrap();

            let ack = mllp_frame(
                "MSH|^~\\&|Recv||Send||20260423||ACK|ACK999|P|2.5\rMSA|AA|MSG999",
            );
            let split = ack.len() / 2;
            stream.write_all(&ack[..split]).await.unwrap();
            stream.flush().await.unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
            stream.write_all(&ack[split..]).await.unwrap();
            stream.flush().await.unwrap();

            // Let the client drive the close; read until EOF so we can
            // confirm the client shut down cleanly (FIN, not RST).
            let _ = stream.read(&mut buf).await;
        });

        let msg = "MSH|^~\\&|Send|SF|Recv|RF|20260423||ADT^A01|MSG999|P|2.5\rPID|||1";
        let result = send("127.0.0.1", port, msg, 5).await;

        assert!(result.success, "send failed: {:?}", result.error);
        assert!(
            result.response.contains("MSA|AA|MSG999"),
            "expected full ACK body, got: {:?}",
            result.response
        );
    }

    /// BL-MLLP-08 equivalent: connection to a closed port fails fast with an
    /// actionable error, not a hang.
    #[tokio::test]
    async fn test_mllp_send_refused() {
        // Grab a port that we immediately release
        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);

        let result = send("127.0.0.1", port, "MSH|^~\\&|x", 2).await;
        assert!(!result.success);
        assert!(result.error.is_some());
    }
}
