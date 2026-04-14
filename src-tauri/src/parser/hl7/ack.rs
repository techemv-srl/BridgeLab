use chrono::Utc;

/// Generate an HL7 ACK message for a given incoming message.
///
/// `ack_code`: "AA" (accept), "AE" (error), "AR" (reject)
/// `message_control_id`: MSH-10 of the original message
/// `sending_app`: application name for MSH-3
/// `text_message`: optional text for MSA-3
pub fn generate_ack(
    ack_code: &str,
    message_control_id: &str,
    sending_app: &str,
    receiving_app: &str,
    text_message: Option<&str>,
) -> String {
    let now = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let ack_control_id = format!("ACK{}", &now);

    let mut ack = format!(
        "MSH|^~\\&|{}||{}||{}||ACK|{}|P|2.5\rMSA|{}|{}",
        sending_app,
        receiving_app,
        now,
        ack_control_id,
        ack_code,
        message_control_id,
    );

    if let Some(text) = text_message {
        ack.push('|');
        ack.push_str(text);
    }

    ack.push('\r');
    ack
}

/// Extract MSH-10 (Message Control ID) from raw HL7 text.
pub fn extract_message_control_id(message: &str) -> Option<String> {
    let first_line = message.lines().next()?;
    if !first_line.starts_with("MSH|") {
        return None;
    }
    // MSH-10 is the 10th field (0-indexed: field separator is MSH-1, so MSH-10 is the 9th pipe-delimited value)
    let fields: Vec<&str> = first_line.split('|').collect();
    // MSH|^~\&|..., fields[0]="MSH", fields[1]="^~\&", etc.
    // MSH-10 = fields[9] (0-indexed)
    fields.get(9).map(|s| s.to_string())
}

/// Extract MSH-3 (Sending Application) from raw HL7 text.
pub fn extract_sending_app(message: &str) -> Option<String> {
    let first_line = message.lines().next()?;
    if !first_line.starts_with("MSH|") {
        return None;
    }
    let fields: Vec<&str> = first_line.split('|').collect();
    // MSH-3 = fields[2]
    fields.get(2).map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ack() {
        let ack = generate_ack("AA", "MSG001", "BridgeLab", "RemoteApp", None);
        assert!(ack.starts_with("MSH|^~\\&|BridgeLab||RemoteApp|"));
        assert!(ack.contains("MSA|AA|MSG001"));
    }

    #[test]
    fn test_generate_ack_with_text() {
        let ack = generate_ack("AE", "MSG002", "BL", "RA", Some("Error in PID"));
        assert!(ack.contains("MSA|AE|MSG002|Error in PID"));
    }

    #[test]
    fn test_extract_message_control_id() {
        let msg = "MSH|^~\\&|SendApp|SF|RecvApp|RF|20230101||ADT^A01|CTRL123|P|2.5";
        assert_eq!(extract_message_control_id(msg), Some("CTRL123".into()));
    }

    #[test]
    fn test_extract_sending_app() {
        let msg = "MSH|^~\\&|MyApp|SF|RecvApp|RF|20230101||ADT^A01|CTRL|P|2.5";
        assert_eq!(extract_sending_app(msg), Some("MyApp".into()));
    }
}
