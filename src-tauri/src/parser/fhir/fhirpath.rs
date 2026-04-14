use serde::Serialize;
use serde_json::Value;

/// Result of FHIRPath evaluation.
#[derive(Debug, Clone, Serialize)]
pub struct FhirPathResult {
    pub expression: String,
    pub results: Vec<Value>,
    pub count: usize,
    pub error: Option<String>,
}

/// Evaluate a simplified FHIRPath expression on a JSON value.
/// Supports:
/// - Simple paths: `Patient.name.family`
/// - Array access: `Patient.name[0].given`
/// - Array flatten: `Patient.name.given` (collects all)
/// - Basic filter: `Bundle.entry.where(resource.resourceType = 'Patient')`
/// - First/last: `Patient.name.first()`, `Patient.name.last()`
/// - Count: `Bundle.entry.count()`
pub fn evaluate(expression: &str, root: &Value) -> FhirPathResult {
    let expr = expression.trim();

    match eval_expr(expr, root) {
        Ok(results) => FhirPathResult {
            expression: expr.to_string(),
            count: results.len(),
            results,
            error: None,
        },
        Err(e) => FhirPathResult {
            expression: expr.to_string(),
            results: vec![],
            count: 0,
            error: Some(e),
        },
    }
}

/// Main evaluation: handle the resource type prefix then walk the path.
fn eval_expr(expr: &str, root: &Value) -> Result<Vec<Value>, String> {
    if expr.is_empty() {
        return Ok(vec![root.clone()]);
    }

    // Tokenize path - split on '.' but respect parens/brackets
    let segments = tokenize_path(expr)?;
    let mut current: Vec<Value> = vec![root.clone()];

    // If first segment matches resourceType, strip it (implicit root filter)
    if let Some(first) = segments.first() {
        if let Some(rt) = root.get("resourceType").and_then(|v| v.as_str()) {
            if first == rt {
                // Skip the resource type prefix
                let remaining = &segments[1..];
                for seg in remaining {
                    current = apply_segment(seg, &current)?;
                }
                return Ok(current);
            }
        }
    }

    for seg in &segments {
        current = apply_segment(seg, &current)?;
    }

    Ok(current)
}

/// Split "Patient.name[0].given" into ["Patient", "name[0]", "given"].
/// Preserves parentheses content (for function calls like where(...)).
fn tokenize_path(expr: &str) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let chars = expr.chars();

    for c in chars {
        match c {
            '.' if depth == 0 => {
                if !current.is_empty() {
                    result.push(current.clone());
                    current.clear();
                }
            }
            '(' | '[' => {
                depth += 1;
                current.push(c);
            }
            ')' | ']' => {
                if depth == 0 {
                    return Err(format!("Unmatched closing bracket in: {}", expr));
                }
                depth -= 1;
                current.push(c);
            }
            _ => current.push(c),
        }
    }

    if depth != 0 {
        return Err(format!("Unmatched opening bracket in: {}", expr));
    }

    if !current.is_empty() {
        result.push(current);
    }

    Ok(result)
}

/// Apply one segment (name, name[idx], funcName(args)) to current values.
fn apply_segment(seg: &str, current: &[Value]) -> Result<Vec<Value>, String> {
    // Check for function call: name(args) or just fname()
    if let Some(paren_idx) = seg.find('(') {
        let func_name = &seg[..paren_idx];
        let args = seg[paren_idx + 1..seg.len() - 1].trim();
        return apply_function(func_name, args, current);
    }

    // Check for array index: name[0]
    if let Some(bracket_idx) = seg.find('[') {
        let name = &seg[..bracket_idx];
        let idx_str = &seg[bracket_idx + 1..seg.len() - 1];
        let idx: usize = idx_str.parse().map_err(|_| format!("Invalid index: {}", idx_str))?;

        let mut results = Vec::new();
        for val in current {
            let target = if name.is_empty() {
                val
            } else {
                match val.get(name) {
                    Some(v) => v,
                    None => continue,
                }
            };
            if let Some(arr) = target.as_array() {
                if let Some(item) = arr.get(idx) {
                    results.push(item.clone());
                }
            }
        }
        return Ok(results);
    }

    // Plain field access
    let mut results = Vec::new();
    for val in current {
        match val.get(seg) {
            Some(Value::Array(arr)) => {
                // Auto-flatten arrays
                for item in arr {
                    results.push(item.clone());
                }
            }
            Some(v) => results.push(v.clone()),
            None => {}
        }
    }
    Ok(results)
}

/// Apply a function like first(), last(), count(), where(condition).
fn apply_function(name: &str, args: &str, current: &[Value]) -> Result<Vec<Value>, String> {
    match name {
        "first" => Ok(current.first().cloned().map(|v| vec![v]).unwrap_or_default()),
        "last" => Ok(current.last().cloned().map(|v| vec![v]).unwrap_or_default()),
        "count" => Ok(vec![Value::from(current.len())]),
        "exists" => Ok(vec![Value::from(!current.is_empty())]),
        "empty" => Ok(vec![Value::from(current.is_empty())]),
        "where" => apply_where(args, current),
        "select" => apply_select(args, current),
        "distinct" => apply_distinct(current),
        _ => Err(format!("Unknown function: {}()", name)),
    }
}

/// Apply where(condition) - supports `field = 'value'` and `field = "value"`.
fn apply_where(condition: &str, current: &[Value]) -> Result<Vec<Value>, String> {
    let cond = condition.trim();

    // Parse simple equality: field.path = 'literal'
    if let Some(eq_idx) = cond.find('=') {
        let left = cond[..eq_idx].trim();
        let right = cond[eq_idx + 1..].trim();

        // Strip quotes from right side
        let right_val = right.trim_matches(|c: char| c == '\'' || c == '"');

        let mut results = Vec::new();
        for item in current {
            let left_vals = eval_expr(left, item)?;
            let matches = left_vals.iter().any(|v| {
                match v {
                    Value::String(s) => s == right_val,
                    Value::Number(n) => n.to_string() == right_val,
                    Value::Bool(b) => b.to_string() == right_val,
                    _ => false,
                }
            });
            if matches {
                results.push(item.clone());
            }
        }
        return Ok(results);
    }

    // Just a path - truthy check
    let mut results = Vec::new();
    for item in current {
        let path_vals = eval_expr(cond, item)?;
        if !path_vals.is_empty() {
            results.push(item.clone());
        }
    }
    Ok(results)
}

/// Apply select(field) - pick a specific path from each item.
fn apply_select(path: &str, current: &[Value]) -> Result<Vec<Value>, String> {
    let mut results = Vec::new();
    for item in current {
        let mut sub = eval_expr(path.trim(), item)?;
        results.append(&mut sub);
    }
    Ok(results)
}

/// Apply distinct() - deduplicate results.
fn apply_distinct(current: &[Value]) -> Result<Vec<Value>, String> {
    let mut seen: Vec<String> = Vec::new();
    let mut results: Vec<Value> = Vec::new();
    for v in current {
        let key = v.to_string();
        if !seen.contains(&key) {
            seen.push(key);
            results.push(v.clone());
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn patient() -> Value {
        json!({
            "resourceType": "Patient",
            "id": "p1",
            "name": [
                {"family": "Smith", "given": ["Jane", "A"]},
                {"family": "Doe", "given": ["John"], "use": "nickname"}
            ],
            "gender": "female",
            "birthDate": "1990-05-15",
            "active": true
        })
    }

    fn bundle() -> Value {
        json!({
            "resourceType": "Bundle",
            "type": "collection",
            "entry": [
                {"resource": {"resourceType": "Patient", "id": "p1", "gender": "male"}},
                {"resource": {"resourceType": "Patient", "id": "p2", "gender": "female"}},
                {"resource": {"resourceType": "Observation", "id": "o1", "status": "final"}}
            ]
        })
    }

    #[test]
    fn test_simple_path() {
        let r = evaluate("Patient.gender", &patient());
        assert_eq!(r.count, 1);
        assert_eq!(r.results[0], json!("female"));
    }

    #[test]
    fn test_array_flatten() {
        let r = evaluate("Patient.name.family", &patient());
        assert_eq!(r.count, 2);
        assert_eq!(r.results[0], json!("Smith"));
        assert_eq!(r.results[1], json!("Doe"));
    }

    #[test]
    fn test_array_index() {
        let r = evaluate("Patient.name[0].family", &patient());
        assert_eq!(r.count, 1);
        assert_eq!(r.results[0], json!("Smith"));
    }

    #[test]
    fn test_deep_flatten() {
        let r = evaluate("Patient.name.given", &patient());
        assert_eq!(r.count, 3); // Jane, A, John
    }

    #[test]
    fn test_count_function() {
        let r = evaluate("Patient.name.count()", &patient());
        assert_eq!(r.results[0], json!(2));
    }

    #[test]
    fn test_first_function() {
        let r = evaluate("Patient.name.first().family", &patient());
        assert_eq!(r.results[0], json!("Smith"));
    }

    #[test]
    fn test_where_filter() {
        let r = evaluate("Bundle.entry.where(resource.resourceType = 'Patient').count()", &bundle());
        assert_eq!(r.results[0], json!(2));
    }

    #[test]
    fn test_missing_path() {
        let r = evaluate("Patient.nonexistent", &patient());
        assert_eq!(r.count, 0);
        assert!(r.error.is_none());
    }

    #[test]
    fn test_invalid_expression() {
        let r = evaluate("Patient.name[", &patient());
        assert!(r.error.is_some());
    }
}
