//! FHIRPath abstract syntax tree.

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Null,
    Bool(bool),
    Integer(i64),
    Decimal(f64),
    Str(String),
    /// `@`-literal text without the `@`. Precision is interpreted at
    /// evaluation time.
    DateTime(String),
    /// `4 'mg'` / `1 year` — value plus unit.
    Quantity(f64, String),
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
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
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
