/// HL7 message delimiters extracted from MSH segment.
/// Default: |^~\&
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Delimiters {
    pub field: u8,
    pub component: u8,
    pub repetition: u8,
    pub escape: u8,
    pub subcomponent: u8,
}

impl Default for Delimiters {
    fn default() -> Self {
        Self {
            field: b'|',
            component: b'^',
            repetition: b'~',
            escape: b'\\',
            subcomponent: b'&',
        }
    }
}

/// A segment that declares the delimiters in its first two fields: the
/// message header, and the file and batch headers of a batch file.
pub fn is_header_type(segment_type: &[u8]) -> bool {
    matches!(segment_type, b"MSH" | b"FHS" | b"BHS")
}

impl Delimiters {
    /// Parse delimiters from the header segment the message starts with:
    /// MSH, or the FHS/BHS of a batch file, which declare their delimiters
    /// the same way.
    /// Expects the raw bytes starting at "MSH|^~\\&"
    /// MSH-1 is the field separator (char after "MSH")
    /// MSH-2 is the encoding characters (next 4 chars)
    pub fn from_msh(data: &[u8]) -> Result<Self, String> {
        // Minimum: "MSH|^~\&" = 8 bytes
        if data.len() < 8 {
            return Err("MSH segment too short to extract delimiters".into());
        }
        if !is_header_type(&data[0..3]) {
            // Say what is there instead: "does not start with MSH" alone
            // gave no clue that the file starts with, say, a stray byte.
            let line_end = data.iter().position(|&b| b == b'\r' || b == b'\n').unwrap_or(data.len()).min(20);
            let found = String::from_utf8_lossy(&data[..line_end]).escape_debug().to_string();
            return Err(format!("Message does not start with MSH (it starts with \"{}\")", found));
        }

        // The declaration is checked before anything is split with it: a
        // line break, a tab or a byte of a UTF-8 character taken as a
        // delimiter, or one character declared twice, shreds every field
        // into values that look real (a CR read as the escape character
        // turned "PAT1" into "AT1") instead of saying what is wrong.
        let header = String::from_utf8_lossy(&data[0..3]).into_owned();
        let field = data[3];
        if !is_delimiter(field) {
            return Err(format!(
                "{}-1 declares {} as the field separator: a delimiter must be a printable ASCII character other than a letter, a digit or a space",
                header,
                describe(field)
            ));
        }
        // MSH-2 runs to the next field separator or the end of the line.
        let declared = data[4..]
            .iter()
            .position(|&b| b == field || b == b'\r' || b == b'\n')
            .unwrap_or(data.len() - 4);
        // Three characters (no subcomponent separator) is a slip some
        // senders make; such messages always parsed, the subcomponent
        // separator never being needed: '&' is taken for it, when the
        // other three leave it free, and validation says so (MSH-006).
        let assumed_subcomponent = declared == 3 && !data[4..7].contains(&DEFAULT_SUBCOMPONENT);
        if declared < 4 && !assumed_subcomponent {
            return Err(format!(
                "{} segment too short: {}-2 holds {} of the four encoding characters (component, repetition, escape, subcomponent), as in {}|^~\\&",
                header, header, declared, header
            ));
        }
        // MSH-2 encoding characters: component, repetition, escape, subcomponent
        let component = data[4];
        let repetition = data[5];
        let escape = data[6];
        let subcomponent = if assumed_subcomponent { DEFAULT_SUBCOMPONENT } else { data[7] };

        let mut all = vec![field, component, repetition, escape, subcomponent];
        let mut names = vec!["field separator", "component separator", "repetition separator", "escape character", "subcomponent separator"];
        // From v2.7 a fifth character is the truncation character: it is
        // a delimiter like the others, so the same rules hold for it (not
        // a letter, not one of the other four or the field separator).
        if declared >= 5 {
            all.push(data[8]);
            names.push("truncation character");
        }
        for (i, &b) in all.iter().enumerate() {
            if !is_delimiter(b) {
                return Err(format!(
                    "{}-2 declares {} as the {}: a delimiter must be a printable ASCII character other than a letter, a digit or a space",
                    header,
                    describe(b),
                    names[i]
                ));
            }
            if let Some(j) = all[..i].iter().position(|&a| a == b) {
                return Err(format!(
                    "{} declares '{}' as both the {} and the {}: each delimiter must be a different character",
                    header, b as char, names[j], names[i]
                ));
            }
        }

        Ok(Self {
            field,
            component,
            repetition,
            escape,
            subcomponent,
        })
    }
}

/// The subcomponent separator taken when MSH-2 declares only the first
/// three encoding characters.
pub const DEFAULT_SUBCOMPONENT: u8 = b'&';

/// A byte that can be a delimiter: printable ASCII that is not a letter, a
/// digit or a space. A letter or a digit would split ordinary values, and
/// HL7 recommends |^~\& anyway.
fn is_delimiter(b: u8) -> bool {
    b.is_ascii_graphic() && !b.is_ascii_alphanumeric()
}

/// A byte as an error message names it: a control character by name or
/// code, a byte of a multi-byte character by its value.
fn describe(b: u8) -> String {
    match b {
        b'\r' => "a carriage return (CR)".into(),
        b'\n' => "a line feed (LF)".into(),
        b'\t' => "a tab".into(),
        b' ' => "a space".into(),
        0x80.. => format!("byte 0x{:02X}, part of a non-ASCII character,", b),
        _ if b.is_ascii_control() => format!("control character 0x{:02X}", b),
        _ => format!("'{}'", b as char),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_delimiters() {
        let d = Delimiters::from_msh(b"MSH|^~\\&|field1|field2").unwrap();
        assert_eq!(d.field, b'|');
        assert_eq!(d.component, b'^');
        assert_eq!(d.repetition, b'~');
        assert_eq!(d.escape, b'\\');
        assert_eq!(d.subcomponent, b'&');
    }

    #[test]
    fn test_custom_delimiters() {
        let d = Delimiters::from_msh(b"MSH#@!\\$#field1").unwrap();
        assert_eq!(d.field, b'#');
        assert_eq!(d.component, b'@');
        assert_eq!(d.repetition, b'!');
        assert_eq!(d.subcomponent, b'$');
    }

    #[test]
    fn test_invalid_msh() {
        assert!(Delimiters::from_msh(b"PID|data").is_err());
        assert!(Delimiters::from_msh(b"MSH|").is_err());
        let err = Delimiters::from_msh(b"XYZ|^~\\&|A\rPID").unwrap_err();
        assert!(err.contains("XYZ|^~"), "{}", err);
    }

    /// A declaration that cannot be split with is refused with what is
    /// wrong, never used: a CR, a tab, a non-ASCII separator, a letter or
    /// one character declared twice.
    #[test]
    fn unusable_declarations_are_refused() {
        let err = |d: &[u8]| Delimiters::from_msh(d).unwrap_err();
        // Only two encoding characters before the line ends.
        let e = err(b"MSH|^~\rPID|1||PAT1^^^HOSP^MR||Pope^Peter");
        assert!(e.contains("too short") && e.contains("holds 2"), "{e}");
        // A short MSH-2 ended by the field separator: three characters
        // are read with '&' for the missing subcomponent separator, unless
        // '&' is already one of them.
        assert!(err(b"MSH|^~&|A|B|C").contains("holds 3"));
        let d = Delimiters::from_msh(b"MSH|^~\\|A|B|C").unwrap();
        assert_eq!((d.component, d.repetition, d.escape, d.subcomponent), (b'^', b'~', b'\\', b'&'));
        let e = err(b"MSH\t^~\\&\tA\tB");
        assert!(e.contains("a tab") && e.contains("field separator"), "{e}");
        let e = err("MSH\u{a6}^~\\&\u{a6}A".as_bytes());
        assert!(e.contains("non-ASCII"), "{e}");
        assert!(err(b"MSH|^~\\ |A").contains("a space"));
        assert!(err(b"MSHA^~\\&A1").contains("field separator"));
        let e = err(b"MSH|^^\\&|A");
        assert!(e.contains("component separator and the repetition separator"), "{e}");
        assert!(err(b"MSH|^~\\^|A").contains("each delimiter"));
        // The usual declaration, and v2.7's with a fifth (truncation) character.
        assert!(Delimiters::from_msh(b"MSH|^~\\&|A").is_ok());
        assert!(Delimiters::from_msh(b"MSH|^~\\&#|A").is_ok());
        assert!(Delimiters::from_msh(b"MSH|^~\\&").is_ok());
    }

    /// A fifth MSH-2 character (v2.7's truncation character) is held to
    /// the same rules as the other delimiters.
    #[test]
    fn the_truncation_character_is_checked_too() {
        let err = |d: &[u8]| Delimiters::from_msh(d).unwrap_err();
        let e = err(b"MSH|^~\\&&|A|B");
        assert!(e.contains("subcomponent separator and the truncation character"), "{e}");
        assert!(err(b"MSH|^~\\&^|A").contains("each delimiter"));
        let e = err(b"MSH|^~\\&T|A|B");
        assert!(e.contains("'T' as the truncation character"), "{e}");
        assert!(err(b"MSH|^~\\&7|A").contains("truncation character"));
        assert!(err(b"MSH|^~\\& |A").contains("a space as the truncation character"));
        // A usable fifth character, and the three- and four-character forms.
        let d = Delimiters::from_msh(b"MSH|^~\\&#|A").unwrap();
        assert_eq!(d.subcomponent, b'&');
        assert!(Delimiters::from_msh(b"MSH|^~\\&|A").is_ok());
        assert!(Delimiters::from_msh(b"MSH|^~\\|A").is_ok());
    }

    #[test]
    fn batch_headers_declare_delimiters_too() {
        let d = Delimiters::from_msh(b"FHS#@!\\$#A").unwrap();
        assert_eq!(d.field, b'#');
        assert!(Delimiters::from_msh(b"BHS|^~\\&|A").is_ok());
    }
}
