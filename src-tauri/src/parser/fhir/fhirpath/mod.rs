//! FHIRPath evaluation.
//!
//! A tokenizer, precedence-climbing parser and collection-based evaluator for
//! [FHIRPath 2.0](http://hl7.org/fhirpath/N1/), over FHIR resources in their
//! JSON encoding.
//!
//! # What is covered
//!
//! - The full operator set with spec precedence and three-valued boolean
//!   logic: `.`, `[]`, unary `+`/`-`, `*` `/` `div` `mod`, `+` `-` `&`,
//!   `is` `as`, `|`, `<` `>` `<=` `>=`, `=` `~` `!=` `!~`, `in` `contains`,
//!   `and`, `xor` `or`, `implies`.
//! - Literals including `{}`, quantities (`4 'mg'`, `1 year`) and
//!   partial-precision `@` date/time values.
//! - `$this`, `$index`, `$total` and the `%resource` / `%context` / `%ucum`
//!   environment constants.
//! - ~70 functions across existence, filtering, subsetting, combining,
//!   conversion, string, math, tree navigation and utility groups, plus the
//!   FHIR extensions `extension()`, `hasValue()`, `getValue()` and
//!   `resolve()` (local resolution against contained and bundled resources).
//!
//! # What is not
//!
//! Element types are inferred from the JSON shape rather than read from
//! StructureDefinitions — see [`types`] for the rules and their limits.
//! There is no UCUM engine, so quantity arithmetic and comparison only work
//! within one unit, and `conformsTo()` reports that profile packages are not
//! loaded. External terminology (`memberOf()`, `subsumes()`) is likewise out
//! of scope for an offline desktop app.
//!
//! The conformance harness in `tests/fhirpath_suite.rs` runs the official
//! HL7 FHIRPath test suite when it is present locally; `scripts/fetch-fhirpath-suite.sh`
//! downloads it.

mod ast;
mod eval;
mod functions;
mod lexer;
mod parser;
mod types;

use serde::Serialize;
use serde_json::Value;

/// Result of FHIRPath evaluation.
#[derive(Debug, Clone, Serialize)]
pub struct FhirPathResult {
    pub expression: String,
    pub results: Vec<Value>,
    pub count: usize,
    pub error: Option<String>,
    /// Lines emitted by `trace()` during evaluation, oldest first.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub trace: Vec<TraceEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceEntry {
    pub name: String,
    pub values: Vec<Value>,
}

/// Evaluate a FHIRPath expression against a FHIR resource.
///
/// Never panics and never returns `Err`: a parse or evaluation failure is
/// reported through `FhirPathResult::error` so the UI can show it inline
/// next to the expression that caused it.
pub fn evaluate(expression: &str, root: &Value) -> FhirPathResult {
    let expr = expression.trim();

    let parsed = match parser::parse(expr) {
        Ok(p) => p,
        Err(e) => return failed(expr, e),
    };

    let mut env = eval::Env::new(root);
    let focus = vec![root.clone()];
    match eval::eval(&parsed, &focus, &mut env) {
        Ok(results) => FhirPathResult {
            expression: expr.to_string(),
            count: results.len(),
            results,
            error: None,
            trace: env
                .trace
                .into_iter()
                .map(|(name, values)| TraceEntry { name, values })
                .collect(),
        },
        Err(e) => failed(expr, e),
    }
}

/// Check an expression without running it — used by the rules editor to
/// validate what the user typed before saving it.
pub fn check(expression: &str) -> Result<(), String> {
    parser::parse(expression.trim()).map(|_| ())
}

fn failed(expression: &str, error: String) -> FhirPathResult {
    FhirPathResult {
        expression: expression.to_string(),
        results: vec![],
        count: 0,
        error: Some(error),
        trace: Vec::new(),
    }
}

#[cfg(test)]
mod tests;
