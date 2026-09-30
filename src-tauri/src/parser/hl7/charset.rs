//! Character sets at the edges: turning the bytes of a file (or stdin) into
//! text, and text back into bytes for a file or the wire.
//!
//! Inside BridgeLab every message is UTF-8. Files are not: interfaces in
//! Europe still write ISO-8859-1 or Windows-1252, and say so (or not) in
//! MSH-18. Decoding happens once, where bytes come in, so the parser, the
//! validators, anonymisation and the tree never see anything but UTF-8.

use encoding_rs::Encoding;

/// Text decoded from input bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    pub text: String,
    /// `None` when the input was UTF-8; otherwise the `encoding_rs` label
    /// the bytes were decoded with (to encode them back the same way). Also
    /// `None` for a message that declares UTF-8 in MSH-18 but is not
    /// UTF-8 (see [`CharsetWarning::NotUtf8`]): it is written back in the
    /// UTF-8 it declares, as the app's Save does.
    pub charset: Option<String>,
    /// Set when the bytes could not be read the way the message says they
    /// are written. The text is then a Windows-1252 reading of them.
    pub warning: Option<CharsetWarning>,
}

/// Why a decoded text may not be what the file holds.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", content = "declared", rename_all = "snake_case")]
pub enum CharsetWarning {
    /// MSH-18 names a character set BridgeLab cannot decode (for example
    /// `CNS 11643-1992` or the ISO 2022 Japanese sets). In a multi-byte
    /// set a byte of a character can equal a delimiter, so field
    /// boundaries, and anything that relies on them, may be wrong.
    Unsupported(String),
    /// MSH-18 says UTF-8 but the bytes are not valid UTF-8.
    NotUtf8(String),
}

impl CharsetWarning {
    /// True when field boundaries cannot be trusted (see [`Self::Unsupported`]).
    pub fn is_unsupported(&self) -> bool {
        matches!(self, Self::Unsupported(_))
    }
}

impl std::fmt::Display for CharsetWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(name) => write!(
                f,
                "MSH-18 declares the character set '{name}', which BridgeLab cannot decode; \
                 the file was read as Windows-1252, so text and field boundaries may be wrong"
            ),
            Self::NotUtf8(name) => write!(
                f,
                "MSH-18 declares '{name}' but the file is not valid UTF-8; it was read as Windows-1252"
            ),
        }
    }
}

/// Decode bytes read from a file or standard input.
///
/// - Valid UTF-8 is taken as it is; a leading byte-order mark is dropped.
/// - Anything else is decoded with the character set MSH-18 declares, when
///   it names one this function knows (the table 0211 single-byte sets,
///   BIG-5, GB 18030-2000 and KS X 1001).
/// - Otherwise with Windows-1252: a superset of ISO-8859-1's printable
///   range, and what "Latin-1" files written on Windows really contain.
///   When MSH-18 names a charset (one BridgeLab cannot decode, or UTF-8
///   for bytes that are not UTF-8), `warning` says so.
///
/// Never fails: undecodable bytes become U+FFFD.
pub fn decode_input(bytes: &[u8]) -> Decoded {
    // UTF-16 with a byte-order mark: what Notepad writes when "Unicode" is
    // chosen. Read as Windows-1252 it was NUL-riddled noise that never
    // parsed; the label makes Save write it back as UTF-16.
    for (bom, enc) in [(&b"\xFF\xFE"[..], encoding_rs::UTF_16LE), (&b"\xFE\xFF"[..], encoding_rs::UTF_16BE)] {
        if let Some(rest) = bytes.strip_prefix(bom) {
            let text = enc.decode_without_bom_handling(rest).0.into_owned();
            return Decoded { text, charset: Some(enc.name().to_string()), warning: None };
        }
    }
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let declared = declared_charset(bytes);
    let utf8 = std::str::from_utf8(bytes).ok();
    match declared.as_deref().map(|c| (c, label_for_hl7(c))) {
        // A charset BridgeLab cannot decode is reported whatever the bytes:
        // the ISO 2022 sets (ISO IR87, ISO IR159) are 7-bit, so valid UTF-8,
        // and their escape sequences would otherwise parse as plain text.
        Some((c, None)) => {
            let warning = Some(CharsetWarning::Unsupported(c.to_string()));
            return match utf8 {
                Some(text) => Decoded { text: text.to_string(), charset: None, warning },
                None => {
                    let label = encoding_rs::WINDOWS_1252.name();
                    Decoded { text: decode_with(bytes, label), charset: Some(label.to_string()), warning }
                }
            };
        }
        // A multi-byte charset (Big5, GB18030, EUC-KR…): its byte pairs
        // are often valid UTF-8 by chance (Big5 C2-DF + A1-BF, say), so the
        // declaration decides. Single-byte declarations keep the UTF-8
        // check below: Latin text is practically never valid UTF-8 beyond
        // ASCII, while UTF-8 files carrying a stale `8859/1` are common, and
        // pure ASCII reads the same either way.
        Some((_, Some(label))) if is_multi_byte(&label) => {
            return Decoded { text: decode_with(bytes, &label), charset: Some(label), warning: None };
        }
        _ => {}
    }
    if let Some(text) = utf8 {
        return Decoded { text: text.to_string(), charset: None, warning: None };
    }
    let (label, warning) = match declared.as_deref().map(|c| (c, label_for_hl7(c))) {
        Some((_, Some(l))) if l != "UTF-8" => (l, None),
        Some((c, Some(_))) => {
            let text = decode_with(bytes, encoding_rs::WINDOWS_1252.name());
            return Decoded { text, charset: None, warning: Some(CharsetWarning::NotUtf8(c.to_string())) };
        }
        Some((c, None)) => (encoding_rs::WINDOWS_1252.name().to_string(), Some(CharsetWarning::Unsupported(c.to_string()))),
        None => (encoding_rs::WINDOWS_1252.name().to_string(), None),
    };
    Decoded { text: decode_with(bytes, &label), charset: Some(label), warning }
}

/// True ISO-8859-1 (Latin-1), which `encoding_rs` does not have: following
/// the web standard it maps every "ISO-8859-1" label to Windows-1252. The
/// two differ in 0x80-0x9F, where Windows-1252 has € and typographic quotes
/// and ISO-8859-1 has control codes; a message declaring `8859/1` must not
/// get Windows-1252 bytes there.
const LATIN1: &str = "ISO-8859-1";

/// Every label `encoding_rs` maps to Windows-1252 that means Latin-1 (the
/// WHATWG list), plus HL7's `8859/1`.
fn is_latin1(label: &str) -> bool {
    matches!(
        label.trim().to_ascii_uppercase().as_str(),
        "ISO-8859-1"
            | "ISO8859-1"
            | "ISO88591"
            | "ISO_8859-1"
            | "ISO_8859-1:1987"
            | "ISO-IR-100"
            | "CP819"
            | "IBM819"
            | "CSISOLATIN1"
            | "LATIN1"
            | "L1"
            | "8859/1"
    )
}

/// Labels of 7-bit ASCII. `encoding_rs` maps some of them (`us-ascii`,
/// `ansi_x3.4-1968`) to Windows-1252, which would put 8-bit bytes on a
/// connection that promised 7.
fn is_ascii_label(label: &str) -> bool {
    matches!(
        label.trim().to_ascii_uppercase().as_str(),
        "ASCII" | "US-ASCII" | "ANSI_X3.4-1968" | "ANSI_X3.4-1986" | "ISO646-US" | "ISO-IR-6" | "CSASCII" | "US" | "IBM367" | "CP367"
    )
}

/// The label BridgeLab uses for an HL7 character-set name: `ISO-8859-1`
/// (true Latin-1) for `8859/1`, `ASCII` (7-bit, see [`encode_output`]) for
/// ASCII and its aliases, the `encoding_rs` name otherwise. `encoding_rs`
/// alone would turn ASCII into Windows-1252, and a Save would then write
/// 8-bit bytes into a message whose MSH-18 promises ASCII.
pub fn label_for_hl7(name: &str) -> Option<String> {
    if is_latin1(name) {
        return Some(LATIN1.to_string());
    }
    if is_ascii_label(name) {
        return Some("ASCII".to_string());
    }
    encoding_for_hl7(name).map(|e| e.name().to_string())
}

/// Decode `bytes` in the charset `label` names (see [`label_for_hl7`]);
/// an unknown label decodes as Windows-1252.
pub fn decode_with(bytes: &[u8], label: &str) -> String {
    if is_latin1(label) {
        return bytes.iter().map(|&b| b as char).collect();
    }
    let enc = Encoding::for_label(label.trim().as_bytes()).unwrap_or(encoding_rs::WINDOWS_1252);
    enc.decode_without_bom_handling(bytes).0.into_owned()
}

/// MSH-18 (first repetition, first component) of the message in `bytes`,
/// read without decoding the rest: the header is ASCII in every HL7 charset.
pub fn declared_charset(bytes: &[u8]) -> Option<String> {
    // MSH at the start of a line only: "MSH" inside a FHIR narrative or a
    // field value is not a header.
    let start = bytes
        .windows(3)
        .enumerate()
        .position(|(i, w)| w == b"MSH" && (i == 0 || matches!(bytes[i - 1], b'\r' | b'\n')))?;
    let header = &bytes[start..];
    let sep = *header.get(3)?;
    let end = header.iter().position(|&b| b == b'\r' || b == b'\n').unwrap_or(header.len());
    // Split MSH on the field separator: ["MSH", MSH-2, MSH-3, ...], so
    // MSH-n sits at index n - 1.
    let field = header[..end].split(|&b| b == sep).nth(17)?;
    let enc_chars = header.get(4..8).unwrap_or(b"^~\\&");
    let (comp, rep) = (enc_chars[0], enc_chars[1]);
    let first = field.split(|&b| b == rep).next()?.split(|&b| b == comp).next()?;
    let value = std::str::from_utf8(first).ok()?.trim();
    (!value.is_empty()).then(|| value.to_string())
}

/// The `encoding_rs` encoding for an HL7 character-set name (table 0211) or
/// for a plain encoding label such as `windows-1252`. `None` for a table
/// 0211 name BridgeLab cannot decode: CNS 11643-1992, the ISO 2022
/// Japanese sets (ISO IR14, ISO IR87, ISO IR159, which switch with escape
/// sequences), and UTF-16/UTF-32 without a byte-order mark.
pub fn encoding_for_hl7(name: &str) -> Option<&'static Encoding> {
    let label = match name.trim().to_ascii_uppercase().as_str() {
        "ASCII" => "windows-1252",
        "8859/1" => "ISO-8859-1",
        "8859/2" => "ISO-8859-2",
        "8859/3" => "ISO-8859-3",
        "8859/4" => "ISO-8859-4",
        "8859/5" => "ISO-8859-5",
        "8859/6" => "ISO-8859-6",
        "8859/7" => "ISO-8859-7",
        "8859/8" => "ISO-8859-8",
        "8859/9" => "windows-1254",
        "8859/15" => "ISO-8859-15",
        "UNICODE UTF-8" | "UNICODE" => "UTF-8",
        // The East Asian sets of table 0211, under their HL7 spellings
        // (the WHATWG labels "big5", "gb18030" and "euc-kr" still work).
        "BIG-5" => "Big5",
        "GB 18030-2000" => "gb18030",
        // KS X 1001 (formerly KS C 5601) travels as EUC-KR.
        "KS X 1001" => "EUC-KR",
        "CNS 11643-1992" | "ISO IR14" | "ISO IR87" | "ISO IR159" | "UNICODE UTF-16" | "UNICODE UTF-32" => return None,
        _ => return Encoding::for_label(name.trim().as_bytes()),
    };
    Encoding::for_label(label.as_bytes())
}

/// Encode UTF-8 text into `label`'s bytes (an `encoding_rs` label, or
/// `ASCII`). A character the target cannot represent becomes `?`: HL7 has
/// no numeric character references, so `&#233;`-style output (what
/// `encoding_rs` produces for the web) would corrupt the message. An empty,
/// UTF-8 or unknown label returns UTF-8 bytes.
pub fn encode_output(text: &str, label: &str) -> Vec<u8> {
    let label = label.trim();
    if is_ascii_label(label) {
        return text.chars().map(|c| if c.is_ascii() { c as u8 } else { b'?' }).collect();
    }
    if is_latin1(label) {
        return text.chars().map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?')).collect();
    }
    // encoding_rs has no UTF-16 encoder (its UTF-16 labels encode to UTF-8,
    // as the web standard says): write the units here, after a BOM, as the
    // file was read.
    let utf16 = Encoding::for_label(label.as_bytes()).filter(|e| *e == encoding_rs::UTF_16LE || *e == encoding_rs::UTF_16BE);
    if let Some(enc) = utf16 {
        let le = enc == encoding_rs::UTF_16LE;
        let mut out = Vec::with_capacity(2 + text.len() * 2);
        out.extend_from_slice(if le { b"\xFF\xFE" } else { b"\xFE\xFF" });
        for unit in text.encode_utf16() {
            out.extend_from_slice(&if le { unit.to_le_bytes() } else { unit.to_be_bytes() });
        }
        return out;
    }
    let enc = match Encoding::for_label(label.as_bytes()) {
        Some(e) if !label.is_empty() && e != encoding_rs::UTF_8 => e,
        _ => return text.as_bytes().to_vec(),
    };
    let mut out = Vec::with_capacity(text.len());
    let mut buf = [0u8; 4];
    for c in text.chars() {
        let (bytes, _, unmappable) = enc.encode(c.encode_utf8(&mut buf));
        if unmappable {
            out.push(b'?');
        } else {
            out.extend_from_slice(&bytes);
        }
    }
    out
}

/// Whether `label` (from [`label_for_hl7`]) names a multi-byte legacy
/// charset such as Big5, GB18030, EUC-KR or ISO-2022-JP: not Latin-1,
/// ASCII, a single-byte set, UTF-8 or UTF-16.
fn is_multi_byte(label: &str) -> bool {
    !is_latin1(label)
        && !is_ascii_label(label)
        && Encoding::for_label(label.trim().as_bytes()).is_some_and(|e| {
            !e.is_single_byte() && e != encoding_rs::UTF_8 && e != encoding_rs::UTF_16LE && e != encoding_rs::UTF_16BE
        })
}

/// Whether `label` names UTF-16 (either byte order).
pub fn is_utf16(label: &str) -> bool {
    Encoding::for_label(label.trim().as_bytes()).is_some_and(|e| e == encoding_rs::UTF_16LE || e == encoding_rs::UTF_16BE)
}

/// Whether `label` names an encoding [`encode_output`] can produce.
pub fn is_known_label(label: &str) -> bool {
    let label = label.trim();
    is_ascii_label(label) || Encoding::for_label(label.as_bytes()).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LATIN1: &[u8] = b"MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5|||||ITA|8859/1\rPID|1||12345||M\xfcller^J\xf6rg\r";

    #[test]
    fn utf8_is_kept_and_its_bom_dropped() {
        let d = decode_input("\u{FEFF}MSH|^~\\&|A\rPID|1||x||Müller\r".as_bytes());
        assert_eq!(d.charset, None);
        assert!(d.text.starts_with("MSH|"));
        assert!(d.text.contains("Müller"));
    }

    #[test]
    fn latin1_is_decoded_with_the_declared_charset() {
        let d = decode_input(LATIN1);
        assert_eq!(d.charset.as_deref(), Some("ISO-8859-1"));
        assert!(d.text.contains("Müller^Jörg"), "{}", d.text);
    }

    #[test]
    fn latin1_is_true_latin1_not_windows_1252() {
        // € and typographic quotes exist in Windows-1252 at 0x80-0x9F but not
        // in ISO-8859-1: they become '?', never C1 control bytes.
        assert_eq!(encode_output("Zoë € “x”", "ISO-8859-1"), b"Zo\xeb ? ?x?");
        assert_eq!(encode_output("Zoë €", "windows-1252"), b"Zo\xeb \x80");
        assert_eq!(decode_with(b"\x80\xe0", "ISO-8859-1"), "\u{80}à");
        assert_eq!(label_for_hl7("8859/1").as_deref(), Some("ISO-8859-1"));
        assert_eq!(label_for_hl7("8859/15").as_deref(), Some("ISO-8859-15"));
    }

    #[test]
    fn undeclared_non_utf8_falls_back_to_windows_1252() {
        let d = decode_input(b"MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPV1|1|I|Lett\xf2 2 \x80\r");
        assert_eq!(d.charset.as_deref(), Some("windows-1252"));
        assert!(d.text.contains("Lettò 2 €"), "{}", d.text);
    }

    #[test]
    fn declared_charset_reads_msh18_only() {
        assert_eq!(declared_charset(LATIN1).as_deref(), Some("8859/1"));
        assert_eq!(declared_charset(b"MSH|^~\\&|A|B\rPID|1\r"), None);
        assert_eq!(declared_charset(b"MSH#^~\\&#A#B#C#D#E#F#G#H#I#J#K#L#M#N#O#8859/2\r").as_deref(), Some("8859/2"));
        assert_eq!(encoding_for_hl7("8859/2").map(|e| e.name()), Some("ISO-8859-2"));
        // Latin-9 is a real codec here, not an alias of Windows-1252: 0xA4 is €.
        let latin9 = encoding_for_hl7("8859/15").unwrap();
        assert_eq!(latin9.name(), "ISO-8859-15");
        assert_eq!(latin9.decode_without_bom_handling(b"\xa4").0, "€");
        assert_eq!(encode_output("€", "ISO-8859-15"), b"\xa4");
        assert_eq!(declared_charset(br#"{"resourceType":"Patient","text":"xMSH|^~\\&|A|B|C|D|E|F|G|H|I|J|K|L|M|N|O|8859/1"}"#), None);
        assert_eq!(declared_charset(b"FHS|^~\\&\rMSH|^~\\&|A|B|C|D|E|F|G|H|I|J|K|L|M|N|O|8859/15\r").as_deref(), Some("8859/15"));
    }

    #[test]
    fn a_declared_latin2_file_decodes_as_latin2() {
        let bytes = b"MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5|||||POL|8859/2\rPID|1||1||\xa3\xf3d\xbc^Jan\r";
        let d = decode_input(bytes);
        assert_eq!(d.charset.as_deref(), Some("ISO-8859-2"));
        assert!(d.text.contains("Łódź"), "{}", d.text);
    }

    #[test]
    fn round_trip_through_the_decoded_charset() {
        let d = decode_input(LATIN1);
        assert_eq!(encode_output(&d.text, d.charset.as_deref().unwrap()), LATIN1);
    }

    #[test]
    fn unmappable_characters_become_question_marks_not_references() {
        assert_eq!(encode_output("Zoë 日本", "ISO-8859-1"), b"Zo\xeb ??");
        assert_eq!(encode_output("Zoë", "ASCII"), b"Zo?");
        assert_eq!(encode_output("Zoë", ""), "Zoë".as_bytes());
        assert_eq!(encode_output("Zoë", "UTF-8"), "Zoë".as_bytes());
    }

    #[test]
    fn every_latin1_and_ascii_alias_is_encoded_as_such() {
        for label in ["iso88591", "cp819", "iso-ir-100", "csisolatin1", "ibm819", "ISO_8859-1:1987", "l1"] {
            assert_eq!(encode_output("Zoë €", label), b"Zo\xeb ?", "{label}");
        }
        for label in ["ascii", "us-ascii", "ansi_x3.4-1968", "ANSI_X3.4-1968"] {
            assert_eq!(encode_output("Müller", label), b"M?ller", "{label}");
            assert!(is_known_label(label));
        }
    }

    #[test]
    fn known_labels() {
        assert!(is_known_label("ISO-8859-1"));
        assert!(is_known_label("ascii"));
        assert!(!is_known_label("FOO-BAR"));
    }

    /// `m` encoded in `enc`, from a UTF-8 literal.
    fn encoded(m: &str, enc: &'static Encoding) -> Vec<u8> {
        let (bytes, _, unmappable) = enc.encode(m);
        assert!(!unmappable);
        bytes.into_owned()
    }

    #[test]
    fn east_asian_table_0211_charsets_decode_with_their_own_encoding() {
        for (name, enc, label, patient) in [
            ("BIG-5", encoding_rs::BIG5, "Big5", "陳四明"),
            ("GB 18030-2000", encoding_rs::GB18030, "gb18030", "王小明"),
            ("KS X 1001", encoding_rs::EUC_KR, "EUC-KR", "김철수"),
        ] {
            let text = format!(
                "MSH|^~\\&|A|B|C|D|20240101120000||ADT^A01|MSG1|P|2.5|||||XX|{name}\rPID|1||12345^^^FAC^MR||{patient}^Jan||19900515|M\r"
            );
            let bytes = encoded(&text, enc);
            let d = decode_input(&bytes);
            assert_eq!(d.charset.as_deref(), Some(label), "{name}");
            assert_eq!(d.warning, None, "{name}");
            assert_eq!(d.text, text, "{name}");
            assert_eq!(encode_output(&d.text, label), bytes, "{name}: round trip");
        }
        // Big5 四 ends in 0x7C, the field separator: read as Windows-1252 it
        // split PID-5 in two.
        assert_eq!(encoded("四", encoding_rs::BIG5), b"\xa5\x7c");
    }

    #[test]
    fn a_big5_message_that_is_valid_utf8_decodes_as_big5() {
        // 簡 is 0xC2 0xB2 in Big5: also the UTF-8 encoding of '²'.
        let text = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5|||||TWN|BIG-5\rPID|1||1||簡^Jan||19900515\r";
        let bytes = encoded(text, encoding_rs::BIG5);
        assert!(std::str::from_utf8(&bytes).is_ok(), "the test needs Big5 bytes that are valid UTF-8");
        let d = decode_input(&bytes);
        assert_eq!(d.charset.as_deref(), Some("Big5"));
        assert_eq!(d.text, text);
    }

    #[test]
    fn a_declaration_bridgelab_cannot_decode_is_reported_even_in_7_bit() {
        // ISO-2022-JP: ESC $ B … ESC ( B around the kanji, all 7-bit.
        let text = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5|||||JPN|ISO IR87\rPID|1||1||山田^太郎||19900515\r";
        let bytes = encoded(text, encoding_rs::ISO_2022_JP);
        assert!(bytes.is_ascii());
        let d = decode_input(&bytes);
        assert_eq!(d.warning, Some(CharsetWarning::Unsupported("ISO IR87".into())));
        // Single-byte declarations keep reading UTF-8 files as UTF-8, and a
        // pure-ASCII 8859/1 message is unchanged.
        let d = decode_input("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5|||||ITA|8859/1\rPID|1||1||Müller\r".as_bytes());
        assert_eq!((d.charset, d.warning), (None, None));
        assert!(d.text.contains("Müller"));
        let d = decode_input(b"MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5|||||ITA|8859/1\rPID|1||1||Rossi\r");
        assert_eq!((d.charset, d.warning), (None, None));
    }

    #[test]
    fn declared_ascii_stays_7_bit() {
        for name in ["ASCII", "ANSI_X3.4-1968", "US-ASCII"] {
            assert_eq!(label_for_hl7(name).as_deref(), Some("ASCII"), "{name}");
        }
        let d = decode_input(b"MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5|||||USA|ASCII\rPID|1||1||M\xfcller\r");
        assert_eq!(d.charset.as_deref(), Some("ASCII"));
        assert!(d.text.contains("Müller"));
        assert!(encode_output(&d.text, d.charset.as_deref().unwrap()).is_ascii());
    }

    #[test]
    fn a_charset_bridgelab_cannot_decode_is_reported() {
        let bytes = b"MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5|||||TWN|CNS 11643-1992\rPID|1||1||\xa4\x7c\r";
        let d = decode_input(bytes);
        assert_eq!(d.warning, Some(CharsetWarning::Unsupported("CNS 11643-1992".into())));
        assert!(d.warning.as_ref().unwrap().is_unsupported());
        for name in ["ISO IR87", "ISO IR159", "ISO IR14", "UNICODE UTF-32"] {
            assert_eq!(label_for_hl7(name), None, "{name}");
        }
        // Declared UTF-8, written in Latin-1: read, but not silently.
        let d = decode_input(b"MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5|||||ITA|UNICODE UTF-8\rPID|1||1||M\xfcller\r");
        assert_eq!(d.warning, Some(CharsetWarning::NotUtf8("UNICODE UTF-8".into())));
        assert!(d.text.contains("Müller"));
        assert_eq!(d.charset, None, "written back as the UTF-8 it declares");
        // Nothing declared: the Windows-1252 fallback, no warning.
        assert_eq!(decode_input(b"MSH|^~\\&|A\rPID|1||1||M\xfcller\r").warning, None);
    }

    #[test]
    fn utf16_with_a_bom_round_trips() {
        let text = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||1||Müller\r";
        for label in ["UTF-16LE", "UTF-16BE"] {
            let bytes = encode_output(text, label);
            assert_eq!(bytes.len(), 2 + text.encode_utf16().count() * 2);
            let d = decode_input(&bytes);
            assert_eq!(d.charset.as_deref(), Some(label));
            assert_eq!(d.text, text);
        }
        assert_eq!(&encode_output("M", "UTF-16LE")[..], b"\xFF\xFEM\0");
    }
}
