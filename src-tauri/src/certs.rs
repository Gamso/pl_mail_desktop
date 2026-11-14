//! "Accept a self-signed certificate", for the configured server only.
//!
//! Starting WebView2 with `--ignore-certificate-errors` would switch certificate checks off for
//! every host the mail window talks to, including whatever a mail body loads. Instead, WebView2
//! reports each certificate error to us, and only an error on the server's own origin is let
//! through; any other one is refused as usual.

use tauri::{Url, WebviewWindow};
use webview2_com::{
    CoTaskMemPWSTR,
    Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_SERVER_CERTIFICATE_ERROR_ACTION_ALWAYS_ALLOW,
        COREWEBVIEW2_SERVER_CERTIFICATE_ERROR_ACTION_CANCEL, ICoreWebView2_14,
    },
    ServerCertificateErrorDetectedEventHandler,
};
use windows_core::{Interface, PWSTR};

/// Is a certificate error on this address one the user agreed to?
pub fn allowed(server: &Url, request: &str) -> bool {
    Url::parse(request).is_ok_and(|url| url.origin() == server.origin())
}

/// Lets the window accept the server's own certificate even when it is not trusted. Runs before
/// the first page is loaded: `open_mail_window` builds the window on a blank page and only
/// navigates to the server once this is attached.
pub fn accept_for(window: &WebviewWindow, server: Url) {
    let attached = window.with_webview(move |webview| {
        let result = (|| unsafe {
            let core = webview
                .controller()
                .CoreWebView2()?
                .cast::<ICoreWebView2_14>()?;
            let handler =
                ServerCertificateErrorDetectedEventHandler::create(Box::new(move |_, args| {
                    let Some(args) = args else { return Ok(()) };
                    let mut uri = PWSTR::null();
                    args.RequestUri(&mut uri)?;
                    let uri = CoTaskMemPWSTR::from(uri).to_string();
                    let action = if allowed(&server, &uri) {
                        COREWEBVIEW2_SERVER_CERTIFICATE_ERROR_ACTION_ALWAYS_ALLOW
                    } else {
                        COREWEBVIEW2_SERVER_CERTIFICATE_ERROR_ACTION_CANCEL
                    };
                    args.SetAction(action)
                }));
            let mut token = 0i64;
            core.add_ServerCertificateErrorDetected(&handler, &mut token)
        })();
        if let Err(e) = result {
            eprintln!("could not watch certificate errors: {e}");
        }
    });
    if let Err(e) = attached {
        eprintln!("could not reach the web view: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_servers_own_origin_is_let_through() {
        let server = Url::parse("https://mail.home.lan:8443").unwrap();
        assert!(allowed(&server, "https://mail.home.lan:8443/mail/inbox"));
        assert!(!allowed(&server, "https://mail.home.lan/"));
        assert!(!allowed(&server, "https://tracker.example.net/pixel.gif"));
        assert!(!allowed(&server, "not a url"));
    }
}
