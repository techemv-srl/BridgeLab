//! Behavioural tests for the FHIRPath engine.
//!
//! The lexer, parser and type modules carry their own unit tests; this file
//! exercises evaluation end-to-end through the public [`evaluate`] entry
//! point, the way the app calls it.

use super::*;
use serde_json::json;

fn patient() -> Value {
    json!({
        "resourceType": "Patient",
        "id": "p1",
        "active": true,
        "name": [
            {"use": "official", "family": "Smith", "given": ["Jane", "A"]},
            {"use": "nickname", "family": "Doe", "given": ["John"]}
        ],
        "telecom": [
            {"system": "phone", "value": "555-1234", "use": "home"},
            {"system": "email", "value": "jane@example.com"}
        ],
        "gender": "female",
        "birthDate": "1990-05-15",
        "deceasedBoolean": false,
        "extension": [
            {"url": "http://example.org/race", "valueString": "other"}
        ]
    })
}

fn bundle() -> Value {
    json!({
        "resourceType": "Bundle",
        "type": "collection",
        "entry": [
            {"resource": {"resourceType": "Patient", "id": "p1", "gender": "male"}},
            {"resource": {"resourceType": "Patient", "id": "p2", "gender": "female"}},
            {"resource": {"resourceType": "Observation", "id": "o1", "status": "final",
                          "subject": {"reference": "Patient/p2"},
                          "valueQuantity": {"value": 6.3, "unit": "mg", "code": "mg"}}}
        ]
    })
}

/// Evaluate and assert success, returning the result collection.
fn ok(expr: &str, root: &Value) -> Vec<Value> {
    let r = evaluate(expr, root);
    assert!(r.error.is_none(), "{}: {}", expr, r.error.unwrap());
    assert_eq!(r.count, r.results.len(), "{}: count disagrees with results", expr);
    r.results
}

/// Evaluate and assert a single value came back.
fn one(expr: &str, root: &Value) -> Value {
    let r = ok(expr, root);
    assert_eq!(r.len(), 1, "{}: expected one value, got {:?}", expr, r);
    r.into_iter().next().unwrap()
}

/// Evaluate and assert the empty collection came back.
fn empty(expr: &str, root: &Value) {
    assert_eq!(ok(expr, root), Vec::<Value>::new(), "{}", expr);
}

/// Evaluate and assert it failed.
fn err(expr: &str, root: &Value) -> String {
    let r = evaluate(expr, root);
    r.error
        .unwrap_or_else(|| panic!("{}: expected an error, got {:?}", expr, r.results))
}

// ---------------------------------------------------------------------------
// Behaviour the previous engine already had, kept as a regression floor
// ---------------------------------------------------------------------------

#[test]
fn navigates_paths_and_flattens_arrays() {
    assert_eq!(one("Patient.gender", &patient()), json!("female"));
    assert_eq!(
        ok("Patient.name.family", &patient()),
        vec![json!("Smith"), json!("Doe")]
    );
    assert_eq!(ok("Patient.name.given", &patient()).len(), 3);
}

#[test]
fn indexes_into_collections() {
    assert_eq!(one("Patient.name[0].family", &patient()), json!("Smith"));
    assert_eq!(one("Patient.name[1].family", &patient()), json!("Doe"));
    empty("Patient.name[9].family", &patient());
}

#[test]
fn a_missing_path_is_empty_not_an_error() {
    empty("Patient.nonexistent", &patient());
    empty("Patient.name.nonexistent.deeper", &patient());
}

#[test]
fn where_filters_and_count_counts() {
    assert_eq!(
        one(
            "Bundle.entry.where(resource.resourceType = 'Patient').count()",
            &bundle()
        ),
        json!(2)
    );
    assert_eq!(one("Patient.name.count()", &patient()), json!(2));
    assert_eq!(one("Patient.name.first().family", &patient()), json!("Smith"));
    assert_eq!(one("Patient.name.last().family", &patient()), json!("Doe"));
}

#[test]
fn reports_syntax_errors() {
    assert!(!err("Patient.name[", &patient()).is_empty());
    assert!(!err("Patient.(", &patient()).is_empty());
}

// ---------------------------------------------------------------------------
// Operators
// ---------------------------------------------------------------------------

#[test]
fn arithmetic_keeps_integer_and_decimal_apart() {
    let p = patient();
    assert_eq!(one("1 + 2", &p), json!(3));
    assert_eq!(one("7 div 2", &p), json!(3));
    assert_eq!(one("7 mod 2", &p), json!(1));
    // '/' is always decimal, even for two integers.
    assert_eq!(one("7 / 2", &p), json!(3.5));
    assert_eq!(one("1.5 + 1.5", &p), json!(3.0));
}

#[test]
fn division_by_zero_is_empty_rather_than_an_error() {
    let p = patient();
    empty("1 / 0", &p);
    empty("1 div 0", &p);
    empty("1 mod 0", &p);
}

#[test]
fn string_concatenation_distinguishes_plus_from_ampersand() {
    let p = patient();
    assert_eq!(one("'a' + 'b'", &p), json!("ab"));
    // '+' propagates empty, '&' treats it as an empty string — the whole
    // reason both operators exist.
    empty("'a' + Patient.nonexistent", &p);
    assert_eq!(one("'a' & Patient.nonexistent", &p), json!("a"));
}

#[test]
fn boolean_operators_use_three_valued_logic() {
    let p = patient();
    assert_eq!(one("true and false", &p), json!(false));
    assert_eq!(one("true or false", &p), json!(true));
    assert_eq!(one("true xor false", &p), json!(true));

    // Empty is the unknown value: false and unknown is still false, but
    // true and unknown is unknown.
    assert_eq!(one("false and {}", &p), json!(false));
    empty("true and {}", &p);
    assert_eq!(one("true or {}", &p), json!(true));
    empty("false or {}", &p);
    empty("{} xor true", &p);
}

#[test]
fn implies_follows_the_spec_truth_table() {
    let p = patient();
    assert_eq!(one("false implies true", &p), json!(true));
    assert_eq!(one("false implies false", &p), json!(true));
    assert_eq!(one("true implies true", &p), json!(true));
    assert_eq!(one("true implies false", &p), json!(false));
    // Anything implies unknown-but-true is true; true implies unknown is not.
    assert_eq!(one("false implies {}", &p), json!(true));
    empty("true implies {}", &p);
}

#[test]
fn comparison_propagates_empty() {
    let p = patient();
    assert_eq!(one("2 > 1", &p), json!(true));
    assert_eq!(one("'a' < 'b'", &p), json!(true));
    empty("Patient.nonexistent > 1", &p);
}

#[test]
fn equality_versus_equivalence() {
    let p = patient();
    assert_eq!(one("'ABC' = 'abc'", &p), json!(false));
    // '~' normalises case and whitespace.
    assert_eq!(one("'ABC' ~ 'abc'", &p), json!(true));
    assert_eq!(one("'a  b' ~ 'a b'", &p), json!(true));
    // '=' on an empty operand is empty; '~' answers true for two empties.
    empty("{} = {}", &p);
    assert_eq!(one("{} ~ {}", &p), json!(true));
    assert_eq!(one("{} !~ {}", &p), json!(false));
}

#[test]
fn membership_operators() {
    let p = patient();
    assert_eq!(one("'Jane' in Patient.name.given", &p), json!(true));
    assert_eq!(one("'Nobody' in Patient.name.given", &p), json!(false));
    assert_eq!(one("Patient.name.given contains 'John'", &p), json!(true));
}

#[test]
fn union_removes_duplicates_and_combine_keeps_them() {
    let p = patient();
    assert_eq!(ok("(1 | 2 | 2 | 3)", &p).len(), 3);
    assert_eq!(one("(1 | 2 | 2).count()", &p), json!(2));
    assert_eq!(one("(1).combine(1).count()", &p), json!(2));
}

#[test]
fn operator_precedence_matches_the_spec() {
    let p = patient();
    assert_eq!(one("1 + 2 * 3", &p), json!(7));
    assert_eq!(one("(1 + 2) * 3", &p), json!(9));
    assert_eq!(one("true or false and false", &p), json!(true));
    assert_eq!(one("2 > 1 and 3 > 2", &p), json!(true));
}

// ---------------------------------------------------------------------------
// Existence, filtering, subsetting
// ---------------------------------------------------------------------------

#[test]
fn existence_functions() {
    let p = patient();
    assert_eq!(one("Patient.name.exists()", &p), json!(true));
    assert_eq!(one("Patient.nonexistent.exists()", &p), json!(false));
    assert_eq!(one("Patient.nonexistent.empty()", &p), json!(true));
    assert_eq!(
        one("Patient.name.exists(use = 'nickname')", &p),
        json!(true)
    );
    assert_eq!(one("Patient.name.all(family.exists())", &p), json!(true));
    // all() over an empty collection is vacuously true.
    assert_eq!(one("Patient.nonexistent.all(false)", &p), json!(true));
}

#[test]
fn boolean_aggregates() {
    let p = patient();
    assert_eq!(one("(true | true).allTrue()", &p), json!(true));
    assert_eq!(one("(true | false).allTrue()", &p), json!(false));
    assert_eq!(one("(true | false).anyTrue()", &p), json!(true));
    assert_eq!(one("(false | false).allFalse()", &p), json!(true));
    assert_eq!(one("(true | false).anyFalse()", &p), json!(true));
}

#[test]
fn subsetting_functions() {
    let p = patient();
    assert_eq!(one("Patient.name.tail().count()", &p), json!(1));
    assert_eq!(one("Patient.name.given.skip(1).count()", &p), json!(2));
    assert_eq!(one("Patient.name.given.take(2).count()", &p), json!(2));
    assert_eq!(one("Patient.name.given.take(0).count()", &p), json!(0));
    assert_eq!(one("Patient.gender.single()", &p), json!("female"));
    // single() on more than one item is an error, not a silent first().
    assert!(err("Patient.name.single()", &p).contains("single()"));
}

#[test]
fn set_operations() {
    let p = patient();
    assert_eq!(one("(1 | 2 | 3).intersect(2 | 3 | 4).count()", &p), json!(2));
    assert_eq!(one("(1 | 2 | 3).exclude(2).count()", &p), json!(2));
    assert_eq!(one("(1 | 2).subsetOf(1 | 2 | 3)", &p), json!(true));
    assert_eq!(one("(1 | 2 | 3).supersetOf(1 | 2)", &p), json!(true));
    assert_eq!(one("(1 | 2 | 2).isDistinct()", &p), json!(true));
    assert_eq!(one("(1).combine(1).isDistinct()", &p), json!(false));
}

#[test]
fn select_projects_and_repeat_walks_recursive_structures() {
    let p = patient();
    assert_eq!(ok("Patient.name.select(family)", &p).len(), 2);

    let questionnaire = json!({
        "resourceType": "Questionnaire",
        "item": [
            {"linkId": "1", "item": [
                {"linkId": "1.1"},
                {"linkId": "1.2", "item": [{"linkId": "1.2.1"}]}
            ]},
            {"linkId": "2"}
        ]
    });
    // Two top-level items, plus 1.1, 1.2 and 1.2.1 found by recursion.
    assert_eq!(
        one("Questionnaire.repeat(item).count()", &questionnaire),
        json!(5)
    );
}

#[test]
fn this_and_index_are_bound_inside_lambdas() {
    let p = patient();
    assert_eq!(
        one("Patient.name.given.where($this = 'Jane').count()", &p),
        json!(1)
    );
    assert_eq!(
        one("Patient.name.given.where($index = 0).count()", &p),
        json!(1)
    );
}

#[test]
fn aggregate_threads_a_running_total() {
    let p = patient();
    assert_eq!(
        one("(1 | 2 | 3).aggregate($this + $total, 0)", &p),
        json!(6)
    );
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[test]
fn type_operators_and_of_type() {
    let b = bundle();
    assert_eq!(
        one("Bundle.entry.resource.ofType(Patient).count()", &b),
        json!(2)
    );
    assert_eq!(
        one("Bundle.entry.resource.ofType(Observation).count()", &b),
        json!(1)
    );

    let p = patient();
    assert_eq!(one("Patient.active is Boolean", &p), json!(true));
    assert_eq!(one("Patient.gender is String", &p), json!(true));
    assert_eq!(one("Patient.birthDate is Date", &p), json!(true));
    assert_eq!(one("Patient.active is String", &p), json!(false));
    assert_eq!(one("(1).is(Integer)", &p), json!(true));
    assert_eq!(one("Patient.gender as String", &p), json!("female"));
    empty("Patient.gender as Integer", &p);
}

#[test]
fn choice_elements_resolve_through_their_type_suffix() {
    let b = bundle();
    // Observation.valueQuantity is reached as `value`, per the FHIR
    // JSON encoding of choice elements.
    assert_eq!(
        one(
            "Bundle.entry.resource.ofType(Observation).value.unit",
            &b
        ),
        json!("mg")
    );
    assert_eq!(
        one(
            "Bundle.entry.resource.ofType(Observation).value.ofType(Quantity).value",
            &b
        ),
        json!(6.3)
    );
}

#[test]
fn type_reports_a_namespace_and_a_name() {
    let p = patient();
    assert_eq!(
        one("Patient.active.type().name", &p),
        json!("Boolean")
    );
    assert_eq!(
        one("Patient.active.type().namespace", &p),
        json!("System")
    );
}

// ---------------------------------------------------------------------------
// Dates
// ---------------------------------------------------------------------------

#[test]
fn dates_compare_by_precision() {
    let p = patient();
    assert_eq!(one("@2015-02-04 < @2015-02-05", &p), json!(true));
    assert_eq!(one("@2015-02-04 = @2015-02-04", &p), json!(true));
    // Equal as far as both go, but one is more precise: indeterminate.
    empty("@2015-02-04 = @2015-02", &p);
    // Already different at a shared component: determinate.
    assert_eq!(one("@2015-02-04 = @2016-02", &p), json!(false));
    assert_eq!(one("Patient.birthDate < @2000-01-01", &p), json!(true));
}

// ---------------------------------------------------------------------------
// Strings and math
// ---------------------------------------------------------------------------

#[test]
fn string_functions() {
    let p = patient();
    assert_eq!(one("'hello'.length()", &p), json!(5));
    assert_eq!(one("'hello'.upper()", &p), json!("HELLO"));
    assert_eq!(one("'HELLO'.lower()", &p), json!("hello"));
    assert_eq!(one("'  x '.trim()", &p), json!("x"));
    assert_eq!(one("'hello'.substring(1, 3)", &p), json!("ell"));
    assert_eq!(one("'hello'.substring(2)", &p), json!("llo"));
    assert_eq!(one("'hello'.startsWith('he')", &p), json!(true));
    assert_eq!(one("'hello'.endsWith('lo')", &p), json!(true));
    assert_eq!(one("'hello'.contains('ell')", &p), json!(true));
    assert_eq!(one("'hello'.indexOf('l')", &p), json!(2));
    assert_eq!(one("'hello'.indexOf('z')", &p), json!(-1));
    assert_eq!(one("'hello'.replace('l', 'L')", &p), json!("heLLo"));
    assert_eq!(one("'hello'.matches('^h.*o$')", &p), json!(true));
    assert_eq!(one("'a,b,c'.split(',').count()", &p), json!(3));
    assert_eq!(one("('a' | 'b').join('-')", &p), json!("a-b"));
    assert_eq!(one("'abc'.toChars().count()", &p), json!(3));
    // Out-of-range substring is empty, not an error.
    empty("'hello'.substring(99)", &p);
}

#[test]
fn string_functions_count_characters_not_bytes() {
    let p = patient();
    assert_eq!(one("'café'.length()", &p), json!(4));
    assert_eq!(one("'café'.substring(3)", &p), json!("é"));
    assert_eq!(one("'aéb'.indexOf('b')", &p), json!(2));
}

#[test]
fn math_functions() {
    let p = patient();
    assert_eq!(one("(-3).abs()", &p), json!(3));
    assert_eq!(one("(1.5).ceiling()", &p), json!(2));
    assert_eq!(one("(1.5).floor()", &p), json!(1));
    assert_eq!(one("(1.7).truncate()", &p), json!(1));
    assert_eq!(one("(4).sqrt()", &p), json!(2.0));
    assert_eq!(one("(2.71828).round(2)", &p), json!(2.72));
    assert_eq!(one("(2).power(3)", &p), json!(8));
    assert_eq!(one("(100).log(10)", &p), json!(2.0));
    // Outside the domain the result is empty rather than NaN.
    empty("(-1).sqrt()", &p);
    empty("(0).ln()", &p);
}

#[test]
fn conversion_functions() {
    let p = patient();
    assert_eq!(one("'42'.toInteger()", &p), json!(42));
    assert_eq!(one("'3.5'.toDecimal()", &p), json!(3.5));
    assert_eq!(one("'true'.toBoolean()", &p), json!(true));
    assert_eq!(one("(42).toString()", &p), json!("42"));
    assert_eq!(one("'abc'.convertsToInteger()", &p), json!(false));
    assert_eq!(one("'42'.convertsToInteger()", &p), json!(true));
    // A failed conversion yields empty, while the convertsTo* form says so.
    empty("'abc'.toInteger()", &p);
}

#[test]
fn iif_only_evaluates_the_branch_it_takes() {
    let p = patient();
    assert_eq!(one("iif(true, 'yes', 'no')", &p), json!("yes"));
    assert_eq!(one("iif(false, 'yes', 'no')", &p), json!("no"));
    empty("iif(false, 'yes')", &p);
    // The untaken branch would error if it were evaluated.
    assert_eq!(
        one("iif(true, 'safe', Patient.name.single())", &p),
        json!("safe")
    );
}

#[test]
fn not_follows_three_valued_logic() {
    let p = patient();
    assert_eq!(one("true.not()", &p), json!(false));
    assert_eq!(one("Patient.active.not()", &p), json!(false));
    empty("{}.not()", &p);
}

// ---------------------------------------------------------------------------
// Tree navigation and FHIR extensions
// ---------------------------------------------------------------------------

#[test]
fn children_and_descendants() {
    let p = patient();
    assert!(!ok("Patient.children()", &p).is_empty());
    assert!(
        ok("Patient.descendants().count()", &p)[0].as_i64().unwrap()
            > ok("Patient.children().count()", &p)[0].as_i64().unwrap()
    );
}

#[test]
fn extension_selects_by_url() {
    let p = patient();
    assert_eq!(
        one("Patient.extension('http://example.org/race').value", &p),
        json!("other")
    );
    empty("Patient.extension('http://example.org/nope')", &p);
}

#[test]
fn resolve_finds_bundled_and_contained_resources() {
    let b = bundle();
    assert_eq!(
        one(
            "Bundle.entry.resource.ofType(Observation).subject.resolve().id",
            &b
        ),
        json!("p2")
    );

    let contained = json!({
        "resourceType": "Observation",
        "contained": [{"resourceType": "Patient", "id": "inner"}],
        "subject": {"reference": "#inner"}
    });
    assert_eq!(
        one("Observation.subject.resolve().id", &contained),
        json!("inner")
    );
}

#[test]
fn resolve_follows_a_urn_uuid_reference_to_the_entry_full_url() {
    // A message bundle: no ids anywhere, every reference is the fullUrl of
    // another entry.
    let b = json!({
        "resourceType": "Bundle", "type": "message",
        "entry": [
            {"fullUrl": "urn:uuid:7c7266e5-ac58-4485-a862-7561427be104",
             "resource": {"resourceType": "Patient", "name": [{"family": "Wrexham"}]}},
            {"fullUrl": "urn:uuid:6f198391-2f49-4a34-b20f-77450e1d01e3",
             "resource": {"resourceType": "Observation", "status": "final",
                          "subject": {"reference": "urn:uuid:7c7266e5-ac58-4485-a862-7561427be104"}}}
        ]
    });
    assert_eq!(
        one(
            "Bundle.entry.resource.ofType(Observation).subject.resolve().name.family",
            &b
        ),
        json!("Wrexham")
    );
}

#[test]
fn has_value_distinguishes_primitives_from_elements() {
    let p = patient();
    assert_eq!(one("Patient.gender.hasValue()", &p), json!(true));
    assert_eq!(one("Patient.name.first().hasValue()", &p), json!(false));
}

#[test]
fn trace_passes_values_through_and_records_them() {
    let r = evaluate("Patient.name.trace('names').count()", &patient());
    assert!(r.error.is_none());
    assert_eq!(r.results[0], json!(2));
    assert_eq!(r.trace.len(), 1);
    assert_eq!(r.trace[0].name, "names");
    assert_eq!(r.trace[0].values.len(), 2);
}

#[test]
fn environment_constants_reach_the_root_resource() {
    let b = bundle();
    assert_eq!(one("%resource.type", &b), json!("collection"));
    assert_eq!(
        one("Bundle.entry.resource.where(id = %context.entry.resource.id.first()).count()", &b),
        json!(1)
    );
}

// ---------------------------------------------------------------------------
// Ordering, durations and units
// ---------------------------------------------------------------------------

#[test]
fn sort_orders_by_natural_comparison_and_by_keys() {
    let p = patient();
    assert_eq!(ok("(3 | 1 | 2).sort()", &p), vec![json!(1), json!(2), json!(3)]);
    assert_eq!(
        ok("('c' | 'a' | 'b').sort($this)", &p),
        vec![json!("a"), json!("b"), json!("c")]
    );
    // A leading '-' marks the key descending — it is a direction, not
    // arithmetic, so it works on strings too.
    assert_eq!(
        ok("('a' | 'c' | 'b').sort(-$this)", &p),
        vec![json!("c"), json!("b"), json!("a")]
    );
    assert_eq!(
        one("Patient.name.sort(family).first().family", &p),
        json!("Doe")
    );
}

#[test]
fn dates_take_durations_and_keep_their_precision() {
    let p = patient();
    assert_eq!(one("@2015-02-04 + 1 day", &p), json!("2015-02-05"));
    assert_eq!(one("@2015-02-04 + 1 week", &p), json!("2015-02-11"));
    assert_eq!(one("@2015-02-04 - 1 month", &p), json!("2015-01-04"));
    assert_eq!(one("@2015-02-04 + 1 year", &p), json!("2016-02-04"));
    // Month arithmetic clamps to the shorter month.
    assert_eq!(one("@2015-01-31 + 1 month", &p), json!("2015-02-28"));
    // A year-precision date stays at year precision.
    assert_eq!(one("@2015 + 1 year", &p), json!("2016"));
    // Durations count whole units of their own unit.
    assert_eq!(one("@2015-02-04 + 7.7 days", &p), json!("2015-02-11"));
    assert_eq!(
        one("@2015-02-04T10:00:00 + 90 minutes", &p),
        json!("2015-02-04T11:30:00")
    );
}

#[test]
fn a_date_rejects_a_duration_it_cannot_use() {
    let p = patient();
    // UCUM 'a' and 'mo' are fixed lengths and do not line up with calendar
    // arithmetic, so they are refused rather than silently approximated.
    assert!(err("@2015-02-04 + 1 'a'", &p).contains("calendar duration"));
    assert!(err("@2015-02-04 - 1 'cm'", &p).contains("not a duration"));
}

#[test]
fn quantities_convert_within_a_dimension() {
    let p = patient();
    assert_eq!(one("4 'g' = 4000 'mg'", &p), json!(true));
    assert_eq!(one("1 'm' > 99 'cm'", &p), json!(true));
    assert_eq!(one("1 'kg' + 500 'g'", &p), json!({"value": 1.5, "unit": "kg"}));
    assert_eq!(one("(2 'mg' * 3)", &p), json!({"value": 6, "unit": "mg"}));
    // Equivalence compares at the coarser precision.
    assert_eq!(one("4 'g' ~ 4040 'mg'", &p), json!(true));
    // Units outside the conversion table are unknowable, not unequal.
    empty("1 'mg' = 1 '[foo]'", &p);
    assert_eq!(one("1 'cm'.comparable(1 '[in_i]')", &p), json!(true));
    assert_eq!(one("1 'cm'.comparable(1 's')", &p), json!(false));
}

#[test]
fn calendar_and_ucum_durations_are_not_interchangeable_for_months() {
    let p = patient();
    // A calendar month has no fixed length, so it cannot be measured
    // against UCUM's 30-day 'mo'.
    empty("1 month = 1 'mo'", &p);
    // Units that do have a fixed length convert normally.
    assert_eq!(one("1 week = 7 'd'", &p), json!(true));
}

#[test]
fn string_codecs() {
    let p = patient();
    assert_eq!(one("'test'.encode('base64')", &p), json!("dGVzdA=="));
    assert_eq!(one("'test'.encode('hex')", &p), json!("74657374"));
    assert_eq!(one("'dGVzdA=='.decode('base64')", &p), json!("test"));
    assert_eq!(
        one("'subjects?_d'.encode('urlbase64')", &p),
        json!("c3ViamVjdHM_X2Q=")
    );
    assert_eq!(one(r#"'"a"'.escape('json')"#, &p), json!(r#"\"a\""#));
    assert_eq!(one("'<a>'.escape('html')", &p), json!("&lt;a&gt;"));
    assert_eq!(one("'&amp;lt;'.unescape('html')", &p), json!("&lt;"));
    // An unknown codec produces nothing rather than a wrong answer.
    empty("'test'.encode('rot13')", &p);
}

#[test]
fn matches_full_anchors_the_pattern() {
    let p = patient();
    assert_eq!(one("'abc'.matches('b')", &p), json!(true));
    assert_eq!(one("'abc'.matchesFull('b')", &p), json!(false));
    assert_eq!(one("'abc'.matchesFull('a.c')", &p), json!(true));
}

#[test]
fn precision_counts_significant_digits_of_a_temporal() {
    let p = patient();
    assert_eq!(one("@2014.precision()", &p), json!(4));
    assert_eq!(one("@2014-01-05T10:30:00.000.precision()", &p), json!(17));
    assert_eq!(one("@T10:30.precision()", &p), json!(4));
}

#[test]
fn decimal_arithmetic_does_not_leak_binary_float_error() {
    let p = patient();
    // 1.8 - 1.2 is 0.5999999999999999 in binary floating point.
    assert_eq!(one("1.8 - 1.2 = 0.6", &p), json!(true));
    assert_eq!(one("2.2 mod 1.8 = 0.4", &p), json!(true));
    assert_eq!(one("0.1 + 0.2 = 0.3", &p), json!(true));
}

#[test]
fn arguments_resolve_against_the_context_not_the_input() {
    let p = patient();
    // `name.family` must find the family names even though the function's
    // input is the given names.
    assert_eq!(
        one("Patient.name.given.combine(Patient.name.family).count()", &p),
        json!(5)
    );
    // Inside a lambda the context is the item being iterated.
    assert_eq!(
        one("Patient.name.select(use.union(given)).count()", &p),
        json!(5)
    );
}

#[test]
fn iif_requires_a_real_boolean_and_a_single_input() {
    let p = patient();
    assert!(err("iif('not a boolean', 1, 2)", &p).contains("boolean criteria"));
    assert!(err("Patient.name.iif(true, 1, 2)", &p).contains("single input"));
    // Inside iif, $this is the input.
    assert_eq!(one("('ctx').iif($this = 'ctx', 'yes', 'no')", &p), json!("yes"));
}

// ---------------------------------------------------------------------------
// Error reporting
// ---------------------------------------------------------------------------

#[test]
fn unknown_names_are_reported_rather_than_silently_ignored() {
    let p = patient();
    assert!(err("Patient.nosuchfunction()", &p).contains("Unknown function"));
    assert!(err("$nope", &p).contains("Unknown variable"));
    assert!(err("%nope", &p).contains("Unknown environment constant"));
}

#[test]
fn operators_that_need_one_value_say_so() {
    let p = patient();
    let msg = err("Patient.name.given > 1", &p);
    assert!(msg.contains("single value"), "{}", msg);
}

#[test]
fn check_validates_without_evaluating() {
    assert!(check("Patient.name.where(use = 'official')").is_ok());
    assert!(check("Patient.name[").is_err());
}
