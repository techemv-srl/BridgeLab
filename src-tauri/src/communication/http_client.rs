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
