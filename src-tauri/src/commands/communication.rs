use std::collections::HashMap;
use tauri::State;

use crate::communication::credentials::{is_auth_header, redact_credentials, url_has_credentials};
use crate::communication::http_client::{self, HttpMethod, HttpResult};
use crate::communication::mllp::{self, MllpSendResult};
use crate::communication::mllp_listener::{ListenerConfig, ListenerState, ListenerStatus};
use crate::communication::profiles::{history_text, ConnectionProfile, HistoryEntry, HISTORY_KEPT};
use crate::database::Database;
use crate::licensing::feature_gate;

// --- MLLP Commands ---

/// Parse a framing-byte override like "0x0B" / "0B" (hex). Anything
/// unparsable falls back to the standard MLLP byte, so a typo in the
/// advanced settings can't silently produce unframeable traffic.
fn parse_framing_byte(input: &Option<String>, default: u8) -> u8 {
    input
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| {
            let hex = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
            u8::from_str_radix(hex, 16).ok()
        })
        .unwrap_or(default)
}

#[tauri::command]
pub async fn mllp_send(
    host: String,
    port: u16,
    message: String,
    timeout_secs: Option<u64>,
    response_timeout_secs: Option<u64>,
    encoding: Option<String>,
    start_char: Option<String>,
    end_char1: Option<String>,
    end_char2: Option<String>,
    profile_name: Option<String>,
    source_charset: Option<String>,
    db: State<'_, Database>,
    tel: State<'_, crate::licensing::telemetry::UsageCounters>,
) -> Result<MllpSendResult, String> {
    feature_gate::require("mllp_send")?;
    tel.bump_mem("mllp_sent");

    // Segments end with CR on the wire, whatever the editor's line endings.
    let message = crate::parser::hl7::to_wire_segments(&message).into_owned();
    // "auto" (the panel's default, and a missing value) sends in the
    // charset MSH-18 declares, else the file's, else UTF-8.
    let requested = encoding.filter(|e| !e.trim().is_empty()).unwrap_or_else(|| mllp::AUTO.into());
    let wire = mllp::resolve_send_encoding(&message, &requested, source_charset.as_deref());
    if !crate::parser::hl7::charset::is_known_label(&wire) {
        return Err(format!("Unknown encoding '{}'", wire));
    }
    let lost = mllp::unencodable_chars(&message, &wire);
    if !lost.is_empty() {
        // Refused, not sent with '?' in place of a patient's name.
        let shown = lost.iter().take(8).map(char::to_string).collect::<Vec<_>>().join(" ");
        return Ok(MllpSendResult {
            success: false,
            response: String::new(),
            response_time_ms: 0,
            error: Some(format!(
                "Not sent: {} cannot represent {} character(s) of this message ({}). Choose an encoding that can (UTF-8), or correct MSH-18.",
                wire,
                lost.len(),
                shown
            )),
            encoding: wire,
            ack_code: None,
        });
    }

    let connect_timeout = timeout_secs.unwrap_or(30);
    let opts = mllp::SendOptions {
        connect_timeout_secs: connect_timeout,
        response_timeout_secs: response_timeout_secs.unwrap_or(connect_timeout),
        // Kept as asked ("auto"): send_with_options resolves the same wire
        // charset, and in automatic mode decodes the ACK by its own MSH-18
        // and bytes rather than by the request's charset.
        encoding: requested,
        source_charset: source_charset.clone(),
        start_byte: parse_framing_byte(&start_char, mllp::MLLP_START),
        end_byte_1: parse_framing_byte(&end_char1, mllp::MLLP_END_1),
        end_byte_2: parse_framing_byte(&end_char2, mllp::MLLP_END_2),
    };
    let size_bytes = mllp::encode_with_label(&message, &wire).len() as u64;
    let result = mllp::send_with_options(&host, port, &message, &opts).await;

    let status = if result.success { "OK" } else { "FAILED" };
    // What the receiver answered, so the history can be filtered by
    // outcome: a send that reached the peer and got an AE back is "OK" at
    // the transport level and a rejection at the application level.
    let ack_code = result.ack_code.clone();
    let target = format!("{}:{}", host, port);
    let entry = HistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        profile_name: profile_name.unwrap_or_else(|| target.clone()),
        profile_type: "mllp".into(),
        direction: "send".into(),
        content_preview: String::new(),
        status: status.into(),
        response_time_ms: result.response_time_ms,
        timestamp: chrono::Utc::now().to_rfc3339(),
        ack_code,
        target,
        size_bytes,
        request: history_text(&message),
        response: history_text(&result.response),
    };
    let _ = db.add_history_entry(&entry);

    Ok(result)
}

// --- Persistent MLLP listener (start / stop / status) -------------------------
//
// The listener keeps accepting connections and emits a Tauri event
// (`mllp:received`) for each incoming message until the user calls stop. This
// is what the Communication panel's Listen / Stop button drives.
// (The old single-shot `mllp_receive` IPC command was removed in 0.7.0 — it
// was never reachable from the UI once the persistent listener shipped. The
// underlying `mllp::receive_one` stays as a tested library primitive.)

#[tauri::command]
pub async fn mllp_listen_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, ListenerState>,
    config: ListenerConfig,
) -> Result<ListenerStatus, String> {
    feature_gate::require("mllp_listen")?;
    state.start(app, config).await
}

#[tauri::command]
pub async fn mllp_listen_stop(
    state: tauri::State<'_, ListenerState>,
) -> Result<ListenerStatus, String> {
    Ok(state.stop().await)
}

#[tauri::command]
pub async fn mllp_listen_status(
    state: tauri::State<'_, ListenerState>,
) -> Result<ListenerStatus, String> {
    Ok(state.status().await)
}

// --- HTTP Commands ---

#[tauri::command]
pub async fn http_request(
    url: String,
    method: String,
    headers: Option<HashMap<String, String>>,
    body: Option<String>,
    timeout_secs: Option<u64>,
    follow_redirects: Option<bool>,
    profile_name: Option<String>,
    db: State<'_, Database>,
    tel: State<'_, crate::licensing::telemetry::UsageCounters>,
) -> Result<HttpResult, String> {
    tel.bump_mem("http_requests");
    let http_method = HttpMethod::from_str(&method)
        .ok_or_else(|| format!("Invalid HTTP method: {}", method))?;

    // GET is community; POST/PUT/DELETE/PATCH require Pro
    match http_method {
        HttpMethod::Get => feature_gate::require("http_get")?,
        _ => feature_gate::require("http_mutate")?,
    }

    // Authentication of any kind is Pro: an Authorization header, any
    // header that carries a key, token, secret, signature, session or
    // cookie, user:password in the URL (which the client turns into an
    // Authorization header itself) and credentials in the query string.
    let hdrs = headers.unwrap_or_default();
    if hdrs.iter().any(|(k, v)| is_auth_header(k, v)) || url_has_credentials(&url) {
        feature_gate::require("http_auth")?;
    }

    // An HL7 v2 body goes out with CR segment terminators, as over MLLP.
    let body = body.map(|b| crate::parser::hl7::to_wire_segments(&b).into_owned());
    let timeout = timeout_secs.unwrap_or(30);
    let result = http_client::send_request(
        &url,
        http_method,
        &hdrs,
        body.as_deref(),
        timeout,
        follow_redirects.unwrap_or(true),
    ).await;

    // A server that answered, with whatever status, is not a failed
    // request: a 404 or a 500 is shown with its code.
    let status = if result.status_code > 0 {
        format!("{} {}", result.status_code, result.status_text).trim_end().to_string()
    } else {
        "FAILED".into()
    };
    // Never keep a password or an API key in the history.
    let target = redact_credentials(&url);
    let body_text = body.as_deref().unwrap_or("");
    let entry = HistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        profile_name: profile_name.unwrap_or_else(|| target.clone()),
        profile_type: "http".into(),
        direction: "send".into(),
        content_preview: format!("{} {}", method.to_uppercase(), target),
        status,
        response_time_ms: result.response_time_ms,
        timestamp: chrono::Utc::now().to_rfc3339(),
        ack_code: None,
        target,
        size_bytes: body_text.len() as u64,
        request: history_text(body_text),
        response: history_text(&result.body),
    };
    let _ = db.add_history_entry(&entry);

    Ok(result)
}

// --- ACK Generation ---

/// The acknowledgment of `message`, mirroring its header (see
/// [`crate::parser::hl7::ack::ack_for`]). Refused when the message has no
/// MSH-10: an ACK that cannot be correlated is no use to the sender.
#[tauri::command]
pub fn generate_ack(
    ack_code: String,
    message: String,
    text_message: Option<String>,
) -> Result<String, String> {
    use crate::parser::hl7::ack;
    if ack::extract_message_control_id(&message).is_none() {
        return Err("No Message Control ID (MSH-10) found in the message".into());
    }
    Ok(ack::ack_for(&message, &ack_code, text_message.as_deref()))
}

// --- Connection Profiles ---

#[tauri::command]
pub fn save_connection_profile(
    profile: ConnectionProfile,
    db: State<'_, Database>,
) -> Result<(), String> {
    db.save_connection_profile(&profile)
}

#[tauri::command]
pub fn get_connection_profiles(db: State<'_, Database>) -> Result<Vec<ConnectionProfile>, String> {
    db.get_connection_profiles()
}

#[tauri::command]
pub fn delete_connection_profile(id: String, db: State<'_, Database>) -> Result<(), String> {
    db.delete_connection_profile(&id)
}

#[tauri::command]
pub fn get_request_history(
    limit: Option<usize>,
    db: State<'_, Database>,
) -> Result<Vec<HistoryEntry>, String> {
    db.get_request_history(limit.unwrap_or(HISTORY_KEPT))
}

#[tauri::command]
pub fn clear_request_history(db: State<'_, Database>) -> Result<(), String> {
    db.clear_request_history()
}
