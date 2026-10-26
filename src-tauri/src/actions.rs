//! What the buttons of a new-mail notification do.

use tauri::{AppHandle, Manager};

use crate::i18n::{fill, tr};
use crate::jmap::Client;

pub enum MailAction {
    MarkRead,
    /// Moves the message to the Trash: in pl_mail "delete" never removes anything for good.
    Trash,
}

/// Runs the action against the server, then asks the poller to look again so that the unread
/// count and the badge catch up. A failure is reported in a notification of its own.
pub fn run(app: &AppHandle, account_id: &str, email_id: &str, action: MailAction) {
    let (app, account_id, email_id) = (app.clone(), account_id.to_string(), email_id.to_string());
    tauri::async_runtime::spawn(async move {
        if let Err(e) = perform(&app, &account_id, &email_id, &action).await {
            let text = fill(tr(crate::lang(&app), "toast.action_failed"), &[("e", &e)]);
            crate::toast(&app, "plMail", &[text.as_str()], &[], |_, _| {});
        }
        app.state::<crate::Shared>().check_now.notify_one();
    });
}

async fn perform(
    app: &AppHandle,
    account_id: &str,
    email_id: &str,
    action: &MailAction,
) -> Result<(), String> {
    let settings = crate::load_settings(app);
    let ui = crate::lang(app);
    let token = crate::password().ok_or_else(|| tr(ui, "err.need_password").to_string())?;
    let client = Client::new(
        &settings.server_url,
        &token,
        settings.accept_invalid_certs,
        ui,
    )?;
    let session = client.session().await?;
    match action {
        MailAction::MarkRead => {
            client
                .mark_read(&session.api_url, account_id, email_id)
                .await
        }
        MailAction::Trash => client.trash(&session.api_url, account_id, email_id).await,
    }
}
