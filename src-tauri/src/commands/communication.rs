use std::collections::HashMap;
use tauri::State;

use crate::communication::http_client::{self, HttpMethod, HttpResult};
use crate::communication::mllp::{self, MllpSendResult};
use crate::communication::mllp_listener::{ListenerConfig, ListenerState, ListenerStatus};
use crate::communication::profiles::{ConnectionProfile, HistoryEntry};
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
    db: State<'_, Database>,
) -> Result<MllpSendResult, String> {
    feature_gate::require("mllp_send")?;

    let connect_timeout = timeout_secs.unwrap_or(30);
    let opts = mllp::SendOptions {
        connect_timeout_secs: connect_timeout,
        response_timeout_secs: response_timeout_secs.unwrap_or(connect_timeout),
        encoding: encoding.unwrap_or_default(),
        start_byte: parse_framing_byte(&start_char, mllp::MLLP_START),
        end_byte_1: parse_framing_byte(&end_char1, mllp::MLLP_END_1),
        end_byte_2: parse_framing_byte(&end_char2, mllp::MLLP_END_2),
    };
    let result = mllp::send_with_options(&host, port, &message, &opts).await;

    let preview: String = message.chars().take(100).collect();
    let status = if result.success { "OK" } else { "FAILED" };
    let entry = HistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        profile_name: profile_name.unwrap_or_else(|| format!("{}:{}", host, port)),
        profile_type: "mllp".into(),
        direction: "send".into(),
        content_preview: preview,
        status: status.into(),
        response_time_ms: result.response_time_ms,
        timestamp: chrono::Utc::now().to_rfc3339(),
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
) -> Result<HttpResult, String> {
    let http_method = HttpMethod::from_str(&method)
        .ok_or_else(|| format!("Invalid HTTP method: {}", method))?;

    // GET is community; POST/PUT/DELETE/PATCH require Pro
    match http_method {
        HttpMethod::Get => feature_gate::require("http_get")?,
        _ => feature_gate::require("http_mutate")?,
    }

    // Auth headers require Pro
    let hdrs = headers.unwrap_or_default();
    if hdrs.keys().any(|k| k.to_lowercase() == "authorization") {
        feature_gate::require("http_auth")?;
    }

    let timeout = timeout_secs.unwrap_or(30);
    let result = http_client::send_request(
        &url,
        http_method,
        &hdrs,
        body.as_deref(),
        timeout,
        follow_redirects.unwrap_or(true),
    ).await;

    let preview: String = body.as_deref().unwrap_or("").chars().take(100).collect();
    let status = if result.success {
        format!("{} {}", result.status_code, result.status_text)
    } else {
        "FAILED".into()
    };
    let entry = HistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        profile_name: profile_name.unwrap_or_else(|| url.clone()),
        profile_type: "http".into(),
        direction: "send".into(),
        content_preview: format!("{} {} | {}", method.to_uppercase(), url, preview),
        status,
        response_time_ms: result.response_time_ms,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    let _ = db.add_history_entry(&entry);

    Ok(result)
}

// --- ACK Generation ---

#[tauri::command]
pub fn generate_ack(
    ack_code: String,
    message_control_id: String,
    text_message: Option<String>,
) -> Result<String, String> {
    Ok(crate::parser::hl7::ack::generate_ack(
        &ack_code,
        &message_control_id,
        "BridgeLab",
        "RemoteApp",
        text_message.as_deref(),
    ))
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
    db.get_request_history(limit.unwrap_or(50))
}

#[tauri::command]
pub fn clear_request_history(db: State<'_, Database>) -> Result<(), String> {
    db.clear_request_history()
}
