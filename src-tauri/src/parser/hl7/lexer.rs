use memchr::memchr;

use super::delimiters::Delimiters;
use super::message::*;

/// Default truncation threshold in bytes.
const DEFAULT_TRUNCATION_THRESHOLD: usize = 100;

/// Streaming HL7 v2.x parser.
/// Uses memchr for SIMD-accelerated delimiter scanning.
/// Produces a flat index of byte-offsets into the raw buffer.
pub struct Hl7Lexer {
    truncation_threshold: usize,
}

impl Hl7Lexer {
    pub fn new() -> Self {
        Self {
            truncation_threshold: DEFAULT_TRUNCATION_THRESHOLD,
        }
    }

    pub fn with_truncation_threshold(mut self, threshold: usize) -> Self {
        self.truncation_threshold = threshold;
        self
    }

    /// Parse a raw HL7 message into an Hl7Message with indexed segments.
    pub fn parse(&self, data: Vec<u8>) -> Result<Hl7Message, String> {
        if data.len() < 8 {
            return Err("Message too short".to_string());
        }

        let delimiters = Delimiters::from_msh(&data)
            .map_err(|e| e.to_string())?;

        let segments = self.index_segments(&data, &delimiters);

        let version = self.extract_msh_field(&data, &segments, &delimiters, 12)
            .unwrap_or_default();
        let message_type = self.extract_msh_field(&data, &segments, &delimiters, 9)
            .unwrap_or_default();

        Ok(Hl7Message {
            raw: data,
            delimiters,
            version,
            message_type,
            segments,
        })
    }

    /// Index all segments in the message using SIMD-accelerated newline scanning.
    fn index_segments(&self, data: &[u8], delimiters: &Delimiters) -> Vec<SegmentIndex> {
        let mut segments = Vec::new();
        let mut offset = 0;
        let mut seg_position = 0;

        while offset < data.len() {
            // Find end of segment (CR, LF, or CRLF)
            let seg_end = self.find_segment_end(data, offset);
            let seg_data = &data[offset..seg_end];

            if !seg_data.is_empty() {
                let segment = self.index_one_segment(data, offset, seg_end, delimiters, seg_position);
                segments.push(segment);
                seg_position += 1;
            }

            // Skip past the line ending
            offset = self.skip_line_ending(data, seg_end);
        }

        segments
    }

    /// Find the end of the current segment (before CR/LF).
    fn find_segment_end(&self, data: &[u8], start: usize) -> usize {
        let remaining = &data[start..];
        // Search for CR or LF using memchr
        let cr_pos = memchr(b'\r', remaining);
        let lf_pos = memchr(b'\n', remaining);

        match (cr_pos, lf_pos) {
            (Some(cr), Some(lf)) => start + cr.min(lf),
            (Some(cr), None) => start + cr,
            (None, Some(lf)) => start + lf,
            (None, None) => data.len(),
        }
    }

    /// Skip past CR, LF, or CRLF at the given position.
    fn skip_line_ending(&self, data: &[u8], pos: usize) -> usize {
        if pos >= data.len() {
            return data.len();
        }
        if data[pos] == b'\r' {
            if pos + 1 < data.len() && data[pos + 1] == b'\n' {
                pos + 2
            } else {
                pos + 1
            }
        } else if data[pos] == b'\n' {
            pos + 1
        } else {
            pos + 1
        }
    }

    /// Index a single segment into fields, components, and subcomponents.
    fn index_one_segment(
        &self,
        data: &[u8],
        seg_start: usize,
        seg_end: usize,
        delimiters: &Delimiters,
        position: usize,
    ) -> SegmentIndex {
        let seg_bytes = &data[seg_start..seg_end];

        // Extract segment type (first 3 chars)
        let type_end = seg_bytes.len().min(3);
        let segment_type = String::from_utf8_lossy(&seg_bytes[..type_end]).to_string();

        let is_msh = segment_type == "MSH";

        // Find fields within segment
        let fields = self.index_fields(data, seg_start, seg_end, delimiters, is_msh);

        SegmentIndex {
            span: Span::new(seg_start, seg_end),
            segment_type,
            position,
            fields,
        }
    }

    /// Index fields within a segment using SIMD field separator scanning.
    fn index_fields(
        &self,
        data: &[u8],
        seg_start: usize,
        seg_end: usize,
        delimiters: &Delimiters,
        is_msh: bool,
    ) -> Vec<FieldIndex> {
        let mut fields = Vec::new();
        let seg_bytes = &data[seg_start..seg_end];

        if is_msh {
            // MSH special handling:
            // MSH-1 is the field separator itself (single char at position 3)
            // MSH-2 is the encoding characters (positions 4-7)
            if seg_bytes.len() > 3 {
                fields.push(FieldIndex {
                    span: Span::new(seg_start + 3, seg_start + 4),
                    position: 1,
                    repetitions: vec![RepetitionIndex {
                        span: Span::new(seg_start + 3, seg_start + 4),
                        components: vec![ComponentIndex {
                            span: Span::new(seg_start + 3, seg_start + 4),
                            subcomponents: Vec::new(),
                        }],
                    }],
                    is_truncated: false,
                });
            }
            if seg_bytes.len() > 4 {
                let enc_end = (seg_start + 8).min(seg_end);
                let next_field = memchr(delimiters.field, &data[seg_start + 4..seg_end])
                    .map(|p| seg_start + 4 + p)
                    .unwrap_or(enc_end);
                let actual_end = next_field.min(enc_end);
                fields.push(FieldIndex {
                    span: Span::new(seg_start + 4, actual_end),
                    position: 2,
                    repetitions: vec![RepetitionIndex {
                        span: Span::new(seg_start + 4, actual_end),
                        components: vec![ComponentIndex {
                            span: Span::new(seg_start + 4, actual_end),
                            subcomponents: Vec::new(),
                        }],
                    }],
                    is_truncated: false,
                });
            }
            // Parse remaining fields starting after MSH-2
            let remaining_start = if seg_bytes.len() > 8 {
                // Find the field separator after encoding chars
                memchr(delimiters.field, &data[seg_start + 4..seg_end])
                    .map(|p| seg_start + 4 + p + 1)
                    .unwrap_or(seg_end)
            } else {
                seg_end
            };
            self.parse_fields_from(data, remaining_start, seg_end, delimiters, 3, &mut fields);
        } else {
            // Non-MSH segments: skip past segment type and first field separator
            let field_start = memchr(delimiters.field, seg_bytes)
                .map(|p| seg_start + p + 1)
                .unwrap_or(seg_end);
            self.parse_fields_from(data, field_start, seg_end, delimiters, 1, &mut fields);
        }

        fields
    }

    /// Parse fields starting from a given offset, splitting by field separator.
    fn parse_fields_from(
        &self,
        data: &[u8],
        start: usize,
        end: usize,
        delimiters: &Delimiters,
        start_position: usize,
        fields: &mut Vec<FieldIndex>,
    ) {
        let mut offset = start;
        let mut position = start_position;

        while offset < end {
            let field_end = memchr(delimiters.field, &data[offset..end])
                .map(|p| offset + p)
                .unwrap_or(end);

            let span = Span::new(offset, field_end);
            let is_truncated = span.len() > self.truncation_threshold;

            let repetitions = self.index_repetitions(data, offset, field_end, delimiters);

            fields.push(FieldIndex {
                span,
                position,
                repetitions,
                is_truncated,
            });

            offset = field_end + 1;
            position += 1;
        }
    }

    /// Index repetitions within a field.
    fn index_repetitions(
        &self,
        data: &[u8],
        start: usize,
        end: usize,
        delimiters: &Delimiters,
    ) -> Vec<RepetitionIndex> {
        let mut reps = Vec::new();
        let mut offset = start;

        loop {
            let rep_end = memchr(delimiters.repetition, &data[offset..end])
                .map(|p| offset + p)
                .unwrap_or(end);

            let components = self.index_components(data, offset, rep_end, delimiters);
            reps.push(RepetitionIndex {
                span: Span::new(offset, rep_end),
                components,
            });

            if rep_end >= end {
                break;
            }
            offset = rep_end + 1;
        }

        reps
    }

    /// Index components within a repetition.
    fn index_components(
        &self,
        data: &[u8],
        start: usize,
        end: usize,
        delimiters: &Delimiters,
    ) -> Vec<ComponentIndex> {
        let mut comps = Vec::new();
        let mut offset = start;

        loop {
            let comp_end = memchr(delimiters.component, &data[offset..end])
                .map(|p| offset + p)
                .unwrap_or(end);

            let subcomponents = self.index_subcomponents(data, offset, comp_end, delimiters);
            comps.push(ComponentIndex {
                span: Span::new(offset, comp_end),
                subcomponents,
            });

            if comp_end >= end {
                break;
            }
            offset = comp_end + 1;
        }

        comps
    }

    /// Index subcomponents within a component.
    fn index_subcomponents(
        &self,
        data: &[u8],
        start: usize,
        end: usize,
        delimiters: &Delimiters,
    ) -> Vec<SubComponentIndex> {
        let mut subs = Vec::new();
        let mut offset = start;

        loop {
            let sub_end = memchr(delimiters.subcomponent, &data[offset..end])
                .map(|p| offset + p)
                .unwrap_or(end);

            subs.push(SubComponentIndex {
                span: Span::new(offset, sub_end),
            });

            if sub_end >= end {
                break;
            }
            offset = sub_end + 1;
        }

        // Only keep subcomponents if there are actually multiple
        if subs.len() <= 1 {
            subs.clear();
        }

        subs
    }

    /// Extract a specific MSH field value as a string.
    fn extract_msh_field(
        &self,
        data: &[u8],
        segments: &[SegmentIndex],
        _delimiters: &Delimiters,
        field_position: usize,
    ) -> Option<String> {
        let msh = segments.first()?;
        if msh.segment_type != "MSH" {
            return None;
        }
        let field = msh.fields.iter().find(|f| f.position == field_position)?;
        Some(field.span.as_str(data).to_string())
    }
}

impl Default for Hl7Lexer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_adt_a01() -> Vec<u8> {
        b"MSH|^~\\&|SENDING|FACILITY|RECEIVING|FACILITY|20240101120000||ADT^A01|123456|P|2.5.1\rPID|||12345^^^MRN||Doe^John^A||19800101|M|||123 Main St^^City^ST^12345\rPV1||I|ICU^101^A".to_vec()
    }

    #[test]
    fn test_parse_basic_message() {
        let lexer = Hl7Lexer::new();
        let msg = lexer.parse(sample_adt_a01()).unwrap();
        assert_eq!(msg.segments.len(), 3);
        assert_eq!(msg.segments[0].segment_type, "MSH");
        assert_eq!(msg.segments[1].segment_type, "PID");
        assert_eq!(msg.segments[2].segment_type, "PV1");
    }

    #[test]
    fn test_msh_delimiters() {
        let lexer = Hl7Lexer::new();
        let msg = lexer.parse(sample_adt_a01()).unwrap();
        assert_eq!(msg.delimiters.field, b'|');
        assert_eq!(msg.delimiters.component, b'^');
    }

    #[test]
    fn test_version_detection() {
        let lexer = Hl7Lexer::new();
        let msg = lexer.parse(sample_adt_a01()).unwrap();
        assert_eq!(msg.version, "2.5.1");
    }

    #[test]
    fn test_message_type_detection() {
        let lexer = Hl7Lexer::new();
        let msg = lexer.parse(sample_adt_a01()).unwrap();
        assert_eq!(msg.message_type, "ADT^A01");
    }

    #[test]
    fn test_field_count() {
        let lexer = Hl7Lexer::new();
        let msg = lexer.parse(sample_adt_a01()).unwrap();
        // MSH has fields 1-12
        let msh = &msg.segments[0];
        assert!(msh.fields.len() >= 12, "MSH should have at least 12 fields, got {}", msh.fields.len());
    }

    #[test]
    fn test_component_parsing() {
        let lexer = Hl7Lexer::new();
        let msg = lexer.parse(sample_adt_a01()).unwrap();
        // PID-5 is "Doe^John^A" - has 3 components
        let pid = &msg.segments[1];
        let pid5 = pid.fields.iter().find(|f| f.position == 5).unwrap();
        assert_eq!(pid5.repetitions[0].components.len(), 3);
        assert_eq!(pid5.repetitions[0].components[0].span.as_str(&msg.raw), "Doe");
        assert_eq!(pid5.repetitions[0].components[1].span.as_str(&msg.raw), "John");
    }

    #[test]
    fn test_truncation_flag() {
        let mut long_msg = b"MSH|^~\\&|SEND|FAC|RECV|FAC|20240101||ORU^R01|1|P|2.5\rOBX|1|ED|BASE64||".to_vec();
        long_msg.extend(vec![b'A'; 200]); // 200 bytes of base64
        let lexer = Hl7Lexer::new().with_truncation_threshold(100);
        let msg = lexer.parse(long_msg).unwrap();
        let obx = &msg.segments[1];
        let last_field = obx.fields.last().unwrap();
        assert!(last_field.is_truncated);
    }

    #[test]
    fn test_empty_fields() {
        let data = b"MSH|^~\\&|SEND||RECV||20240101||ADT^A01|1|P|2.5\r".to_vec();
        let lexer = Hl7Lexer::new();
        let msg = lexer.parse(data).unwrap();
        let msh = &msg.segments[0];
        // Field 4 (FACILITY) should be empty
        let field4 = msh.fields.iter().find(|f| f.position == 4).unwrap();
        assert!(field4.span.is_empty() || field4.span.as_str(&msg.raw).is_empty());
    }

    #[test]
    fn test_crlf_line_endings() {
        let data = b"MSH|^~\\&|S|F|R|F|20240101||ADT^A01|1|P|2.5\r\nPID|||123\r\nPV1||I".to_vec();
        let lexer = Hl7Lexer::new();
        let msg = lexer.parse(data).unwrap();
        assert_eq!(msg.segments.len(), 3);
    }

    #[test]
    fn test_lf_line_endings() {
        let data = b"MSH|^~\\&|S|F|R|F|20240101||ADT^A01|1|P|2.5\nPID|||123\nPV1||I".to_vec();
        let lexer = Hl7Lexer::new();
        let msg = lexer.parse(data).unwrap();
        assert_eq!(msg.segments.len(), 3);
    }
}

#[cfg(test)]
mod bench_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_large_message_performance() {
        let data = std::fs::read("../tests/fixtures/hl7/large_oru_with_base64.hl7")
            .expect("Failed to read large test file");
        let size_mb = data.len() as f64 / 1024.0 / 1024.0;
        println!("File size: {:.2} MB", size_mb);

        let lexer = Hl7Lexer::new().with_truncation_threshold(100);

        let start = Instant::now();
        let msg = lexer.parse(data).expect("Failed to parse");
        let parse_duration = start.elapsed();

        println!("Parse time: {:?}", parse_duration);
        println!("Segments: {}", msg.segments.len());

        let truncated_count: usize = msg.segments.iter()
            .flat_map(|s| &s.fields)
            .filter(|f| f.is_truncated)
            .count();
        println!("Truncated fields: {}", truncated_count);

        // Build truncated text
        let start2 = Instant::now();
        let truncated = crate::parser::truncation::build_truncated_text(&msg, 50);
        let trunc_duration = start2.elapsed();

        println!("Truncation time: {:?}", trunc_duration);
        println!("Truncated text size: {} bytes ({:.2} KB)", truncated.len(), truncated.len() as f64 / 1024.0);
        println!("Size reduction: {:.1}x", msg.raw.len() as f64 / truncated.len() as f64);

        // MUST be under 1 second total
        let total = parse_duration + trunc_duration;
        println!("TOTAL time: {:?}", total);
        assert!(total.as_secs_f64() < 1.0, "Total time exceeded 1 second: {:?}", total);
    }
}
