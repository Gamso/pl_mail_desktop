//! Updates: look for a newer release on GitHub, tell the user, and install it.
//!
//! The release workflow publishes a signed `latest.json` next to the installer. The address used
//! below, `releases/latest/download/…`, is resolved by GitHub to the newest release that is
//! neither a draft nor a pre-release, so betas are never offered. The updater plugin downloads the
//! installer, checks its signature against the public key in `tauri.conf.json`, and runs it in
//! update mode (`/UPDATE`): the old version is not uninstalled first, so shortcuts, the taskbar
//! pin, the settings and the autostart entry all survive.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::i18n::{fill, tr};

/// Where a copy that cannot update itself sends the user.
const RELEASES_URL: &str = "https://github.com/Gamso/pl_mail_desktop/releases/latest";
/// An automatic check happens at most this often.
const CHECK_EVERY_SECS: i64 = 24 * 60 * 60;
const NOTES_MAX: usize = 4000;

/// What the update dialog shows.
#[derive(Serialize, Clone, Default)]
pub struct UpdateInfo {
    /// `checking`, `available`, `uptodate`, `error` or `installing`.
    pub state: String,
    pub current: String,
    pub version: String,
    pub notes: String,
    /// A copy that was not put in place by the installer cannot install an update itself.
    pub portable: bool,
    pub message: String,
}

#[derive(Default)]
pub struct Pending {
    info: UpdateInfo,
    update: Option<Update>,
}

/// What survives between launches: when we last looked, and which version the user waved away.
#[derive(Serialize, Deserialize, Default)]
struct Memory {
    last_check: i64,
    skipped: String,
}

/// Only final releases are offered: `0.2.0-beta.1` is a pre-release in semver terms.
pub fn is_stable(version: &str) -> bool {
    semver::Version::parse(version.trim_start_matches('v')).is_ok_and(|v| v.pre.is_empty())
}

/// Is an automatic check due? A last check in the future (clock changed) counts as due.
pub fn due(last_check: i64, now: i64) -> bool {
    last_check > now || now - last_check >= CHECK_EVERY_SECS
}

/// An automatic check stays quiet about a version the user chose to skip; a manual one does not.
pub fn should_offer(version: &str, skipped: &str, manual: bool) -> bool {
    manual || version != skipped
}

/// The installer puts `uninstall.exe` next to the application; a copy without one is portable.
pub fn is_installed_at(dir: &Path) -> bool {
    dir.join("uninstall.exe").exists()
}

fn is_installed() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(is_installed_at))
        .unwrap_or(false)
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn memory_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("update_state.json")
}

fn load_memory(app: &AppHandle) -> Memory {
    std::fs::read_to_string(memory_path(app))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_memory(app: &AppHandle, memory: &Memory) {
    let path = memory_path(app);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string(memory) {
        let _ = std::fs::write(path, text);
    }
}

fn set_info(app: &AppHandle, info: UpdateInfo, update: Option<Update>) {
    let shared = app.state::<crate::Shared>();
    let mut pending = shared.pending_update.lock().unwrap();
    pending.info = info;
    pending.update = update;
}

fn base_info(app: &AppHandle, state: &str) -> UpdateInfo {
    UpdateInfo {
        state: state.to_string(),
        current: app.package_info().version.to_string(),
        portable: !is_installed(),
        ..Default::default()
    }
}

/// The plugin asks for `Accept: application/json`, and GitHub answers 404 to that on a release
/// asset's address, although the very same address answers 200 to `*/*` (the user then saw "Could
/// not fetch a valid release JSON from the remote" with a published release). So the check asks for
/// `*/*` instead.
fn updater(app: &AppHandle) -> Result<tauri_plugin_updater::Updater, String> {
    app.updater_builder()
        .header("Accept", "*/*")
        .map_err(|e| e.to_string())?
        .build()
        .map_err(|e| e.to_string())
}

async fn fetch(app: &AppHandle) -> Result<Option<Update>, String> {
    match updater(app)?.check().await {
        Ok(Some(mut update)) if is_stable(&update.version) => {
            // The update inherits the check's headers. The download of an asset through
            // GitHub's API needs `application/octet-stream` (the plugin's own default, used
            // when no Accept is set), and would receive JSON metadata with `*/*`.
            update.headers.remove("accept");
            Ok(Some(update))
        }
        Ok(_) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Looks for an update. A manual check always answers in the dialog; an automatic one speaks
/// only when there is something new, and stays silent about network trouble.
async fn run_check(app: &AppHandle, manual: bool) {
    let mut memory = load_memory(app);
    memory.last_check = now();
    save_memory(app, &memory);

    match fetch(app).await {
        Ok(Some(update)) if should_offer(&update.version, &memory.skipped, manual) => {
            let mut info = base_info(app, "available");
            info.version = update.version.clone();
            info.notes = update
                .body
                .clone()
                .unwrap_or_default()
                .chars()
                .take(NOTES_MAX)
                .collect();
            set_info(app, info, Some(update.clone()));
            if manual {
                crate::show_view(app, "update", "");
            } else {
                let ui = crate::lang(app);
                crate::toast(
                    app,
                    &fill(
                        tr(ui, "toast.update_title"),
                        &[("version", &update.version)],
                    ),
                    tr(ui, "toast.update_body"),
                    |app| crate::show_view(app, "update", ""),
                );
            }
        }
        Ok(_) if manual => {
            set_info(app, base_info(app, "uptodate"), None);
            crate::show_view(app, "update", "");
        }
        Err(e) if manual => {
            let mut info = base_info(app, "error");
            info.message = fill(tr(crate::lang(app), "err.update"), &[("e", &e)]);
            set_info(app, info, None);
            crate::show_view(app, "update", "");
        }
        Err(e) => eprintln!("update check failed: {e}"),
        Ok(_) => {}
    }
}

/// Help → Check for updates: opens the dialog straight away, then fills in the answer.
pub fn check_manually(app: &AppHandle) {
    set_info(app, base_info(app, "checking"), None);
    crate::show_view(app, "update", "");
    let app = app.clone();
    tauri::async_runtime::spawn(async move { run_check(&app, true).await });
}

/// Looks once shortly after launch, then every few hours as long as the application stays in
/// the tray; the once-a-day rule keeps the number of requests down. A development build does
/// not check on its own, so it never nags while the code is being worked on.
pub fn spawn_auto_check(app: &AppHandle) {
    if cfg!(debug_assertions) && std::env::var_os("PLMAIL_UPDATE_CHECK").is_none() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;
        loop {
            if crate::load_settings(&app).check_updates && due(load_memory(&app).last_check, now())
            {
                run_check(&app, false).await;
            }
            tokio::time::sleep(Duration::from_secs(6 * 60 * 60)).await;
        }
    });
}

#[tauri::command]
pub fn get_update_info(app: AppHandle) -> UpdateInfo {
    let shared = app.state::<crate::Shared>();
    let pending = shared.pending_update.lock().unwrap();
    let mut info = pending.info.clone();
    if info.current.is_empty() {
        info.current = app.package_info().version.to_string();
    }
    info
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let update = app
        .state::<crate::Shared>()
        .pending_update
        .lock()
        .unwrap()
        .update
        .clone()
        .ok_or_else(|| "No update to install.".to_string())?;
    if !is_installed() {
        return Err(tr(crate::lang(&app), "err.update_portable").to_string());
    }

    let mut info = base_info(&app, "installing");
    info.version = update.version.clone();
    set_info(&app, info, Some(update.clone()));

    let mut downloaded: u64 = 0;
    let progress = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = progress.emit("update-progress", (downloaded, total));
            },
            || {},
        )
        .await
        .map_err(|e| {
            fill(
                tr(crate::lang(&app), "err.update"),
                &[("e", &e.to_string())],
            )
        })?;
    // On Windows the installer takes over and ends this process; it relaunches the application.
    app.restart();
}

/// "Skip this version": no more automatic notices for it; a newer one will be offered again.
#[tauri::command]
pub fn skip_update(app: AppHandle) {
    let version = get_update_info(app.clone()).version;
    let mut memory = load_memory(&app);
    memory.skipped = version;
    save_memory(&app, &memory);
    crate::hide_main(&app);
}

#[tauri::command]
pub fn open_release_page(app: AppHandle) {
    let _ = app.opener().open_url(RELEASES_URL, None::<&str>);
}

/// "Try again" in the dialog.
#[tauri::command]
pub fn check_updates_now(app: AppHandle) {
    check_manually(&app);
}
