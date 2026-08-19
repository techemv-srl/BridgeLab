# BridgeLab — Online license activation + opt-in telemetry

**Target repo:** `techemv-srl/BridgeLab` (Rust/Tauri 2 + Svelte 5)
**Companion spec (server side):** `AURORA-BACKEND-BRIDGELAB-EXTENSION.md` — the TECHEMV license server (`aurora-backend`, AWS SAM) gains `/api/v1/bridgelab/*` endpoints. This document covers **only the client**.
**Target version:** 1.3.0

---

## 0. Goal and constraints

Today a Pro/Enterprise license is a Base64 JSON `LicenseFile` (Ed25519-signed payload, optionally hardware-bound) that TECHEMV generates by hand and e-mails. That blocks self-serve purchase: the buyer's hardware ID is unknown at checkout time.

New flow (the **offline** flow stays exactly as is, for air-gapped / enterprise sites):

1. Buyer receives a short **activation code** `BL-PRO-XXXX-XXXX-XXXX` (e-mail from the shop / from TECHEMV).
2. In BridgeLab → License dialog, user pastes the code. The app POSTs `{ code, hardwareId, installationId, appVersion, os, arch }` to the license server.
3. Server validates the code (exists, not revoked, seats left), signs a `LicensePayload` with the **same Ed25519 key pair already trusted by the app** (`PUBLIC_KEY_HEX` in `src-tauri/src/licensing/mod.rs`), returns the Base64 `LicenseFile`.
4. The app feeds that string into the existing `activate_from_key()` → verified and stored in `license.json`. From here on **everything is offline, as before**. No heartbeat, no phone-home required for the license to work.

Separately, an **opt-in** (default OFF) anonymous telemetry ping is added, as the landing-page FAQ already promises ("Telemetry is off by default and, when enabled, carries only anonymous usage counters").

Hard constraints:

- Do **not** change `LicensePayload` (its JSON serialization is what gets signed — the server must reproduce it byte-for-byte, see §6).
- `LicenseFile` may gain **new optional fields** outside `payload` (they are not signed).
- Existing keys (`license.json` written by 1.2.x) must keep working untouched.
- Network failures must never break the app. Activation shows a clear error; telemetry fails silently.
- All new UI strings go in all 5 locale files (`src/lib/i18n/{en,it,fr,es,de}.json`).

---

## 1. Configuration

`src-tauri/src/licensing/online.rs` (new):

```rust
/// Base URL of the TECHEMV license server (shared with Aurora).
/// Override for dev/staging with env var BRIDGELAB_LICENSE_SERVER.
pub const DEFAULT_LICENSE_SERVER: &str = "https://license-api.techemv.it/api/v1";

pub fn license_server_base() -> String {
    std::env::var("BRIDGELAB_LICENSE_SERVER")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim_end_matches('/').to_string())
        .unwrap_or_else(|| DEFAULT_LICENSE_SERVER.to_string())
}
```

> The final hostname is decided server-side (custom domain on the existing API Gateway). Until DNS is live, point `BRIDGELAB_LICENSE_SERVER` at the raw `https://<id>.execute-api.eu-south-1.amazonaws.com/prod/api/v1` URL. Keep the constant in one place only.

HTTP client: `reqwest` is already a dependency (`json`, `rustls-tls`). Use a single lazily-built client with `timeout(10s)` for activation and `timeout(5s)` for telemetry, `User-Agent: BridgeLab/<version> (<os>; <arch>)`.

Version string: read from `env!("CARGO_PKG_VERSION")` (matches `tauri.conf.json` via the release script; verify — if `Cargo.toml` version is not kept in sync, use `app.package_info().version` and pass it down).

---

## 2. Activation code format

```
BL-<TIER>-XXXX-XXXX-XXXX
TIER  ∈ { PRO, ENT }
X     ∈ Crockford Base32 alphabet: 0123456789ABCDEFGHJKMNPQRSTVWXYZ  (no I, L, O, U)
```

Detection (case-insensitive, whitespace/dashes tolerant) in **both** frontend and backend:

```rust
pub fn looks_like_activation_code(input: &str) -> bool {
    let s: String = input.trim().to_ascii_uppercase().chars().filter(|c| *c != ' ').collect();
    // BL-PRO-XXXX-XXXX-XXXX  → 22 chars
    let re = regex::Regex::new(r"^BL-(PRO|ENT)-[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$").unwrap();
    re.is_match(&s)
}
pub fn normalize_activation_code(input: &str) -> String { /* uppercase, collapse, re-insert dashes */ }
```

If `regex` is not already a dependency, implement with plain string checks — do not add a crate just for this.

Anything that is **not** an activation code is treated as the legacy Base64 signed key (offline path, unchanged). The debug-only `BL-FREE/PRO/ENT-<code>` simple-key path must keep working in debug builds: check `looks_like_activation_code()` **first** (it requires exactly three 4-char groups, so `BL-PRO-ABCD1234EFGH` still falls through to the simple-key path).

---

## 3. Rust changes

### 3.1 `src-tauri/src/licensing/mod.rs`

1. `LicenseFile` — add optional, unsigned metadata:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseFile {
    pub payload: LicensePayload,
    pub signature: String,
    /// Activation code used to obtain this license online (None for offline keys).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation_code: Option<String>,
    /// ISO-8601 timestamp of the online activation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activated_at: Option<String>,
}
```

   Update the two `LicenseFile { .. }` literals (`activate_simple_key`, tests) with `activation_code: None, activated_at: None`.

2. `activate_from_key()` — unchanged logic, but expose a variant that does not save, so the online path can attach metadata before saving:

```rust
pub fn parse_and_verify_key(key: &str) -> Result<LicenseFile, String>   // decode + signature + hardware check, no save
pub fn activate_from_key(key: &str) -> Result<LicenseFile, String> {    // = parse_and_verify_key + save_license (existing behaviour)
```

3. `LicenseStatus` — add `#[serde(default)] pub activation_code: Option<String>` so the frontend can show "activated with code BL-PRO-…" and know whether online deactivation is possible. Fill it in `check_license_status()` from the loaded file (None for trial/free).

4. Add `pub mod online;` and `pub mod telemetry;` re-export as needed.

### 3.2 `src-tauri/src/licensing/online.rs` (new)

```rust
#[derive(Serialize)]
struct ActivateRequest<'a> {
    code: &'a str,            // normalized BL-PRO-XXXX-XXXX-XXXX
    hardware_id: &'a str,     // licensing::get_hardware_id()
    installation_id: &'a str, // telemetry::installation_id(db)  (random UUID, see §3.3)
    app_version: &'a str,
    os: &'a str,              // std::env::consts::OS
    arch: &'a str,            // std::env::consts::ARCH
    locale: Option<&'a str>,  // UI language if cheaply available, else None
}

#[derive(Deserialize)]
struct ActivateResponse {
    success: bool,
    #[serde(default)] license_key: Option<String>,   // Base64 LicenseFile, present when success
    #[serde(default)] message: Option<String>,       // human-readable, shown verbatim on error
    #[serde(default)] error_code: Option<String>,    // machine code, see table
    #[serde(default)] seats_used: Option<u32>,
    #[serde(default)] seats_total: Option<u32>,
}

pub async fn activate_online(code: &str, installation_id: &str) -> Result<LicenseFile, String>
pub async fn deactivate_online(code: &str, hardware_id: &str) -> Result<(), String>   // best-effort
```

`activate_online`:
- `POST {base}/bridgelab/activate`, JSON body, 10 s timeout.
- HTTP/network error → `Err("Could not reach the license server: <detail>. Check your connection or use an offline key.")`
- `success == false` → `Err(message)` — the server's message is already user-facing. Map `error_code` to i18n only if trivial; otherwise pass through.
- `success == true` → `parse_and_verify_key(license_key)` (this also enforces hardware binding and signature — **never trust the server blindly**), set `activation_code = Some(code)`, `activated_at = Some(now)`, `save_license()`, return.

Server error codes the client should expect (for tests and optional i18n):

| `error_code`         | Meaning                                        |
|----------------------|------------------------------------------------|
| `INVALID_CODE`       | unknown / malformed code                       |
| `REVOKED`            | code revoked (message contains reason)         |
| `EXPIRED`            | license expired                                |
| `NO_SEATS`           | all activations used → message tells how to free one / contact |
| `RATE_LIMITED`       | too many attempts, try later                   |
| `SERVER_ERROR`       | generic                                        |

`deactivate_online`:
- `POST {base}/bridgelab/deactivate` with `{ code, hardware_id }`, 5 s timeout, ignore result except logging. Called from the `deactivate_license` command **before** `remove_license()` when `activation_code` is present. Local removal must proceed even if the server call fails.

### 3.3 `src-tauri/src/licensing/telemetry.rs` (new) — opt-in usage counters

Storage: the existing SQLite `preferences` table via `Database::get_preference/set_preference` (no schema migration needed). Keys:

| key                       | value                                   |
|---------------------------|-----------------------------------------|
| `installation_id`         | UUID v4, created on first read, **never** derived from hardware id |
| `telemetry_enabled`       | `"true"` / `"false"` (default `"false"`) |
| `telemetry_last_sent`     | RFC-3339                                 |
| `telemetry_counters`      | JSON object `{ "messages_parsed": n, ... }` (cumulative since install) |
| `first_run_at`            | RFC-3339, set on first launch            |

Counters (`UsageCounters`, held in Tauri state as `Mutex<HashMap<&'static str, u64>>`, flushed to the preference on each send and on app exit / every 50 increments — cheap either way):

```
messages_parsed        commands::parser::parse_message (HL7)
fhir_parsed            commands::parser::parse_fhir_message
validations_hl7        commands::validation::validate_message
validations_fhir       commands::validation::validate_fhir
fhirpath_evals         commands::parser::evaluate_fhirpath
mllp_sent              commands::communication::mllp_send
mllp_received          mllp listener, per received message
http_requests          commands::communication::http_request
anonymizations         commands::anonymization::anonymize_message
exports                export_as_json + export_as_csv
xsd_exports            commands::schema_export::hl7_schema_export_xsd
batch_runs             batch_validate + batch_anonymize
sessions               +1 per app start
```

Add a one-liner `telemetry::bump(&state, "messages_parsed")` at the top of each of those commands (after argument validation, before the work — a failed parse still counts as an attempt; that's fine). Keep it to a single helper call per command; no other behavioural change.

Payload sent (anonymous by construction — **no** hostname, username, file names, message content, IPs beyond what TCP reveals):

```json
{
  "product": "BRIDGELAB",
  "installation_id": "uuid",
  "app_version": "1.3.0",
  "os": "windows", "os_version": "10.0.22631", "arch": "x86_64",
  "locale": "it",
  "license_type": "trial|free|professional|enterprise|expired",
  "license_code": "BL-PRO-XXXX-XXXX-XXXX",        // ONLY if activated online, else omitted
  "days_since_install": 12,
  "counters": { "messages_parsed": 1234, "...": 0 },
  "active_plugin_packs": 2,
  "timestamp": 1710000000000
}
```

(`os_version` via the `os_info` crate only if it is already present; otherwise omit the field — do not add a dependency for it.)

Sending rules:
- Only if `telemetry_enabled == "true"`.
- At startup (spawned on the tokio runtime from Tauri `setup`, after DB is managed), **at most once per 24 h** (`telemetry_last_sent`). Also exposed as a command for the Settings "Send now" button.
- `POST {base}/bridgelab/telemetry`, 5 s timeout. On HTTP 200 update `telemetry_last_sent`. Any failure: log at debug level, nothing else.
- Response may contain `{ "status": "OK", "revoked": true, "message": "..." }`. If `revoked` is true, store the message in preference `license_server_notice` so the UI can show a non-blocking banner. **Do not delete the local license automatically.**

### 3.4 `src-tauri/src/commands/licensing.rs`

Add / change commands (register all in `lib.rs` `generate_handler!`):

```rust
#[tauri::command]
pub async fn activate_license_online(code: String, db: State<'_, Database>) -> Result<LicenseStatus, String>
    // normalize → online::activate_online(code, installation_id) → check_license_status()

#[tauri::command]
pub async fn deactivate_license(db: State<'_, Database>) -> Result<LicenseStatus, String>
    // if loaded license has activation_code → deactivate_online (best effort), then remove_license()
    // NOTE: becomes async; update ipc/licensing.ts accordingly (signature unchanged on the TS side)

#[tauri::command]
pub fn is_activation_code(input: String) -> bool

#[tauri::command]
pub fn get_telemetry_settings(db) -> TelemetrySettings   // { enabled, installation_id, last_sent, counters_preview }
#[tauri::command]
pub fn set_telemetry_enabled(enabled: bool, db) -> Result<(), String>
#[tauri::command]
pub async fn send_telemetry_now(...) -> Result<String, String>   // returns server message or error text (for the Settings button)
#[tauri::command]
pub fn get_telemetry_preview(...) -> serde_json::Value        // the exact JSON that would be sent (transparency)
```

Keep `activate_license(key, licensee, email)` as is for the offline path. The frontend decides which command to call based on `is_activation_code` (or the TS regex — keep both in sync; the Rust side is authoritative).

### 3.5 `lib.rs` `setup`

After `.manage(db)`: manage `telemetry::UsageCounters::new()`, ensure `installation_id` and `first_run_at` exist, bump `sessions`, then `tauri::async_runtime::spawn(telemetry::maybe_send_on_startup(app_handle))`.

---

## 4. Frontend changes

### 4.1 `src/lib/ipc/licensing.ts`

Add `activateLicenseOnline(code)`, `isActivationCode(input)`, `getTelemetrySettings()`, `setTelemetryEnabled(bool)`, `sendTelemetryNow()`, `getTelemetryPreview()`. Extend `LicenseStatus` with `activation_code: string | null`.

### 4.2 `src/lib/components/licensing/ActivationDialog.svelte`

- Single input (keep the textarea). Placeholder → `act.keyPlaceholder` updated: "Paste your activation code (BL-PRO-…) or an offline license key".
- `handleActivate()`: `if (await isActivationCode(licenseKey))` → `activateLicenseOnline(normalized)` else existing `activateLicense(...)`.
- While an online activation runs show `act.activatingOnline` ("Contacting license server…").
- On `NO_SEATS` / server errors show the server message verbatim under the input (existing `error` slot).
- When `currentStatus.activation_code` is set, show it under the licensee line (`act.activatedWith`: "Activated online with code {code}") and make the Deactivate button label explain that it frees the seat (`act.deactivateOnline`).
- Keep the Hardware ID block + hint (still needed for offline keys), but re-order: hardware ID block goes **below** the input, collapsed under a "Need an offline key?" disclosure (`act.offlineKeyHelp`). The existing `act.contactPrompt` / `act.contactUs` stay inside that disclosure.
- Key preview (`keyPreview` derived) must not throw for activation codes — guard with the regex first.

### 4.3 `src/lib/components/layout/SettingsModal.svelte`

New section **Privacy** (`settings.privacy`):

- Toggle `settings.telemetryEnabled` — "Send anonymous usage statistics". Default off. Help text `settings.telemetryHelp`: "Once a day BridgeLab sends anonymous counters (messages parsed, validations, MLLP/HTTP calls, exports) plus app version, OS and license tier to TECHEMV. No message content, file names, host names or personal data are ever sent."
- Small link/button `settings.telemetryPreview` ("Show what is sent") → opens a read-only `<pre>` with `getTelemetryPreview()`.
- Button `settings.telemetrySendNow` ("Send now") — visible only when enabled; shows result text inline.
- Line `settings.installationId` showing the UUID (so support can correlate if the user asks).

### 4.4 `TrialBanner.svelte` / `AppShell.svelte`

If preference `license_server_notice` is non-empty, show it as a dismissible warning banner (same styling as the trial banner). Dismiss clears the preference.

### 4.5 i18n keys to add (all 5 files)

```
act.keyPlaceholder        (update text)
act.activatingOnline
act.activatedWith          "{code}" placeholder
act.deactivateOnline
act.offlineKeyHelp
act.onlineHint             "Bought a license? Paste the activation code from your confirmation e-mail."
settings.privacy
settings.telemetryEnabled
settings.telemetryHelp
settings.telemetryPreview
settings.telemetrySendNow
settings.telemetrySent     "Sent — thank you."
settings.telemetryFailed   "Could not send: {error}"
settings.installationId
banner.licenseNotice       (generic label, message body comes from server)
```

---

## 5. Docs / landing / changelog (same PR)

- `README.md` → **License Keys** section: describe the activation-code flow (buy → code by e-mail → paste in app → seat activated; 2 activations per seat; deactivate to move machines) and keep "offline keys available for air-gapped sites — contact info@techemv.it". Fix the contradictory license wording at the bottom (`Free for non-commercial use` vs MIT/FAQ): align with the landing ("core MIT, free for commercial use; Pro/Enterprise features are licensed separately").
- `docs/site/index.html` FAQ → "Does it work offline?" add one sentence: online activation needs a single HTTPS call; afterwards the license is verified locally; offline keys on request. Telemetry FAQ already matches — link it to the new Settings → Privacy section.
- `CHANGELOG.md` → 1.3.0: Online activation codes; opt-in anonymous telemetry (default off) with in-app preview; Settings → Privacy; deactivate frees the seat.
- `TEST_PLAN.md` → add cases: online activation happy path; wrong code; revoked; no seats; offline (network down) error path; legacy Base64 key still works; deactivate with network down still removes local license; telemetry off → no request (assert with a local mock server / env override); telemetry on → one request per 24 h; preview matches sent payload; no PHI/hostname/username in payload.

---

## 6. Signature compatibility — READ THIS

`verify_signature()` signs/verifies `serde_json::to_string(&payload)` — compact JSON, **struct field order**, `license_type` as snake_case string, `expires_at` as `null` or string, `features` as array of strings. The server (Python) must produce exactly:

```
{"license_type":"professional","licensee":"ACME Srl","email":"x@y.z","hardware_id":"BL-0123456789ABCDEF","issued_at":"2026-08-19T10:00:00Z","expires_at":null,"features":["core","hl7v2",...]}
```

To make this contract testable in CI, add to `src-tauri/src/licensing/mod.rs`:

```rust
pub fn verify_signature_with(public_key_hex: &str, payload: &LicensePayload, signature_hex: &str) -> bool
// and keep verify_signature() as a thin wrapper using PUBLIC_KEY_HEX
```

and a test that loads `tests/fixtures/licensing/server_signed_license.json` (committed, produced by the backend repo's `scripts/make_test_vector.py` with a **throw-away test key pair** whose public key is stored next to the fixture as `test_public_key.hex`) and asserts `verify_signature_with(test_pub, &file.payload, &file.signature)`. If this test fails after a backend change, the two sides have drifted.

Do **not** change `LicensePayload` field order, names, or types. Ever.

---

## 7. Acceptance checklist

- [ ] `cargo test` green, including the new cross-language vector test.
- [ ] `pnpm check` 0 errors; 5 locale files have every new key (run the existing i18n lint script if present).
- [ ] Fresh install, Pro code → activated, `license.json` has `activation_code`/`activated_at`, features = Pro set.
- [ ] Same code on a 3rd machine → clear `NO_SEATS` message; after Deactivate on machine 1 → machine 3 activates.
- [ ] A 1.2.0 `license.json` (no new fields) loads and validates unchanged.
- [ ] Network off + activation code → error mentions offline keys; app stays usable (trial/free).
- [ ] Telemetry default OFF: zero requests to the server (verify with `BRIDGELAB_LICENSE_SERVER=http://127.0.0.1:9/api/v1` and a packet capture or a tiny local listener).
- [ ] Telemetry ON: exactly one POST per 24 h; "Send now" works; preview JSON == sent JSON; payload contains no hostname / username / message text.
- [ ] Docs, FAQ, CHANGELOG, TEST_PLAN updated; README license wording aligned with landing.
