use tauri::Url;

use crate::links::{LinkAction, classify};

fn url(text: &str) -> Url {
    Url::parse(text).unwrap()
}

#[test]
fn another_site_goes_to_the_default_browser() {
    let server = url("https://mail.example.com");
    let target = url("https://www.linkedin.com/comm/mynetwork/invite-accept?token=abc");
    assert_eq!(
        classify(&server, &target),
        LinkAction::OpenInBrowser(target)
    );
}

#[test]
fn the_server_itself_stays_in_the_mail_window() {
    let server = url("https://mail.example.com");
    let target = url("https://mail.example.com/mail/message/42");
    assert_eq!(classify(&server, &target), LinkAction::OpenHere(target));
}

#[test]
fn a_look_alike_host_is_another_site() {
    let server = url("https://mail.example.com");
    for other in [
        "https://mail.example.com.evil.net/",
        "http://mail.example.com/",
        "https://mail.example.com:8443/",
    ] {
        let target = url(other);
        assert_eq!(
            classify(&server, &target),
            LinkAction::OpenInBrowser(target),
            "{other}"
        );
    }
}

#[test]
fn a_mailto_link_opens_a_composer() {
    let server = url("https://mail.example.com");
    assert_eq!(
        classify(&server, &url("mailto:alice@example.com")),
        LinkAction::Compose("/compose/new?to=alice%40example.com".to_string())
    );
}

#[test]
fn only_web_links_ever_reach_the_system() {
    let server = url("https://mail.example.com");
    for risky in [
        "file:///C:/Windows/System32/calc.exe",
        "ms-msdt:/id",
        "javascript:alert(1)",
        "tel:+33123456789",
        "data:text/html,hi",
    ] {
        assert_eq!(
            classify(&server, &url(risky)),
            LinkAction::Ignore,
            "{risky}"
        );
    }
}
