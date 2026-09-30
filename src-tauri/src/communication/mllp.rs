use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

/// MLLP framing bytes
pub const MLLP_START: u8 = 0x0B; // VT (vertical tab)
pub const MLLP_END_1: u8 = 0x1C; // FS (file separator)
pub const MLLP_END_2: u8 = 0x0D; // CR (carriage return)

/// Hard cap on how much ACK payload we'll buffer from the peer. Real HL7 ACKs
/// fit in a few KB; anything past this means the peer is either misbehaving
/// or hostile (streaming without a terminator). Prevents unbounded memory
/// growth that could otherwise run until the read timeout expires.
const MAX_ACK_BYTES: usize = 1 * 1024 * 1024; // 1 MiB

/// Inner state of the ACK read loop.
enum ReadOutcome {
    /// Saw the MLLP terminator (FS CR) — full ACK received.
    Terminator,
    /// Peer closed the connection before sending a terminator.
    Eof,
    /// Peer exceeded MAX_ACK_BYTES without sending a terminator.
    CapExceeded,
}

/// Result of an MLLP send operation.
#[derive(Debug, Clone, Serialize)]
pub struct MllpSendResult {
    pub success: bool,
    pub response: String,
    pub response_time_ms: u64,
    pub error: Option<String>,
    /// The character set the message was encoded in on the wire.
    #[serde(default)]
    pub encoding: String,
    /// MSA-1 of the reply ("AA", "AE", "AR", "CA", "CE", "CR"), when the
    /// send got one.
    #[serde(default)]
    pub ack_code: Option<String>,
}

/// Options for an MLLP send. `Default` gives standard MLLP framing
/// (VT / FS CR), 30s timeouts and UTF-8.
/// Non-standard framing bytes exist in the wild on legacy interfaces —
/// that's why they're tunable at all.
#[derive(Debug, Clone)]
pub struct SendOptions {
    /// Timeout for the TCP connect phase.
    pub connect_timeout_secs: u64,
    /// Timeout for reading the ACK after the message has been written.
    pub response_timeout_secs: u64,
    /// encoding_rs label ("UTF-8", "ISO-8859-1", ...); empty = UTF-8;
    /// [`AUTO`] = the charset the message's MSH-18 declares, else UTF-8.
    pub encoding: String,
    /// With [`AUTO`]: the charset the message's file was read in, used when
    /// MSH-18 names none (see [`resolve_send_encoding`]).
    pub source_charset: Option<String>,
    /// Start-of-block byte (standard: 0x0B VT).
    pub start_byte: u8,
    /// First end-of-block byte (standard: 0x1C FS).
    pub end_byte_1: u8,
    /// Second end-of-block byte (standard: 0x0D CR).
    pub end_byte_2: u8,
}

impl Default for SendOptions {
    fn default() -> Self {
        Self {
            connect_timeout_secs: 30,
            response_timeout_secs: 30,
            encoding: String::new(),
            source_charset: None,
            start_byte: MLLP_START,
            end_byte_1: MLLP_END_1,
            end_byte_2: MLLP_END_2,
        }
    }
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
///
/// HL7 v2 traffic in the wild is often Latin-1 / Windows-1252 (Italian
/// hospitals, German patient names, French diacritics) — the standard
/// declares the character set in MSH-18 but most senders never set it.
/// We try UTF-8 first and fall back to Latin-1 (a lossless 1:1 byte→char
/// mapping for the 0x00–0xFF range) so the listener doesn't reject
/// otherwise-valid messages with `could not unframe MLLP payload`.
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

    let bytes = &data[start..end];
    Some(decode_payload(bytes))
}

/// Decode a byte slice to a String. UTF-8 is preferred; on invalid UTF-8
/// we fall back to Latin-1 (every byte maps to its codepoint), which never
/// fails. Caller can re-encode if MSH-18 indicates something else.
fn decode_payload(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

/// The encoding label that picks the charset from the message: on
/// receive, MSH-18 when it names a charset, else UTF-8, else Latin-1 (see
/// [`decode_auto`]); on send, MSH-18, else UTF-8 (see
/// [`resolve_send_encoding`]). The default of the listener and the sender.
pub const AUTO: &str = "auto";

fn is_auto(label: &str) -> bool {
    label.trim().eq_ignore_ascii_case(AUTO)
}

/// The label BridgeLab encodes an HL7 character-set name (MSH-18) with:
/// `ASCII` stays 7-bit, the others as [`charset::label_for_hl7`].
fn label_for_msh18(name: &str) -> Option<String> {
    if name.trim().eq_ignore_ascii_case("ASCII") {
        return Some("ASCII".into());
    }
    crate::parser::hl7::charset::label_for_hl7(name)
}

/// Decode received bytes choosing the charset from them: the one MSH-18
/// declares, when BridgeLab knows it; otherwise UTF-8 when the bytes are
/// valid UTF-8; otherwise Latin-1, which maps every byte to a character.
/// A UTF-8 byte-order mark is dropped. Returns the text and the label of
/// the charset used.
pub fn decode_auto(bytes: &[u8]) -> (String, String) {
    use crate::parser::hl7::charset;
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    if let Some(label) = charset::declared_charset(bytes).and_then(|c| label_for_msh18(&c)) {
        if label != "UTF-8" {
            return (charset::decode_with(bytes, &label), label);
        }
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => (s.to_string(), "UTF-8".into()),
        Err(_) => (decode_payload(bytes), "ISO-8859-1".into()),
    }
}

/// Decode a byte slice using a named encoding (encoding_rs label, e.g.
/// "UTF-8", "ISO-8859-1", "windows-1252", "windows-1250", "windows-1251",
/// "ASCII"). An empty, [`AUTO`] or unknown label decodes with
/// [`decode_auto`].
pub fn decode_with_label(bytes: &[u8], label: &str) -> String {
    if label.trim().is_empty() || is_auto(label) || !crate::parser::hl7::charset::is_known_label(label) {
        return decode_auto(bytes).0;
    }
    crate::parser::hl7::charset::decode_with(bytes, label)
}

/// Encode a UTF-8 String into the bytes for the named encoding. Characters
/// the encoding cannot represent become `?` (never `&#NNN;` references), and
/// "ASCII" means 7-bit. Empty, UTF-8 or unknown labels give UTF-8. The
/// in-memory message is always UTF-8 inside BridgeLab.
pub fn encode_with_label(text: &str, label: &str) -> Vec<u8> {
    crate::parser::hl7::charset::encode_output(text, label)
}

/// The charset a message is sent in. An explicit label is kept. [`AUTO`]
/// takes the charset the message's MSH-18 declares, else the one the
/// message's file was read in (`source_charset`), else UTF-8: a message
/// that says 8859/1 must not go out as UTF-8 bytes under that label.
pub fn resolve_send_encoding(message: &str, requested: &str, source_charset: Option<&str>) -> String {
    if !is_auto(requested) {
        return if requested.trim().is_empty() { "UTF-8".into() } else { requested.trim().to_string() };
    }
    // A UTF-16 file (Notepad's "Unicode") is not sent as UTF-16: few
    // receivers read UTF-16 frames, and HL7 declares no UTF-16 without a
    // byte-order mark. It goes out as UTF-8 unless MSH-18 says otherwise.
    crate::parser::hl7::charset::declared_charset(message.as_bytes())
        .and_then(|c| label_for_msh18(&c))
        .or_else(|| {
            source_charset
                .map(str::trim)
                .filter(|c| !c.is_empty() && !crate::parser::hl7::charset::is_utf16(c))
                .map(str::to_string)
        })
        .unwrap_or_else(|| "UTF-8".into())
}

/// The characters of `text` that `label` cannot represent (each once, in
/// order of appearance): they would go out as `?`.
pub fn unencodable_chars(text: &str, label: &str) -> Vec<char> {
    let mut out: Vec<char> = Vec::new();
    for c in text.chars().filter(|c| !c.is_ascii() || label.trim().eq_ignore_ascii_case("ASCII")) {
        if c != '?' && !out.contains(&c) && encode_with_label(c.encode_utf8(&mut [0u8; 4]), label).ends_with(b"?") {
            out.push(c);
        }
    }
    out
}

/// Send an HL7 message via MLLP to a remote host with default options
/// (standard framing, `timeout_secs` for both connect and response).
/// `encoding` is an `encoding_rs` label; empty string = UTF-8 failover.
pub async fn send(
    host: &str,
    port: u16,
    message: &str,
    timeout_secs: u64,
    encoding: &str,
) -> MllpSendResult {
    send_with_options(host, port, message, &SendOptions {
        connect_timeout_secs: timeout_secs,
        response_timeout_secs: timeout_secs,
        encoding: encoding.to_string(),
        ..SendOptions::default()
    })
    .await
}

/// Send an HL7 message via MLLP with full control over timeouts, charset
/// and framing bytes. This is what the `mllp_send` IPC command drives —
/// the UI exposes every knob in "Advanced MLLP Settings".
pub async fn send_with_options(
    host: &str,
    port: u16,
    message: &str,
    opts: &SendOptions,
) -> MllpSendResult {
    let start = Instant::now();
    let addr = format!("{}:{}", host, port);
    let wire = resolve_send_encoding(message, &opts.encoding, opts.source_charset.as_deref());
    let encoding = wire.as_str();

    let connect_result = tokio::time::timeout(
        Duration::from_secs(opts.connect_timeout_secs),
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
                encoding: wire.clone(),
            ack_code: None,
            error: Some(format!("Connection failed: {}", e)),
            };
        }
        Err(_) => {
            return MllpSendResult {
                success: false,
                response: String::new(),
                response_time_ms: start.elapsed().as_millis() as u64,
                encoding: wire.clone(),
            ack_code: None,
            error: Some("Connection timed out".into()),
            };
        }
    };

    // Send framed message — re-encode to the user-selected charset before
    // wrapping in the (possibly non-standard) framing bytes.
    let payload = encode_with_label(message, encoding);
    let mut framed = Vec::with_capacity(payload.len() + 3);
    framed.push(opts.start_byte);
    framed.extend_from_slice(&payload);
    framed.push(opts.end_byte_1);
    framed.push(opts.end_byte_2);
    // The response timeout also bounds the write: a peer that accepts the
    // connection but never reads fills the socket buffers, and an unbounded
    // write_all would then wait forever. It is an inactivity timeout: it
    // restarts after every write the socket accepts, so a large message on
    // a slow link is never cut off while it is still moving.
    let write_timeout = Duration::from_secs(opts.response_timeout_secs);
    let mut written = 0usize;
    while written < framed.len() {
        let error = match tokio::time::timeout(write_timeout, stream.write(&framed[written..])).await {
            Ok(Ok(0)) => "Send failed: the peer closed the connection".to_string(),
            Ok(Ok(n)) => {
                written += n;
                continue;
            }
            Ok(Err(e)) => format!("Send failed: {}", e),
            Err(_) => format!(
                "Send timed out: the peer accepted no data for {} s",
                opts.response_timeout_secs
            ),
        };
        return MllpSendResult {
            success: false,
            response: String::new(),
            response_time_ms: start.elapsed().as_millis() as u64,
            encoding: wire.clone(),
            ack_code: None,
            error: Some(error),
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

    let read_status = tokio::time::timeout(Duration::from_secs(opts.response_timeout_secs), async {
        loop {
            match stream.read(&mut chunk).await {
                Ok(0) => return Ok::<ReadOutcome, std::io::Error>(ReadOutcome::Eof),
                Ok(n) => {
                    response_bytes.extend_from_slice(&chunk[..n]);
                    let len = response_bytes.len();
                    if len >= 2
                        && response_bytes[len - 2] == opts.end_byte_1
                        && response_bytes[len - 1] == opts.end_byte_2
                    {
                        return Ok(ReadOutcome::Terminator);
                    }
                    if len > MAX_ACK_BYTES {
                        return Ok(ReadOutcome::CapExceeded);
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

    // Decode ACK bytes using the same encoding the user selected for send
    // (the peer typically echoes back the same charset it received); in
    // automatic mode the ACK's own MSH-18 and bytes decide.
    let response = if response_bytes.is_empty() {
        String::new()
    } else {
        let mut start_idx = 0usize;
        let mut end_idx = response_bytes.len();
        if response_bytes[0] == opts.start_byte { start_idx = 1; }
        if end_idx > start_idx && response_bytes[end_idx - 1] == opts.end_byte_2 { end_idx -= 1; }
        if end_idx > start_idx && response_bytes[end_idx - 1] == opts.end_byte_1 { end_idx -= 1; }
        let body = &response_bytes[start_idx..end_idx];
        // Sent as UTF-16 on request, answered in an 8-bit charset (no NUL
        // byte, which UTF-16 text in the ASCII range always has): read the
        // ACK by its own bytes, or no MSA is found in it.
        let ascii_reply_to_utf16 = crate::parser::hl7::charset::is_utf16(encoding) && !body.contains(&0);
        decode_with_label(body, if is_auto(&opts.encoding) || ascii_reply_to_utf16 { AUTO } else { encoding })
    };
    let response_time_ms = start.elapsed().as_millis() as u64;

    match read_status {
        Ok(Ok(ReadOutcome::Terminator)) => MllpSendResult {
            success: true,
            ack_code: crate::parser::hl7::ack::ack_code_of(&response),
            response,
            response_time_ms,
            error: None,
            encoding: wire,
        },
        Ok(Ok(ReadOutcome::Eof)) if !response_bytes.is_empty() => MllpSendResult {
            success: true,
            ack_code: crate::parser::hl7::ack::ack_code_of(&response),
            response,
            response_time_ms,
            error: None,
            encoding: wire,
        },
        Ok(Ok(ReadOutcome::Eof)) => MllpSendResult {
            success: false,
            response: String::new(),
            response_time_ms,
            encoding: wire.clone(),
            ack_code: None,
            error: Some("Empty response (connection closed by peer)".into()),
        },
        Ok(Ok(ReadOutcome::CapExceeded)) => MllpSendResult {
            success: false,
            response: String::new(),
            response_time_ms,
            encoding: wire.clone(),
            ack_code: None,
            error: Some(format!(
                "Response exceeded {} bytes without MLLP terminator; aborted",
                MAX_ACK_BYTES
            )),
        },
        Ok(Err(e)) => MllpSendResult {
            success: false,
            response,
            response_time_ms,
            encoding: wire.clone(),
            ack_code: None,
            error: Some(format!("Read failed: {}", e)),
        },
        Err(_) => MllpSendResult {
            success: false,
            response,
            response_time_ms,
            encoding: wire.clone(),
            ack_code: None,
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
        let ack_msg = crate::parser::hl7::ack::ack_for(&content, "AA", None);
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

    /// Italian hospitals and many EU integration engines emit HL7 v2 in
    /// ISO-8859-1 / Windows-1252 without setting MSH-18. The unframer must
    /// not reject those messages — fallback to Latin-1 keeps the bytes
    /// round-trippable as Unicode codepoints.
    #[test]
    fn test_mllp_unframe_latin1_fallback() {
        // "Forlì" in Latin-1: 0x46 0x6F 0x72 0x6C 0xEC ('ì' is 0xEC in Latin-1
        // but 0xC3 0xAC in UTF-8). 0xEC alone is invalid UTF-8.
        let bytes = vec![
            MLLP_START, b'P', b'I', b'D', b'|', b'|', b'|',
            b'F', b'o', b'r', b'l', 0xEC,
            MLLP_END_1, MLLP_END_2,
        ];
        let unframed = mllp_unframe(&bytes).expect("must not return None");
        assert_eq!(unframed, "PID|||Forlì");
    }

    #[test]
    fn automatic_send_encoding_follows_msh18_then_the_file() {
        let declared = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5|||||ITA|8859/1\rPID|1||1||Müller\r";
        let plain = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||1||Müller\r";
        assert_eq!(resolve_send_encoding(declared, AUTO, None), "ISO-8859-1");
        assert_eq!(resolve_send_encoding(declared, "AUTO", Some("windows-1252")), "ISO-8859-1", "MSH-18 wins");
        assert_eq!(resolve_send_encoding(plain, AUTO, Some("windows-1252")), "windows-1252");
        assert_eq!(resolve_send_encoding(plain, AUTO, None), "UTF-8");
        assert_eq!(resolve_send_encoding(declared, "UTF-8", None), "UTF-8", "an explicit choice is kept");
        assert_eq!(resolve_send_encoding(declared, "", None), "UTF-8", "empty stays UTF-8 (CLI)");
        assert_eq!(resolve_send_encoding("MSH|^~\\&|A|B|C|D|E||F|1|P|2.5|||||X|ASCII\r", AUTO, None), "ASCII");
    }

    #[test]
    fn characters_a_charset_cannot_hold_are_named() {
        assert!(unencodable_chars("Müller Zoë", "ISO-8859-1").is_empty());
        assert_eq!(unencodable_chars("Łódź Łukasz", "ISO-8859-1"), vec!['Ł', 'ź']);
        assert_eq!(unencodable_chars("Müller?", "ASCII"), vec!['ü']);
        assert!(unencodable_chars("日本", "UTF-8").is_empty());
    }

    #[test]
    fn automatic_decoding_prefers_msh18_then_utf8_then_latin1() {
        assert_eq!(decode_auto("MSH|^~\\&|Müller".as_bytes()), ("MSH|^~\\&|Müller".into(), "UTF-8".into()));
        assert_eq!(decode_auto(b"MSH|^~\\&|M\xfcller"), ("MSH|^~\\&|Müller".into(), "ISO-8859-1".into()));
        assert_eq!(decode_with_label(b"MSH|^~\\&|M\xfcller", ""), "MSH|^~\\&|Müller");
        assert_eq!(decode_with_label(b"MSH|^~\\&|M\xfcller", "auto"), "MSH|^~\\&|Müller");
    }

    /// A message declaring 8859/1 goes out in Latin-1 in automatic mode,
    /// not as UTF-8 bytes under that label.
    #[tokio::test]
    async fn automatic_send_writes_the_declared_charset() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let n = stream.read(&mut buf).await.unwrap();
            stream.write_all(&mllp_frame("MSH|^~\\&|R||S||20260101||ACK|A1|P|2.5\rMSA|AA|L1")).await.unwrap();
            buf.truncate(n);
            buf
        });
        let msg = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|L1|P|2.5|||||ITA|8859/1\rPID|1||1||Müller\r";
        let opts = SendOptions { encoding: AUTO.into(), connect_timeout_secs: 5, response_timeout_secs: 5, ..SendOptions::default() };
        let result = send_with_options("127.0.0.1", port, msg, &opts).await;
        assert!(result.success, "{:?}", result.error);
        assert_eq!(result.encoding, "ISO-8859-1");
        assert_eq!(result.ack_code.as_deref(), Some("AA"));
        let wire = peer.await.unwrap();
        assert!(wire.windows(6).any(|w| w == b"M\xfcller"), "{:?}", String::from_utf8_lossy(&wire));
    }

    /// A message read from a UTF-16 file goes out as UTF-8 (or MSH-18's
    /// charset), never as a UTF-16 frame with a byte-order mark.
    #[test]
    fn a_utf16_source_is_sent_in_the_message_charset() {
        let plain = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|L1|P|2.5\rPID|1||1||Müller\r";
        assert_eq!(resolve_send_encoding(plain, AUTO, Some("UTF-16LE")), "UTF-8");
        assert_eq!(resolve_send_encoding(plain, AUTO, Some("UTF-16BE")), "UTF-8");
        let latin = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|L1|P|2.5|||||ITA|8859/1\rPID|1||1||Müller\r";
        assert_eq!(resolve_send_encoding(latin, AUTO, Some("UTF-16LE")), "ISO-8859-1");
        assert_eq!(resolve_send_encoding(plain, "UTF-16LE", None), "UTF-16LE", "asked for explicitly");
    }

    /// Sent as UTF-16 on request, answered in ASCII: the ACK is read by its
    /// own bytes, and its AA is found.
    #[tokio::test]
    async fn an_ascii_ack_to_a_utf16_frame_is_read() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let n = stream.read(&mut buf).await.unwrap();
            stream.write_all(&mllp_frame("MSH|^~\\&|R||S||20260101||ACK|A1|P|2.5\rMSA|AA|L1")).await.unwrap();
            buf.truncate(n);
            buf
        });
        let msg = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|L1|P|2.5\rPID|1||1||Müller\r";
        let opts = SendOptions { encoding: "UTF-16LE".into(), connect_timeout_secs: 5, response_timeout_secs: 5, ..SendOptions::default() };
        let result = send_with_options("127.0.0.1", port, msg, &opts).await;
        assert!(result.success, "{:?}", result.error);
        assert_eq!(result.ack_code.as_deref(), Some("AA"), "{}", result.response);
        assert!(peer.await.unwrap().starts_with(b"\x0b\xff\xfeM\0"));
    }

    /// In automatic mode the ACK is decoded by its own charset: a request
    /// sent in Latin-1 can get a UTF-8 ACK back, and that ACK must not be
    /// read as Latin-1.
    #[tokio::test]
    async fn automatic_mode_decodes_the_ack_by_its_own_charset() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let _ = stream.read(&mut buf).await.unwrap();
            stream.write_all(&mllp_frame("MSH|^~\\&|R||S||20260101||ACK|A1|P|2.5\rMSA|AE|L1|Nome già presente\r")).await.unwrap();
        });
        let msg = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|L1|P|2.5\rPID|1||1||Müller\r";
        let opts = SendOptions {
            encoding: AUTO.into(),
            source_charset: Some("ISO-8859-1".into()),
            connect_timeout_secs: 5,
            response_timeout_secs: 5,
            ..SendOptions::default()
        };
        let result = send_with_options("127.0.0.1", port, msg, &opts).await;
        assert!(result.success, "{:?}", result.error);
        assert_eq!(result.encoding, "ISO-8859-1", "the file's charset, MSH-18 being empty");
        assert!(result.response.contains("Nome già presente"), "{}", result.response);
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

        let result = send("127.0.0.1", port, msg, 5, "").await;
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
        let result = send("127.0.0.1", port, msg, 5, "").await;
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
        let result = send("127.0.0.1", port, msg, 5, "").await;

        assert!(result.success, "send failed: {:?}", result.error);
        assert!(
            result.response.contains("MSA|AA|MSG999"),
            "expected full ACK body, got: {:?}",
            result.response
        );
    }

    /// A hostile / misbehaving peer that streams data without ever sending the
    /// MLLP terminator must not blow up client memory. The send() loop caps
    /// the response buffer at MAX_ACK_BYTES and aborts with a clear error.
    #[tokio::test]
    async fn test_mllp_send_caps_unbounded_response() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        // Peer that writes 2 MiB of junk with NO MLLP terminator.
        let _peer = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let _ = stream.read(&mut buf).await.unwrap();

            // 2 MiB chunks so we comfortably exceed MAX_ACK_BYTES (1 MiB)
            let junk = vec![b'A'; 64 * 1024];
            for _ in 0..32 {
                if stream.write_all(&junk).await.is_err() {
                    break;
                }
            }
            let _ = stream.shutdown().await;
        });

        let msg = "MSH|^~\\&|Send|SF|Recv|RF|20260423||ADT^A01|CAP001|P|2.5\rPID|||1";
        let result = send("127.0.0.1", port, msg, 10, "").await;

        assert!(!result.success, "should abort, not succeed on unbounded stream");
        let err = result.error.expect("must have an error");
        assert!(
            err.contains("exceeded") && err.contains("MLLP terminator"),
            "expected cap-exceeded error, got: {}",
            err
        );
    }

    /// Non-standard framing bytes (legacy interfaces): client must frame the
    /// outbound message with the configured bytes and detect the ACK
    /// terminator using the same bytes.
    #[tokio::test]
    async fn test_mllp_send_custom_framing() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        const START: u8 = 0x02; // STX
        const END1: u8 = 0x03; // ETX
        const END2: u8 = 0x0A; // LF

        let _peer = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let n = stream.read(&mut buf).await.unwrap();
            // Verify the client framed with the custom bytes
            assert_eq!(buf[0], START, "client must use custom start byte");
            assert_eq!(buf[n - 2], END1, "client must use custom end byte 1");
            assert_eq!(buf[n - 1], END2, "client must use custom end byte 2");

            let ack_body = "MSH|^~\\&|Recv||Send||20260610||ACK|CF01|P|2.5\rMSA|AA|CF01";
            let mut framed = Vec::new();
            framed.push(START);
            framed.extend_from_slice(ack_body.as_bytes());
            framed.push(END1);
            framed.push(END2);
            stream.write_all(&framed).await.unwrap();
            let _ = stream.read(&mut buf).await;
        });

        let opts = SendOptions {
            connect_timeout_secs: 5,
            response_timeout_secs: 5,
            start_byte: START,
            end_byte_1: END1,
            end_byte_2: END2,
            ..SendOptions::default()
        };
        let msg = "MSH|^~\\&|Send|SF|Recv|RF|20260610||ADT^A01|CF01|P|2.5\rPID|||1";
        let result = send_with_options("127.0.0.1", port, msg, &opts).await;

        assert!(result.success, "send failed: {:?}", result.error);
        assert!(result.response.contains("MSA|AA|CF01"), "got: {}", result.response);
    }

    /// The response timeout is independent from the connect timeout: a peer
    /// that accepts but never answers must trip the response timeout.
    #[tokio::test]
    async fn test_mllp_send_response_timeout_independent() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        // Peer accepts, reads, then stays silent.
        let _peer = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let _ = stream.read(&mut buf).await;
            tokio::time::sleep(Duration::from_secs(10)).await;
        });

        let opts = SendOptions {
            connect_timeout_secs: 5,
            response_timeout_secs: 1, // must trip well before the peer's 10s
            ..SendOptions::default()
        };
        let started = Instant::now();
        let result = send_with_options("127.0.0.1", port, "MSH|^~\\&|x", &opts).await;

        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some("Response timed out"));
        assert!(started.elapsed() < Duration::from_secs(4),
                "response timeout should trip at ~1s, took {:?}", started.elapsed());
    }

    /// BL-MLLP-08 equivalent: connection to a closed port fails fast with an
    /// actionable error, not a hang.
    #[tokio::test]
    async fn test_mllp_send_refused() {
        // Grab a port that we immediately release
        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);

        let result = send("127.0.0.1", port, "MSH|^~\\&|x", 2, "").await;
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    /// A peer that accepts the connection and never reads must not hang
    /// the send: the write is bounded by the response timeout too.
    #[tokio::test]
    async fn test_mllp_send_times_out_when_peer_never_reads() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (_sock, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_secs(20)).await;
        });

        // Far larger than the loopback socket buffers.
        let message = format!("MSH|^~\\&|x\rOBX|1|ED|X||{}", "A".repeat(32 * 1024 * 1024));
        let opts = SendOptions {
            connect_timeout_secs: 5,
            response_timeout_secs: 1,
            ..SendOptions::default()
        };
        let started = Instant::now();
        let result = send_with_options("127.0.0.1", port, &message, &opts).await;
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap_or("").starts_with("Send timed out"), "{:?}", result.error);
        assert!(started.elapsed() < Duration::from_secs(10), "took {:?}", started.elapsed());
    }
}
