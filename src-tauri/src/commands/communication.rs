use std::collections::HashMap;
use tauri::State;

use crate::communication::http_client::{self, HttpMethod, HttpResult};
use crate::communication::mllp::{self, MllpSendResult, MllpReceivedMessage};
use crate::communication::profiles::{ConnectionProfile, HistoryEntry};
use crate::database::Database;
use crate::licensing::feature_gate;

// --- MLLP Commands ---

#[tauri::command]
pub async fn mllp_send(
    host: String,
    port: u16,
    message: String,
    timeout_secs: Option<u64>,
    profile_name: Option<String>,
    db: State<'_, Database>,
) -> Result<MllpSendResult, String> {
    feature_gate::require("mllp_send")?;

    let timeout = timeout_secs.unwrap_or(30);
    let result = mllp::send(&host, port, &message, timeout).await;

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

#[tauri::command]
pub async fn mllp_receive(
    port: u16,
    timeout_secs: Option<u64>,
    auto_ack: Option<bool>,
) -> Result<MllpReceivedMessage, String> {
    feature_gate::require("mllp_listen")?;

    let timeout = timeout_secs.unwrap_or(60);
    let ack = auto_ack.unwrap_or(true);
    mllp::receive_one(port, timeout, ack).await
}

// --- HTTP Commands ---

#[tauri::command]
pub async fn http_request(
    url: String,
    method: String,
    headers: Option<HashMap<String, String>>,
    body: Option<String>,
    timeout_secs: Option<u64>,
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
