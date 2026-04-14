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

impl Delimiters {
    /// Parse delimiters from MSH segment header.
    /// Expects the raw bytes starting at "MSH|^~\\&"
    /// MSH-1 is the field separator (char after "MSH")
    /// MSH-2 is the encoding characters (next 4 chars)
    pub fn from_msh(data: &[u8]) -> Result<Self, &'static str> {
        // Minimum: "MSH|^~\&" = 8 bytes
        if data.len() < 8 {
            return Err("MSH segment too short to extract delimiters");
        }
        if &data[0..3] != b"MSH" {
            return Err("Message does not start with MSH");
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
    }
}
