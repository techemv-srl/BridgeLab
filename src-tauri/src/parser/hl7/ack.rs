use chrono::Utc;

/// MSA-1 of an acknowledgment message: the code that says whether the
/// message was accepted ("AA"), rejected for an application error ("AE") or
/// refused outright ("AR"), or the commit-mode equivalents ("CA", "CE",
/// "CR"). None when `response` carries no MSA segment — an empty reply, a
/// non-HL7 payload, or a message that is not an acknowledgment at all.
///
/// The segment separator is the HL7 `\r`, but responses copied through
/// tools that normalise line endings arrive with `\n` too, so both are
/// accepted. The field separator is whatever the ACK's own MSH-1 declares
/// (the character right after `MSH`), `|` when there is no MSH to read it
/// from. The code is returned as sent: the standard's codes are upper
/// case, and a lower-case one is a sender's deviation worth seeing.
pub fn ack_code_of(response: &str) -> Option<String> {
    let segments: Vec<&str> = response.split(['\r', '\n']).collect();
    let sep = segments
        .iter()
        .find_map(|seg| seg.strip_prefix("MSH").and_then(|rest| rest.chars().next()))
        .unwrap_or('|');
    segments
        .iter()
        .find(|seg| seg.starts_with("MSA") && seg[3..].starts_with(sep))
        .and_then(|msa| msa.split(sep).nth(1))
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .map(str::to_string)
}

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
    fn ack_code_is_read_from_msa_1() {
        let ack = generate_ack("AE", "MSG002", "BL", "RA", Some("Error in PID"));
        assert_eq!(ack_code_of(&ack).as_deref(), Some("AE"));
        assert_eq!(
            ack_code_of("MSH|^~\\&|R||S||20260101||ACK^A01^ACK|1|P|2.5\nMSA|CA|1\n").as_deref(),
            Some("CA"),
            "LF-separated responses are read too"
        );
        assert_eq!(ack_code_of("MSA|AR|1"), Some("AR".to_string()), "a bare MSA still counts");
        assert_eq!(ack_code_of(""), None);
        assert_eq!(ack_code_of("HTTP/1.1 200 OK"), None);
        assert_eq!(ack_code_of("MSH|^~\\&|R||S||20260101||ADT^A01|1|P|2.5\rPID|1"), None, "not an ACK");
        assert_eq!(ack_code_of("MSH|^~\\&|R||S\rMSA||1"), None, "empty MSA-1");
        assert_eq!(ack_code_of("MSH|^~\\&|R||S\rMSA|aa|1").as_deref(), Some("aa"), "returned as sent");
        // Codex review: the field separator is the ACK's own MSH-1, not
        // always "|".
        assert_eq!(
            ack_code_of("MSH*^~\\&*R**S**20260101**ACK*1*P*2.5\rMSA*AA*123").as_deref(),
            Some("AA")
        );
        assert_eq!(ack_code_of("MSH*^~\\&*R\rMSA|AE|1"), None, "MSA must use the declared separator");
        assert_eq!(ack_code_of("MSAX|AA|1"), None, "MSAX is not MSA");
    }

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
