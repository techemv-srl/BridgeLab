//! The proxy BridgeLab's HTTP requests go through: the HTTP tool, SOAP,
//! online activation and usage statistics (MLLP never uses one).
//!
//! reqwest takes the proxy from `HTTPS_PROXY`, `HTTP_PROXY` and
//! `ALL_PROXY`, with `NO_PROXY` for the hosts to reach directly, and when
//! none is set from the manual proxy of the Windows or macOS network
//! settings. Proxy auto-configuration (a PAC script, WPAD) is not read.
//! Of the exceptions in those settings only plain host and domain names
//! work: the Windows `<local>` entry and address patterns such as `10.*`
//! are not understood, and the macOS bypass list is not read at all.

use std::net::IpAddr;

/// A client builder for requests to `url`, with the proxy as above,
/// except for a loopback destination (see [`goes_direct`]), which is
/// always reached directly.
pub fn client_builder(url: &str) -> reqwest::ClientBuilder {
    let builder = reqwest::Client::builder();
    if goes_direct(url) {
        builder.no_proxy()
    } else {
        builder
    }
}

/// Whether a request to `url` must not go through a proxy: `localhost`
/// (and names under it), 127.0.0.0/8 and `::1`. Windows and macOS leave
/// them out of the proxy themselves, but the exceptions of their settings
/// are not read here and an environment proxy without `NO_PROXY` covers
/// them too: a request to a test server on this computer went to the
/// company proxy, and the proxy's error page came back as the server's
/// answer. Private addresses (10.x, 192.168.x) and single-label intranet
/// names are left to the proxy settings: some networks reach those only
/// through the proxy.
pub fn goes_direct(url: &str) -> bool {
    reqwest::Url::parse(url.trim())
        .ok()
        .and_then(|u| u.host_str().map(is_loopback_host))
        .unwrap_or(false)
}

/// `host` as a URL gives it (an IPv6 address in brackets).
fn is_loopback_host(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if host == "localhost" || host.ends_with(".localhost") {
        return true;
    }
    match host.trim_start_matches('[').trim_end_matches(']').parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => ip.is_loopback(),
        Ok(IpAddr::V6(ip)) => ip.is_loopback() || ip.to_ipv4_mapped().is_some_and(|v4| v4.is_loopback()),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::goes_direct;

    #[test]
    fn loopback_goes_direct_and_everything_else_follows_the_proxy_settings() {
        for url in [
            "http://localhost:8080/fhir",
            "https://LOCALHOST/",
            "http://localhost./x",
            "http://app.localhost:3000/",
            "http://127.0.0.1:2575/",
            "http://127.1.2.3/",
            "http://[::1]:8080/fhir",
            "http://[::ffff:127.0.0.1]/",
            " http://user:pw@127.0.0.1/ ",
        ] {
            assert!(goes_direct(url), "{url}");
        }
        for url in [
            "https://fhir.example.org/",
            "http://10.0.0.5:8080/",
            "http://192.168.1.5/",
            "http://intranet/",
            "http://localhost.example.org/",
            "http://notlocalhost/",
            "http://[::2]/",
            "not a url",
        ] {
            assert!(!goes_direct(url), "{url}");
        }
    }
}
