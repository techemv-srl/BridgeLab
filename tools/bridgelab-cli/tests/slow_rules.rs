//! A FHIR rule pack whose rule would take hours stops at its work budget:
//! `bridgelab-cli validate` in a CI job must finish and say which rule was
//! too slow, not hang.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const CLI: &str = env!("CARGO_BIN_EXE_bridgelab-cli");

/// Criteria nested nine deep over ten names: 10^9 evaluations, which took
/// hours, in a 500-byte pack loaded from the config directory.
#[cfg(target_os = "linux")]
#[test]
fn a_rule_that_would_take_hours_is_reported_as_too_slow() {
    let d = std::env::temp_dir().join(format!("bl-cli-slow-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let packs = d.join("BridgeLab/plugins/fhir");
    std::fs::create_dir_all(&packs).unwrap();
    let mut nested = "%resource.name".to_string();
    for _ in 0..9 {
        nested = format!("%resource.name.where({nested}.exists())");
    }
    let pack = serde_json::json!({
        "id": "slow", "name": "slow", "version": "1", "enabled": true,
        "fhir_rules": [{"rule_id": "slow-1", "severity": "warning", "resource": "Patient",
            "expression": format!("{nested}.count() > 100"), "message": "never"}]
    });
    std::fs::write(packs.join("slow.json"), pack.to_string()).unwrap();
    let names: Vec<serde_json::Value> = (0..10).map(|i| serde_json::json!({"family": format!("F{i}")})).collect();
    let patient = d.join("p10.json");
    std::fs::write(&patient, serde_json::json!({"resourceType": "Patient", "id": "p", "name": names}).to_string())
        .unwrap();

    let mut child = Command::new(CLI)
        .args(["validate".as_ref(), patient.as_os_str()])
        .env("XDG_CONFIG_HOME", &d)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the CLI runs");
    // A few seconds in a release build; well under this in a test build.
    let started = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > Duration::from_secs(300) {
            child.kill().ok();
            panic!("validate was still running after {:?}", started.elapsed());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("slow-1"), "{stdout}");
    assert!(stdout.contains("takes too long to evaluate (more than 50 million steps)"), "{stdout}");
    std::fs::remove_dir_all(&d).ok();
}
