//! What the CLI writes to a terminal: no control character from a
//! message, an ACK, a plugin pack, a file name or an argument may reach it
//! raw, on standard output or standard error. An escape sequence there can
//! clear the screen, retitle the window, or draw "AA" over a rejection.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CLI: &str = env!("CARGO_BIN_EXE_bridgelab-cli");

/// What an attacker would put in front of the terminal.
const ESC_SEQ: &str = "\u{1b}[2J\u{1b}[32mAA\u{1b}]0;pwned\u{7}";

const PLAIN: &str = "MSH|^~\\&|SEND|FAC|RECV|FAC|20240101120000||ADT^A01|MSG1|P|2.5\rPID|1||12345^^^FAC^MR||Smith^Jane||19900515|F\r";

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("bl-cli-term-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The CLI run with `args`, with no plugin pack of the user's own.
fn run(args: &[&std::ffi::OsStr], config: &Path) -> Output {
    Command::new(CLI)
        .args(args)
        .env("XDG_CONFIG_HOME", config)
        .output()
        .expect("the CLI runs")
}

/// No control character but newline (and tab) in `bytes`.
fn assert_no_controls(what: &str, bytes: &[u8]) {
    let text = String::from_utf8_lossy(bytes);
    let bad: Vec<char> = text.chars().filter(|c| c.is_control() && *c != '\n' && *c != '\t').collect();
    assert!(bad.is_empty(), "{what} holds {bad:?}: {text}");
}

/// A peer that answers every message with an ACK whose MSA-1 holds an
/// escape sequence; its port.
fn hostile_peer() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut got = Vec::new();
            let mut buf = [0u8; 4096];
            while !got.windows(2).any(|w| w == [0x1c, 0x0d]) {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => return,
                    Ok(n) => got.extend_from_slice(&buf[..n]),
                }
            }
            let ack = format!("MSH|^~\\&|R|F|S|F|20240101||ACK|A1|P|2.5\rMSA|{ESC_SEQ}|MSG1\r");
            let mut frame = vec![0x0b];
            frame.extend_from_slice(ack.as_bytes());
            frame.extend_from_slice(&[0x1c, 0x0d]);
            let _ = stream.write_all(&frame);
        }
    });
    port
}

#[test]
fn send_shows_the_peers_msa1_escaped() {
    let d = tmp("send");
    let msg = d.join("plain.hl7");
    std::fs::write(&msg, PLAIN).unwrap();
    let port = hostile_peer().to_string();
    let out = run(
        &[
            "send".as_ref(),
            msg.as_os_str(),
            "--host".as_ref(),
            "127.0.0.1".as_ref(),
            "--port".as_ref(),
            port.as_ref(),
            "--timeout".as_ref(),
            "5".as_ref(),
        ],
        &d,
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(r"ACK \x1B[2J\x1B[32mAA\x1B]0;pwned\x07"), "{stderr}");
    assert_no_controls("stderr", &out.stderr);
    assert_no_controls("stdout", &out.stdout);
    // Not accepted: an ACK that only looks like AA.
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    std::fs::remove_dir_all(&d).ok();
}

/// A plugin pack's rule id, severity and file name, quoted in its warning.
#[cfg(target_os = "linux")]
#[test]
fn plugin_pack_warnings_are_escaped() {
    let d = tmp("pack");
    let packs = d.join("BridgeLab/plugins/validation");
    std::fs::create_dir_all(&packs).unwrap();
    let pack = serde_json::json!({
        "id": "esc-pack", "name": "Team pack", "version": "1.0", "enabled": true,
        "validation_rules": [{
            "rule_id": format!("R{ESC_SEQ}-1"), "severity": "bad\u{1b}[2J",
            "segment": "PID", "field": 0, "check": {"type": "not_empty"}, "message": "m"
        }]
    });
    std::fs::write(packs.join("e\u{1b}[2Jsc.json"), pack.to_string()).unwrap();
    let msg = d.join("plain.hl7");
    std::fs::write(&msg, PLAIN).unwrap();
    let out = run(&["validate".as_ref(), msg.as_os_str()], &d);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(r"e\x1B[2Jsc.json"), "{stderr}");
    assert!(stderr.contains(r"R\x1B[2J\x1B[32mAA\x1B]0;pwned\x07-1"), "{stderr}");
    assert!(stderr.contains(r#""bad\x1B[2J""#), "{stderr}");
    assert_no_controls("stderr", &out.stderr);
    assert_no_controls("stdout", &out.stdout);
    std::fs::remove_dir_all(&d).ok();
}

/// Command-line values and a message's MSH-18, quoted in errors and notes.
#[cfg(unix)]
#[test]
fn arguments_file_names_and_charsets_are_escaped() {
    let d = tmp("args");
    let msg = d.join("plain.hl7");
    std::fs::write(&msg, PLAIN).unwrap();
    let charset = d.join("cs.hl7");
    std::fs::write(&charset, "MSH|^~\\&|S|F|R|F|20240101||ADT^A01|1|P|2.5|||||ITA|X\u{1b}[2JY\rPID|1\r").unwrap();
    let written = d.join("out\u{1b}[2J.json");
    let no_dir = d.join("no\u{1b}[2Jdir");
    let cases: Vec<(Vec<&std::ffi::OsStr>, &str)> = vec![
        (vec!["to-json".as_ref(), msg.as_os_str(), "-o".as_ref(), written.as_os_str()], r"out\x1B[2J.json"),
        (vec!["xsd".as_ref(), "ADT\u{1b}[2J_A01".as_ref()], r"ADT\x1B[2J_A01"),
        (vec!["to-json".as_ref(), charset.as_os_str()], r"X\x1B[2JY"),
        (vec!["anonymize".as_ref(), charset.as_os_str()], r"X\x1B[2JY"),
        (vec!["batch".as_ref(), no_dir.as_os_str()], r"no\x1B[2Jdir"),
    ];
    for (args, shown) in &cases {
        let out = run(args, &d);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains(shown), "{args:?}: {stderr}");
        assert_no_controls("stderr", &out.stderr);
    }
    std::fs::remove_dir_all(&d).ok();
}

/// JSON escapes DEL and the C1 controls too (U+009B is CSI to the Linux
/// console); serde_json alone wrote them raw.
#[test]
fn json_output_carries_no_raw_del_or_c1() {
    let d = tmp("json");
    let c1 = "X\u{9b}2J\u{9b}31mFAKE OK\u{9d}0;title\u{9c}\u{7f}";
    let msg = d.join("c1.hl7");
    std::fs::write(&msg, format!("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||1||{c1}\r")).unwrap();
    let out = run(&["to-json".as_ref(), msg.as_os_str()], &d);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    assert_no_controls("to-json", &out.stdout);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains(r"X\u009b2J\u009b31mFAKE OK\u009d0;title\u009c\u007f"), "{text}");

    let patient = d.join("p.json");
    std::fs::write(&patient, serde_json::json!({"resourceType": "Patient", "name": [{"family": c1}]}).to_string()).unwrap();
    let out = run(&["fhirpath".as_ref(), "Patient.name.family".as_ref(), patient.as_os_str(), "--json".as_ref()], &d);
    assert_no_controls("fhirpath --json", &out.stdout);
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["results"][0], serde_json::json!(c1), "{value}");
    std::fs::remove_dir_all(&d).ok();
}

/// JUnit: DEL and C1 as character references, here from a file name.
#[cfg(target_os = "linux")]
#[test]
fn junit_output_carries_no_raw_del_or_c1() {
    let d = tmp("junit");
    let name = d.join("a\u{9b}31m\u{7f}b.hl7");
    std::fs::write(&name, PLAIN).unwrap();
    let out = run(&["validate".as_ref(), name.as_os_str(), "--format".as_ref(), "junit".as_ref()], &d);
    assert_no_controls("junit", &out.stdout);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("a&#155;31m&#127;b.hl7"), "{text}");
    std::fs::remove_dir_all(&d).ok();
}
