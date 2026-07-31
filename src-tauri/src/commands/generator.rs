//! Realistic test-message generator. The inverse of the anonymizer: builds
//! HL7 v2 messages with plausible-but-synthetic patient data (names, dates,
//! MRNs, addresses, lab values). Deterministic when a seed is supplied, so
//! generated regression sets are reproducible.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::Serialize;

const MAX_COUNT: usize = 500;

const FIRST_NAMES_M: &[&str] = &[
    "MARIO", "LUCA", "GIUSEPPE", "ANDREA", "FRANCESCO", "MARCO", "PAOLO",
    "JOHN", "MICHAEL", "DAVID", "THOMAS", "HANS", "PIERRE", "CARLOS",
];
const FIRST_NAMES_F: &[&str] = &[
    "MARIA", "ANNA", "GIULIA", "FRANCESCA", "LAURA", "SOFIA", "ELENA",
    "MARY", "SARAH", "EMMA", "CLAIRE", "GRETA", "CARMEN", "LUCIA",
];
const LAST_NAMES: &[&str] = &[
    "ROSSI", "BIANCHI", "FERRARI", "ESPOSITO", "RUSSO", "COLOMBO", "RICCI",
    "SMITH", "JOHNSON", "MUELLER", "MARTIN", "GARCIA", "DUBOIS", "SILVA",
];
const CITIES: &[&str] = &[
    "MILANO", "ROMA", "TORINO", "BOLOGNA", "FIRENZE", "NAPOLI", "GENOVA", "VERONA",
];
const STREETS: &[&str] = &[
    "VIA ROMA", "VIA GARIBALDI", "CORSO ITALIA", "VIA DANTE", "VIA VERDI",
    "PIAZZA DUOMO", "VIA MAZZINI", "VIALE EUROPA",
];

/// (code, name, unit, low, high) — plausible lab panel for ORU messages.
const LAB_TESTS: &[(&str, &str, &str, f64, f64)] = &[
    ("GLU", "Glucose", "mg/dL", 65.0, 110.0),
    ("WBC", "White Blood Cell Count", "10*3/uL", 4.0, 11.0),
    ("HGB", "Hemoglobin", "g/dL", 12.0, 17.5),
    ("PLT", "Platelet Count", "10*3/uL", 150.0, 400.0),
    ("CREA", "Creatinine", "mg/dL", 0.6, 1.3),
    ("NA", "Sodium", "mmol/L", 135.0, 145.0),
    ("K", "Potassium", "mmol/L", 3.5, 5.1),
];

#[derive(Debug, Clone, Serialize)]
pub struct GeneratedMessage {
    pub content: String,
    pub label: String,
}

struct Patient {
    family: String,
    given: String,
    sex: char,
    dob: String,
    mrn: String,
    street: String,
    city: String,
    zip: String,
}

fn gen_patient(rng: &mut StdRng) -> Patient {
    let sex = if rng.gen_bool(0.5) { 'M' } else { 'F' };
    let given = if sex == 'M' {
        FIRST_NAMES_M[rng.gen_range(0..FIRST_NAMES_M.len())]
    } else {
        FIRST_NAMES_F[rng.gen_range(0..FIRST_NAMES_F.len())]
    };
    let year = rng.gen_range(1930..=2015);
    let month = rng.gen_range(1..=12);
    let day = rng.gen_range(1..=28);
    Patient {
        family: LAST_NAMES[rng.gen_range(0..LAST_NAMES.len())].to_string(),
        given: given.to_string(),
        sex,
        dob: format!("{:04}{:02}{:02}", year, month, day),
        mrn: format!("{:07}", rng.gen_range(1_000_000u32..10_000_000)),
        street: format!("{} {}", STREETS[rng.gen_range(0..STREETS.len())], rng.gen_range(1..200)),
        city: CITIES[rng.gen_range(0..CITIES.len())].to_string(),
        zip: format!("{:05}", rng.gen_range(10000..99999)),
    }
}

/// Message timestamp: fixed base plus a pseudo-random offset so batches
/// spread over a plausible window while staying seed-deterministic.
fn gen_ts(rng: &mut StdRng, idx: usize) -> String {
    let day = rng.gen_range(1..=28);
    let hour = (8 + idx % 10) as u32;
    let min = rng.gen_range(0..60);
    format!("202607{:02}{:02}{:02}00", day, hour, min)
}

fn msh(ts: &str, msg_type: &str, ctrl: &str) -> String {
    format!(
        "MSH|^~\\&|BRIDGELAB_GEN|TESTFAC|TESTAPP|TESTFAC|{}||{}|{}|P|2.5",
        ts, msg_type, ctrl
    )
}

fn pid(p: &Patient) -> String {
    format!(
        "PID|1||{}^^^HOSPITAL^MR||{}^{}||{}|{}|||{}^^{}^^{}^IT",
        p.mrn, p.family, p.given, p.dob, p.sex, p.street, p.city, p.zip
    )
}

fn build_adt(rng: &mut StdRng, idx: usize, event: &str) -> String {
    let p = gen_patient(rng);
    let ts = gen_ts(rng, idx);
    let ctrl = format!("GEN{:06}", idx + 1);
    let ward = rng.gen_range(1..9);
    let room = rng.gen_range(100..500);
    let class = ["I", "O", "E"][rng.gen_range(0..3)];
    [
        msh(&ts, &format!("ADT^{}", event), &ctrl),
        format!("EVN|{}|{}", event, ts),
        pid(&p),
        format!("PV1|1|{}|WARD{}^{}^A|||||||MED||||||||V{}", class, ward, room, idx + 1),
    ]
    .join("\r")
}

fn build_oru(rng: &mut StdRng, idx: usize) -> String {
    let p = gen_patient(rng);
    let ts = gen_ts(rng, idx);
    let ctrl = format!("GEN{:06}", idx + 1);
    let order = format!("ORD{:06}", rng.gen_range(100_000u32..1_000_000));
    let mut segs = vec![
        msh(&ts, "ORU^R01", &ctrl),
        pid(&p),
        format!("OBR|1|{}||CBC^Complete Blood Count^L|||{}|||||||||||||||{}|F", order, ts, ts),
    ];
    let n_tests = rng.gen_range(3..=LAB_TESTS.len());
    for (i, (code, name, unit, low, high)) in LAB_TESTS.iter().take(n_tests).enumerate() {
        // 15% of results deliberately out of range with an abnormal flag —
        // realistic sets need pathological values too.
        let abnormal = rng.gen_bool(0.15);
        let value = if abnormal {
            high + (high - low) * rng.gen_range(0.1..0.5)
        } else {
            rng.gen_range(*low..*high)
        };
        let flag = if abnormal { "H" } else { "N" };
        segs.push(format!(
            "OBX|{}|NM|{}^{}^L||{:.1}|{}|{:.1}-{:.1}|{}|||F|||{}",
            i + 1, code, name, value, unit, low, high, flag, ts
        ));
    }
    segs.join("\r")
}

fn build_orm(rng: &mut StdRng, idx: usize) -> String {
    let p = gen_patient(rng);
    let ts = gen_ts(rng, idx);
    let ctrl = format!("GEN{:06}", idx + 1);
    let order = format!("ORD{:06}", rng.gen_range(100_000u32..1_000_000));
    [
        msh(&ts, "ORM^O01", &ctrl),
        pid(&p),
        format!("ORC|NW|{}||||||||||^GENERATOR^TEST", order),
        format!("OBR|1|{}||CBC^Complete Blood Count^L|||{}", order, ts),
    ]
    .join("\r")
}

/// Generate `count` synthetic messages of the given kind
/// ("ADT^A01" | "ADT^A08" | "ORU^R01" | "ORM^O01" | "mixed").
/// A seed makes the output reproducible.
#[tauri::command]
pub fn generate_test_messages(
    kind: String,
    count: usize,
    seed: Option<u64>,
) -> Result<Vec<GeneratedMessage>, String> {
    let count = count.clamp(1, MAX_COUNT);
    let mut rng = match seed {
        Some(s) => StdRng::seed_from_u64(s),
        None => StdRng::from_entropy(),
    };

    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let effective = if kind == "mixed" {
            ["ADT^A01", "ADT^A08", "ORU^R01", "ORM^O01"][rng.gen_range(0..4)]
        } else {
            kind.as_str()
        };
        let content = match effective {
            "ADT^A01" => build_adt(&mut rng, i, "A01"),
            "ADT^A08" => build_adt(&mut rng, i, "A08"),
            "ORU^R01" => build_oru(&mut rng, i),
            "ORM^O01" => build_orm(&mut rng, i),
            other => return Err(format!("Unknown message kind: {}", other)),
        };
        out.push(GeneratedMessage {
            content,
            label: format!("{} #{}", effective, i + 1),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::hl7::lexer::Hl7Lexer;
    use crate::validation::validate_hl7_message;

    #[test]
    fn test_generated_messages_parse_and_validate() {
        for kind in ["ADT^A01", "ADT^A08", "ORU^R01", "ORM^O01", "mixed"] {
            let msgs = generate_test_messages(kind.into(), 10, Some(42)).unwrap();
            assert_eq!(msgs.len(), 10);
            for m in &msgs {
                let parsed = Hl7Lexer::new()
                    .parse(m.content.clone().into_bytes())
                    .unwrap_or_else(|e| panic!("{} failed to parse: {} — {}", kind, e, m.content));
                let report = validate_hl7_message(&parsed);
                assert_eq!(
                    report.error_count, 0,
                    "{} must validate clean, got issues: {:?}",
                    kind, report.issues
                );
            }
        }
    }

    #[test]
    fn test_seed_is_deterministic() {
        let a = generate_test_messages("ADT^A01".into(), 5, Some(7)).unwrap();
        let b = generate_test_messages("ADT^A01".into(), 5, Some(7)).unwrap();
        assert_eq!(
            a.iter().map(|m| &m.content).collect::<Vec<_>>(),
            b.iter().map(|m| &m.content).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_count_clamped() {
        let msgs = generate_test_messages("ADT^A01".into(), 100_000, Some(1)).unwrap();
        assert_eq!(msgs.len(), MAX_COUNT);
    }

    #[test]
    fn test_unknown_kind() {
        assert!(generate_test_messages("XXX^Y99".into(), 1, Some(1)).is_err());
    }
}
