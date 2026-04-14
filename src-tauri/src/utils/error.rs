use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum BridgeLabError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("File error: {0}")]
    FileError(String),

    #[error("Message not found: {0}")]
    MessageNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl Serialize for BridgeLabError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
