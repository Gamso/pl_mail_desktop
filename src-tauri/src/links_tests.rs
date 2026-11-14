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

mod navigation {
    use super::url;
    use crate::links::{Navigation, navigation};

    const SERVER: &str = "https://mail.example.com";

    #[test]
    fn pages_of_the_server_stay_in_the_window() {
        let target = url("https://mail.example.com/mail/inbox");
        assert_eq!(navigation(&url(SERVER), &target, false), Navigation::Stay);
    }

    #[test]
    fn another_site_opens_in_the_browser() {
        let target = url("https://phishing.example.net/login");
        assert_eq!(
            navigation(&url(SERVER), &target, false),
            Navigation::OpenInBrowser(target)
        );
        // Same host, another scheme: another origin.
        let target = url("http://mail.example.com/");
        assert_eq!(
            navigation(&url(SERVER), &target, false),
            Navigation::OpenInBrowser(target)
        );
    }

    #[test]
    fn a_sign_in_the_server_started_is_let_through_until_it_comes_back() {
        let authorize = url(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id=x&redirect_uri=https%3A%2F%2Fmail.example.com%2Foauth%2Fgoogle%2Fcallback&response_type=code",
        );
        assert_eq!(
            navigation(&url(SERVER), &authorize, false),
            Navigation::StartSignIn
        );
        // The provider's own steps, on whatever host, while the flow is under way.
        let step = url("https://accounts.youtube.com/accounts/SetSID");
        assert_eq!(navigation(&url(SERVER), &step, true), Navigation::Stay);
    }

    #[test]
    fn a_redirect_uri_to_another_site_is_not_a_sign_in() {
        let target =
            url("https://evil.example.net/auth?redirect_uri=https%3A%2F%2Fevil.example.net%2F");
        assert_eq!(
            navigation(&url(SERVER), &target, false),
            Navigation::OpenInBrowser(target)
        );
        let look_alike = url(
            "https://evil.example.net/auth?redirect_uri=https%3A%2F%2Fmail.example.com.evil.net%2F",
        );
        assert!(matches!(
            navigation(&url(SERVER), &look_alike, false),
            Navigation::OpenInBrowser(_)
        ));
    }

    #[test]
    fn other_schemes_are_refused_even_during_a_sign_in() {
        for target in [
            "file:///C:/Windows/System32/calc.exe",
            "ms-msdt:/id",
            "javascript:alert(1)",
            "data:text/html,hi",
            "blob:https://evil.example.net/0a1b",
        ] {
            assert_eq!(
                navigation(&url(SERVER), &url(target), true),
                Navigation::Refuse,
                "{target}"
            );
        }
        assert_eq!(
            navigation(&url(SERVER), &url("about:blank"), false),
            Navigation::Stay
        );
        assert_eq!(
            navigation(
                &url(SERVER),
                &url("blob:https://mail.example.com/0a1b"),
                false
            ),
            Navigation::Stay
        );
    }

    #[test]
    fn mailto_opens_the_composer() {
        assert!(matches!(
            navigation(&url(SERVER), &url("mailto:bob@example.com"), false),
            Navigation::Compose(path) if path.starts_with("/compose/new?to=bob")
        ));
    }
}
