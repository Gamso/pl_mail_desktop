//! The mail window's zoom, remembered between launches and across the rebuilds of the window
//! (saving the settings builds it again, at 100 % unless told otherwise).

use std::{
    path::PathBuf,
    sync::atomic::{AtomicI32, Ordering},
};

use tauri::{AppHandle, Manager, WebviewWindow};

pub const DEFAULT: i32 = 100;
pub const STEP: i32 = 10;
const MIN: i32 = 50;
const MAX: i32 = 300;

/// The zoom the mail window is at, in percent.
static CURRENT: AtomicI32 = AtomicI32::new(DEFAULT);

/// A zoom the window accepts.
pub fn clamp(percent: i32) -> i32 {
    percent.clamp(MIN, MAX)
}

/// Reads a saved zoom; anything unreadable or out of range gives a sane value.
pub fn parse(text: &str) -> i32 {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|v| v["zoom"].as_i64())
        .map_or(DEFAULT, |z| {
            clamp(z.clamp(i32::MIN.into(), i32::MAX.into()) as i32)
        })
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    Some(app.path().app_config_dir().ok()?.join("zoom.json"))
}

pub fn current() -> i32 {
    CURRENT.load(Ordering::Relaxed)
}

/// Sets the window's zoom and remembers it.
pub fn set(app: &AppHandle, window: &WebviewWindow, percent: i32) {
    let percent = clamp(percent);
    CURRENT.store(percent, Ordering::Relaxed);
    let _ = window.set_zoom(f64::from(percent) / 100.0);
    if let Some(path) = path(app) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(&path, format!("{{\"zoom\":{percent}}}")) {
            log::warn!("could not save the zoom: {e}");
        }
    }
}

/// Gives a freshly built mail window the zoom it had last time.
pub fn restore(app: &AppHandle, window: &WebviewWindow) {
    let percent = path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map_or(DEFAULT, |text| parse(&text));
    CURRENT.store(percent, Ordering::Relaxed);
    if percent != DEFAULT {
        let _ = window.set_zoom(f64::from(percent) / 100.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_zoom_is_read_back() {
        assert_eq!(parse(r#"{"zoom":130}"#), 130);
    }

    #[test]
    fn a_broken_or_wild_file_gives_a_usable_zoom() {
        assert_eq!(parse(""), DEFAULT);
        assert_eq!(parse("not json"), DEFAULT);
        assert_eq!(parse(r#"{"zoom":"big"}"#), DEFAULT);
        assert_eq!(parse(r#"{"zoom":5}"#), 50);
        assert_eq!(parse(r#"{"zoom":99999999999}"#), 300);
    }
}
