//! Recognising credentials in an HTTP request, and keeping them out of the
//! history.
//!
//! Authentication of any kind is a Pro feature, and APIs carry keys and
//! tokens in many places besides `Authorization`: vendor headers
//! (`Ocp-Apim-Subscription-Key`, `X-Goog-Api-Key`, `x-functions-key`),
//! cookies, signatures, and query parameters (`?access_token=`,
//! `?api_key=`, a SAS `sig=`). A short list of header names let those
//! through, so the check goes by what a name suggests instead.

/// Words that, in a header name, mean the header carries a credential.
const HEADER_WORDS: &[&str] = &[
    "auth", "token", "key", "secret", "signature", "session", "cookie", "password", "passwd", "credential",
];

/// Query parameters that carry a credential, by exact (lower-case) name.
const QUERY_NAMES: &[&str] = &[
    "key", "token", "sig", "auth", "authorization", "pwd", "pass", "secret", "session", "sessionid",
    "session_id", "jsessionid", "code_verifier",
];

/// Words that, inside a query parameter name, mean a credential. Plain
/// "auth" and "key" are not among them: FHIR search has `author`, and
/// `code` (also left out) is one of its most common parameters.
const QUERY_WORDS: &[&str] = &[
    "token", "secret", "password", "passwd", "apikey", "api_key", "api-key", "signature", "credential",
];

/// Whether a request header carries credentials: its name suggests a key,
/// token, secret, signature, session or authentication, or its value is
/// an HTTP authentication scheme (`Bearer …`, `Basic …`).
pub fn is_auth_header(name: &str, value: &str) -> bool {
    let name = name.trim().to_ascii_lowercase();
    let value = value.trim_start().to_ascii_lowercase();
    HEADER_WORDS.iter().any(|w| name.contains(w))
        || ["bearer ", "basic ", "digest ", "negotiate ", "hmac "].iter().any(|s| value.starts_with(s))
}

/// Whether a query parameter name carries credentials.
pub fn is_sensitive_query_param(name: &str) -> bool {
    let name = name.trim().to_ascii_lowercase();
    QUERY_NAMES.contains(&name.as_str()) || QUERY_WORDS.iter().any(|w| name.contains(w))
}

/// Whether the URL carries credentials: user:password, or a query
/// parameter that holds a key or token.
pub fn url_has_credentials(url: &str) -> bool {
    reqwest::Url::parse(url.trim()).is_ok_and(|u| {
        !u.username().is_empty() || u.password().is_some() || u.query_pairs().any(|(k, _)| is_sensitive_query_param(&k))
    })
}

/// The URL with its user:password and the values of credential query
/// parameters replaced by `***`, for the history. A URL that does not
/// parse is returned as it is.
pub fn redact_credentials(url: &str) -> String {
    let Ok(mut u) = reqwest::Url::parse(url.trim()) else {
        return url.to_string();
    };
    let mut changed = false;
    if !u.username().is_empty() || u.password().is_some() {
        let _ = u.set_username("***");
        let _ = u.set_password(None);
        changed = true;
    }
    let pairs: Vec<(String, String)> = u.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect();
    if pairs.iter().any(|(k, _)| is_sensitive_query_param(k)) {
        let mut q = u.query_pairs_mut();
        q.clear();
        for (k, v) in &pairs {
            q.append_pair(k, if is_sensitive_query_param(k) { "***" } else { v });
        }
        drop(q);
        changed = true;
    }
    if changed { u.to_string() } else { url.to_string() }
}

/// `text` with every URL in it (a run of non-blank characters holding
/// `://`) passed through [`redact_credentials`]: for history rows written
/// before the redaction existed.
pub fn redact_urls_in_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(|c: char| !c.is_whitespace()) {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let word = &rest[..end];
        if word.contains("://") { out.push_str(&redact_credentials(word)) } else { out.push_str(word) }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_headers_are_recognised_by_name_or_scheme() {
        for h in [
            "Authorization", "Proxy-Authorization", "Cookie", "X-API-Key", "Ocp-Apim-Subscription-Key",
            "X-Api-Token", "x-functions-key", "X-Goog-Api-Key", "X-Amz-Security-Token", "X-Hub-Signature",
            "X-Session-Id", "Client-Secret",
        ] {
            assert!(is_auth_header(h, "k"), "{h}");
        }
        assert!(is_auth_header("X-Custom", "Bearer abc"));
        for h in ["Content-Type", "Accept", "Accept-Encoding", "X-Request-Id", "Keep-Alive", "Prefer"] {
            assert!(!is_auth_header(h, "application/fhir+json"), "{h}");
        }
    }

    #[test]
    fn credential_query_parameters_count_but_fhir_search_does_not() {
        for u in [
            "https://h/x?access_token=t", "https://h/x?api_key=k", "https://h/x?apikey=k", "https://h/x?key=k",
            "https://h/x?token=t", "https://h/x?sig=s", "https://h/x?signature=s", "https://h/x?client_secret=s",
            "https://h/x?X-Amz-Signature=s", "http://alice:s3cret@host/x", "http://alice@host/x",
        ] {
            assert!(url_has_credentials(u), "{u}");
        }
        for u in [
            "https://h/fhir/Observation?code=http://loinc.org|1234-5&subject=Patient/1",
            "https://h/fhir/DocumentReference?author=Practitioner/1&_count=10",
            "http://host/x?user=alice",
        ] {
            assert!(!url_has_credentials(u), "{u}");
        }
    }

    #[test]
    fn history_urls_lose_their_credentials() {
        assert_eq!(redact_credentials("http://alice:s3cret@host/x"), "http://***@host/x");
        assert_eq!(
            redact_credentials("https://h/p?code=1234&access_token=abc&x=1"),
            "https://h/p?code=1234&access_token=***&x=1"
        );
        assert_eq!(redact_credentials("https://h/p?code=1"), "https://h/p?code=1");
        assert_eq!(redact_credentials("not a url"), "not a url");
        assert_eq!(
            redact_urls_in_text("GET http://bob:pw123@127.0.0.1:8/creds?api_key=K9 | MSH|x"),
            "GET http://***@127.0.0.1:8/creds?api_key=*** | MSH|x"
        );
    }
}
