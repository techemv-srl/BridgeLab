//! Who decides whether BridgeLab looks for new versions at startup.
//!
//! Three sources, strongest first:
//!
//! 1. **Machine policy**, for IT departments: the environment variable
//!    `BRIDGELAB_DISABLE_UPDATE_CHECK=1`, or a `policy.json` containing
//!    `{"disable_update_check": true}` in the machine-wide folder —
//!    `%ProgramData%\BridgeLab\` on Windows,
//!    `/Library/Application Support/BridgeLab/` on macOS,
//!    `/etc/bridgelab/` on Linux. Either one turns the check off for every
//!    user of the machine, and the Settings checkbox is shown locked.
//! 2. **The installer's choice**: the Windows setup asks, and writes
//!    `installer.json` (`{"update_check_on_startup": false}`) into the
//!    user's BridgeLab data folder. The app copies it into the preferences
//!    the first time, when the user has not chosen yet.
//! 3. **The user's preference** in Settings → Privacy, on by default.
//!
//! Only 1 and 2 are read here; the preference lives in the database.
//!
//! The same machine policy can also force the opt-in usage statistics off:
//! `BRIDGELAB_DISABLE_TELEMETRY=1`, or `"disable_telemetry": true` in the
//! same `policy.json`. Nothing is sent then, whatever the user ticked, and
//! the Settings checkbox is shown locked (see [`telemetry_policy`]).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const POLICY_ENV: &str = "BRIDGELAB_DISABLE_UPDATE_CHECK";
pub const TELEMETRY_POLICY_ENV: &str = "BRIDGELAB_DISABLE_TELEMETRY";

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct UpdatePolicy {
    /// True when a machine policy turns the startup check off.
    pub disabled_by_policy: bool,
    /// What turned it off ("environment" or the policy file path), for the
    /// Settings hint.
    pub policy_source: Option<String>,
    /// The Windows installer's choice, when one was recorded.
    pub installer_choice: Option<bool>,
}

/// A machine policy that forces the usage statistics off.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TelemetryPolicy {
    pub disabled_by_policy: bool,
    /// "environment (…)" or the policy file path, for the Settings hint.
    pub policy_source: Option<String>,
}

#[derive(Deserialize)]
struct PolicyFile {
    #[serde(default)]
    disable_update_check: bool,
    #[serde(default)]
    disable_telemetry: bool,
}

fn read_policy_file(policy_file: &Path) -> Option<PolicyFile> {
    std::fs::read_to_string(policy_file)
        .ok()
        .and_then(|s| serde_json::from_str::<PolicyFile>(&s).ok())
}

#[derive(Deserialize)]
struct InstallerFile {
    update_check_on_startup: Option<bool>,
}

fn env_says_disabled(value: Option<String>) -> bool {
    matches!(
        value.as_deref().map(|v| v.trim().to_ascii_lowercase()).as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

/// The machine-wide policy file for this platform.
pub fn machine_policy_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var_os("PROGRAMDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
        base.join("BridgeLab").join("policy.json")
    }
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/Library/Application Support/BridgeLab/policy.json")
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        PathBuf::from("/etc/bridgelab/policy.json")
    }
}

/// Where the Windows installer records its choice (the same data folder
/// the database lives in).
pub fn installer_choice_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("BridgeLab").join("installer.json"))
}

/// Resolve the policy from explicit inputs, so it can be tested without
/// touching the real environment or file system. An unreadable or
/// malformed file is ignored: a typo must not change behaviour silently
/// in a direction nobody asked for, and "on" is the documented default.
pub fn resolve(env_value: Option<String>, policy_file: &Path, installer_file: Option<&Path>) -> UpdatePolicy {
    if env_says_disabled(env_value) {
        return UpdatePolicy {
            disabled_by_policy: true,
            policy_source: Some(format!("environment ({})", POLICY_ENV)),
            installer_choice: None,
        };
    }
    let from_file = read_policy_file(policy_file)
        .map(|p| p.disable_update_check)
        .unwrap_or(false);
    let installer_choice = installer_file
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<InstallerFile>(&s).ok())
        .and_then(|f| f.update_check_on_startup);
    UpdatePolicy {
        disabled_by_policy: from_file,
        policy_source: from_file.then(|| policy_file.display().to_string()),
        installer_choice,
    }
}

#[tauri::command]
pub fn get_update_policy() -> UpdatePolicy {
    let installer = installer_choice_path();
    resolve(std::env::var(POLICY_ENV).ok(), &machine_policy_path(), installer.as_deref())
}

/// Telemetry counterpart of [`resolve`]: same environment/file rules, with
/// `BRIDGELAB_DISABLE_TELEMETRY` and `"disable_telemetry"`. A malformed file
/// is ignored here too, which can only leave the user's own choice in
/// place — telemetry is off unless the user turned it on.
pub fn resolve_telemetry(env_value: Option<String>, policy_file: &Path) -> TelemetryPolicy {
    if env_says_disabled(env_value) {
        return TelemetryPolicy {
            disabled_by_policy: true,
            policy_source: Some(format!("environment ({})", TELEMETRY_POLICY_ENV)),
        };
    }
    let from_file = read_policy_file(policy_file)
        .map(|p| p.disable_telemetry)
        .unwrap_or(false);
    TelemetryPolicy {
        disabled_by_policy: from_file,
        policy_source: from_file.then(|| policy_file.display().to_string()),
    }
}

/// The telemetry policy of this machine, read fresh on every call so a
/// policy deployed while the app is open takes effect at the next send.
pub fn telemetry_policy() -> TelemetryPolicy {
    resolve_telemetry(std::env::var(TELEMETRY_POLICY_ENV).ok(), &machine_policy_path())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str, content: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bl-update-policy-{}-{}", std::process::id(), name));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("f.json");
        std::fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn nothing_configured_means_no_policy_and_no_installer_choice() {
        let missing = Path::new("/nonexistent/bridgelab/policy.json");
        assert_eq!(
            resolve(None, missing, None),
            UpdatePolicy { disabled_by_policy: false, policy_source: None, installer_choice: None }
        );
    }

    #[test]
    fn the_environment_variable_turns_it_off() {
        let missing = Path::new("/nonexistent/p.json");
        for v in ["1", "true", "YES", " on "] {
            assert!(resolve(Some(v.into()), missing, None).disabled_by_policy, "{v:?}");
        }
        for v in ["0", "false", "", "maybe"] {
            assert!(!resolve(Some(v.into()), missing, None).disabled_by_policy, "{v:?}");
        }
    }

    #[test]
    fn the_policy_file_turns_it_off_and_names_itself() {
        let p = tmp("on", r#"{"disable_update_check": true}"#);
        let r = resolve(None, &p, None);
        assert!(r.disabled_by_policy);
        assert_eq!(r.policy_source.as_deref(), Some(p.display().to_string().as_str()));
        let off = tmp("off", r#"{"disable_update_check": false}"#);
        assert!(!resolve(None, &off, None).disabled_by_policy);
    }

    #[test]
    fn a_malformed_policy_file_is_ignored() {
        let p = tmp("bad", "disable_update_check = yes");
        assert!(!resolve(None, &p, None).disabled_by_policy);
    }

    #[test]
    fn telemetry_policy_from_environment_or_file() {
        let missing = Path::new("/nonexistent/p.json");
        assert_eq!(
            resolve_telemetry(None, missing),
            TelemetryPolicy { disabled_by_policy: false, policy_source: None }
        );
        let env = resolve_telemetry(Some("1".into()), missing);
        assert!(env.disabled_by_policy);
        assert!(env.policy_source.unwrap().contains(TELEMETRY_POLICY_ENV));
        assert!(!resolve_telemetry(Some("0".into()), missing).disabled_by_policy);

        let p = tmp("tel-on", r#"{"disable_telemetry": true}"#);
        let r = resolve_telemetry(None, &p);
        assert!(r.disabled_by_policy);
        assert_eq!(r.policy_source.as_deref(), Some(p.display().to_string().as_str()));
        assert!(!resolve(None, &p, None).disabled_by_policy, "telemetry key must not touch the update check");

        let upd = tmp("tel-upd", r#"{"disable_update_check": true}"#);
        assert!(!resolve_telemetry(None, &upd).disabled_by_policy, "update key must not touch telemetry");
        let both = tmp("tel-both", r#"{"disable_update_check": true, "disable_telemetry": true}"#);
        assert!(resolve_telemetry(None, &both).disabled_by_policy);
        assert!(resolve(None, &both, None).disabled_by_policy);
        let bad = tmp("tel-bad", "disable_telemetry = yes");
        assert!(!resolve_telemetry(None, &bad).disabled_by_policy);
    }

    #[test]
    fn the_installer_choice_is_reported_both_ways() {
        let missing = Path::new("/nonexistent/p.json");
        let no = tmp("inst-no", r#"{"update_check_on_startup": false}"#);
        let yes = tmp("inst-yes", r#"{"update_check_on_startup": true}"#);
        assert_eq!(resolve(None, missing, Some(&no)).installer_choice, Some(false));
        assert_eq!(resolve(None, missing, Some(&yes)).installer_choice, Some(true));
        let junk = tmp("inst-junk", "{}");
        assert_eq!(resolve(None, missing, Some(&junk)).installer_choice, None);
    }
}
