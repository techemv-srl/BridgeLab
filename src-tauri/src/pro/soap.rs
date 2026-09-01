//! SOAP 1.1/1.2 client for HL7 exchanges over SOAP endpoints (IHE-style
//! middlewares, regional gateways, legacy hospital services).
//!
//! First cut of the Enterprise SOAP feature: envelope building (default
//! template or user-supplied with a `{payload}` placeholder), optional
//! WS-Security UsernameToken (PasswordText) and WS-Addressing headers,
//! HTTP transport, and response parsing (Body extraction + Fault
//! detection for both SOAP versions). WSDL import is a later step.
//!
//! Licensed under the Business Source License 1.1 — see ../LICENSE.

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct SoapRequest {
    pub endpoint: String,
    /// "1.1" or "1.2"
    #[serde(default = "default_version")]
    pub soap_version: String,
    /// SOAPAction (1.1 header / 1.2 content-type action parameter) and,
    /// when WS-Addressing is on, the wsa:Action value.
    #[serde(default)]
    pub action: String,
    /// Inner payload. Content starting with '<' is inserted as-is (it is
    /// the caller's XML); anything else is XML-escaped and wrapped in a
    /// <payload> element so raw HL7 v2 pipe messages travel safely.
    pub payload: String,
    /// Optional custom envelope; `{payload}` is replaced verbatim.
    #[serde(default)]
    pub envelope_template: Option<String>,
    #[serde(default)]
    pub ws_security: Option<WsSecurity>,
    #[serde(default)]
    pub ws_addressing: bool,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_version() -> String { "1.1".into() }
fn default_timeout() -> u64 { 30 }

#[derive(Debug, Clone, Deserialize)]
pub struct WsSecurity {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SoapFault {
    pub code: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SoapResult {
    pub success: bool,
    pub status_code: u16,
    pub fault: Option<SoapFault>,
    /// Inner XML of soap:Body (without the Body element itself).
    pub body: Option<String>,
    pub response_time_ms: u64,
    pub error: Option<String>,
}

const NS_11: &str = "http://schemas.xmlsoap.org/soap/envelope/";
const NS_12: &str = "http://www.w3.org/2003/05/soap-envelope";
const NS_WSSE: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd";
const NS_WSA: &str = "http://www.w3.org/2005/08/addressing";

pub fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Build the full envelope. A user template wins (only `{payload}` is
/// substituted — the user owns the rest); otherwise a standard envelope
/// with the requested optional headers is produced.
pub fn build_envelope(req: &SoapRequest) -> String {
    let payload_xml = if req.payload.trim_start().starts_with('<') {
        req.payload.clone()
    } else {
        format!("<payload>{}</payload>", xml_escape(&req.payload))
    };

    if let Some(tpl) = &req.envelope_template {
        if !tpl.trim().is_empty() {
            return tpl.replace("{payload}", &payload_xml);
        }
    }

    let ns = if req.soap_version == "1.2" { NS_12 } else { NS_11 };
    let mut headers = String::new();
    if let Some(ws) = &req.ws_security {
        headers.push_str(&format!(
            concat!(
                "<wsse:Security xmlns:wsse=\"{ns}\" soap:mustUnderstand=\"1\">",
                "<wsse:UsernameToken>",
                "<wsse:Username>{u}</wsse:Username>",
                "<wsse:Password Type=\"http://docs.oasis-open.org/wss/2004/01/",
                "oasis-200401-wss-username-token-profile-1.0#PasswordText\">{p}</wsse:Password>",
                "</wsse:UsernameToken></wsse:Security>"
            ),
            ns = NS_WSSE,
            u = xml_escape(&ws.username),
            p = xml_escape(&ws.password),
        ));
    }
    if req.ws_addressing {
        headers.push_str(&format!(
            concat!(
                "<wsa:To xmlns:wsa=\"{ns}\">{to}</wsa:To>",
                "<wsa:Action xmlns:wsa=\"{ns}\">{action}</wsa:Action>",
                "<wsa:MessageID xmlns:wsa=\"{ns}\">urn:uuid:{id}</wsa:MessageID>"
            ),
            ns = NS_WSA,
            to = xml_escape(&req.endpoint),
            action = xml_escape(&req.action),
            id = uuid::Uuid::new_v4(),
        ));
    }

    let header_block = if headers.is_empty() {
        String::new()
    } else {
        format!("<soap:Header>{}</soap:Header>", headers)
    };

    format!(
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
            "<soap:Envelope xmlns:soap=\"{ns}\">",
            "{header}",
            "<soap:Body>{body}</soap:Body>",
            "</soap:Envelope>"
        ),
        ns = ns,
        header = header_block,
        body = payload_xml,
    )
}

/// Extract the Fault (if any) and the inner Body XML from a response.
/// Namespace-prefix agnostic: matches on local element names, which keeps
/// it working across soap/soapenv/env/s prefixes and both SOAP versions.
pub fn parse_response(xml: &str) -> (Option<SoapFault>, Option<String>) {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut body_depth: i32 = -1;
    let mut depth: i32 = 0;
    let mut body_start: Option<usize> = None;
    let mut body_inner: Option<String> = None;

    // Fault fields (1.1: faultcode/faultstring; 1.2: Code/Value + Reason/Text)
    let mut in_fault = false;
    let mut fault_code = String::new();
    let mut fault_reason = String::new();
    let mut capture: Option<&'static str> = None;
    let mut in_code = false;
    let mut in_reason = false;

    let mut buf = Vec::new();
    loop {
        let pos = reader.buffer_position() as usize;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local = local_name(e.name().as_ref());
                depth += 1;
                match local.as_str() {
                    "Body" if body_depth < 0 => {
                        body_depth = depth;
                        // inner XML starts right after this tag closes
                        body_start = Some(reader.buffer_position() as usize);
                    }
                    "Fault" => in_fault = true,
                    "faultcode" if in_fault => capture = Some("code"),
                    "faultstring" if in_fault => capture = Some("reason"),
                    "Code" if in_fault => in_code = true,
                    "Reason" if in_fault => in_reason = true,
                    "Value" if in_fault && in_code => capture = Some("code"),
                    "Text" if in_fault && in_reason => capture = Some("reason"),
                    _ => {}
                }
            }
            Ok(Event::Text(t)) => {
                if let Some(which) = capture {
                    let txt = t.unescape().unwrap_or_default().to_string();
                    if which == "code" && fault_code.is_empty() {
                        fault_code = txt.trim().to_string();
                    } else if which == "reason" && fault_reason.is_empty() {
                        fault_reason = txt.trim().to_string();
                    }
                }
            }
            Ok(Event::End(e)) => {
                let local = local_name(e.name().as_ref());
                if local == "Body" && depth == body_depth {
                    if let Some(start) = body_start {
                        body_inner = Some(xml[start..pos].to_string());
                    }
                }
                match local.as_str() {
                    "Fault" => in_fault = false,
                    "Code" => in_code = false,
                    "Reason" => in_reason = false,
                    "faultcode" | "faultstring" | "Value" | "Text" => capture = None,
                    _ => {}
                }
                depth -= 1;
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    let fault = if !fault_code.is_empty() || !fault_reason.is_empty() {
        Some(SoapFault { code: fault_code, reason: fault_reason })
    } else {
        None
    };
    (fault, body_inner.map(|s| s.trim().to_string()))
}

fn local_name(qname: &[u8]) -> String {
    let s = String::from_utf8_lossy(qname);
    s.rsplit(':').next().unwrap_or(&s).to_string()
}

/// Send the request and parse the reply. Transport errors come back inside
/// the result (success:false + error) so the UI shows them inline like the
/// HTTP client does.
pub async fn send(req: SoapRequest) -> SoapResult {
    let envelope = build_envelope(&req);
    let started = std::time::Instant::now();

    let client = reqwest::Client::new();
    let mut builder = client
        .post(&req.endpoint)
        .timeout(Duration::from_secs(req.timeout_secs.clamp(1, 300)))
        .body(envelope);

    if req.soap_version == "1.2" {
        let ct = if req.action.is_empty() {
            "application/soap+xml; charset=utf-8".to_string()
        } else {
            format!("application/soap+xml; charset=utf-8; action=\"{}\"", req.action)
        };
        builder = builder.header("Content-Type", ct);
    } else {
        builder = builder
            .header("Content-Type", "text/xml; charset=utf-8")
            .header("SOAPAction", format!("\"{}\"", req.action));
    }

    match builder.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let elapsed = started.elapsed().as_millis() as u64;
            let text = resp.text().await.unwrap_or_default();
            let (fault, body) = parse_response(&text);
            SoapResult {
                success: fault.is_none() && (200..300).contains(&status),
                status_code: status,
                fault,
                body,
                response_time_ms: elapsed,
                error: None,
            }
        }
        Err(e) => SoapResult {
            success: false,
            status_code: 0,
            fault: None,
            body: None,
            response_time_ms: started.elapsed().as_millis() as u64,
            error: Some(e.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_req() -> SoapRequest {
        SoapRequest {
            endpoint: "http://example/ws".into(),
            soap_version: "1.1".into(),
            action: "urn:sendHl7".into(),
            payload: "MSH|^~\\&|A|B".into(),
            envelope_template: None,
            ws_security: None,
            ws_addressing: false,
            timeout_secs: 5,
        }
    }

    #[test]
    fn test_envelope_escapes_pipe_message() {
        let env = build_envelope(&base_req());
        assert!(env.contains(NS_11));
        assert!(env.contains("<payload>MSH|^~\\&amp;|A|B</payload>"));
        assert!(!env.contains("<soap:Header>"));
    }

    #[test]
    fn test_envelope_xml_payload_passthrough() {
        let mut r = base_req();
        r.payload = "<msg><x>1</x></msg>".into();
        let env = build_envelope(&r);
        assert!(env.contains("<soap:Body><msg><x>1</x></msg></soap:Body>"));
    }

    #[test]
    fn test_envelope_soap12_namespace() {
        let mut r = base_req();
        r.soap_version = "1.2".into();
        assert!(build_envelope(&r).contains(NS_12));
    }

    #[test]
    fn test_envelope_ws_security_and_addressing() {
        let mut r = base_req();
        r.ws_security = Some(WsSecurity { username: "u<a>".into(), password: "p&w".into() });
        r.ws_addressing = true;
        let env = build_envelope(&r);
        assert!(env.contains("<wsse:Username>u&lt;a&gt;</wsse:Username>"));
        assert!(env.contains("p&amp;w"));
        assert!(env.contains("<wsa:Action"));
        assert!(env.contains("urn:uuid:"));
        assert!(env.contains("<soap:Header>"));
    }

    #[test]
    fn test_envelope_template_substitution() {
        let mut r = base_req();
        r.envelope_template = Some("<e><b>{payload}</b></e>".into());
        let env = build_envelope(&r);
        assert_eq!(env, "<e><b><payload>MSH|^~\\&amp;|A|B</payload></b></e>");
    }

    #[test]
    fn test_parse_fault_11() {
        let xml = r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
            <soap:Body><soap:Fault><faultcode>soap:Client</faultcode>
            <faultstring>Bad payload</faultstring></soap:Fault></soap:Body></soap:Envelope>"#;
        let (fault, _body) = parse_response(xml);
        let f = fault.expect("fault detected");
        assert_eq!(f.code, "soap:Client");
        assert_eq!(f.reason, "Bad payload");
    }

    #[test]
    fn test_parse_fault_12() {
        let xml = r#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope">
            <env:Body><env:Fault><env:Code><env:Value>env:Sender</env:Value></env:Code>
            <env:Reason><env:Text xml:lang="en">Nope</env:Text></env:Reason>
            </env:Fault></env:Body></env:Envelope>"#;
        let (fault, _body) = parse_response(xml);
        let f = fault.expect("fault detected");
        assert_eq!(f.code, "env:Sender");
        assert_eq!(f.reason, "Nope");
    }

    #[test]
    fn test_parse_body_inner() {
        let xml = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
            <s:Body><ackResponse><code>AA</code></ackResponse></s:Body></s:Envelope>"#;
        let (fault, body) = parse_response(xml);
        assert!(fault.is_none());
        assert_eq!(body.unwrap(), "<ackResponse><code>AA</code></ackResponse>");
    }

    #[tokio::test]
    async fn test_soap_roundtrip() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 65536];
            let n = sock.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            // hyper writes header names lowercased on the wire
            assert!(
                req.to_ascii_lowercase().contains("soapaction:"),
                "SOAPAction header missing in: {req}"
            );
            let body = concat!(
                "<soap:Envelope xmlns:soap=\"http://schemas.xmlsoap.org/soap/envelope/\">",
                "<soap:Body><ok>1</ok></soap:Body></soap:Envelope>"
            );
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            );
            sock.write_all(resp.as_bytes()).await.unwrap();
        });

        let mut r = base_req();
        r.endpoint = format!("http://{}/", addr);
        let out = send(r).await;
        assert!(out.success, "roundtrip failed: {:?}", out.error);
        assert_eq!(out.status_code, 200);
        assert_eq!(out.body.unwrap(), "<ok>1</ok>");
    }

    #[tokio::test]
    async fn test_soap_connection_refused() {
        let mut r = base_req();
        r.endpoint = "http://127.0.0.1:1/".into();
        r.timeout_secs = 2;
        let out = send(r).await;
        assert!(!out.success);
        assert!(out.error.is_some());
    }
}
