//! `mailto:` links: turns one into the pl_mail address that opens a composer.
//!
//! pl_mail's `/compose/new` is sent to the mailbox, which opens the composer in its dock, so a
//! plain navigation is enough. Today it reads only `to`; `cc`, `bcc`, `subject` and `body` are
//! passed along as well so that they start working the day the server learns them.

use percent_encoding::percent_decode_str;
use tauri::Url;

const SUBJECT_MAX: usize = 255;
const BODY_MAX: usize = 1500;

/// The first argument of a launch that is a `mailto:` link, if any.
pub fn from_args(args: &[String]) -> Option<String> {
    args.iter().find_map(|a| compose_path(a))
}

/// `/compose/new?to=…` for a `mailto:` link, or `None` when it is not a usable one.
pub fn compose_path(link: &str) -> Option<String> {
    let link = link.trim();
    // The scheme is case-insensitive (`MAILTO:` is valid); normalise it before parsing.
    if !link.get(..7)?.eq_ignore_ascii_case("mailto:") {
        return None;
    }
    let url = Url::parse(&format!("mailto:{}", &link[7..])).ok()?;

    // Recipients sit in the path, percent-encoded and comma-separated; `?to=` adds more.
    let mut recipients: Vec<String> = Vec::new();
    let path = percent_decode_str(url.path()).decode_utf8().ok()?;
    recipients.extend(path.split(',').map(|a| a.trim().to_string()));
    let (mut cc, mut bcc, mut subject, mut body) = (Vec::new(), Vec::new(), None, None);
    for (key, value) in url.query_pairs() {
        match key.to_ascii_lowercase().as_str() {
            "to" => recipients.extend(value.split(',').map(|a| a.trim().to_string())),
            "cc" => cc.extend(value.split(',').map(|a| a.trim().to_string())),
            "bcc" => bcc.extend(value.split(',').map(|a| a.trim().to_string())),
            "subject" => subject = Some(value.chars().take(SUBJECT_MAX).collect::<String>()),
            "body" => body = Some(value.chars().take(BODY_MAX).collect::<String>()),
            _ => {}
        }
    }
    recipients.retain(|a| looks_like_address(a));
    cc.retain(|a| looks_like_address(a));
    bcc.retain(|a| looks_like_address(a));

    let mut out = Url::parse("http://localhost/compose/new").ok()?;
    {
        let mut pairs = out.query_pairs_mut();
        let mut recipients = recipients.into_iter();
        // pl_mail takes a single address in `to`: the first one. The rest go to `cc` so
        // nobody is dropped once the server reads it.
        if let Some(first) = recipients.next() {
            pairs.append_pair("to", &first);
        }
        cc.extend(recipients);
        if !cc.is_empty() {
            pairs.append_pair("cc", &cc.join(","));
        }
        if !bcc.is_empty() {
            pairs.append_pair("bcc", &bcc.join(","));
        }
        if let Some(subject) = subject.filter(|s| !s.is_empty()) {
            pairs.append_pair("subject", &subject);
        }
        if let Some(body) = body.filter(|s| !s.is_empty()) {
            pairs.append_pair("body", &body);
        }
    }
    // No `?` when nothing was added: an empty composer is plain `/compose/new`.
    let query = out
        .query()
        .filter(|q| !q.is_empty())
        .map(|q| format!("?{q}"))
        .unwrap_or_default();
    Some(format!("{}{query}", out.path()))
}

/// Not full validation (the server does that): just enough to keep a link from putting anything
/// but an address into the URL we build.
fn looks_like_address(text: &str) -> bool {
    let Some((local, domain)) = text.rsplit_once('@') else {
        return false;
    };
    !local.is_empty()
        && text.len() <= 254
        && domain.contains('.')
        && domain
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '.')
        && !local
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '<' | '>' | '"' | '\\'))
}
