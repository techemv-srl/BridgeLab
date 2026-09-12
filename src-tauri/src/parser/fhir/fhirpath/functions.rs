//! FHIRPath function library.
//!
//! Grouped as in the specification: existence, filtering & projection,
//! subsetting, combining, conversion, string manipulation, math, tree
//! navigation, utilities, and the FHIR-specific extensions.

use chrono::{Local, Utc};
use regex::Regex;
use serde_json::{Map, Value};

use super::ast::Expr;
use super::eval::{
    as_condition, as_tri, children_of, dedup, descendants_of, equal, eval, eval_per_item,
    resolve_reference, singleton, string_of, Env,
};
use super::types;

pub fn call(
    name: &str,
    args: &[Expr],
    focus: &[Value],
    env: &mut Env,
) -> Result<Vec<Value>, String> {
    match name {
        // ---- existence ------------------------------------------------
        "empty" => {
            no_args(name, args)?;
            Ok(vec![Value::Bool(focus.is_empty())])
        }
        "exists" => match args {
            [] => Ok(vec![Value::Bool(!focus.is_empty())]),
            [criteria] => {
                let kept = filter(criteria, focus, env)?;
                Ok(vec![Value::Bool(!kept.is_empty())])
            }
            _ => Err(arity(name, "0 or 1", args.len())),
        },
        "all" => {
            let criteria = one_arg(name, args)?;
            for (_, res) in eval_per_item(criteria, focus, env)? {
                if !as_condition(&res)? {
                    return Ok(vec![Value::Bool(false)]);
                }
            }
            Ok(vec![Value::Bool(true)])
        }
        "allTrue" | "anyTrue" | "allFalse" | "anyFalse" => {
            no_args(name, args)?;
            boolean_aggregate(name, focus)
        }
        "subsetOf" => {
            let other = eval_arg(one_arg(name, args)?, env)?;
            Ok(vec![Value::Bool(focus.iter().all(|a| contains(&other, a)))])
        }
        "supersetOf" => {
            let other = eval_arg(one_arg(name, args)?, env)?;
            Ok(vec![Value::Bool(other.iter().all(|a| contains(focus, a)))])
        }
        "count" => {
            no_args(name, args)?;
            Ok(vec![Value::from(focus.len() as i64)])
        }
        "distinct" => {
            no_args(name, args)?;
            Ok(dedup(focus.to_vec()))
        }
        "isDistinct" => {
            no_args(name, args)?;
            Ok(vec![Value::Bool(dedup(focus.to_vec()).len() == focus.len())])
        }

        // ---- filtering and projection ---------------------------------
        "where" => filter(one_arg(name, args)?, focus, env),
        "select" => {
            let projection = one_arg(name, args)?;
            let mut out = Vec::new();
            for (_, mut res) in eval_per_item(projection, focus, env)? {
                out.append(&mut res);
            }
            Ok(out)
        }
        "repeat" => repeat(one_arg(name, args)?, focus, env),
        "ofType" => {
            let type_name = type_arg(name, args)?;
            Ok(focus
                .iter()
                .filter(|v| types::is_type(v, &type_name))
                .cloned()
                .collect())
        }

        // ---- subsetting -----------------------------------------------
        "single" => {
            no_args(name, args)?;
            match focus.len() {
                0 => Ok(vec![]),
                1 => Ok(focus.to_vec()),
                n => Err(format!("single() expects one item but found {}", n)),
            }
        }
        "first" => {
            no_args(name, args)?;
            Ok(focus.first().cloned().into_iter().collect())
        }
        "last" => {
            no_args(name, args)?;
            Ok(focus.last().cloned().into_iter().collect())
        }
        "tail" => {
            no_args(name, args)?;
            Ok(focus.iter().skip(1).cloned().collect())
        }
        "skip" => {
            let n = int_arg(name, args, focus, env)?;
            Ok(focus.iter().skip(n.max(0) as usize).cloned().collect())
        }
        "take" => {
            let n = int_arg(name, args, focus, env)?;
            if n <= 0 {
                return Ok(vec![]);
            }
            Ok(focus.iter().take(n as usize).cloned().collect())
        }
        "intersect" => {
            let other = eval_arg(one_arg(name, args)?, env)?;
            Ok(dedup(
                focus.iter().filter(|v| contains(&other, v)).cloned().collect(),
            ))
        }
        "exclude" => {
            let other = eval_arg(one_arg(name, args)?, env)?;
            Ok(focus.iter().filter(|v| !contains(&other, v)).cloned().collect())
        }

        "sort" => sort(name, args, focus, env),

        // ---- combining ------------------------------------------------
        "union" => {
            let other = eval_arg(one_arg(name, args)?, env)?;
            Ok(dedup(focus.iter().cloned().chain(other).collect()))
        }
        "combine" => {
            let other = eval_arg(one_arg(name, args)?, env)?;
            Ok(focus.iter().cloned().chain(other).collect())
        }

        // ---- conversion -----------------------------------------------
        "iif" => iif(name, args, focus, env),
        "toBoolean" => convert(focus, to_boolean),
        "toInteger" => convert(focus, to_integer),
        "toDecimal" => convert(focus, to_decimal),
        "toString" => convert(focus, to_string_value),
        "toDate" => convert(focus, |v| to_temporal(v, types::TemporalKind::Date)),
        "toDateTime" => convert(focus, |v| to_temporal(v, types::TemporalKind::DateTime)),
        "toTime" => convert(focus, |v| to_temporal(v, types::TemporalKind::Time)),
        "toQuantity" => convert(focus, to_quantity),
        "convertsToBoolean" => converts(focus, to_boolean),
        "convertsToInteger" => converts(focus, to_integer),
        "convertsToDecimal" => converts(focus, to_decimal),
        "convertsToString" => converts(focus, to_string_value),
        "convertsToDate" => converts(focus, |v| to_temporal(v, types::TemporalKind::Date)),
        "convertsToDateTime" => converts(focus, |v| to_temporal(v, types::TemporalKind::DateTime)),
        "convertsToTime" => converts(focus, |v| to_temporal(v, types::TemporalKind::Time)),
        "convertsToQuantity" => converts(focus, to_quantity),
        "toChars" => {
            no_args(name, args)?;
            let Some(s) = opt_string(focus)? else {
                return Ok(vec![]);
            };
            Ok(s.chars().map(|c| Value::from(c.to_string())).collect())
        }

        // ---- strings ---------------------------------------------------
        "length" => {
            no_args(name, args)?;
            match opt_string(focus)? {
                Some(s) => Ok(vec![Value::from(s.chars().count() as i64)]),
                None => Ok(vec![]),
            }
        }
        "startsWith" | "endsWith" | "contains" | "indexOf" | "matches" => {
            let arg = string_arg(name, args, focus, env)?;
            let (Some(s), Some(needle)) = (opt_string(focus)?, arg) else {
                return Ok(vec![]);
            };
            string_predicate(name, &s, &needle)
        }
        "matchesFull" => {
            let arg = string_arg(name, args, focus, env)?;
            let (Some(s), Some(pattern)) = (opt_string(focus)?, arg) else {
                return Ok(vec![]);
            };
            let re = compile(&format!("^(?:{})$", pattern))?;
            Ok(vec![Value::Bool(re.is_match(&s))])
        }
        "substring" => substring(name, args, focus, env),
        "encode" | "decode" => {
            let format = string_arg(name, args, focus, env)?;
            let (Some(s), Some(format)) = (opt_string(focus)?, format) else {
                return Ok(vec![]);
            };
            let out = if name == "encode" {
                encode(&s, &format)
            } else {
                decode(&s, &format)
            };
            Ok(out.map(Value::from).into_iter().collect())
        }
        "escape" | "unescape" => {
            let format = string_arg(name, args, focus, env)?;
            let (Some(s), Some(format)) = (opt_string(focus)?, format) else {
                return Ok(vec![]);
            };
            let out = if name == "escape" {
                escape(&s, &format)
            } else {
                unescape(&s, &format)
            };
            Ok(out.map(Value::from).into_iter().collect())
        }
        "upper" => map_string(name, args, focus, |s| s.to_uppercase()),
        "lower" => map_string(name, args, focus, |s| s.to_lowercase()),
        "trim" => map_string(name, args, focus, |s| s.trim().to_string()),
        "replace" => {
            let (pattern, replacement) = two_string_args(name, args, focus, env)?;
            let (Some(s), Some(p), Some(r)) = (opt_string(focus)?, pattern, replacement) else {
                return Ok(vec![]);
            };
            // Replacing the empty string surrounds every character, which is
            // what the spec's "insert between each character" rule means.
            if p.is_empty() {
                let mut out = String::from(&r);
                for c in s.chars() {
                    out.push(c);
                    out.push_str(&r);
                }
                return Ok(vec![Value::from(out)]);
            }
            Ok(vec![Value::from(s.replace(&p, &r))])
        }
        "replaceMatches" => {
            let (pattern, replacement) = two_string_args(name, args, focus, env)?;
            let (Some(s), Some(p), Some(r)) = (opt_string(focus)?, pattern, replacement) else {
                return Ok(vec![]);
            };
            // An empty pattern matches at every position; the spec says the
            // input comes back untouched rather than shredded.
            if p.is_empty() {
                return Ok(vec![Value::from(s)]);
            }
            let re = compile(&p)?;
            Ok(vec![Value::from(re.replace_all(&s, r.as_str()).to_string())])
        }
        "split" => {
            let sep = string_arg(name, args, focus, env)?;
            let (Some(s), Some(sep)) = (opt_string(focus)?, sep) else {
                return Ok(vec![]);
            };
            if sep.is_empty() {
                return Err("split() needs a non-empty separator".into());
            }
            Ok(s.split(&sep).map(|p| Value::from(p.to_string())).collect())
        }
        "join" => {
            let sep = match args {
                [] => String::new(),
                [a] => eval(a, focus, env)?
                    .first()
                    .map(string_of)
                    .unwrap_or_default(),
                _ => return Err(arity(name, "0 or 1", args.len())),
            };
            let parts: Vec<String> = focus.iter().map(string_of).collect();
            Ok(vec![Value::from(parts.join(&sep))])
        }

        // ---- math ------------------------------------------------------
        "abs" => math1(name, args, focus, f64::abs),
        "ceiling" => math_int(name, args, focus, f64::ceil),
        "floor" => math_int(name, args, focus, f64::floor),
        "truncate" => math_int(name, args, focus, f64::trunc),
        "sqrt" => math_checked(name, args, focus, |x| (x >= 0.0).then(|| x.sqrt())),
        "exp" => math1(name, args, focus, f64::exp),
        "ln" => math_checked(name, args, focus, |x| (x > 0.0).then(|| x.ln())),
        "round" => {
            let places = match args {
                [] => 0,
                [a] => eval(a, focus, env)?
                    .first()
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                _ => return Err(arity(name, "0 or 1", args.len())),
            };
            let Some(x) = opt_number(focus)? else {
                return Ok(vec![]);
            };
            let f = 10f64.powi(places as i32);
            let r = (x * f).round() / f;
            Ok(vec![if places <= 0 && r.fract() == 0.0 {
                Value::from(r as i64)
            } else {
                Value::from(r)
            }])
        }
        "log" => {
            let base = number_arg(name, args, focus, env)?;
            let (Some(x), Some(b)) = (opt_number(focus)?, base) else {
                return Ok(vec![]);
            };
            if x <= 0.0 || b <= 0.0 || b == 1.0 {
                return Ok(vec![]);
            }
            Ok(vec![Value::from(x.log(b))])
        }
        "power" => {
            let exponent = number_arg(name, args, focus, env)?;
            let (Some(x), Some(e)) = (opt_number(focus)?, exponent) else {
                return Ok(vec![]);
            };
            let r = x.powf(e);
            if r.is_nan() {
                return Ok(vec![]);
            }
            let both_int = focus[0].is_i64() && e.fract() == 0.0 && e >= 0.0;
            Ok(vec![if both_int {
                Value::from(r as i64)
            } else {
                Value::from(r)
            }])
        }

        // ---- tree navigation -------------------------------------------
        "children" => {
            no_args(name, args)?;
            Ok(focus.iter().flat_map(children_of).collect())
        }
        "descendants" => {
            no_args(name, args)?;
            Ok(focus.iter().flat_map(descendants_of).collect())
        }

        // ---- types ------------------------------------------------------
        "is" => {
            let type_name = type_arg(name, args)?;
            match singleton(focus)? {
                None => Ok(vec![]),
                Some(v) => Ok(vec![Value::Bool(types::is_type(v, &type_name))]),
            }
        }
        "as" => {
            let type_name = type_arg(name, args)?;
            match singleton(focus)? {
                Some(v) if types::is_type(v, &type_name) => Ok(vec![v.clone()]),
                _ => Ok(vec![]),
            }
        }
        "type" => {
            no_args(name, args)?;
            Ok(focus
                .iter()
                .map(|v| {
                    let mut m = Map::new();
                    let (ns, n) = types::namespaced_type(v);
                    m.insert("namespace".into(), Value::from(ns));
                    m.insert("name".into(), Value::from(n));
                    Value::Object(m)
                })
                .collect())
        }

        // ---- booleans ---------------------------------------------------
        "not" => {
            no_args(name, args)?;
            Ok(match as_tri(focus)? {
                Some(b) => vec![Value::Bool(!b)],
                None => vec![],
            })
        }

        // ---- aggregates --------------------------------------------------
        "aggregate" => aggregate(name, args, focus, env),

        // ---- utilities ----------------------------------------------------
        "trace" => {
            let label = match args.first() {
                Some(a) => eval(a, focus, env)?
                    .first()
                    .map(string_of)
                    .unwrap_or_default(),
                None => return Err(arity(name, "1 or 2", 0)),
            };
            let logged = match args.get(1) {
                Some(projection) => {
                    let mut out = Vec::new();
                    for (_, mut res) in eval_per_item(projection, focus, env)? {
                        out.append(&mut res);
                    }
                    out
                }
                None => focus.to_vec(),
            };
            env.trace.push((label, logged));
            Ok(focus.to_vec())
        }
        "now" => {
            no_args(name, args)?;
            Ok(vec![Value::from(
                Utc::now().format("%Y-%m-%dT%H:%M:%S%.3f+00:00").to_string(),
            )])
        }
        "today" => {
            no_args(name, args)?;
            Ok(vec![Value::from(Local::now().format("%Y-%m-%d").to_string())])
        }
        "timeOfDay" => {
            no_args(name, args)?;
            Ok(vec![Value::from(
                Local::now().format("T%H:%M:%S%.3f").to_string(),
            )])
        }

        // ---- FHIR-specific -------------------------------------------------
        "extension" => {
            let url = string_arg(name, args, focus, env)?;
            let Some(url) = url else { return Ok(vec![]) };
            let mut out = Vec::new();
            for item in focus {
                let Some(exts) = item.get("extension").and_then(|e| e.as_array()) else {
                    continue;
                };
                out.extend(
                    exts.iter()
                        .filter(|e| e.get("url").and_then(|u| u.as_str()) == Some(url.as_str()))
                        .cloned(),
                );
            }
            Ok(out)
        }
        "hasValue" => {
            no_args(name, args)?;
            match singleton(focus)? {
                None => Ok(vec![Value::Bool(false)]),
                Some(v) => Ok(vec![Value::Bool(!v.is_object() && !v.is_array())]),
            }
        }
        "getValue" => {
            no_args(name, args)?;
            Ok(focus.iter().filter(|v| !v.is_object()).cloned().collect())
        }
        "resolve" => {
            no_args(name, args)?;
            let mut out = Vec::new();
            for item in focus {
                let reference = item
                    .get("reference")
                    .and_then(|r| r.as_str())
                    .or_else(|| item.as_str());
                if let Some(r) = reference {
                    if let Some(found) = resolve_reference(r, env.root) {
                        out.push(found);
                    }
                }
            }
            Ok(out)
        }
        "precision" => {
            no_args(name, args)?;
            let Some(v) = singleton(focus)? else {
                return Ok(vec![]);
            };
            if let Some(t) = v.as_str().and_then(types::as_temporal) {
                return Ok(vec![Value::from(t.precision())]);
            }
            // Significant digits of a decimal. The JSON encoding has already
            // dropped trailing zeros, so `1.58700` reports 3, not 5.
            match v {
                Value::Number(n) if n.is_f64() => {
                    let text = n.to_string();
                    let digits = text.split_once('.').map(|(_, f)| f.len()).unwrap_or(0);
                    Ok(vec![Value::from(digits as i64)])
                }
                _ => Ok(vec![]),
            }
        }
        "comparable" => {
            let other = eval_arg(one_arg(name, args)?, env)?;
            let (Some(a), Some(b)) = (singleton(focus)?, singleton(&other)?) else {
                return Ok(vec![]);
            };
            let comparable = match (types::quantity_parts(a), types::quantity_parts(b)) {
                (Some((_, au)), Some((_, bu))) => {
                    types::convert_quantity(1.0, &bu, &au).is_some()
                }
                _ => false,
            };
            Ok(vec![Value::Bool(comparable)])
        }
        "lowBoundary" | "highBoundary" => Err(format!(
            "{}() needs exact decimal arithmetic, which this engine does not implement",
            name
        )),
        "conformsTo" => Err(
            "conformsTo() needs the profile packages, which this build does not load".into(),
        ),

        other => Err(format!("Unknown function: {}()", other)),
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn arity(name: &str, expected: &str, got: usize) -> String {
    format!("{}() takes {} arguments but got {}", name, expected, got)
}

fn no_args(name: &str, args: &[Expr]) -> Result<(), String> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(arity(name, "no", args.len()))
    }
}

fn one_arg<'a>(name: &str, args: &'a [Expr]) -> Result<&'a Expr, String> {
    match args {
        [a] => Ok(a),
        _ => Err(arity(name, "1", args.len())),
    }
}

/// A type name given as a bare identifier rather than an evaluated argument.
fn type_arg(name: &str, args: &[Expr]) -> Result<String, String> {
    match one_arg(name, args)? {
        Expr::Member { base: None, name } => Ok(name.clone()),
        Expr::Member {
            base: Some(b),
            name: n,
        } => match b.as_ref() {
            Expr::Member { base: None, name: ns } => Ok(format!("{}.{}", ns, n)),
            _ => Err(format!("{}() expects a type name", name)),
        },
        _ => Err(format!("{}() expects a type name", name)),
    }
}

/// Evaluate an ordinary (non-lambda) argument against the context node.
fn eval_arg(expr: &Expr, env: &mut Env) -> Result<Vec<Value>, String> {
    let context = env.context();
    eval(expr, &context, env)
}

fn contains(haystack: &[Value], needle: &Value) -> bool {
    haystack.iter().any(|v| equal(v, needle) == Some(true))
}

fn filter(criteria: &Expr, focus: &[Value], env: &mut Env) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    for (item, res) in eval_per_item(criteria, focus, env)? {
        if as_condition(&res)? {
            out.push(item);
        }
    }
    Ok(out)
}

/// `repeat(projection)` applies the projection until it stops producing
/// values that have not been seen, which is how FHIRPath walks recursive
/// structures such as Questionnaire.item.
fn repeat(projection: &Expr, focus: &[Value], env: &mut Env) -> Result<Vec<Value>, String> {
    let mut out: Vec<Value> = Vec::new();
    let mut frontier: Vec<Value> = focus.to_vec();
    // Structures are trees in practice; the bound stops a cycle in data that
    // is not, instead of hanging the UI.
    for _ in 0..1000 {
        if frontier.is_empty() {
            break;
        }
        let mut next = Vec::new();
        for (_, res) in eval_per_item(projection, &frontier, env)? {
            for v in res {
                if !contains(&out, &v) {
                    out.push(v.clone());
                    next.push(v);
                }
            }
        }
        frontier = next;
    }
    Ok(out)
}

fn aggregate(
    name: &str,
    args: &[Expr],
    focus: &[Value],
    env: &mut Env,
) -> Result<Vec<Value>, String> {
    let (expr, init) = match args {
        [e] => (e, Vec::new()),
        [e, i] => (e, eval(i, focus, env)?),
        _ => return Err(arity(name, "1 or 2", args.len())),
    };

    let saved_total = env.total.take();
    env.total = Some(init);
    let mut result = Ok(());
    for (i, item) in focus.iter().enumerate() {
        let single = [item.clone()];
        let saved_this = env.this.take();
        let saved_index = env.index.take();
        env.this = Some(vec![item.clone()]);
        env.index = Some(i as i64);
        let step = eval(expr, &single, env);
        env.this = saved_this;
        env.index = saved_index;
        match step {
            Ok(v) => env.total = Some(v),
            Err(e) => {
                result = Err(e);
                break;
            }
        }
    }
    let total = env.total.take().unwrap_or_default();
    env.total = saved_total;
    result.map(|()| total)
}

fn iif(name: &str, args: &[Expr], focus: &[Value], env: &mut Env) -> Result<Vec<Value>, String> {
    let (condition, then_branch, else_branch) = match args {
        [c, t] => (c, t, None),
        [c, t, e] => (c, t, Some(e)),
        _ => return Err(arity(name, "2 or 3", args.len())),
    };
    if focus.len() > 1 {
        return Err(format!(
            "iif() expects a single input but found {}",
            focus.len()
        ));
    }

    // Unlike `where()`, iif() requires an actual Boolean: a criteria that is
    // merely non-empty is a mistake worth reporting, not a `true`.
    let saved = env.this.replace(focus.to_vec());
    let outcome = (|env: &mut Env| {
        let decided = eval(condition, focus, env)?;
        let taken = match singleton(&decided)? {
            None => false,
            Some(Value::Bool(b)) => *b,
            Some(other) => {
                return Err(format!(
                    "iif() expects a boolean criteria but found a {}",
                    types::type_name(other)
                ))
            }
        };
        // Only the taken branch is evaluated.
        if taken {
            eval(then_branch, focus, env)
        } else {
            match else_branch {
                Some(e) => eval(e, focus, env),
                None => Ok(vec![]),
            }
        }
    })(env);
    env.this = saved;
    outcome
}

fn boolean_aggregate(name: &str, focus: &[Value]) -> Result<Vec<Value>, String> {
    let mut bools = Vec::with_capacity(focus.len());
    for v in focus {
        match v.as_bool() {
            Some(b) => bools.push(b),
            None => {
                return Err(format!(
                    "{}() expects booleans but found a {}",
                    name,
                    types::type_name(v)
                ))
            }
        }
    }
    let result = match name {
        "allTrue" => bools.iter().all(|b| *b),
        "anyTrue" => bools.iter().any(|b| *b),
        "allFalse" => bools.iter().all(|b| !*b),
        _ => bools.iter().any(|b| !*b),
    };
    Ok(vec![Value::Bool(result)])
}

// ---- argument readers ------------------------------------------------------

fn string_arg(
    name: &str,
    args: &[Expr],
    focus: &[Value],
    env: &mut Env,
) -> Result<Option<String>, String> {
    let vals = eval(one_arg(name, args)?, focus, env)?;
    Ok(singleton(&vals)?.map(string_of))
}

fn two_string_args(
    name: &str,
    args: &[Expr],
    focus: &[Value],
    env: &mut Env,
) -> Result<(Option<String>, Option<String>), String> {
    match args {
        [a, b] => {
            let x = eval(a, focus, env)?;
            let y = eval(b, focus, env)?;
            Ok((
                singleton(&x)?.map(string_of),
                singleton(&y)?.map(string_of),
            ))
        }
        _ => Err(arity(name, "2", args.len())),
    }
}

fn number_arg(
    name: &str,
    args: &[Expr],
    focus: &[Value],
    env: &mut Env,
) -> Result<Option<f64>, String> {
    let vals = eval(one_arg(name, args)?, focus, env)?;
    match singleton(&vals)? {
        None => Ok(None),
        Some(v) => v
            .as_f64()
            .map(Some)
            .ok_or_else(|| format!("{}() expects a number", name)),
    }
}

fn int_arg(name: &str, args: &[Expr], focus: &[Value], env: &mut Env) -> Result<i64, String> {
    let vals = eval(one_arg(name, args)?, focus, env)?;
    match singleton(&vals)? {
        None => Ok(0),
        Some(v) => v
            .as_i64()
            .ok_or_else(|| format!("{}() expects an integer", name)),
    }
}

fn opt_string(focus: &[Value]) -> Result<Option<String>, String> {
    Ok(singleton(focus)?.map(string_of))
}

fn opt_number(focus: &[Value]) -> Result<Option<f64>, String> {
    match singleton(focus)? {
        None => Ok(None),
        Some(v) => v
            .as_f64()
            .map(Some)
            .ok_or_else(|| format!("Expected a number but found a {}", types::type_name(v))),
    }
}

// ---- string and math workers ------------------------------------------------

fn string_predicate(name: &str, s: &str, arg: &str) -> Result<Vec<Value>, String> {
    Ok(match name {
        "startsWith" => vec![Value::Bool(s.starts_with(arg))],
        "endsWith" => vec![Value::Bool(s.ends_with(arg))],
        "contains" => vec![Value::Bool(s.contains(arg))],
        "indexOf" => {
            // Char offsets, not byte offsets.
            let idx = s
                .find(arg)
                .map(|b| s[..b].chars().count() as i64)
                .unwrap_or(-1);
            vec![Value::from(idx)]
        }
        "matches" => {
            let re = compile(arg)?;
            vec![Value::Bool(re.is_match(s))]
        }
        _ => unreachable!(),
    })
}

/// `sort()` orders by natural comparison; `sort(key)` by the projection,
/// which may be negated (`sort(-$this)`) for descending order.
fn sort(_name: &str, args: &[Expr], focus: &[Value], env: &mut Env) -> Result<Vec<Value>, String> {
    // Each key is a projection, optionally negated to sort descending —
    // `sort(-family, -given.first())` orders by two keys, both reversed.
    // A leading '-' is a direction marker here, not arithmetic, so it works
    // on strings as well as numbers.
    let keys: Vec<(&Expr, bool)> = if args.is_empty() {
        Vec::new()
    } else {
        args.iter()
            .map(|a| match a {
                Expr::Unary {
                    op: super::ast::UnaryOp::Neg,
                    operand,
                } => (operand.as_ref(), true),
                other => (other, false),
            })
            .collect()
    };

    // Column-wise: one pass per key produces that key's value for each item.
    let mut columns: Vec<Vec<Value>> = Vec::with_capacity(keys.len().max(1));
    if keys.is_empty() {
        columns.push(focus.to_vec());
    } else {
        for (key, _) in &keys {
            let mut column = Vec::with_capacity(focus.len());
            for (_, mut projected) in eval_per_item(key, focus, env)? {
                column.push(if projected.is_empty() {
                    Value::Null
                } else {
                    projected.remove(0)
                });
            }
            columns.push(column);
        }
    }

    let descending: Vec<bool> = if keys.is_empty() {
        vec![false]
    } else {
        keys.iter().map(|(_, d)| *d).collect()
    };

    let mut order: Vec<usize> = (0..focus.len()).collect();
    // A comparison error would make the whole sort meaningless, so surface it
    // instead of silently leaving the collection half-ordered.
    let mut failure: Option<String> = None;
    order.sort_by(|&i, &j| {
        use std::cmp::Ordering;
        if failure.is_some() {
            return Ordering::Equal;
        }
        for (column, desc) in columns.iter().zip(descending.iter()) {
            // An item whose key is empty sorts after every value, and the
            // direction marker flips that along with everything else.
            let ord = match (&column[i], &column[j]) {
                (Value::Null, Value::Null) => Ordering::Equal,
                (Value::Null, _) => Ordering::Greater,
                (_, Value::Null) => Ordering::Less,
                (a, b) => match super::eval::compare(a, b) {
                    Ok(Some(o)) => o,
                    Ok(None) => Ordering::Equal,
                    Err(e) => {
                        failure = Some(e);
                        return Ordering::Equal;
                    }
                },
            };
            let ord = if *desc { ord.reverse() } else { ord };
            if ord != Ordering::Equal {
                return ord;
            }
        }
        Ordering::Equal
    });

    match failure {
        Some(e) => Err(e),
        None => Ok(order.into_iter().map(|i| focus[i].clone()).collect()),
    }
}

fn encode(s: &str, format: &str) -> Option<String> {
    use base64::Engine;
    Some(match format {
        "base64" => base64::engine::general_purpose::STANDARD.encode(s),
        "urlbase64" => base64::engine::general_purpose::URL_SAFE.encode(s),
        "hex" => s.bytes().map(|b| format!("{:02x}", b)).collect(),
        _ => return None,
    })
}

fn decode(s: &str, format: &str) -> Option<String> {
    use base64::Engine;
    let bytes = match format {
        "base64" => base64::engine::general_purpose::STANDARD.decode(s).ok()?,
        "urlbase64" => base64::engine::general_purpose::URL_SAFE.decode(s).ok()?,
        "hex" => {
            if !s.len().is_multiple_of(2) {
                return None;
            }
            (0..s.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
                .collect::<Option<Vec<u8>>>()?
        }
        _ => return None,
    };
    String::from_utf8(bytes).ok()
}

fn escape(s: &str, format: &str) -> Option<String> {
    Some(match format {
        "html" => s
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;"),
        "json" => {
            let mut out = String::with_capacity(s.len());
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c => out.push(c),
                }
            }
            out
        }
        _ => return None,
    })
}

fn unescape(s: &str, format: &str) -> Option<String> {
    Some(match format {
        // '&amp;' last, so an escaped ampersand does not re-expand what it
        // introduced.
        "html" => s
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&"),
        "json" => {
            let mut out = String::with_capacity(s.len());
            let mut chars = s.chars();
            while let Some(c) = chars.next() {
                if c != '\\' {
                    out.push(c);
                    continue;
                }
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('/') => out.push('/'),
                    Some(other) => {
                        out.push('\\');
                        out.push(other);
                    }
                    None => out.push('\\'),
                }
            }
            out
        }
        _ => return None,
    })
}

fn compile(pattern: &str) -> Result<Regex, String> {
    // FHIRPath regexes are single-line by default with '.' matching newline.
    Regex::new(&format!("(?s){}", pattern)).map_err(|e| format!("Invalid regex: {}", e))
}

fn substring(
    name: &str,
    args: &[Expr],
    focus: &[Value],
    env: &mut Env,
) -> Result<Vec<Value>, String> {
    let (start, length) = match args {
        [s] => (eval(s, focus, env)?, None),
        [s, l] => (eval(s, focus, env)?, Some(eval(l, focus, env)?)),
        _ => return Err(arity(name, "1 or 2", args.len())),
    };
    let Some(s) = opt_string(focus)? else {
        return Ok(vec![]);
    };
    let Some(start) = singleton(&start)?.and_then(|v| v.as_i64()) else {
        return Ok(vec![]);
    };
    let chars: Vec<char> = s.chars().collect();
    if start < 0 || start as usize >= chars.len() {
        return Ok(vec![]);
    }
    let begin = start as usize;
    let end = match length {
        None => chars.len(),
        Some(l) => match singleton(&l)?.and_then(|v| v.as_i64()) {
            None => return Ok(vec![]),
            Some(n) if n <= 0 => return Ok(vec![Value::from("")]),
            Some(n) => (begin + n as usize).min(chars.len()),
        },
    };
    Ok(vec![Value::from(chars[begin..end].iter().collect::<String>())])
}

fn map_string(
    name: &str,
    args: &[Expr],
    focus: &[Value],
    f: impl Fn(&str) -> String,
) -> Result<Vec<Value>, String> {
    no_args(name, args)?;
    match opt_string(focus)? {
        Some(s) => Ok(vec![Value::from(f(&s))]),
        None => Ok(vec![]),
    }
}

fn math1(
    name: &str,
    args: &[Expr],
    focus: &[Value],
    f: impl Fn(f64) -> f64,
) -> Result<Vec<Value>, String> {
    no_args(name, args)?;
    // abs() on a quantity keeps the unit.
    if let Some(v) = singleton(focus)? {
        if types::is_quantity(v) {
            let (value, unit) = types::quantity_parts(v).unwrap();
            return Ok(vec![types::make_quantity(f(value), &unit)]);
        }
    }
    let Some(x) = opt_number(focus)? else {
        return Ok(vec![]);
    };
    let r = f(x);
    Ok(vec![if focus[0].is_i64() && r.fract() == 0.0 {
        Value::from(r as i64)
    } else {
        Value::from(r)
    }])
}

/// ceiling/floor/truncate always produce an Integer.
fn math_int(
    name: &str,
    args: &[Expr],
    focus: &[Value],
    f: impl Fn(f64) -> f64,
) -> Result<Vec<Value>, String> {
    no_args(name, args)?;
    let Some(x) = opt_number(focus)? else {
        return Ok(vec![]);
    };
    Ok(vec![Value::from(f(x) as i64)])
}

/// Math whose domain is restricted; outside it the result is empty rather
/// than NaN.
fn math_checked(
    name: &str,
    args: &[Expr],
    focus: &[Value],
    f: impl Fn(f64) -> Option<f64>,
) -> Result<Vec<Value>, String> {
    no_args(name, args)?;
    let Some(x) = opt_number(focus)? else {
        return Ok(vec![]);
    };
    Ok(f(x).map(Value::from).into_iter().collect())
}

// ---- conversions -------------------------------------------------------------

fn convert(
    focus: &[Value],
    f: impl Fn(&Value) -> Option<Value>,
) -> Result<Vec<Value>, String> {
    match singleton(focus)? {
        None => Ok(vec![]),
        Some(v) => Ok(f(v).into_iter().collect()),
    }
}

fn converts(
    focus: &[Value],
    f: impl Fn(&Value) -> Option<Value>,
) -> Result<Vec<Value>, String> {
    match singleton(focus)? {
        None => Ok(vec![]),
        Some(v) => Ok(vec![Value::Bool(f(v).is_some())]),
    }
}

fn to_boolean(v: &Value) -> Option<Value> {
    match v {
        Value::Bool(_) => Some(v.clone()),
        Value::Number(n) => match n.as_f64()? {
            1.0 => Some(Value::Bool(true)),
            0.0 => Some(Value::Bool(false)),
            _ => None,
        },
        Value::String(s) => match s.to_lowercase().as_str() {
            "true" | "t" | "yes" | "y" | "1" | "1.0" => Some(Value::Bool(true)),
            "false" | "f" | "no" | "n" | "0" | "0.0" => Some(Value::Bool(false)),
            _ => None,
        },
        _ => None,
    }
}

fn to_integer(v: &Value) -> Option<Value> {
    match v {
        Value::Bool(b) => Some(Value::from(if *b { 1 } else { 0 })),
        Value::Number(n) if n.is_i64() || n.is_u64() => Some(v.clone()),
        Value::String(s) => s.trim().parse::<i64>().ok().map(Value::from),
        _ => None,
    }
}

fn to_decimal(v: &Value) -> Option<Value> {
    match v {
        Value::Bool(b) => Some(Value::from(if *b { 1.0 } else { 0.0 })),
        Value::Number(_) => Some(v.clone()),
        Value::String(s) => s.trim().parse::<f64>().ok().map(Value::from),
        _ => None,
    }
}

fn to_string_value(v: &Value) -> Option<Value> {
    if let Some(rendered) = types::quantity_to_string(v) {
        return Some(Value::from(rendered));
    }
    match v {
        Value::Object(_) | Value::Array(_) | Value::Null => None,
        other => Some(Value::from(string_of(other))),
    }
}

fn to_temporal(v: &Value, kind: types::TemporalKind) -> Option<Value> {
    let s = v.as_str()?;
    if kind == types::TemporalKind::Time {
        return types::as_time_lenient(s).map(|_| Value::from(s.to_string()));
    }
    let t = types::as_temporal(s)?;
    // A Date converts to a DateTime, but not the other way round.
    let ok = t.kind == kind
        || (kind == types::TemporalKind::DateTime && t.kind == types::TemporalKind::Date);
    ok.then(|| Value::from(s.to_string()))
}

fn to_quantity(v: &Value) -> Option<Value> {
    if types::is_quantity(v) {
        return Some(v.clone());
    }
    match v {
        Value::Number(n) => Some(types::make_quantity(n.as_f64()?, "1")),
        Value::Bool(b) => Some(types::make_quantity(if *b { 1.0 } else { 0.0 }, "1")),
        Value::String(s) => {
            // A quantity string is `<number> '<ucum unit>'` or
            // `<number> <calendar unit>`. A bare word such as "1 wk" is
            // neither, and does not convert.
            let s = s.trim();
            let Some((num, unit)) = s.split_once(' ') else {
                return Some(types::make_quantity(s.parse().ok()?, "1"));
            };
            let value: f64 = num.trim().parse().ok()?;
            let unit = unit.trim();
            if let Some(quoted) = unit.strip_prefix('\'').and_then(|u| u.strip_suffix('\'')) {
                return Some(types::make_quantity(value, quoted));
            }
            types::ucum_for_time_unit(unit).map(|u| types::make_quantity(value, u))
        }
        _ => None,
    }
}
