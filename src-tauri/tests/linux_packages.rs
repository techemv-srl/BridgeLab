//! What the Linux packages declare.
//!
//! "Open plugins folder" (and every link the app opens on Linux) runs
//! `xdg-open`, which comes with xdg-utils: a system without it showed a raw
//! "No such file or directory" error. The .deb and .rpm therefore depend on
//! it, next to the WebKitGTK and GTK libraries the window needs.

use std::path::PathBuf;

fn conf() -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
    serde_json::from_str(&text).unwrap()
}

#[test]
fn deb_and_rpm_depend_on_xdg_utils() {
    let conf = conf();
    for pkg in ["deb", "rpm"] {
        let depends: Vec<&str> = conf["bundle"]["linux"][pkg]["depends"]
            .as_array()
            .unwrap_or_else(|| panic!("{pkg}: no depends list"))
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert!(depends.contains(&"xdg-utils"), "{pkg}: {depends:?}");
    }
}

/// The packages are named "bridgelab", like the binary, the folders and
/// the documentation (Tauri names them after productName, so the Linux
/// build sets it in tauri.linux.conf.json), and they replace the
/// "bridge-lab" packages of earlier releases: installing over one removes
/// it. The menu entry still reads "BridgeLab".
#[test]
fn the_packages_are_named_bridgelab_and_replace_bridge_lab() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let linux: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("tauri.linux.conf.json")).unwrap()).unwrap();
    assert_eq!(linux["productName"], "bridgelab");
    let names = |v: &serde_json::Value| -> Vec<String> {
        v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
    };
    let deb = &linux["bundle"]["linux"]["deb"];
    let rpm = &linux["bundle"]["linux"]["rpm"];
    for (pkg, field) in [(deb, "replaces"), (deb, "conflicts"), (deb, "provides"), (rpm, "obsoletes"), (rpm, "conflicts"), (rpm, "provides")] {
        assert!(names(&pkg[field]).contains(&"bridge-lab".to_string()), "{field}: {:?}", pkg[field]);
    }
    for pkg in [deb, rpm] {
        let template = pkg["desktopTemplate"].as_str().expect("desktop template");
        let text = std::fs::read_to_string(dir.join(template)).unwrap();
        assert!(text.lines().any(|l| l == "Name=BridgeLab"), "{text}");
    }
    // The documentation sits under the package's own name.
    for pkg in ["deb", "rpm"] {
        let files = conf()["bundle"]["linux"][pkg]["files"].as_object().unwrap().clone();
        assert!(files.keys().any(|k| k == "/usr/share/doc/bridgelab/copyright"), "{pkg}: {files:?}");
    }
}
