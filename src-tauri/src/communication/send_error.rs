//! Why an HTTP request got no answer, read from the error and its sources.
//!
//! reqwest's message for every failed connection is "error sending request
//! for url (…)": the reason is further down the error's chain of sources,
//! which its message leaves out. A certificate this computer does not
//! trust, a closed port and a name that does not resolve all read the
//! same, word for word, although each calls for something else (installing
//! the company's root certificate, starting the server, fixing the URL).

use serde::Serialize;
use std::error::Error;
use std::io;

/// What stopped a request, as far as its error tells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendFailure {
    /// The certificate does not chain to a root this computer trusts: a
    /// self-signed one, or a TLS-inspecting proxy whose root certificate is
    /// not in the operating system's store.
    CertificateUntrusted,
    /// Expired, or not valid yet (a wrong clock).
    CertificateExpired,
    /// Issued for another host name than the one in the URL.
    CertificateWrongHost,
    /// The TLS handshake failed for another reason (a peer that does not
    /// speak TLS on that port, no common protocol version…).
    TlsFailed,
    ConnectionRefused,
    /// The host name does not resolve.
    HostNotFound,
    TimedOut,
    /// The proxy answered the tunnel request with a refusal.
    ProxyRefused,
    /// The proxy wants a login (407).
    ProxyAuthRequired,
    /// The connection was closed before an answer came.
    ConnectionReset,
    Other,
}

impl SendFailure {
    /// The code the frontend shows in the user's language (`comm.sendError.<code>`).
    pub fn code(self) -> &'static str {
        match self {
            Self::CertificateUntrusted => "certUntrusted",
            Self::CertificateExpired => "certExpired",
            Self::CertificateWrongHost => "certWrongHost",
            Self::TlsFailed => "tlsFailed",
            Self::ConnectionRefused => "refused",
            Self::HostNotFound => "hostNotFound",
            Self::TimedOut => "timedOut",
            Self::ProxyRefused => "proxyRefused",
            Self::ProxyAuthRequired => "proxyAuth",
            Self::ConnectionReset => "reset",
            Self::Other => "other",
        }
    }

    /// The same in English, for the backend's own message (history, logs,
    /// a frontend without the translation).
    pub fn explanation(self) -> &'static str {
        match self {
            Self::CertificateUntrusted => "the server's certificate is not trusted: it was not issued by a certificate authority this computer trusts, as happens with a self-signed certificate or with a proxy that inspects TLS traffic using a root certificate that is not installed",
            Self::CertificateExpired => "the server's certificate has expired or is not valid yet; check this computer's date and time",
            Self::CertificateWrongHost => "the server's certificate was issued for another host name than the one in the URL",
            Self::TlsFailed => "a secure (TLS) connection could not be set up: the server may not use HTTPS on this port",
            Self::ConnectionRefused => "connection refused: nothing is listening on this port, or a firewall turned the connection away",
            Self::HostNotFound => "the host name was not found in DNS: check the URL and the network connection",
            Self::TimedOut => "request timed out: the server did not answer in time",
            Self::ProxyRefused => "the proxy refused to connect to this server",
            Self::ProxyAuthRequired => "the proxy asks for a login: 407 Proxy Authentication Required",
            Self::ConnectionReset => "the connection was closed before the server answered",
            Self::Other => "the request failed",
        }
    }

    /// A refused certificate, whatever the reason.
    pub fn is_certificate(self) -> bool {
        matches!(self, Self::CertificateUntrusted | Self::CertificateExpired | Self::CertificateWrongHost)
    }
}

/// A failed request as the frontend gets it next to the error message.
#[derive(Debug, Clone, Serialize)]
pub struct Failure {
    /// [`SendFailure::code`].
    pub kind: &'static str,
    /// The innermost reason, as the TLS library or the OS gave it.
    pub detail: String,
}

/// One error of a chain: its message, and its kind when it is an I/O
/// error. The kind, not the message, tells a refused connection: Windows
/// words OS errors in the user's language.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub message: String,
    pub io_kind: Option<io::ErrorKind>,
}

/// `error` and its sources, outermost first.
pub fn chain(error: &(dyn Error + 'static)) -> Vec<Link> {
    let mut links = Vec::new();
    let mut current = Some(error);
    while let Some(e) = current {
        links.push(Link { message: e.to_string(), io_kind: e.downcast_ref::<io::Error>().map(io::Error::kind) });
        current = e.source();
    }
    links
}

/// What the chain says happened. Certificate and TLS errors come from
/// rustls and the tunnel and DNS ones from hyper-util, in English
/// whatever the system language; connection errors by their I/O kind.
pub fn classify(chain: &[Link]) -> SendFailure {
    for link in chain {
        let m = link.message.as_str();
        // rustls: "invalid peer certificate: UnknownIssuer",
        // "…: certificate expired: …", "…: certificate not valid for name …".
        if m.starts_with("invalid peer certificate") {
            return if m.contains("xpired") || m.contains("NotValidYet") || m.contains("not valid yet") {
                SendFailure::CertificateExpired
            } else if m.contains("NotValidForName") || m.contains("not valid for name") {
                SendFailure::CertificateWrongHost
            } else {
                SendFailure::CertificateUntrusted
            };
        }
        if m.starts_with("tunnel error: proxy authorization required") {
            return SendFailure::ProxyAuthRequired;
        }
        if m.starts_with("tunnel error: unsuccessful") {
            return SendFailure::ProxyRefused;
        }
    }
    for link in chain {
        match link.io_kind {
            Some(io::ErrorKind::ConnectionRefused) => return SendFailure::ConnectionRefused,
            Some(io::ErrorKind::TimedOut) => return SendFailure::TimedOut,
            Some(io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted | io::ErrorKind::UnexpectedEof) => {
                return SendFailure::ConnectionReset
            }
            _ => {}
        }
        let m = link.message.as_str();
        if m == "dns error" {
            return SendFailure::HostNotFound;
        }
        if m == "operation timed out" {
            return SendFailure::TimedOut;
        }
        // The rest of rustls's handshake errors.
        if ["received corrupt message", "received fatal alert", "peer is incompatible", "peer misbehaved", "received unexpected"]
            .iter()
            .any(|p| m.starts_with(p))
        {
            return SendFailure::TlsFailed;
        }
    }
    SendFailure::Other
}

/// A reqwest error read as above, with its innermost message as the detail.
pub fn describe(error: &reqwest::Error) -> (SendFailure, Failure) {
    let links = chain(error);
    let kind = if error.is_timeout() { SendFailure::TimedOut } else { classify(&links) };
    let detail = links.last().map(|l| l.message.clone()).unwrap_or_default();
    (kind, Failure { kind: kind.code(), detail })
}

/// The error and all its sources in one line, for a failure [`classify`]
/// cannot name: still more than reqwest's own "error sending request".
pub fn full_message(error: &(dyn Error + 'static)) -> String {
    let mut parts: Vec<String> = Vec::new();
    for link in chain(error) {
        // hyper's wrappers repeat nothing useful ("client error (Connect)").
        if !parts.last().is_some_and(|p| p.contains(&link.message)) {
            parts.push(link.message);
        }
    }
    parts.join(": ")
}

/// The message for a request that failed with `error`, and its failure for
/// the frontend when it is one [`classify`] can name.
pub fn message(error: &reqwest::Error) -> (String, Option<Failure>) {
    let (kind, failure) = describe(error);
    match kind {
        SendFailure::Other if error.is_connect() => (format!("Connection failed: {}", full_message(error)), None),
        SendFailure::Other => (format!("Request failed: {}", full_message(error)), None),
        kind => {
            let mut text = kind.explanation().to_string();
            if let Some(first) = text.get(..1) {
                text.replace_range(..1, &first.to_uppercase());
            }
            (format!("{} ({})", text, failure.detail), Some(failure))
        }
    }
}

/// An HTTPS server whose certificate this computer does not trust, for
/// the tests here and of activation.
#[cfg(test)]
pub(crate) mod test_server {
    use std::sync::OnceLock;

    /// A throwaway self-signed certificate for the name `bridgelab.test`
    /// and its PKCS#8 key (both DER), made with `openssl` the first time a
    /// test asks. Generated rather than committed: a private key in the
    /// source tree, however harmless, is what a secret scan rightly flags.
    /// None when `openssl` is not installed; the tests then say so and pass.
    fn pair() -> Option<&'static (Vec<u8>, Vec<u8>)> {
        static PAIR: OnceLock<Option<(Vec<u8>, Vec<u8>)>> = OnceLock::new();
        PAIR.get_or_init(|| {
            let dir = std::env::temp_dir().join(format!("bridgelab-test-tls-{}", std::process::id()));
            std::fs::create_dir_all(&dir).ok()?;
            let (key_pem, cert, key) = (dir.join("key.pem"), dir.join("cert.der"), dir.join("key.der"));
            let run = |args: &[&str]| {
                std::process::Command::new("openssl").args(args).output().ok().filter(|o| o.status.success())
            };
            run(&["genpkey", "-algorithm", "EC", "-pkeyopt", "ec_paramgen_curve:P-256", "-out", key_pem.to_str()?])?;
            run(&[
                "req", "-x509", "-new", "-key", key_pem.to_str()?, "-days", "2", "-subj", "/CN=bridgelab.test",
                "-addext", "subjectAltName=DNS:bridgelab.test", "-addext", "basicConstraints=critical,CA:FALSE",
                "-addext", "keyUsage=critical,digitalSignature", "-addext", "extendedKeyUsage=serverAuth",
                "-outform", "DER", "-out", cert.to_str()?,
            ])?;
            run(&["pkcs8", "-topk8", "-nocrypt", "-in", key_pem.to_str()?, "-outform", "DER", "-out", key.to_str()?])?;
            let pair = (std::fs::read(&cert).ok()?, std::fs::read(&key).ok()?);
            let _ = std::fs::remove_dir_all(&dir);
            Some(pair)
        })
        .as_ref()
    }

    /// The test certificate (DER), when `openssl` could make one.
    pub fn test_cert() -> Option<Vec<u8>> {
        pair().map(|(cert, _)| cert.clone())
    }

    /// An HTTPS server on 127.0.0.1 presenting the test certificate; its
    /// port. None (after a note on stderr) when no certificate could be made.
    pub async fn tls_server() -> Option<u16> {
        use std::sync::Arc;
        use tokio_rustls::rustls;
        let Some((cert, key)) = pair() else {
            eprintln!("openssl not found: skipping the TLS server test");
            return None;
        };
        let key = rustls::pki_types::PrivatePkcs8KeyDer::from(key.clone());
        let config = rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(vec![cert.clone().into()], key.into())
            .unwrap();
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((tcp, _)) = listener.accept().await {
                let acceptor = acceptor.clone();
                tokio::spawn(async move {
                    let _ = acceptor.accept(tcp).await;
                });
            }
        });
        Some(port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(message: &str) -> Link {
        Link { message: message.into(), io_kind: None }
    }

    fn io_link(message: &str, kind: io::ErrorKind) -> Link {
        Link { message: message.into(), io_kind: Some(kind) }
    }

    fn outer() -> Vec<Link> {
        vec![link("error sending request for url (https://127.0.0.1:23511/x)"), link("client error (Connect)")]
    }

    fn with(mut links: Vec<Link>, more: Vec<Link>) -> Vec<Link> {
        links.extend(more);
        links
    }

    #[test]
    fn certificate_failures_are_told_apart() {
        let cert = |m: &str| classify(&with(outer(), vec![io_link(m, io::ErrorKind::InvalidData)]));
        assert_eq!(cert("invalid peer certificate: UnknownIssuer"), SendFailure::CertificateUntrusted);
        assert_eq!(cert("invalid peer certificate: BadSignature"), SendFailure::CertificateUntrusted);
        assert_eq!(cert("invalid peer certificate: Revoked"), SendFailure::CertificateUntrusted);
        assert_eq!(
            cert("invalid peer certificate: certificate expired: verification time 1800000000 (UNIX), but certificate is not valid after 1700000000 (100000000 seconds ago)"),
            SendFailure::CertificateExpired,
        );
        assert_eq!(cert("invalid peer certificate: Expired"), SendFailure::CertificateExpired);
        assert_eq!(cert("invalid peer certificate: NotValidYet"), SendFailure::CertificateExpired);
        assert_eq!(
            cert("invalid peer certificate: certificate not valid for name \"fhir.example.org\"; certificate is only valid for DnsName(\"other.example.org\")"),
            SendFailure::CertificateWrongHost,
        );
        assert_eq!(cert("invalid peer certificate: NotValidForName"), SendFailure::CertificateWrongHost);
        assert_eq!(cert("received corrupt message of type InvalidContentType"), SendFailure::TlsFailed);
        assert_eq!(cert("received fatal alert: HandshakeFailure"), SendFailure::TlsFailed);
        assert!(SendFailure::CertificateWrongHost.is_certificate() && !SendFailure::TlsFailed.is_certificate());
    }

    #[test]
    fn network_failures_are_told_apart() {
        // Windows words OS errors in the user's language: the kind counts.
        let refused = with(outer(), vec![link("tcp connect error"), io_link("Impossibile stabilire la connessione (os error 10061)", io::ErrorKind::ConnectionRefused)]);
        assert_eq!(classify(&refused), SendFailure::ConnectionRefused);
        let dns = with(outer(), vec![link("dns error"), io_link("failed to lookup address information: Name or service not known", io::ErrorKind::Other)]);
        assert_eq!(classify(&dns), SendFailure::HostNotFound);
        let timeout = with(outer(), vec![link("operation timed out")]);
        assert_eq!(classify(&timeout), SendFailure::TimedOut);
        let reset = with(outer(), vec![io_link("Connection reset by peer (os error 104)", io::ErrorKind::ConnectionReset)]);
        assert_eq!(classify(&reset), SendFailure::ConnectionReset);
        assert_eq!(classify(&with(outer(), vec![link("tunnel error: unsuccessful")])), SendFailure::ProxyRefused);
        assert_eq!(classify(&with(outer(), vec![link("tunnel error: proxy authorization required")])), SendFailure::ProxyAuthRequired);
        assert_eq!(classify(&outer()), SendFailure::Other);
    }

    /// The real errors of a closed port and of a TLS handshake with a
    /// server that does not speak TLS, as reqwest gives them.
    #[tokio::test]
    async fn real_connection_errors_are_classified() {
        let client = reqwest::Client::builder().no_proxy().build().unwrap();

        let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = closed.local_addr().unwrap().port();
        drop(closed);
        let err = client.get(format!("http://127.0.0.1:{port}/")).send().await.unwrap_err();
        let (text, failure) = message(&err);
        assert_eq!(failure.as_ref().map(|f| f.kind), Some("refused"), "{text}");
        assert!(text.starts_with("Connection refused"), "{text}");

        // Plain bytes where a TLS server hello is expected.
        let plain = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = plain.local_addr().unwrap().port();
        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let (mut s, _) = plain.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf).await;
            let _ = s.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n").await;
        });
        let err = client.get(format!("https://127.0.0.1:{port}/")).send().await.unwrap_err();
        let (text, failure) = message(&err);
        assert_eq!(failure.as_ref().map(|f| f.kind), Some("tlsFailed"), "{text}");
    }

    /// The case that read like a dead network: the server answers, and its
    /// certificate is refused.
    #[tokio::test]
    async fn a_refused_certificate_is_not_reported_as_a_network_failure() {
        use super::test_server::{test_cert, tls_server};
        let Some(port) = tls_server().await else { return };
        let url = format!("https://127.0.0.1:{port}/x");

        // Not issued by any authority this computer trusts.
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let err = client.get(&url).send().await.unwrap_err();
        let (text, failure) = message(&err);
        let failure = failure.unwrap_or_else(|| panic!("not classified: {text}"));
        assert_eq!(failure.kind, "certUntrusted", "{text}");
        assert!(failure.detail.contains("UnknownIssuer"), "{}", failure.detail);
        assert!(text.starts_with("The server's certificate is not trusted"), "{text}");
        assert!(SendFailure::CertificateUntrusted.is_certificate());

        // Trusted, but issued for another name than 127.0.0.1.
        let root = reqwest::Certificate::from_der(&test_cert().unwrap()).unwrap();
        let client = reqwest::Client::builder().no_proxy().add_root_certificate(root).build().unwrap();
        let err = client.get(&url).send().await.unwrap_err();
        let (text, failure) = message(&err);
        assert_eq!(failure.map(|f| f.kind), Some("certWrongHost"), "{text}");
    }
}
