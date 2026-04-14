//! Minimal HL7 v2.x parser for CLI use.
//! Lightweight subset of the main BridgeLab parser.

use memchr::memchr;

#[derive(Debug, Clone)]
pub struct Delimiters {
    pub field: u8,
    pub component: u8,
    pub repetition: u8,
    pub escape: u8,
    pub subcomponent: u8,
}

impl Default for Delimiters {
    fn default() -> Self {
        Self { field: b'|', component: b'^', repetition: b'~', escape: b'\\', subcomponent: b'&' }
    }
}

impl Delimiters {
    pub fn from_msh(data: &[u8]) -> Result<Self, String> {
        if data.len() < 8 || &data[..3] != b"MSH" {
            return Err("Not a valid HL7 message (missing MSH header)".into());
        }
        Ok(Self {
            field: data[3],
            component: data[4],
            repetition: data[5],
            escape: data[6],
            subcomponent: data[7],
        })
    }
}

#[derive(Debug, Clone)]
pub struct Span { pub start: usize, pub end: usize }

impl Span {
    pub fn as_str<'a>(&self, data: &'a [u8]) -> &'a str {
        std::str::from_utf8(&data[self.start..self.end]).unwrap_or("")
    }
    pub fn len(&self) -> usize { self.end - self.start }
}

#[derive(Debug, Clone)]
pub struct FieldIndex {
    pub span: Span,
    pub position: usize,
    pub is_truncated: bool,
}

#[derive(Debug, Clone)]
pub struct SegmentIndex {
    pub span: Span,
    pub segment_type: String,
    pub position: usize,
    pub fields: Vec<FieldIndex>,
}

#[derive(Debug, Clone)]
pub struct Hl7Message {
    pub raw: Vec<u8>,
    pub delimiters: Delimiters,
    pub version: String,
    pub message_type: String,
    pub segments: Vec<SegmentIndex>,
}

impl Hl7Message {
    pub fn parse(data: Vec<u8>) -> Result<Self, String> {
        if data.len() < 8 {
            return Err("Message too short".into());
        }
        let delimiters = Delimiters::from_msh(&data)?;
        let segments = index_segments(&data, &delimiters, 100);
        let version = extract_msh_field(&data, &segments, &delimiters, 12).unwrap_or_default();
        let message_type = extract_msh_field(&data, &segments, &delimiters, 9).unwrap_or_default();

        Ok(Self { raw: data, delimiters, version, message_type, segments })
    }
}

fn index_segments(data: &[u8], delim: &Delimiters, trunc_threshold: usize) -> Vec<SegmentIndex> {
    let mut segments = Vec::new();
    let mut offset = 0;
    let mut pos = 0;

    while offset < data.len() {
        let seg_end = find_line_end(data, offset);
        if seg_end > offset {
            let seg_data = &data[offset..seg_end];
            if seg_data.len() >= 3 {
                let seg_type = String::from_utf8_lossy(&seg_data[..3]).to_string();
                let fields = index_fields(data, offset, seg_end, delim, &seg_type, trunc_threshold);
                segments.push(SegmentIndex {
                    span: Span { start: offset, end: seg_end },
                    segment_type: seg_type,
                    position: pos,
                    fields,
                });
                pos += 1;
            }
        }
        offset = skip_newline(data, seg_end);
    }
    segments
}

fn find_line_end(data: &[u8], start: usize) -> usize {
    let cr = memchr(b'\r', &data[start..]).map(|i| start + i);
    let lf = memchr(b'\n', &data[start..]).map(|i| start + i);
    match (cr, lf) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => data.len(),
    }
}

fn skip_newline(data: &[u8], offset: usize) -> usize {
    let mut i = offset;
    while i < data.len() && (data[i] == b'\r' || data[i] == b'\n') {
        i += 1;
    }
    i
}

fn index_fields(data: &[u8], seg_start: usize, seg_end: usize, delim: &Delimiters, seg_type: &str, trunc: usize) -> Vec<FieldIndex> {
    let mut fields = Vec::new();
    let seg_bytes = &data[seg_start..seg_end];
    let is_msh = seg_type == "MSH";

    // MSH has special handling: MSH-1 = field separator, MSH-2 = encoding chars
    if is_msh {
        // MSH-1 is the separator itself at position 3
        if seg_bytes.len() > 3 {
            fields.push(FieldIndex {
                span: Span { start: seg_start + 3, end: seg_start + 4 },
                position: 1,
                is_truncated: false,
            });
        }
        // MSH-2 is the 4 encoding chars after MSH|
        if seg_bytes.len() > 7 {
            fields.push(FieldIndex {
                span: Span { start: seg_start + 4, end: seg_start + 8 },
                position: 2,
                is_truncated: false,
            });
        }

        let mut pos = 3;
        let mut cursor = seg_start + 8; // Past "MSH|^~\&"
        if cursor < seg_end && data[cursor] == delim.field {
            cursor += 1;
        }
        while cursor <= seg_end {
            let next = find_delim(data, cursor, seg_end, delim.field);
            let field_len = next - cursor;
            fields.push(FieldIndex {
                span: Span { start: cursor, end: next },
                position: pos,
                is_truncated: field_len > trunc,
            });
            pos += 1;
            cursor = next + 1;
            if next >= seg_end { break; }
        }
    } else {
        // Non-MSH: split by | starting after segment type
        let first_pipe = find_delim(data, seg_start + 3, seg_end, delim.field);
        if first_pipe >= seg_end { return fields; }
        let mut cursor = first_pipe + 1;
        let mut pos = 1;
        while cursor <= seg_end {
            let next = find_delim(data, cursor, seg_end, delim.field);
            let field_len = next - cursor;
            fields.push(FieldIndex {
                span: Span { start: cursor, end: next },
                position: pos,
                is_truncated: field_len > trunc,
            });
            pos += 1;
            cursor = next + 1;
            if next >= seg_end { break; }
        }
    }
    fields
}

fn find_delim(data: &[u8], start: usize, end: usize, d: u8) -> usize {
    let slice = &data[start..end];
    memchr(d, slice).map(|i| start + i).unwrap_or(end)
}

fn extract_msh_field(data: &[u8], segments: &[SegmentIndex], _delim: &Delimiters, field_pos: usize) -> Option<String> {
    let msh = segments.iter().find(|s| s.segment_type == "MSH")?;
    let field = msh.fields.iter().find(|f| f.position == field_pos)?;
    Some(field.span.as_str(data).to_string())
}
