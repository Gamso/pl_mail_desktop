use crate::mailto::{compose_path, from_args};

#[test]
fn a_plain_address_becomes_the_recipient() {
    assert_eq!(
        compose_path("mailto:alice@example.com").as_deref(),
        Some("/compose/new?to=alice%40example.com")
    );
}

#[test]
fn the_scheme_is_case_insensitive() {
    assert!(compose_path("MAILTO:alice@example.com").is_some());
    assert!(compose_path("MailTo:alice@example.com").is_some());
}

#[test]
fn subject_and_body_are_decoded_and_re_encoded() {
    let path =
        compose_path("mailto:alice@example.com?subject=Hello%20there&body=Line%201%0ALine%202")
            .unwrap();
    assert!(path.starts_with("/compose/new?to=alice%40example.com"));
    assert!(path.contains("subject=Hello+there"), "{path}");
    assert!(path.contains("body=Line+1%0ALine+2"), "{path}");
}

#[test]
fn an_encoded_address_is_decoded() {
    assert_eq!(
        compose_path("mailto:bob%40example.com").as_deref(),
        Some("/compose/new?to=bob%40example.com")
    );
}

#[test]
fn extra_recipients_are_kept_in_cc() {
    let path = compose_path("mailto:a@example.com,b@example.com?cc=c@example.com").unwrap();
    assert!(
        path.starts_with("/compose/new?to=a%40example.com"),
        "{path}"
    );
    assert!(
        path.contains("cc=c%40example.com%2Cb%40example.com"),
        "{path}"
    );
}

#[test]
fn to_can_live_in_the_query() {
    assert_eq!(
        compose_path("mailto:?to=alice@example.com").as_deref(),
        Some("/compose/new?to=alice%40example.com")
    );
}

#[test]
fn a_link_without_a_valid_address_still_opens_an_empty_composer() {
    assert_eq!(
        compose_path("mailto:not-an-address").as_deref(),
        Some("/compose/new")
    );
    assert_eq!(compose_path("mailto:").as_deref(), Some("/compose/new"));
}

#[test]
fn nothing_but_an_address_can_reach_the_url() {
    let path = compose_path("mailto:a@example.com&evil=1?subject=x").unwrap();
    assert!(!path.contains("evil"), "{path}");
    let path = compose_path("mailto:%3Cscript%3E@example.com").unwrap();
    assert!(!path.contains("script"), "{path}");
}

#[test]
fn other_schemes_and_junk_are_ignored() {
    assert!(compose_path("https://example.com").is_none());
    assert!(compose_path("--minimized").is_none());
    assert!(compose_path("").is_none());
}

#[test]
fn the_link_is_found_among_launch_arguments() {
    let args = vec![
        "plmail.exe".to_string(),
        "mailto:alice@example.com".to_string(),
    ];
    assert_eq!(
        from_args(&args).as_deref(),
        Some("/compose/new?to=alice%40example.com")
    );
    assert!(from_args(&["plmail.exe".to_string()]).is_none());
}
