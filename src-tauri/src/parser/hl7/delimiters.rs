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

        let field = data[3];
        // MSH-2 encoding characters: component, repetition, escape, subcomponent
        let component = data[4];
        let repetition = data[5];
        let escape = data[6];
        let subcomponent = data[7];

        Ok(Self {
            field,
            component,
            repetition,
            escape,
            subcomponent,
        })
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

    #[test]
    fn batch_headers_declare_delimiters_too() {
        let d = Delimiters::from_msh(b"FHS#@!\\$#A").unwrap();
        assert_eq!(d.field, b'#');
        assert!(Delimiters::from_msh(b"BHS|^~\\&|A").is_ok());
    }
}
