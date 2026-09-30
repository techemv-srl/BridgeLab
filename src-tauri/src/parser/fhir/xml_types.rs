//! Give FHIR XML the JSON types it would have had.
//!
//! In XML every primitive is a `value="..."` string and nothing says whether
//! an element repeats, so the plain conversion in [`super::xml`] yields
//! `"active": "true"`, `"value": "6.3"` and `"profile": "http://..."` where
//! the JSON encoding has `true`, `6.3` and `["http://..."]`. Validators and
//! FHIRPath then see a different resource from the same data in JSON.
//!
//! This pass walks the converted resource with the R4 core definitions the
//! binary carries and fixes both: booleans, integers and decimals become
//! JSON booleans and numbers, every element whose maximum cardinality is
//! above one becomes an array, and a single-valued element the converter's
//! name list made an array (`Organization.name`) goes back to one value.

use std::sync::OnceLock;

use serde_json::{Map, Number, Value};

use super::profile::model::{Profile, ProfileElement, ProfileIndex};
use super::profile::package;

/// The built-in core definitions, decoded once.
fn core() -> Option<&'static ProfileIndex> {
    static CORE: OnceLock<Option<ProfileIndex>> = OnceLock::new();
    CORE.get_or_init(|| {
        package::builtin().map(|p| {
            let mut index = ProfileIndex::default();
            index.add_builtin(p);
            index
        })
    })
    .as_ref()
}

/// Type a resource converted from XML in place. Elements the core
/// definitions do not know (a typo, an unknown resource) are left as they
/// are, for the validator to report.
pub fn apply(resource: &mut Value) {
    if let Some(index) = core() {
        type_resource(resource, index);
    }
}

fn type_resource(value: &mut Value, index: &ProfileIndex) {
    let Some(obj) = value.as_object_mut() else { return };
    let Some(rt) = obj.get("resourceType").and_then(|v| v.as_str()).map(str::to_string) else { return };
    if let Some(profile) = index.base_for_type(&rt) {
        walk(obj, profile, &rt, index);
    }
}

/// Type the children of the object at `path` in `profile`.
fn walk(obj: &mut Map<String, Value>, profile: &Profile, path: &str, index: &ProfileIndex) {
    for (key, value) in obj.iter_mut() {
        if key == "resourceType" {
            continue;
        }
        // `_name` carries a primitive's id and extensions: an Element, with
        // the cardinality of `name`.
        let (name, is_extension_part) = match key.strip_prefix('_') {
            Some(n) => (n, true),
            None => (key.as_str(), false),
        };
        let Some((element, type_code)) = find(profile, path, name) else { continue };

        let repeats = element.max != Some(1) && element.max != Some(0);
        if repeats && !value.is_array() {
            let single = value.take();
            *value = Value::Array(vec![single]);
        } else if !repeats && value.as_array().is_some_and(|a| a.len() == 1) {
            // The converter lists some names as always-repeating ("name",
            // "profile"…); where this resource defines them 0..1, unwrap.
            let single = value.as_array_mut().and_then(|a| a.pop()).unwrap_or(Value::Null);
            *value = single;
        }
        let items: Vec<&mut Value> = match value {
            Value::Array(a) => a.iter_mut().collect(),
            v => vec![v],
        };
        for item in items {
            if is_extension_part {
                if let (Some(o), Some(el)) = (item.as_object_mut(), index.base_for_type("Element")) {
                    walk(o, el, "Element", index);
                }
            } else {
                type_value(item, &type_code, element, profile, index);
            }
        }
    }
}

/// The element `name` under `path`, with the type code it takes: the
/// element's own type, or for a choice (`value[x]`) the type its key names.
fn find<'a>(profile: &'a Profile, path: &str, name: &str) -> Option<(&'a ProfileElement, String)> {
    let direct = format!("{}.{}", path, name);
    if let Some(e) = profile.elements.iter().find(|e| e.path == direct && e.slice_name.is_none()) {
        return Some((e, e.types.first().cloned().unwrap_or_default()));
    }
    profile
        .children_of(path)
        .into_iter()
        .filter(|e| e.is_choice() && e.slice_name.is_none() && name.starts_with(e.base_name()))
        .find_map(|e| e.types.iter().find(|t| e.choice_key(t) == name).map(|t| (e, t.clone())))
}

fn type_value(value: &mut Value, type_code: &str, element: &ProfileElement, profile: &Profile, index: &ProfileIndex) {
    match type_code {
        "boolean" => {
            if let Some(b) = value.as_str().and_then(|s| match s {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            }) {
                *value = Value::Bool(b);
            }
        }
        "integer" | "positiveInt" | "unsignedInt" | "integer64" => {
            if let Some(n) = value.as_str().filter(|s| is_integer(s)).and_then(|s| s.parse::<i64>().ok()) {
                *value = Value::Number(n.into());
            }
        }
        "decimal" => {
            if let Some(n) = value.as_str().filter(|s| is_decimal(s)).and_then(|s| s.parse::<Number>().ok()) {
                *value = Value::Number(n);
            }
        }
        "Resource" => type_resource(value, index),
        "BackboneElement" | "Element" => {
            // Inline structure: its children are defined under this path.
            if let Some(o) = value.as_object_mut() {
                walk(o, profile, &element.path, index);
            }
        }
        t if t.starts_with(|c: char| c.is_ascii_uppercase()) && !t.starts_with("System.") => {
            if let (Some(o), Some(tp)) = (value.as_object_mut(), index.base_for_type(t)) {
                walk(o, tp, t, index);
            }
        }
        _ => {}
    }
}

/// FHIR integer lexical form: an optional minus, no leading zeros.
fn is_integer(s: &str) -> bool {
    let d = s.strip_prefix('-').unwrap_or(s);
    !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()) && (d == "0" || !d.starts_with('0'))
}

/// FHIR decimal lexical form, which is also valid JSON number syntax.
fn is_decimal(s: &str) -> bool {
    let d = s.strip_prefix('-').unwrap_or(s);
    let (mantissa, exp) = match d.find(['e', 'E']) {
        Some(i) => (&d[..i], Some(&d[i + 1..])),
        None => (d, None),
    };
    let (int, frac) = mantissa.split_once('.').map_or((mantissa, None), |(i, f)| (i, Some(f)));
    let digits = |x: &str| !x.is_empty() && x.bytes().all(|b| b.is_ascii_digit());
    digits(int)
        && (int == "0" || !int.starts_with('0'))
        && frac.map_or(true, digits)
        && exp.map_or(true, |e| digits(e.strip_prefix(['+', '-']).unwrap_or(e)))
}

#[cfg(test)]
mod tests {
    use super::super::xml::fhir_xml_to_json;
    use super::*;

    fn typed(xml: &str) -> Value {
        let (_, mut json) = fhir_xml_to_json(xml).unwrap();
        apply(&mut json);
        json
    }

    #[test]
    fn primitives_get_their_json_types() {
        let p = typed(r#"<Patient xmlns="http://hl7.org/fhir"><id value="p1"/><active value="true"/><multipleBirthInteger value="2"/><name><family value="Rossi"/></name></Patient>"#);
        assert_eq!(p["active"], Value::Bool(true));
        assert_eq!(p["multipleBirthInteger"], Value::from(2));
        assert_eq!(p["id"], "p1");
        assert_eq!(p["name"][0]["family"], "Rossi");
    }

    #[test]
    fn quantities_nested_types_and_choice_elements() {
        let o = typed(r#"<Observation xmlns="http://hl7.org/fhir"><status value="final"/><code><text value="x"/></code>
            <valueQuantity><value value="6.30"/><unit value="mmol/L"/></valueQuantity>
            <component><code><text value="y"/></code><valueBoolean value="false"/></component></Observation>"#);
        assert_eq!(o["valueQuantity"]["value"].as_f64(), Some(6.3));
        assert_eq!(o["component"][0]["valueBoolean"], Value::Bool(false));
        assert_eq!(o["status"], "final");
    }

    #[test]
    fn repeating_elements_are_arrays_even_alone() {
        let p = typed(r#"<Patient xmlns="http://hl7.org/fhir"><meta><profile value="http://example.org/StructureDefinition/StrictPatient"/></meta><generalPractitioner><reference value="Practitioner/1"/></generalPractitioner></Patient>"#);
        assert_eq!(p["meta"]["profile"], serde_json::json!(["http://example.org/StructureDefinition/StrictPatient"]));
        assert!(p["generalPractitioner"].is_array());
    }

    #[test]
    fn single_valued_elements_stay_single() {
        let org = typed(r#"<Organization xmlns="http://hl7.org/fhir"><name value="Ospedale"/><alias value="OSP"/></Organization>"#);
        assert_eq!(org["name"], "Ospedale", "Organization.name is 0..1");
        assert_eq!(org["alias"], serde_json::json!(["OSP"]));
        let ad = typed(r#"<ActivityDefinition xmlns="http://hl7.org/fhir"><status value="draft"/><profile value="http://example.org/sd"/></ActivityDefinition>"#);
        assert_eq!(ad["profile"], "http://example.org/sd", "ActivityDefinition.profile is 0..1");
        let pat = typed(r#"<Patient xmlns="http://hl7.org/fhir"><name><family value="Rossi"/></name></Patient>"#);
        assert!(pat["name"].is_array(), "Patient.name is 0..*");
    }

    #[test]
    fn bundle_entries_and_contained_resources_are_typed_too() {
        let b = typed(r#"<Bundle xmlns="http://hl7.org/fhir"><type value="collection"/><total value="1"/>
            <entry><resource><Patient><active value="false"/></Patient></resource></entry></Bundle>"#);
        assert_eq!(b["total"], Value::from(1));
        assert_eq!(b["entry"][0]["resource"]["active"], Value::Bool(false));
    }

    #[test]
    fn invalid_lexical_forms_stay_strings_for_the_validator() {
        let p = typed(r#"<Patient xmlns="http://hl7.org/fhir"><active value="yes"/><multipleBirthInteger value="02"/></Patient>"#);
        assert_eq!(p["active"], "yes");
        assert_eq!(p["multipleBirthInteger"], "02");
    }

    #[test]
    fn primitive_extensions_keep_their_shape() {
        let p = typed(r#"<Patient xmlns="http://hl7.org/fhir"><birthDate value="1980-01-01"><extension url="http://x"><valueBoolean value="true"/></extension></birthDate></Patient>"#);
        assert_eq!(p["birthDate"], "1980-01-01");
        assert_eq!(p["_birthDate"]["extension"][0]["valueBoolean"], Value::Bool(true));
    }

    #[test]
    fn lexical_helpers() {
        for ok in ["0", "12", "-4"] { assert!(is_integer(ok), "{ok}"); }
        for bad in ["01", "+1", "1.0", "", "-"] { assert!(!is_integer(bad), "{bad}"); }
        for ok in ["0", "6.30", "-0.5", "1e3", "1.2E-4"] { assert!(is_decimal(ok), "{ok}"); }
        for bad in ["06", ".5", "5.", "abc", "1e", "+1"] { assert!(!is_decimal(bad), "{bad}"); }
    }
}
