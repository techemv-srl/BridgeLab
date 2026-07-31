//! FHIR XML → JSON conversion, good enough to run the same validation
//! rules on XML resources that already run on JSON ones.
//!
//! The FHIR XML encoding maps 1:1 onto the JSON model:
//! * element name → object key
//! * `value="..."` attribute → JSON primitive (kept as string)
//! * repeated sibling elements → JSON array
//! * nested elements → JSON object
//! * the root element name → `resourceType`
//! * `<div>` (XHTML narrative) subtrees are collapsed to a placeholder —
//!   their internals are presentation, not data.

use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::{Map, Value};

/// Insert honoring FHIR's repeated-element = array rule.
fn insert_multi(map: &mut Map<String, Value>, key: String, val: Value) {
    match map.get_mut(&key) {
        Some(Value::Array(arr)) => arr.push(val),
        Some(existing) => {
            let prev = existing.take();
            *existing = Value::Array(vec![prev, val]);
        }
        None => {
            map.insert(key, val);
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
        let val = attr
            .unescape_value()
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

fn frame_to_value(f: Frame) -> Value {
    match (f.children.is_empty(), f.value_attr) {
        (true, Some(v)) => Value::String(v),
        (false, Some(v)) => {
            let mut m = f.children;
            m.insert("value".into(), Value::String(v));
            Value::Object(m)
        }
        (true, None) => Value::Object(Map::new()),
        (false, None) => Value::Object(f.children),
    }
}

/// Convert a FHIR XML document into (resource_type, JSON value).
pub fn fhir_xml_to_json(xml: &str) -> Result<(String, Value), String> {
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<Frame> = Vec::new();
    let mut root: Option<(String, Value)> = None;

    loop {
        match reader.read_event() {
            Err(e) => return Err(format!("Invalid XML: {}", e)),
            Ok(Event::Eof) => break,
            Ok(Event::Start(e)) => {
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
                stack.push(frame);
            }
            Ok(Event::Empty(e)) => {
                let frame = frame_from_start(&e)?;
                let name = frame.name.clone();
                let value = frame_to_value(frame);
                if let Some(parent) = stack.last_mut() {
                    insert_multi(&mut parent.children, name, value);
                }
            }
            Ok(Event::End(_)) => {
                let frame = match stack.pop() {
                    Some(f) => f,
                    None => return Err("Unbalanced XML end tag".into()),
                };
                let name = frame.name.clone();
                let value = frame_to_value(frame);
                if let Some(parent) = stack.last_mut() {
                    insert_multi(&mut parent.children, name, value);
                } else {
                    root = Some((name, value));
                }
            }
            // FHIR data lives in value attributes; free text only occurs
            // inside <div>, which is skipped above.
            Ok(_) => {}
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
    Ok((resource_type, Value::Object(obj)))
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
        // repeated <given> → array
        assert_eq!(json["name"]["given"][0], "Mario");
        assert_eq!(json["name"]["given"][1], "Giuseppe");
        // nested object
        assert_eq!(json["identifier"]["value"], "RSSMRA80A01F205X");
        // narrative collapsed, not exploded into xhtml structure
        assert_eq!(json["text"]["div"], "[xhtml narrative]");
    }

    #[test]
    fn test_invalid_xml() {
        assert!(fhir_xml_to_json("<Patient><id value=\"x\"/>").is_err());
        assert!(fhir_xml_to_json("").is_err());
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
}
