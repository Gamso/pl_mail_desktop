mod altkey;
mod badge;
#[cfg(windows)]
mod default_app;
mod i18n;
#[cfg(test)]
mod i18n_tests;
mod jmap;
#[cfg(test)]
mod jmap_tests;
mod links;
#[cfg(test)]
mod links_tests;
mod mailto;
#[cfg(test)]
mod mailto_tests;

use std::sync::atomic::{AtomicI32, Ordering};
use std::{path::PathBuf, sync::Arc, time::Duration};

use serde::{Deserialize, Serialize};
use tauri::webview::NewWindowResponse;
use tauri::{
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent,
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::{Mutex, Notify};

use i18n::{Lang, fill, tr};
use jmap::{AccountCursor, Client};

/// Working name of the client, shown in titles, the About page and the tray.
const APP_NAME: &str = "plMail Desktop";
const KEYRING_SERVICE: &str = "plmail-client";
const KEYRING_USER: &str = "app-password";
const MIN_INTERVAL: u64 = 15;

/// Injected in every frame of the mail window, the mail body's included.
///
/// WebView2 reports `window.open()` to the application but not a click on a `target="_blank"`
/// link, which pl_mail puts on every link of a mail: those clicks would do nothing at all. Turning
/// the click into the equivalent `window.open()` sends it through `on_new_window`.
const OPEN_LINKS_SCRIPT: &str = r#"
addEventListener('click', function (event) {
  if (event.defaultPrevented || event.button !== 0) return;
  var link = event.target instanceof Element ? event.target.closest('a[href]') : null;
  if (!link || (link.getAttribute('target') || '').toLowerCase() !== '_blank') return;
  event.preventDefault();
  window.open(link.href, '_blank');
}, true);
"#;

/// Zoom of the mail window in percent, shared by the menu entries.
static ZOOM: AtomicI32 = AtomicI32::new(100);

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
struct Settings {
    server_url: String,
    interval_secs: u64,
    accept_invalid_certs: bool,
    start_with_windows: bool,
    /// Offer this application to Windows as a mail app for `mailto:` links.
    handle_mailto: bool,
    /// `auto` follows Windows; otherwise one of plMail's locale codes (`en`, `de`, `fr`, `pi`).
    language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server_url: String::new(),
            interval_secs: 60,
            accept_invalid_certs: false,
            start_with_windows: true,
            handle_mailto: false,
            language: "auto".to_string(),
        }
    }
}

/// What the settings window should show next: `settings`, `about` or `unreachable`.
#[derive(Serialize, Clone, Default)]
struct ViewRequest {
    view: String,
    message: String,
}

/// What the settings window shows beside the connection: `ok`, `error` or `none`.
#[derive(Serialize, Clone, Default)]
struct StatusInfo {
    kind: String,
    text: String,
}

#[derive(Default)]
struct Shared {
    status: Mutex<StatusInfo>,
    poller: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    check_now: Arc<Notify>,
    pending_view: std::sync::Mutex<Option<ViewRequest>>,
}

fn settings_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("settings.json")
}

fn load_settings(app: &AppHandle) -> Settings {
    std::fs::read_to_string(settings_path(app))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn password() -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .ok()?
        .get_password()
        .ok()
}

fn lang(app: &AppHandle) -> Lang {
    Lang::resolve(&load_settings(app).language)
}

fn configured(app: &AppHandle) -> bool {
    password().is_some() && !load_settings(app).server_url.is_empty()
}

fn show_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        // A dialog over the mail: the mail stays out of reach until it is closed.
        set_mail_enabled(app, false);
        center_on_mail(app, &w);
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Centres a dialog over the mail window, or on the screen when there is no mail window showing.
fn center_on_mail(app: &AppHandle, dialog: &tauri::WebviewWindow) {
    let mail = app
        .get_webview_window("web")
        .filter(|m| m.is_visible().unwrap_or(false) && !m.is_minimized().unwrap_or(false));
    let placed = mail.and_then(|m| {
        let (pos, size) = (m.outer_position().ok()?, m.outer_size().ok()?);
        let own = dialog.outer_size().ok()?;
        let mut x = pos.x + (size.width as i32 - own.width as i32) / 2;
        let mut y = pos.y + (size.height as i32 - own.height as i32) / 2;
        // Stay on the screen the mail window is on, even when it is half off it.
        if let Ok(Some(screen)) = m.current_monitor() {
            let (origin, extent) = (screen.position(), screen.size());
            x = x.clamp(
                origin.x,
                (origin.x + extent.width as i32 - own.width as i32).max(origin.x),
            );
            y = y.clamp(
                origin.y,
                (origin.y + extent.height as i32 - own.height as i32).max(origin.y),
            );
        }
        dialog.set_position(tauri::PhysicalPosition::new(x, y)).ok()
    });
    if placed.is_none() {
        let _ = dialog.center();
    }
}

/// Closes the settings/about/error window and hands the mail window its input and the focus back.
fn hide_main(app: &AppHandle) {
    // Re-enable the mail window before hiding the dialog: Windows hands the focus to the next
    // window it can activate, and a disabled one is skipped in favour of some other application.
    set_mail_enabled(app, true);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    if let Some(mail) = app.get_webview_window("web")
        && mail.is_visible().unwrap_or(false)
        && !mail.is_minimized().unwrap_or(false)
    {
        let _ = mail.set_focus();
    }
}

fn set_mail_enabled(app: &AppHandle, enabled: bool) {
    if let Some(w) = app.get_webview_window("web") {
        let _ = w.set_enabled(enabled);
    }
}

/// Brings up the settings window on one of its pages.
fn show_view(app: &AppHandle, view: &str, message: &str) {
    *app.state::<Shared>().pending_view.lock().unwrap() = Some(ViewRequest {
        view: view.to_string(),
        message: message.to_string(),
    });
    // The page re-reads the request itself; the event only tells a loaded page to do so.
    let _ = app.emit_to("main", "view", ());
    show_window(app);
}

async fn set_status(app: &AppHandle, ok: bool, text: &str) {
    *app.state::<Shared>().status.lock().await = StatusInfo {
        kind: if ok { "ok" } else { "error" }.to_string(),
        text: text.to_string(),
    };
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(format!("{APP_NAME} — {text}")));
    }
}

/// A toast whose click brings up the mail window. Unpackaged dev builds borrow
/// PowerShell's identity, otherwise Windows would not show the toast at all.
#[cfg(windows)]
fn notify(app: &AppHandle, title: &str, body: &str, target: Option<String>) {
    let app_id = if cfg!(debug_assertions) {
        tauri_winrt_notification::Toast::POWERSHELL_APP_ID
    } else {
        "fr.gamso.plmailclient"
    };
    let handle = app.clone();
    let shown = tauri_winrt_notification::Toast::new(app_id)
        .title(title)
        .text1(body)
        .on_activated(move |_| {
            open_mail_at(&handle, target.clone());
            Ok(())
        })
        .show();
    if let Err(e) = shown {
        eprintln!("toast failed: {e}");
    }
}

#[cfg(not(windows))]
fn notify(_app: &AppHandle, _title: &str, _body: &str, _target: Option<String>) {}

fn notify_mails(app: &AppHandle, mails: &[jmap::NewMail]) {
    if mails.len() > 3 {
        let text = fill(
            tr(lang(app), "toast.many"),
            &[("n", &mails.len().to_string())],
        );
        // Several at once: land on the inbox, where they all are.
        notify(app, "plMail", &text, Some("/mail/inbox".to_string()));
        return;
    }
    for m in mails {
        // pl_mail's address for a message is its JMAP id; anything that is not plain digits
        // would not match its route anyway, so it is not turned into a link.
        let target =
            m.id.chars()
                .all(|c| c.is_ascii_digit())
                .then(|| format!("/mail/message/{}", m.id));
        notify(
            app,
            &format!("{} — {}", m.from, m.subject),
            &m.preview,
            target,
        );
    }
}

async fn run_poller(app: AppHandle) {
    let settings = load_settings(&app);
    let Some(token) = password() else { return };
    let interval = Duration::from_secs(settings.interval_secs.max(MIN_INTERVAL));
    let ui = Lang::resolve(&settings.language);
    let client = match Client::new(
        &settings.server_url,
        &token,
        settings.accept_invalid_certs,
        ui,
    ) {
        Ok(c) => c,
        Err(e) => return set_status(&app, false, &e).await,
    };
    let check_now = app.state::<Shared>().check_now.clone();

    let mut api_url = String::new();
    let mut cursors: Vec<AccountCursor> = vec![];
    let mut unread: u64 = 0;

    loop {
        let result: Result<usize, String> = async {
            if cursors.is_empty() {
                let session = client.session().await?;
                api_url = session.api_url;
                cursors = session
                    .mail_accounts
                    .into_iter()
                    .map(|account_id| AccountCursor {
                        account_id,
                        state: None,
                        inbox_id: None,
                    })
                    .collect();
            }
            let mut total = 0;
            unread = 0;
            for cur in cursors.iter_mut() {
                let mails = client.new_mail(&api_url, cur).await?;
                total += mails.len();
                unread += client.unread(&api_url, cur).await.unwrap_or(0);
                if !mails.is_empty() {
                    notify_mails(&app, &mails);
                }
            }
            Ok(total)
        }
        .await;

        match result {
            Ok(n) => {
                let mut text = fill(
                    tr(ui, "status.ok"),
                    &[("unread", &unread.to_string()), ("time", &now_hhmm())],
                );
                if n > 0 {
                    text.push_str(&fill(tr(ui, "status.new"), &[("n", &n.to_string())]));
                }
                badge::set(&app, unread);
                set_status(&app, true, &text).await;
            }
            Err(e) => {
                // Forget the session so the next round re-reads apiUrl and accounts.
                cursors.clear();
                set_status(&app, false, &e).await;
            }
        }

        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = check_now.notified() => {}
        }
    }
}

fn now_hhmm() -> String {
    chrono::Local::now().format("%H:%M").to_string()
}

fn restart_poller(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let shared = app.state::<Shared>();
        let mut slot = shared.poller.lock().await;
        if let Some(h) = slot.take() {
            h.abort();
        }
        if configured(&app) {
            *slot = Some(tauri::async_runtime::spawn(run_poller(app.clone())));
        }
    });
}

#[derive(Serialize)]
struct SettingsView {
    #[serde(flatten)]
    settings: Settings,
    has_password: bool,
    /// The language the windows should use right now, `auto` already resolved.
    ui_language: &'static str,
}

#[tauri::command]
fn get_settings(app: AppHandle) -> SettingsView {
    let settings = load_settings(&app);
    SettingsView {
        ui_language: Lang::resolve(&settings.language).code(),
        settings,
        has_password: password().is_some(),
    }
}

#[tauri::command]
async fn get_status(app: AppHandle) -> StatusInfo {
    let status = app.state::<Shared>().status.lock().await.clone();
    if status.text.is_empty() {
        StatusInfo {
            kind: "none".to_string(),
            text: tr(lang(&app), "status.none").to_string(),
        }
    } else {
        status
    }
}

#[derive(Serialize)]
struct About {
    name: &'static str,
    version: String,
}

#[tauri::command]
fn get_about(app: AppHandle) -> About {
    About {
        name: APP_NAME,
        version: app.package_info().version.to_string(),
    }
}

/// Hands the page whatever `show_view` queued; a freshly opened window gets the
/// settings page, which is also the first-run welcome.
#[tauri::command]
fn take_view(app: AppHandle) -> ViewRequest {
    app.state::<Shared>()
        .pending_view
        .lock()
        .unwrap()
        .take()
        .unwrap_or(ViewRequest {
            view: "settings".to_string(),
            message: String::new(),
        })
}

/// Checks the credentials against the server first, so a typo never gets saved.
#[tauri::command]
async fn save_settings(
    app: AppHandle,
    mut settings: Settings,
    app_password: Option<String>,
) -> Result<(), String> {
    settings.server_url = settings.server_url.trim().trim_end_matches('/').to_string();
    let ui = Lang::resolve(&settings.language);
    if !settings.server_url.starts_with("http://") && !settings.server_url.starts_with("https://") {
        return Err(tr(ui, "err.scheme").to_string());
    }
    settings.interval_secs = settings.interval_secs.max(MIN_INTERVAL);

    let token = match app_password.filter(|p| !p.is_empty()) {
        Some(p) => p,
        None => password().ok_or_else(|| tr(ui, "err.need_password").to_string())?,
    };
    let client = Client::new(
        &settings.server_url,
        &token,
        settings.accept_invalid_certs,
        ui,
    )?;
    client.session().await?;

    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .and_then(|e| e.set_password(&token))
        .map_err(|e| fill(tr(ui, "err.save_password"), &[("e", &e.to_string())]))?;
    let path = settings_path(&app);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&settings).unwrap())
        .map_err(|e| e.to_string())?;

    let launcher = app.autolaunch();
    let _ = if settings.start_with_windows {
        launcher.enable()
    } else {
        launcher.disable()
    };

    #[cfg(windows)]
    {
        let registered = if settings.handle_mailto {
            std::env::current_exe().and_then(|exe| {
                default_app::register(&exe, APP_NAME, "Opens mailto: links in your pl_mail inbox")
            })
        } else {
            default_app::unregister()
        };
        if let Err(e) = registered {
            eprintln!("mailto registration failed: {e}");
        }
    }

    // The mail window may point at the old address or certificate rules.
    if let Some(w) = app.get_webview_window("web") {
        let _ = w.destroy();
    }
    refresh_tray_menu(&app);
    restart_poller(&app);
    hide_main(&app);
    open_mail(&app);
    Ok(())
}

#[tauri::command]
fn check_now(app: AppHandle) {
    app.state::<Shared>().check_now.notify_one();
}

#[tauri::command]
fn send_test_notification(app: AppHandle) {
    notify(&app, "plMail", tr(lang(&app), "toast.test"), None);
}

/// Windows will not let an application make itself the default mail app, so the user picks
/// it in its own settings page.
#[tauri::command]
fn open_default_apps(app: AppHandle) {
    let _ = app
        .opener()
        .open_url("ms-settings:defaultapps", None::<&str>);
}

#[tauri::command]
fn retry_open(app: AppHandle) {
    open_mail(&app);
}

/// Carries out what `links::classify` decided for a link that asked for a window of its own.
fn follow_link(app: &AppHandle, server: &tauri::Url, target: &tauri::Url) {
    match links::classify(server, target) {
        links::LinkAction::OpenInBrowser(url) => {
            let _ = app.opener().open_url(url.as_str(), None::<&str>);
        }
        links::LinkAction::OpenHere(url) => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Some(w) = app.get_webview_window("web") {
                    let _ = w.navigate(url);
                }
            });
        }
        links::LinkAction::Compose(path) => open_mail_at(app, Some(path)),
        links::LinkAction::Ignore => {}
    }
}

/// What launching the app again should show: the mail, or the setup if there is none yet.
fn show_main(app: &AppHandle) {
    if configured(app) {
        open_mail(app);
    } else {
        show_view(app, "settings", "");
    }
}

/// Shows the pl_mail web UI in its own window, or the unreachable page when the
/// server does not answer. The mail window loads a remote page, so it is
/// deliberately absent from every capability: it cannot call our commands.
fn open_mail(app: &AppHandle) {
    open_mail_at(app, None);
}

/// Same, landing on one of pl_mail's pages (`/mail/message/42`) instead of wherever it was.
fn open_mail_at(app: &AppHandle, path: Option<String>) {
    let app = app.clone();
    // Windows deadlocks when a window is built from the event-loop thread.
    tauri::async_runtime::spawn(async move {
        if let Err(e) = open_mail_window(&app, path).await {
            eprintln!("[open] failed: {e}");
            show_view(&app, "unreachable", &e);
        }
    });
}

async fn open_mail_window(app: &AppHandle, path: Option<String>) -> Result<(), String> {
    let settings = load_settings(app);
    let ui = Lang::resolve(&settings.language);
    let url = settings
        .server_url
        .parse::<tauri::Url>()
        .map_err(|_| tr(ui, "err.no_server").to_string())?;
    // Where to land: the page asked for, on the configured server.
    let server = url.clone();
    let links_app = app.clone();
    let landing = path.as_deref().and_then(|p| {
        format!("{}{p}", settings.server_url)
            .parse::<tauri::Url>()
            .ok()
    });
    if let Some(w) = app.get_webview_window("web") {
        if app
            .get_webview_window("main")
            .and_then(|m| m.is_visible().ok())
            .unwrap_or(false)
        {
            show_window(app);
            return Ok(());
        }
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        if let Some(target) = landing {
            let _ = w.navigate(target);
        }
        return Ok(());
    }
    jmap::probe(&settings.server_url, settings.accept_invalid_certs, ui).await?;

    let mut args = String::from("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection");
    if settings.accept_invalid_certs {
        args.push_str(" --ignore-certificate-errors");
    }
    let menu = mail_menu(app, ui).map_err(|e| e.to_string())?;

    let window =
        WebviewWindowBuilder::new(app, "web", WebviewUrl::External(landing.unwrap_or(url)))
            .title(APP_NAME)
            .inner_size(1280.0, 820.0)
            .menu(menu)
            .initialization_script_for_all_frames(OPEN_LINKS_SCRIPT)
            // Links in a mail ask for a window of their own; a bare window would ignore them.
            .on_new_window(move |target, _| {
                follow_link(&links_app, &server, &target);
                NewWindowResponse::Deny
            })
            .additional_browser_args(&args)
            // WebView2 refuses two windows on one data folder with different launch
            // options; the settings window has its own, so this one gets a folder too.
            .data_directory(
                app.path()
                    .app_local_data_dir()
                    .unwrap_or_default()
                    .join("webview-plmail"),
            )
            .build()
            .map_err(|e| fill(tr(ui, "err.window"), &[("e", &e.to_string())]))?;

    // The menu bar stays out of sight until Alt is pressed.
    let _ = window.hide_menu();
    altkey::attach(&window);
    // Forces the next poll to repaint the taskbar badge on the new window.
    badge::LAST.store(u64::MAX, Ordering::Relaxed);
    hide_main(app);
    Ok(())
}

/// The menu bar of the mail window (shown with Alt), laid out like Thunderbird's. Item ids
/// starting with `go:` carry the pl_mail page to open (mail folders sit under `/mail`, the
/// settings pages are sections of `/settings`); `act:` ones are handled by `handle_menu`.
/// Thunderbird's Message menu is left out on purpose: it acts on the selected message, which
/// lives inside the web page where this application has no hands.
fn mail_menu(app: &AppHandle, ui: Lang) -> tauri::Result<Menu<tauri::Wry>> {
    let item = |id: &str, key: &str| MenuItem::with_id(app, id, tr(ui, key), true, None::<&str>);
    let sep = || PredefinedMenuItem::separator(app);
    let about = MenuItem::with_id(
        app,
        "menu-about",
        fill(tr(ui, "menu.about"), &[("name", APP_NAME)]),
        true,
        None::<&str>,
    )?;
    let sub = |key: &str, items: &[&dyn tauri::menu::IsMenuItem<tauri::Wry>]| {
        Submenu::with_items(app, tr(ui, key), true, items)
    };

    let file = sub(
        "menu.file",
        &[
            &item("act:new-message", "file.new_message")?,
            &item("act:check", "file.check")?,
            &sep()?,
            &item("act:print", "file.print")?,
            &sep()?,
            &item("act:close", "file.close")?,
            &item("act:quit", "file.quit")?,
        ],
    )?;
    let view = sub(
        "menu.view",
        &[
            &item("act:reload", "view.reload")?,
            &item("act:back", "view.back")?,
            &item("act:forward", "view.forward")?,
            &sep()?,
            &item("act:zoom-in", "view.zoom_in")?,
            &item("act:zoom-out", "view.zoom_out")?,
            &item("act:zoom-reset", "view.zoom_reset")?,
            &sep()?,
            &item("act:fullscreen", "view.fullscreen")?,
        ],
    )?;
    let go = sub(
        "menu.go",
        &[
            &item("go:/mail/inbox", "go.inbox")?,
            &item("go:/mail/starred", "go.starred")?,
            &item("go:/mail/snoozed", "go.snoozed")?,
            &item("go:/mail/drafts", "go.drafts")?,
            &item("go:/mail/sent", "go.sent")?,
            &item("go:/mail/archive", "go.archive")?,
            &item("go:/mail/spam", "go.spam")?,
            &item("go:/mail/trash", "go.trash")?,
            &item("go:/mail/all", "go.all")?,
            &sep()?,
            &item("go:/calendar", "go.calendar")?,
            &item("go:/contacts", "go.contacts")?,
        ],
    )?;
    let tools = sub(
        "menu.tools",
        &[
            &item("go:/settings?section=filters", "tools.filters")?,
            &item("go:/settings?section=templates", "tools.templates")?,
            &item("go:/settings?section=integrations", "tools.integrations")?,
            &item("go:/settings?section=app-passwords", "tools.app_passwords")?,
            &sep()?,
            &item("menu-plmail-settings", "menu.plmail_settings")?,
            &item("menu-admin", "menu.admin")?,
            &sep()?,
            &item("menu-client-settings", "menu.client_settings")?,
        ],
    )?;
    let help = sub(
        "menu.help",
        &[
            &item("act:shortcuts", "help.shortcuts")?,
            &item("go:/whats-new", "help.whats_new")?,
            &sep()?,
            &item("act:data-folder", "help.data_folder")?,
            &sep()?,
            &about,
        ],
    )?;
    Menu::with_items(app, &[&file, &view, &go, &tools, &help])
}

/// What a click on the mail window's menu bar does.
fn handle_menu(app: &AppHandle, id: &str) {
    let web = app.get_webview_window("web");
    // Whatever is picked, the bar goes away again, as in Thunderbird.
    if let Some(w) = &web {
        let _ = w.hide_menu();
    }
    if let Some(path) = id.strip_prefix("go:") {
        return open_web_page(app, path);
    }
    match id {
        "menu-plmail-settings" => open_web_page(app, "/settings"),
        "menu-admin" => open_web_page(app, "/admin"),
        "menu-client-settings" => show_view(app, "settings", ""),
        "menu-about" => show_view(app, "about", ""),
        "act:check" => sync_now(app),
        "act:quit" => app.exit(0),
        "act:data-folder" => {
            if let Ok(dir) = app.path().app_config_dir() {
                let _ = app.opener().open_path(dir.to_string_lossy(), None::<&str>);
            }
        }
        _ => {
            let Some(w) = web else { return };
            match id {
                // The sidebar's own link opens the composer in its dock; fall back to the inbox
                // when the page being shown has no sidebar.
                "act:new-message" => {
                    let _ = w.eval(
                        "(document.querySelector('a[href$=\"/compose/new\"]') || { click() { location.href = '/inbox'; } }).click();",
                    );
                }
                // pl_mail's own, themed dialog; with shortcuts switched off it does not exist,
                // so land on the settings section where they can be switched back on.
                "act:shortcuts" => {
                    let _ = w.eval(
                        "(document.querySelector(\"[data-shortcut='help']\") || { click() { location.href = '/settings?section=shortcuts'; } }).click();",
                    );
                }
                "act:print" => {
                    let _ = w.print();
                }
                "act:close" => {
                    let _ = w.hide();
                }
                "act:reload" => {
                    let _ = w.reload();
                }
                "act:back" => {
                    let _ = w.eval("history.back();");
                }
                "act:forward" => {
                    let _ = w.eval("history.forward();");
                }
                "act:zoom-in" => set_zoom(&w, ZOOM.load(Ordering::Relaxed) + 10),
                "act:zoom-out" => set_zoom(&w, ZOOM.load(Ordering::Relaxed) - 10),
                "act:zoom-reset" => set_zoom(&w, 100),
                "act:fullscreen" => {
                    let _ = w.set_fullscreen(!w.is_fullscreen().unwrap_or(false));
                }
                _ => {}
            }
        }
    }
}

/// "Sync now": presses pl_mail's own sync button, which makes the server fetch from Gmail,
/// IMAP and the others, then re-reads the server a little later so the badge and the
/// notifications catch up. Without the mail window open there is no button to press, so
/// only the re-read happens.
fn sync_now(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("web") {
        let _ = w.eval("document.querySelector('[data-sync-now-bar]')?.click();");
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let check = app.state::<Shared>().check_now.clone();
        check.notify_one();
        tokio::time::sleep(Duration::from_secs(8)).await;
        check.notify_one();
    });
}

fn set_zoom(w: &tauri::WebviewWindow, percent: i32) {
    let percent = percent.clamp(50, 300);
    ZOOM.store(percent, Ordering::Relaxed);
    let _ = w.set_zoom(f64::from(percent) / 100.0);
}

fn tray_menu(app: &AppHandle, ui: Lang) -> tauri::Result<Menu<tauri::Wry>> {
    let item = |id: &str, key: &str| MenuItem::with_id(app, id, tr(ui, key), true, None::<&str>);
    Menu::with_items(
        app,
        &[
            &item("open", "tray.open")?,
            &item("prefs", "tray.settings")?,
            &item("quit", "tray.quit")?,
        ],
    )
}

/// The tray menu is built once, so a language change has to swap it.
fn refresh_tray_menu(app: &AppHandle) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id("main"), tray_menu(app, lang(app))) {
        let _ = tray.set_menu(Some(menu));
    }
}

/// Opens one of pl_mail's own pages (its settings, its admin panel) in the mail window.
fn open_web_page(app: &AppHandle, path: &str) {
    let (app, path) = (app.clone(), path.to_string());
    tauri::async_runtime::spawn(async move {
        let base = load_settings(&app).server_url;
        if let (Ok(url), Some(w)) = (
            format!("{base}{path}").parse::<tauri::Url>(),
            app.get_webview_window("web"),
        ) {
            let _ = w.hide_menu();
            let _ = w.navigate(url);
            let _ = w.set_focus();
        }
    });
}

#[tauri::command]
fn close_dialog(app: AppHandle) {
    hide_main(&app);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            // A second launch is how Windows hands over a clicked `mailto:` link.
            match mailto::from_args(&args) {
                Some(path) if configured(app) => open_mail_at(app, Some(path)),
                _ => show_main(app),
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_opener::init())
        .manage(Shared::default())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            get_status,
            get_about,
            take_view,
            save_settings,
            check_now,
            send_test_notification,
            retry_open,
            open_default_apps,
            close_dialog
        ])
        // Menu bar of the mail window; the tray has its own handler.
        .on_menu_event(|app, event| handle_menu(app, event.id.as_ref()))
        .on_window_event(|window, event| {
            // Closing a window keeps the notifier running in the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if window.label() == "main" {
                    hide_main(window.app_handle());
                } else {
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            let handle = app.handle().clone();
            let menu = tray_menu(&handle, lang(&handle))?;

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip(APP_NAME)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_main(app),
                    "prefs" => show_view(app, "settings", ""),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            if configured(&handle) {
                restart_poller(&handle);
                let args: Vec<String> = std::env::args().collect();
                // Started by a `mailto:` link: open the composer. Started by Windows at logon:
                // stay in the tray. Started by the user: show the mail.
                if let Some(path) = mailto::from_args(&args) {
                    open_mail_at(&handle, Some(path));
                } else if !args.iter().any(|a| a == "--minimized") {
                    open_mail(&handle);
                }
            } else {
                show_view(&handle, "settings", "");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
