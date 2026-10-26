use crate::i18n::{Lang, fill, tr};

/// Every key the code asks for. Keep in step with the `match` in `tr`.
const KEYS: &[&str] = &[
    "tray.open",
    "tray.settings",
    "tray.quit",
    "menu.tools",
    "menu.help",
    "menu.plmail_settings",
    "menu.admin",
    "menu.client_settings",
    "menu.about",
    "toast.many",
    "toast.test",
    "toast.unknown_sender",
    "toast.no_subject",
    "status.none",
    "status.ok",
    "status.new",
    "err.unreachable",
    "err.unreachable_short",
    "err.unauthorized",
    "err.http",
    "err.not_jmap",
    "err.no_api_url",
    "err.jmap",
    "err.app_down",
    "err.timeout",
    "err.refused",
    "err.scheme",
    "err.need_password",
    "err.save_password",
    "err.no_server",
    "err.window",
    "menu.file",
    "menu.view",
    "menu.go",
    "file.new_message",
    "file.check",
    "file.print",
    "file.close",
    "file.quit",
    "view.reload",
    "view.back",
    "view.forward",
    "view.zoom_in",
    "view.zoom_out",
    "view.zoom_reset",
    "view.fullscreen",
    "go.inbox",
    "go.starred",
    "go.snoozed",
    "go.drafts",
    "go.sent",
    "go.archive",
    "go.spam",
    "go.trash",
    "go.all",
    "go.calendar",
    "go.contacts",
    "tools.filters",
    "tools.templates",
    "tools.integrations",
    "tools.app_passwords",
    "help.shortcuts",
    "help.whats_new",
    "help.data_folder",
    "menu.check_updates",
    "toast.update_title",
    "toast.update_body",
    "err.update",
    "err.update_portable",
    "toast.received_one",
    "toast.from",
    "toast.btn_read",
    "toast.btn_delete",
    "toast.action_failed",
];
const LANGS: [Lang; 4] = [Lang::En, Lang::De, Lang::Fr, Lang::Pi];

fn placeholders(text: &str) -> Vec<String> {
    let mut found: Vec<String> = text
        .split('{')
        .skip(1)
        .filter_map(|rest| rest.split('}').next().map(str::to_string))
        .collect();
    found.sort();
    found
}

#[test]
fn every_key_is_translated_in_every_language() {
    for key in KEYS {
        for lang in LANGS {
            assert!(
                !tr(lang, key).trim().is_empty(),
                "{key} is empty in {lang:?}"
            );
        }
    }
}

#[test]
fn translations_keep_the_same_placeholders() {
    for key in KEYS {
        let expected = placeholders(tr(Lang::En, key));
        for lang in LANGS {
            assert_eq!(
                placeholders(tr(lang, key)),
                expected,
                "{key} differs in {lang:?}"
            );
        }
    }
}

#[test]
fn unknown_keys_are_empty() {
    assert_eq!(tr(Lang::En, "no.such.key"), "");
}

#[test]
fn language_codes_round_trip() {
    for lang in LANGS {
        assert_eq!(Lang::from_code(lang.code()), Some(lang));
    }
    assert_eq!(Lang::from_code("en_PI"), Some(Lang::Pi));
    assert_eq!(Lang::from_code("auto"), None);
}

#[test]
fn fill_replaces_every_placeholder() {
    assert_eq!(
        fill("{n} of {n} for {who}", &[("n", "2"), ("who", "you")]),
        "2 of 2 for you"
    );
}
