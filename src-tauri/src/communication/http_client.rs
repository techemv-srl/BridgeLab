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

/// Send an HTTP request.
pub async fn send_request(
    url: &str,
    method: HttpMethod,
    headers: &HashMap<String, String>,
    body: Option<&str>,
    timeout_secs: u64,
) -> HttpResult {
    let start = Instant::now();

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
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

    // Read body
    let body = match response.text().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResult {
                success: true,
                status_code,
                status_text,
                headers: resp_headers,
                body: String::new(),
                response_time_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("Failed to read response body: {}", e)),
            };
        }
    };

    HttpResult {
        success: status_code >= 200 && status_code < 400,
        status_code,
        status_text,
        headers: resp_headers,
        body,
        response_time_ms: start.elapsed().as_millis() as u64,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// Tiny HTTP/1.1 server: reads a single request, extracts the method +
    /// body, replies with 200 + JSON echo. Enough to exercise the client.
    async fn serve_one(port: u16, expect_body: &'static str) {
        let listener = TcpListener::bind(("127.0.0.1", port)).await.unwrap();
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

    fn pick_free_port() -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    }

    #[tokio::test]
    async fn test_http_get_roundtrip() {
        let port = pick_free_port();
        let server = tokio::spawn(async move { serve_one(port, "").await });
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;

        let headers = HashMap::new();
        let url = format!("http://127.0.0.1:{}/ping", port);
        let res = send_request(&url, HttpMethod::Get, &headers, None, 5).await;
        server.await.unwrap();

        assert!(res.success, "GET failed: {:?}", res.error);
        assert_eq!(res.status_code, 200);
        assert!(res.body.contains("\"method\":\"GET\""),
                "body did not reflect GET method: {}", res.body);
    }

    #[tokio::test]
    async fn test_http_post_with_body_and_header() {
        let port = pick_free_port();
        let payload = "MSH|^~\\&|Sender|Fac|Recv|Fac|20260415||ADT^A01|CTRL|P|2.5";
        let server = tokio::spawn(async move { serve_one(port, "ADT^A01").await });
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/hl7-v2".to_string());
        headers.insert("X-Test".to_string(), "bridgelab".to_string());

        let url = format!("http://127.0.0.1:{}/submit", port);
        let res = send_request(&url, HttpMethod::Post, &headers, Some(payload), 5).await;
        server.await.unwrap();

        assert!(res.success, "POST failed: {:?}", res.error);
        assert_eq!(res.status_code, 200);
        assert!(res.body.contains("\"method\":\"POST\""));
    }

    #[tokio::test]
    async fn test_http_connection_refused() {
        // Pick a port and don't bind it
        let port = pick_free_port();
        let headers = HashMap::new();
        let url = format!("http://127.0.0.1:{}/nobody-home", port);
        let res = send_request(&url, HttpMethod::Get, &headers, None, 2).await;
        assert!(!res.success);
        assert!(res.error.is_some());
    }
}
