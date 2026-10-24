use serde_json::json;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

use crate::i18n::Lang;
use crate::jmap::{AccountCursor, Client, probe};

type Answer = fn(&str, &str, &str, &str) -> (u16, String);

/// A throwaway HTTP server: `answer(method, path, body, origin)` gives (status, JSON body).
async fn serve(answer: Answer) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let origin = base.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let origin = origin.clone();
            tokio::spawn(async move {
                let mut raw = Vec::new();
                let mut chunk = [0u8; 4096];
                let (head_end, length) = loop {
                    let n = socket.read(&mut chunk).await.unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    raw.extend_from_slice(&chunk[..n]);
                    let text = String::from_utf8_lossy(&raw).to_string();
                    if let Some(pos) = text.find("\r\n\r\n") {
                        let length = text
                            .lines()
                            .find_map(|l| {
                                l.to_lowercase()
                                    .strip_prefix("content-length:")
                                    .map(|v| v.trim().parse().unwrap_or(0))
                            })
                            .unwrap_or(0usize);
                        break (pos + 4, length);
                    }
                };
                while raw.len() < head_end + length {
                    let n = socket.read(&mut chunk).await.unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    raw.extend_from_slice(&chunk[..n]);
                }
                let text = String::from_utf8_lossy(&raw).to_string();
                let mut first = text.lines().next().unwrap_or("").split_whitespace();
                let (method, path) = (first.next().unwrap_or(""), first.next().unwrap_or(""));
                let body = &text[head_end.min(text.len())..];
                let (status, payload) = answer(method, path, body, &origin);
                let reply = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                    payload.len()
                );
                let _ = socket.write_all(reply.as_bytes()).await;
            });
        }
    });
    base
}

fn healthy(method: &str, path: &str, body: &str, origin: &str) -> (u16, String) {
    if method == "GET" {
        let session = json!({
            "apiUrl": format!("{origin}/jmap/api"),
            "accounts": {
                "7": { "accountCapabilities": { "urn:ietf:params:jmap:mail": {} } },
                "9": { "accountCapabilities": { "urn:ietf:params:jmap:calendars": {} } }
            }
        });
        return (200, session.to_string());
    }
    assert!(path.ends_with("/jmap/api"));
    let call = |name: &str| body.contains(&format!("\"{name}\""));
    let response = if call("Email/changes") {
        json!([["Email/changes", { "newState": "s2", "created": ["a", "b", "c"], "hasMoreChanges": false }, "c"]])
    } else if call("Email/get") && body.contains("\"ids\":[]") {
        json!([
            ["Email/get", { "state": "s1", "list": [] }, "e"],
            ["Mailbox/get", { "list": [{ "id": "inbox1", "role": "inbox" }, { "id": "box2", "role": null }] }, "m"]
        ])
    } else if call("Email/get") {
        json!([["Email/get", { "list": [
            { "id": "a", "from": [{ "name": "Alice", "email": "alice@example.com" }], "subject": "Hello",
              "preview": "First line", "keywords": {}, "mailboxIds": { "inbox1": true } },
            { "id": "b", "from": [{ "email": "bob@example.com" }], "subject": "Already read",
              "preview": "", "keywords": { "$seen": true }, "mailboxIds": { "inbox1": true } },
            { "id": "c", "from": [{ "email": "carol@example.com" }], "subject": "Filed away",
              "preview": "", "keywords": {}, "mailboxIds": { "box2": true } }
        ] }, "g"]])
    } else if call("Mailbox/get") {
        json!([["Mailbox/get", { "list": [{ "id": "inbox1", "unreadEmails": 4 }] }, "u"]])
    } else {
        json!([])
    };
    (200, json!({ "methodResponses": response }).to_string())
}

fn client(base: &str) -> Client {
    Client::new(base, "plmail_token", false, Lang::En).unwrap()
}

#[tokio::test]
async fn session_lists_only_mail_accounts() {
    let base = serve(healthy).await;
    let session = client(&base).session().await.unwrap();
    assert_eq!(session.mail_accounts, vec!["7".to_string()]);
    assert!(session.api_url.ends_with("/jmap/api"));
}

#[tokio::test]
async fn only_unread_inbox_mail_is_reported() {
    let base = serve(healthy).await;
    let client = client(&base);
    let session = client.session().await.unwrap();
    let mut cursor = AccountCursor {
        account_id: "7".into(),
        state: None,
        inbox_id: None,
    };

    // The first pass only records where we stand.
    assert!(
        client
            .new_mail(&session.api_url, &mut cursor)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(cursor.state.as_deref(), Some("s1"));
    assert_eq!(cursor.inbox_id.as_deref(), Some("inbox1"));

    let mails = client
        .new_mail(&session.api_url, &mut cursor)
        .await
        .unwrap();
    assert_eq!(mails.len(), 1);
    assert_eq!(mails[0].from, "Alice");
    assert_eq!(mails[0].subject, "Hello");
    assert_eq!(cursor.state.as_deref(), Some("s2"));
    assert_eq!(client.unread(&session.api_url, &cursor).await.unwrap(), 4);
}

#[tokio::test]
async fn a_revoked_password_is_reported_clearly() {
    let base = serve(|_, _, _, _| (401, "{}".to_string())).await;
    let error = client(&base).session().await.err().unwrap();
    assert!(error.contains("revoked"), "{error}");
}

#[tokio::test]
async fn probe_tells_a_proxy_error_from_a_live_server() {
    let live = serve(|_, _, _, _| (200, "{}".to_string())).await;
    assert!(probe(&live, false, Lang::En).await.is_ok());
    let down = serve(|_, _, _, _| (502, "{}".to_string())).await;
    assert!(probe(&down, false, Lang::En).await.is_err());
    // Nothing listens on port 1, so the connection is refused.
    assert!(probe("http://127.0.0.1:1", false, Lang::En).await.is_err());
}
