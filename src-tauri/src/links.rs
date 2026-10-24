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
