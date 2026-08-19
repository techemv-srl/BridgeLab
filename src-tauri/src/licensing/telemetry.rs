//! Opt-in (default OFF) anonymous usage telemetry.
//!
//! Counts feature usage locally (SQLite preferences, cumulative since
//! install) and — only when the user enables it in Settings → Privacy —
//! POSTs an anonymous summary to the TECHEMV license server at most once
//! per 24 h. The payload contains no message content, file names, host
//! names or user names; the installation id is a random UUID, never
//! derived from the hardware id. Every failure is silent: telemetry must
//! never affect the app.

use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Manager;

use crate::database::Database;
use crate::plugins::PluginRegistry;

use super::online::license_server_base;

// Preference keys (existing `preferences` table — no schema migration).
const PREF_INSTALLATION_ID: &str = "installation_id";
const PREF_ENABLED: &str = "telemetry_enabled";
const PREF_LAST_SENT: &str = "telemetry_last_sent";
const PREF_COUNTERS: &str = "telemetry_counters";
const PREF_FIRST_RUN: &str = "first_run_at";
pub const PREF_SERVER_NOTICE: &str = "license_server_notice";

/// How many in-memory increments may accumulate before `bump` also flushes
/// to the preferences DB (cheap protection against losing counts on exit).
const FLUSH_EVERY: u64 = 50;

/// In-memory usage counters, managed as Tauri state. Cloning shares the
/// underlying map (Arc), so the MLLP listener task can hold its own handle.
#[derive(Clone)]
pub struct UsageCounters {
    inner: Arc<Mutex<HashMap<&'static str, u64>>>,
    pending: Arc<AtomicU64>,
}

impl UsageCounters {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            pending: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Increment a counter in memory only (used where no DB handle is
    /// available, e.g. inside the MLLP listener task). Durable on the next
    /// flush from any other call site or the periodic flusher.
    pub fn bump_mem(&self, key: &'static str) {
        if let Ok(mut m) = self.inner.lock() {
            *m.entry(key).or_insert(0) += 1;
        }
        self.pending.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment a counter; every [`FLUSH_EVERY`] increments the in-memory
    /// deltas are merged into the cumulative preference row.
    pub fn bump(&self, key: &'static str, db: &Database) {
        self.bump_mem(key);
        if self.pending.load(Ordering::Relaxed) >= FLUSH_EVERY {
            self.flush(db);
        }
    }

    /// Take the accumulated deltas, leaving the map empty.
    fn drain(&self) -> HashMap<&'static str, u64> {
        self.pending.store(0, Ordering::Relaxed);
        self.inner
            .lock()
            .map(|mut m| std::mem::take(&mut *m))
            .unwrap_or_default()
    }

    /// Merge in-memory deltas into the cumulative `telemetry_counters`
    /// preference. Best-effort: on failure the deltas are restored so no
    /// count is lost.
    pub fn flush(&self, db: &Database) {
        let deltas = self.drain();
        if deltas.is_empty() {
            return;
        }
        let mut cumulative = read_cumulative(db);
        for (k, v) in &deltas {
            *cumulative.entry((*k).to_string()).or_insert(0) += v;
        }
        let json = match serde_json::to_string(&cumulative) {
            Ok(j) => j,
            Err(_) => return,
        };
        if db.set_preference(PREF_COUNTERS, &json).is_err() {
            // Put the deltas back; they'll ride the next flush.
            if let Ok(mut m) = self.inner.lock() {
                for (k, v) in deltas {
                    *m.entry(k).or_insert(0) += v;
                }
            }
        }
    }
}

fn read_cumulative(db: &Database) -> HashMap<String, u64> {
    db.get_preference(PREF_COUNTERS)
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Cumulative counters with any in-memory deltas folded in first.
pub fn cumulative_counters(db: &Database, counters: &UsageCounters) -> HashMap<String, u64> {
    counters.flush(db);
    read_cumulative(db)
}

// =============================================================================
// Identity / settings
// =============================================================================

/// Random installation UUID, created on first read. Deliberately NOT derived
/// from the hardware id — it identifies an install, not a machine or person.
pub fn installation_id(db: &Database) -> String {
    if let Ok(Some(id)) = db.get_preference(PREF_INSTALLATION_ID) {
        if !id.is_empty() {
            return id;
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    let _ = db.set_preference(PREF_INSTALLATION_ID, &id);
    id
}

pub fn ensure_first_run(db: &Database) {
    match db.get_preference(PREF_FIRST_RUN) {
        Ok(Some(v)) if !v.is_empty() => {}
        _ => {
            let _ = db.set_preference(PREF_FIRST_RUN, &chrono::Utc::now().to_rfc3339());
        }
    }
}

pub fn is_enabled(db: &Database) -> bool {
    matches!(db.get_preference(PREF_ENABLED), Ok(Some(v)) if v == "true")
}

pub fn set_enabled(db: &Database, enabled: bool) -> Result<(), String> {
    db.set_preference(PREF_ENABLED, if enabled { "true" } else { "false" })
}

#[derive(Serialize)]
pub struct TelemetrySettings {
    pub enabled: bool,
    pub installation_id: String,
    pub last_sent: Option<String>,
    pub counters: HashMap<String, u64>,
}

pub fn settings(db: &Database, counters: &UsageCounters) -> TelemetrySettings {
    TelemetrySettings {
        enabled: is_enabled(db),
        installation_id: installation_id(db),
        last_sent: db.get_preference(PREF_LAST_SENT).ok().flatten(),
        counters: cumulative_counters(db, counters),
    }
}

// =============================================================================
// Payload
// =============================================================================

fn days_since_install(db: &Database) -> i64 {
    db.get_preference(PREF_FIRST_RUN)
        .ok()
        .flatten()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| (chrono::Utc::now() - d.with_timezone(&chrono::Utc)).num_days().max(0))
        .unwrap_or(0)
}

/// The exact JSON that gets sent — also surfaced verbatim in Settings →
/// Privacy ("Show what is sent") so the promise is verifiable by the user.
pub fn build_payload(app: &tauri::AppHandle) -> serde_json::Value {
    let db = app.state::<Database>();
    let counters = app.state::<UsageCounters>();
    let plugins = app.state::<PluginRegistry>();

    let status = super::check_license_status();
    let license_type = serde_json::to_value(&status.license_type)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "free".into());

    let mut payload = serde_json::json!({
        "product": "BRIDGELAB",
        "installation_id": installation_id(&db),
        "app_version": app.package_info().version.to_string(),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "locale": db.get_preference("language").ok().flatten().unwrap_or_else(|| "en".into()),
        "license_type": license_type,
        "days_since_install": days_since_install(&db),
        "counters": cumulative_counters(&db, &counters),
        "active_plugin_packs": plugins.enabled_count(),
        "timestamp": chrono::Utc::now().timestamp_millis(),
    });
    // Only when the license was activated online — never for offline keys.
    if let Some(code) = status.activation_code {
        payload["license_code"] = serde_json::Value::String(code);
    }
    payload
}

// =============================================================================
// Sending
// =============================================================================

#[derive(serde::Deserialize, Default)]
struct TelemetryResponse {
    #[serde(default)]
    #[allow(dead_code)]
    status: Option<String>,
    #[serde(default)]
    revoked: Option<bool>,
    #[serde(default)]
    message: Option<String>,
}

fn last_sent_within_24h(db: &Database) -> bool {
    db.get_preference(PREF_LAST_SENT)
        .ok()
        .flatten()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| chrono::Utc::now() - d.with_timezone(&chrono::Utc) < chrono::Duration::hours(24))
        .unwrap_or(false)
}

/// POST the payload. Returns the server's message and the payload that was
/// actually transmitted (so the Settings "Send now" button can show the
/// exact bytes sent); the periodic path ignores both.
pub async fn send(app: &tauri::AppHandle) -> Result<(String, serde_json::Value), String> {
    let payload = build_payload(app);
    let url = format!("{}/bridgelab/telemetry", license_server_base());
    let version = app.package_info().version.to_string();

    let resp = reqwest::Client::new()
        .post(&url)
        .timeout(Duration::from_secs(5))
        .header("User-Agent", format!("BridgeLab/{}", version))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("{}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Server returned HTTP {}", resp.status().as_u16()));
    }

    let db = app.state::<Database>();
    let _ = db.set_preference(PREF_LAST_SENT, &chrono::Utc::now().to_rfc3339());

    let body: TelemetryResponse = resp.json().await.unwrap_or_default();
    if body.revoked == Some(true) {
        // Never delete the local license automatically — store the server's
        // notice so the UI can show a non-blocking, dismissible banner.
        let notice = body
            .message
            .clone()
            .unwrap_or_else(|| "Your license was revoked. Contact info@techemv.it.".into());
        let _ = db.set_preference(PREF_SERVER_NOTICE, &notice);
    }
    Ok((body.message.unwrap_or_else(|| "OK".into()), payload))
}

/// Startup task: establish identity, count the session, then keep the
/// counters durable and — when telemetry is enabled — send at most one
/// report per 24 h for as long as the app stays open. Every network
/// failure is logged at debug level and otherwise ignored.
pub async fn maybe_send_on_startup(app: tauri::AppHandle) {
    {
        let db = app.state::<Database>();
        let counters = app.state::<UsageCounters>();
        ensure_first_run(&db);
        let _ = installation_id(&db);
        counters.bump("sessions", &db);
        counters.flush(&db);
    }

    loop {
        {
            let db = app.state::<Database>();
            if is_enabled(&db) && !last_sent_within_24h(&db) {
                if let Err(e) = send(&app).await {
                    #[cfg(debug_assertions)]
                    eprintln!("[telemetry] send failed: {}", e);
                    let _ = e;
                }
            }
        }
        // Periodic durability flush + daily re-check for long-lived sessions.
        tokio::time::sleep(Duration::from_secs(600)).await;
        let db = app.state::<Database>();
        let counters = app.state::<UsageCounters>();
        counters.flush(&db);
    }
}
