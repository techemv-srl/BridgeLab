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
    /// SOAP endpoint profiles (Enterprise): `url` holds the endpoint,
    /// `headers` holds the SOAPAction.
    Soap,
}

/// How many history entries are kept: older ones are deleted as new ones
/// are added.
pub const HISTORY_KEPT: usize = 100;

/// Longest request or response text kept in a history entry, in bytes; a
/// longer one is cut and says so. Real HL7 messages and ACKs are a few KB,
/// and a message can reach 10 MB.
pub const HISTORY_TEXT_CAP: usize = 256 * 1024;

/// `text` as stored in the history: whole up to [`HISTORY_TEXT_CAP`],
/// otherwise cut at a character boundary with a note of the full size.
pub fn history_text(text: &str) -> String {
    if text.len() <= HISTORY_TEXT_CAP {
        return text.to_string();
    }
    let mut end = HISTORY_TEXT_CAP;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n… (cut: {} bytes in all)", &text[..end], text.len())
}

/// A request/response history entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub profile_name: String,
    pub profile_type: String,
    pub direction: String,
    pub content_preview: String,
    pub status: String,
    pub response_time_ms: u64,
    pub timestamp: String,
    /// MSA-1 of the acknowledgment an MLLP send received ("AA", "AE",
    /// "AR", or a commit-mode code). None when there was no ACK to read:
    /// a failed send, or an HTTP/SOAP request.
    #[serde(default)]
    pub ack_code: Option<String>,
    /// Where it went or came from: `host:port` for MLLP (the peer's
    /// address for a received message), the URL for HTTP and SOAP (with
    /// any credentials redacted). Empty in entries from older versions.
    #[serde(default)]
    pub target: String,
    /// Size of the message sent or received, in bytes.
    #[serde(default)]
    pub size_bytes: u64,
    /// The message sent or received (see [`history_text`]).
    #[serde(default)]
    pub request: String,
    /// The reply: the ACK received or sent back, the HTTP response body or
    /// the SOAP Body or Fault.
    #[serde(default)]
    pub response: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_history_text_is_cut_at_a_character_boundary() {
        assert_eq!(history_text("MSH|x"), "MSH|x");
        let long = "é".repeat(HISTORY_TEXT_CAP);
        let kept = history_text(&long);
        assert!(kept.len() < long.len());
        assert!(kept.ends_with(&format!("(cut: {} bytes in all)", long.len())));
    }
}
