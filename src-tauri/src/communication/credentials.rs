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
/// Also the names of path parameters (`/app;jsessionid=…`).
const QUERY_NAMES: &[&str] = &[
    "key", "token", "sig", "auth", "authorization", "pwd", "pass", "secret", "session", "sessionid",
    "session_id", "jsessionid", "phpsessid", "code_verifier",
];

/// Words that, inside a query parameter name, mean a credential. Plain
/// "auth" and "key" are not among them: FHIR search has `author`, and
/// `code` (also left out) is one of its most common parameters.
const QUERY_WORDS: &[&str] = &[
    "token", "secret", "password", "passwd", "pass_word", "apikey", "api_key", "api-key", "signature",
    "credential",
];

/// Query parameter names that are keys under a service's own name (Azure
/// API Management's `subscription-key`, `access_key`, `app_key`…). A list,
/// not every name ending in `-key`/`_key`: `sort_key` or `partition-key`
/// are not credentials, and a credential in the URL makes a request Pro
/// only, so a false match would block an ordinary Community request.
const QUERY_KEY_NAMES: &[&str] = &[
    "subscription-key", "subscription_key", "access-key", "access_key", "accesskey",
    "app-key", "app_key", "appkey", "client-key", "client_key", "private-key", "private_key",
    "auth-key", "auth_key", "master-key", "master_key", "sas-key", "sas_key",
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
    QUERY_NAMES.contains(&name.as_str())
        || QUERY_WORDS.iter().any(|w| name.contains(w))
        || QUERY_KEY_NAMES.contains(&name.as_str())
}

/// Whether the URL carries credentials: user:password, or a query or path
/// parameter that holds a key, token or session.
pub fn url_has_credentials(url: &str) -> bool {
    reqwest::Url::parse(url.trim()).is_ok_and(|u| {
        !u.username().is_empty()
            || u.password().is_some()
            || u.query_pairs().any(|(k, _)| is_sensitive_query_param(&k))
            || path_parameter_names(u.path()).any(|name| is_sensitive_query_param(&percent_decoded(name)))
    })
}

/// The names of the parameters in a URL path, the `name=value` after a `;`
/// in a segment: a Java servlet container puts the session there when the
/// browser takes no cookies (`/app/echo;jsessionid=…`), and a session is as
/// good as a password while it lasts.
fn path_parameter_names(path: &str) -> impl Iterator<Item = &str> {
    path.split('/')
        .flat_map(|segment| segment.split(';').skip(1))
        .filter_map(|param| param.split_once('=').map(|(name, _)| name))
}

/// The URL with its user:password and the values of credential
/// parameters replaced by `***`, for the history; the rest is kept as
/// typed.
///
/// This works on the text, not on what a URL parser makes of it. A URL
/// that does not parse used to be kept whole, and an unencoded `/`, `#` or
/// `?` in a password is enough for that ("invalid port number"). And when
/// one parses, the parser ends the user:password at the first `/`, so
/// `alice:p@ss/w@host` left "ss/w" in clear. So everything between `://`
/// and the last `@` is masked whenever the URL has a user:password or does
/// not parse: a password may hold any character, and hiding a little too
/// much (an `@` further on) costs less than showing part of a password. A
/// URL that parses without one keeps its `@`s (an e-mail address in a
/// search). Credential parameters are found by name in the query and in
/// the fragment alike: a sign-in can hand its tokens back in the fragment
/// (`#access_token=…`), which is never sent but was kept in the history.
pub fn redact_credentials(url: &str) -> String {
    let has_user = match reqwest::Url::parse(url.trim()) {
        // Without `//` there is no user:password for the parser to find:
        // `alice:pw@host` reads as the scheme "alice" and a path.
        Ok(u) if !u.cannot_be_a_base() => !u.username().is_empty() || u.password().is_some(),
        _ => true,
    };
    let start = url.find("://").map_or(0, |i| i + 3);
    let masked = match url[start..].rfind('@') {
        Some(at) if has_user => format!("{}***{}", &url[..start], &url[start + at..]),
        _ => url.to_string(),
    };
    redact_params(&masked)
}

/// `text` with the value of every credential parameter in it replaced by
/// `***`. A parameter is `name=value` at the start or after a `?`, `#`,
/// `&` or `;`, and its value runs to the next `&` or `#`: a `;` or `?`
/// inside a token is part of it. In the path (before any `?` or `#`) a
/// parameter's value also ends at the next `/` or `?`, so the rest of the
/// URL stays readable after a `;jsessionid=…`.
fn redact_params(text: &str) -> String {
    const SEPARATORS: [char; 4] = ['&', ';', '?', '#'];
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut in_path = true;
    loop {
        let end = rest.find(SEPARATORS).unwrap_or(rest.len());
        match rest[..end].split_once('=') {
            Some((name, _)) if is_sensitive_query_param(&percent_decoded(name)) => {
                out.push_str(name);
                out.push_str("=***");
                let value = name.len() + 1;
                let value_end: &[char] = if in_path { &['/', '?', '&', '#'] } else { &['&', '#'] };
                rest = &rest[rest[value..].find(value_end).map_or(rest.len(), |i| value + i)..];
            }
            _ => {
                out.push_str(&rest[..end]);
                rest = &rest[end..];
            }
        }
        // The separator, kept; every separator is one byte.
        let Some(sep) = rest.get(..1) else { break };
        if sep == "?" || sep == "#" {
            in_path = false;
        }
        out.push_str(sep);
        rest = &rest[1..];
    }
    out
}

/// A parameter name as a server reads it: `%5F` is `_`, `+` a space.
fn percent_decoded(name: &str) -> String {
    let bytes = name.as_bytes();
    let hex = |i: usize| bytes.get(i).and_then(|&b| (b as char).to_digit(16));
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match (bytes[i], hex(i + 1), hex(i + 2)) {
            (b'%', Some(hi), Some(lo)) => {
                out.push((hi * 16 + lo) as u8);
                i += 3;
            }
            (b'+', ..) => {
                out.push(b' ');
                i += 1;
            }
            (b, ..) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
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

    /// A password with an unencoded `/`, `#`, `?` or `@` was kept in clear:
    /// the URL does not parse, or parses with part of the password read as
    /// the host or the path.
    #[test]
    fn passwords_a_url_parser_misreads_are_redacted() {
        for (url, expected) in [
            ("http://alice:UNPARSE_PW_11@127.0.0.1:99999/x", "http://***@127.0.0.1:99999/x"),
            ("http://alice:Pa#ss_A44@127.0.0.1:8080/x", "http://***@127.0.0.1:8080/x"),
            ("http://alice:A45slash/Pw@127.0.0.1:8080/x", "http://***@127.0.0.1:8080/x"),
            ("http://alice:what?ever@127.0.0.1:8080/x", "http://***@127.0.0.1:8080/x"),
            ("http://alice:p@ss/w@127.0.0.1:8080/x", "http://***@127.0.0.1:8080/x"),
            ("alice:pw@127.0.0.1/x", "***@127.0.0.1/x"),
            ("http://host:99999/x?access_token=TOK1&q=1", "http://host:99999/x?access_token=***&q=1"),
        ] {
            assert_eq!(redact_credentials(url), expected, "{url}");
        }
        assert_eq!(
            redact_urls_in_text("POST http://alice:Pa#ss@h:8080/x | body"),
            "POST http://***@h:8080/x | body"
        );
        // An `@` in a URL without a user:password is not one.
        let search = "https://h/fhir/Patient?email=a@b.example&_count=5";
        assert_eq!(redact_credentials(search), search);
    }

    #[test]
    fn credential_parameters_in_the_fragment_are_redacted() {
        assert_eq!(
            redact_credentials("https://h/cb#access_token=FRAG_TOKEN_13&id_token=FRAG_ID_14&state=7"),
            "https://h/cb#access_token=***&id_token=***&state=7"
        );
        assert_eq!(redact_credentials("https://h/cb?x=1#/route?token=t"), "https://h/cb?x=1#/route?token=***");
        assert_eq!(redact_credentials("https://h/doc#section-2"), "https://h/doc#section-2");
        assert_eq!(redact_credentials("https://h/x?q=1;sig=S1&a=2"), "https://h/x?q=1;sig=***&a=2");
        // A name is read as a server reads it, and a value runs to the next
        // `&` or `#`.
        assert_eq!(redact_credentials("https://h/x?access%5Ftoken=t&y=1"), "https://h/x?access%5Ftoken=***&y=1");
        assert_eq!(redact_credentials("https://h/x?token=a;b?c#frag"), "https://h/x?token=***#frag");
    }

    /// A session ID in the path (`;jsessionid=`) is a credential: a
    /// Community request carrying one is Pro, and the History keeps it as
    /// `***` with the rest of the URL readable.
    #[test]
    fn a_session_id_in_the_path_is_a_credential() {
        for u in [
            "http://host/echo;jsessionid=abc",
            "https://h/app;JSESSIONID=ABC123/fhir/Patient?_count=1",
            "https://h/a;foo=1;jsessionid=x/b",
            "https://h/a;jsession%69d=x",
            "https://h/x?PHPSESSID=abc",
        ] {
            assert!(url_has_credentials(u), "{u}");
        }
        // `sid` is left out: a study, site or subject id in a query is
        // ordinary data, and a Community request carrying one is not Pro.
        for u in ["https://h/fhir/Patient;v=2", "https://h/a;b/c", "https://h/fhir/Observation?code=1&subject=Patient/1", "https://h/x?sid=STUDY42"] {
            assert!(!url_has_credentials(u), "{u}");
        }
        assert_eq!(redact_credentials("http://host/echo;jsessionid=abc"), "http://host/echo;jsessionid=***");
        assert_eq!(
            redact_credentials("https://h/app;jsessionid=ABC123/fhir/Patient?_count=1&access_token=t"),
            "https://h/app;jsessionid=***/fhir/Patient?_count=1&access_token=***"
        );
        assert_eq!(redact_credentials("https://h/app;jsessionid=ABC?x=1"), "https://h/app;jsessionid=***?x=1");
        // In the query a value still runs to the next `&` or `#`.
        assert_eq!(redact_credentials("https://h/x?token=a/b?c&y=1"), "https://h/x?token=***&y=1");
    }

    #[test]
    fn key_parameters_named_after_their_service_count() {
        for u in ["https://apim.example/x?subscription-key=k", "https://h/x?access_key=k", "https://h/x?pass_word=p"] {
            assert!(url_has_credentials(u), "{u}");
            assert!(redact_credentials(u).ends_with("=***"), "{u}");
        }
        assert!(!url_has_credentials("https://h/fhir/Observation?code=1&_has:Observation:patient:code=2"));
        // Ordinary parameters that merely end in -key/_key are not keys:
        // matching them would make a Community request Pro only.
        for u in ["https://h/x?sort_key=name", "https://h/x?partition-key=p1", "https://h/x?primary_key=id"] {
            assert!(!url_has_credentials(u), "{u}");
            assert_eq!(redact_credentials(u), u);
        }
    }
}
