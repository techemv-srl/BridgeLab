use serde::Serialize;
use std::collections::HashMap;
use std::time::Instant;

/// Result of an HTTP request.
#[derive(Debug, Clone, Serialize)]
pub struct HttpResult {
    pub success: bool,
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub response_time_ms: u64,
    pub error: Option<String>,
    /// The URL that answered, when redirects were followed to it; `None`
    /// when it is the one requested.
    #[serde(default)]
    pub final_url: Option<String>,
}

/// Largest response body kept: bigger ones are cut here and reported,
/// rather than held whole in memory and pushed over IPC.
const MAX_BODY_BYTES: usize = 50 * 1024 * 1024;

/// Whether a redirect from `from` to `to` is followed automatically: only
/// on the same origin (scheme, host and port), or from http to https on
/// the same host. Anything else is shown as the 3xx it is, so a request
/// with a patient message or an API key never goes to another server, or
/// to plain http, without the user seeing it.
fn follow_redirect(from: &reqwest::Url, to: &reqwest::Url) -> bool {
    let same_host = from.host_str() == to.host_str();
    (same_host && from.scheme() == to.scheme() && from.port_or_known_default() == to.port_or_known_default())
        || (same_host && from.scheme() == "http" && to.scheme() == "https")
}

/// The response body as text, in the charset its Content-Type declares
/// (`charset=ISO-8859-1`…); UTF-8 when it declares none.
fn decode_body(bytes: &[u8], content_type: Option<&str>) -> String {
    let declared = content_type.and_then(|ct| {
        ct.split(';').skip(1).find_map(|param| {
            let (k, v) = param.split_once('=')?;
            k.trim().eq_ignore_ascii_case("charset").then(|| v.trim().trim_matches('"').to_string())
        })
    });
    match declared.and_then(|c| encoding_rs::Encoding::for_label(c.as_bytes())) {
        Some(enc) if enc != encoding_rs::UTF_8 => enc.decode_with_bom_removal(bytes).0.into_owned(),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// Supported HTTP methods.
#[derive(Debug, Clone)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

impl HttpMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(Self::Get),
            "POST" => Some(Self::Post),
            "PUT" => Some(Self::Put),
            "DELETE" => Some(Self::Delete),
            "PATCH" => Some(Self::Patch),
            _ => None,
        }
    }
}

/// Send an HTTP request. `follow_redirects: false` returns the 3xx
/// response as-is (useful to inspect Location headers of integration
/// endpoints); `true` follows up to 10 hops on the same server (see
/// [`follow_redirect`]) and returns the first redirect elsewhere as is.
pub async fn send_request(
    url: &str,
    method: HttpMethod,
    headers: &HashMap<String, String>,
    body: Option<&str>,
    timeout_secs: u64,
    follow_redirects: bool,
) -> HttpResult {
    let start = Instant::now();

    let redirect_policy = if follow_redirects {
        reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 10 {
                return attempt.error("too many redirects");
            }
            match attempt.previous().last() {
                Some(from) if !follow_redirect(from, attempt.url()) => attempt.stop(),
                _ => attempt.follow(),
            }
        })
    } else {
        reqwest::redirect::Policy::none()
    };

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .redirect(redirect_policy)
        .danger_accept_invalid_certs(false)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return HttpResult {
                success: false,
                status_code: 0,
                status_text: String::new(),
                headers: HashMap::new(),
                body: String::new(),
                response_time_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("Failed to create HTTP client: {}", e)),
                final_url: None,
            };
        }
    };

    let mut request = match method {
        HttpMethod::Get => client.get(url),
        HttpMethod::Post => client.post(url),
        HttpMethod::Put => client.put(url),
        HttpMethod::Delete => client.delete(url),
        HttpMethod::Patch => client.patch(url),
    };

    // Add custom headers
    for (key, value) in headers {
        request = request.header(key.as_str(), value.as_str());
    }

    // Add body for methods that support it
    if let Some(body_text) = body {
        request = request.body(body_text.to_string());
    }

    // Send request
    let response = match request.send().await {
        Ok(r) => r,
        Err(e) => {
            let error_msg = if e.is_timeout() {
                "Request timed out".to_string()
            } else if e.is_connect() {
                format!("Connection failed: {}", e)
            } else {
                format!("Request failed: {}", e)
            };
            return HttpResult {
                success: false,
                status_code: 0,
                status_text: String::new(),
                headers: HashMap::new(),
                body: String::new(),
                response_time_ms: start.elapsed().as_millis() as u64,
                error: Some(error_msg),
                final_url: None,
            };
        }
    };

    let status_code = response.status().as_u16();
    let status_text = response.status().canonical_reason().unwrap_or("").to_string();

    // Collect response headers
    let resp_headers: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    // Compared without user:password, which the client moves into an
    // Authorization header: a URL with credentials is not a redirect.
    let final_url = Some(response.url().to_string()).filter(|u| {
        reqwest::Url::parse(url.trim()).map_or(true, |mut req| {
            let _ = req.set_username("");
            let _ = req.set_password(None);
            req.as_str() != u
        })
    });
    let content_type = resp_headers.get("content-type").cloned();

    // Read the body, up to the cap.
    let mut response = response;
    let mut bytes: Vec<u8> = Vec::new();
    let mut error = None;
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                if bytes.len() + chunk.len() > MAX_BODY_BYTES {
                    bytes.extend_from_slice(&chunk[..MAX_BODY_BYTES - bytes.len()]);
                    error = Some(format!(
                        "The response is larger than {} MB: only the first {} MB are shown",
                        MAX_BODY_BYTES >> 20,
                        MAX_BODY_BYTES >> 20
                    ));
                    break;
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(e) => {
                error = Some(format!("Failed to read response body: {}", e));
                break;
            }
        }
    }
    let body = decode_body(&bytes, content_type.as_deref());

    HttpResult {
        success: status_code >= 200 && status_code < 400,
        status_code,
        status_text,
        headers: resp_headers,
        body,
        response_time_ms: start.elapsed().as_millis() as u64,
        error,
        final_url,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// Tiny HTTP/1.1 server: reads a single request, extracts the method +
    /// body, replies with 200 + JSON echo. Enough to exercise the client.
    async fn serve_one(listener: TcpListener, expect_body: &'static str) {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 16 * 1024];
        let n = stream.read(&mut buf).await.unwrap();
        let req = String::from_utf8_lossy(&buf[..n]).to_string();

        // Parse request line (first line)
        let first = req.lines().next().unwrap_or("");
        let method = first.split_whitespace().next().unwrap_or("GET").to_string();

        // Parse body (after blank line)
        let body = req.split("\r\n\r\n").nth(1).unwrap_or("");
        if !expect_body.is_empty() {
            assert!(body.contains(expect_body),
                    "server did not receive expected body '{}', got '{}'",
                    expect_body, body);
        }

        let resp_body = format!("{{\"method\":\"{}\",\"echo\":\"{}\"}}", method, body);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            resp_body.len(), resp_body
        );
        stream.write_all(response.as_bytes()).await.unwrap();
        let _ = stream.flush().await;
    }

    /// A listener on a free port, kept open: picking a port, closing it
    /// and binding it again let a concurrent test or CI job take it in
    /// between ("Address already in use").
    async fn listen_free() -> (TcpListener, u16) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        (listener, port)
    }

    #[tokio::test]
    async fn test_http_get_roundtrip() {
        let (listener, port) = listen_free().await;
        let server = tokio::spawn(async move { serve_one(listener, "").await });

        let headers = HashMap::new();
        let url = format!("http://127.0.0.1:{}/ping", port);
        let res = send_request(&url, HttpMethod::Get, &headers, None, 5, true).await;
        server.await.unwrap();

        assert!(res.success, "GET failed: {:?}", res.error);
        assert_eq!(res.status_code, 200);
        assert!(res.body.contains("\"method\":\"GET\""),
                "body did not reflect GET method: {}", res.body);
    }

    #[tokio::test]
    async fn test_http_post_with_body_and_header() {
        let (listener, port) = listen_free().await;
        let payload = "MSH|^~\\&|Sender|Fac|Recv|Fac|20260415||ADT^A01|CTRL|P|2.5";
        let server = tokio::spawn(async move { serve_one(listener, "ADT^A01").await });

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/hl7-v2".to_string());
        headers.insert("X-Test".to_string(), "bridgelab".to_string());

        let url = format!("http://127.0.0.1:{}/submit", port);
        let res = send_request(&url, HttpMethod::Post, &headers, Some(payload), 5, true).await;
        server.await.unwrap();

        assert!(res.success, "POST failed: {:?}", res.error);
        assert_eq!(res.status_code, 200);
        assert!(res.body.contains("\"method\":\"POST\""));
    }

    #[tokio::test]
    async fn test_http_connection_refused() {
        // A port that was free a moment ago, with nothing listening on it.
        let port = listen_free().await.1;
        let headers = HashMap::new();
        let url = format!("http://127.0.0.1:{}/nobody-home", port);
        let res = send_request(&url, HttpMethod::Get, &headers, None, 2, true).await;
        assert!(!res.success);
        assert!(res.error.is_some());
    }

    #[test]
    fn the_body_is_decoded_in_the_declared_charset() {
        assert_eq!(decode_body(b"M\xfcller", Some("text/plain; charset=ISO-8859-1")), "Müller");
        assert_eq!(decode_body(b"M\xfcller", Some("application/hl7-v2;charset=\"windows-1252\"")), "Müller");
        assert_eq!(decode_body("Müller".as_bytes(), Some("application/json")), "Müller");
        assert_eq!(decode_body("Müller".as_bytes(), None), "Müller");
    }

    /// user:password in the URL is not a redirect to another URL.
    #[tokio::test]
    async fn a_url_with_credentials_is_not_reported_as_redirected() {
        let (listener, port) = listen_free().await;
        let server = tokio::spawn(async move { serve_one(listener, "").await });
        let url = format!("http://bob:pw9@127.0.0.1:{}/creds", port);
        let res = send_request(&url, HttpMethod::Get, &HashMap::new(), None, 5, true).await;
        server.await.unwrap();
        assert!(res.success, "{:?}", res.error);
        assert_eq!(res.final_url, None);
    }

    #[test]
    fn redirects_are_followed_only_on_the_same_server() {
        let u = |s: &str| reqwest::Url::parse(s).unwrap();
        assert!(follow_redirect(&u("http://a:8080/x"), &u("http://a:8080/y")));
        assert!(follow_redirect(&u("http://a/x"), &u("https://a/y")));
        assert!(!follow_redirect(&u("http://a:8080/x"), &u("http://b:8080/y")));
        assert!(!follow_redirect(&u("http://a:8080/x"), &u("http://a:9090/y")));
        assert!(!follow_redirect(&u("https://a/x"), &u("http://a/y")));
    }
}
