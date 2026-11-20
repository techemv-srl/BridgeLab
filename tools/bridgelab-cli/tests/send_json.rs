//! `send --json` gives a report on standard output whatever stops the send,
//! the checks made before anything goes out included: a script that parses
//! it must not get an empty standard output instead.

use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;

const CLI: &str = env!("CARGO_BIN_EXE_bridgelab-cli");

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("bl-cli-sendjson-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// `send --json` of `bytes` to a port where nothing may arrive: the exit
/// code and the report parsed from standard output.
fn send_json(name: &str, bytes: &[u8]) -> (Option<i32>, serde_json::Value) {
    let d = tmp(name);
    let input = d.join("in.hl7");
    std::fs::write(&input, bytes).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port().to_string();
    let out = Command::new(CLI)
        .args(["send".as_ref(), input.as_os_str(), "--host".as_ref(), "127.0.0.1".as_ref()])
        .args(["--port", &port, "--timeout", "2", "--json"])
        .env("XDG_CONFIG_HOME", &d)
        .output()
        .expect("the CLI runs");
    assert!(listener.accept().is_err(), "nothing may be sent");
    std::fs::remove_dir_all(&d).ok();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let report = serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {stdout:?}"));
    (out.status.code(), report)
}

#[test]
fn a_charset_bridgelab_cannot_send_in_is_reported_as_json() {
    let (code, report) = send_json(
        "ir87",
        b"MSH|^~\\&|A|B|C|D|20260101||ADT^A01|M1|P|2.5\rPID|1\rMSH|^~\\&|A|B|C|D|20260101||ADT^A01|M2|P|2.5|||||JPN|ISO IR87\rPID|1\r",
    );
    assert_eq!(code, Some(1));
    assert_eq!(report["delivered"], false);
    assert_eq!(report["control_id"], "M2");
    assert_eq!(report["refused"]["reason"], "charset_not_written");
    assert_eq!(report["refused"]["message"], 2);
    assert_eq!(report["refused"]["declared"], "ISO IR87");
    assert!(report["error"].as_str().is_some_and(|e| e.contains("ISO IR87")), "{report}");
}

#[test]
fn messages_in_different_charsets_are_reported_as_json() {
    // Latin-1 bytes: the file goes out in 8859/1, and the second message,
    // declaring UTF-8, would reach the receiver in other bytes.
    let (code, report) = send_json(
        "mixed",
        b"MSH|^~\\&|A|B|C|D|20260101||ADT^A01|M1|P|2.5|||||ITA|8859/1\rPID|1||1||M\xfcller\r\
          MSH|^~\\&|A|B|C|D|20260101||ADT^A01|M2|P|2.5|||||ITA|UNICODE UTF-8\rPID|1||1||M\xfcller\r",
    );
    assert_eq!(code, Some(1));
    assert_eq!(report["delivered"], false);
    assert_eq!(report["control_id"], "M2");
    assert_eq!(report["refused"]["reason"], "mixed_charsets");
    assert_eq!(report["refused"]["message"], 2);
    assert_eq!(report["refused"]["declared"], "UNICODE UTF-8");
    assert_eq!(report["refused"]["charset"], "UTF-8");
    assert_eq!(report["refused"]["sent_in"], "ISO-8859-1");
    assert!(report["error"].as_str().is_some_and(|e| e.contains("separate files")), "{report}");
}
