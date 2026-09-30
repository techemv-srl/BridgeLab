use ed25519_dalek::{Signature, Verifier, VerifyingKey, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod feature_gate;
#[cfg(feature = "desktop")]
pub mod online;
#[cfg(feature = "desktop")]
pub mod telemetry;

// =============================================================================
// The PUBLIC key is embedded in the app for offline verification.
// The PRIVATE key is kept secret in the CLI tool only.
// Generate a new keypair with: bridgelab-keygen generate-keypair
// =============================================================================
const PUBLIC_KEY_HEX: &str = "cd9559f4beffe61a9c2878434a84fb2c3de85e36247c4188537c722d9fcc2649";

/// License payload (the data that gets signed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePayload {
    pub license_type: LicenseType,
    pub licensee: String,
    pub email: String,
    pub hardware_id: String,
    pub issued_at: String,
    pub expires_at: Option<String>,
    pub features: Vec<String>,
}

/// A complete license = payload + signature.
///
/// The extra fields below `signature` are unsigned metadata written by the
/// online-activation flow; they never participate in signature verification
/// and stay `None` for offline keys and pre-1.3 `license.json` files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseFile {
    pub payload: LicensePayload,
    /// Hex-encoded Ed25519 signature of the JSON-serialized payload
    pub signature: String,
    /// Activation code used to obtain this license online (None for offline keys).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation_code: Option<String>,
    /// ISO-8601 timestamp of the online activation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activated_at: Option<String>,
}

/// License status returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseStatus {
    pub is_valid: bool,
    pub license_type: LicenseType,
    pub days_remaining: Option<i64>,
    pub licensee: String,
    pub email: String,
    pub features: Vec<String>,
    pub message: String,
    /// Set when the installed license was obtained via online activation —
    /// lets the UI show the code and offer seat-freeing deactivation.
    #[serde(default)]
    pub activation_code: Option<String>,
    /// License expiry (RFC-3339) — None for perpetual licenses and trials
    /// (the trial communicates via days_remaining only).
    #[serde(default)]
    pub expires_at: Option<String>,
    /// True when a license file is installed, in force or not: the UI then
    /// offers Deactivate, which is the way back from a license that no
    /// longer applies.
    #[serde(default)]
    pub has_license: bool,
    /// Why an installed license is not in force: "other_machine",
    /// "invalid" or "expired". `None` when it is, or when there is none.
    #[serde(default)]
    pub problem: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LicenseType {
    Trial,
    Free,
    Professional,
    Enterprise,
    Expired,
}

/// Trial tracking data.
///
/// v2 binds the record to the machine (`hw`), carries a high-water
/// timestamp (`last_seen`) so rolling the system clock back does not give
/// days back, and an integrity tag (`sig`) so the dates cannot simply be
/// edited. A record copied from another machine keeps its start date. This
/// is a fair-play deterrent, not a lock: the salt is in public source,
/// and deleting both copies starts a new trial (nothing offline can stop
/// a determined user, and the Community edition stays free anyway).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialData {
    pub started_at: String,
    pub trial_days: i64,
    #[serde(default)]
    pub hw: String,
    #[serde(default)]
    pub last_seen: String,
    #[serde(default)]
    pub sig: String,
}

// =============================================================================
// Hardware ID
// =============================================================================

/// The hardware ID of this machine, as shown to the user and sent when a
/// license is activated: a hash of the operating system's own machine
/// identifier (Windows `MachineGuid`, macOS `IOPlatformUUID`, Linux
/// `/etc/machine-id`). It does not change when the computer is renamed or
/// another user logs in, which the pre-1.9 ID did. Where the OS has no
/// identifier the pre-1.9 ID is used.
pub fn get_hardware_id() -> String {
    static ID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    ID.get_or_init(|| machine_uid().map(|u| hardware_id_from_uid(&u)).unwrap_or_else(legacy_hardware_id))
        .clone()
}

/// Every ID this machine answers to: the current one and the pre-1.9 one,
/// so licenses, offline keys and trials bound before 1.9 keep working.
pub fn hardware_ids() -> Vec<String> {
    let mut ids = vec![get_hardware_id()];
    let legacy = legacy_hardware_id();
    if !ids.contains(&legacy) {
        ids.push(legacy);
    }
    ids
}

/// True when a license or trial bound to `id` belongs here (an empty ID
/// binds to no machine).
pub fn is_this_machine(id: &str) -> bool {
    id.is_empty() || hardware_ids().iter().any(|h| h == id)
}

fn hardware_id_from_uid(uid: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"bridgelab-hwid-v2\0");
    h.update(uid.trim().to_ascii_lowercase().as_bytes());
    let d = h.finalize();
    format!("BL-{}", hex::encode(&d[..8]).to_ascii_uppercase())
}

/// The operating system's identifier for this installation.
fn machine_uid() -> Option<String> {
    let found = machine_uid_raw()?;
    let found = found.trim().to_string();
    (!found.is_empty() && found.chars().any(|c| c.is_ascii_alphanumeric() && c != '0')).then_some(found)
}

/// `MachineGuid`, read through the registry API. Up to 1.9.0 it came from
/// the output of `reg.exe`, which a "Prevent access to registry editing
/// tools" policy blocks; the silent fallback to the pre-1.9 ID then made
/// the licence read as bound to another computer. The string is the same
/// either way (see [`registry_string`]), so the hardware ID does not change.
#[cfg(target_os = "windows")]
fn machine_uid_raw() -> Option<String> {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, KEY_WOW64_64KEY};
    // The 64-bit view: a 32-bit process would otherwise read the WOW64
    // copy, which does not exist.
    let key = winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(r"SOFTWARE\Microsoft\Cryptography", KEY_QUERY_VALUE | KEY_WOW64_64KEY)
        .ok()?;
    let value: String = key.get_value("MachineGuid").ok()?;
    Some(registry_string(&value))
}

/// A REG_SZ value as `reg query` printed it: without the terminating NULs
/// the API may leave on the string.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn registry_string(value: &str) -> String {
    value.trim_end_matches('\0').to_string()
}

#[cfg(target_os = "macos")]
fn machine_uid_raw() -> Option<String> {
    let out = std::process::Command::new("/usr/sbin/ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| l.contains("\"IOPlatformUUID\""))
        .and_then(|l| l.split('"').nth(3).map(str::to_string))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn machine_uid_raw() -> Option<String> {
    ["/etc/machine-id", "/var/lib/dbus/machine-id"]
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok().filter(|s| !s.trim().is_empty()))
}

/// The pre-1.9 hardware ID: host name, OS, architecture and user name
/// through SipHash-1-3 with zero keys. That is what std's `DefaultHasher`
/// computes today, but std documents it may change, so it is spelled out
/// here: a Rust upgrade must not unbind every existing license.
pub fn legacy_hardware_id() -> String {
    let host = hostname::get().ok().map(|h| h.to_string_lossy().into_owned());
    let user = std::env::var("USERNAME").or_else(|_| std::env::var("USER")).ok();
    legacy_hash(host.as_deref(), std::env::consts::OS, std::env::consts::ARCH, user.as_deref())
}

fn legacy_hash(host: Option<&str>, os: &str, arch: &str, user: Option<&str>) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = siphasher::sip::SipHasher13::new_with_keys(0, 0);
    if let Some(h) = host {
        h.hash(&mut hasher);
    }
    os.hash(&mut hasher);
    arch.hash(&mut hasher);
    if let Some(u) = user {
        u.hash(&mut hasher);
    }
    format!("BL-{:016X}", hasher.finish())
}

// =============================================================================
// File paths
// =============================================================================

/// Where license.json and trial.json live. On Windows that is the local
/// (non-roaming) AppData: a license bound to one machine must not travel
/// to the next PC with a roaming profile and overwrite that PC's own.
fn license_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    let base = dirs::data_local_dir();
    #[cfg(not(target_os = "windows"))]
    let base = dirs::data_dir();
    let dir = base.ok_or_else(|| "Could not determine data directory".to_string())?.join("BridgeLab");
    #[cfg(target_os = "windows")]
    migrate_from_roaming(&dir);
    Ok(dir)
}

/// Before 1.9 the Windows files were in the roaming AppData. Move them
/// once, but only when they belong to this machine; a roaming copy from
/// another PC is left alone (deleting it would delete it there too).
#[cfg(target_os = "windows")]
fn migrate_from_roaming(local: &std::path::Path) {
    static DONE: std::sync::Once = std::sync::Once::new();
    DONE.call_once(|| {
        let Some(roaming) = dirs::data_dir().map(|d| d.join("BridgeLab")) else { return };
        if roaming == local { return; }
        for name in ["license.json", "trial.json"] {
            let (from, to) = (roaming.join(name), local.join(name));
            if to.exists() { continue; }
            let Ok(text) = std::fs::read_to_string(&from) else { continue };
            let ours = match name {
                "license.json" => serde_json::from_str::<LicenseFile>(&text)
                    .is_ok_and(|l| is_this_machine(&l.payload.hardware_id)),
                _ => serde_json::from_str::<TrialData>(&text)
                    .is_ok_and(|t| is_this_machine(&t.hw)),
            };
            if ours && std::fs::create_dir_all(local).is_ok() && std::fs::write(&to, &text).is_ok() {
                let _ = std::fs::remove_file(&from);
            }
        }
    });
}

fn license_file_path() -> Result<PathBuf, String> {
    Ok(license_dir()?.join("license.json"))
}

fn trial_file_path() -> Result<PathBuf, String> {
    Ok(license_dir()?.join("trial.json"))
}

/// Redundant copy of the trial record in a second base directory, so
/// deleting `trial.json` (or its whole folder) alone does not restart the
/// trial. macOS and Linux use the cache dir. On Windows the cache dir *is*
/// the local AppData that holds trial.json, so the copy goes to the
/// roaming AppData (where the database already is).
fn trial_marker_path() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    let dir = dirs::data_dir();
    #[cfg(not(target_os = "windows"))]
    let dir = dirs::cache_dir();
    let dir = dir.ok_or_else(|| "Could not determine the trial marker directory".to_string())?;
    Ok(dir.join("BridgeLab").join(".bl-state.json"))
}

/// Every place the trial record is kept: trial.json, the marker, and on
/// Windows the marker's place up to 1.8.1 (the cache dir, i.e. the local
/// AppData), still read and kept so an upgrade never restarts or shortens
/// a running trial.
fn trial_paths() -> Vec<PathBuf> {
    #[allow(unused_mut)]
    let mut paths: Vec<PathBuf> = [trial_file_path().ok(), trial_marker_path().ok()].into_iter().flatten().collect();
    #[cfg(target_os = "windows")]
    if let Some(old) = dirs::cache_dir().map(|d| d.join("BridgeLab").join(".bl-state.json")) {
        if !paths.contains(&old) {
            paths.push(old);
        }
    }
    paths
}

// =============================================================================
// License verification (Ed25519 signature check)
// =============================================================================

/// Verify an Ed25519 signature on a license payload.
fn verify_signature(payload: &LicensePayload, signature_hex: &str) -> bool {
    verify_signature_with(PUBLIC_KEY_HEX, payload, signature_hex)
}

/// Signature verification against an explicit public key. Exists so the
/// client/server signing contract (compact JSON, struct field order) can be
/// exercised in CI with a throw-away test key pair — see the
/// `server_signed_license` fixture test.
pub fn verify_signature_with(
    public_key_hex: &str,
    payload: &LicensePayload,
    signature_hex: &str,
) -> bool {
    // Reject if no public key has been configured (shipping placeholder = no valid licenses)
    if public_key_hex == "PLACEHOLDER_GENERATE_WITH_CLI" {
        return false;
    }

    let pub_bytes = match hex::decode(public_key_hex) {
        Ok(b) if b.len() == PUBLIC_KEY_LENGTH => b,
        _ => return false,
    };

    let pub_key = match VerifyingKey::from_bytes(
        pub_bytes.as_slice().try_into().unwrap_or(&[0u8; PUBLIC_KEY_LENGTH])
    ) {
        Ok(k) => k,
        Err(_) => return false,
    };

    let sig_bytes = match hex::decode(signature_hex) {
        Ok(b) if b.len() == SIGNATURE_LENGTH => b,
        _ => return false,
    };

    let signature = match Signature::from_bytes(
        sig_bytes.as_slice().try_into().unwrap_or(&[0u8; SIGNATURE_LENGTH])
    ) {
        sig => sig,
    };

    let payload_json = match serde_json::to_string(payload) {
        Ok(j) => j,
        Err(_) => return false,
    };

    pub_key.verify(payload_json.as_bytes(), &signature).is_ok()
}

// =============================================================================
// License persistence
// =============================================================================

pub fn load_license() -> Option<LicenseFile> {
    let path = license_file_path().ok()?;
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn save_license(license: &LicenseFile) -> Result<(), String> {
    let path = license_file_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(license)
        .map_err(|e| format!("Serialize failed: {}", e))?;
    std::fs::write(path, json).map_err(|e| format!("Write failed: {}", e))
}

/// Put a license the server has revoked aside instead of deleting it: the
/// app falls back to Community, and the file is still there (as
/// `license.revoked.json`, replacing an older one) if support needs to
/// look at it. Only the license activated with `code` is moved — the user
/// may have activated another one while the check was in flight — and
/// returns whether it was. The trial is ended too, so a revoked license
/// does not fall back to the Pro trial of a young installation.
pub fn set_aside_revoked_license(code: &str) -> Result<bool, String> {
    let Some(current) = load_license() else { return Ok(false) };
    if current.activation_code.as_deref() != Some(code) {
        return Ok(false);
    }
    let path = license_file_path()?;
    let archive = path.with_file_name("license.revoked.json");
    // rename() does not replace an existing target on Windows.
    if archive.exists() {
        std::fs::remove_file(&archive).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&path, &archive).map_err(|e| e.to_string())?;
    end_trial();
    Ok(true)
}

/// End the trial now: move its start back so its full length is already
/// used up. The record stays a valid, signed one, and an earlier start is
/// what the loader keeps when the two copies disagree.
fn end_trial() {
    let trial = expire_trial(load_or_init_trial());
    persist_trial(&trial);
}

fn expire_trial(mut trial: TrialData) -> TrialData {
    let now = chrono::Utc::now();
    let spent = now - chrono::Duration::days(trial.trial_days.max(1));
    let started = chrono::DateTime::parse_from_rfc3339(&trial.started_at)
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or(now);
    if started > spent {
        trial.started_at = spent.to_rfc3339();
    }
    trial.last_seen = now.to_rfc3339();
    sign_trial(&mut trial);
    trial
}

pub fn remove_license() -> Result<(), String> {
    let path = license_file_path()?;
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// =============================================================================
// Trial management
// =============================================================================

const TRIAL_DAYS: i64 = 14;
/// Duration written by pre-1.3 versions — the legacy-migration check must
/// match what the OLD code actually wrote, not the current duration.
const LEGACY_TRIAL_DAYS: i64 = 7;

/// Salt for the trial integrity tag. Embedded in the binary: raises the
/// bar from "edit a JSON file" to "reverse engineer the executable".
const TRIAL_SIG_SALT: &[u8] = &[
    0x42, 0x4c, 0x54, 0x32, 0x9f, 0x4e, 0x7c, 0x11,
    0xd2, 0xa6, 0x4b, 0x08, 0x5e, 0x31, 0xc7, 0xe9,
];

fn trial_sig(started_at: &str, trial_days: i64, hw: &str, last_seen: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(TRIAL_SIG_SALT);
    for part in [started_at, hw, last_seen] {
        h.update(part.as_bytes());
        h.update([0x1f]);
    }
    h.update(trial_days.to_le_bytes());
    hex::encode(&h.finalize())
}

fn sign_trial(trial: &mut TrialData) {
    trial.sig = trial_sig(&trial.started_at, trial.trial_days, &trial.hw, &trial.last_seen);
}

/// A trial record is genuine when its tag matches and its duration was not
/// inflated. Which machine it is bound to is checked separately: a record
/// bound to another ID (the pre-1.9 one after a rename, or a profile
/// copied from another PC) keeps its start date and is rebound, so it
/// neither gains days nor loses them.
fn trial_sig_ok(trial: &TrialData) -> bool {
    !trial.sig.is_empty()
        && trial.trial_days > 0
        && trial.trial_days <= TRIAL_DAYS
        && chrono::DateTime::parse_from_rfc3339(&trial.started_at).is_ok()
        && trial.sig == trial_sig(&trial.started_at, trial.trial_days, &trial.hw, &trial.last_seen)
}

#[cfg(test)]
fn trial_is_authentic(trial: &TrialData, hw: &str) -> bool {
    trial_sig_ok(trial) && trial.hw == hw
}

/// Pre-hardening `trial.json` files had only `started_at` + `trial_days`.
/// Accept them (so honest mid-trial users keep their remaining days) only
/// when they look exactly like what the old code wrote and do not claim a
/// start in the future.
fn is_plausible_legacy(trial: &TrialData) -> bool {
    trial.sig.is_empty()
        && trial.hw.is_empty()
        && trial.last_seen.is_empty()
        && trial.trial_days == LEGACY_TRIAL_DAYS
        && chrono::DateTime::parse_from_rfc3339(&trial.started_at)
            .map(|d| d.with_timezone(&chrono::Utc) <= chrono::Utc::now() + chrono::Duration::hours(1))
            .unwrap_or(false)
}

pub fn load_or_init_trial() -> TrialData {
    let hw = get_hardware_id();
    let now = chrono::Utc::now();

    let mut candidates: Vec<TrialData> = Vec::new();
    let mut tampered = false;
    let mut migrated = false;

    let mut missing = false;
    for path in trial_paths() {
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => {
                missing = true;
                continue;
            }
        };
        match serde_json::from_str::<TrialData>(&content) {
            Ok(mut t) if trial_sig_ok(&t) => {
                if t.hw != hw {
                    t.hw = hw.clone();
                    sign_trial(&mut t);
                    migrated = true;
                }
                candidates.push(t);
            }
            Ok(mut t) if is_plausible_legacy(&t) => {
                t.hw = hw.clone();
                t.last_seen = now.to_rfc3339();
                sign_trial(&mut t);
                migrated = true;
                candidates.push(t);
            }
            _ => tampered = true,
        }
    }

    let created = candidates.is_empty();

    // When both copies survive, the least generous (earliest start) wins.
    let mut trial = match candidates.into_iter().min_by_key(|t| {
        chrono::DateTime::parse_from_rfc3339(&t.started_at)
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or(now)
    }) {
        Some(t) => t,
        None if tampered => {
            // A record existed but failed verification: fail closed to an
            // already-expired trial instead of granting a fresh one.
            let mut t = TrialData {
                started_at: (now - chrono::Duration::days(TRIAL_DAYS + 1)).to_rfc3339(),
                trial_days: TRIAL_DAYS,
                hw: hw.clone(),
                last_seen: now.to_rfc3339(),
                sig: String::new(),
            };
            sign_trial(&mut t);
            t
        }
        None => new_trial(&hw),
    };

    // Advance the monotonic high-water mark (throttled to hourly so gated
    // IPC calls don't rewrite the files on every invocation).
    let should_advance = match chrono::DateTime::parse_from_rfc3339(&trial.last_seen) {
        Ok(seen) => now > seen.with_timezone(&chrono::Utc) + chrono::Duration::hours(1),
        Err(_) => true,
    };
    if should_advance {
        trial.last_seen = now.to_rfc3339();
        sign_trial(&mut trial);
    }
    // Write only when the record actually changed. A missing copy (e.g. an
    // unwritable cache dir, or a deleted trial.json) is repaired on the
    // hourly advance rather than on every call — reads always take the
    // earliest surviving copy, so enforcement never depends on an
    // immediate rewrite.
    // A copy that is missing (the marker's new place after an upgrade, a
    // deleted folder) is restored at once, but only once per run: an
    // unwritable place must not cost a write on every call.
    static RESTORED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    let restore = missing && !created && !RESTORED.swap(true, std::sync::atomic::Ordering::SeqCst);
    if should_advance || tampered || migrated || created || restore {
        persist_trial(&trial);
    }
    trial
}

fn new_trial(hw: &str) -> TrialData {
    let now = chrono::Utc::now().to_rfc3339();
    let mut trial = TrialData {
        started_at: now.clone(),
        trial_days: TRIAL_DAYS,
        hw: hw.to_string(),
        last_seen: now,
        sig: String::new(),
    };
    sign_trial(&mut trial);
    trial
}

/// Best-effort write to both locations; a single surviving copy is enough
/// for the next load to restore the other.
fn persist_trial(trial: &TrialData) {
    let json = match serde_json::to_string_pretty(trial) {
        Ok(j) => j,
        Err(_) => return,
    };
    for path in trial_paths() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&path, &json).ok();
    }
}

pub fn trial_days_remaining(trial: &TrialData) -> i64 {
    let started = match chrono::DateTime::parse_from_rfc3339(&trial.started_at) {
        Ok(d) => d.with_timezone(&chrono::Utc),
        // Unparseable start = tampered record: fail closed, never fail open.
        Err(_) => return 0,
    };

    // If the clock was rolled back past the last time the app ran, count
    // from the high-water mark instead of the (rewound) wall clock.
    let mut now = chrono::Utc::now();
    if let Ok(seen) = chrono::DateTime::parse_from_rfc3339(&trial.last_seen) {
        let seen = seen.with_timezone(&chrono::Utc);
        if seen > now {
            now = seen;
        }
    }

    let expires = started + chrono::Duration::days(trial.trial_days);
    days_remaining_ceil(expires - now)
}

/// Whole days left in a remaining duration, rounded UP: a 14-day trial
/// started a minute ago has 14 days left, not 13 (`num_days` truncates).
/// Anything at or past expiry is 0.
pub fn days_remaining_ceil(remaining: chrono::Duration) -> i64 {
    let secs = remaining.num_seconds();
    if secs <= 0 { 0 } else { (secs + 86_399) / 86_400 }
}

// =============================================================================
// Activate from license key (Base64-encoded JSON)
// =============================================================================

/// Decode a Base64 JSON license key, verify signature and hardware binding —
/// WITHOUT saving. The online-activation flow uses this to attach unsigned
/// metadata (activation code, timestamp) before persisting.
pub fn parse_and_verify_key(key: &str) -> Result<LicenseFile, String> {
    // Try Base64 decode
    let decoded = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        key.trim(),
    ).map_err(|_| {
        // Fallback: try as a simple format key BL-TYPE-CODE
        return format!("Invalid key format");
    })?;

    let license: LicenseFile = serde_json::from_slice(&decoded)
        .map_err(|e| format!("Invalid license data: {}", e))?;

    // Verify signature
    if !verify_signature(&license.payload, &license.signature) {
        return Err("License signature verification failed".to_string());
    }

    // Verify hardware
    if !is_this_machine(&license.payload.hardware_id) {
        return Err(format!(
            "License is bound to a different machine. Expected: {}, Got: {}",
            license.payload.hardware_id,
            get_hardware_id()
        ));
    }

    Ok(license)
}

/// Activate a license from a key string (offline path).
/// The key is a Base64-encoded JSON LicenseFile.
pub fn activate_from_key(key: &str) -> Result<LicenseFile, String> {
    let license = parse_and_verify_key(key)?;
    save_license(&license)?;
    Ok(license)
}

/// Simple key activation (BL-TYPE-CODE format).
/// Only available in debug builds; release builds require a signed license.
#[cfg(debug_assertions)]
pub fn activate_simple_key(key: &str, licensee: &str, email: &str) -> Result<LicenseFile, String> {
    let parts: Vec<&str> = key.split('-').collect();
    if parts.len() < 3 || parts[0] != "BL" {
        return Err("Invalid key format. Expected: BL-FREE/PRO/ENT-{code}".into());
    }

    let license_type = match parts[1] {
        "FREE" => LicenseType::Free,
        "PRO" => LicenseType::Professional,
        "ENT" => LicenseType::Enterprise,
        _ => return Err("Unknown license type. Use FREE, PRO, or ENT.".into()),
    };

    let key_body = parts[2..].join("-");
    if key_body.len() < 8 {
        return Err("License key too short (minimum 8 characters)".into());
    }

    let features = feature_gate::available_features_for_type(&license_type);

    let expires_at = match license_type {
        LicenseType::Free => None,
        _ => Some((chrono::Utc::now() + chrono::Duration::days(365)).to_rfc3339()),
    };

    let license = LicenseFile {
        payload: LicensePayload {
            license_type,
            licensee: licensee.to_string(),
            email: email.to_string(),
            hardware_id: get_hardware_id(),
            issued_at: chrono::Utc::now().to_rfc3339(),
            expires_at,
            features,
        },
        signature: "dev-mode-no-signature".to_string(),
        activation_code: None,
        activated_at: None,
    };

    save_license(&license)?;
    Ok(license)
}

// =============================================================================
// Check current status
// =============================================================================

pub fn check_license_status() -> LicenseStatus {
    if let Some(license) = load_license() {
        // Hardware check
        if !is_this_machine(&license.payload.hardware_id) {
            return LicenseStatus {
                is_valid: false,
                license_type: LicenseType::Expired,
                days_remaining: None,
                licensee: license.payload.licensee,
                email: license.payload.email,
                features: vec![],
                message: "License is bound to a different machine".into(),
                activation_code: license.activation_code.clone(),
                expires_at: license.payload.expires_at.clone(),
                has_license: true,
                problem: Some("other_machine".into()),
            };
        }

        // Signature check (always enforced in release builds).
        // In debug builds the simple-key activation flow writes a sentinel
        // signature `"dev-mode-no-signature"` — accept it so that `BL-PRO-XXX`
        // activations remain usable during local development.
        #[cfg(debug_assertions)]
        let skip_sig_check = license.signature == "dev-mode-no-signature";
        #[cfg(not(debug_assertions))]
        let skip_sig_check = false;

        if !skip_sig_check && !verify_signature(&license.payload, &license.signature) {
            return LicenseStatus {
                is_valid: false,
                license_type: LicenseType::Expired,
                days_remaining: None,
                licensee: license.payload.licensee,
                email: license.payload.email,
                features: vec![],
                message: "License signature is invalid".into(),
                activation_code: license.activation_code.clone(),
                expires_at: None,
                has_license: true,
                problem: Some("invalid".into()),
            };
        }

        // Expiration check
        if let Some(ref expires) = license.payload.expires_at {
            if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(expires) {
                let remaining = exp.with_timezone(&chrono::Utc) - chrono::Utc::now();
                let days = days_remaining_ceil(remaining);
                // Expired exactly at the expiry instant (num_days() used to
                // truncate, granting up to one extra day past expiry).
                if remaining.num_seconds() < 0 {
                    return LicenseStatus {
                        is_valid: false,
                        license_type: LicenseType::Expired,
                        days_remaining: Some(0),
                        licensee: license.payload.licensee,
                        email: license.payload.email,
                        features: vec![],
                        message: "License has expired".into(),
                        activation_code: license.activation_code.clone(),
                        expires_at: license.payload.expires_at.clone(),
                        has_license: true,
                        problem: Some("expired".into()),
                    };
                }
                return LicenseStatus {
                    is_valid: true,
                    license_type: license.payload.license_type,
                    days_remaining: Some(days),
                    licensee: license.payload.licensee,
                    email: license.payload.email,
                    features: license.payload.features,
                    message: format!("{} days remaining", days),
                    activation_code: license.activation_code.clone(),
                    expires_at: license.payload.expires_at.clone(),
                    has_license: true,
                    problem: None,
                };
            }
        }

        // No expiration (Free license)
        return LicenseStatus {
            is_valid: true,
            license_type: license.payload.license_type,
            days_remaining: None,
            licensee: license.payload.licensee,
            email: license.payload.email,
            features: license.payload.features,
            message: "License is valid".into(),
            activation_code: license.activation_code.clone(),
            expires_at: None,
            has_license: true,
            problem: None,
        };
    }

    // No license - check trial
    let trial = load_or_init_trial();
    let days = trial_days_remaining(&trial);

    if days > 0 {
        LicenseStatus {
            is_valid: true,
            license_type: LicenseType::Trial,
            days_remaining: Some(days),
            licensee: String::new(),
            email: String::new(),
            features: feature_gate::available_features_for_type(&LicenseType::Professional),
            message: format!("Trial: {} days remaining", days),
            activation_code: None,
            expires_at: None,
            has_license: false,
            problem: None,
        }
    } else {
        // Trial expired → fall back to Community (Free) tier, not zero features
        LicenseStatus {
            is_valid: true,
            license_type: LicenseType::Free,
            days_remaining: None,
            licensee: String::new(),
            email: String::new(),
            features: feature_gate::available_features_for_type(&LicenseType::Free),
            message: "Trial expired. Community features are still available.".into(),
            activation_code: None,
            expires_at: None,
            has_license: false,
            problem: None,
        }
    }
}

// Hex encode/decode helpers (avoid adding hex crate dependency)
mod hex {
    pub fn decode(s: &str) -> Result<Vec<u8>, ()> {
        if s.len() % 2 != 0 { return Err(()); }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
            .collect()
    }

    #[allow(dead_code)]
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pre-1.9 ID of host DESKTOP-ABC1234, user mrossi, windows/x86_64,
    /// computed with the 1.8 binary's hasher.
    const LEGACY_KAT: &str = "BL-E0B47550E6AF6245";

    #[test]
    fn test_hardware_id_stable() {
        let id1 = get_hardware_id();
        let id2 = get_hardware_id();
        assert_eq!(id1, id2);
        assert!(id1.starts_with("BL-"));
    }

    #[test]
    fn test_simple_key_free() {
        let license = activate_simple_key("BL-FREE-ABCD1234EFGH", "Test", "test@test.com").unwrap();
        assert_eq!(license.payload.license_type, LicenseType::Free);
        // Cleanup
        remove_license().ok();
    }

    #[test]
    fn test_simple_key_invalid() {
        assert!(activate_simple_key("INVALID", "", "").is_err());
        assert!(activate_simple_key("BL-FREE-short", "", "").is_err());
    }

    fn make_trial(started_at: String, trial_days: i64) -> TrialData {
        let mut t = TrialData {
            started_at,
            trial_days,
            hw: get_hardware_id(),
            last_seen: chrono::Utc::now().to_rfc3339(),
            sig: String::new(),
        };
        sign_trial(&mut t);
        t
    }

    #[test]
    fn an_ended_trial_has_no_days_left_and_stays_authentic() {
        let t = make_trial(chrono::Utc::now().to_rfc3339(), TRIAL_DAYS);
        assert!(trial_days_remaining(&t) > 0);
        let ended = expire_trial(t);
        assert_eq!(trial_days_remaining(&ended), 0);
        assert!(trial_sig_ok(&ended), "the ended trial is re-signed, not treated as tampered");
        // An already expired trial keeps its (earlier) start.
        let old_start = (chrono::Utc::now() - chrono::Duration::days(40)).to_rfc3339();
        let old = expire_trial(make_trial(old_start.clone(), TRIAL_DAYS));
        assert_eq!(old.started_at, old_start);
    }

    #[test]
    fn test_trial_days() {
        let trial = make_trial(chrono::Utc::now().to_rfc3339(), 7);
        let days = trial_days_remaining(&trial);
        assert!(days >= 6 && days <= 7, "expected 6-7 days remaining, got {}", days);
    }

    #[test]
    fn test_fresh_trial_reports_full_duration() {
        // A trial that started seconds ago must show the advertised 14 days,
        // not 13 (truncation regression seen in the trial banner).
        let trial = make_trial(chrono::Utc::now().to_rfc3339(), TRIAL_DAYS);
        assert_eq!(trial_days_remaining(&trial), TRIAL_DAYS);
    }

    #[test]
    fn test_days_remaining_ceil() {
        use chrono::Duration;
        assert_eq!(days_remaining_ceil(Duration::seconds(1)), 1);
        assert_eq!(days_remaining_ceil(Duration::hours(23)), 1);
        assert_eq!(days_remaining_ceil(Duration::hours(25)), 2);
        assert_eq!(days_remaining_ceil(Duration::days(14) - Duration::seconds(30)), 14);
        assert_eq!(days_remaining_ceil(Duration::zero()), 0);
        assert_eq!(days_remaining_ceil(Duration::seconds(-5)), 0);
    }

    #[test]
    fn test_signed_trial_is_authentic() {
        // Both the current duration (14) and pre-1.3 signed records (7)
        // must verify.
        for days in [TRIAL_DAYS, LEGACY_TRIAL_DAYS] {
            let trial = make_trial(chrono::Utc::now().to_rfc3339(), days);
            assert!(trial_is_authentic(&trial, &get_hardware_id()), "days={}", days);
        }
        // Anything above the current duration is rejected even if re-signed.
        let over = make_trial(chrono::Utc::now().to_rfc3339(), TRIAL_DAYS + 1);
        assert!(!trial_is_authentic(&over, &get_hardware_id()));
    }

    #[test]
    fn test_edited_fields_break_authenticity() {
        let hw = get_hardware_id();
        let base = make_trial(chrono::Utc::now().to_rfc3339(), 7);

        // Editing the start date without re-signing
        let mut edited = base.clone();
        edited.started_at = (chrono::Utc::now() + chrono::Duration::days(300)).to_rfc3339();
        assert!(!trial_is_authentic(&edited, &hw));

        // Inflating the duration — even re-signed, >TRIAL_DAYS is rejected
        let mut inflated = base.clone();
        inflated.trial_days = 999_999;
        sign_trial(&mut inflated);
        assert!(!trial_is_authentic(&inflated, &hw));

        // Record copied from a different machine
        let mut foreign = base.clone();
        foreign.hw = "BL-0000000000000000".into();
        sign_trial(&mut foreign);
        assert!(!trial_is_authentic(&foreign, &hw));

        // v1-style record with no tag at all
        let bare = TrialData {
            started_at: chrono::Utc::now().to_rfc3339(),
            trial_days: 7,
            hw: String::new(),
            last_seen: String::new(),
            sig: String::new(),
        };
        assert!(!trial_is_authentic(&bare, &hw));
    }

    #[test]
    fn test_future_start_is_genuine_but_recounted_on_load() {
        // A start in the future is what a clock set ahead on the first
        // launch leaves behind: the record is genuine and must not be
        // rewritten as expired. The days left are counted from the
        // high-water mark (`last_seen`), so turning the clock back gains
        // nothing.
        let hw = get_hardware_id();
        let trial = make_trial((chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339(), 7);
        assert!(trial_is_authentic(&trial, &hw));

        // Clock ahead on the first launch (start = last seen = +2 days),
        // then corrected: the full trial is still there.
        let ahead = (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339();
        let mut first = make_trial(ahead.clone(), 14);
        first.last_seen = ahead;
        sign_trial(&mut first);
        assert_eq!(trial_days_remaining(&first), 14);
        // A trial well under way does not come back by turning the clock
        // back before its start: the high-water mark still counts.
        let mut used = make_trial((chrono::Utc::now() - chrono::Duration::days(10)).to_rfc3339(), 14);
        used.last_seen = (chrono::Utc::now() + chrono::Duration::days(20)).to_rfc3339();
        sign_trial(&mut used);
        assert_eq!(trial_days_remaining(&used), 0);
    }

    #[test]
    fn the_legacy_hardware_id_is_frozen() {
        // Same as std's DefaultHasher today...
        use std::hash::{Hash, Hasher};
        let mut std_hasher = std::collections::hash_map::DefaultHasher::new();
        for part in ["DESKTOP-ABC1234", "windows", "x86_64", "mrossi"] {
            part.hash(&mut std_hasher);
        }
        let ours = legacy_hash(Some("DESKTOP-ABC1234"), "windows", "x86_64", Some("mrossi"));
        assert_eq!(ours, format!("BL-{:016X}", std_hasher.finish()));
        // ...and pinned, so a change in std cannot move it.
        assert_eq!(ours, LEGACY_KAT);
    }

    #[test]
    fn this_machine_answers_to_its_current_and_legacy_ids() {
        assert!(is_this_machine(&get_hardware_id()));
        assert!(is_this_machine(&legacy_hardware_id()));
        assert!(is_this_machine(""));
        assert!(!is_this_machine("BL-0000000000000000"));
    }

    #[test]
    fn the_machine_based_id_ignores_case_and_whitespace() {
        let a = hardware_id_from_uid("4C4C4544-0042-3010-8052-B4C04F4B4E32");
        assert_eq!(a, hardware_id_from_uid(" 4c4c4544-0042-3010-8052-b4c04f4b4e32\n"));
        assert!(a.starts_with("BL-") && a.len() == 19);
    }

    #[test]
    fn a_genuine_trial_bound_to_another_id_is_rebound_not_burned() {
        let mut t = make_trial((chrono::Utc::now() - chrono::Duration::days(3)).to_rfc3339(), 14);
        t.hw = "BL-0123456789ABCDEF".into();
        sign_trial(&mut t);
        assert!(trial_sig_ok(&t), "a foreign binding is still a genuine record");
        assert!(!trial_is_authentic(&t, &get_hardware_id()));
    }

    #[test]
    fn test_unparseable_start_fails_closed() {
        // Pre-hardening this fell back to `now` → a never-expiring trial.
        let mut trial = make_trial("not-a-date".into(), 7);
        sign_trial(&mut trial);
        assert_eq!(trial_days_remaining(&trial), 0);
    }

    #[test]
    fn test_clock_rollback_does_not_extend() {
        // last_seen far beyond the wall clock simulates a rolled-back clock:
        // remaining days count from the high-water mark, not from `now`.
        let mut trial = make_trial(chrono::Utc::now().to_rfc3339(), 7);
        trial.last_seen = (chrono::Utc::now() + chrono::Duration::days(20)).to_rfc3339();
        sign_trial(&mut trial);
        assert_eq!(trial_days_remaining(&trial), 0);
    }

    #[test]
    fn test_legacy_plausibility() {
        let legacy = TrialData {
            started_at: (chrono::Utc::now() - chrono::Duration::days(3)).to_rfc3339(),
            trial_days: LEGACY_TRIAL_DAYS,
            hw: String::new(),
            last_seen: String::new(),
            sig: String::new(),
        };
        assert!(is_plausible_legacy(&legacy));

        // Forged legacy with a future start is not migrated
        let mut forged = legacy.clone();
        forged.started_at = (chrono::Utc::now() + chrono::Duration::days(300)).to_rfc3339();
        assert!(!is_plausible_legacy(&forged));

        // Forged legacy with inflated duration is not migrated
        let mut inflated = legacy.clone();
        inflated.trial_days = 9_999;
        assert!(!is_plausible_legacy(&inflated));

        // The old code never wrote 14 — a bare record claiming the NEW
        // duration is a forgery, not a legacy file.
        let mut fake14 = legacy.clone();
        fake14.trial_days = TRIAL_DAYS;
        assert!(!is_plausible_legacy(&fake14));
    }

    #[test]
    fn test_hex_roundtrip() {
        let data = b"hello world";
        let encoded = hex::encode(data);
        let decoded = hex::decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    /// Cross-language signing contract: the license server (Python) must
    /// reproduce byte-for-byte the compact serde JSON of `LicensePayload`
    /// (struct field order, snake_case license_type, null/string expires_at).
    /// The fixture was signed with a throw-away test key pair — if this test
    /// fails after a backend change, the two sides have drifted.
    #[test]
    fn test_server_signing_contract() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/licensing");
        let pub_hex = std::fs::read_to_string(format!("{}/test_public_key.hex", dir))
            .expect("fixture public key")
            .trim()
            .to_string();
        let file_json = std::fs::read_to_string(format!("{}/server_signed_license.json", dir))
            .expect("fixture license");
        let file: LicenseFile = serde_json::from_str(&file_json).expect("fixture parses");

        assert!(
            verify_signature_with(&pub_hex, &file.payload, &file.signature),
            "server-signed fixture failed verification — signing contract drifted"
        );

        // Tampering with any signed field must break verification.
        let mut tampered = file.payload.clone();
        tampered.licensee = "Someone Else".into();
        assert!(!verify_signature_with(&pub_hex, &tampered, &file.signature));
    }

    /// A 1.2.x license.json (no activation metadata) must keep loading, and
    /// the unsigned metadata must round-trip without touching the payload.
    #[test]
    fn test_pre_13_license_file_compat() {
        let legacy = r#"{
            "payload": {
                "license_type": "professional",
                "licensee": "Old Customer",
                "email": "old@customer.it",
                "hardware_id": "",
                "issued_at": "2026-01-01T00:00:00Z",
                "expires_at": null,
                "features": ["core"]
            },
            "signature": "aa"
        }"#;
        let parsed: LicenseFile = serde_json::from_str(legacy).expect("legacy parses");
        assert!(parsed.activation_code.is_none());
        assert!(parsed.activated_at.is_none());

        // Serializing a legacy file must not add the optional fields
        // (skip_serializing_if) — the signed payload bytes stay identical.
        let out = serde_json::to_string(&parsed).unwrap();
        assert!(!out.contains("activation_code"));
        assert!(!out.contains("activated_at"));

        // And a 1.3 file with metadata round-trips.
        let mut online = parsed.clone();
        online.activation_code = Some("BL-PRO-2345-ABCD-WXYZ".into());
        online.activated_at = Some("2026-08-19T10:00:00Z".into());
        let out = serde_json::to_string(&online).unwrap();
        let back: LicenseFile = serde_json::from_str(&out).unwrap();
        assert_eq!(back.activation_code.as_deref(), Some("BL-PRO-2345-ABCD-WXYZ"));
    }

    /// The Windows hardware ID must not change with the move from `reg.exe`
    /// to the registry API: the same MachineGuid gives the same ID, pinned
    /// here, whichever way it was read.
    #[test]
    fn machine_guid_gives_the_same_id_read_either_way() {
        const GUID: &str = "3f2c8a51-7d4e-4b6a-9e0f-1a2b3c4d5e6f";
        // What 1.9.0 parsed: the last word of the `reg query` line.
        let reg_exe = "\r\nHKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Cryptography\r\n    MachineGuid    REG_SZ    3f2c8a51-7d4e-4b6a-9e0f-1a2b3c4d5e6f\r\n\r\n";
        let old = reg_exe.lines().find(|l| l.contains("MachineGuid")).and_then(|l| l.split_whitespace().last()).unwrap();
        let api = registry_string(&format!("{GUID}\0"));
        assert_eq!(old, api);
        let id = hardware_id_from_uid(&api);
        assert_eq!(id, hardware_id_from_uid(old));
        assert_eq!(id, hardware_id_from_uid(&GUID.to_ascii_uppercase()), "case does not matter");
        assert_eq!(id, "BL-D62E79225FA0B2F8", "the ID 1.9.0 derives from this GUID");
    }

    #[test]
    fn the_trial_marker_is_not_next_to_trial_json() {
        let marker = trial_marker_path().unwrap();
        let trial = trial_file_path().unwrap();
        assert_ne!(marker.parent(), trial.parent());
        assert!(trial_paths().contains(&marker) && trial_paths().contains(&trial));
    }
}
