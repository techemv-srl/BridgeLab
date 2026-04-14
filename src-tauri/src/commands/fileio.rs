use serde::Serialize;
use tauri::State;

use super::parser::{parse_message, ParseResult};
use crate::message_store::MessageStore;
use crate::utils::error::BridgeLabError;

/// Open a file from disk and parse it.
#[tauri::command]
pub async fn open_file(
    path: String,
    store: State<'_, MessageStore>,
) -> Result<ParseResult, BridgeLabError> {
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| BridgeLabError::FileError(format!("Failed to read {}: {}", path, e)))?;

    parse_message(content, Some(path), store)
}

/// Save message content to a file.
#[tauri::command]
pub async fn save_file(
    message_id: String,
    path: String,
    store: State<'_, MessageStore>,
) -> Result<SaveResult, BridgeLabError> {
    let msg = store
        .get(&message_id)
        .ok_or_else(|| BridgeLabError::MessageNotFound(message_id))?;

    let content = String::from_utf8_lossy(&msg.raw);
    tokio::fs::write(&path, content.as_bytes())
        .await
        .map_err(|e| BridgeLabError::FileError(format!("Failed to write {}: {}", path, e)))?;

    Ok(SaveResult {
        path,
        bytes_written: msg.raw.len() as u64,
    })
}

#[derive(Debug, Serialize)]
pub struct SaveResult {
    pub path: String,
    pub bytes_written: u64,
}
