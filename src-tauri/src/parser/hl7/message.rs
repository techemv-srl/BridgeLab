use serde::Serialize;

use super::delimiters::Delimiters;

/// A byte range within the raw message buffer.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Extract the content as a string slice from the raw buffer.
    pub fn as_str<'a>(&self, data: &'a [u8]) -> &'a str {
        std::str::from_utf8(&data[self.start..self.end]).unwrap_or("<invalid utf8>")
    }
}

/// Index for a subcomponent within a component.
#[derive(Debug, Clone, Serialize)]
pub struct SubComponentIndex {
    pub span: Span,
}

/// Index for a component within a field.
#[derive(Debug, Clone, Serialize)]
pub struct ComponentIndex {
    pub span: Span,
    pub subcomponents: Vec<SubComponentIndex>,
}

/// Index for a field repetition.
#[derive(Debug, Clone, Serialize)]
pub struct RepetitionIndex {
    pub span: Span,
    pub components: Vec<ComponentIndex>,
}

/// Index for a field within a segment.
#[derive(Debug, Clone, Serialize)]
pub struct FieldIndex {
    pub span: Span,
    /// Field position (0-based within segment, but HL7 fields are 1-based in display)
    pub position: usize,
    /// Repetitions of this field (most fields have exactly one)
    pub repetitions: Vec<RepetitionIndex>,
    /// Whether this field content exceeds truncation threshold
    pub is_truncated: bool,
}

/// Index for a segment within the message.
#[derive(Debug, Clone, Serialize)]
pub struct SegmentIndex {
    pub span: Span,
    /// Segment type (e.g., "MSH", "PID", "OBX")
    pub segment_type: String,
    /// 0-based position in the message
    pub position: usize,
    /// Fields within this segment
    pub fields: Vec<FieldIndex>,
}

/// The parsed representation of an HL7 v2.x message.
/// Stores byte-offsets into the raw buffer instead of copying strings.
#[derive(Debug, Clone)]
pub struct Hl7Message {
    /// The raw message content
    pub raw: Vec<u8>,
    /// Detected delimiters
    pub delimiters: Delimiters,
    /// Detected HL7 version (from MSH-12)
    pub version: String,
    /// Detected message type (from MSH-9, e.g. "ADT^A01")
    pub message_type: String,
    /// Segment index (flat structure for fast access)
    pub segments: Vec<SegmentIndex>,
}

/// Tree node for the frontend tree view.
#[derive(Debug, Clone, Serialize)]
pub struct TreeNode {
    pub id: String,
    pub label: String,
    pub value_preview: String,
    pub node_type: TreeNodeType,
    pub depth: u32,
    pub has_children: bool,
    pub is_truncated: bool,
    pub child_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TreeNodeType {
    Message,
    Segment,
    Field,
    Component,
    Subcomponent,
}
