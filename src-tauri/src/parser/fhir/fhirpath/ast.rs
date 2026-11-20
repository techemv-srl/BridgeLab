//! FHIRPath abstract syntax tree.

use serde_json::Number;

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Null,
    Bool(bool),
    Integer(i64),
    /// The number as written: it keeps its text (`12.500`), and with it the
    /// digits `precision()` and the boundaries count, exactly as a decimal
    /// read from a resource does.
    Decimal(Number),
    Str(String),
    /// `@`-literal text without the `@`. Precision is interpreted at
    /// evaluation time.
    DateTime(String),
    /// `4 'mg'` / `1 year` — value, as written, plus unit.
    Quantity(Number, String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Pos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    // multiplicative
    Mul,
    Div,
    IntDiv,
    Mod,
    // additive
    Add,
    Sub,
    Concat,
    // union
    Union,
    // comparison
    Lt,
    Gt,
    Lte,
    Gte,
    // equality
    Eq,
    NotEq,
    Equiv,
    NotEquiv,
    // membership
    In,
    Contains,
    // boolean
    And,
    Or,
    Xor,
    Implies,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeOp {
    Is,
    As,
}

/// `$this`, `$index`, `$total`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variable {
    This,
    Index,
    Total,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal),
    /// Member access. `base` is `None` for the leading element of a path,
    /// which resolves against the current focus.
    Member {
        base: Option<Box<Expr>>,
        name: String,
    },
    Function {
        base: Option<Box<Expr>>,
        name: String,
        args: Vec<Expr>,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    /// `first` followed by operators and their right operands, folded left
    /// to right: `a + b - c` is `(a + b) - c`. A list (`'a' | 'b' | …`,
    /// `x or y or …`) is one node however long it is, not a tree as deep as
    /// the list, so evaluating or dropping it does not recurse once per
    /// term.
    Binary {
        first: Box<Expr>,
        rest: Vec<(BinOp, Expr)>,
    },
    /// `x is Type` / `x as Type`. The type is kept as written, including any
    /// `System.`/`FHIR.` namespace prefix.
    TypeOp {
        op: TypeOp,
        operand: Box<Expr>,
        type_name: String,
    },
    Variable(Variable),
    /// `%resource`, `%context`, `%ucum`, …
    EnvConstant(String),
}
