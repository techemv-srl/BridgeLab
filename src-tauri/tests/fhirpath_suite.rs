//! Conformance harness for the official HL7 FHIRPath test suite.
//!
//! The suite is third-party material and is not vendored here; run
//! `./scripts/fetch-fhirpath-suite.sh` to download it, then point this test
//! at the result:
//!
//! ```text
//! BL_FHIRPATH_SUITE=.fhirpath-suite cargo test --test fhirpath_suite -- --nocapture
//! ```
//!
//! Without `BL_FHIRPATH_SUITE` the test reports that it was skipped and
//! passes, so a normal `cargo test` run stays offline and self-contained.
//!
//! The engine implements a large subset of FHIRPath rather than the whole
//! language (see `parser::fhir::fhirpath`), so the harness asserts against a
//! recorded baseline pass rate instead of demanding 100%: it fails when a
//! change makes conformance *worse*, and prints every failure so the gap is
//! visible rather than hidden.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use bridgelab_lib::parser::fhir::fhirpath;
use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::Value;

/// Pass rate the engine is known to reach over the runnable cases. Raise it
/// whenever conformance improves; never lower it to make a change pass.
///
/// The shortfall is concentrated in three places, all of them known:
///
/// - `lowBoundary()` / `highBoundary()` (52 cases) need exact decimal
///   arithmetic over the literal's written precision; this engine's numbers
///   are JSON doubles.
/// - Cases that expect an *unknown element* to be rejected (`name.given1`,
///   `Observation.valueQuantity`) and the `type()` of a FHIR primitive need
///   the StructureDefinitions loaded.
/// - Compound units (`2 'cm' * 2 'm'`) need a full UCUM engine.
const BASELINE_PASS_RATE: f64 = 0.90;

#[derive(Debug, Default, Clone)]
struct TestCase {
    group: String,
    name: String,
    input_file: String,
    expression: String,
    /// `invalid="syntax"` / `invalid="semantic"`: the expression must fail.
    expect_error: bool,
    /// `predicate="true"`: the expected output is whether the expression
    /// produced anything, not the values themselves.
    predicate: bool,
    outputs: Vec<(String, String)>,
}

#[test]
fn official_fhirpath_suite() {
    let Ok(dir) = std::env::var("BL_FHIRPATH_SUITE") else {
        eprintln!(
            "skipped: set BL_FHIRPATH_SUITE to the directory produced by \
             scripts/fetch-fhirpath-suite.sh to run the official suite"
        );
        return;
    };
    let dir = PathBuf::from(dir);
    let xml_path = dir.join("tests-fhir-r4.xml");
    let xml = fs::read_to_string(&xml_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", xml_path.display(), e));

    let cases = parse_suite(&xml).expect("the suite XML should parse");
    assert!(!cases.is_empty(), "no test cases found in {}", xml_path.display());

    let mut inputs: BTreeMap<String, Option<Value>> = BTreeMap::new();
    let mut passed = 0usize;
    let mut failed: Vec<(TestCase, String)> = Vec::new();
    let mut skipped: BTreeMap<String, usize> = BTreeMap::new();

    for case in &cases {
        let input = inputs
            .entry(case.input_file.clone())
            .or_insert_with(|| load_input(&dir, &case.input_file));

        let Some(root) = input else {
            *skipped.entry(case.input_file.clone()).or_default() += 1;
            continue;
        };

        match check_case(case, root) {
            Ok(()) => passed += 1,
            Err(why) => failed.push((case.clone(), why)),
        }
    }

    let runnable = passed + failed.len();
    let rate = if runnable == 0 {
        0.0
    } else {
        passed as f64 / runnable as f64
    };

    if !failed.is_empty() {
        eprintln!("\n--- failures ({}) ---", failed.len());
        for (case, why) in &failed {
            eprintln!("{}/{}: {}\n    {}", case.group, case.name, case.expression, why);
        }
    }
    if !skipped.is_empty() {
        eprintln!("\n--- skipped, input resource not available as JSON ---");
        for (file, n) in &skipped {
            eprintln!("{:<40} {} cases", file, n);
        }
    }
    eprintln!(
        "\nFHIRPath conformance: {}/{} runnable cases pass ({:.1}%), \
         {} skipped for missing inputs, {} total",
        passed,
        runnable,
        rate * 100.0,
        cases.len() - runnable,
        cases.len()
    );

    assert!(
        rate >= BASELINE_PASS_RATE,
        "conformance regressed: {:.1}% is below the {:.1}% baseline",
        rate * 100.0,
        BASELINE_PASS_RATE * 100.0
    );
}

/// Run one case and describe the mismatch, if any.
fn check_case(case: &TestCase, root: &Value) -> Result<(), String> {
    let result = fhirpath::evaluate(&case.expression, root);

    if case.expect_error {
        return match result.error {
            Some(_) => Ok(()),
            None => Err(format!(
                "expected the expression to be rejected, got {:?}",
                result.results
            )),
        };
    }

    if let Some(e) = result.error {
        return Err(format!("evaluation failed: {}", e));
    }

    // A predicate test asserts only whether the expression produced
    // anything, so compare existence rather than values.
    if case.predicate {
        let expected = case
            .outputs
            .first()
            .map(|(_, v)| v == "true")
            .unwrap_or(false);
        let got = !result.results.is_empty();
        return if got == expected {
            Ok(())
        } else {
            Err(format!("predicate: expected {}, got {}", expected, got))
        };
    }

    if result.results.len() != case.outputs.len() {
        return Err(format!(
            "expected {} value(s) {:?}, got {} {:?}",
            case.outputs.len(),
            case.outputs.iter().map(|(_, v)| v).collect::<Vec<_>>(),
            result.results.len(),
            result.results
        ));
    }

    for (actual, (kind, expected)) in result.results.iter().zip(case.outputs.iter()) {
        if !output_matches(actual, kind, expected) {
            return Err(format!(
                "expected {} {:?}, got {:?}",
                kind, expected, actual
            ));
        }
    }
    Ok(())
}

fn output_matches(actual: &Value, kind: &str, expected: &str) -> bool {
    match kind {
        "boolean" => actual.as_bool() == expected.parse::<bool>().ok(),
        "integer" => actual.as_i64() == expected.parse::<i64>().ok(),
        "decimal" => match (actual.as_f64(), expected.parse::<f64>()) {
            (Some(a), Ok(b)) => (a - b).abs() < 1e-9,
            _ => false,
        },
        "Quantity" => {
            // Rendered by the suite as `4 'mg'`.
            let Some(obj) = actual.as_object() else {
                return false;
            };
            let value = obj.get("value").map(render).unwrap_or_default();
            let unit = obj
                .get("code")
                .or_else(|| obj.get("unit"))
                .and_then(|u| u.as_str())
                .unwrap_or("");
            expected == format!("{} '{}'", value, unit)
        }
        // The suite writes temporal outputs as literals, `@` and all.
        "date" | "dateTime" | "time" => render(actual) == expected.trim_start_matches('@'),
        // string, code, id — plain text.
        _ => render(actual) == expected,
    }
}

fn render(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

/// The suite names inputs by their XML filename; this engine evaluates JSON,
/// so look for the JSON encoding of the same example.
fn load_input(dir: &Path, input_file: &str) -> Option<Value> {
    // Many cases are pure literal arithmetic and declare no input at all.
    if input_file.is_empty() {
        return Some(Value::Object(Default::default()));
    }
    let stem = Path::new(input_file).file_stem()?.to_str()?;
    let path = dir.join("input").join(format!("{}.json", stem));
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Minimal reader for the suite's XML shape:
/// `<group name><test name inputfile><expression invalid?/><output type/></test></group>`.
fn parse_suite(xml: &str) -> Result<Vec<TestCase>, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut cases = Vec::new();
    let mut group = String::new();
    let mut current: Option<TestCase> = None;
    let mut field: Option<String> = None;
    let mut output_type = String::new();
    let mut text = String::new();

    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Eof => break,
            Event::Empty(e) => {
                // Self-closing elements carry no text and get no End event.
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let attrs = attributes(&e)?;
                if name == "output" {
                    if let Some(c) = current.as_mut() {
                        c.outputs
                            .push((attrs.get("type").cloned().unwrap_or_default(), String::new()));
                    }
                }
            }
            Event::Start(e) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let attrs = attributes(&e)?;
                match name.as_str() {
                    "group" => group = attrs.get("name").cloned().unwrap_or_default(),
                    "test" => {
                        current = Some(TestCase {
                            group: group.clone(),
                            name: attrs.get("name").cloned().unwrap_or_default(),
                            input_file: attrs.get("inputfile").cloned().unwrap_or_default(),
                            predicate: attrs.get("predicate").map(String::as_str) == Some("true"),
                            ..TestCase::default()
                        });
                    }
                    "expression" => {
                        if let Some(c) = current.as_mut() {
                            c.expect_error = attrs.contains_key("invalid");
                        }
                        field = Some("expression".into());
                        text.clear();
                    }
                    "output" => {
                        output_type = attrs.get("type").cloned().unwrap_or_default();
                        field = Some("output".into());
                        text.clear();
                    }
                    _ => {}
                }
            }
            Event::Text(t) => {
                if field.is_some() {
                    text.push_str(&t.unescape().map_err(|e| e.to_string())?);
                }
            }
            Event::CData(t) => {
                if field.is_some() {
                    text.push_str(&String::from_utf8_lossy(&t));
                }
            }
            Event::End(e) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "expression" => {
                        if let Some(c) = current.as_mut() {
                            c.expression = text.trim().to_string();
                        }
                        field = None;
                    }
                    "output" => {
                        if let Some(c) = current.as_mut() {
                            c.outputs.push((output_type.clone(), text.trim().to_string()));
                        }
                        field = None;
                    }
                    "test" => {
                        if let Some(c) = current.take() {
                            cases.push(c);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    Ok(cases)
}

fn attributes(e: &quick_xml::events::BytesStart) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    for attr in e.attributes() {
        let attr = attr.map_err(|e| e.to_string())?;
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        let value = attr
            .unescape_value()
            .map_err(|e| e.to_string())?
            .to_string();
        out.insert(key, value);
    }
    Ok(out)
}
