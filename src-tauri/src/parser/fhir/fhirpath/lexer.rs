//! FHIRPath tokenizer.
//!
//! Follows the lexical rules of the FHIRPath 2.0 grammar: identifiers
//! (plain, delimited with backticks, or keyword-shaped), string literals with
//! backslash escapes, numbers, `@`-prefixed date/time literals, `$`
//! variables, `%` environment constants, and both comment forms.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Bare or backtick-delimited identifier.
    Ident(String),
    /// `'...'` string literal, escapes already resolved.
    Str(String),
    Integer(i64),
    Decimal(f64),
    /// `@2015-02-04`, `@2015-02-04T14:34:28Z`, `@T14:34:28` — the leading
    /// `@` is stripped, the rest is kept verbatim.
    DateTime(String),
    /// `$this`, `$index`, `$total` — the `$` is stripped.
    Variable(String),
    /// `%resource`, `%`ext-1`` — the `%` is stripped and any quoting removed.
    EnvConstant(String),
    Symbol(Symbol),
    Keyword(Keyword),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symbol {
    Dot,
    Comma,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Plus,
    Minus,
    Star,
    Slash,
    Amp,
    Pipe,
    Eq,
    NotEq,
    Equiv,
    NotEquiv,
    Lt,
    Gt,
    Lte,
    Gte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    And,
    Or,
    Xor,
    Implies,
    Div,
    Mod,
    In,
    Contains,
    Is,
    As,
    True,
    False,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Ident(s) => write!(f, "{}", s),
            Token::Str(s) => write!(f, "'{}'", s),
            Token::Integer(n) => write!(f, "{}", n),
            Token::Decimal(n) => write!(f, "{}", n),
            Token::DateTime(s) => write!(f, "@{}", s),
            Token::Variable(s) => write!(f, "${}", s),
            Token::EnvConstant(s) => write!(f, "%{}", s),
            Token::Symbol(s) => write!(f, "{}", symbol_text(*s)),
            Token::Keyword(k) => write!(f, "{}", keyword_text(*k)),
        }
    }
}

fn symbol_text(s: Symbol) -> &'static str {
    match s {
        Symbol::Dot => ".",
        Symbol::Comma => ",",
        Symbol::LParen => "(",
        Symbol::RParen => ")",
        Symbol::LBracket => "[",
        Symbol::RBracket => "]",
        Symbol::LBrace => "{",
        Symbol::RBrace => "}",
        Symbol::Plus => "+",
        Symbol::Minus => "-",
        Symbol::Star => "*",
        Symbol::Slash => "/",
        Symbol::Amp => "&",
        Symbol::Pipe => "|",
        Symbol::Eq => "=",
        Symbol::NotEq => "!=",
        Symbol::Equiv => "~",
        Symbol::NotEquiv => "!~",
        Symbol::Lt => "<",
        Symbol::Gt => ">",
        Symbol::Lte => "<=",
        Symbol::Gte => ">=",
    }
}

fn keyword_text(k: Keyword) -> &'static str {
    match k {
        Keyword::And => "and",
        Keyword::Or => "or",
        Keyword::Xor => "xor",
        Keyword::Implies => "implies",
        Keyword::Div => "div",
        Keyword::Mod => "mod",
        Keyword::In => "in",
        Keyword::Contains => "contains",
        Keyword::Is => "is",
        Keyword::As => "as",
        Keyword::True => "true",
        Keyword::False => "false",
    }
}

/// Keyword lookup. Note that these words are only keywords in operator
/// position — the parser re-admits them as member names where the grammar
/// allows it (`Patient.contains`, `Questionnaire.item.text`).
fn keyword_for(word: &str) -> Option<Keyword> {
    Some(match word {
        "and" => Keyword::And,
        "or" => Keyword::Or,
        "xor" => Keyword::Xor,
        "implies" => Keyword::Implies,
        "div" => Keyword::Div,
        "mod" => Keyword::Mod,
        "in" => Keyword::In,
        "contains" => Keyword::Contains,
        "is" => Keyword::Is,
        "as" => Keyword::As,
        "true" => Keyword::True,
        "false" => Keyword::False,
        _ => return None,
    })
}

/// The identifier text a keyword token carries when used as a member name.
pub fn keyword_as_ident(k: Keyword) -> &'static str {
    keyword_text(k)
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    Lexer::new(input).run()
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
}

impl Lexer {
    fn new(input: &str) -> Self {
        Lexer {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn run(mut self) -> Result<Vec<Token>, String> {
        let mut out = Vec::new();
        loop {
            self.skip_trivia()?;
            let Some(c) = self.peek() else { break };
            let token = match c {
                '\'' => self.lex_string('\'')?,
                '"' => {
                    // Not legal FHIRPath, but common enough in hand-typed
                    // expressions that rejecting it helps nobody.
                    self.lex_string('"')?
                }
                '`' => {
                    let name = self.lex_delimited('`')?;
                    Token::Ident(name)
                }
                '@' => self.lex_datetime()?,
                '$' => {
                    self.bump();
                    let name = self.lex_word();
                    if name.is_empty() {
                        return Err("Expected a variable name after '$'".into());
                    }
                    Token::Variable(name)
                }
                '%' => {
                    self.bump();
                    let name = match self.peek() {
                        Some('`') => self.lex_delimited('`')?,
                        Some('\'') => match self.lex_string('\'')? {
                            Token::Str(s) => s,
                            _ => unreachable!(),
                        },
                        _ => self.lex_word(),
                    };
                    if name.is_empty() {
                        return Err("Expected a name after '%'".into());
                    }
                    Token::EnvConstant(name)
                }
                c if c.is_ascii_digit() => self.lex_number()?,
                c if c.is_alphabetic() || c == '_' => {
                    let word = self.lex_word();
                    match keyword_for(&word) {
                        Some(k) => Token::Keyword(k),
                        None => Token::Ident(word),
                    }
                }
                _ => self.lex_symbol()?,
            };
            out.push(token);
        }
        Ok(out)
    }

    /// Whitespace plus `//` line comments and `/* */` block comments.
    fn skip_trivia(&mut self) -> Result<(), String> {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some('/') if self.peek_at(1) == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                Some('/') if self.peek_at(1) == Some('*') => {
                    self.bump();
                    self.bump();
                    loop {
                        match self.peek() {
                            None => return Err("Unterminated block comment".into()),
                            Some('*') if self.peek_at(1) == Some('/') => {
                                self.bump();
                                self.bump();
                                break;
                            }
                            _ => {
                                self.bump();
                            }
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn lex_word(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.bump();
            } else {
                break;
            }
        }
        s
    }

    fn lex_delimited(&mut self, quote: char) -> Result<String, String> {
        self.bump(); // opening quote
        let mut s = String::new();
        loop {
            match self.bump() {
                None => return Err(format!("Unterminated {} literal", quote)),
                Some(c) if c == quote => break,
                Some('\\') => s.push(self.escape()?),
                Some(c) => s.push(c),
            }
        }
        Ok(s)
    }

    fn lex_string(&mut self, quote: char) -> Result<Token, String> {
        Ok(Token::Str(self.lex_delimited(quote)?))
    }

    fn escape(&mut self) -> Result<char, String> {
        match self.bump() {
            Some('n') => Ok('\n'),
            Some('r') => Ok('\r'),
            Some('t') => Ok('\t'),
            Some('f') => Ok('\u{000C}'),
            Some('\\') => Ok('\\'),
            Some('/') => Ok('/'),
            Some('\'') => Ok('\''),
            Some('"') => Ok('"'),
            Some('`') => Ok('`'),
            Some('u') => {
                let mut hex = String::new();
                for _ in 0..4 {
                    match self.bump() {
                        Some(c) if c.is_ascii_hexdigit() => hex.push(c),
                        _ => return Err("Invalid \\u escape: expected 4 hex digits".into()),
                    }
                }
                let code = u32::from_str_radix(&hex, 16)
                    .map_err(|_| "Invalid \\u escape".to_string())?;
                char::from_u32(code).ok_or_else(|| format!("Invalid code point U+{}", hex))
            }
            Some(c) => Err(format!("Unknown escape sequence \\{}", c)),
            None => Err("Trailing backslash in literal".into()),
        }
    }

    fn lex_number(&mut self) -> Result<Token, String> {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                s.push(c);
                self.bump();
            } else {
                break;
            }
        }
        // A '.' only continues the number when a digit follows; otherwise it
        // is the path separator in `1.toString()`.
        if self.peek() == Some('.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
            s.push('.');
            self.bump();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    s.push(c);
                    self.bump();
                } else {
                    break;
                }
            }
            let v: f64 = s.parse().map_err(|_| format!("Invalid decimal: {}", s))?;
            return Ok(Token::Decimal(v));
        }
        let v: i64 = s.parse().map_err(|_| format!("Integer out of range: {}", s))?;
        Ok(Token::Integer(v))
    }

    /// `@` date, dateTime or time literal. Kept as text; the evaluator
    /// interprets precision.
    fn lex_datetime(&mut self) -> Result<Token, String> {
        self.bump(); // '@'
        let mut s = String::new();
        while let Some(c) = self.peek() {
            let belongs = match c {
                // '+' only belongs to the literal as the sign of a timezone
                // offset; anywhere else it is the addition operator.
                '+' => s.contains('T'),
                // A '.' belongs to the literal only as the decimal point of
                // fractional seconds. Otherwise it is the path separator, as
                // in `@2015-02-04.is(Date)`.
                '.' => self.peek_at(1).is_some_and(|c| c.is_ascii_digit()),
                '-' | ':' => true,
                c => c.is_ascii_alphanumeric(),
            };
            if belongs {
                s.push(c);
                self.bump();
            } else {
                break;
            }
        }
        if s.is_empty() {
            return Err("Expected a date or time after '@'".into());
        }
        Ok(Token::DateTime(s))
    }

    fn lex_symbol(&mut self) -> Result<Token, String> {
        let c = self.bump().expect("caller checked");
        let sym = match c {
            '.' => Symbol::Dot,
            ',' => Symbol::Comma,
            '(' => Symbol::LParen,
            ')' => Symbol::RParen,
            '[' => Symbol::LBracket,
            ']' => Symbol::RBracket,
            '{' => Symbol::LBrace,
            '}' => Symbol::RBrace,
            '+' => Symbol::Plus,
            '-' => Symbol::Minus,
            '*' => Symbol::Star,
            '/' => Symbol::Slash,
            '&' => Symbol::Amp,
            '|' => Symbol::Pipe,
            '~' => Symbol::Equiv,
            '=' => {
                // '==' is not FHIRPath, but accept it as '=' rather than
                // failing on a habit carried over from other languages.
                if self.peek() == Some('=') {
                    self.bump();
                }
                Symbol::Eq
            }
            '!' => match self.peek() {
                Some('=') => {
                    self.bump();
                    Symbol::NotEq
                }
                Some('~') => {
                    self.bump();
                    Symbol::NotEquiv
                }
                _ => return Err("'!' must be followed by '=' or '~'".into()),
            },
            '<' => {
                if self.peek() == Some('=') {
                    self.bump();
                    Symbol::Lte
                } else {
                    Symbol::Lt
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.bump();
                    Symbol::Gte
                } else {
                    Symbol::Gt
                }
            }
            other => return Err(format!("Unexpected character '{}'", other)),
        };
        Ok(Token::Symbol(sym))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(s: &str) -> Vec<Token> {
        tokenize(s).expect("lex")
    }

    #[test]
    fn lexes_a_simple_path() {
        assert_eq!(
            lex("Patient.name.given"),
            vec![
                Token::Ident("Patient".into()),
                Token::Symbol(Symbol::Dot),
                Token::Ident("name".into()),
                Token::Symbol(Symbol::Dot),
                Token::Ident("given".into()),
            ]
        );
    }

    #[test]
    fn distinguishes_integers_from_decimals() {
        assert_eq!(lex("3"), vec![Token::Integer(3)]);
        assert_eq!(lex("3.25"), vec![Token::Decimal(3.25)]);
    }

    #[test]
    fn a_dot_after_an_integer_is_a_path_separator() {
        assert_eq!(
            lex("1.toString()"),
            vec![
                Token::Integer(1),
                Token::Symbol(Symbol::Dot),
                Token::Ident("toString".into()),
                Token::Symbol(Symbol::LParen),
                Token::Symbol(Symbol::RParen),
            ]
        );
    }

    #[test]
    fn resolves_string_escapes() {
        assert_eq!(lex(r"'a\nb'"), vec![Token::Str("a\nb".into())]);
        assert_eq!(lex(r"'A'"), vec![Token::Str("A".into())]);
        assert_eq!(lex(r"'it\'s'"), vec![Token::Str("it's".into())]);
    }

    #[test]
    fn skips_both_comment_forms() {
        assert_eq!(lex("2 // trailing\n+ 2").len(), 3);
        assert_eq!(lex("2 /* inline */ + 2").len(), 3);
    }

    #[test]
    fn reads_datetime_literals() {
        assert_eq!(lex("@2015-02-04"), vec![Token::DateTime("2015-02-04".into())]);
        assert_eq!(
            lex("@2015-02-04T14:34:28Z"),
            vec![Token::DateTime("2015-02-04T14:34:28Z".into())]
        );
        assert_eq!(lex("@T14:34:28"), vec![Token::DateTime("T14:34:28".into())]);
    }

    #[test]
    fn a_plus_after_a_date_is_an_operator() {
        // Without the timezone carve-out this would swallow the '+' into
        // the literal and then fail to parse.
        let t = lex("@2015-02-04 + 1");
        assert_eq!(t[0], Token::DateTime("2015-02-04".into()));
        assert_eq!(t[1], Token::Symbol(Symbol::Plus));
    }

    #[test]
    fn reads_variables_and_environment_constants() {
        assert_eq!(lex("$this"), vec![Token::Variable("this".into())]);
        assert_eq!(lex("%resource"), vec![Token::EnvConstant("resource".into())]);
        assert_eq!(lex("%`vs-name`"), vec![Token::EnvConstant("vs-name".into())]);
    }

    #[test]
    fn backticks_delimit_identifiers_that_clash_with_keywords() {
        assert_eq!(lex("`div`"), vec![Token::Ident("div".into())]);
        assert_eq!(lex("div"), vec![Token::Keyword(Keyword::Div)]);
    }

    #[test]
    fn rejects_unterminated_literals() {
        assert!(tokenize("'abc").is_err());
        assert!(tokenize("/* abc").is_err());
        assert!(tokenize("#").is_err());
    }
}
