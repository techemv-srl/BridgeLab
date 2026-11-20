//! With a proxy configured, the HTTP tool reaches a server on this computer
//! directly and everything else through the proxy.
//!
//! The proxy comes from the environment, which reqwest reads with the same
//! matcher it uses for the Windows and macOS settings. A test binary of its
//! own, with a single test: the variables are set before any other thread
//! could read the environment.

use std::collections::HashMap;

use bridgelab_lib::communication::http_client::{send_request, HttpMethod};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Answer one request with `body`; returns the request line received.
async fn answer_once(listener: TcpListener, body: &'static str) -> String {
    let (mut stream, _) = listener.accept().await.unwrap();
    let mut buf = vec![0u8; 16 * 1024];
    let n = stream.read(&mut buf).await.unwrap();
    let request_line = String::from_utf8_lossy(&buf[..n]).lines().next().unwrap_or("").to_string();
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).await.unwrap();
    let _ = stream.flush().await;
    request_line
}

#[test]
fn loopback_requests_bypass_the_proxy_and_others_use_it() {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let proxy = rt.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
    let proxy_url = format!("http://{}", proxy.local_addr().unwrap());
    for name in ["no_proxy", "NO_PROXY", "http_proxy", "https_proxy", "all_proxy", "REQUEST_METHOD"] {
        std::env::remove_var(name);
    }
    for name in ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY"] {
        std::env::set_var(name, &proxy_url);
    }

    rt.block_on(async move {
        for host in ["127.0.0.1", "localhost"] {
            let server = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = server.local_addr().unwrap().port();
            let served = tokio::spawn(answer_once(server, "direct"));
            let url = format!("http://{host}:{port}/fhir/metadata");
            let res = send_request(&url, HttpMethod::Get, &HashMap::new(), None, 5, false).await;
            assert_eq!(res.body, "direct", "{url}: {:?}", res.error);
            assert_eq!(served.await.unwrap(), "GET /fhir/metadata HTTP/1.1");
        }

        // A host that does not exist: only the proxy can answer for it.
        let proxied = tokio::spawn(answer_once(proxy, "proxy"));
        let res = send_request("http://fhir.example.invalid/x", HttpMethod::Get, &HashMap::new(), None, 5, false).await;
        assert_eq!(res.body, "proxy", "{:?}", res.error);
        assert_eq!(proxied.await.unwrap(), "GET http://fhir.example.invalid/x HTTP/1.1");
    });
}
