use serde::{Deserialize, Serialize};

/// A saved connection profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProfile {
    pub id: String,
    pub name: String,
    pub profile_type: ProfileType,
    pub host: String,
    pub port: u16,
    pub timeout_secs: u64,
    /// For HTTP: base URL, headers template
    pub url: Option<String>,
    pub headers: Option<String>,
    /// For MLLP: auto-ACK setting
    pub auto_ack: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProfileType {
    Mllp,
    Http,
}

/// A request/response history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub profile_name: String,
    pub profile_type: String,
    pub direction: String,
    pub content_preview: String,
    pub status: String,
    pub response_time_ms: u64,
    pub timestamp: String,
}
