use chrono::Local;

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
    msa_field(response, 1)
}

/// MSA-2 of an acknowledgment: the control ID (MSH-10) of the message it
/// acknowledges. None when there is no MSA or the field is empty.
pub fn acknowledged_control_id(response: &str) -> Option<String> {
    msa_field(response, 2)
}

fn msa_field(response: &str, n: usize) -> Option<String> {
    let segments: Vec<&str> = response.split(['\r', '\n']).collect();
    let sep = segments
        .iter()
        .find_map(|seg| seg.strip_prefix("MSH").and_then(|rest| rest.chars().next()))
        .unwrap_or('|');
    segments
        .iter()
        .find(|seg| seg.starts_with("MSA") && seg[3..].starts_with(sep))
        .and_then(|msa| msa.split(sep).nth(n))
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .map(str::to_string)
}

/// The MSH segment of `message`, split on the field separator its MSH-1
/// declares: `["MSH", MSH-2, MSH-3, …]`, so MSH-n sits at index n - 1.
///
/// Segments end with CR, LF or CR LF; a byte-order mark and blank lines
/// before the header are skipped. None when there is no MSH segment.
fn msh_fields(message: &str) -> Option<(char, Vec<&str>)> {
    let msh = message
        .trim_start_matches('\u{FEFF}')
        .split(['\r', '\n'])
        .map(|seg| seg.trim_start_matches('\u{FEFF}'))
        .find(|seg| seg.starts_with("MSH"))?;
    let sep = msh[3..].chars().next().filter(|c| !c.is_alphanumeric() && !c.is_whitespace())?;
    Some((sep, msh.split(sep).collect()))
}

/// MSH-`n` of `message` (n >= 2), trimmed; None when absent or empty.
fn msh_field(message: &str, n: usize) -> Option<String> {
    let (_, fields) = msh_fields(message)?;
    fields.get(n - 1).map(|f| f.trim()).filter(|f| !f.is_empty()).map(str::to_string)
}

/// Extract MSH-10 (Message Control ID) from raw HL7 text, whatever field
/// separator MSH-1 declares.
pub fn extract_message_control_id(message: &str) -> Option<String> {
    msh_field(message, 10)
}

/// Extract MSH-3 (Sending Application) from raw HL7 text.
pub fn extract_sending_app(message: &str) -> Option<String> {
    msh_field(message, 3)
}

/// Whether an HL7 version (MSH-12, e.g. "2.3.1") is at least `min`.
fn version_at_least(version: &str, min: &[u32]) -> bool {
    let parts: Vec<u32> = version.split('.').map(|p| p.trim().parse().unwrap_or(0)).collect();
    for (i, want) in min.iter().enumerate() {
        let have = parts.get(i).copied().unwrap_or(0);
        if have != *want {
            return have > *want;
        }
    }
    true
}

/// A Message Control ID for an ACK that no other ACK of this run shares:
/// "ACK" and a millisecond timestamp, 20 characters (the MSH-10 length in
/// v2.5). The timestamp is issued process-wide and never repeats: when
/// ACKs come faster than one per millisecond each takes the next
/// millisecond after the last one issued, so the IDs run briefly ahead of
/// the clock instead of wrapping.
fn next_ack_control_id() -> String {
    use std::sync::atomic::{AtomicI64, Ordering};
    static LAST_MS: AtomicI64 = AtomicI64::new(0);
    let now = Local::now().timestamp_millis();
    let prev = LAST_MS
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |last| Some(now.max(last + 1)))
        .unwrap_or(now);
    let ms = now.max(prev + 1);
    let at = chrono::DateTime::from_timestamp_millis(ms)
        .map(|t| t.with_timezone(&Local))
        .unwrap_or_else(Local::now);
    format!("ACK{}", at.format("%Y%m%d%H%M%S%3f"))
}

/// Build the acknowledgment of `message` with MSA-1 `ack_code` ("AA",
/// "AE", "AR", or a commit-mode code) and an optional MSA-3 text.
///
/// The ACK mirrors the message it answers, as HL7 expects of a receiver:
/// - the field separator and encoding characters of its MSH;
/// - sender and receiver swapped (MSH-3/4 are the message's MSH-5/6, and
///   the other way round), with "BridgeLab" when the message named no
///   receiving application;
/// - MSH-9 `ACK^<trigger>^ACK` (the message structure from v2.3.1 on);
/// - the message's processing ID (MSH-11), version (MSH-12) and
///   character set (MSH-18);
/// - MSA-2 its Message Control ID (MSH-10).
///
/// MSH-10 of the ACK itself is unique per ACK, and MSH-7 is local time
/// with its UTC offset. A message without a readable MSH gets an ACK with
/// the standard separators, P and 2.5, and an empty MSA-2.
pub fn ack_for(message: &str, ack_code: &str, text_message: Option<&str>) -> String {
    let (sep, fields) = msh_fields(message).unwrap_or(('|', Vec::new()));
    let field = |n: usize| fields.get(n - 1).map(|f| f.trim()).unwrap_or("");
    fn or<'a>(value: &'a str, default: &'a str) -> &'a str {
        if value.is_empty() { default } else { value }
    }
    let encoding_chars = or(fields.get(1).copied().unwrap_or(""), "^~\\&");
    let comp = encoding_chars.chars().next().unwrap_or('^');
    let version = or(field(12), "2.5");
    let trigger = field(9).split(comp).nth(1).map(str::trim).unwrap_or("");
    let message_type = match (trigger.is_empty(), version_at_least(version.split(comp).next().unwrap_or(""), &[2, 3, 1])) {
        (true, _) => "ACK".to_string(),
        (false, true) => format!("ACK{c}{}{c}ACK", trigger, c = comp),
        (false, false) => format!("ACK{c}{}", trigger, c = comp),
    };
    let s = sep.to_string();
    let mut msh = vec![
        "MSH".to_string(),
        encoding_chars.to_string(),
        or(field(5), "BridgeLab").to_string(),
        field(6).to_string(),
        field(3).to_string(),
        field(4).to_string(),
        Local::now().format("%Y%m%d%H%M%S%z").to_string(),
        String::new(),
        message_type,
        next_ack_control_id(),
        or(field(11), "P").to_string(),
        version.to_string(),
    ];
    // The ACK is written in the message's character set: say so.
    if !field(18).is_empty() {
        msh.resize(17, String::new());
        msh.push(field(18).to_string());
    }
    let mut msa = vec!["MSA".to_string(), ack_code.to_string(), field(10).to_string()];
    if let Some(text) = text_message {
        // Our own text: never let it open a new field.
        msa.push(text.replace(sep, " "));
    }
    format!("{}\r{}\r", msh.join(&s), msa.join(&s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acknowledged_control_id_reads_msa2() {
        let ack = "MSH|^~\\&|R||S||20260101||ACK|A1|P|2.5\rMSA|AA|MSG42\r";
        assert_eq!(acknowledged_control_id(ack).as_deref(), Some("MSG42"));
        assert_eq!(acknowledged_control_id("MSA|AA\r"), None);
    }

    #[test]
    fn ack_code_is_read_from_msa_1() {
        let ack = ack_for("MSH|^~\\&|A|B|C|D|20260101||ADT^A01|MSG002|P|2.5\r", "AE", Some("Error in PID"));
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

    /// The ACK mirrors the message: swapped sender and receiver, trigger,
    /// processing ID, version, charset and separators, and MSA-2.
    #[test]
    fn the_ack_mirrors_the_message_header() {
        let msg = "MSH|^~\\&|LAB|HOSP|EHR|WARD|20260101120000||ORU^R01^ORU_R01|C42|T|2.3|||||ITA|8859/1\rPID|1\r";
        let ack = ack_for(msg, "AA", None);
        let msh: Vec<&str> = ack.split('\r').next().unwrap().split('|').collect();
        assert_eq!(&msh[..6], ["MSH", "^~\\&", "EHR", "WARD", "LAB", "HOSP"]);
        assert_eq!(msh[8], "ACK^R01", "no message structure before v2.3.1");
        assert_eq!((msh[10], msh[11]), ("T", "2.3"));
        assert_eq!(msh[17], "8859/1");
        assert_eq!(msh[9].len(), 20);
        assert!(ack.contains("\rMSA|AA|C42\r"), "{ack}");

        let v25 = ack_for("MSH|^~\\&|A||||20260101||ADT^A01|X|P|2.5.1\r", "AE", Some("bad | text"));
        assert!(v25.starts_with("MSH|^~\\&|BridgeLab||A||"), "{v25}");
        assert!(v25.contains("|ACK^A01^ACK|"));
        assert!(v25.contains("|P|2.5.1\rMSA|AE|X|bad   text\r"), "{v25}");
    }

    #[test]
    fn every_ack_gets_its_own_control_id() {
        let msg = "MSH|^~\\&|A|B|C|D|20260101||ADT^A01|X|P|2.5\r";
        let ids: std::collections::HashSet<String> = (0..50)
            .map(|_| ack_for(msg, "AA", None).split('|').nth(9).unwrap().to_string())
            .collect();
        assert_eq!(ids.len(), 50);
        // Far more than any per-second or per-millisecond budget, from
        // several threads at once: still no repeat, still 20 characters.
        let handles: Vec<_> = (0..4)
            .map(|_| std::thread::spawn(|| (0..5000).map(|_| next_ack_control_id()).collect::<Vec<_>>()))
            .collect();
        let all: Vec<String> = handles.into_iter().flat_map(|h| h.join().unwrap()).collect();
        assert!(all.iter().all(|id| id.len() == 20), "{:?}", all.first());
        let unique: std::collections::HashSet<&String> = all.iter().collect();
        assert_eq!(unique.len(), all.len());
    }

    /// Other separators, CR-only segments, a BOM and leading blank lines
    /// all still give the control ID; a short MSH does not borrow a field
    /// from the next segment.
    #[test]
    fn control_id_is_read_with_the_declared_separator() {
        let hash = "MSH#^~\\&#A#B#C#D#20260101##ADT^A01#HASH1#P#2.5\rPID#1";
        assert_eq!(extract_message_control_id(hash).as_deref(), Some("HASH1"));
        let ack = ack_for(hash, "AA", None);
        assert!(ack.starts_with("MSH#^~\\&#C#D#A#B#"), "{ack}");
        assert!(ack.contains("\rMSA#AA#HASH1\r"), "{ack}");
        assert_eq!(extract_message_control_id("MSH|^~\\&|A\rPID|1||2|3|4|5|6|7|8"), None);
        assert_eq!(ack_for("MSH|^~\\&|A\rPID|1||2|3|4|5|6|7|8", "AA", None).split('\r').nth(1), Some("MSA|AA|"));
        assert_eq!(extract_message_control_id("\u{FEFF}MSH|^~\\&|A|B|C|D|E||F|BOM1|P|2.5").as_deref(), Some("BOM1"));
        assert_eq!(extract_message_control_id("\r\nMSH|^~\\&|A|B|C|D|E||F|LEAD1|P|2.5").as_deref(), Some("LEAD1"));
        assert_eq!(extract_message_control_id("PID|1"), None);
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
