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
        // XML parsers turn a literal CR into LF (XML 1.0, 2.11), and CR is
        // the HL7 v2 segment terminator: it travels as a character reference.
        .replace('\r', "&#13;")
}

/// The WS-Security and WS-Addressing header blocks the request asks for,
/// empty when it asks for none. `must_understand` is the attribute that
/// marks the Security header, qualified with the envelope's own prefix.
fn header_blocks(req: &SoapRequest, must_understand: &str) -> String {
    let mut headers = String::new();
    if let Some(ws) = &req.ws_security {
        headers.push_str(&format!(
            concat!(
                "<wsse:Security xmlns:wsse=\"{ns}\" {mu}>",
                "<wsse:UsernameToken>",
                "<wsse:Username>{u}</wsse:Username>",
                "<wsse:Password Type=\"http://docs.oasis-open.org/wss/2004/01/",
                "oasis-200401-wss-username-token-profile-1.0#PasswordText\">{p}</wsse:Password>",
                "</wsse:UsernameToken></wsse:Security>"
            ),
            ns = NS_WSSE,
            mu = must_understand,
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
    headers
}

/// One start tag found in a template: where it begins and ends, and its
/// qualified name.
struct Tag<'a> {
    start: usize,
    /// Index just past the closing `>`.
    end: usize,
    name: &'a str,
    self_closing: bool,
}

/// Where the markup construct starting at `start` (a `<`) ends, just past
/// its closing `>`: comments, CDATA and processing instructions end at
/// their own terminator, and a `>` inside a quoted attribute value does
/// not end a tag.
fn markup_end(xml: &str, start: usize) -> Option<usize> {
    let rest = &xml[start..];
    for (open, close) in [("<!--", "-->"), ("<![CDATA[", "]]>"), ("<?", "?>")] {
        if rest.starts_with(open) {
            return rest[open.len()..].find(close).map(|i| start + open.len() + i + close.len());
        }
    }
    let mut quote: Option<char> = None;
    for (i, c) in rest.char_indices().skip(1) {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '>') => return Some(start + i + 1),
            _ => {}
        }
    }
    None
}

/// The start tags of `xml`, in order (end tags, comments, CDATA,
/// processing instructions and declarations skipped).
fn start_tags(xml: &str) -> impl Iterator<Item = Tag<'_>> {
    let mut pos = 0;
    std::iter::from_fn(move || loop {
        let start = pos + xml[pos..].find('<')?;
        let end = markup_end(xml, start)?;
        pos = end;
        let rest = &xml[start + 1..end];
        if rest.starts_with(['/', '?', '!']) {
            continue;
        }
        let name_len = rest.find(|c: char| c.is_whitespace() || c == '/' || c == '>').unwrap_or(rest.len());
        return Some(Tag {
            start,
            end,
            name: &rest[..name_len],
            self_closing: xml[..end].ends_with("/>"),
        });
    })
}

/// Put `headers` into the template's SOAP Header, creating the Header
/// before the Body when the template has none. The template is the
/// user's; when its root is not a SOAP Envelope with a Body, there is no
/// place the receiver would look for the headers, and it is refused.
fn inject_headers(tpl: &str, headers: &str, payload_xml: &str) -> Result<String, String> {
    let refuse = || {
        "The custom envelope template has no SOAP Envelope with a Body, so the WS-Security / \
         WS-Addressing headers cannot be added to it. Add a <soap:Header/> (or at least the \
         Envelope and Body) to the template, or turn those options off."
            .to_string()
    };
    let root = start_tags(tpl).next().ok_or_else(refuse)?;
    let (prefix, local) = match root.name.split_once(':') {
        Some((p, l)) => (Some(p), l),
        None => (None, root.name),
    };
    if local != "Envelope" {
        return Err(refuse());
    }
    let qualified = |l: &str| match prefix {
        Some(p) => format!("{}:{}", p, l),
        None => l.to_string(),
    };
    let (header_name, body_name) = (qualified("Header"), qualified("Body"));
    let (at, insert) = if let Some(h) = start_tags(tpl).find(|t| t.name == header_name) {
        if h.self_closing {
            // <soap:Header/> becomes <soap:Header>…</soap:Header>.
            let open = format!("{}>", tpl[h.start..h.end - 2].trim_end());
            (h.start..h.end, format!("{}{}</{}>", open, headers, header_name))
        } else {
            (h.end..h.end, headers.to_string())
        }
    } else {
        let b = start_tags(tpl).find(|t| t.name == body_name).ok_or_else(refuse)?;
        (b.start..b.start, format!("<{h}>{}</{h}>", headers, h = header_name))
    };
    // {payload} is replaced in the template's own text only: never inside
    // the credentials just added.
    Ok(format!(
        "{}{}{}",
        tpl[..at.start].replace("{payload}", payload_xml),
        insert,
        tpl[at.end..].replace("{payload}", payload_xml)
    ))
}

/// Build the full envelope. With a user template only `{payload}` is
/// substituted (the user owns the rest) and the WS-Security and
/// WS-Addressing headers, when asked for, are added to its SOAP Header;
/// otherwise a standard envelope with those headers is produced.
pub fn build_envelope(req: &SoapRequest) -> Result<String, String> {
    let payload_xml = if req.payload.trim_start().starts_with('<') {
        req.payload.clone()
    } else {
        // HL7 v2 segments end with CR (LF from the editor is converted);
        // xml_escape keeps the CR as &#13;.
        format!("<payload>{}</payload>", xml_escape(&crate::parser::hl7::to_wire_segments(&req.payload)))
    };
    let ns = if req.soap_version == "1.2" { NS_12 } else { NS_11 };

    if let Some(tpl) = &req.envelope_template {
        if !tpl.trim().is_empty() {
            let root_prefix = start_tags(tpl).next().and_then(|t| t.name.split_once(':').map(|(p, _)| p.to_string()));
            let must_understand = match &root_prefix {
                Some(p) => format!("{}:mustUnderstand=\"1\"", p),
                // A default-namespace envelope has no prefix to qualify
                // the attribute with: declare one on the header itself.
                None => format!("xmlns:soapenv=\"{}\" soapenv:mustUnderstand=\"1\"", ns),
            };
            let headers = header_blocks(req, &must_understand);
            if headers.is_empty() {
                return Ok(tpl.replace("{payload}", &payload_xml));
            }
            return inject_headers(tpl, &headers, &payload_xml);
        }
    }

    let headers = header_blocks(req, "soap:mustUnderstand=\"1\"");
    let header_block = if headers.is_empty() {
        String::new()
    } else {
        format!("<soap:Header>{}</soap:Header>", headers)
    };

    Ok(format!(
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
    ))
}

/// Extract the Fault (if any) and the inner Body XML from a response.
/// Namespace-prefix agnostic: matches on local element names, which keeps
/// it working across soap/soapenv/env/s prefixes and both SOAP versions.
pub fn parse_response(xml: &str) -> (Option<SoapFault>, Option<String>) {
    use quick_xml::events::Event;
    use quick_xml::name::ResolveResult;
    use quick_xml::NsReader;

    // Body and Fault are only recognized in the SOAP envelope namespace
    // (either version) — an application payload element that happens to be
    // named "Fault" must not be classified as a protocol fault. Prefix
    // independence comes from real namespace resolution, not from ignoring
    // namespaces.
    fn is_soap_ns(res: &ResolveResult) -> bool {
        matches!(res, ResolveResult::Bound(ns)
            if ns.as_ref() == NS_11.as_bytes() || ns.as_ref() == NS_12.as_bytes())
    }

    let mut reader = NsReader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut body_depth: i32 = -1;
    let mut depth: i32 = 0;
    let mut body_start: Option<usize> = None;
    let mut body_inner: Option<String> = None;

    // Fault fields (1.1: faultcode/faultstring; 1.2: Code/Value + Reason/Text).
    // The Fault element itself must be a direct child of the SOAP Body in the
    // SOAP namespace; its 1.1 subelements are unqualified per spec, so those
    // are matched by local name only while inside a verified Fault.
    let mut in_fault = false;
    let mut fault_code = String::new();
    let mut fault_reason = String::new();
    let mut capture: Option<&'static str> = None;
    let mut captured = String::new();
    let mut in_code = false;
    let mut in_reason = false;

    loop {
        let pos = reader.buffer_position() as usize;
        match reader.read_resolved_event() {
            Ok((res, Event::Start(e))) => {
                let local = e.local_name();
                let local = local.as_ref();
                depth += 1;
                match local {
                    b"Body" if body_depth < 0 && is_soap_ns(&res) => {
                        body_depth = depth;
                        // inner XML starts right after this tag closes
                        body_start = Some(reader.buffer_position() as usize);
                    }
                    b"Fault"
                        if !in_fault
                            && body_depth >= 0
                            && depth == body_depth + 1
                            && is_soap_ns(&res) =>
                    {
                        in_fault = true;
                    }
                    b"faultcode" if in_fault => capture = Some("code"),
                    b"faultstring" if in_fault => capture = Some("reason"),
                    b"Code" if in_fault && is_soap_ns(&res) => in_code = true,
                    b"Reason" if in_fault && is_soap_ns(&res) => in_reason = true,
                    b"Value" if in_fault && in_code => capture = Some("code"),
                    b"Text" if in_fault && in_reason => capture = Some("reason"),
                    _ => {}
                }
            }
            Ok((res, Event::Empty(e))) => {
                // A self-closing <soap:Body/> is a present-but-empty Body:
                // the response is still a valid SOAP envelope.
                if body_depth < 0
                    && body_inner.is_none()
                    && e.local_name().as_ref() == b"Body"
                    && is_soap_ns(&res)
                {
                    body_inner = Some(String::new());
                }
            }
            // quick-xml splits text at entity references: "Bad &amp; wrong"
            // arrives as Text("Bad "), GeneralRef("amp"), Text(" wrong").
            // Everything inside a captured element is accumulated and
            // assigned when the element closes.
            Ok((_, Event::Text(t))) => {
                if capture.is_some() {
                    if let Ok(txt) = t.xml10_content() {
                        captured.push_str(&txt);
                    }
                }
            }
            Ok((_, Event::GeneralRef(r))) => {
                if capture.is_some() {
                    if let Ok(Some(ch)) = r.resolve_char_ref() {
                        captured.push(ch);
                    } else if let Ok(name) = r.decode() {
                        match quick_xml::escape::resolve_predefined_entity(&name) {
                            Some(s) => captured.push_str(s),
                            // An entity this parser cannot expand is kept as written.
                            None => {
                                captured.push('&');
                                captured.push_str(&name);
                                captured.push(';');
                            }
                        }
                    }
                }
            }
            Ok((res, Event::End(e))) => {
                let local = e.local_name();
                let local = local.as_ref();
                if local == b"Body" && depth == body_depth && is_soap_ns(&res) {
                    if let Some(start) = body_start {
                        body_inner = Some(xml[start..pos].to_string());
                    }
                }
                match local {
                    b"Fault" if depth == body_depth + 1 => in_fault = false,
                    b"Code" => in_code = false,
                    b"Reason" => in_reason = false,
                    b"faultcode" | b"faultstring" | b"Value" | b"Text" => {
                        match capture {
                            Some("code") if fault_code.is_empty() => fault_code = captured.trim().to_string(),
                            Some("reason") if fault_reason.is_empty() => fault_reason = captured.trim().to_string(),
                            _ => {}
                        }
                        captured.clear();
                        capture = None;
                    }
                    _ => {}
                }
                depth -= 1;
            }
            Ok((_, Event::Eof)) => break,
            Err(_) => break,
            _ => {}
        }
    }

    let fault = if !fault_code.is_empty() || !fault_reason.is_empty() {
        Some(SoapFault { code: fault_code, reason: fault_reason })
    } else {
        None
    };
    (fault, body_inner.map(|s| s.trim().to_string()))
}

/// Largest SOAP reply read: a bigger one is an error, not a 1 GB string.
const MAX_REPLY_BYTES: usize = 50 * 1024 * 1024;

async fn read_capped(mut resp: reqwest::Response) -> Result<String, String> {
    let mut bytes = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > MAX_REPLY_BYTES {
            return Err(format!("the reply is larger than {} MB", MAX_REPLY_BYTES >> 20));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Send the request and parse the reply. Transport errors come back inside
/// the result (success:false + error) so the UI shows them inline like the
/// HTTP client does.
pub async fn send(req: SoapRequest) -> SoapResult {
    let started = std::time::Instant::now();
    let envelope = match build_envelope(&req) {
        Ok(e) => e,
        Err(e) => {
            return SoapResult {
                success: false,
                status_code: 0,
                fault: None,
                body: None,
                response_time_ms: 0,
                error: Some(e),
            }
        }
    };

    // No redirects: a 307/308 would re-post the envelope, WS-Security
    // password included, to whatever host the Location names. A redirect
    // is shown as the non-SOAP reply it is.
    let client = match reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build() {
        Ok(c) => c,
        Err(_) => reqwest::Client::new(),
    };
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
            match read_capped(resp).await {
                Ok(text) => {
                    // Measured after the body is fully consumed: send()
                    // resolves on headers, and time-to-first-byte alone
                    // undercounts streamed/delayed responses.
                    let elapsed = started.elapsed().as_millis() as u64;
                    let (fault, body) = parse_response(&text);
                    // A reply without a SOAP Body (proxy HTML page, auth
                    // gateway, empty stream) is not a successful SOAP
                    // exchange, whatever the HTTP status says.
                    let is_soap = fault.is_some() || body.is_some();
                    let ok_status = (200..300).contains(&status);
                    let error = if ok_status && !is_soap {
                        Some("Response is not a SOAP envelope (no SOAP Body found in the reply)".to_string())
                    } else {
                        None
                    };
                    SoapResult {
                        success: ok_status && fault.is_none() && is_soap,
                        status_code: status,
                        fault,
                        body,
                        response_time_ms: elapsed,
                        error,
                    }
                }
                Err(e) => SoapResult {
                    success: false,
                    status_code: status,
                    fault: None,
                    body: None,
                    response_time_ms: started.elapsed().as_millis() as u64,
                    error: Some(format!("Failed to read response body: {}", e)),
                },
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
        let env = build_envelope(&base_req()).unwrap();
        assert!(env.contains(NS_11));
        assert!(env.contains("<payload>MSH|^~\\&amp;|A|B</payload>"));
        assert!(!env.contains("<soap:Header>"));
    }

    #[test]
    fn test_envelope_xml_payload_passthrough() {
        let mut r = base_req();
        r.payload = "<msg><x>1</x></msg>".into();
        let env = build_envelope(&r).unwrap();
        assert!(env.contains("<soap:Body><msg><x>1</x></msg></soap:Body>"));
    }

    #[test]
    fn test_envelope_soap12_namespace() {
        let mut r = base_req();
        r.soap_version = "1.2".into();
        assert!(build_envelope(&r).unwrap().contains(NS_12));
    }

    #[test]
    fn test_envelope_ws_security_and_addressing() {
        let mut r = base_req();
        r.ws_security = Some(WsSecurity { username: "u<a>".into(), password: "p&w".into() });
        r.ws_addressing = true;
        let env = build_envelope(&r).unwrap();
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
        let env = build_envelope(&r).unwrap();
        assert_eq!(env, "<e><b><payload>MSH|^~\\&amp;|A|B</payload></b></e>");
    }

    /// A raw HL7 message keeps its CR segment terminators through XML
    /// parsing, and LF from the editor becomes CR first.
    #[test]
    fn hl7_segment_terminators_survive_the_xml() {
        let mut r = base_req();
        r.payload = "MSH|^~\\&|A|B\nPID|1\n".into();
        let env = build_envelope(&r).unwrap();
        assert!(env.contains("<payload>MSH|^~\\&amp;|A|B&#13;PID|1&#13;</payload>"), "{env}");
        assert!(!env.contains('\r') && !env.contains('\n'));
    }

    /// WS-Security and WS-Addressing reach a custom template: into its
    /// Header, or a new Header before its Body, with its own prefix.
    #[test]
    fn a_template_still_gets_the_ws_headers() {
        let mut r = base_req();
        r.ws_security = Some(WsSecurity { username: "{payload}".into(), password: "pw".into() });
        r.ws_addressing = true;

        r.envelope_template = Some(
            "<se:Envelope xmlns:se=\"http://schemas.xmlsoap.org/soap/envelope/\"><se:Header><x:h xmlns:x=\"urn:x\"/></se:Header><se:Body><m>{payload}</m></se:Body></se:Envelope>".into(),
        );
        let env = build_envelope(&r).unwrap();
        assert!(env.contains("<se:Header><wsse:Security"), "{env}");
        assert!(env.contains("se:mustUnderstand=\"1\""));
        assert!(env.contains("<x:h xmlns:x=\"urn:x\"/></se:Header>"));
        assert!(env.contains("<wsa:Action"));
        assert!(env.contains("<wsse:Username>{payload}</wsse:Username>"), "credentials are not substituted");
        assert!(env.contains("<m><payload>MSH|"));

        r.envelope_template = Some("<s:Envelope xmlns:s=\"x\"><s:Header /><s:Body>{payload}</s:Body></s:Envelope>".into());
        let env = build_envelope(&r).unwrap();
        assert!(env.starts_with("<s:Envelope xmlns:s=\"x\"><s:Header><wsse:Security"), "{env}");
        assert!(env.contains("</wsa:MessageID></s:Header><s:Body>"));

        r.envelope_template = Some("<?xml version=\"1.0\"?><!-- c --><Envelope xmlns=\"http://schemas.xmlsoap.org/soap/envelope/\"><Body>{payload}</Body></Envelope>".into());
        let env = build_envelope(&r).unwrap();
        assert!(env.contains("<Header><wsse:Security"), "{env}");
        assert!(env.contains("xmlns:soapenv=\"http://schemas.xmlsoap.org/soap/envelope/\" soapenv:mustUnderstand=\"1\""));
        assert!(env.contains("</Header><Body>"));

        // A '>' inside an attribute value, a comment or CDATA does not end
        // a tag or hide one.
        r.envelope_template = Some(
            "<s:Envelope xmlns:s=\"x\" a='1>2'><!-- <s:Header> --><s:Header data=\"a>b\" c='d>'><h/></s:Header><s:Body><![CDATA[<s:Header>]]>{payload}</s:Body></s:Envelope>".into(),
        );
        let env = build_envelope(&r).unwrap();
        assert!(env.contains("<s:Header data=\"a>b\" c='d>'><wsse:Security"), "{env}");
        assert!(env.contains("<!-- <s:Header> -->") && env.contains("<![CDATA[<s:Header>]]>"));
        assert_eq!(env.matches("<wsse:Security").count(), 1);

        // No Envelope to put them in: refused, never sent without them.
        r.envelope_template = Some("<e><b>{payload}</b></e>".into());
        assert!(build_envelope(&r).unwrap_err().contains("WS-Security"));
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

    /// Entity references in the fault text are resolved (quick-xml 0.41
    /// hands text over as written; the parser decodes it).
    #[test]
    fn fault_text_entities_are_resolved() {
        let xml = r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
            <soap:Body><soap:Fault><faultcode>soap:Server</faultcode>
            <faultstring>Bad &amp; wrong &lt;here&gt;</faultstring></soap:Fault></soap:Body></soap:Envelope>"#;
        let (fault, _) = parse_response(xml);
        assert_eq!(fault.expect("fault detected").reason, "Bad & wrong <here>");
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
    fn test_parse_ignores_app_fault_element() {
        // An application payload element named "Fault" outside the SOAP
        // namespace must NOT be classified as a protocol fault.
        let xml = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
            <s:Body><result xmlns="urn:app"><Fault><Code><Value>APP-1</Value></Code>
            <Reason><Text>domain error payload</Text></Reason></Fault></result>
            </s:Body></s:Envelope>"#;
        let (fault, body) = parse_response(xml);
        assert!(fault.is_none(), "app-level Fault misread as SOAP Fault");
        assert!(body.unwrap().contains("APP-1"));
    }

    #[test]
    fn test_parse_default_namespace_fault() {
        // Unprefixed envelope bound via a default namespace still counts.
        let xml = r#"<Envelope xmlns="http://www.w3.org/2003/05/soap-envelope">
            <Body><Fault><Code><Value>Sender</Value></Code>
            <Reason><Text>bad</Text></Reason></Fault></Body></Envelope>"#;
        let (fault, _) = parse_response(xml);
        let f = fault.expect("default-ns fault detected");
        assert_eq!(f.code, "Sender");
        assert_eq!(f.reason, "bad");
    }

    #[test]
    fn test_parse_empty_self_closing_body() {
        let xml = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body/></s:Envelope>"#;
        let (fault, body) = parse_response(xml);
        assert!(fault.is_none());
        assert_eq!(body, Some(String::new()));
    }

    #[test]
    fn test_parse_non_soap_response() {
        let (fault, body) = parse_response("<html><body>login required</body></html>");
        assert!(fault.is_none());
        assert!(body.is_none(), "HTML <body> must not count as a SOAP Body");
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
    async fn test_soap_roundtrip_non_soap_response() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 65536];
            let _ = sock.read(&mut buf).await.unwrap();
            let body = "<html><body>Please sign in</body></html>";
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            );
            sock.write_all(resp.as_bytes()).await.unwrap();
        });

        let mut r = base_req();
        r.endpoint = format!("http://{}/", addr);
        let out = send(r).await;
        assert!(!out.success, "a 2xx HTML page must not be a SOAP success");
        assert_eq!(out.status_code, 200);
        assert!(out.error.unwrap().contains("not a SOAP envelope"));
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
