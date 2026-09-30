//! Realistic test-message generator. The inverse of the anonymizer: builds
//! HL7 v2 messages with plausible-but-synthetic patient data (names, dates,
//! MRNs, addresses, lab values). Deterministic when a seed is supplied, so
//! generated regression sets are reproducible.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

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
    let sex = if rng.random_bool(0.5) { 'M' } else { 'F' };
    let given = if sex == 'M' {
        FIRST_NAMES_M[rng.random_range(0..FIRST_NAMES_M.len())]
    } else {
        FIRST_NAMES_F[rng.random_range(0..FIRST_NAMES_F.len())]
    };
    let year = rng.random_range(1930..=2015);
    let month = rng.random_range(1..=12);
    let day = rng.random_range(1..=28);
    Patient {
        family: LAST_NAMES[rng.random_range(0..LAST_NAMES.len())].to_string(),
        given: given.to_string(),
        sex,
        dob: format!("{:04}{:02}{:02}", year, month, day),
        mrn: format!("{:07}", rng.random_range(1_000_000u32..10_000_000)),
        street: format!("{} {}", STREETS[rng.random_range(0..STREETS.len())], rng.random_range(1..200)),
        city: CITIES[rng.random_range(0..CITIES.len())].to_string(),
        zip: format!("{:05}", rng.random_range(10000..99999)),
    }
}

/// Message timestamp: fixed base plus a pseudo-random offset so batches
/// spread over a plausible window while staying seed-deterministic.
fn gen_ts(rng: &mut StdRng, idx: usize) -> String {
    let day = rng.random_range(1..=28);
    let hour = (8 + idx % 10) as u32;
    let min = rng.random_range(0..60);
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
    let ward = rng.random_range(1..9);
    let room = rng.random_range(100..500);
    let class = ["I", "O", "E"][rng.random_range(0..3)];
    [
        msh(&ts, &format!("ADT^{}", event), &ctrl),
        format!("EVN|{}|{}", event, ts),
        pid(&p),
        format!("PV1|1|{}|WARD{}^{}^A|||||||MED|||||||||V{}", class, ward, room, idx + 1),
    ]
    .join("\r")
}

fn build_oru(rng: &mut StdRng, idx: usize) -> String {
    let p = gen_patient(rng);
    let ts = gen_ts(rng, idx);
    let ctrl = format!("GEN{:06}", idx + 1);
    let order = format!("ORD{:06}", rng.random_range(100_000u32..1_000_000));
    let mut segs = vec![
        msh(&ts, "ORU^R01", &ctrl),
        pid(&p),
        format!("OBR|1|{}||CBC^Complete Blood Count^L|||{}|||||||||||||||{}|F", order, ts, ts),
    ];
    let n_tests = rng.random_range(3..=LAB_TESTS.len());
    for (i, (code, name, unit, low, high)) in LAB_TESTS.iter().take(n_tests).enumerate() {
        // 15% of results deliberately out of range with an abnormal flag —
        // realistic sets need pathological values too.
        let abnormal = rng.random_bool(0.15);
        let value = if abnormal {
            high + (high - low) * rng.random_range(0.1..0.5)
        } else {
            rng.random_range(*low..*high)
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
    let order = format!("ORD{:06}", rng.random_range(100_000u32..1_000_000));
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
        None => StdRng::from_os_rng(),
    };

    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let effective = if kind == "mixed" {
            ["ADT^A01", "ADT^A08", "ORU^R01", "ORM^O01"][rng.random_range(0..4)]
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

/// One generated message to write, under a bare file name.
#[derive(Debug, Clone, Deserialize)]
pub struct NamedMessage {
    pub name: String,
    pub content: String,
}

/// What happened to one file of "Save all to folder".
#[derive(Debug, Clone, Serialize)]
pub struct SaveOutcome {
    pub name: String,
    /// `"exists"` when a file of that name was already there (it is left as
    /// it was), otherwise the write error; `None` when written.
    pub error: Option<String>,
}

/// Write generated messages into `dir`, each under its own name. A name
/// already taken in the folder is never replaced (nor a link at that name
/// followed): that file is reported as `exists` and the rest are written.
#[tauri::command]
pub async fn save_generated_messages(dir: String, files: Vec<NamedMessage>) -> Result<Vec<SaveOutcome>, String> {
    let dir = std::path::PathBuf::from(dir);
    if !tokio::fs::metadata(&dir).await.map(|m| m.is_dir()).unwrap_or(false) {
        return Err(format!("{} is not a folder", dir.display()));
    }
    let mut out = Vec::with_capacity(files.len());
    for f in files {
        // A bare name only: the folder is the one the user picked.
        let bare = std::path::Path::new(&f.name).file_name().map(|n| n == f.name.as_str()).unwrap_or(false);
        let error = if !bare || f.name.contains(['/', '\\']) {
            Some(format!("invalid file name: {}", f.name))
        } else {
            match crate::commands::batch::write_new(&dir.join(&f.name), f.content.as_bytes()).await {
                Ok(()) => None,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Some("exists".into()),
                Err(e) => Some(e.to_string()),
            }
        };
        out.push(SaveOutcome { name: f.name, error });
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

    #[test]
    fn the_visit_number_is_pv1_19() {
        for m in generate_test_messages("ADT^A01".into(), 60, Some(1234)).unwrap() {
            let pv1 = m.content.split('\r').find(|s| s.starts_with("PV1|")).unwrap();
            let fields: Vec<&str> = pv1.split('|').collect();
            assert!(fields[19].starts_with('V') && fields[18].is_empty(), "{pv1}");
            let report = validate_hl7_message(&Hl7Lexer::new().parse(m.content.clone().into_bytes()).unwrap());
            assert!(!report.issues.iter().any(|i| i.rule_id.contains("PV1-18")), "{:?}", report.issues);
        }
    }

    #[tokio::test]
    async fn save_all_never_replaces_an_existing_file() {
        let dir = std::env::temp_dir().join(format!("bl_gen_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("adt_a01_001.hl7"), "OLD").unwrap();
        let files = vec![
            NamedMessage { name: "adt_a01_001.hl7".into(), content: "NEW1".into() },
            NamedMessage { name: "adt_a01_002.hl7".into(), content: "NEW2".into() },
            NamedMessage { name: "../escape.hl7".into(), content: "X".into() },
        ];
        let out = save_generated_messages(dir.display().to_string(), files).await.unwrap();
        assert_eq!(out[0].error.as_deref(), Some("exists"));
        assert!(out[1].error.is_none());
        assert!(out[2].error.as_deref().unwrap_or("").contains("invalid"));
        assert_eq!(std::fs::read_to_string(dir.join("adt_a01_001.hl7")).unwrap(), "OLD");
        assert_eq!(std::fs::read_to_string(dir.join("adt_a01_002.hl7")).unwrap(), "NEW2");
        assert!(!dir.parent().unwrap().join("escape.hl7").exists());
        std::fs::remove_dir_all(&dir).ok();
    }
}
