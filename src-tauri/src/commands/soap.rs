//! MIT-licensed IPC shim for the Enterprise SOAP client.
//!
//! The command surface (name, parameters) is MIT like the rest of the
//! commands layer; the implementation it delegates to lives in the
//! BUSL-1.1 `crate::pro::soap` module and is only compiled when the
//! `pro` cargo feature is enabled. Community-only builds
//! (`--no-default-features`) keep the same command registered but answer
//! with a clear "not included in this build" error.

use tauri::State;

use crate::database::Database;

#[cfg(feature = "pro")]
#[tauri::command]
pub async fn soap_send(
    req: crate::pro::soap::SoapRequest,
    profile_name: Option<String>,
    db: State<'_, Database>,
    tel: State<'_, crate::licensing::telemetry::UsageCounters>,
) -> Result<crate::pro::soap::SoapResult, String> {
    use crate::communication::profiles::HistoryEntry;
    use crate::licensing::feature_gate;

    feature_gate::require("soap")?;
    tel.bump_mem("soap_sent");

    let endpoint = req.endpoint.clone();
    let action = req.action.clone();
    let preview: String = req.payload.chars().take(100).collect();

    let result = crate::pro::soap::send(req).await;

    let status = if result.success {
        format!("{}", result.status_code)
    } else if let Some(fault) = &result.fault {
        format!("FAULT {}", fault.code)
    } else {
        "FAILED".into()
    };
    let label = if action.is_empty() { endpoint.clone() } else { action };
    let entry = HistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        profile_name: profile_name.unwrap_or(endpoint),
        profile_type: "soap".into(),
        direction: "send".into(),
        content_preview: format!("{} | {}", label, preview),
        status,
        response_time_ms: result.response_time_ms,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    let _ = db.add_history_entry(&entry);

    Ok(result)
}

/// Community-only builds compile without `crate::pro`; the command stays
/// registered so the frontend gets a deterministic error instead of a
/// "command not found" panic from the IPC layer.
#[cfg(not(feature = "pro"))]
#[tauri::command]
pub async fn soap_send(
    _req: serde_json::Value,
    _profile_name: Option<String>,
    _db: State<'_, Database>,
) -> Result<serde_json::Value, String> {
    Err(
        "The SOAP client is not included in this build. \
         Official BridgeLab builds include it for Enterprise licenses."
            .into(),
    )
}
