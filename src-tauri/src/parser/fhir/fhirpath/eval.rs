//! FHIRPath evaluator.
//!
//! Everything in FHIRPath is a collection, so every expression evaluates to
//! a `Vec<Value>`. The empty collection doubles as "no value" and propagates
//! through most operators; operators that need a single value use
//! [`singleton`], which errors on a collection of more than one.

use std::cmp::Ordering;
use std::collections::HashMap;

use serde_json::{Map, Value};

use super::ast::*;
use super::types;

/// Evaluation environment: the resource the expression started from plus any
/// lambda bindings currently in scope.
pub struct Env<'a> {
    /// `%resource` / `%context` — the resource the evaluation started from.
    pub root: &'a Value,
    /// `$this`, when inside a lambda argument.
    pub this: Option<Vec<Value>>,
    /// `$index`, when inside a lambda argument.
    pub index: Option<i64>,
    /// `$total`, inside `aggregate()`.
    pub total: Option<Vec<Value>>,
    /// Lines emitted by `trace()`, in order.
    pub trace: Vec<(String, Vec<Value>)>,
}

impl<'a> Env<'a> {
    pub fn new(root: &'a Value) -> Self {
        Env {
            root,
            this: None,
            index: None,
            total: None,
            trace: Vec::new(),
        }
    }

    /// The context node ordinary function arguments are evaluated against:
    /// the item a lambda is iterating over, or the root resource.
    ///
    /// This is *not* the function's input collection, which is why
    /// `name.given.combine(name.family)` finds the family names — `name`
    /// resolves from the context, not from the given names the function
    /// happens to be operating on.
    pub fn context(&self) -> Vec<Value> {
        self.this
            .clone()
            .unwrap_or_else(|| vec![self.root.clone()])
    }

    /// Re-bind `$this`/`$index` for one iteration of a lambda.
    fn with_item<T>(
        &mut self,
        item: &Value,
        index: usize,
        f: impl FnOnce(&mut Env<'a>) -> T,
    ) -> T {
        let saved_this = self.this.take();
        let saved_index = self.index.take();
        self.this = Some(vec![item.clone()]);
        self.index = Some(index as i64);
        let out = f(self);
        self.this = saved_this;
        self.index = saved_index;
        out
    }
}

pub fn eval(expr: &Expr, focus: &[Value], env: &mut Env) -> Result<Vec<Value>, String> {
    match expr {
        Expr::Literal(lit) => Ok(literal_value(lit)),

        Expr::Variable(v) => match v {
            // Outside a lambda, `$this` is the resource the evaluation
            // started from — that is what makes `x.subsetOf($this.name)`
            // mean what it says.
            Variable::This => Ok(env
                .this
                .clone()
                .unwrap_or_else(|| vec![env.root.clone()])),
            Variable::Index => Ok(env.index.map(|i| vec![Value::from(i)]).unwrap_or_default()),
            Variable::Total => Ok(env.total.clone().unwrap_or_default()),
        },

        Expr::EnvConstant(name) => env_constant(name, env),

        Expr::Member { base, name } => {
            let target = resolve_base(base.as_deref(), focus, env)?;
            Ok(member_access(&target, name, base.is_none(), env.root))
        }

        Expr::Function { base, name, args } => {
            let target = resolve_base(base.as_deref(), focus, env)?;
            super::functions::call(name, args, &target, env)
        }

        Expr::Index { base, index } => {
            let target = eval(base, focus, env)?;
            let idx = eval(index, focus, env)?;
            let Some(i) = singleton(&idx)? else {
                return Ok(vec![]);
            };
            let Some(i) = i.as_i64() else {
                return Err("Index must be an integer".into());
            };
            if i < 0 {
                return Ok(vec![]);
            }
            Ok(target.get(i as usize).cloned().into_iter().collect())
        }

        Expr::Unary { op, operand } => {
            let vals = eval(operand, focus, env)?;
            let Some(v) = singleton(&vals)? else {
                return Ok(vec![]);
            };
            match op {
                UnaryOp::Pos => Ok(vec![v.clone()]),
                UnaryOp::Neg => negate(v).map(|v| vec![v]),
            }
        }

        Expr::TypeOp { op, operand, type_name } => {
            let vals = eval(operand, focus, env)?;
            let Some(v) = singleton(&vals)? else {
                return Ok(vec![]);
            };
            let matches = types::is_type(v, type_name);
            match op {
                TypeOp::Is => Ok(vec![Value::Bool(matches)]),
                TypeOp::As => Ok(if matches { vec![v.clone()] } else { vec![] }),
            }
        }

        Expr::Binary { op, left, right } => binary(*op, left, right, focus, env),
    }
}

/// The focus a member access or function call applies to: the base
/// expression's result, or the incoming focus for a leading element.
fn resolve_base(
    base: Option<&Expr>,
    focus: &[Value],
    env: &mut Env,
) -> Result<Vec<Value>, String> {
    match base {
        Some(b) => eval(b, focus, env),
        None => Ok(focus.to_vec()),
    }
}

fn env_constant(name: &str, env: &Env) -> Result<Vec<Value>, String> {
    match name {
        "resource" | "context" | "rootResource" => Ok(vec![env.root.clone()]),
        "ucum" => Ok(vec![Value::from("http://unitsofmeasure.org")]),
        "sct" => Ok(vec![Value::from("http://snomed.info/sct")]),
        "loinc" => Ok(vec![Value::from("http://loinc.org")]),
        // `%vs-<name>` and `%ext-<name>` expand to the canonical URL of a
        // core FHIR value set or extension.
        other => match other.split_once('-') {
            Some(("vs", name)) => Ok(vec![Value::from(format!(
                "http://hl7.org/fhir/ValueSet/{}",
                name
            ))]),
            Some(("ext", name)) => Ok(vec![Value::from(format!(
                "http://hl7.org/fhir/StructureDefinition/{}",
                name
            ))]),
            _ => Err(format!("Unknown environment constant %{}", other)),
        },
    }
}

fn literal_value(lit: &Literal) -> Vec<Value> {
    match lit {
        Literal::Null => vec![],
        Literal::Bool(b) => vec![Value::Bool(*b)],
        Literal::Integer(n) => vec![Value::from(*n)],
        Literal::Decimal(n) => vec![Value::from(*n)],
        Literal::Str(s) => vec![Value::from(s.clone())],
        Literal::DateTime(s) => vec![Value::from(s.clone())],
        Literal::Quantity(v, u) => {
            let mut m = Map::new();
            m.insert("value".into(), Value::from(*v));
            m.insert("unit".into(), Value::from(u.clone()));
            vec![Value::Object(m)]
        }
    }
}

// ---------------------------------------------------------------------------
// Path navigation
// ---------------------------------------------------------------------------

/// Resolve `name` against every item of `focus`.
///
/// `leading` marks the first element of a path, where a name matching the
/// resource type acts as a filter on the context rather than a member lookup
/// (`Patient.name` evaluated with a Patient as context).
pub fn member_access(focus: &[Value], name: &str, leading: bool, root: &Value) -> Vec<Value> {
    if leading {
        // `Patient.name` / bare `Patient` against a Patient resource.
        if focus.len() == 1 && resource_type_of(&focus[0]) == Some(name) {
            return focus.to_vec();
        }
        // A leading type name with no focus of its own resolves against the
        // resource the evaluation started from.
        if focus.is_empty() && resource_type_of(root) == Some(name) {
            return vec![root.clone()];
        }
    }

    let mut out = Vec::new();
    for item in focus {
        let Some(obj) = item.as_object() else { continue };
        match obj.get(name) {
            Some(Value::Array(arr)) => out.extend(arr.iter().cloned()),
            Some(v) => out.push(v.clone()),
            None => {
                // FHIR choice elements are serialised with the type appended
                // to the element name, so `Observation.value` has to find
                // `valueQuantity`, `valueString`, …
                if let Some(v) = choice_element(obj, name) {
                    match v {
                        Value::Array(arr) => out.extend(arr.iter().cloned()),
                        v => out.push(v.clone()),
                    }
                }
            }
        }
    }
    out
}

/// A `<name><Type>` key, e.g. `valueQuantity` for `value`.
fn choice_element<'a>(obj: &'a Map<String, Value>, name: &str) -> Option<&'a Value> {
    obj.iter()
        .find(|(k, _)| {
            k.len() > name.len()
                && k.starts_with(name)
                && k[name.len()..].starts_with(|c: char| c.is_ascii_uppercase())
        })
        .map(|(_, v)| v)
}

fn resource_type_of(v: &Value) -> Option<&str> {
    v.get("resourceType").and_then(|t| t.as_str())
}

// ---------------------------------------------------------------------------
// Operators
// ---------------------------------------------------------------------------

fn binary(
    op: BinOp,
    left: &Expr,
    right: &Expr,
    focus: &[Value],
    env: &mut Env,
) -> Result<Vec<Value>, String> {
    // Boolean operators short-circuit on three-valued logic, so their
    // operands are evaluated lazily.
    match op {
        BinOp::And => {
            let l = as_tri(&eval(left, focus, env)?)?;
            if l == Some(false) {
                return Ok(vec![Value::Bool(false)]);
            }
            let r = as_tri(&eval(right, focus, env)?)?;
            return Ok(match (l, r) {
                (_, Some(false)) => vec![Value::Bool(false)],
                (Some(true), Some(true)) => vec![Value::Bool(true)],
                _ => vec![],
            });
        }
        BinOp::Or => {
            let l = as_tri(&eval(left, focus, env)?)?;
            if l == Some(true) {
                return Ok(vec![Value::Bool(true)]);
            }
            let r = as_tri(&eval(right, focus, env)?)?;
            return Ok(match (l, r) {
                (_, Some(true)) => vec![Value::Bool(true)],
                (Some(false), Some(false)) => vec![Value::Bool(false)],
                _ => vec![],
            });
        }
        BinOp::Implies => {
            let l = as_tri(&eval(left, focus, env)?)?;
            if l == Some(false) {
                return Ok(vec![Value::Bool(true)]);
            }
            let r = as_tri(&eval(right, focus, env)?)?;
            return Ok(match (l, r) {
                (_, Some(true)) => vec![Value::Bool(true)],
                (Some(true), Some(false)) => vec![Value::Bool(false)],
                _ => vec![],
            });
        }
        BinOp::Xor => {
            let l = as_tri(&eval(left, focus, env)?)?;
            let r = as_tri(&eval(right, focus, env)?)?;
            return Ok(match (l, r) {
                (Some(a), Some(b)) => vec![Value::Bool(a != b)],
                _ => vec![],
            });
        }
        _ => {}
    }

    let l = eval(left, focus, env)?;
    let r = eval(right, focus, env)?;

    match op {
        BinOp::Union => Ok(dedup(l.into_iter().chain(r).collect())),

        BinOp::Eq => collection_equality(&l, &r).map(opt_bool),
        BinOp::NotEq => collection_equality(&l, &r).map(|o| opt_bool(o.map(|b| !b))),
        BinOp::Equiv => Ok(vec![Value::Bool(collection_equivalence(&l, &r))]),
        BinOp::NotEquiv => Ok(vec![Value::Bool(!collection_equivalence(&l, &r))]),

        BinOp::Lt | BinOp::Gt | BinOp::Lte | BinOp::Gte => {
            let (Some(a), Some(b)) = (singleton(&l)?, singleton(&r)?) else {
                return Ok(vec![]);
            };
            let Some(ord) = compare(a, b)? else {
                return Ok(vec![]);
            };
            let result = match op {
                BinOp::Lt => ord == Ordering::Less,
                BinOp::Gt => ord == Ordering::Greater,
                BinOp::Lte => ord != Ordering::Greater,
                _ => ord != Ordering::Less,
            };
            Ok(vec![Value::Bool(result)])
        }

        BinOp::In => {
            if l.is_empty() {
                return Ok(vec![]);
            }
            let Some(item) = singleton(&l)? else {
                return Ok(vec![]);
            };
            Ok(vec![Value::Bool(r.iter().any(|v| equal(item, v) == Some(true)))])
        }
        BinOp::Contains => {
            if r.is_empty() {
                return Ok(vec![]);
            }
            let Some(item) = singleton(&r)? else {
                return Ok(vec![]);
            };
            Ok(vec![Value::Bool(l.iter().any(|v| equal(v, item) == Some(true)))])
        }

        BinOp::Concat => {
            // '&' treats an empty operand as an empty string, which is the
            // whole reason it exists next to '+'.
            let a = l.first().map(string_of).unwrap_or_default();
            let b = r.first().map(string_of).unwrap_or_default();
            if l.len() > 1 || r.len() > 1 {
                return Err("'&' expects single values".into());
            }
            Ok(vec![Value::from(format!("{}{}", a, b))])
        }

        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::IntDiv | BinOp::Mod => {
            let (Some(a), Some(b)) = (singleton(&l)?, singleton(&r)?) else {
                return Ok(vec![]);
            };
            arithmetic(op, a, b)
        }

        BinOp::And | BinOp::Or | BinOp::Xor | BinOp::Implies => unreachable!("handled above"),
    }
}

fn arithmetic(op: BinOp, a: &Value, b: &Value) -> Result<Vec<Value>, String> {
    // A date or time plus a duration moves the instant, keeping precision.
    if matches!(op, BinOp::Add | BinOp::Sub) {
        if let (Some(text), true) = (a.as_str(), types::is_quantity(b)) {
            if let Some(temporal) = types::as_temporal(text) {
                let (amount, unit) = types::quantity_parts(b).unwrap_or((0.0, String::new()));
                let raw = types::quantity_raw_unit(b).unwrap_or_default();
                // Only durations are meaningful here, and a calendar year or
                // month has to be written as one: UCUM 'a' and 'mo' are fixed
                // lengths that do not line up with calendar arithmetic.
                if types::ucum_for_time_unit(&raw).is_none()
                    && matches!(unit.as_str(), "a" | "mo")
                {
                    return Err(format!(
                        "A date needs a calendar duration: write `{} year(s)` or \
                         `{} month(s)` rather than the UCUM unit '{}'",
                        amount, amount, raw
                    ));
                }
                let signed = if op == BinOp::Sub { -amount } else { amount };
                return match temporal.add_duration(signed, &unit) {
                    Some(t) => Ok(vec![Value::from(t.to_literal())]),
                    None => Err(format!(
                        "'{}' is not a duration, so it cannot be added to a date",
                        raw
                    )),
                };
            }
        }
    }

    // String concatenation with '+'.
    if let (Some(x), Some(y)) = (a.as_str(), b.as_str()) {
        return match op {
            BinOp::Add => Ok(vec![Value::from(format!("{}{}", x, y))]),
            _ => Err(format!("Operator {:?} is not defined for strings", op)),
        };
    }

    if types::is_quantity(a) || types::is_quantity(b) {
        return quantity_arithmetic(op, a, b);
    }

    let (Some(x), Some(y)) = (a.as_f64(), b.as_f64()) else {
        return Err(format!(
            "Operator {:?} expects numbers, got {} and {}",
            op,
            types::type_name(a),
            types::type_name(b)
        ));
    };
    let both_int = a.is_i64() && b.is_i64();

    // FHIRPath decimals are exact; f64 is not. Rounding the result to the
    // scale exact decimal arithmetic would have produced keeps
    // `1.8 - 1.2 = 0.6` true instead of 0.5999999999999999.
    let scale = |places: u32, v: f64| round_to(v, places);
    let (sa, sb) = (decimals(a), decimals(b));

    let result = match op {
        BinOp::Add => scale(sa.max(sb), x + y),
        BinOp::Sub => scale(sa.max(sb), x - y),
        BinOp::Mul => scale(sa + sb, x * y),
        BinOp::Div => {
            if y == 0.0 {
                return Ok(vec![]);
            }
            // '/' always produces a decimal, even for two integers.
            return Ok(vec![Value::from(x / y)]);
        }
        BinOp::IntDiv => {
            if y == 0.0 {
                return Ok(vec![]);
            }
            let q = (x / y).trunc();
            return Ok(vec![if both_int {
                Value::from(q as i64)
            } else {
                Value::from(q)
            }]);
        }
        BinOp::Mod => {
            if y == 0.0 {
                return Ok(vec![]);
            }
            let m = scale(sa.max(sb), x % y);
            return Ok(vec![if both_int {
                Value::from(m as i64)
            } else {
                Value::from(m)
            }]);
        }
        _ => unreachable!(),
    };

    Ok(vec![if both_int {
        Value::from(result as i64)
    } else {
        Value::from(result)
    }])
}

fn quantity_arithmetic(op: BinOp, a: &Value, b: &Value) -> Result<Vec<Value>, String> {
    // Scaling a quantity by a plain number keeps the unit.
    if let (true, Some(n)) = (types::is_quantity(a), b.as_f64()) {
        let (av, au) = types::quantity_parts(a).unwrap();
        return match op {
            BinOp::Mul => Ok(vec![types::make_quantity(av * n, &au)]),
            BinOp::Div if n != 0.0 => Ok(vec![types::make_quantity(av / n, &au)]),
            BinOp::Div => Ok(vec![]),
            _ => Err(format!("Operator {:?} needs two quantities", op)),
        };
    }
    if let (Some(n), true) = (a.as_f64(), types::is_quantity(b)) {
        let (bv, bu) = types::quantity_parts(b).unwrap();
        return match op {
            BinOp::Mul => Ok(vec![types::make_quantity(n * bv, &bu)]),
            _ => Err(format!("Operator {:?} needs two quantities", op)),
        };
    }

    let (Some((av, au)), Some((bv, bu))) = (types::quantity_parts(a), types::quantity_parts(b))
    else {
        return Err(format!(
            "Operator {:?} is not defined between a {} and a {}",
            op,
            types::type_name(a),
            types::type_name(b)
        ));
    };

    match op {
        BinOp::Add | BinOp::Sub => {
            // Convert the right operand into the left's unit; units outside
            // the conversion table give the empty collection rather than a
            // wrong number.
            let Some(converted) = types::convert_quantity(bv, &bu, &au) else {
                return Ok(vec![]);
            };
            let v = if op == BinOp::Add {
                av + converted
            } else {
                av - converted
            };
            Ok(vec![types::make_quantity(v, &au)])
        }
        BinOp::Mul | BinOp::Div => Err(format!(
            "Operator {:?} between two quantities produces a compound unit, \
             which needs a full UCUM engine",
            op
        )),
        _ => Err(format!("Operator {:?} is not defined for quantities", op)),
    }
}

fn negate(v: &Value) -> Result<Value, String> {
    if let Some(n) = v.as_i64() {
        return Ok(Value::from(-n));
    }
    if let Some(n) = v.as_f64() {
        return Ok(Value::from(-n));
    }
    if let Some((val, unit)) = types::quantity_parts(v) {
        return Ok(types::make_quantity(-val, &unit));
    }
    Err(format!("Cannot negate a {}", types::type_name(v)))
}

// ---------------------------------------------------------------------------
// Equality, equivalence and ordering
// ---------------------------------------------------------------------------

/// `=` over collections. `None` means the result is the empty collection:
/// either operand was empty, or an item comparison was indeterminate.
fn collection_equality(l: &[Value], r: &[Value]) -> Result<Option<bool>, String> {
    if l.is_empty() || r.is_empty() {
        return Ok(None);
    }
    if l.len() != r.len() {
        return Ok(Some(false));
    }
    let mut indeterminate = false;
    for (a, b) in l.iter().zip(r.iter()) {
        match equal(a, b) {
            Some(false) => return Ok(Some(false)),
            Some(true) => {}
            None => indeterminate = true,
        }
    }
    Ok(if indeterminate { None } else { Some(true) })
}

/// `~` over collections: order-insensitive, and empty ~ empty is true.
fn collection_equivalence(l: &[Value], r: &[Value]) -> bool {
    if l.len() != r.len() {
        return false;
    }
    let mut used = vec![false; r.len()];
    for a in l {
        let Some(pos) = r
            .iter()
            .enumerate()
            .position(|(i, b)| !used[i] && equivalent(a, b))
        else {
            return false;
        };
        used[pos] = true;
    }
    true
}

/// Single-value `=`. `None` when the comparison is indeterminate (dates of
/// differing precision that agree so far).
pub fn equal(a: &Value, b: &Value) -> Option<bool> {
    match (a, b) {
        (Value::Number(_), Value::Number(_)) => Some(a.as_f64() == b.as_f64()),
        (Value::Bool(x), Value::Bool(y)) => Some(x == y),
        (Value::String(x), Value::String(y)) => {
            if let (Some(dx), Some(dy)) = (types::as_temporal(x), types::as_temporal(y)) {
                return types::temporal_equal(&dx, &dy);
            }
            Some(x == y)
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Some(false);
            }
            let mut indeterminate = false;
            for (i, j) in x.iter().zip(y.iter()) {
                match equal(i, j) {
                    Some(false) => return Some(false),
                    None => indeterminate = true,
                    Some(true) => {}
                }
            }
            if indeterminate {
                None
            } else {
                Some(true)
            }
        }
        (Value::Object(x), Value::Object(y)) => {
            // Quantities compare by magnitude and normalised unit, so
            // `1 week` equals `1 'wk'` despite different JSON.
            if types::is_quantity(a) && types::is_quantity(b) {
                let (av, au) = types::quantity_parts(a)?;
                let (bv, bu) = types::quantity_parts(b)?;
                // A calendar year or month is not a fixed number of seconds,
                // so it cannot be measured against the UCUM unit of the same
                // name: `1 month = 1 'mo'` is unknowable, not false.
                let calendar = |v: &Value| {
                    types::quantity_raw_unit(v)
                        .is_some_and(|u| types::ucum_for_time_unit(&u).is_some())
                };
                if matches!(au.as_str(), "a" | "mo") && calendar(a) != calendar(b) {
                    return None;
                }
                // Units outside the conversion table are unknowable.
                return types::convert_quantity(bv, &bu, &au).map(|c| av == c);
            }
            if x.len() != y.len() {
                return Some(false);
            }
            for (k, v) in x {
                match y.get(k) {
                    Some(other) => match equal(v, other) {
                        Some(true) => {}
                        Some(false) => return Some(false),
                        None => return None,
                    },
                    None => return Some(false),
                }
            }
            Some(true)
        }
        (Value::Null, Value::Null) => Some(true),
        _ => Some(false),
    }
}

/// Single-value `~`: strings compare case- and whitespace-insensitively,
/// decimals to the lesser precision, and there is no indeterminate result.
pub fn equivalent(a: &Value, b: &Value) -> bool {
    // Quantities are equivalent when they convert to the same magnitude at
    // the coarser of the two precisions.
    if types::is_quantity(a) && types::is_quantity(b) {
        let (Some((av, au)), Some((bv, bu))) = (types::quantity_parts(a), types::quantity_parts(b))
        else {
            return false;
        };
        let Some(converted) = types::convert_quantity(bv, &bu, &au) else {
            return au == bu && av == bv;
        };
        let places = decimals(a.get("value").unwrap_or(&Value::Null))
            .min(decimals(b.get("value").unwrap_or(&Value::Null)));
        return round_to(av, places) == round_to(converted, places);
    }

    match (a, b) {
        (Value::String(x), Value::String(y)) => {
            if let (Some(tx), Some(ty)) = (types::as_temporal(x), types::as_temporal(y)) {
                return types::temporal_equivalent(&tx, &ty);
            }
            normalize_ws(x) == normalize_ws(y)
        }
        (Value::Number(_), Value::Number(_)) => {
            let (x, y) = (a.as_f64().unwrap_or(f64::NAN), b.as_f64().unwrap_or(f64::NAN));
            let places = decimals(a).min(decimals(b));
            round_to(x, places) == round_to(y, places)
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(i, j)| equivalent(i, j))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|other| equivalent(v, other)))
        }
        _ => equal(a, b).unwrap_or(false),
    }
}

fn normalize_ws(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn decimals(v: &Value) -> u32 {
    let s = v.to_string();
    match s.split_once('.') {
        Some((_, frac)) => frac.len() as u32,
        None => 0,
    }
}

fn round_to(v: f64, places: u32) -> f64 {
    let f = 10f64.powi(places as i32);
    (v * f).round() / f
}

/// Ordering for `<`, `>`, `<=`, `>=`. `None` means indeterminate.
pub fn compare(a: &Value, b: &Value) -> Result<Option<Ordering>, String> {
    match (a, b) {
        (Value::Number(_), Value::Number(_)) => {
            let (x, y) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            Ok(x.partial_cmp(&y))
        }
        (Value::String(x), Value::String(y)) => {
            if let (Some(dx), Some(dy)) = (types::as_temporal(x), types::as_temporal(y)) {
                return Ok(types::temporal_compare(&dx, &dy));
            }
            Ok(Some(x.cmp(y)))
        }
        _ if types::is_quantity(a) && types::is_quantity(b) => {
            let (av, au) = types::quantity_parts(a).unwrap();
            let (bv, bu) = types::quantity_parts(b).unwrap();
            match types::convert_quantity(bv, &bu, &au) {
                Some(converted) => Ok(av.partial_cmp(&converted)),
                // Units outside the conversion table: unknowable.
                None => Ok(None),
            }
        }
        _ => Err(format!(
            "Cannot order a {} against a {}",
            types::type_name(a),
            types::type_name(b)
        )),
    }
}

// ---------------------------------------------------------------------------
// Helpers shared with the function library
// ---------------------------------------------------------------------------

/// The single value of a collection. `None` for the empty collection; an
/// error for more than one, which is what the FHIRPath spec requires of
/// operators and functions that expect a singleton.
pub fn singleton(vals: &[Value]) -> Result<Option<&Value>, String> {
    match vals.len() {
        0 => Ok(None),
        1 => Ok(Some(&vals[0])),
        n => Err(format!(
            "Expected a single value but the expression produced {}",
            n
        )),
    }
}

/// Three-valued boolean view of a collection: `None` is the empty collection.
///
/// Implements the spec's *singleton evaluation of collections*: a single item
/// converts to Boolean when it has a Boolean conversion (`1`/`0`,
/// `'true'`/`'false'`), and any other single item stands in for `true`.
pub fn as_tri(vals: &[Value]) -> Result<Option<bool>, String> {
    match singleton(vals)? {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        // Only Integer→Decimal→Quantity and Date→DateTime convert implicitly
        // in FHIRPath; nothing converts implicitly to Boolean. So the spec's
        // fallback applies and any other single item stands for `true` —
        // which is why `(0).not()` is false, not true.
        Some(_) => Ok(Some(true)),
    }
}

/// `where`-style criteria: the empty collection counts as false.
pub fn as_condition(vals: &[Value]) -> Result<bool, String> {
    Ok(as_tri(vals)?.unwrap_or(false))
}

fn opt_bool(b: Option<bool>) -> Vec<Value> {
    b.map(|b| vec![Value::Bool(b)]).unwrap_or_default()
}

/// Remove equivalent duplicates, keeping first occurrences.
pub fn dedup(vals: Vec<Value>) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::with_capacity(vals.len());
    for v in vals {
        if !out.iter().any(|seen| equal(seen, &v) == Some(true)) {
            out.push(v);
        }
    }
    out
}

/// String rendering used by `&`, `toString()` and `join()`.
pub fn string_of(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Evaluate `expr` once per item of `focus` with `$this`/`$index` bound.
pub fn eval_per_item(
    expr: &Expr,
    focus: &[Value],
    env: &mut Env,
) -> Result<Vec<(Value, Vec<Value>)>, String> {
    let mut out = Vec::with_capacity(focus.len());
    for (i, item) in focus.iter().enumerate() {
        let single = [item.clone()];
        let res = env.with_item(item, i, |env| eval(expr, &single, env))?;
        out.push((item.clone(), res));
    }
    Ok(out)
}

/// Depth-first walk of an item's direct children, in document order.
pub fn children_of(value: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    if let Some(obj) = value.as_object() {
        for (k, v) in obj {
            // resourceType is metadata of the JSON encoding, not an element.
            if k == "resourceType" {
                continue;
            }
            match v {
                Value::Array(arr) => out.extend(arr.iter().cloned()),
                v => out.push(v.clone()),
            }
        }
    }
    out
}

pub fn descendants_of(value: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    let mut stack: Vec<Value> = children_of(value);
    stack.reverse();
    while let Some(v) = stack.pop() {
        out.push(v.clone());
        let mut kids = children_of(&v);
        kids.reverse();
        stack.extend(kids);
    }
    out
}

/// Resolve a `Reference` against the contained/bundled resources reachable
/// from the root. Only local resolution is possible offline.
pub fn resolve_reference(reference: &str, root: &Value) -> Option<Value> {
    let target = reference.trim();

    // "#contained-id"
    if let Some(id) = target.strip_prefix('#') {
        return find_in_array(root.get("contained"), id);
    }

    // "Patient/p1" or a full URL ending in the same.
    let tail = target.rsplit('/').take(2).collect::<Vec<_>>();
    let (rtype, id) = match tail.as_slice() {
        [id, rtype] => (*rtype, *id),
        _ => return None,
    };

    let mut index: HashMap<String, Value> = HashMap::new();
    collect_resources(root, &mut index);
    index
        .get(&format!("{}/{}", rtype, id))
        .or_else(|| index.get(id))
        .cloned()
}

fn find_in_array(arr: Option<&Value>, id: &str) -> Option<Value> {
    arr?.as_array()?
        .iter()
        .find(|r| r.get("id").and_then(|v| v.as_str()) == Some(id))
        .cloned()
}

fn collect_resources(value: &Value, out: &mut HashMap<String, Value>) {
    if let (Some(rt), Some(id)) = (
        value.get("resourceType").and_then(|v| v.as_str()),
        value.get("id").and_then(|v| v.as_str()),
    ) {
        out.entry(format!("{}/{}", rt, id))
            .or_insert_with(|| value.clone());
        out.entry(id.to_string()).or_insert_with(|| value.clone());
    }
    match value {
        Value::Object(map) => {
            for v in map.values() {
                collect_resources(v, out);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_resources(v, out);
            }
        }
        _ => {}
    }
}
