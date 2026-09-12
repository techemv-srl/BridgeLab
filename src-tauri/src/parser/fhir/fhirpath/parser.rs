//! Recursive-descent / precedence-climbing parser for FHIRPath 2.0.
//!
//! Operator precedence, tightest first (FHIRPath spec §6):
//!
//! ```text
//!   .  (invocation)            handled as a postfix chain
//!   [] (indexer)               postfix
//!   unary + -
//!   *  /  div  mod
//!   +  -  &
//!   is  as
//!   |
//!   >  <  >=  <=
//!   =  ~  !=  !~
//!   in  contains
//!   and
//!   xor  or
//!   implies                    (lowest, right associative)
//! ```

use super::ast::*;
use super::lexer::{keyword_as_ident, tokenize, Keyword, Symbol, Token};

pub fn parse(input: &str) -> Result<Expr, String> {
    let tokens = tokenize(input)?;
    if tokens.is_empty() {
        return Err("Empty expression".into());
    }
    let mut p = Parser { tokens, pos: 0 };
    let expr = p.parse_expr(0)?;
    if p.pos < p.tokens.len() {
        return Err(format!("Unexpected trailing input at '{}'", p.tokens[p.pos]));
    }
    Ok(expr)
}

/// Binding power of each infix level; higher binds tighter.
const BP_IMPLIES: u8 = 1;
const BP_OR: u8 = 2;
const BP_AND: u8 = 3;
const BP_MEMBERSHIP: u8 = 4;
const BP_EQUALITY: u8 = 5;
const BP_COMPARISON: u8 = 6;
const BP_UNION: u8 = 7;
const BP_TYPE: u8 = 8;
const BP_ADDITIVE: u8 = 9;
const BP_MULTIPLICATIVE: u8 = 10;

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn bump(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn eat_symbol(&mut self, s: Symbol) -> bool {
        if self.peek() == Some(&Token::Symbol(s)) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect_symbol(&mut self, s: Symbol) -> Result<(), String> {
        if self.eat_symbol(s) {
            Ok(())
        } else {
            Err(match self.peek() {
                Some(t) => format!("Expected '{}' but found '{}'", Token::Symbol(s), t),
                None => format!("Expected '{}' but the expression ended", Token::Symbol(s)),
            })
        }
    }

    /// Precedence climbing: parse a unary/postfix operand, then fold in
    /// every infix operator that binds at least as tightly as `min_bp`.
    fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;

        while let Some(token) = self.peek() {

            // `is` / `as` take a type name rather than an expression on the
            // right, so they are folded here instead of through parse_expr.
            if let Token::Keyword(k @ (Keyword::Is | Keyword::As)) = token {
                if BP_TYPE < min_bp {
                    break;
                }
                let op = if *k == Keyword::Is { TypeOp::Is } else { TypeOp::As };
                self.bump();
                let type_name = self.parse_type_name()?;
                let typed = Expr::TypeOp {
                    op,
                    operand: Box::new(left),
                    type_name,
                };
                // Deliberate deviation from the letter of the grammar: there,
                // a type specifier is a qualified identifier, so
                // `value as Quantity.unit` would name a type called
                // "Quantity.unit" and always yield empty. Continuing the
                // postfix chain instead reads it as `(value as Quantity).unit`,
                // which is what anyone writing it means. Real namespaces
                // (`System.`, `FHIR.`) are still folded into the type name by
                // parse_type_name, so nothing legitimate is lost.
                left = self.parse_suffixes(typed)?;
                continue;
            }

            let Some((op, bp)) = infix_op(token) else { break };
            if bp < min_bp {
                break;
            }
            self.bump();
            // `implies` is right-associative; everything else is left.
            let next_min = if op == BinOp::Implies { bp } else { bp + 1 };
            let right = self.parse_expr(next_min)?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Token::Symbol(Symbol::Minus)) => {
                self.bump();
                let operand = self.parse_unary()?;
                Ok(fold_unary(UnaryOp::Neg, operand))
            }
            Some(Token::Symbol(Symbol::Plus)) => {
                self.bump();
                let operand = self.parse_unary()?;
                Ok(fold_unary(UnaryOp::Pos, operand))
            }
            _ => self.parse_postfix(),
        }
    }

    /// A primary term followed by any number of `.member`, `.func(...)` and
    /// `[index]` suffixes.
    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let expr = self.parse_primary()?;
        self.parse_suffixes(expr)
    }

    /// `.member`, `.func(...)` and `[index]` suffixes applied to `expr`.
    fn parse_suffixes(&mut self, mut expr: Expr) -> Result<Expr, String> {
        loop {
            if self.eat_symbol(Symbol::Dot) {
                expr = self.parse_invocation(Some(expr))?;
            } else if self.eat_symbol(Symbol::LBracket) {
                let index = self.parse_expr(0)?;
                self.expect_symbol(Symbol::RBracket)?;
                expr = Expr::Index {
                    base: Box::new(expr),
                    index: Box::new(index),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    /// A member name or function call, optionally hanging off `base`.
    fn parse_invocation(&mut self, base: Option<Expr>) -> Result<Expr, String> {
        let name = match self.bump() {
            Some(Token::Ident(n)) => n,
            // Keywords are ordinary member names in invocation position:
            // `Patient.contains`, `Group.characteristic.exclude`.
            Some(Token::Keyword(k)) => keyword_as_ident(k).to_string(),
            Some(Token::Variable(v)) if base.is_none() => return variable_expr(&v),
            Some(t) => return Err(format!("Expected a name but found '{}'", t)),
            None => return Err("Expected a name but the expression ended".into()),
        };

        if self.eat_symbol(Symbol::LParen) {
            let mut args = Vec::new();
            if !self.eat_symbol(Symbol::RParen) {
                loop {
                    args.push(self.parse_expr(0)?);
                    if self.eat_symbol(Symbol::Comma) {
                        continue;
                    }
                    self.expect_symbol(Symbol::RParen)?;
                    break;
                }
            }
            Ok(Expr::Function {
                base: base.map(Box::new),
                name,
                args,
            })
        } else {
            Ok(Expr::Member {
                base: base.map(Box::new),
                name,
            })
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek().cloned() {
            Some(Token::Symbol(Symbol::LParen)) => {
                self.bump();
                let inner = self.parse_expr(0)?;
                self.expect_symbol(Symbol::RParen)?;
                Ok(inner)
            }
            // `{}` is the empty collection literal.
            Some(Token::Symbol(Symbol::LBrace)) => {
                self.bump();
                self.expect_symbol(Symbol::RBrace)?;
                Ok(Expr::Literal(Literal::Null))
            }
            Some(Token::Integer(n)) => {
                self.bump();
                Ok(self.maybe_quantity(n as f64, Literal::Integer(n)))
            }
            Some(Token::Decimal(n)) => {
                self.bump();
                Ok(self.maybe_quantity(n, Literal::Decimal(n)))
            }
            Some(Token::Str(s)) => {
                self.bump();
                Ok(Expr::Literal(Literal::Str(s)))
            }
            Some(Token::DateTime(s)) => {
                self.bump();
                Ok(Expr::Literal(Literal::DateTime(s)))
            }
            Some(Token::Keyword(Keyword::True)) => {
                self.bump();
                Ok(Expr::Literal(Literal::Bool(true)))
            }
            Some(Token::Keyword(Keyword::False)) => {
                self.bump();
                Ok(Expr::Literal(Literal::Bool(false)))
            }
            Some(Token::Variable(v)) => {
                self.bump();
                variable_expr(&v)
            }
            Some(Token::EnvConstant(name)) => {
                self.bump();
                Ok(Expr::EnvConstant(name))
            }
            Some(Token::Ident(_)) | Some(Token::Keyword(_)) => self.parse_invocation(None),
            Some(t) => Err(format!("Unexpected '{}'", t)),
            None => Err("Unexpected end of expression".into()),
        }
    }

    /// A number immediately followed by a unit is a Quantity: `4 'mg'`,
    /// `1 year`, `3 months`.
    fn maybe_quantity(&mut self, value: f64, plain: Literal) -> Expr {
        match self.peek().cloned() {
            Some(Token::Str(unit)) => {
                self.bump();
                Expr::Literal(Literal::Quantity(value, unit))
            }
            Some(Token::Ident(word)) if is_time_unit(&word) => {
                self.bump();
                Expr::Literal(Literal::Quantity(value, word))
            }
            _ => Expr::Literal(plain),
        }
    }

    /// A (possibly namespace-qualified) type name: `String`, `System.String`,
    /// `FHIR.Patient`.
    fn parse_type_name(&mut self) -> Result<String, String> {
        let mut name = match self.bump() {
            Some(Token::Ident(n)) => n,
            Some(Token::Keyword(k)) => keyword_as_ident(k).to_string(),
            Some(t) => return Err(format!("Expected a type name but found '{}'", t)),
            None => return Err("Expected a type name but the expression ended".into()),
        };
        while self.peek() == Some(&Token::Symbol(Symbol::Dot)) {
            // Only consume the dot when a bare name follows it, so
            // `x as Quantity.value` still reads `.value` as member access…
            // which is exactly what a qualified name looks like. FHIRPath
            // resolves this by allowing at most one namespace qualifier.
            if name.contains('.') {
                break;
            }
            let Some(Token::Ident(next)) = self.tokens.get(self.pos + 1).cloned() else {
                break;
            };
            if !is_namespace(&name) {
                break;
            }
            self.bump();
            self.bump();
            name = format!("{}.{}", name, next);
        }
        Ok(name)
    }
}

/// Only these two namespaces exist in FHIRPath as used by FHIR, so a dot
/// after anything else is member access rather than qualification.
fn is_namespace(name: &str) -> bool {
    matches!(name, "System" | "FHIR")
}

fn is_time_unit(word: &str) -> bool {
    matches!(
        word,
        "year"
            | "years"
            | "month"
            | "months"
            | "week"
            | "weeks"
            | "day"
            | "days"
            | "hour"
            | "hours"
            | "minute"
            | "minutes"
            | "second"
            | "seconds"
            | "millisecond"
            | "milliseconds"
    )
}

fn variable_expr(name: &str) -> Result<Expr, String> {
    match name {
        "this" => Ok(Expr::Variable(Variable::This)),
        "index" => Ok(Expr::Variable(Variable::Index)),
        "total" => Ok(Expr::Variable(Variable::Total)),
        other => Err(format!("Unknown variable ${}", other)),
    }
}

/// Fold a sign directly into a numeric literal so `-1` is a literal rather
/// than a negation of one — it keeps integer/decimal typing intact.
fn fold_unary(op: UnaryOp, operand: Expr) -> Expr {
    match (op, &operand) {
        (UnaryOp::Pos, _) => operand,
        (UnaryOp::Neg, Expr::Literal(Literal::Integer(n))) => Expr::Literal(Literal::Integer(-n)),
        (UnaryOp::Neg, Expr::Literal(Literal::Decimal(n))) => Expr::Literal(Literal::Decimal(-n)),
        (UnaryOp::Neg, Expr::Literal(Literal::Quantity(v, u))) => {
            Expr::Literal(Literal::Quantity(-v, u.clone()))
        }
        _ => Expr::Unary {
            op,
            operand: Box::new(operand),
        },
    }
}

fn infix_op(token: &Token) -> Option<(BinOp, u8)> {
    Some(match token {
        Token::Symbol(Symbol::Star) => (BinOp::Mul, BP_MULTIPLICATIVE),
        Token::Symbol(Symbol::Slash) => (BinOp::Div, BP_MULTIPLICATIVE),
        Token::Keyword(Keyword::Div) => (BinOp::IntDiv, BP_MULTIPLICATIVE),
        Token::Keyword(Keyword::Mod) => (BinOp::Mod, BP_MULTIPLICATIVE),

        Token::Symbol(Symbol::Plus) => (BinOp::Add, BP_ADDITIVE),
        Token::Symbol(Symbol::Minus) => (BinOp::Sub, BP_ADDITIVE),
        Token::Symbol(Symbol::Amp) => (BinOp::Concat, BP_ADDITIVE),

        Token::Symbol(Symbol::Pipe) => (BinOp::Union, BP_UNION),

        Token::Symbol(Symbol::Lt) => (BinOp::Lt, BP_COMPARISON),
        Token::Symbol(Symbol::Gt) => (BinOp::Gt, BP_COMPARISON),
        Token::Symbol(Symbol::Lte) => (BinOp::Lte, BP_COMPARISON),
        Token::Symbol(Symbol::Gte) => (BinOp::Gte, BP_COMPARISON),

        Token::Symbol(Symbol::Eq) => (BinOp::Eq, BP_EQUALITY),
        Token::Symbol(Symbol::NotEq) => (BinOp::NotEq, BP_EQUALITY),
        Token::Symbol(Symbol::Equiv) => (BinOp::Equiv, BP_EQUALITY),
        Token::Symbol(Symbol::NotEquiv) => (BinOp::NotEquiv, BP_EQUALITY),

        Token::Keyword(Keyword::In) => (BinOp::In, BP_MEMBERSHIP),
        Token::Keyword(Keyword::Contains) => (BinOp::Contains, BP_MEMBERSHIP),

        Token::Keyword(Keyword::And) => (BinOp::And, BP_AND),
        Token::Keyword(Keyword::Or) => (BinOp::Or, BP_OR),
        Token::Keyword(Keyword::Xor) => (BinOp::Xor, BP_OR),
        Token::Keyword(Keyword::Implies) => (BinOp::Implies, BP_IMPLIES),

        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Expr {
        parse(s).unwrap_or_else(|e| panic!("parse {:?}: {}", s, e))
    }

    /// Render an AST in fully-parenthesised prefix form, so precedence and
    /// associativity are visible in a single string.
    fn sexpr(e: &Expr) -> String {
        match e {
            Expr::Literal(Literal::Null) => "{}".into(),
            Expr::Literal(Literal::Bool(b)) => b.to_string(),
            Expr::Literal(Literal::Integer(n)) => n.to_string(),
            Expr::Literal(Literal::Decimal(n)) => n.to_string(),
            Expr::Literal(Literal::Str(s)) => format!("'{}'", s),
            Expr::Literal(Literal::DateTime(s)) => format!("@{}", s),
            Expr::Literal(Literal::Quantity(v, u)) => format!("{} '{}'", v, u),
            Expr::Member { base, name } => match base {
                Some(b) => format!("(. {} {})", sexpr(b), name),
                None => name.clone(),
            },
            Expr::Function { base, name, args } => {
                let a: Vec<String> = args.iter().map(sexpr).collect();
                let call = if a.is_empty() {
                    format!("{}()", name)
                } else {
                    format!("{}({})", name, a.join(", "))
                };
                match base {
                    Some(b) => format!("(. {} {})", sexpr(b), call),
                    None => call,
                }
            }
            Expr::Index { base, index } => format!("([] {} {})", sexpr(base), sexpr(index)),
            Expr::Unary { op, operand } => {
                let o = if *op == UnaryOp::Neg { "-" } else { "+" };
                format!("({} {})", o, sexpr(operand))
            }
            Expr::Binary { op, left, right } => {
                format!("({:?} {} {})", op, sexpr(left), sexpr(right))
            }
            Expr::TypeOp { op, operand, type_name } => {
                format!("({:?} {} {})", op, sexpr(operand), type_name)
            }
            Expr::Variable(v) => format!("${:?}", v),
            Expr::EnvConstant(n) => format!("%{}", n),
        }
    }

    #[test]
    fn parses_a_path_as_a_left_leaning_chain() {
        assert_eq!(sexpr(&p("Patient.name.given")), "(. (. Patient name) given)");
    }

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        assert_eq!(sexpr(&p("1 + 2 * 3")), "(Add 1 (Mul 2 3))");
        assert_eq!(sexpr(&p("(1 + 2) * 3")), "(Mul (Add 1 2) 3)");
    }

    #[test]
    fn comparison_binds_tighter_than_and_which_binds_tighter_than_or() {
        assert_eq!(
            sexpr(&p("a > 1 and b < 2 or c = 3")),
            "(Or (And (Gt a 1) (Lt b 2)) (Eq c 3))"
        );
    }

    #[test]
    fn equality_binds_tighter_than_membership() {
        assert_eq!(sexpr(&p("a = 1 in b")), "(In (Eq a 1) b)");
    }

    #[test]
    fn implies_is_right_associative_and_lowest() {
        assert_eq!(
            sexpr(&p("a implies b implies c")),
            "(Implies a (Implies b c))"
        );
        assert_eq!(sexpr(&p("a or b implies c")), "(Implies (Or a b) c)");
    }

    #[test]
    fn arithmetic_is_left_associative() {
        assert_eq!(sexpr(&p("1 - 2 - 3")), "(Sub (Sub 1 2) 3)");
    }

    #[test]
    fn union_sits_between_type_ops_and_comparison() {
        assert_eq!(sexpr(&p("a | b = c")), "(Eq (Union a b) c)");
        assert_eq!(sexpr(&p("a + b | c")), "(Union (Add a b) c)");
    }

    #[test]
    fn parses_function_calls_with_arguments() {
        assert_eq!(
            sexpr(&p("Bundle.entry.where(resource.id = 'p1').count()")),
            "(. (. (. Bundle entry) where((Eq (. resource id) 'p1'))) count())"
        );
        assert_eq!(sexpr(&p("substring(1, 2)")), "substring(1, 2)");
    }

    #[test]
    fn parses_indexers_and_chains_after_them() {
        assert_eq!(
            sexpr(&p("Patient.name[0].given[1]")),
            "([] (. ([] (. Patient name) 0) given) 1)"
        );
    }

    #[test]
    fn folds_a_sign_into_a_numeric_literal() {
        assert_eq!(sexpr(&p("-1")), "-1");
        assert_eq!(sexpr(&p("1 + -2")), "(Add 1 -2)");
        assert_eq!(sexpr(&p("-a")), "(- a)");
    }

    #[test]
    fn parses_type_operators() {
        assert_eq!(sexpr(&p("value is Quantity")), "(Is value Quantity)");
        assert_eq!(sexpr(&p("value as System.String")), "(As value System.String)");
        // A dot after a non-namespace type name is member access again.
        assert_eq!(
            sexpr(&p("value as Quantity.unit")),
            "(. (As value Quantity) unit)"
        );
    }

    #[test]
    fn keywords_are_member_names_in_invocation_position() {
        assert_eq!(sexpr(&p("Patient.contains")), "(. Patient contains)");
        assert_eq!(sexpr(&p("a.as")), "(. a as)");
    }

    #[test]
    fn parses_the_empty_collection_literal() {
        assert_eq!(sexpr(&p("{}")), "{}");
        assert_eq!(sexpr(&p("a = {}")), "(Eq a {})");
    }

    #[test]
    fn parses_quantities() {
        assert_eq!(sexpr(&p("4 'mg'")), "4 'mg'");
        assert_eq!(sexpr(&p("1 year")), "1 'year'");
        // A bare identifier that is not a time unit stays a separate term.
        assert!(parse("4 mg").is_err());
    }

    #[test]
    fn parses_variables_and_environment_constants() {
        assert_eq!(sexpr(&p("$this.name")), "(. $This name)");
        assert_eq!(sexpr(&p("%resource")), "%resource");
        assert_eq!(sexpr(&p("where($index > 2)")), "where((Gt $Index 2))");
    }

    #[test]
    fn reports_structural_errors() {
        assert!(parse("Patient.name[").is_err());
        assert!(parse("Patient..name").is_err());
        assert!(parse("1 +").is_err());
        assert!(parse("(1 + 2").is_err());
        assert!(parse("").is_err());
        assert!(parse("$nope").is_err());
    }
}
