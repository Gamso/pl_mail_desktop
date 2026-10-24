// Text of the app's own windows, in the languages plMail ships:
// English, German, French and Pirate English (plMail's `en_PI`).
// The tray, the menu bar, the toasts and the errors are translated in src-tauri/src/i18n.rs.

export type Lang = "en" | "de" | "fr" | "pi";

export const LANGUAGES: { code: Lang; label: string }[] = [
  { code: "en", label: "English" },
  { code: "de", label: "Deutsch" },
  { code: "fr", label: "Français" },
  { code: "pi", label: "Pirate English" },
];

type Entry = Record<Lang, string>;

const T: Record<string, Entry> = {
  "settings.title_first": { en: "Welcome", de: "Willkommen", fr: "Bienvenue", pi: "Ahoy, matey" },
  "settings.title": { en: "Client settings", de: "Client-Einstellungen", fr: "Paramètres du client", pi: "Ship's settings" },
  "settings.lead_first": {
    en: "Connect this client to your pl_mail server to get notifications and read your mail.",
    de: "Verbinde diesen Client mit deinem pl_mail-Server, um Benachrichtigungen zu erhalten und deine Mails zu lesen.",
    fr: "Connectez ce client à votre serveur pl_mail pour recevoir les notifications et lire vos mails.",
    pi: "Link this ship to yer pl_mail server to hear the bells and read yer mail.",
  },
  "settings.lead": {
    en: "Change the server address, the app password or the language.",
    de: "Ändere die Serveradresse, das App-Passwort oder die Sprache.",
    fr: "Modifiez l'adresse du serveur, le mot de passe d'application ou la langue.",
    pi: "Change the server address, the spare key or the tongue.",
  },
  "field.server": { en: "Server address", de: "Serveradresse", fr: "Adresse du serveur", pi: "Server address" },
  "field.password": { en: "App password", de: "App-Passwort", fr: "Mot de passe d'application", pi: "Spare key" },
  "field.password_saved": {
    en: "•••• saved (leave empty to keep it)",
    de: "•••• gespeichert (leer lassen, um es zu behalten)",
    fr: "•••• enregistré (laisser vide pour le garder)",
    pi: "•••• stashed (leave empty to keep it)",
  },
  "hint.before": { en: "In pl_mail: ", de: "In pl_mail: ", fr: "Dans pl_mail : ", pi: "In pl_mail: " },
  "hint.path": {
    en: "Settings → App passwords",
    de: "Einstellungen → App-Passwörter",
    fr: "Réglages → Mots de passe d'application",
    pi: "Settings → Spare keys",
  },
  "hint.after": {
    en: ", create one and copy it right away, it is shown only once.",
    de: ", lege eines an und kopiere es sofort, es wird nur einmal angezeigt.",
    fr: ", créez-en un et copiez-le tout de suite, il n'est affiché qu'une fois.",
    pi: ", forge one and copy it at once, it be shown but once.",
  },
  "advanced": { en: "Advanced options", de: "Erweiterte Optionen", fr: "Options avancées", pi: "Fancy options" },
  "field.interval": {
    en: "Check for new mail every",
    de: "Auf neue Mails prüfen alle",
    fr: "Vérifier les nouveaux mails toutes les",
    pi: "Scan the horizon every",
  },
  "unit.seconds": { en: "s", de: "s", fr: "s", pi: "s" },
  "field.insecure": {
    en: "Accept a self-signed certificate",
    de: "Selbstsigniertes Zertifikat akzeptieren",
    fr: "Accepter un certificat auto-signé",
    pi: "Trust a home-forged certificate",
  },
  "field.autostart": { en: "Start with Windows", de: "Mit Windows starten", fr: "Démarrer avec Windows", pi: "Set sail with Windows" },
  "field.language": { en: "Language", de: "Sprache", fr: "Langue", pi: "Tongue" },
  "lang.auto": { en: "Automatic (Windows)", de: "Automatisch (Windows)", fr: "Automatique (Windows)", pi: "Automatic (Windows)" },
  "btn.connect": { en: "Connect", de: "Verbinden", fr: "Se connecter", pi: "Set sail" },
  "btn.save": { en: "Save", de: "Speichern", fr: "Enregistrer", pi: "Save" },
  "btn.connecting": { en: "Connecting…", de: "Verbinde …", fr: "Connexion…", pi: "Sailing…" },
  "btn.test": { en: "Test notifications", de: "Benachrichtigungen testen", fr: "Tester les notifications", pi: "Test the bells" },

  "about.version": { en: "Version", de: "Version", fr: "Version", pi: "Version" },
  "about.text": {
    en: "A desktop client for pl_mail: your mailbox in a window of its own, with Windows notifications and the number of unread messages on the icon.",
    de: "Ein Desktop-Client für pl_mail: dein Postfach in einem eigenen Fenster, mit Windows-Benachrichtigungen und der Zahl ungelesener Nachrichten am Symbol.",
    fr: "Client de bureau pour pl_mail : votre messagerie dans une fenêtre dédiée, avec les notifications Windows et le nombre de messages non lus sur l'icône.",
    pi: "A desktop ship for pl_mail: yer mailbox in a window o' its own, with Windows bells and the count o' unread bottles on the icon.",
  },
  "about.muted": {
    en: "pl_mail is free software (AGPL-3.0) by karatektus. This client is independent of the project.",
    de: "pl_mail ist freie Software (AGPL-3.0) von karatektus. Dieser Client ist unabhängig vom Projekt.",
    fr: "pl_mail est un logiciel libre (AGPL-3.0) de karatektus. Ce client est indépendant du projet.",
    pi: "pl_mail be free software (AGPL-3.0) by karatektus. This ship sails independent o' the project.",
  },
  "about.link": { en: "See pl_mail on GitHub", de: "pl_mail auf GitHub ansehen", fr: "Voir pl_mail sur GitHub", pi: "See pl_mail on GitHub" },
  "about.close": { en: "Close", de: "Schließen", fr: "Fermer", pi: "Close" },

  "unreachable.title": { en: "Server unreachable", de: "Server nicht erreichbar", fr: "Serveur injoignable", pi: "Server lost at sea" },
  "unreachable.lead": {
    en: "Could not open {host}.",
    de: "{host} konnte nicht geöffnet werden.",
    fr: "Impossible d'ouvrir {host}.",
    pi: "Could not reach {host}.",
  },
  "unreachable.tip1": {
    en: "Check your internet connection or VPN.",
    de: "Prüfe deine Internetverbindung oder das VPN.",
    fr: "Vérifiez votre connexion Internet ou le VPN.",
    pi: "Check yer internet connection or VPN.",
  },
  "unreachable.tip2": {
    en: "The server may be restarting: try again in a moment.",
    de: "Der Server startet vielleicht gerade neu: Versuche es gleich noch einmal.",
    fr: "Le serveur est peut-être en cours de redémarrage : réessayez dans un instant.",
    pi: "The server may be restarting: try again in a moment.",
  },
  "unreachable.tip3": {
    en: "If the address has changed, edit it in the settings.",
    de: "Hat sich die Adresse geändert, ändere sie in den Einstellungen.",
    fr: "Si l'adresse a changé, modifiez-la dans les paramètres.",
    pi: "If the address has changed, edit it in the settings.",
  },
  "unreachable.retry": { en: "Try again", de: "Erneut versuchen", fr: "Réessayer", pi: "Try again" },
  "unreachable.settings": { en: "Edit settings", de: "Einstellungen ändern", fr: "Modifier les paramètres", pi: "Edit settings" },
};

let current: Lang = "en";

export function systemLanguage(): Lang {
  const code = (navigator.language || "en").toLowerCase();
  if (code.startsWith("de")) return "de";
  if (code.startsWith("fr")) return "fr";
  return "en";
}

export function isLang(value: string): value is Lang {
  return LANGUAGES.some((l) => l.code === value);
}

export function t(key: string, values: Record<string, string> = {}): string {
  const text = T[key]?.[current] ?? T[key]?.en ?? key;
  return Object.entries(values).reduce((acc, [k, v]) => acc.split(`{${k}}`).join(v), text);
}

/** Sets the language and rewrites every element marked with a data-i18n attribute. */
export function applyLanguage(lang: Lang) {
  current = lang;
  document.documentElement.lang = lang === "pi" ? "en" : lang;
  document.querySelectorAll<HTMLElement>("[data-i18n]").forEach((el) => {
    el.textContent = t(el.dataset.i18n!, el.dataset.i18nHost ? { host: el.dataset.i18nHost } : {});
  });
}
