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
fn type_tests_on_a_choice_element_use_the_type_in_its_key() {
    let obs = json!({"resourceType": "Observation", "id": "o1", "status": "final",
        "code": {"text": "x"}, "effectiveDateTime": "2016-03-28", "valueString": "abc",
        "extension": [{"url": "u", "valueString": "s"}]});
    // A day-precision dateTime is still a dateTime, not a date.
    assert_eq!(one("Observation.effective.ofType(dateTime)", &obs), json!("2016-03-28"));
    assert_eq!(one("Observation.effective is dateTime", &obs), json!(true));
    assert_eq!(one("Observation.effective.is(DateTime)", &obs), json!(true));
    assert_eq!(one("Observation.effective is date", &obs), json!(false));
    empty("Observation.effective.ofType(date)", &obs);
    assert_eq!(one("(Observation.effective as dateTime)", &obs), json!("2016-03-28"));
    assert_eq!(one("effective.exists() implies effective.ofType(dateTime).exists() or effective.ofType(Period).exists()", &obs), json!(true));
    // A string is a string, not a code or a dateTime.
    assert_eq!(one("Observation.value.ofType(string)", &obs), json!("abc"));
    empty("Observation.extension.value.ofType(code)", &obs);
    let dt = json!({"resourceType": "Observation", "valueDateTime": "2020-01-01"});
    empty("Observation.value.ofType(string)", &dt);
    // Subtypes: a code is a string, an Age is a Quantity.
    let cond = json!({"resourceType": "Condition", "onsetAge": {"value": 3, "unit": "a"},
        "abatementString": "later"});
    assert_eq!(one("Condition.onset.ofType(Quantity).value", &cond), json!(3));
    assert_eq!(one("Condition.onset is Age", &cond), json!(true));
    let code = json!({"resourceType": "Observation", "valueCode": "x"});
    assert_eq!(one("Observation.value is string", &code), json!(true));
    assert_eq!(one("Observation.value is System.String", &code), json!(true));
    // The SearchParameter expression for Condition.onset-date.
    let onset = json!({"resourceType": "Condition", "onsetDateTime": "2016-03-28"});
    assert_eq!(one("Condition.onset.as(dateTime) | Condition.onset.as(Period)", &onset), json!("2016-03-28"));
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
fn resolve_matches_type_and_server_not_just_the_id() {
    let b = json!({"resourceType": "Bundle", "type": "collection", "entry": [
        {"fullUrl": "http://x/fhir/Patient/123",
         "resource": {"resourceType": "Patient", "id": "123", "name": [{"family": "WrongPerson"}]}},
        {"fullUrl": "http://x/fhir/Organization/2",
         "resource": {"resourceType": "Organization", "id": "2"}},
        {"fullUrl": "http://x/fhir/Observation/o1",
         "resource": {"resourceType": "Observation", "id": "o1", "status": "final",
            "performer": [{"reference": "Practitioner/123"},
                          {"reference": "http://elsewhere.example/fhir/Patient/123"}],
            "subject": {"reference": "Patient/123/_history/2"},
            "focus": [{"reference": "http://x/fhir/Patient/123"}]}}
    ]});
    empty("Bundle.entry[2].resource.performer.resolve()", &b);
    // A versioned reference names the resource, not a resource with id 2.
    assert_eq!(one("Bundle.entry[2].resource.subject.resolve().resourceType", &b), json!("Patient"));
    assert_eq!(one("Bundle.entry[2].resource.focus.resolve().id", &b), json!("123"));
}

/// Two servers' entries share Patient/p8: a relative reference resolves on
/// the referring entry's own server, whichever entry comes first.
#[test]
fn resolve_prefers_the_referring_entrys_server() {
    let b = |first: &str, second: &str| json!({"resourceType": "Bundle", "type": "collection", "entry": [
        {"fullUrl": format!("{first}/Patient/p8"),
         "resource": {"resourceType": "Patient", "id": "p8", "name": [{"family": first}]}},
        {"fullUrl": format!("{second}/Patient/p8"),
         "resource": {"resourceType": "Patient", "id": "p8", "name": [{"family": second}]}},
        {"fullUrl": "http://b.example/fhir/Observation/o1",
         "resource": {"resourceType": "Observation", "id": "o1", "status": "final",
            "subject": {"reference": "Patient/p8"}}}
    ]});
    let family = "Bundle.entry[2].resource.subject.resolve().name.family";
    assert_eq!(one(family, &b("http://a.example/fhir", "http://b.example/fhir")), json!("http://b.example/fhir"));
    assert_eq!(one(family, &b("http://b.example/fhir", "http://a.example/fhir")), json!("http://b.example/fhir"));
    // Neither server is the referring entry's: nothing, not another
    // server's Patient/p8.
    empty("Bundle.entry[2].resource.subject.resolve()", &b("http://a.example/fhir", "http://c.example/fhir"));
    // An entry with a urn fullUrl has no server: by type and id, which
    // name two different patients here, so nothing.
    let mut urn = b("http://a.example/fhir", "http://c.example/fhir");
    urn["entry"][2]["fullUrl"] = json!("urn:uuid:3f1c0c1e-0000-4000-8000-000000000001");
    empty("Bundle.entry[2].resource.subject.resolve()", &urn);
}

/// Two servers each send the same Observation (identical JSON) and their
/// own Patient/1: each entry's reference resolves on its own server. The
/// path says which entry it is in; content alone cannot.
#[test]
fn identical_resources_from_two_servers_resolve_on_their_own_server() {
    let obs = json!({"resourceType": "Observation", "id": "x", "status": "final", "subject": {"reference": "Patient/1"},
        "effectiveDateTime": "2024-01-01",
        "_effectiveDateTime": {"id": "e", "extension": [{"url": "http://example.org/by", "valueReference": {"reference": "Patient/1"}}]},
        "extension": [{"url": "x", "valueReference": {"reference": "Patient/1"}}]});
    let b = json!({"resourceType": "Bundle", "type": "collection", "entry": [
        {"fullUrl": "http://a.example/fhir/Patient/1", "resource": {"resourceType": "Patient", "id": "1", "name": [{"family": "A"}]}},
        {"fullUrl": "http://b.example/fhir/Patient/1", "resource": {"resourceType": "Patient", "id": "1", "name": [{"family": "B"}]}},
        {"fullUrl": "http://a.example/fhir/Observation/x", "resource": obs.clone()},
        {"fullUrl": "http://b.example/fhir/Observation/x", "resource": obs.clone()}
    ]});
    assert_eq!(one("Bundle.entry[2].resource.subject.resolve().name.family", &b), json!("A"));
    assert_eq!(one("Bundle.entry[3].resource.subject.resolve().name.family", &b), json!("B"));
    // A primitive's extensions, in its `_effectiveDateTime` sibling, sit in
    // the same resource and entry.
    let by = |i: usize| one(&format!("Bundle.entry[{i}].resource.effectiveDateTime.extension.valueReference.resolve().name.family"), &b);
    assert_eq!(by(2), json!("A"));
    assert_eq!(by(3), json!("B"));
    assert_eq!(ok("Bundle.entry.resource.ofType(Observation).effectiveDateTime.extension.value.resolve().name.family", &b), vec![json!("A"), json!("B")]);
    assert_eq!(one("Bundle.entry[3].resource.effectiveDateTime.id", &b), json!("e"));
    // A resolve() inside a lambda knows the entry of the item it runs on.
    let obs_path = |rest: &str| ok(&format!("Bundle.entry.resource.ofType(Observation).{rest}"), &b);
    let a_b = vec![json!("A"), json!("B")];
    assert_eq!(obs_path("select(subject.resolve()).name.family"), a_b);
    assert_eq!(obs_path("select($this.subject.resolve()).name.family"), a_b);
    assert_eq!(obs_path("select(subject).select(resolve()).name.family"), a_b);
    assert_eq!(obs_path("where(subject.resolve().exists()).subject.resolve().name.family"), a_b);
    assert_eq!(obs_path("where(subject.resolve().name.family = 'B').count()"), vec![json!(1)]);
    assert_eq!(obs_path("all(subject.resolve().exists())"), vec![json!(true)]);
    assert_eq!(obs_path("exists(subject.resolve().name.family = 'A')"), vec![json!(true)]);
    assert_eq!(obs_path("select(iif(subject.resolve().exists(), subject.resolve(), {})).name.family"), a_b);
    assert_eq!(
        one("Bundle.entry[3].resource.aggregate($total | subject.resolve().name.family, {})", &b),
        json!("B")
    );
    assert_eq!(one("(Bundle.entry[3].resource as Observation).subject.resolve().name.family", &b), json!("B"));
    // extension(url) results sit in the item's resource and entry.
    assert_eq!(obs_path("extension('x').valueReference.resolve().name.family"), a_b);
    assert_eq!(obs_path("effectiveDateTime.extension('http://example.org/by').value.resolve().name.family"), a_b);
    assert_eq!(one("Bundle.entry.where(fullUrl.contains('b.example')).resource.subject.resolve().name.family", &b), json!("B"));
    assert_eq!(ok("Bundle.entry.resource.subject.resolve().name.family", &b), vec![json!("A"), json!("B")]);
    // Functions that return their input items as they are keep each one's entry.
    let families = |path: &str| ok(&format!("Bundle.entry.resource.ofType(Observation).{path}.subject.resolve().name.family"), &b);
    assert_eq!(families("exclude({})"), vec![json!("A"), json!("B")]);
    assert_eq!(families("trace('x')"), vec![json!("A"), json!("B")]);
    // intersect() drops duplicates: the two copies are equal, the first stays.
    assert_eq!(families("intersect(Bundle.entry.resource.ofType(Observation))"), vec![json!("A")]);
    assert_eq!(families("exclude(Bundle.entry[0].resource)"), vec![json!("A"), json!("B")]);
    assert_eq!(one("Bundle.entry[3].resource.as(Observation).subject.resolve().name.family", &b), json!("B"));
    // Any function or operator that passes items through, not a list of them.
    assert_eq!(families("combine({})"), vec![json!("A"), json!("B")]);
    assert_eq!(families("select($this)"), vec![json!("A"), json!("B")]);
    assert_eq!(families("select(iif(true, $this, {}))"), vec![json!("A"), json!("B")]);
    assert_eq!(one("Bundle.entry[2].resource.iif(true, $this, {}).subject.resolve().name.family", &b), json!("A"));
    assert_eq!(one("Bundle.entry[3].resource.iif(true, $this, {}).subject.resolve().name.family", &b), json!("B"));
    // A projection keeps the item's entry for what it reads from the item.
    assert_eq!(
        ok("Bundle.entry.resource.ofType(Observation).select(subject).resolve().name.family", &b),
        vec![json!("A"), json!("B")]
    );
    assert_eq!(
        ok("(Bundle.entry[2].resource | Bundle.entry[3].resource).combine({}).subject.resolve().name.family", &b),
        vec![json!("A")],
        "| drops the equal second copy, as union does"
    );
    // union() and | drop equal duplicates: the two copies are one item, the
    // first, with its own entry (as intersect() and distinct() keep it).
    assert_eq!(families("union({})"), vec![json!("A")]);
    assert_eq!(
        ok("(Bundle.entry.resource.ofType(Observation) | {}).subject.resolve().name.family", &b),
        vec![json!("A")]
    );
    assert_eq!(
        ok("({} | Bundle.entry[3].resource).subject.resolve().name.family", &b),
        vec![json!("B")]
    );
    // A function that returns one of several equal copies from servers
    // that disagree: which one cannot be told, so nothing.
    assert_eq!(families("aggregate($this)"), Vec::<Value>::new());
    assert_eq!(families("repeat($this)"), Vec::<Value>::new());
    // Without the path through the entry (a rule run on the Observation
    // itself, inside the Bundle), the copies cannot be told apart and
    // their servers disagree: nothing rather than either patient.
    let r = evaluate_within("subject.resolve()", &obs, &b);
    assert!(r.error.is_none(), "{:?}", r.error);
    assert!(r.results.is_empty(), "{:?}", r.results);
}

/// A contained resource stays in its container's entry: its relative
/// references resolve on that entry's server, even when another server's
/// resource of the same type and id comes first. `#id` is unchanged.
#[test]
fn a_contained_resource_resolves_on_its_containers_server() {
    let b = json!({"resourceType": "Bundle", "type": "collection", "entry": [
        {"fullUrl": "http://b.example/fhir/Patient/1", "resource": {"resourceType": "Patient", "id": "1", "name": [{"family": "B"}]}},
        {"fullUrl": "http://a.example/fhir/Patient/1", "resource": {"resourceType": "Patient", "id": "1", "name": [{"family": "A"}]}},
        {"fullUrl": "http://a.example/fhir/DiagnosticReport/r",
         "resource": {"resourceType": "DiagnosticReport", "id": "r", "status": "final",
            "contained": [{"resourceType": "Observation", "id": "o", "status": "final", "subject": {"reference": "Patient/1"}}],
            "result": [{"reference": "#o"}]}}
    ]});
    assert_eq!(one("Bundle.entry[2].resource.contained[0].subject.resolve().name.family", &b), json!("A"));
    assert_eq!(one("Bundle.entry[2].resource.contained.subject.resolve().name.family", &b), json!("A"));
    assert_eq!(one("Bundle.entry[2].resource.result.resolve().id", &b), json!("o"));
    assert_eq!(one("Bundle.entry[2].resource.result.resolve().subject.reference", &b), json!("Patient/1"));
    // Functions that pass items through keep the entry they came from.
    let family = |path: &str| one(&format!("{path}.subject.resolve().name.family"), &b);
    assert_eq!(family("Bundle.entry.resource.ofType(DiagnosticReport).first().contained[0]"), json!("A"));
    assert_eq!(family("Bundle.entry.resource.where(resourceType = 'DiagnosticReport').contained.last()"), json!("A"));
    assert_eq!(family("Bundle.entry.resource.skip(2).take(1).single().contained.distinct()"), json!("A"));
    assert_eq!(family("Bundle.entry.tail().tail().resource.contained"), json!("A"));
    // Without a path through the entry (the report itself, or a resolve()
    // of the contained one), the contained resource is found inside A's
    // entry by content: A's patient, not the first Patient/1.
    let r = evaluate_within("contained[0].subject.resolve().name.family", &b["entry"][2]["resource"], &b);
    assert_eq!(r.results, vec![json!("A")], "{:?}", r.error);
    assert_eq!(one("Bundle.entry[2].resource.result.resolve().subject.resolve().name.family", &b), json!("A"));
}

/// The same report, contained Observation included, from two servers that
/// each have their own Patient/1, evaluated with no path through the
/// entries: the copies cannot be told apart and their servers disagree,
/// so nothing, never one server's patient for the other's report. With
/// the path, each gets its own.
#[test]
fn a_contained_resource_in_identical_copies_resolves_to_nothing_without_its_entry() {
    let report = json!({"resourceType": "DiagnosticReport", "id": "r", "status": "final",
        "contained": [{"resourceType": "Observation", "id": "o", "status": "final", "subject": {"reference": "Patient/1"}}],
        "result": [{"reference": "#o"}]});
    let b = json!({"resourceType": "Bundle", "type": "collection", "entry": [
        {"fullUrl": "http://b.example/fhir/Patient/1", "resource": {"resourceType": "Patient", "id": "1", "name": [{"family": "B"}]}},
        {"fullUrl": "http://a.example/fhir/Patient/1", "resource": {"resourceType": "Patient", "id": "1", "name": [{"family": "A"}]}},
        {"fullUrl": "http://a.example/fhir/DiagnosticReport/r", "resource": report.clone()},
        {"fullUrl": "http://b.example/fhir/DiagnosticReport/r", "resource": report.clone()}
    ]});
    let r = evaluate_within("contained[0].subject.resolve()", &report, &b);
    assert!(r.error.is_none() && r.results.is_empty(), "{:?} {:?}", r.results, r.error);
    let r = evaluate_within("subject.resolve()", &b["entry"][2]["resource"]["contained"][0], &b);
    assert!(r.results.is_empty(), "{:?}", r.results);
    assert_eq!(one("Bundle.entry[2].resource.contained[0].subject.resolve().name.family", &b), json!("A"));
    assert_eq!(one("Bundle.entry[3].resource.contained[0].subject.resolve().name.family", &b), json!("B"));
    // A resource outside the Bundle goes by type and id, which names two
    // different patients here: nothing, not the first one listed.
    let outside = json!({"resourceType": "Observation", "id": "z", "status": "final", "subject": {"reference": "Patient/1"}});
    assert!(evaluate_within("subject.resolve()", &outside, &b).results.is_empty());
}

/// A resolved resource is in the entry it was found in: a resolve()
/// chained on it answers on that server, even when the resource has an
/// identical copy on another one. A contained target is in its
/// container's entry.
#[test]
fn a_chained_resolve_answers_on_the_resolved_resources_server() {
    let patient = json!({"resourceType": "Patient", "id": "1", "managingOrganization": {"reference": "Organization/1"}});
    let obs = json!({"resourceType": "Observation", "id": "x", "status": "final", "subject": {"reference": "Patient/1"}});
    let with_contained = json!({"resourceType": "Observation", "id": "y", "status": "final",
        "contained": [{"resourceType": "Patient", "id": "p", "managingOrganization": {"reference": "Organization/1"}}],
        "subject": {"reference": "#p"}});
    let b = json!({"resourceType": "Bundle", "type": "collection", "entry": [
        {"fullUrl": "http://a.example/fhir/Observation/x", "resource": obs.clone()},
        {"fullUrl": "http://b.example/fhir/Observation/x", "resource": obs.clone()},
        {"fullUrl": "http://a.example/fhir/Patient/1", "resource": patient.clone()},
        {"fullUrl": "http://b.example/fhir/Patient/1", "resource": patient.clone()},
        {"fullUrl": "http://b.example/fhir/Organization/1", "resource": {"resourceType": "Organization", "id": "1", "name": "B org"}},
        {"fullUrl": "http://a.example/fhir/Organization/1", "resource": {"resourceType": "Organization", "id": "1", "name": "A org"}},
        {"fullUrl": "http://a.example/fhir/Observation/y", "resource": with_contained}
    ]});
    let org = |i: usize| one(&format!("Bundle.entry[{i}].resource.subject.resolve().managingOrganization.resolve().name"), &b);
    assert_eq!(org(0), json!("A org"));
    assert_eq!(org(1), json!("B org"));
    assert_eq!(org(6), json!("A org"), "the contained Patient is in A's entry");
    assert_eq!(
        ok("Bundle.entry.resource.ofType(Observation).subject.resolve().managingOrganization.resolve().name", &b),
        vec![json!("A org"), json!("B org"), json!("A org")]
    );
    assert_eq!(
        ok("Bundle.entry.resource.ofType(Observation).select(subject.resolve().managingOrganization.resolve().name)", &b),
        vec![json!("A org"), json!("B org"), json!("A org")]
    );
}

/// A reference with no holder at all (built by repeat(), here) has no
/// server to go by: it resolves by type and id only when the Bundle holds
/// one such resource, whatever servers it sits under.
#[test]
fn a_reference_with_no_known_server_resolves_only_when_type_and_id_name_one_resource() {
    let obs = json!({"resourceType": "Observation", "id": "x", "status": "final", "subject": {"reference": "Patient/1"}});
    let bundle = |patients: Vec<(&str, &str)>| {
        let mut entry: Vec<Value> = patients
            .into_iter()
            .map(|(url, family)| json!({"fullUrl": url, "resource": {"resourceType": "Patient", "id": "1", "name": [{"family": family}]}}))
            .collect();
        entry.push(json!({"fullUrl": "http://a.example/fhir/Observation/x", "resource": obs.clone()}));
        entry.push(json!({"fullUrl": "http://b.example/fhir/Observation/x", "resource": obs.clone()}));
        json!({"resourceType": "Bundle", "type": "collection", "entry": entry})
    };
    let path = "Bundle.entry.resource.ofType(Observation).repeat(subject).resolve().name.family";
    let two = bundle(vec![("http://a.example/fhir/Patient/1", "A"), ("http://b.example/fhir/Patient/1", "B")]);
    assert_eq!(ok(path, &two), Vec::<Value>::new());
    let one_patient = bundle(vec![("http://a.example/fhir/Patient/1", "A")]);
    assert_eq!(ok(path, &one_patient), vec![json!("A")]);
    // The same patient twice (identical copies) is one resource.
    let copies = bundle(vec![("http://a.example/fhir/Patient/1", "A"), ("urn:uuid:3f1c0c1e-0000-4000-8000-0000000000aa", "A")]);
    assert_eq!(ok(path, &copies), vec![json!("A")]);
    // With the path through the entries, each still resolves on its server.
    assert_eq!(ok("Bundle.entry.resource.ofType(Observation).subject.resolve().name.family", &two), vec![json!("A"), json!("B")]);
}

#[test]
fn a_versioned_reference_resolves_only_to_that_version() {
    let b = json!({"resourceType": "Bundle", "type": "collection", "entry": [
        {"fullUrl": "http://x/fhir/Patient/1",
         "resource": {"resourceType": "Patient", "id": "1", "meta": {"versionId": "3"}}},
        {"fullUrl": "http://x/fhir/Observation/o1",
         "resource": {"resourceType": "Observation", "id": "o1", "status": "final",
            "subject": {"reference": "http://x/fhir/Patient/1/_history/3"},
            "focus": [{"reference": "http://x/fhir/Patient/1/_history/2"},
                      {"reference": "Patient/1/_history/2"}]}}
    ]});
    assert_eq!(one("Bundle.entry[1].resource.subject.resolve().id", &b), json!("1"));
    // Version 2 is not in the Bundle: the entry holds version 3.
    empty("Bundle.entry[1].resource.focus.resolve()", &b);
}

#[test]
fn resolve_finds_a_resource_contained_in_a_bundle_entry() {
    let b = json!({"resourceType": "Bundle", "type": "collection", "entry": [
        {"resource": {"resourceType": "Observation", "id": "o1",
            "contained": [{"resourceType": "Practitioner", "id": "c1", "name": [{"family": "One"}]}],
            "basedOn": [{"reference": "#c1"}]}},
        {"resource": {"resourceType": "Observation", "id": "o2",
            "contained": [{"resourceType": "Practitioner", "id": "c1", "name": [{"family": "Two"}]}],
            "performer": [{"reference": "#c1"}]}}
    ]});
    assert_eq!(one("Bundle.entry[0].resource.basedOn.resolve().name.family", &b), json!("One"));
    assert_eq!(one("Bundle.entry[1].resource.performer.resolve().name.family", &b), json!("Two"));
    // A contained resource is not reachable as Practitioner/c1.
    let r = json!({"resourceType": "Observation",
        "contained": [{"resourceType": "Practitioner", "id": "c1"}],
        "performer": [{"reference": "Practitioner/c1"}]});
    empty("Observation.performer.resolve()", &r);
}

#[test]
fn resolve_over_a_large_bundle_indexes_once() {
    let n = 3000;
    let entries: Vec<Value> = (0..n)
        .map(|i| json!({"fullUrl": format!("urn:uuid:e{}", i),
            "resource": {"resourceType": "Observation", "id": format!("o{}", i), "status": "final",
                "subject": {"reference": "urn:uuid:e0"}}}))
        .collect();
    let b = json!({"resourceType": "Bundle", "type": "collection", "entry": entries});
    let start = std::time::Instant::now();
    assert_eq!(one("Bundle.entry.resource.subject.resolve().count()", &b), json!(n));
    assert!(start.elapsed().as_secs() < 10, "took {:?}", start.elapsed());
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

/// A date moved past the years 1 to 9999 is empty, as an integer that
/// overflows is: never a panic (the app's release build aborts on one),
/// never a wrapped or saturated date.
#[test]
fn date_arithmetic_out_of_range_is_empty() {
    let p = patient();
    for expr in [
        "@2024-01-01 + 100000000 days",
        "@2024-01-01 + 14285714285 weeks",
        "@2024-01-01 + 8640000000000000 seconds",
        "@2024-01-01 - 99999999999 days",
        "@2024-01-01 + 9223372036854775807 days",
        "@2024 + 99999999999 days",
        "@2024-01 + 400000000 days",
        "@2024-01-01 + 3000000 days",
        "@2024-01-01 - 738886 days",
        // Calendar units, which wrapped or named the wrong cause.
        "@2024-01-01 + 9223372036854775807 years",
        "@2024-01-01 + 2147483647 years",
        "@0001-01-01 - 5 years",
        "@9999-12 + 1 month",
        // A date and time: these saturated at 292 years and read as 2316.
        "@2024-01-01T00:00:00 + 99999999999 days",
        "@2024-01-01T00:00:00 + 9223372036854775807 seconds",
        "@9999-12-31T23:59:59 + 1 second",
        // Past what a double holds exactly: not the number written.
        "@T23:59:59 + 9223372036854775807 seconds",
    ] {
        empty(expr, &p);
    }
    // Rules that compare such a value get empty, not a crash.
    empty("(Patient.birthDate + 100000000 days) > today()", &p);
    // The duration can come from the resource itself.
    for value in [json!(100000000000i64), json!(1e300)] {
        let enc = json!({"resourceType": "Encounter", "period": {"start": "2024-01-01"},
            "length": {"value": value, "unit": "d", "code": "d", "system": "http://unitsofmeasure.org"}});
        empty("Encounter.period.start + Encounter.length", &enc);
    }
}

#[test]
fn date_arithmetic_still_reaches_far_dates() {
    let p = patient();
    assert_eq!(one("@2024-01-01 + 2000000 days", &p), json!("7499-10-25"));
    assert_eq!(one("@2024-01-01 - 738885 days", &p), json!("0001-01-01"));
    assert_eq!(one("@2024-01-01 + 100000 weeks", &p), json!("3940-07-15"));
    assert_eq!(one("@2024-01-01 + 7975 years", &p), json!("9999-01-01"));
    assert_eq!(one("@0001-02 - 1 month", &p), json!("0001-01"));
    // Beyond 292 years of elapsed time (where nanoseconds ran out).
    assert_eq!(one("@2024-01-01T00:00:00 + 200000 days", &p), json!("2571-08-01T00:00:00"));
    assert_eq!(
        one("@2024-01-01T10:00:00 + 15768000007 seconds", &p),
        json!("2523-09-02T10:00:07")
    );
    assert_eq!(one("@9999-12-31T23:59:58 + 1 second", &p), json!("9999-12-31T23:59:59"));
    // A time of day goes round the clock.
    assert_eq!(one("@T23:59:59 + 1 second", &p), json!("00:00:00"));
    assert_eq!(one("@T10:00 + 1000000 days", &p), json!("10:00"));
    assert_eq!(one("@T10:00 - 90 minutes", &p), json!("08:30"));
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
    // Prefixes on any metric unit, and one unit per another.
    assert_eq!(one("1 'mmol/L' = 1000 'umol/L'", &p), json!(true));
    assert_eq!(one("5.5 'mmol/L' > 5000 'umol/L'", &p), json!(true));
    assert_eq!(one("100 'mg/dL' = 1 'g/L'", &p), json!(true));
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
fn precision_of_a_decimal_literal_counts_the_digits_as_written() {
    let p = patient();
    assert_eq!(one("1.58700.precision()", &p), json!(5));
    assert_eq!(one("(-1.5).precision()", &p), json!(1));
}

#[test]
fn decimal_boundaries_are_exact_and_follow_the_written_precision() {
    let p = patient();
    let num = |e: &str| one(e, &p).as_f64().unwrap_or_else(|| panic!("{}: not a number", e));
    // Half a unit of the last digit written, default 8 digits.
    assert_eq!(num("1.587.lowBoundary()"), 1.5865);
    assert_eq!(num("1.587.highBoundary()"), 1.5875);
    assert_eq!(num("(-1.587).lowBoundary()"), -1.5875);
    // Trailing zeros count: 12.500 has three digits, 12.5 one.
    assert_eq!(num("12.500.lowBoundary(4)"), 12.4995);
    assert_eq!(num("12.5.lowBoundary(4)"), 12.45);
    // An integer is exact to the unit.
    assert_eq!(num("120.lowBoundary(2)"), 119.5);
    assert_eq!(num("1.highBoundary(0)"), 2.0);
    // Fewer digits: towards zero inside, away from zero outside.
    assert_eq!(num("1.587.lowBoundary(2)"), 1.58);
    assert_eq!(num("1.587.highBoundary(2)"), 1.59);
    assert_eq!(num("(-1.587).lowBoundary(2)"), -1.59);
    assert_eq!(num("(-9.96).lowBoundary(1)"), -10.0);
    assert_eq!(num("0.996.highBoundary(1)"), 1.0);
    assert_eq!(num("0.0034.highBoundary(1)"), 0.0);
    // A result is a Decimal even when it is whole.
    assert_eq!(one("1.587.lowBoundary(0) is Decimal", &p), json!(true));
    // No result for a precision out of range or an empty one.
    empty("1.587.lowBoundary(-1)", &p);
    empty("1.587.lowBoundary(32)", &p);
    empty("1.587.lowBoundary({})", &p);
}

#[test]
fn decimals_from_the_resource_keep_the_digits_they_were_written_with() {
    // 6.30 used to read back as the double 6.3: precision() said 1 and
    // the boundaries were those of 6.3, unlike the literal 6.30.
    let observation_json = r#"{"resourceType": "Observation", "id": "o", "status": "final",
        "code": {"text": "x"}, "valueQuantity": {"value": 6.30, "unit": "mmol/L"}}"#;
    let observation_xml = r#"<Observation xmlns="http://hl7.org/fhir"><id value="o"/><status value="final"/>
        <code><text value="x"/></code><valueQuantity><value value="6.30"/><unit value="mmol/L"/></valueQuantity></Observation>"#;
    let parameters_json = r#"{"resourceType": "Parameters", "parameter": [
        {"name": "big", "valueDecimal": 12345678901234567890.5},
        {"name": "exp", "valueDecimal": 9.5e2},
        {"name": "small", "valueDecimal": 1.5E-3}]}"#;
    let parameters_xml = r#"<Parameters xmlns="http://hl7.org/fhir">
        <parameter><name value="big"/><valueDecimal value="12345678901234567890.5"/></parameter>
        <parameter><name value="exp"/><valueDecimal value="9.5e2"/></parameter>
        <parameter><name value="small"/><valueDecimal value="1.5E-3"/></parameter></Parameters>"#;
    let from_json = |text: &str| crate::parser::fhir::parse_fhir_json(text).unwrap().json_value.unwrap();
    let from_xml = |text: &str| crate::parser::fhir::parse_fhir_xml(text).unwrap().json_value.unwrap();
    let read = [
        (from_json(observation_json), from_json(parameters_json)),
        (from_xml(observation_xml), from_xml(parameters_xml)),
    ];
    for (o, parameters) in &read {
        let text = |e: &str| one(e, o).to_string();
        assert_eq!(one("Observation.value.value.precision()", o), json!(2));
        assert_eq!(text("Observation.value.value.lowBoundary()"), "6.29500000");
        assert_eq!(text("Observation.value.value.highBoundary()"), "6.30500000");
        assert_eq!(one("Observation.value.value.lowBoundary() = 6.30.lowBoundary()", o), json!(true));
        assert_eq!(one("Observation.value.lowBoundary().value = 6.295", o), json!(true));
        assert_eq!(one("Observation.value.value.toString()", o), json!("6.30"));
        assert_eq!(one("Observation.value.toString()", o), json!("6.30 'mmol/L'"));
        assert_eq!(one("(-Observation.value.value).toString()", o), json!("-6.30"));
        // Still the same number as 6.3 to compare with.
        assert_eq!(one("Observation.value.value = 6.3", o), json!(true));
        assert_eq!(one("Observation.value.value ~ 6.3", o), json!(true));
        assert_eq!(one("Observation.value.value > 6.29", o), json!(true));

        let p = |name: &str, f: &str| {
            let r = one(&format!("Parameters.parameter.where(name = '{}').value.{}", name, f), parameters);
            assert!(!r.is_string() || f.starts_with("toString"), "{name}.{f}: {r:?} is still text");
            r
        };
        // More digits than a double holds: kept, not 1.2345678901234567e19.
        assert_eq!(p("big", "toString()"), json!("12345678901234567890.5"));
        assert_eq!(p("big", "precision()"), json!(1));
        assert_eq!(p("big", "lowBoundary()").to_string(), "12345678901234567890.45000000");
        // An exponent moves the point: 9.5e2 is known to the tens.
        assert_eq!(p("exp", "lowBoundary(0)").to_string(), "945.0");
        assert_eq!(p("exp", "highBoundary(1)").to_string(), "955.0");
        assert_eq!(p("small", "precision()"), json!(4));
        assert_eq!(p("small", "lowBoundary()").to_string(), "0.00145000");
    }
}

#[test]
fn numbers_beyond_a_double_are_read_without_a_crash() {
    // Numbers keeping their text means a document may now hold one that no
    // double reaches; nothing may unwrap it as one.
    let r = crate::parser::fhir::parse_fhir_json(
        r#"{"resourceType": "Parameters", "parameter": [
            {"name": "huge", "valueDecimal": 1e2147483647}, {"name": "tiny", "valueDecimal": 1.5e-400}]}"#,
    )
    .unwrap()
    .json_value
    .unwrap();
    for name in ["huge", "tiny"] {
        let at = |f: &str| format!("Parameters.parameter.where(name = '{}').value{}", name, f);
        empty(&at(".lowBoundary()"), &r);
        empty(&at(".highBoundary(2)"), &r);
        // A double holds 1.5e-400 as 0; it has no value at all for 1e2147483647.
        if name == "huge" {
            empty(&at(" > 1"), &r);
        } else {
            assert_eq!(one(&at(" > 1"), &r), json!(false));
        }
        assert_eq!(one(&at(" = 1"), &r), json!(false));
        assert_eq!(one(&at(" = %resource.parameter.where(name = '{n}').value").replace("{n}", name), &r), json!(true));
        assert_eq!(one(&at(".precision()"), &r).as_i64().map(|p| p >= 0), Some(true));
        assert!(one(&at(".toString()"), &r).as_str().unwrap().contains('e'));
        let _ = evaluate(&at(" + 1"), &r);
        let _ = evaluate(&at(".round(2)"), &r);
        let _ = evaluate(&at(" ~ 1.0"), &r);
    }
    let q = crate::parser::fhir::parse_fhir_json(
        r#"{"resourceType": "Observation", "valueQuantity": {"value": 1e2147483647, "unit": "mg"}}"#,
    )
    .unwrap()
    .json_value
    .unwrap();
    for e in ["abs()", "lowBoundary()", "toString()"] {
        let _ = evaluate(&format!("Observation.value.{}", e), &q);
    }
    for e in [" > 1 'mg'", " + 1 'mg'", " = 1 'mg'", " ~ 1 'mg'", " * 2"] {
        let _ = evaluate(&format!("Observation.value{}", e), &q);
    }
}

#[test]
fn quantity_and_temporal_boundaries() {
    let p = patient();
    let q = one("1.587 'cm'.lowBoundary(8)", &p);
    // Written to the precision asked for, as the specification shows it.
    assert_eq!(q["value"].to_string(), "1.58650000");
    assert_eq!(q["unit"], json!("cm"));
    // The quantity's written digits count too: 12.500 has three.
    assert_eq!(one("12.500 'cm'.lowBoundary(4)", &p)["value"], json!(12.4995));
    assert_eq!(one("(-12.500 'cm').highBoundary(4)", &p)["value"], json!(-12.4995));
    assert_eq!(one("@2014.lowBoundary(6)", &p), json!("2014-01"));
    assert_eq!(one("@2014.highBoundary(6)", &p), json!("2014-12"));
    assert_eq!(one("@2016-02.highBoundary(8)", &p), json!("2016-02-29"));
    // No zone: the extreme zones; a zone is kept.
    assert_eq!(one("@2014-01-01T08:05.lowBoundary(17)", &p), json!("2014-01-01T08:05:00.000+14:00"));
    assert_eq!(one("@2014-01-01T08:05-05:00.highBoundary(17)", &p), json!("2014-01-01T08:05:59.999-05:00"));
    // Cut to the date, a dateTime becomes a date.
    assert_eq!(one("@2014-01-01T08:05.lowBoundary(8)", &p), json!("2014-01-01"));
    assert_eq!(one("@T10:30.highBoundary(9)", &p), json!("T10:30:59.999"));
    assert_eq!(one("@T10:30:15.5.lowBoundary(9)", &p), json!("T10:30:15.500"));
    // One or two digits of a second.
    assert_eq!(one("@T10:30.lowBoundary(7)", &p), json!("T10:30:00.0"));
    assert_eq!(one("@2014-01-01T08:00.highBoundary(16)", &p), json!("2014-01-01T08:00:59.99-12:00"));
    // An hour alone: its minutes take their extreme value, as the
    // specification's example has it.
    assert_eq!(one("@2014-01-01T08.highBoundary(17)", &p), json!("2014-01-01T08:59:59.999-12:00"));
    assert_eq!(one("@2014-01-01T08.lowBoundary(17)", &p), json!("2014-01-01T08:00:00.000+14:00"));
    // A precision the kind does not have.
    empty("@2014-01-01.lowBoundary(17)", &p);
    empty("@T10:30.lowBoundary(5)", &p);
    empty("@T10:30.lowBoundary(10)", &p);
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

#[test]
fn hex_decode_of_non_ascii_text_is_empty_not_a_crash() {
    // Slicing '0é0' two bytes at a time used to cut 'é' in half and abort
    // the app.
    empty("'0é0'.decode('hex')", &patient());
    assert_eq!(one("'41'.decode('hex')", &patient()), Value::String("A".into()));
}

#[test]
fn deeply_nested_expressions_are_refused_not_a_crash() {
    let p = patient();
    let parens = format!("{}1{}", "(".repeat(20000), ")".repeat(20000));
    assert!(err(&parens, &p).contains("nested more than"));
    // A flat sum is one chain, not 50,000 levels: it is refused for its
    // length instead.
    let sum = format!("1{}", "+1".repeat(50000));
    assert!(err(&sum, &p).contains("more than 4096 terms"));
    let nested_sum = format!("{}1{}", "1+(".repeat(20000), ")".repeat(20000));
    assert!(err(&nested_sum, &p).contains("nested more than"));
    let minus = format!("{}1", "-(".repeat(20000));
    assert!(err(&minus, &p).contains("nested more than"));
    let path = format!("Patient{}", ".name".repeat(20000));
    assert!(err(&path, &p).contains("nested more than"));
    let calls = format!("{}true{}", "not(".repeat(20000), ")".repeat(20000));
    assert!(err(&calls, &p).contains("nested more than"));
    assert!(super::check(&parens).is_err());
}

#[test]
fn expressions_up_to_the_nesting_limit_evaluate_on_a_small_stack() {
    // 1 MiB is the main thread's stack on Windows.
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(|| {
            let p = patient();
            let n = super::parser::MAX_DEPTH - 8;
            let sum = format!("1{}", "+1".repeat(n));
            assert_eq!(one(&sum, &p), json!(n as i64 + 1));
            let parens = format!("{}1{}", "(".repeat(n), ")".repeat(n));
            assert_eq!(one(&parens, &p), json!(1));
            let mixed = format!("{}true{}", "(true and ".repeat(n / 2), ")".repeat(n / 2));
            assert_eq!(one(&mixed, &p), json!(true));
            let calls = format!("{}Patient.active{}", "iif(true, ".repeat(n / 2), ", false)".repeat(n / 2));
            assert_eq!(one(&calls, &p), json!(true));
            // Criteria inside criteria, about as deep as the limit allows:
            // each level runs a where() and an exists() on every item.
            let one_name = json!({"resourceType": "Patient", "name": [{"family": "A"}]});
            let mut criteria = "%resource.name".to_string();
            for _ in 0..28 {
                criteria = format!("%resource.name.where({criteria}.exists())");
            }
            assert_eq!(one(&format!("{criteria}.exists()"), &one_name), json!(true));
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn long_flat_lists_evaluate_on_a_small_stack() {
    // A list of codes or alternatives used to count one nesting level per
    // term, so the 64th item was refused. 2 MiB is a test thread's stack.
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(|| {
            let p = patient();
            let codes: Vec<String> = (0..999).map(|i| format!("'c{}'", i)).collect();
            let list = format!("{} | 'female'", codes.join(" | "));
            assert_eq!(one(&format!("Patient.gender in ({})", list), &p), json!(true));
            assert_eq!(one(&format!("({}).count()", list), &p), json!(1000));
            // Duplicates are still dropped, keeping the first occurrence.
            assert_eq!(ok(&format!("{} | 'c0' | 'female'", list), &p).len(), 1000);
            assert_eq!(one(&format!("Patient.gender in ({}).exclude('female')", list), &p), json!(false));

            let alternatives: Vec<String> =
                (0..999).map(|i| format!("Patient.gender = 'c{}'", i)).collect();
            let or_chain = format!("{} or Patient.gender = 'female'", alternatives.join(" or "));
            assert_eq!(one(&or_chain, &p), json!(true));
            let and_chain = format!("{} and Patient.active", ["Patient.active"; 999].join(" and "));
            assert_eq!(one(&and_chain, &p), json!(true));

            // The longest chain allowed evaluates; one more term is refused.
            let n = super::parser::MAX_TERMS;
            let sum = format!("1{}", "+1".repeat(n - 1));
            assert_eq!(one(&sum, &p), json!(n as i64));
            let too_long = format!("1{}", "+1".repeat(n));
            assert!(err(&too_long, &p).contains("more than 4096 terms"));
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn a_chain_of_mixed_operators_folds_left_to_right() {
    let p = patient();
    assert_eq!(one("10 - 2 - 3 + 1", &p), json!(6));
    assert_eq!(one("2 + 3 * 4 - 1", &p), json!(13));
    assert_eq!(one("1 + 1 = 2 and 2 * 2 = 4 or false", &p), json!(true));
    assert_eq!(one("(Patient.gender | 'x' | Patient.gender).count()", &p), json!(2));
    assert_eq!(
        ok("Patient.name.given | 'Jane' | 'Z'", &p),
        vec![json!("Jane"), json!("A"), json!("John"), json!("Z")]
    );
    assert_eq!(one("1 + 2 is Integer", &p), json!(true));
    // Three-valued logic still short-circuits along the chain: the
    // comparison in the middle would fail if it were evaluated.
    assert_eq!(one("false and Patient.name.given > 1 and true", &p), json!(false));
    assert_eq!(one("true or Patient.name.given > 1 or false", &p), json!(true));
    assert!(err("true and Patient.name.given > 1", &p).contains("single value"));
}

#[test]
fn date_arithmetic_stays_on_real_dates() {
    let p = json!({"resourceType": "Patient", "id": "x", "birthDate": "2008-02-29"});
    assert_eq!(one("Patient.birthDate + 18 years", &p), json!("2026-02-28"));
    assert_eq!(one("@2016-02-29 + 1 year", &p), json!("2017-02-28"));
    assert_eq!(one("@2016-02-29 + 4 years", &p), json!("2020-02-29"));
    assert_eq!(one("@2016-02-29 + 1 year + 1 day", &p), json!("2017-03-01"));
    assert!(err("@2015-02-30", &p).contains("not a valid date"));
    assert!(err("@2015-13", &p).contains("not a valid date"));
    assert_eq!(one("'2015-02-30'.convertsToDate()", &p), json!(false));
    assert_eq!(one("'2016-02-29'.convertsToDate()", &p), json!(true));
    empty("'2015-02-30'.toDate()", &p);
}

#[test]
fn extensions_on_primitives_are_read_from_the_underscore_sibling() {
    let url = "http://hl7.org/fhir/StructureDefinition/patient-birthTime";
    let p = json!({"resourceType": "Patient", "id": "x", "birthDate": "1970-01-01",
        "_birthDate": {"id": "bd", "extension": [{"url": url, "valueDateTime": "1970-01-01T10:00:00Z"}]},
        "name": [{"given": ["A", "B"], "_given": [null, {"extension": [{"url": "u", "valueString": "v"}]}]}]});
    assert_eq!(one("Patient.birthDate.extension.count()", &p), json!(1));
    assert_eq!(
        one(&format!("Patient.birthDate.extension('{}').value", url), &p),
        json!("1970-01-01T10:00:00Z")
    );
    assert_eq!(one("Patient.birthDate.id", &p), json!("bd"));
    assert_eq!(one("Patient.name.given.extension('u').value", &p), json!("v"));
    // A resource's own extensions and a primitive without any are unchanged.
    assert_eq!(one("Patient.id", &p), json!("x"));
    assert_eq!(one("Patient.name.extension.count()", &p), json!(0));
}

#[test]
fn conforms_to_says_it_is_not_implemented_not_that_packages_are_missing() {
    let e = err("Patient.conformsTo('http://hl7.org/fhir/StructureDefinition/Patient')", &patient());
    assert!(e.contains("not implemented"), "{e}");
    assert!(!e.contains("does not load"), "{e}");
}

#[test]
fn integer_arithmetic_is_exact_and_overflow_is_empty() {
    let p = patient();
    empty("2147483647 + 1", &p);
    empty("-2147483648 - 1", &p);
    empty("65536 * 65536", &p);
    empty("9223372036854775807 + 1", &p);
    assert_eq!(one("2147483646 + 1", &p), json!(2147483647));
    assert_eq!(one("7 div 2", &p), json!(3));
    assert_eq!(one("-7 mod 3", &p), json!(-1));
    empty("5 div 0", &p);
    empty("5 mod 0", &p);
    // Values beyond 32 bits from the data are held to 64 bits, exactly.
    let big = json!({"resourceType": "Observation", "valueInteger64": 9007199254740993i64});
    assert_eq!(one("Observation.value + 0", &big), json!(9007199254740993i64));
    assert_eq!(one("1.5 + 1", &p), json!(2.5));
}

/// `seed` followed by `step` `times` times.
fn chain(seed: &str, step: &str, times: usize) -> String {
    format!("{}{}", seed, step.repeat(times))
}

/// A string too long to build is refused before it is built: a failed
/// allocation aborts the whole app, and a short expression can ask for
/// terabytes.
#[test]
fn string_growth_stops_at_the_string_cap() {
    let p = patient();
    let tenfold = ".replace('a', 'aaaaaaaaaa')";
    for expr in [
        // 10^12 bytes.
        chain("'a'", tenfold, 12),
        // Between every character: 2.6 GB.
        chain("'abc'", ".replace('', 'XXXXXXXXXXXXXXXXXXXX')", 7),
        chain("'a'", ".replaceMatches('a', 'aaaaaaaaaa')", 12),
        // Each match copied ten times over, by number and by name.
        chain("'ab'", ".replaceMatches('.+', '$0$0$0$0$0$0$0$0$0$0')", 9),
        chain("'ab'", ".replaceMatches('(?P<c>.)', '${c}${c}${c}${c}${c}${c}${c}${c}${c}${c}')", 9),
    ] {
        let e = err(&format!("{expr}.length()"), &p);
        assert!(e.contains("would exceed 16 MB"), "{expr}: {e}");
    }
    // The message names what refused.
    let e = err(&format!("{}.length()", chain("'a'", tenfold, 12)), &p);
    assert!(e.starts_with("replace() result would exceed 16 MB"), "{e}");
    // Strings of 10, 5 and 8 MB, then ways of growing them past 16 MB.
    let ten_mb = chain("'a'", tenfold, 7);
    let one_mb = chain("'a'", tenfold, 6);
    for (base, grow) in [
        (ten_mb.clone(), "$this & $this"),
        (ten_mb.clone(), "$this + $this"),
        (ten_mb.clone(), "($this | 'b').join($this)"),
        (ten_mb.clone(), "$this.encode('hex')"),
        (ten_mb.clone(), "$this.encode('base64') & $this"),
        (format!("{one_mb}.replace('a', '<<<<<')"), "$this.escape('html')"),
        (format!("{one_mb}.replace('a', 'ΐΐΐΐ')"), "$this.upper()"),
    ] {
        let e = err(&format!("{base}.select({grow}).length()"), &p);
        assert!(e.contains("would exceed 16 MB"), "{grow}: {e}");
    }
}

#[test]
fn strings_under_the_cap_are_built() {
    let p = patient();
    let tenfold = ".replace('a', 'aaaaaaaaaa')";
    assert_eq!(one(&format!("{}.length()", chain("'a'", tenfold, 5)), &p), json!(100000));
    assert_eq!(one(&format!("{}.length()", chain("'a'", tenfold, 7)), &p), json!(10_000_000));
    assert_eq!(
        one(&format!("{}.length()", chain("'a'", ".replaceMatches('a', 'aaaaaaaaaa')", 5)), &p),
        json!(100000)
    );
    assert_eq!(one("'abc'.replace('', '-')", &p), json!("-a-b-c-"));
    assert_eq!(one("'aXbX'.replace('X', 'YY')", &p), json!("aYYbYY"));
    assert_eq!(one("('a' | 'b' | 'c').join(', ')", &p), json!("a, b, c"));
    assert_eq!(one("'<a & \"b\">'.escape('html')", &p), json!("&lt;a &amp; &quot;b&quot;&gt;"));
    assert_eq!(one("'ab'.encode('base64')", &p), json!("YWI="));
    assert_eq!(one("'abcd'.encode('urlbase64')", &p), json!("YWJjZA=="));
    assert_eq!(one("'ab'.encode('hex')", &p), json!("6162"));
    assert_eq!(one("'straße'.upper()", &p), json!("STRASSE"));
    assert_eq!(one("'abc'.toChars().count()", &p), json!(3));
    assert_eq!(one("'a,b,c'.split(',').count()", &p), json!(3));
    assert_eq!(one("'a' & 'b' + 'c'", &p), json!("abc"));
}

/// `replaceMatches()` builds its result itself, so it can be measured:
/// the replacement syntax must read as the regex crate reads it.
#[test]
fn replace_matches_reads_the_replacement_like_the_regex_crate() {
    let p = patient();
    let text = "John Smith, Jane Doe";
    let pattern = r"(?P<first>\w+) (?P<last>\w+)";
    let re = regex::Regex::new(pattern).unwrap();
    for replacement in [
        "$last $first",
        "${last}_${first}",
        "$2, $1",
        "${2}x${1}",
        "$1x",
        "$$1",
        "$",
        "a$",
        "${",
        "${last",
        "${}",
        "$9",
        "$nope",
        "$0$0",
        "$é",
        "100$ off",
        "$01",
        "",
    ] {
        let expected = re.replace_all(text, replacement).to_string();
        let expr = format!("'{}'.replaceMatches('{}', '{}')", text, pattern.replace('\\', "\\\\"), replacement);
        assert_eq!(one(&expr, &p), json!(expected), "{replacement:?}");
    }
    // Empty matches, at every position, as replace_all makes them.
    let expected = regex::Regex::new("x*").unwrap().replace_all("abc", "-").to_string();
    assert_eq!(one("'abc'.replaceMatches('x*', '-')", &p), json!(expected));
}

/// Evaluate with a small budget, so the tests reach it without building
/// half a gigabyte.
fn eval_with_budget(expr: &str, root: &Value, budget: usize) -> Result<Vec<Value>, String> {
    let parsed = parser::parse(expr)?;
    let mut env = eval::Env::within(root, root).with_budget(budget);
    eval::eval(&parsed, std::slice::from_ref(root), &mut env)
}

/// Values that multiply without any long string count against one budget
/// per evaluation.
#[test]
fn growing_collections_stop_at_the_evaluation_budget() {
    let p = patient();
    let ten = "(1|2|3|4|5|6|7|8|9|10)";
    let hundred_kb = chain("'a'", ".replace('a', 'aaaaaaaaaa')", 5);
    let mb = 1024 * 1024;
    for expr in [
        // 10^6 numbers.
        format!("{ten}{}", format!(".select({ten})").repeat(5)),
        // 10^4 copies of the resource.
        format!("{ten}.select({ten}.select({ten}.select({ten}.select(%resource))))"),
        chain("Patient.combine(%resource)", ".select($this.combine($this))", 12),
        chain("Patient.union(%resource.name)", &format!(".select({ten}.select(%resource))"), 3),
        format!("{hundred_kb}.toChars()"),
        format!("{hundred_kb}.split('a')"),
        format!("{ten}.select({ten}.select({ten}.select(trace('x', %resource))))"),
        format!("{ten}.aggregate($total.combine({ten}.select({ten}.select(%resource))), {{}})"),
        // Distinct strings, so repeat() keeps finding new ones.
        "'a'.repeat($this & 'aaaa')".to_string(),
    ] {
        let e = eval_with_budget(&expr, &p, mb).expect_err(&expr);
        assert!(e.contains("would build more than 1 MB"), "{expr}: {e}");
    }
    let e = eval_with_budget("Patient.descendants()", &p, 1024).unwrap_err();
    assert!(e.contains("would build more than"), "{e}");
    // What a real expression builds fits.
    let count = |expr: &str| eval_with_budget(expr, &p, mb).unwrap();
    assert_eq!(count(&format!("{ten}.select({ten}).count()")), vec![json!(100)]);
    assert_eq!(count("Patient.descendants().count()").len(), 1);
    assert_eq!(count("'a'.repeat(iif($this.length() < 50, $this & 'a', {})).count()"), vec![json!(49)]);
}

/// A Patient with ten names, for `%resource.name` wrapped in
/// `%resource.name.where(… .exists())`: ten times the work per level,
/// while building almost nothing.
fn ten_names() -> Value {
    let names: Vec<Value> = (0..10).map(|i| json!({"family": format!("F{i}")})).collect();
    json!({"resourceType": "Patient", "id": "p", "name": names})
}

fn nested_criteria(depth: usize) -> String {
    let mut e = "%resource.name".to_string();
    for _ in 0..depth {
        e = format!("%resource.name.where({e}.exists())");
    }
    format!("{e}.count()")
}

/// Evaluate under `steps` steps, returning what is left of them.
fn eval_with_steps(expr: &str, root: &Value, steps: u64) -> (FhirPathResult, u64) {
    let mut budget = Budget::with_limits(steps, std::time::Duration::from_secs(3600));
    let r = evaluate_sharing(expr, root, root, &mut budget);
    (r, budget.steps_left())
}

/// Work that builds nothing still counts. Nine levels of nested criteria
/// over ten names are 10^9 evaluations: hours, in a few MB, which the
/// memory budget never saw.
#[test]
fn nested_criteria_stop_at_the_step_budget() {
    let p = ten_names();
    // Three levels run to the end, and the steps say what they cost.
    let (r, left) = eval_with_steps(&nested_criteria(3), &p, 1_000_000);
    assert_eq!(r.results, vec![json!(10)], "{:?}", r.error);
    let three = 1_000_000 - left;
    assert!((10_000..100_000).contains(&three), "{three}");
    // Each level costs about ten times the one below.
    let (_, left) = eval_with_steps(&nested_criteria(4), &p, 10_000_000);
    let four = 10_000_000 - left;
    assert!(four > 8 * three && four < 12 * three, "{three} then {four}");

    // Nine levels stop at the budget, quickly, with an error that says so.
    let started = std::time::Instant::now();
    let (r, left) = eval_with_steps(&nested_criteria(9), &p, 1_000_000);
    let e = r.error.expect("refused");
    assert_eq!(e, "The expression takes too long to evaluate (more than 1 million steps); narrow it down");
    assert_eq!(left, 0);
    assert!(started.elapsed() < std::time::Duration::from_secs(10), "{:?}", started.elapsed());

    // The default budget is the one the panel and the rules run under.
    assert_eq!(Budget::new().steps_left(), MAX_EVAL_STEPS);
    assert!(evaluate(&nested_criteria(2), &p).error.is_none());
}

/// Comparisons are steps too: `distinct()`, `|`, `subsetOf()` and the like
/// compare each item with the ones before it, the square of the length.
#[test]
fn comparisons_count_against_the_step_budget() {
    let p = patient();
    let numbers: Vec<String> = (0..300).map(|i| i.to_string()).collect();
    let list = format!("({})", numbers.join(" | "));
    // 300 distinct numbers: about 45 000 comparisons to build the list.
    let (r, left) = eval_with_steps(&format!("{list}.count()"), &p, 1_000_000);
    assert_eq!(r.results, vec![json!(300)]);
    assert!(1_000_000 - left > 40_000, "{}", 1_000_000 - left);
    // Past the list itself, each of these compares as much again.
    let built = 1_000_000 - left;
    for expr in [
        format!("{list}.combine({list}).distinct().count()"),
        format!("{list}.combine({list}).isDistinct()"),
        format!("{list}.subsetOf({list}.combine({list}))"),
        format!("{list}.intersect({list}).count()"),
        format!("{list}.exclude({list}).count()"),
        format!("{list}.union({list}).count()"),
        format!("{list}.combine({list}) ~ {list}.combine({list})"),
    ] {
        let (r, _) = eval_with_steps(&expr, &p, 3 * built);
        let e = r.error.unwrap_or_else(|| panic!("{expr}: ran to the end"));
        assert!(e.contains("takes too long"), "{expr}: {e}");
    }
    // A sort counts its n·log2(n) comparisons.
    let (r, left) = eval_with_steps(&format!("{list}.sort().count()"), &p, 1_000_000);
    assert_eq!(r.results, vec![json!(300)]);
    assert!(1_000_000 - left >= built + 300 * 9, "{}", 1_000_000 - left);
}

/// A copy of the whole resource counts by its size: copying a large
/// resource for every item of a criteria is far more than one step.
/// Reading a path from `%resource` copies nothing.
#[test]
fn copies_of_the_resource_count_by_their_size() {
    let notes: Vec<Value> = (0..1000).map(|i| json!({"text": format!("{i:0>200}")})).collect();
    let big = json!({"resourceType": "Patient", "id": "p", "name": [{"family": "F"}], "note": notes});
    let small = ten_names();
    let cost = |expr: &str, root: &Value| {
        let (r, left) = eval_with_steps(expr, root, 100_000_000);
        assert!(r.error.is_none(), "{expr}: {:?}", r.error);
        100_000_000 - left
    };
    let ten = "(1|2|3|4|5|6|7|8|9|10)";
    // About 300 KB, so about 1000 steps a copy: ten copies here.
    let copying = format!("{ten}.where(%resource.exists()).count()");
    let (on_big, on_small) = (cost(&copying, &big), cost(&copying, &small));
    assert!(on_big > 10_000 && on_small < 500, "{on_big} vs {on_small}");
    // `$this` outside a criteria, and a function's argument, are copies too.
    assert!(cost("$this.exists()", &big) > 500, "{}", cost("$this.exists()", &big));
    assert!(cost("name.combine(name).count()", &big) > 500);
    // A path read from the resource does not copy it.
    let reading = format!("{ten}.where(%resource.id.exists()).count()");
    assert_eq!(cost(&reading, &big), cost(&reading, &ten_names()));
}

/// The clock stops what the steps do not: a few steps can each copy a
/// resource of many MB. A budget whose time is already up stops at once.
#[test]
fn the_time_limit_stops_an_evaluation_too() {
    let p = ten_names();
    let mut budget = Budget::with_limits(u64::MAX, std::time::Duration::ZERO);
    let r = evaluate_sharing(&nested_criteria(3), &p, &p, &mut budget);
    assert_eq!(
        r.error.as_deref(),
        Some("The expression takes too long to evaluate (more than 0 seconds); narrow it down")
    );
    assert!(budget.exhausted());
    assert_eq!(MAX_EVAL_TIME, std::time::Duration::from_secs(60));
}

/// Evaluations drawing on one budget share it: each takes what it used,
/// and once it is spent the next one stops at once.
#[test]
fn evaluations_can_share_one_budget() {
    let p = ten_names();
    let hour = std::time::Duration::from_secs(3600);
    let mut shared = Budget::with_limits(100_000, hour);
    let r = evaluate_sharing(&nested_criteria(3), &p, &p, &mut shared);
    assert!(r.error.is_none());
    let after_one = shared.steps_left();
    assert!(after_one < 100_000 && after_one > 0);
    // Each evaluation also keeps to its own limit within the shared one.
    let mut capped = Budget::with_limits(1_000_000, hour).each_at_most(1000, hour);
    let r = evaluate_sharing(&nested_criteria(3), &p, &p, &mut capped);
    assert!(r.error.unwrap().contains("more than 1000 steps"));
    assert!(!capped.exhausted());
    assert_eq!(capped.steps_left(), 999_000);
    // Spent, the shared one refuses every evaluation after.
    while !shared.exhausted() {
        evaluate_sharing(&nested_criteria(3), &p, &p, &mut shared);
    }
    assert!(evaluate_sharing("1 + 1", &p, &p, &mut shared).error.unwrap().contains("takes too long"));
    // A validation's budget is the evaluations' worth it is said to be.
    let validation = Budget::for_validation();
    assert_eq!(validation.steps_left(), MAX_EVAL_STEPS * u64::from(VALIDATION_EVALS));
    assert_eq!(validation.for_one_evaluation().steps_left(), MAX_EVAL_STEPS);
}
