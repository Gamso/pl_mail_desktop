//! Thunderbird-style menu bar: a bare press of Alt shows or hides it.
//!
//! The mail window shows a remote page whose Content-Security-Policy forbids any
//! request to us, so the key cannot be read from the page. WebView2 reports every key
//! pressed in the window to its host before the page sees it, which is where Alt is
//! caught: no global keyboard hook, and nothing outside the mail window is observed.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tauri::{Manager, WebviewWindow};
use webview2_com::{
    AcceleratorKeyPressedEventHandler,
    Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_KEY_EVENT_KIND, COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN,
        COREWEBVIEW2_KEY_EVENT_KIND_KEY_UP, COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN,
        COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_UP,
    },
};

const VK_MENU: u32 = 0x12;

pub fn attach(window: &WebviewWindow) {
    let app = window.app_handle().clone();
    let label = window.label().to_string();
    let _ = window.with_webview(move |webview| {
        let held = Arc::new(AtomicBool::new(false));
        let alone = Arc::new(AtomicBool::new(false));
        let handler = AcceleratorKeyPressedEventHandler::create(Box::new(move |_, args| {
            let Some(args) = args else { return Ok(()) };
            let mut kind = COREWEBVIEW2_KEY_EVENT_KIND::default();
            let mut vk = 0u32;
            unsafe {
                args.KeyEventKind(&mut kind)?;
                args.VirtualKey(&mut vk)?;
            }
            let down = kind == COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN
                || kind == COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN;
            let up = kind == COREWEBVIEW2_KEY_EVENT_KIND_KEY_UP
                || kind == COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_UP;
            // Handling Alt ourselves keeps Windows from entering its own menu mode, which
            // would swallow the next press and leave the bar stuck open.
            if vk == VK_MENU {
                unsafe { args.SetHandled(true)? };
            }
            // "Alone" means Alt went down and came back up with no other key in between.
            if down && vk != VK_MENU {
                alone.store(false, Ordering::Relaxed);
            } else if down && !held.swap(true, Ordering::Relaxed) {
                alone.store(true, Ordering::Relaxed);
            } else if up && vk == VK_MENU {
                held.store(false, Ordering::Relaxed);
                if alone.swap(false, Ordering::Relaxed) {
                    toggle_menu(&app, &label);
                }
            }
            Ok(())
        }));
        let mut token = 0i64;
        unsafe {
            let _ = webview
                .controller()
                .add_AcceleratorKeyPressed(&handler, &mut token);
        }
    });
}

fn toggle_menu(app: &tauri::AppHandle, label: &str) {
    let (app, label) = (app.clone(), label.to_string());
    // This runs inside WebView2's key handling: hand the work to another thread.
    tauri::async_runtime::spawn(async move {
        if let Some(w) = app.get_webview_window(&label) {
            if w.is_menu_visible().unwrap_or(false) {
                let _ = w.hide_menu();
            } else {
                let _ = w.show_menu();
            }
        }
    });
}
