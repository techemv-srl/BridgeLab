//! FHIR XML → JSON conversion, good enough to run the same validation
//! rules on XML resources that already run on JSON ones.
//!
//! The FHIR XML encoding maps 1:1 onto the JSON model:
//! * element name → object key
//! * `value="..."` attribute → JSON primitive (a string here; typed by
//!   [`super::xml_types`] from the core definitions)
//! * repeated sibling elements → JSON array
//! * nested elements → JSON object
//! * the root element name → `resourceType`
//! * `<div>` (XHTML narrative) subtrees are collapsed to a placeholder —
//!   their internals are presentation, not data.

use quick_xml::events::Event;
use serde_json::{Map, Value};

/// FHIR properties that are arrays in JSON even with a single occurrence.
/// A generic converter cannot know cardinality without the full FHIR
/// schema; this curated set covers the properties our consumers rely on
/// (Bundle analysis, FHIRPath indexing, validation).
const ALWAYS_ARRAY: &[&str] = &[
    "entry", "name", "given", "prefix", "suffix", "identifier", "telecom",
    "address", "line", "extension", "modifierExtension", "coding", "contact",
    "communication", "link", "item", "component", "performer", "category",
    "note", "contained", "author", "result", "diagnosis", "participant",
    "reaction", "dosage", "instantiates", "basedOn", "partOf",
];

/// Insert honoring FHIR's repeated-element = array rule. Known repeating
/// properties become arrays on the FIRST occurrence: a Bundle with one
/// `<entry>` must still produce `entry: [...]` or analyze_bundle sees no
/// entries and `Patient.name[0]` stops matching for XML resources.
fn insert_multi(map: &mut Map<String, Value>, key: String, val: Value) {
    match map.get_mut(&key) {
        Some(Value::Array(arr)) => arr.push(val),
        Some(existing) => {
            let prev = existing.take();
            *existing = Value::Array(vec![prev, val]);
        }
        None => {
            if ALWAYS_ARRAY.contains(&key.as_str()) {
                map.insert(key, Value::Array(vec![val]));
            } else {
                map.insert(key, val);
            }
        }
    }
}

struct Frame {
    name: String,
    children: Map<String, Value>,
    value_attr: Option<String>,
}

fn frame_from_start(
    e: &quick_xml::events::BytesStart<'_>,
) -> Result<Frame, String> {
    let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
    let mut children = Map::new();
    let mut value_attr = None;
    for attr in e.attributes() {
        let attr = attr.map_err(|err| format!("Malformed XML attribute: {}", err))?;
        let key = String::from_utf8_lossy(attr.key.local_name().as_ref()).into_owned();
        if key.starts_with("xmlns") {
            continue;
        }
        // Entity references resolved and whitespace normalised as the XML
        // specification requires of attribute values.
        let val = attr
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .map_err(|err| format!("Malformed XML attribute value: {}", err))?
            .into_owned();
        if key == "value" {
            value_attr = Some(val);
        } else {
            children.insert(key, Value::String(val));
        }
    }
    Ok(Frame { name, children, value_attr })
}

/// Emit a completed frame into its parent (or as the document root).
/// Handles three FHIR-specific encodings beyond plain nesting:
/// * `<resource>`/`<contained>` wrap a full resource whose element name is
///   the resource type — promote it to a `resourceType` field, or Bundle
///   entries all analyze as "Unknown".
/// * A primitive with BOTH a value attribute and child elements (primitive
///   extension) splits into `key: value` + `_key: {extensions}` per the
///   canonical JSON mapping — otherwise `gender` becomes an object and the
///   value-set check silently skips it.
fn emit(frame: Frame, stack: &mut Vec<Frame>, root: &mut Option<(String, Value)>) {
    let name = frame.name.clone();

    // Primitive extension split: value + children → two inserts.
    if frame.value_attr.is_some() && !frame.children.is_empty() {
        if let Some(parent) = stack.last_mut() {
            insert_multi(&mut parent.children, name.clone(), Value::String(frame.value_attr.unwrap()));
            insert_multi(&mut parent.children, format!("_{}", name), Value::Object(frame.children));
            return;
        }
    }

    let mut value = match (frame.children.is_empty(), frame.value_attr) {
        (true, Some(v)) => Value::String(v),
        (false, Some(v)) => {
            // unreachable with a parent (handled above); root fallback
            let mut m = frame.children;
            m.insert("value".into(), Value::String(v));
            Value::Object(m)
        }
        (true, None) => Value::Object(Map::new()),
        (false, None) => Value::Object(frame.children),
    };

    // Resource container promotion: {resource: {Patient: {...}}} →
    // {resource: {resourceType: "Patient", ...}}.
    if name == "resource" || name == "contained" {
        if let Value::Object(m) = &value {
            if m.len() == 1 {
                let (inner_name, inner_val) = m.iter().next().unwrap();
                if inner_name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                    if let Value::Object(inner_map) = inner_val {
                        let mut promoted = inner_map.clone();
                        promoted.insert("resourceType".into(), Value::String(inner_name.clone()));
                        value = Value::Object(promoted);
                    }
                }
            }
        }
    }

    if let Some(parent) = stack.last_mut() {
        insert_multi(&mut parent.children, name, value);
    } else {
        *root = Some((name, value));
    }
}

/// Deepest element nesting accepted: the recursion limit serde_json applies
/// to FHIR JSON. Real resources stay far below it; a hostile document
/// nested thousands deep used to overflow the stack of the code that walks
/// the result and take the whole app down.
const MAX_XML_DEPTH: usize = 128;

/// The FHIR namespace every element of a FHIR XML resource is in.
pub const FHIR_NS: &[u8] = b"http://hl7.org/fhir";

/// The namespace of the narrative, the `<div>` of a `text` element.
pub const XHTML_NS: &[u8] = b"http://www.w3.org/1999/xhtml";

/// The first element found outside the namespace it belongs in.
#[derive(Debug, Clone, PartialEq)]
pub struct OutsideFhir {
    /// Its name as written (`Patient`, `f:name`).
    pub element: String,
    /// The namespace it is in; `None` for no namespace at all.
    pub namespace: Option<String>,
    /// Where it is, as a validation path (empty for the root).
    pub path: String,
    /// It is the narrative `<div>`, which belongs in XHTML's namespace.
    pub narrative: bool,
}

/// Convert a FHIR XML document into (resource_type, JSON value).
pub fn fhir_xml_to_json(xml: &str) -> Result<(String, Value), String> {
    fhir_xml_to_json_checked(xml).map(|(rt, json, _)| (rt, json))
}

/// [`fhir_xml_to_json`], also giving the first element that is not in the
/// FHIR namespace (the XHTML narrative excepted), if any. The conversion
/// itself goes by local names, so without this a `<Patient>` with no
/// namespace, or in another one, read as a FHIR Patient and validated
/// clean.
pub fn fhir_xml_to_json_checked(xml: &str) -> Result<(String, Value, Option<OutsideFhir>), String> {
    let mut reader = quick_xml::NsReader::from_str(xml);
    let mut stack: Vec<Frame> = Vec::new();
    let mut root: Option<(String, Value)> = None;
    let mut outside: Option<OutsideFhir> = None;

    loop {
        use quick_xml::name::ResolveResult;
        let (ns, event) = reader.read_resolved_event().map_err(|e| format!("Invalid XML: {}", e))?;
        // The element's namespace (`None` for none at all) and whether it
        // is FHIR's or XHTML's. Owned, so the reader can move on.
        let (namespace, in_fhir, in_xhtml): (Option<String>, bool, bool) = match ns {
            ResolveResult::Bound(n) => {
                (Some(String::from_utf8_lossy(n.as_ref()).into_owned()), n.as_ref() == FHIR_NS, n.as_ref() == XHTML_NS)
            }
            ResolveResult::Unbound => (None, false, false),
            ResolveResult::Unknown(prefix) => {
                (Some(format!("of the undeclared prefix '{}'", String::from_utf8_lossy(&prefix))), false, false)
            }
        };
        if let Event::Start(e) | Event::Empty(e) = &event {
            // The narrative is the one place for another namespace: the
            // <div> of a `text` element, in XHTML's (its subtree is skipped
            // below). Any other <div> is held to FHIR's like the rest, and
            // a narrative <div> in no namespace, or in another one, is not
            // a narrative.
            let narrative = e.local_name().as_ref() == b"div" && stack.last().is_some_and(|f| f.name == "text");
            let misplaced = if narrative { !in_xhtml } else { !in_fhir };
            if misplaced && outside.is_none() {
                let local = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                let mut path: Vec<&str> = stack.iter().skip(1).map(|f| f.name.as_str()).collect();
                if !stack.is_empty() {
                    path.push(&local);
                }
                outside = Some(OutsideFhir {
                    element: String::from_utf8_lossy(e.name().as_ref()).into_owned(),
                    namespace,
                    path: path.join("."),
                    narrative,
                });
            }
        }
        match event {
            Event::Eof => break,
            Event::Start(e) => {
                let frame = frame_from_start(&e)?;
                if frame.name == "div" {
                    // XHTML narrative: skip the subtree, keep a placeholder.
                    reader
                        .read_to_end(e.name())
                        .map_err(|err| format!("Unclosed narrative div: {}", err))?;
                    if let Some(parent) = stack.last_mut() {
                        insert_multi(
                            &mut parent.children,
                            "div".into(),
                            Value::String("[xhtml narrative]".into()),
                        );
                    }
                    continue;
                }
                if stack.len() >= MAX_XML_DEPTH {
                    return Err(format!(
                        "Invalid FHIR XML: elements are nested more than {} deep",
                        MAX_XML_DEPTH
                    ));
                }
                stack.push(frame);
            }
            Event::Empty(e) => {
                // A self-closing element with no parent is a valid (minimal)
                // document root: <Patient xmlns="..."/> must still validate.
                let frame = frame_from_start(&e)?;
                emit(frame, &mut stack, &mut root);
            }
            Event::End(_) => {
                let frame = match stack.pop() {
                    Some(f) => f,
                    None => return Err("Unbalanced XML end tag".into()),
                };
                emit(frame, &mut stack, &mut root);
            }
            // FHIR data lives in value attributes; free text only occurs
            // inside <div>, which is skipped above.
            _ => {}
        }
    }

    let (resource_type, value) = root.ok_or_else(|| "Empty XML document".to_string())?;
    let mut obj = match value {
        Value::Object(m) => m,
        other => {
            let mut m = Map::new();
            m.insert("value".into(), other);
            m
        }
    };
    obj.insert("resourceType".into(), Value::String(resource_type.clone()));
    Ok((resource_type, Value::Object(obj), outside))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATIENT_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Patient xmlns="http://hl7.org/fhir">
  <id value="pat-1"/>
  <text><status value="generated"/><div xmlns="http://www.w3.org/1999/xhtml"><p>Mario Rossi</p></div></text>
  <identifier>
    <system value="urn:oid:2.16.840.1.113883.2.9"/>
    <value value="RSSMRA80A01F205X"/>
  </identifier>
  <name>
    <family value="Rossi"/>
    <given value="Mario"/>
    <given value="Giuseppe"/>
  </name>
  <gender value="male"/>
  <birthDate value="1980-01-01"/>
</Patient>"#;

    #[test]
    fn test_patient_conversion() {
        let (rt, json) = fhir_xml_to_json(PATIENT_XML).unwrap();
        assert_eq!(rt, "Patient");
        assert_eq!(json["resourceType"], "Patient");
        assert_eq!(json["id"], "pat-1");
        assert_eq!(json["gender"], "male");
        assert_eq!(json["birthDate"], "1980-01-01");
        // Known repeating properties are arrays even with one occurrence
        assert!(json["name"].is_array(), "name must be an array");
        assert_eq!(json["name"][0]["given"][0], "Mario");
        assert_eq!(json["name"][0]["given"][1], "Giuseppe");
        assert!(json["identifier"].is_array(), "identifier must be an array");
        assert_eq!(json["identifier"][0]["value"], "RSSMRA80A01F205X");
        // narrative collapsed, not exploded into xhtml structure
        assert_eq!(json["text"]["div"], "[xhtml narrative]");
    }

    /// A Bundle with ONE entry must still produce entry: [...] and the
    /// <resource><Patient>…</Patient></resource> container must promote the
    /// inner element to resourceType — or analyze_bundle sees zero entries
    /// of type "Unknown".
    #[test]
    fn test_single_entry_bundle() {
        let xml = r#"<Bundle xmlns="http://hl7.org/fhir">
  <type value="collection"/>
  <entry>
    <resource>
      <Patient>
        <id value="p1"/>
        <gender value="female"/>
      </Patient>
    </resource>
  </entry>
</Bundle>"#;
        let (rt, json) = fhir_xml_to_json(xml).unwrap();
        assert_eq!(rt, "Bundle");
        assert!(json["entry"].is_array(), "single entry must still be an array");
        let resource = &json["entry"][0]["resource"];
        assert_eq!(resource["resourceType"], "Patient", "container must promote resourceType");
        assert_eq!(resource["id"], "p1");
        assert_eq!(resource["gender"], "female");
    }

    /// Primitive with value + extension splits into key + _key per the
    /// canonical JSON mapping — the value-set check must still see the
    /// plain string.
    #[test]
    fn test_primitive_extension_split() {
        let xml = r#"<Patient xmlns="http://hl7.org/fhir">
  <gender value="banana"><extension url="http://x"><valueString value="y"/></extension></gender>
</Patient>"#;
        let (_, json) = fhir_xml_to_json(xml).unwrap();
        assert_eq!(json["gender"], "banana", "primitive must stay a string");
        assert!(json["_gender"]["extension"].is_array(), "extensions land under _gender");

        use crate::parser::fhir::{parse_fhir_xml, validate_fhir_json};
        let resource = parse_fhir_xml(xml).unwrap();
        let issues = validate_fhir_json(&resource);
        assert!(
            issues.iter().any(|i| i.path == "gender"),
            "invalid gender with extension must still be flagged"
        );
    }

    /// Elements outside the FHIR namespace are found: a root with no xmlns
    /// or another one, a nested element in a foreign namespace. The FHIR
    /// namespace by prefix, and the XHTML narrative, are fine.
    #[test]
    fn elements_outside_the_fhir_namespace_are_found() {
        let outside = |xml: &str| fhir_xml_to_json_checked(xml).unwrap().2;
        assert_eq!(outside(PATIENT_XML), None);
        assert_eq!(outside(r#"<f:Patient xmlns:f="http://hl7.org/fhir"><f:id value="1"/></f:Patient>"#), None);

        let none = outside(r#"<Patient><id value="1"/><gender value="male"/></Patient>"#).unwrap();
        assert_eq!((none.element.as_str(), none.namespace.clone(), none.path.as_str()), ("Patient", None, ""));

        let wrong = outside(r#"<Patient xmlns="http://example.org/fhir"><id value="1"/></Patient>"#).unwrap();
        assert_eq!(wrong.namespace.as_deref(), Some("http://example.org/fhir"));

        let nested = outside(
            r#"<Patient xmlns="http://hl7.org/fhir" xmlns:x="urn:x"><name><x:family value="R"/></name></Patient>"#,
        )
        .unwrap();
        assert_eq!(nested.element, "x:family");
        assert_eq!(nested.path, "name.family");
        assert_eq!(nested.namespace.as_deref(), Some("urn:x"));
    }

    #[test]
    fn validation_reports_a_resource_outside_the_fhir_namespace() {
        use crate::parser::fhir::{parse_fhir_xml, validate_fhir_json};
        let issues = |xml: &str| validate_fhir_json(&parse_fhir_xml(xml).unwrap());
        let plain = issues(r#"<Patient><id value="1"/><gender value="male"/></Patient>"#);
        let first = &plain[0];
        assert_eq!(first.severity, "error");
        assert!(first.message.contains("no namespace"), "{}", first.message);
        let wrong = issues(r#"<Patient xmlns="http://example.org"><id value="1"/></Patient>"#);
        assert!(wrong[0].message.contains("http://example.org"), "{}", wrong[0].message);
        assert!(!issues(PATIENT_XML).iter().any(|i| i.message.contains("namespace")));
    }

    /// Only the narrative <div> of a `text`, in the XHTML namespace, may
    /// leave FHIR's: one in no namespace, in another or in FHIR's is not a
    /// narrative, and a <div> anywhere else is held to FHIR's namespace.
    #[test]
    fn only_an_xhtml_narrative_div_is_exempt() {
        let outside = |xml: &str| fhir_xml_to_json_checked(xml).unwrap().2;
        let patient = |text: &str| format!(r#"<Patient xmlns="http://hl7.org/fhir"><id value="1"/>{text}</Patient>"#);
        // The narrative as FHIR writes it, by a declaration on the div, by a
        // prefix, self-closing, and inside a contained resource.
        for ok in [
            r#"<text><status value="generated"/><div xmlns="http://www.w3.org/1999/xhtml"><p>Hi</p></div></text>"#,
            r#"<text><status value="generated"/><h:div xmlns:h="http://www.w3.org/1999/xhtml"><h:p>Hi</h:p></h:div></text>"#,
            r#"<text><status value="empty"/><div xmlns="http://www.w3.org/1999/xhtml"/></text>"#,
            r#"<contained><Patient><text><div xmlns="http://www.w3.org/1999/xhtml">x</div></text></Patient></contained>"#,
        ] {
            assert_eq!(outside(&patient(ok)), None, "{ok}");
        }
        let bad = |text: &str| outside(&patient(text)).unwrap_or_else(|| panic!("not reported: {text}"));
        let foreign = bad(r#"<text><div xmlns="urn:not-xhtml"><p>x</p></div></text>"#);
        assert_eq!((foreign.narrative, foreign.namespace.as_deref(), foreign.path.as_str()), (true, Some("urn:not-xhtml"), "text.div"));
        let none = bad(r#"<text><div xmlns=""><p>x</p></div></text>"#);
        assert_eq!((none.narrative, none.namespace.clone()), (true, None));
        // Inheriting FHIR's namespace is not XHTML either.
        assert_eq!(bad(r#"<text><div><p>x</p></div></text>"#).namespace.as_deref(), Some("http://hl7.org/fhir"));
        // A <div> that is no narrative, in XHTML's namespace: outside FHIR's.
        let elsewhere = bad(r#"<div xmlns="http://www.w3.org/1999/xhtml">x</div>"#);
        assert_eq!((elsewhere.narrative, elsewhere.path.as_str()), (false, "div"));

        use crate::parser::fhir::{parse_fhir_xml, validate_fhir_json};
        let issues = validate_fhir_json(&parse_fhir_xml(&patient(r#"<text><div xmlns="urn:not-xhtml">x</div></text>"#)).unwrap());
        assert!(issues[0].message.contains("narrative") && issues[0].message.contains("urn:not-xhtml"), "{:?}", issues[0]);
        assert_eq!(issues[0].severity, "error");
    }

    /// A self-closing root is a valid minimal document.
    #[test]
    fn test_self_closing_root() {
        let (rt, json) = fhir_xml_to_json(r#"<Patient xmlns="http://hl7.org/fhir"/>"#).unwrap();
        assert_eq!(rt, "Patient");
        assert_eq!(json["resourceType"], "Patient");
    }

    #[test]
    fn test_invalid_xml() {
        assert!(fhir_xml_to_json("<Patient><id value=\"x\"/>").is_err());
        assert!(fhir_xml_to_json("").is_err());
    }

    /// Entity references in attribute values are resolved (quick-xml 0.41
    /// moved this from `unescape_value` to `normalized_value`).
    #[test]
    fn attribute_entities_are_resolved() {
        let xml = r#"<Organization xmlns="http://hl7.org/fhir"><name value="Smith &amp; Sons &lt;Ltd&gt;"/></Organization>"#;
        let (_, json) = fhir_xml_to_json(xml).unwrap();
        assert_eq!(json["name"][0], "Smith & Sons <Ltd>");
    }

    #[test]
    fn test_validation_pipeline_on_xml() {
        use crate::parser::fhir::{parse_fhir_xml, validate_fhir_json};
        // Same rules as the JSON path: a Patient without a name is flagged,
        // and an invalid gender value is flagged.
        let xml = r#"<Patient xmlns="http://hl7.org/fhir"><id value="p1"/><gender value="banana"/></Patient>"#;
        let resource = parse_fhir_xml(xml).unwrap();
        assert!(resource.json_value.is_some(), "XML parse must now hydrate json_value");
        let issues = validate_fhir_json(&resource);
        let paths: Vec<_> = issues.iter().map(|i| i.path.as_str()).collect();
        assert!(paths.contains(&"name"), "expected missing-name issue, got: {:?}", paths);
        assert!(paths.contains(&"gender"), "expected invalid-gender issue, got: {:?}", paths);
    }

    #[test]
    fn absurdly_deep_documents_are_refused_not_crashed_on() {
        let n = 20_000;
        let xml = format!(
            "<Patient xmlns=\"http://hl7.org/fhir\"><id value=\"1\"/>{}{}</Patient>",
            "<extension url=\"x\">".repeat(n),
            "</extension>".repeat(n)
        );
        let err = fhir_xml_to_json(&xml).unwrap_err();
        assert!(err.contains("nested"), "{err}");
        let ok = format!(
            "<Patient xmlns=\"http://hl7.org/fhir\">{}{}</Patient>",
            "<extension url=\"x\">".repeat(40),
            "</extension>".repeat(40)
        );
        assert!(fhir_xml_to_json(&ok).is_ok());
    }
}
