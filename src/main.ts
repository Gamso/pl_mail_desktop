import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { applyLanguage, isLang, Lang, LANGUAGES, systemLanguage, t } from "./i18n";
import "./styles.css";

interface Settings {
  server_url: string;
  interval_secs: number;
  accept_invalid_certs: boolean;
  start_with_windows: boolean;
  start_in_tray: boolean;
  handle_mailto: boolean;
  check_updates: boolean;
  language: string;
}
type SettingsView = Settings & { has_password: boolean; ui_language: Lang };
interface ViewRequest {
  view: "settings" | "about" | "unreachable" | "update";
  message: string;
}
interface UpdateInfo {
  state: "checking" | "available" | "uptodate" | "error" | "installing";
  current: string;
  version: string;
  notes: string;
  portable: boolean;
  message: string;
}
interface Status {
  kind: "ok" | "error" | "none";
  text: string;
}

const PROJECT_URL = "https://github.com/karatektus/pl_mail";
const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const views = ["settings", "about", "unreachable", "update"] as const;

let statusTimer: number | undefined;
let firstRun = true;
let lastUpdate: UpdateInfo | undefined;

function show(view: (typeof views)[number]) {
  for (const v of views) $(`view-${v}`).hidden = v !== view;
  window.clearInterval(statusTimer);
  if (view === "settings") {
    refreshStatus();
    statusTimer = window.setInterval(refreshStatus, 3000);
  }
}

async function refreshStatus() {
  const status = await invoke<Status>("get_status");
  const chip = $("status-chip");
  chip.hidden = status.kind === "none";
  chip.classList.toggle("ok", status.kind === "ok");
  chip.classList.toggle("bad", status.kind === "error");
  $("status").textContent = status.text;
}

/** Texts that depend on the language and on whether this is the first launch. */
function renderDynamicTexts() {
  $("settings-title").textContent = t(firstRun ? "settings.title_first" : "settings.title");
  $("settings-lead").textContent = t(firstRun ? "settings.lead_first" : "settings.lead");
  $("save").textContent = t(firstRun ? "btn.connect" : "btn.save");
  const password = $<HTMLInputElement>("app_password");
  password.placeholder = password.dataset.saved ? t("field.password_saved") : "plmail_…";
  const auto = document.querySelector<HTMLOptionElement>("#language option[value=auto]");
  if (auto) auto.textContent = t("lang.auto");
}

function setLanguage(lang: Lang) {
  applyLanguage(lang);
  renderDynamicTexts();
}

function languageFromSelect(): Lang {
  const value = $<HTMLSelectElement>("language").value;
  return isLang(value) ? value : systemLanguage();
}

function buildLanguageSelect() {
  const select = $<HTMLSelectElement>("language");
  if (select.options.length) return;
  select.append(new Option("", "auto"));
  for (const l of LANGUAGES) select.append(new Option(l.label, l.code));
}

/** "Start in the tray only" only means something when the application starts with Windows. */
function syncTrayOption() {
  $<HTMLInputElement>("start_in_tray").disabled = !$<HTMLInputElement>("start_with_windows").checked;
}

async function loadSettings() {
  buildLanguageSelect();
  const s = await invoke<SettingsView>("get_settings");
  firstRun = !s.has_password || !s.server_url;
  $<HTMLSelectElement>("language").value = s.language;
  $<HTMLInputElement>("server_url").value = s.server_url;
  $<HTMLInputElement>("interval_secs").value = String(s.interval_secs);
  $<HTMLInputElement>("accept_invalid_certs").checked = s.accept_invalid_certs;
  $<HTMLInputElement>("start_with_windows").checked = s.start_with_windows;
  $<HTMLInputElement>("start_in_tray").checked = s.start_in_tray;
  syncTrayOption();
  $<HTMLInputElement>("handle_mailto").checked = s.handle_mailto;
  $<HTMLInputElement>("check_updates").checked = s.check_updates;
  $("mailto-help").hidden = !s.handle_mailto;
  const password = $<HTMLInputElement>("app_password");
  if (s.has_password) password.dataset.saved = "1";
  else delete password.dataset.saved;
  $("error").textContent = "";
  setLanguage(s.ui_language);
  return s;
}

const UPDATE_TITLES = {
  checking: "update.title_checking",
  available: "update.title_available",
  uptodate: "update.title_uptodate",
  error: "update.title_error",
  installing: "update.title_installing",
} as const;

/** Shows the update dialog in the state the Rust side reports. */
function renderUpdate(info: UpdateInfo) {
  lastUpdate = info;
  const values = { version: info.version, current: info.current };
  const available = info.state === "available";
  $("update-title").textContent = t(UPDATE_TITLES[info.state]);
  $("update-lead").textContent =
    available && info.portable
      ? t("update.lead_portable", values)
      : info.state === "available"
        ? t("update.lead_available", values)
        : info.state === "uptodate"
          ? t("update.lead_uptodate", values)
          : info.state === "installing"
            ? t("update.lead_installing", values)
            : "";
  $("update-notes").hidden = !(available && info.notes);
  $("update-notes-body").textContent = info.notes;
  $("update-progress-box").hidden = info.state !== "installing";
  $("update-message").textContent = info.message;
  $("update-install").hidden = !(available && !info.portable);
  $("update-open-page").hidden = !(available && info.portable);
  $("update-later").hidden = !available;
  $("update-skip").hidden = !available;
  $("update-retry").hidden = info.state !== "error";
  $("update-close").hidden = !["checking", "uptodate", "error"].includes(info.state);
}

async function route() {
  const req = await invoke<ViewRequest>("take_view");
  const s = await loadSettings();
  if (req.view === "about") {
    const about = await invoke<{ name: string; version: string }>("get_about");
    $("about-name").textContent = about.name;
    $("about-version").textContent = about.version;
  } else if (req.view === "unreachable") {
    const host = s.server_url.replace(/^https?:\/\//, "");
    $("unreachable-lead").textContent = t("unreachable.lead", { host });
    $("unreachable-detail").textContent = req.message;
  } else if (req.view === "update") {
    renderUpdate(await invoke<UpdateInfo>("get_update_info"));
  }
  show(req.view);
}

$("language").addEventListener("change", () => setLanguage(languageFromSelect()));

$("form").addEventListener("submit", async (e) => {
  e.preventDefault();
  const error = $("error");
  const save = $<HTMLButtonElement>("save");
  const password = $<HTMLInputElement>("app_password");
  error.textContent = "";
  save.disabled = true;
  const label = save.textContent;
  save.textContent = t("btn.connecting");
  try {
    await invoke("save_settings", {
      settings: {
        server_url: $<HTMLInputElement>("server_url").value,
        interval_secs: Number($<HTMLInputElement>("interval_secs").value) || 60,
        accept_invalid_certs: $<HTMLInputElement>("accept_invalid_certs").checked,
        start_with_windows: $<HTMLInputElement>("start_with_windows").checked,
        start_in_tray: $<HTMLInputElement>("start_in_tray").checked,
        handle_mailto: $<HTMLInputElement>("handle_mailto").checked,
        check_updates: $<HTMLInputElement>("check_updates").checked,
        language: $<HTMLSelectElement>("language").value,
      },
      appPassword: password.value || null,
    });
    password.value = "";
  } catch (err) {
    error.textContent = String(err);
    // A self-signed certificate is the usual reason to fail on a home server.
    if (/certificate|certificat|zertifikat|ssl|tls/i.test(String(err))) $<HTMLDetailsElement>("advanced").open = true;
  } finally {
    save.disabled = false;
    save.textContent = label;
  }
});

$("start_with_windows").addEventListener("change", syncTrayOption);
$("handle_mailto").addEventListener("change", () => {
  $("mailto-help").hidden = !$<HTMLInputElement>("handle_mailto").checked;
});
$("default-apps").addEventListener("click", () => invoke("open_default_apps"));
$("test").addEventListener("click", () => invoke("send_test_notification"));
$("retry").addEventListener("click", () => invoke("retry_open"));
$("to-settings").addEventListener("click", async () => {
  await loadSettings();
  show("settings");
});
$("about-link").addEventListener("click", () => openUrl(PROJECT_URL));
$("about-close").addEventListener("click", () => invoke("close_dialog"));

$("update-install").addEventListener("click", async () => {
  if (!lastUpdate) return;
  renderUpdate({ ...lastUpdate, state: "installing", message: "" });
  $<HTMLProgressElement>("update-progress").removeAttribute("value");
  $("update-progress-text").textContent = "";
  try {
    await invoke("install_update");
  } catch (err) {
    renderUpdate({ ...lastUpdate, state: "available", message: String(err) });
  }
});
$("update-later").addEventListener("click", () => invoke("close_dialog"));
$("update-close").addEventListener("click", () => invoke("close_dialog"));
$("update-skip").addEventListener("click", () => invoke("skip_update"));
$("update-open-page").addEventListener("click", () => invoke("open_release_page"));
$("update-retry").addEventListener("click", () => invoke("check_updates_now"));

listen<[number, number | null]>("update-progress", ({ payload: [done, total] }) => {
  const bar = $<HTMLProgressElement>("update-progress");
  if (!total) return;
  const percent = Math.min(100, Math.round((done / total) * 100));
  bar.value = percent;
  $("update-progress-text").textContent = `${percent} %`;
});

listen("view", route);
route();
