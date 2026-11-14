//! What to do with a link that wants a window of its own (`target="_blank"`).
//!
//! pl_mail forces `target="_blank"` on every link inside a mail, and a bare WebView2 window
//! simply ignores such a request. Links to other sites belong in the user's browser; links to
//! the pl_mail server itself stay in the mail window; and since the content of a mail is
//! untrusted, only web links are ever handed to the system.

use tauri::Url;

#[derive(Debug, PartialEq)]
pub enum LinkAction {
    /// Another site: open it in the default browser.
    OpenInBrowser(Url),
    /// A page of the pl_mail server itself: show it in the mail window.
    OpenHere(Url),
    /// A `mailto:` link: open a composer in pl_mail (the path to open).
    Compose(String),
    /// Anything else (`file:`, `ms-msdt:`, `tel:`…): never handed to the system.
    Ignore,
}

pub fn classify(server: &Url, target: &Url) -> LinkAction {
    match target.scheme() {
        "mailto" => crate::mailto::compose_path(target.as_str())
            .map_or(LinkAction::Ignore, LinkAction::Compose),
        "http" | "https" if target.origin() == server.origin() => {
            LinkAction::OpenHere(target.clone())
        }
        "http" | "https" => LinkAction::OpenInBrowser(target.clone()),
        _ => LinkAction::Ignore,
    }
}

/// What to do with a navigation of the mail window itself (a link without `target`, a redirect,
/// a script setting `location`).
#[derive(Debug, PartialEq)]
pub enum Navigation {
    /// A page of the server, or a step of a sign-in it started: let it load.
    Stay,
    /// An OAuth authorization page sending the user back to the server (pl_mail connecting a
    /// Gmail or Microsoft account): let it load, and every page until we are back on the server.
    StartSignIn,
    /// A `mailto:` link: open a composer in pl_mail instead (the path to open).
    Compose(String),
    /// Another site: open it in the default browser and stay where we are.
    OpenInBrowser(Url),
    /// Anything else (`file:`, `ms-msdt:`, …): refused.
    Refuse,
}

/// Decides about a navigation of the mail window. `signing_in` says whether an OAuth flow the
/// server started is under way, in which case the provider's pages (which can span several
/// hosts) are let through until the flow comes back to the server.
pub fn navigation(server: &Url, target: &Url, signing_in: bool) -> Navigation {
    match target.scheme() {
        "mailto" => crate::mailto::compose_path(target.as_str())
            .map_or(Navigation::Refuse, Navigation::Compose),
        // A blank page is what a fresh window starts with; a blob made by the server's own page
        // (an attachment it builds) belongs to it.
        "about" => Navigation::Stay,
        "blob" if target.origin() == server.origin() => Navigation::Stay,
        "http" | "https" if target.origin() == server.origin() || signing_in => Navigation::Stay,
        "http" | "https" if returns_to(server, target) => Navigation::StartSignIn,
        "http" | "https" => Navigation::OpenInBrowser(target.clone()),
        _ => Navigation::Refuse,
    }
}

/// Is this an OAuth authorization request whose `redirect_uri` leads back to the server?
fn returns_to(server: &Url, target: &Url) -> bool {
    target
        .query_pairs()
        .filter(|(key, _)| key == "redirect_uri")
        .filter_map(|(_, value)| Url::parse(&value).ok())
        .any(|back| back.origin() == server.origin())
}
