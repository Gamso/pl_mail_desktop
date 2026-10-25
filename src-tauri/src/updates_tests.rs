use crate::updates::{due, is_installed_at, is_stable, should_offer};

#[test]
fn only_final_releases_are_stable() {
    assert!(is_stable("0.2.0"));
    assert!(is_stable("v1.10.3"));
    for beta in ["0.2.0-beta.1", "v0.2.0-rc.2", "1.0.0-alpha", "0.3.0-beta"] {
        assert!(!is_stable(beta), "{beta}");
    }
    assert!(!is_stable("not a version"));
    assert!(!is_stable(""));
}

#[test]
fn an_automatic_check_waits_a_day() {
    let now = 1_800_000_000;
    assert!(!due(now - 60, now));
    assert!(!due(now - 23 * 3600, now));
    assert!(due(now - 24 * 3600, now));
    assert!(due(now - 10 * 24 * 3600, now));
    // Never checked before.
    assert!(due(0, now));
    // The clock went backwards: do not stay silent until the future catches up.
    assert!(due(now + 3600, now));
}

#[test]
fn a_skipped_version_is_only_hidden_from_automatic_checks() {
    assert!(!should_offer("0.2.0", "0.2.0", false));
    assert!(should_offer("0.2.0", "0.2.0", true));
    assert!(should_offer("0.2.1", "0.2.0", false));
    assert!(should_offer("0.2.0", "", false));
}

#[test]
fn a_copy_without_an_uninstaller_is_portable() {
    let dir = std::env::temp_dir().join(format!("plmail-updates-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    assert!(!is_installed_at(&dir));
    std::fs::write(dir.join("uninstall.exe"), b"").unwrap();
    assert!(is_installed_at(&dir));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The plugin reads `plugins.updater` when the application starts: a mistake there would only
/// show up as a crash or a silent "no update", so the real file is parsed here.
#[test]
fn the_updater_configuration_is_valid() {
    let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let updater = conf["plugins"]["updater"].clone();
    let config: tauri_plugin_updater::Config = serde_json::from_value(updater).unwrap();

    assert_eq!(config.endpoints.len(), 1);
    let endpoint = &config.endpoints[0];
    assert_eq!(endpoint.scheme(), "https");
    assert_eq!(endpoint.host_str(), Some("github.com"));
    // GitHub's "latest" skips drafts and pre-releases, which is what keeps betas out.
    assert!(
        endpoint.path().contains("/releases/latest/download/"),
        "{endpoint}"
    );

    // A minisign public key starts with "untrusted comment", which is "dW50cnVzdGVk" in base64.
    assert!(
        config.pubkey.starts_with("dW50cnVzdGVk"),
        "not a minisign public key"
    );
    // An update must carry the version it was signed for, so an old signed build cannot be replayed.
    assert!(config.require_signed_version);

    assert_eq!(conf["bundle"]["createUpdaterArtifacts"], true);
}
