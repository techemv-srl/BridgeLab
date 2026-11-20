//! Where the app's webviews may go.
//!
//! The main window and the manual window show the app's own pages and
//! nothing else. The frontend opens web and e-mail links with the system's
//! browser or mail client, but only for the clicks it sees: a link the
//! webview followed by itself (a middle click, its own "Open Link") loaded
//! the URL in the app's window, and on Linux a `mailto:` link replaced the
//! whole app with "The URL can't be shown". Every navigation of a webview
//! now goes through [`allowed_navigation`] first.

use tauri::Url;

/// What happens to a navigation of one of the app's webviews.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Navigation {
    /// The app's own page, or a frame inside it: the webview loads it.
    Allow,
    /// A web or e-mail link: the system's browser or mail client opens it,
    /// and the window stays where it is.
    OpenExternally,
    /// Anything else (`file:`, `data:`, an unknown scheme...): not loaded.
    Refuse,
}

/// Whether `url` is the origin the app's pages are served from:
/// `tauri://localhost` on Linux and macOS, `http(s)://tauri.localhost` on
/// Windows, or the dev server while running `tauri dev` (`dev_url`).
fn is_app_origin(url: &Url, dev_url: Option<&Url>) -> bool {
    let host = url.host_str();
    match url.scheme() {
        "tauri" if host == Some("localhost") && url.port().is_none() => true,
        "http" | "https" if host == Some("tauri.localhost") && url.port().is_none() => true,
        _ => dev_url.is_some_and(|dev| dev.origin() == url.origin()),
    }
}

/// What to do with a navigation of a webview to `url`. `dev_url` is the
/// dev server's address when the app is running from it, `None` otherwise.
pub(crate) fn allowed_navigation(url: &Url, dev_url: Option<&Url>) -> Navigation {
    if is_app_origin(url, dev_url) {
        return Navigation::Allow;
    }
    match url.scheme() {
        // An empty frame, or the manual's in-window fallback (an iframe
        // whose document is written by the page).
        "about" if matches!(url.path(), "blank" | "srcdoc") => Navigation::Allow,
        // A blob made by the app's own page; one made elsewhere is not ours.
        "blob" => match Url::parse(url.path()) {
            Ok(inner) if is_app_origin(&inner, dev_url) => Navigation::Allow,
            _ => Navigation::Refuse,
        },
        // The links the frontend hands to the opener too.
        "http" | "https" | "mailto" | "tel" => Navigation::OpenExternally,
        _ => Navigation::Refuse,
    }
}

/// The plugin that applies [`allowed_navigation`] to every webview.
pub(crate) fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("navigation-guard")
        .on_navigation(|webview, url| {
            use tauri::Manager;
            let app = webview.app_handle();
            let dev_url = if tauri::is_dev() { app.config().build.dev_url.as_ref() } else { None };
            match allowed_navigation(url, dev_url) {
                Navigation::Allow => true,
                Navigation::OpenExternally => {
                    use tauri_plugin_opener::OpenerExt;
                    // Without a program to open it (no xdg-open), nothing
                    // happens, as for a click the frontend handles.
                    let _ = app.opener().open_url(url.as_str(), None::<&str>);
                    false
                }
                Navigation::Refuse => false,
            }
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::{allowed_navigation, Navigation};
    use tauri::Url;

    fn decide(url: &str) -> Navigation {
        allowed_navigation(&Url::parse(url).unwrap(), None)
    }

    #[test]
    fn the_apps_own_pages_load() {
        for url in [
            "tauri://localhost",
            "tauri://localhost/",
            "tauri://localhost/manual?lang=en&keys=%5B%5D",
            "tauri://localhost/#section",
            "http://tauri.localhost/",
            "https://tauri.localhost/manual?lang=it",
            "about:blank",
            "about:srcdoc",
            "blob:tauri://localhost/2f1c8a3e-0000-4000-8000-000000000000",
            "blob:http://tauri.localhost/2f1c8a3e-0000-4000-8000-000000000000",
        ] {
            assert_eq!(decide(url), Navigation::Allow, "{url}");
        }
    }

    #[test]
    fn web_and_mail_links_open_outside_the_app() {
        for url in [
            "mailto:info@techemv.it",
            "MAILTO:info@techemv.it?subject=Quote",
            "tel:+390000000",
            "https://www.techemv.it",
            "http://example.com/x",
            // Look-alikes of the app's origin are other hosts.
            "http://tauri.localhost.example.com/",
            "https://tauri.localhost:8443/",
            "http://localhost:1420/",
        ] {
            assert_eq!(decide(url), Navigation::OpenExternally, "{url}");
        }
    }

    #[test]
    fn every_other_scheme_is_refused() {
        for url in [
            "file:///etc/passwd",
            "data:text/html,<h1>x</h1>",
            "javascript:alert(1)",
            "ftp://example.com/",
            "tauri://evil/",
            "tauri://localhost:9000/",
            "ipc://localhost/open_file",
            "about:config",
            "blob:https://example.com/2f1c8a3e-0000-4000-8000-000000000000",
            "vscode://file/etc/passwd",
            "ms-settings:privacy",
            "bl-e2e-nowhere:test",
        ] {
            assert_eq!(decide(url), Navigation::Refuse, "{url}");
        }
    }

    #[test]
    fn the_dev_server_loads_only_while_running_from_it() {
        let dev = Url::parse("http://localhost:1420").unwrap();
        let page = Url::parse("http://localhost:1420/manual?lang=en").unwrap();
        assert_eq!(allowed_navigation(&page, Some(&dev)), Navigation::Allow);
        assert_eq!(allowed_navigation(&page, None), Navigation::OpenExternally);
        // Another port on the same host is not the dev server.
        let other = Url::parse("http://localhost:1421/").unwrap();
        assert_eq!(allowed_navigation(&other, Some(&dev)), Navigation::OpenExternally);
    }
}
