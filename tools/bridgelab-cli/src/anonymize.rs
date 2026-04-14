use crate::parser::Hl7Message;

/// Known PHI fields (segment_type, field_position, sensitivity_level)
/// high = fully masked, medium = partial, low = first 3 chars kept
const PHI: &[(&str, usize, &str)] = &[
    ("PID", 3, "high"), ("PID", 4, "high"), ("PID", 5, "high"),
    ("PID", 6, "medium"), ("PID", 7, "high"), ("PID", 9, "medium"),
    ("PID", 11, "high"), ("PID", 13, "high"), ("PID", 14, "medium"),
    ("PID", 18, "high"), ("PID", 19, "high"), ("PID", 20, "high"),
    ("NK1", 2, "medium"), ("NK1", 4, "medium"), ("NK1", 5, "medium"),
    ("IN1", 16, "medium"), ("IN1", 36, "high"),
    ("GT1", 3, "medium"), ("GT1", 5, "medium"), ("GT1", 6, "medium"), ("GT1", 12, "high"),
];

pub fn anonymize(msg: &Hl7Message) -> String {
    let sep = msg.delimiters.field as char;
    let mut out_segments: Vec<String> = Vec::new();

    for seg in &msg.segments {
        let seg_text = seg.span.as_str(&msg.raw);
        let is_msh = seg.segment_type == "MSH";
        let mut fields: Vec<String> = seg_text.split(sep).map(String::from).collect();

        for &(seg_type, pos, level) in PHI {
            if seg.segment_type != seg_type { continue; }
            let idx = if is_msh { pos } else { pos };
            if idx < fields.len() && !fields[idx].trim().is_empty() {
                fields[idx] = mask_value(&fields[idx], level);
            }
        }

        out_segments.push(fields.join(&sep.to_string()));
    }

    out_segments.join("\r")
}

fn mask_value(value: &str, level: &str) -> String {
    if value.is_empty() { return String::new(); }
    // Mask each component separately if value has components
    if value.contains('^') {
        return value.split('^').map(|c| mask_single(c, level)).collect::<Vec<_>>().join("^");
    }
    mask_single(value, level)
}

fn mask_single(value: &str, level: &str) -> String {
    if value.is_empty() { return String::new(); }
    match level {
        "high" => {
            if value.chars().all(|c| c.is_ascii_digit()) {
                "0".repeat(value.len().min(20))
            } else {
                "REDACTED".into()
            }
        }
        "medium" => {
            if value.len() <= 1 { "*".into() }
            else { format!("{}***", value.chars().take(1).collect::<String>()) }
        }
        _ => value.chars().take(3).collect::<String>(),
    }
}
