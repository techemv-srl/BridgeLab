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
    use crate::communication::profiles::{history_text, HistoryEntry};
    use crate::licensing::feature_gate;

    feature_gate::require("soap")?;
    tel.bump_mem("soap_sent");

    // Never keep a password or an API key in the history.
    let endpoint = crate::communication::credentials::redact_credentials(&req.endpoint);
    let action = req.action.clone();
    let payload = req.payload.clone();

    let result = crate::pro::soap::send(req).await;

    // A server that answered is not a failed request: its status (or
    // fault) is shown instead.
    let status = if let Some(fault) = &result.fault {
        format!("FAULT {}", fault.code)
    } else if result.status_code > 0 {
        format!("{}", result.status_code)
    } else {
        "FAILED".into()
    };
    let response = match &result.fault {
        Some(f) => format!("{} {}", f.code, f.reason),
        None => result.body.clone().unwrap_or_default(),
    };
    let entry = HistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        profile_name: profile_name.unwrap_or_else(|| endpoint.clone()),
        profile_type: "soap".into(),
        direction: "send".into(),
        content_preview: action,
        status,
        response_time_ms: result.response_time_ms,
        timestamp: chrono::Utc::now().to_rfc3339(),
        ack_code: None,
        target: endpoint,
        size_bytes: payload.len() as u64,
        // The payload, not the envelope: that holds the WS-Security password.
        request: history_text(&payload),
        response: history_text(&response),
    };
    let _ = db.add_history_entry(&entry);

    Ok(result)
}

/// Bytes of the envelope `soap_send` sends for `req`: the panel asks
/// before a send over 10 MB, and the envelope (escaping, a custom template,
/// the WS-* headers) can be many times the payload. Counted without
/// building it, so a template with the payload many times takes no memory
/// for the question. Nothing is sent.
#[cfg(feature = "pro")]
#[tauri::command]
pub async fn soap_envelope_size(req: crate::pro::soap::SoapRequest) -> Result<usize, String> {
    crate::pro::soap::envelope_len(&req)
}

#[cfg(not(feature = "pro"))]
#[tauri::command]
pub async fn soap_envelope_size(_req: serde_json::Value) -> Result<usize, String> {
    Err("The SOAP client is not included in this build.".into())
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

#[cfg(all(test, feature = "pro"))]
mod tests {
    use super::*;
    use crate::pro::soap::{build_envelope, SoapRequest, WsSecurity};

    /// The size the panel asks about is the envelope's, escaping, template
    /// slots and WS-* headers included: 3 MiB of `&` sends 15 MiB.
    #[tokio::test]
    async fn the_envelope_size_is_the_envelope_sent() {
        let base = SoapRequest {
            endpoint: "http://example/ws".into(),
            soap_version: "1.2".into(),
            action: "urn:a".into(),
            payload: "MSH|^~\\&|A|B\nPID|1|<x>\"q\"'s\n".into(),
            envelope_template: None,
            ws_security: None,
            ws_addressing: false,
            timeout_secs: 5,
        };
        let self_closing = "<soapenv:Envelope xmlns:soapenv=\"urn:x\"><soapenv:Header/><soapenv:Body>{payload}</soapenv:Body></soapenv:Envelope>";
        let cases = [
            base.clone(),
            SoapRequest { payload: "\u{feff}<?xml version=\"1.0\"?>\n<ClinicalDocument>é</ClinicalDocument>\n".into(), ..base.clone() },
            SoapRequest { envelope_template: Some(self_closing.into()), ws_addressing: true, ..base.clone() },
            SoapRequest { envelope_template: Some("<E><B>{payload}{payload}{payload}</B></E>".into()), ..base.clone() },
            SoapRequest { ws_security: Some(WsSecurity { username: "u<".into(), password: "p&".into() }), ..base.clone() },
        ];
        for req in cases {
            assert_eq!(soap_envelope_size(req.clone()).await.unwrap(), build_envelope(&req).unwrap().len(), "{:?}", req.envelope_template);
        }

        let req = SoapRequest {
            endpoint: "http://example/ws".into(),
            soap_version: "1.1".into(),
            action: "urn:send".into(),
            payload: "&".repeat(3 * 1024 * 1024),
            envelope_template: Some(
                "<long-prefix-of-a-namespace:Envelope xmlns:long-prefix-of-a-namespace=\"urn:x\"><long-prefix-of-a-namespace:Body>{payload}{payload}</long-prefix-of-a-namespace:Body></long-prefix-of-a-namespace:Envelope>".into(),
            ),
            ws_security: Some(WsSecurity { username: "u".into(), password: "p&ss".into() }),
            ws_addressing: true,
            timeout_secs: 5,
        };
        let size = soap_envelope_size(req.clone()).await.unwrap();
        assert_eq!(size, build_envelope(&req).unwrap().len());
        assert!(size > 30 * 1024 * 1024, "{size}");
    }
}
