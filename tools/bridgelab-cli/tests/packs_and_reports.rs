//! Plugin packs as the CLI sees them without the app, and reports written
//! with --output, run as a user runs them.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CLI: &str = env!("CARGO_BIN_EXE_bridgelab-cli");

const ADT: &str = "MSH|^~\\&|SEND|FAC|RECV|FAC|20240101120000||ADT^A01|MSG1|P|2.5\rEVN|A01|20240101120000\rPID|1||12345^^^FAC^MR||Smith^Jane||19900515|F\r";

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("bl-cli-packs-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The CLI run with `args`, reading plugin packs from `config`.
fn run(args: &[&std::ffi::OsStr], config: &Path) -> Output {
    Command::new(CLI).args(args).env("XDG_CONFIG_HOME", config).output().expect("the CLI runs")
}

/// Four FHIR packs of one rule each in `config`'s plugins folder.
fn four_fhir_packs(config: &Path) {
    let dir = config.join("BridgeLab/plugins/fhir");
    std::fs::create_dir_all(&dir).unwrap();
    for id in ["a", "b", "c", "d"] {
        let pack = serde_json::json!({"id": id, "name": id,
            "fhir_rules": [{"rule_id": format!("{id}-1"), "expression": "true", "message": "m"}]});
        std::fs::write(dir.join(format!("{id}.json")), pack.to_string()).unwrap();
    }
}

/// A FHIR report said nothing when FHIR packs were left out by the cap
/// (only a line on standard error); it now carries the PLUGIN-CAP note
/// the HL7 v2 report has. The report goes to --output: nothing on
/// standard output, the file in UTF-8.
#[cfg(target_os = "linux")]
#[test]
fn a_fhir_report_written_to_a_file_notes_packs_left_out() {
    let d = tmp("fhir-cap");
    four_fhir_packs(&d);
    let patient = d.join("p.json");
    std::fs::write(&patient, r#"{"resourceType": "Patient", "id": "p"}"#).unwrap();
    let report = d.join("report.json");
    let out = run(
        &["validate".as_ref(), patient.as_os_str(), "--format".as_ref(), "json".as_ref(), "--output".as_ref(), report.as_os_str()],
        &d,
    );
    assert!(out.stdout.is_empty(), "{}", String::from_utf8_lossy(&out.stdout));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Report written to"), "{stderr}");
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
    let issues = json[0]["report"]["issues"].as_array().unwrap();
    let cap: Vec<&serde_json::Value> = issues.iter().filter(|i| i["rule_id"] == "PLUGIN-CAP").collect();
    assert_eq!(cap.len(), 1, "{issues:?}");
    assert!(cap[0]["message"].as_str().unwrap().starts_with("1 plugin pack not applied"), "{:?}", cap[0]);
    std::fs::remove_dir_all(&d).ok();
}

/// A .state.json that does not parse was read as empty without a word:
/// a pack switched off in the app ran in CI. The CLI now says so.
#[cfg(target_os = "linux")]
#[test]
fn an_unreadable_state_file_is_named() {
    let d = tmp("state");
    let plugins = d.join("BridgeLab/plugins");
    std::fs::create_dir_all(plugins.join("validation")).unwrap();
    std::fs::write(plugins.join(".state.json"), "{\"enabled\": {\"validation/x\": false,}}").unwrap();
    let msg = d.join("m.hl7");
    std::fs::write(&msg, ADT).unwrap();
    let out = run(&["validate".as_ref(), msg.as_os_str()], &d);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(".state.json could not be read"), "{stderr}");
    assert_eq!(
        std::fs::read_to_string(plugins.join(".state.json")).unwrap(),
        "{\"enabled\": {\"validation/x\": false,}}",
        "read only"
    );
    std::fs::remove_dir_all(&d).ok();
}

/// `validate <(zcat msg.hl7.gz)`: a process substitution is a /dev/fd
/// path to a pipe, which validate, info and test refused as matching no
/// file.
#[cfg(target_os = "linux")]
#[test]
fn a_process_substitution_is_read() {
    let d = tmp("procsub");
    let msg = d.join("m.hl7");
    std::fs::write(&msg, ADT).unwrap();
    let script = format!("\"$0\" validate <(cat '{}') && \"$0\" info --json <(cat '{}')", msg.display(), msg.display());
    let Ok(out) = Command::new("bash").arg("-c").arg(script).arg(CLI).env("XDG_CONFIG_HOME", &d).output() else {
        return; // no bash here
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}\n{}", String::from_utf8_lossy(&out.stderr));
    assert!(stdout.contains("HL7 ADT^A01") && stdout.contains("\"parsed\": true"), "{stdout}");
    std::fs::remove_dir_all(&d).ok();
}
