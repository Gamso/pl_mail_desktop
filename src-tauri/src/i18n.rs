//! Strings the Rust side shows: tray, menu bar, toasts, status line and errors.
//! The windows' own text lives in `src/i18n.ts`; both cover the languages plMail ships.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    De,
    Fr,
    Pi,
}

impl Lang {
    /// The codes plMail uses for its locales; `en_PI` is Pirate English.
    pub fn from_code(code: &str) -> Option<Lang> {
        match code {
            "en" => Some(Lang::En),
            "de" => Some(Lang::De),
            "fr" => Some(Lang::Fr),
            "pi" | "en_PI" => Some(Lang::Pi),
            _ => None,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::De => "de",
            Lang::Fr => "fr",
            Lang::Pi => "pi",
        }
    }

    /// Windows' display language, falling back to English.
    pub fn detect() -> Lang {
        let locale = sys_locale::get_locale().unwrap_or_default().to_lowercase();
        match locale.get(..2) {
            Some("de") => Lang::De,
            Some("fr") => Lang::Fr,
            _ => Lang::En,
        }
    }

    /// `auto` (or anything unknown) follows Windows.
    pub fn resolve(setting: &str) -> Lang {
        Lang::from_code(setting).unwrap_or_else(Lang::detect)
    }
}

fn pick(lang: Lang, [en, de, fr, pi]: [&'static str; 4]) -> &'static str {
    match lang {
        Lang::En => en,
        Lang::De => de,
        Lang::Fr => fr,
        Lang::Pi => pi,
    }
}

/// Looks a string up by key. Placeholders such as `{n}` are filled by the caller.
pub fn tr(lang: Lang, key: &str) -> &'static str {
    match key {
        // Tray
        "tray.open" => pick(
            lang,
            [
                "Open pl_mail",
                "pl_mail öffnen",
                "Ouvrir pl_mail",
                "Open the mail chest",
            ],
        ),
        "tray.settings" => pick(
            lang,
            [
                "Client settings",
                "Client-Einstellungen",
                "Paramètres du client",
                "Ship's settings",
            ],
        ),
        "tray.quit" => pick(lang, ["Quit", "Beenden", "Quitter", "Abandon ship"]),

        // Menu bar of the mail window
        "menu.tools" => pick(lang, ["&Tools", "&Extras", "&Outils", "&Tools"]),
        "menu.help" => pick(lang, ["&Help", "&Hilfe", "&Aide", "&Help"]),
        "menu.plmail_settings" => pick(
            lang,
            [
                "plMail &settings",
                "plMail-&Einstellungen",
                "&Réglages de plMail",
                "plMail &settings",
            ],
        ),
        "menu.admin" => pick(
            lang,
            [
                "A&dministration",
                "A&dministration",
                "A&dministration",
                "Cap'n's &quarters",
            ],
        ),
        "menu.client_settings" => pick(
            lang,
            [
                "&Client settings…",
                "&Client-Einstellungen …",
                "Paramètres du &client…",
                "&Ship's settings…",
            ],
        ),
        "menu.about" => pick(
            lang,
            [
                "&About {name}",
                "&Über {name}",
                "À &propos de {name}",
                "&About {name}",
            ],
        ),

        // Toasts and status
        "toast.many" => pick(
            lang,
            [
                "{n} new messages",
                "{n} neue Nachrichten",
                "{n} nouveaux messages",
                "{n} new bottles washed ashore",
            ],
        ),
        "toast.test" => pick(
            lang,
            [
                "Notifications are working.",
                "Benachrichtigungen funktionieren.",
                "Les notifications fonctionnent.",
                "The bells be ringing.",
            ],
        ),
        "toast.unknown_sender" => pick(
            lang,
            [
                "Unknown sender",
                "Unbekannter Absender",
                "Expéditeur inconnu",
                "Stranger from the sea",
            ],
        ),
        "toast.no_subject" => pick(
            lang,
            [
                "(no subject)",
                "(kein Betreff)",
                "(sans objet)",
                "(no title)",
            ],
        ),
        "status.none" => pick(
            lang,
            [
                "not configured",
                "nicht eingerichtet",
                "non configuré",
                "not rigged yet",
            ],
        ),
        "status.ok" => pick(
            lang,
            [
                "{unread} unread, last check at {time}",
                "{unread} ungelesen, zuletzt geprüft um {time}",
                "{unread} non lu(s), dernière vérification à {time}",
                "{unread} unread, last scouted at {time}",
            ],
        ),
        "status.new" => pick(
            lang,
            [
                " ({n} new)",
                " ({n} neu)",
                " ({n} nouveau(x))",
                " ({n} new)",
            ],
        ),

        // Errors
        "err.unreachable" => pick(
            lang,
            [
                "Server unreachable ({base}): {e}",
                "Server nicht erreichbar ({base}): {e}",
                "Serveur injoignable ({base}) : {e}",
                "Server be lost at sea ({base}): {e}",
            ],
        ),
        "err.unreachable_short" => pick(
            lang,
            [
                "Server unreachable: {e}",
                "Server nicht erreichbar: {e}",
                "Serveur injoignable : {e}",
                "Server be lost at sea: {e}",
            ],
        ),
        "err.unauthorized" => pick(
            lang,
            [
                "The app password is invalid or has been revoked.",
                "Das App-Passwort ist ungültig oder wurde widerrufen.",
                "Mot de passe d'application invalide ou révoqué.",
                "The spare key be wrong or has been thrown overboard.",
            ],
        ),
        "err.http" => pick(
            lang,
            [
                "Unexpected server response (HTTP {code}).",
                "Unerwartete Antwort des Servers (HTTP {code}).",
                "Réponse inattendue du serveur (HTTP {code}).",
                "Strange reply from the server (HTTP {code}).",
            ],
        ),
        "err.not_jmap" => pick(
            lang,
            [
                "This server does not answer like a JMAP server.",
                "Dieser Server antwortet nicht wie ein JMAP-Server.",
                "Ce serveur ne répond pas comme un serveur JMAP.",
                "This server does not speak JMAP.",
            ],
        ),
        "err.no_api_url" => pick(
            lang,
            [
                "JMAP session without apiUrl.",
                "JMAP-Sitzung ohne apiUrl.",
                "Session JMAP sans apiUrl.",
                "JMAP session without apiUrl.",
            ],
        ),
        "err.jmap" => pick(
            lang,
            [
                "JMAP error (HTTP {code}).",
                "JMAP-Fehler (HTTP {code}).",
                "Erreur JMAP (HTTP {code}).",
                "JMAP error (HTTP {code}).",
            ],
        ),
        "err.app_down" => pick(
            lang,
            [
                "The server answers but the application is unavailable (HTTP {code}).",
                "Der Server antwortet, aber die Anwendung ist nicht verfügbar (HTTP {code}).",
                "Le serveur répond mais l'application est indisponible (HTTP {code}).",
                "The server answers but the app be sunk (HTTP {code}).",
            ],
        ),
        "err.timeout" => pick(
            lang,
            [
                "The server is not answering (timed out).",
                "Der Server antwortet nicht (Zeitüberschreitung).",
                "Le serveur ne répond pas (délai dépassé).",
                "The server be not answering (timed out).",
            ],
        ),
        "err.refused" => pick(
            lang,
            [
                "Connection refused, or the server is off.",
                "Verbindung abgelehnt oder Server ausgeschaltet.",
                "Connexion refusée ou serveur éteint.",
                "Connection refused, or the server be asleep.",
            ],
        ),
        "err.scheme" => pick(
            lang,
            [
                "The address must start with http:// or https://",
                "Die Adresse muss mit http:// oder https:// beginnen.",
                "L'adresse doit commencer par http:// ou https://",
                "The address must start with http:// or https://",
            ],
        ),
        "err.need_password" => pick(
            lang,
            [
                "Enter an app password.",
                "Gib ein App-Passwort ein.",
                "Saisissez un mot de passe d'application.",
                "Enter a spare key.",
            ],
        ),
        "err.save_password" => pick(
            lang,
            [
                "Could not save the password: {e}",
                "Das Passwort konnte nicht gespeichert werden: {e}",
                "Impossible d'enregistrer le mot de passe : {e}",
                "Could not stash the key: {e}",
            ],
        ),
        "err.no_server" => pick(
            lang,
            [
                "No server address is configured.",
                "Es ist keine Serveradresse eingerichtet.",
                "Aucune adresse de serveur n'est configurée.",
                "No server address be set.",
            ],
        ),
        "err.window" => pick(
            lang,
            [
                "Could not open the window: {e}",
                "Das Fenster konnte nicht geöffnet werden: {e}",
                "Impossible d'ouvrir la fenêtre : {e}",
                "Could not open the hatch: {e}",
            ],
        ),
        // Menu bar of the mail window
        "menu.file" => pick(lang, ["&File", "&Datei", "&Fichier", "&File"]),
        "menu.view" => pick(lang, ["&View", "&Ansicht", "&Affichage", "&View"]),
        "menu.go" => pick(lang, ["&Go", "&Gehe zu", "&Aller à", "&Go"]),
        "file.new_message" => pick(
            lang,
            [
                "New &message",
                "Neue &Nachricht",
                "Nouveau &message",
                "New &message",
            ],
        ),
        "file.check" => pick(
            lang,
            [
                "&Sync now",
                "Jetzt &synchronisieren",
                "&Synchroniser maintenant",
                "&Scan the horizon",
            ],
        ),
        "file.print" => pick(lang, ["&Print…", "&Drucken …", "&Imprimer…", "&Print…"]),
        "file.close" => pick(
            lang,
            [
                "&Close window",
                "Fenster &schließen",
                "&Fermer la fenêtre",
                "&Close window",
            ],
        ),
        "file.quit" => pick(lang, ["&Quit", "&Beenden", "&Quitter", "&Abandon ship"]),
        "view.reload" => pick(lang, ["&Reload", "&Neu laden", "&Actualiser", "&Reload"]),
        "view.back" => pick(lang, ["&Back", "&Zurück", "&Page précédente", "&Back"]),
        "view.forward" => pick(lang, ["&Forward", "&Vor", "Page &suivante", "&Forward"]),
        "view.zoom_in" => pick(lang, ["Zoom &in", "&Vergrößern", "Zoom &avant", "Zoom &in"]),
        "view.zoom_out" => pick(
            lang,
            ["Zoom &out", "&Verkleinern", "Zoom a&rrière", "Zoom &out"],
        ),
        "view.zoom_reset" => pick(
            lang,
            [
                "&Actual size",
                "&Originalgröße",
                "Taille &normale",
                "&Actual size",
            ],
        ),
        "view.fullscreen" => pick(
            lang,
            ["&Full screen", "&Vollbild", "&Plein écran", "&Full screen"],
        ),
        "go.inbox" => pick(
            lang,
            ["&Inbox", "&Posteingang", "Boîte de &réception", "&Inbox"],
        ),
        "go.starred" => pick(lang, ["&Starred", "&Markiert", "&Suivis", "&Starred"]),
        "go.snoozed" => pick(
            lang,
            ["S&noozed", "&Zurückgestellt", "&Reportés", "S&noozed"],
        ),
        "go.drafts" => pick(lang, ["&Drafts", "&Entwürfe", "&Brouillons", "&Drafts"]),
        "go.sent" => pick(lang, ["S&ent", "&Gesendet", "&Envoyés", "S&ent"]),
        "go.archive" => pick(lang, ["A&rchive", "&Archiv", "A&rchives", "A&rchive"]),
        "go.spam" => pick(lang, ["S&pam", "S&pam", "&Indésirables", "S&pam"]),
        "go.trash" => pick(lang, ["&Trash", "Papier&korb", "&Corbeille", "&Trash"]),
        "go.all" => pick(
            lang,
            [
                "A&ll mail",
                "A&lle Mails",
                "&Tous les messages",
                "A&ll mail",
            ],
        ),
        "go.calendar" => pick(lang, ["Calen&dar", "&Kalender", "A&genda", "Calen&dar"]),
        "go.contacts" => pick(lang, ["C&ontacts", "K&ontakte", "C&ontacts", "C&ontacts"]),
        "tools.filters" => pick(
            lang,
            [
                "Mail &filters",
                "Mail-&Filter",
                "&Filtres de messages",
                "Mail &filters",
            ],
        ),
        "tools.templates" => pick(lang, ["&Templates", "&Vorlagen", "&Modèles", "&Templates"]),
        "tools.integrations" => pick(
            lang,
            [
                "&Integrations",
                "&Integrationen",
                "&Intégrations",
                "&Integrations",
            ],
        ),
        "tools.app_passwords" => pick(
            lang,
            [
                "&App passwords",
                "App-&Passwörter",
                "Mots de passe d'&application",
                "&Spare keys",
            ],
        ),
        "help.shortcuts" => pick(
            lang,
            [
                "&Keyboard shortcuts",
                "&Tastenkürzel",
                "&Raccourcis clavier",
                "&Keyboard shortcuts",
            ],
        ),
        "help.whats_new" => pick(
            lang,
            ["&What's new", "&Neuigkeiten", "&Nouveautés", "&What's new"],
        ),
        "help.data_folder" => pick(
            lang,
            [
                "Open &data folder",
                "&Datenordner öffnen",
                "Ouvrir le dossier de &données",
                "Open &data folder",
            ],
        ),
        "menu.check_updates" => pick(
            lang,
            [
                "Check for &updates…",
                "Nach &Updates suchen …",
                "Rechercher des &mises à jour…",
                "Check for &updates…",
            ],
        ),
        "toast.update_title" => pick(
            lang,
            [
                "plMail Desktop {version} is available",
                "plMail Desktop {version} ist verfügbar",
                "plMail Desktop {version} est disponible",
                "plMail Desktop {version} be ready to sail",
            ],
        ),
        "toast.update_body" => pick(
            lang,
            [
                "Click to see what's new and install it.",
                "Klicke, um die Neuerungen zu sehen und zu installieren.",
                "Cliquez pour voir les nouveautés et l'installer.",
                "Click to see what's new and install it.",
            ],
        ),
        "err.update" => pick(
            lang,
            [
                "Could not check for updates: {e}",
                "Die Suche nach Updates ist fehlgeschlagen: {e}",
                "Impossible de rechercher les mises à jour : {e}",
                "Could not check for updates: {e}",
            ],
        ),
        "err.update_portable" => pick(
            lang,
            [
                "This copy was not installed by the installer: download the new version from the release page.",
                "Diese Kopie wurde nicht mit dem Installer installiert: lade die neue Version von der Release-Seite herunter.",
                "Cette copie n'a pas été installée par l'installeur : téléchargez la nouvelle version depuis la page de la release.",
                "This copy was not installed by the installer: download the new version from the release page.",
            ],
        ),
        "toast.received_one" => pick(
            lang,
            [
                "{account} received 1 new message",
                "{account} hat 1 neue Nachricht erhalten",
                "{account} a reçu 1 nouveau message",
                "{account} found 1 new bottle",
            ],
        ),
        "toast.from" => pick(
            lang,
            [
                "from \"{sender}\"",
                "von „{sender}“",
                "de « {sender} »",
                "from \"{sender}\"",
            ],
        ),
        "toast.btn_reply" => pick(lang, ["Reply", "Antworten", "Répondre", "Answer back"]),
        "toast.btn_read" => pick(lang, ["Mark read", "Gelesen", "Marquer lu", "Mark read"]),
        "toast.btn_delete" => pick(lang, ["Delete", "Löschen", "Supprimer", "Toss it"]),
        "toast.action_failed" => pick(
            lang,
            [
                "The action failed: {e}",
                "Die Aktion ist fehlgeschlagen: {e}",
                "L'action a échoué : {e}",
                "The action failed: {e}",
            ],
        ),
        _ => "",
    }
}

/// Fills `{name}` placeholders.
pub fn fill(text: &str, values: &[(&str, &str)]) -> String {
    values.iter().fold(text.to_string(), |acc, (k, v)| {
        acc.replace(&format!("{{{k}}}"), v)
    })
}
