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
/// If `content` is provided, it is written directly (use when editor text differs from stored).
/// Otherwise falls back to the MessageStore content using `message_id`.
#[tauri::command]
pub async fn save_file(
    message_id: Option<String>,
    path: String,
    content: Option<String>,
    store: State<'_, MessageStore>,
) -> Result<SaveResult, BridgeLabError> {
    let bytes: Vec<u8> = if let Some(c) = content {
        c.into_bytes()
    } else if let Some(id) = message_id {
        let msg = store
            .get(&id)
            .ok_or_else(|| BridgeLabError::MessageNotFound(id))?;
        msg.raw.clone()
    } else {
        return Err(BridgeLabError::FileError(
            "Either content or message_id must be provided".into(),
        ));
    };

    let bytes_written = bytes.len() as u64;
    tokio::fs::write(&path, &bytes)
        .await
        .map_err(|e| BridgeLabError::FileError(format!("Failed to write {}: {}", path, e)))?;

    Ok(SaveResult { path, bytes_written })
}

#[derive(Debug, Serialize)]
pub struct SaveResult {
    pub path: String,
    pub bytes_written: u64,
}
