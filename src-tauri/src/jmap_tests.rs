use serde_json::json;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

use crate::i18n::Lang;
use crate::jmap::{AccountCursor, Client, MailAccount, merge_cursors, probe};

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
                "7": { "name": "me@example.com", "accountCapabilities": { "urn:ietf:params:jmap:mail": {} } },
                "9": { "accountCapabilities": { "urn:ietf:params:jmap:calendars": {} } }
            }
        });
        return (200, session.to_string());
    }
    assert!(path.ends_with("/jmap/api"));
    let call = |name: &str| body.contains(&format!("\"{name}\""));
    let response = if call("Email/changes") {
        let mut answers = vec![
            json!(["Email/changes", { "newState": "s2", "created": ["a", "b", "c"], "hasMoreChanges": false }, "c"]),
        ];
        // The unread count travels in the same request.
        if call("Mailbox/get") {
            answers.push(
                json!(["Mailbox/get", { "list": [{ "id": "inbox1", "unreadEmails": 4 }] }, "u"]),
            );
        }
        json!(answers)
    } else if call("Email/get") && body.contains("\"ids\":[]") {
        json!([
            ["Email/get", { "state": "s1", "list": [] }, "e"],
            ["Mailbox/get", { "list": [{ "id": "inbox1", "role": "inbox", "unreadEmails": 3 }, { "id": "box2", "role": null }] }, "m"]
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

fn cursor() -> AccountCursor {
    AccountCursor::new(MailAccount {
        id: "7".into(),
        name: "me@example.com".into(),
    })
}

/// A cursor that already went through its first check.
fn known_cursor() -> AccountCursor {
    AccountCursor {
        state: Some("s1".into()),
        inbox_id: Some("inbox1".into()),
        unread: 2,
        ..cursor()
    }
}

#[tokio::test]
async fn session_lists_only_mail_accounts() {
    let base = serve(healthy).await;
    let session = client(&base).session().await.unwrap();
    assert_eq!(session.mail_accounts.len(), 1);
    assert_eq!(session.mail_accounts[0].id, "7");
    // The notification names the account it is about.
    assert_eq!(session.mail_accounts[0].name, "me@example.com");
    assert!(session.api_url.ends_with("/jmap/api"));
}

#[tokio::test]
async fn only_unread_inbox_mail_is_reported() {
    let base = serve(healthy).await;
    let client = client(&base);
    let session = client.session().await.unwrap();
    let mut cursor = cursor();

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
    assert_eq!(cursor.unread, 3);

    let mails = client
        .new_mail(&session.api_url, &mut cursor)
        .await
        .unwrap();
    assert_eq!(mails.len(), 1);
    assert_eq!(mails[0].id, "a");
    assert_eq!(mails[0].account_id, "7");
    assert_eq!(mails[0].account_name, "me@example.com");
    assert_eq!(mails[0].from, "Alice");
    assert_eq!(mails[0].subject, "Hello");
    assert_eq!(cursor.state.as_deref(), Some("s2"));
    assert_eq!(cursor.unread, 4);
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

/// A server that only accepts exactly the requests the notification buttons should send.
fn actions(method: &str, _path: &str, body: &str, origin: &str) -> (u16, String) {
    if method == "GET" {
        return healthy(method, "", body, origin);
    }
    let sent = |text: &str| body.contains(text);
    let result = if sent("\"destroy\":[\"42\"]") && sent("\"accountId\":\"7\"") {
        json!({ "destroyed": ["42"] })
    } else if sent("\"destroy\":[\"gone\"]") {
        json!({ "notDestroyed": { "gone": { "type": "notFound" } } })
    } else if sent("\"keywords/$seen\":true") && sent("\"accountId\":\"7\"") && sent("\"42\"") {
        json!({ "updated": { "42": null } })
    } else if sent("\"gone\"") {
        json!({ "notUpdated": { "gone": { "type": "notFound" } } })
    } else {
        json!({
            "notUpdated": { "42": { "type": "invalidProperties", "description": "unexpected request" } },
            "notDestroyed": { "42": { "type": "forbidden" } }
        })
    };
    (
        200,
        json!({ "methodResponses": [["Email/set", result, "r"]] }).to_string(),
    )
}

#[tokio::test]
async fn mark_read_sets_the_seen_keyword_on_that_message() {
    let base = serve(actions).await;
    let client = client(&base);
    let api = client.session().await.unwrap().api_url;
    client.mark_read(&api, "7", "42").await.unwrap();
    // The same request for another account is not what the server accepts.
    assert!(client.mark_read(&api, "8", "42").await.is_err());
}

#[tokio::test]
async fn trash_destroys_that_message_which_pl_mail_turns_into_a_move_to_the_trash() {
    let base = serve(actions).await;
    let client = client(&base);
    let api = client.session().await.unwrap().api_url;
    client.trash(&api, "7", "42").await.unwrap();
    assert!(client.trash(&api, "8", "42").await.is_err());
}

#[tokio::test]
async fn a_message_that_is_already_gone_is_not_an_error() {
    let base = serve(actions).await;
    let client = client(&base);
    let api = client.session().await.unwrap().api_url;
    client.mark_read(&api, "7", "gone").await.unwrap();
    client.trash(&api, "7", "gone").await.unwrap();
}

#[tokio::test]
async fn a_refusal_from_the_server_is_reported() {
    let base = serve(actions).await;
    let client = client(&base);
    let api = client.session().await.unwrap().api_url;
    let error = client.trash(&api, "9", "42").await.unwrap_err();
    assert!(error.contains("forbidden"), "{error}");
    let error = client.mark_read(&api, "9", "42").await.unwrap_err();
    assert!(error.contains("unexpected request"), "{error}");
}

#[test]
fn a_new_session_keeps_the_state_of_the_accounts_it_knew() {
    let old = vec![
        AccountCursor {
            account_name: "old name".into(),
            ..known_cursor()
        },
        AccountCursor::new(MailAccount {
            id: "8".into(),
            name: "gone@example.com".into(),
        }),
    ];
    let accounts = vec![
        MailAccount {
            id: "9".into(),
            name: "new@example.com".into(),
        },
        MailAccount {
            id: "7".into(),
            name: "me@example.com".into(),
        },
    ];
    let merged = merge_cursors(old, accounts);
    assert_eq!(merged.len(), 2, "the account that is gone is dropped");
    assert_eq!(merged[0].account_id, "9");
    assert_eq!(merged[0].state, None, "a new account starts from now");
    assert_eq!(merged[1].account_id, "7");
    assert_eq!(merged[1].state.as_deref(), Some("s1"));
    assert_eq!(merged[1].unread, 2);
    assert_eq!(merged[1].account_name, "me@example.com");
}

#[tokio::test]
async fn a_state_that_does_not_move_ends_the_changes_loop() {
    // Claims more changes forever, without ever giving a new state.
    let base = serve(|method, path, body, origin| {
        if method == "GET" || !body.contains("Email/changes") {
            return healthy(method, path, body, origin);
        }
        let answer = json!([["Email/changes", { "created": [], "hasMoreChanges": true }, "c"]]);
        (200, json!({ "methodResponses": answer }).to_string())
    })
    .await;
    let client = client(&base);
    let api = client.session().await.unwrap().api_url;
    let mut cursor = known_cursor();
    let mails = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        client.new_mail(&api, &mut cursor),
    )
    .await
    .expect("the loop ends")
    .unwrap();
    assert!(mails.is_empty());
    assert_eq!(cursor.state.as_deref(), Some("s1"));
}

#[tokio::test]
async fn a_failed_fetch_leaves_the_state_for_the_next_check() {
    let base = serve(|method, path, body, origin| {
        if method == "POST" && body.contains("\"Email/get\"") && !body.contains("\"ids\":[]") {
            return (500, "{}".to_string());
        }
        healthy(method, path, body, origin)
    })
    .await;
    let client = client(&base);
    let api = client.session().await.unwrap().api_url;
    let mut cursor = known_cursor();
    assert!(client.new_mail(&api, &mut cursor).await.is_err());
    assert_eq!(
        cursor.state.as_deref(),
        Some("s1"),
        "the messages are asked about again next time"
    );
}

#[tokio::test]
async fn created_messages_are_fetched_in_chunks() {
    let base = serve(|method, path, body, origin| {
        if method == "GET" {
            return healthy(method, path, body, origin);
        }
        let answer = if body.contains("Email/changes") {
            let created: Vec<String> = (0..120).map(|i| i.to_string()).collect();
            json!([["Email/changes", { "newState": "s2", "created": created, "hasMoreChanges": false }, "c"]])
        } else {
            let request: serde_json::Value = serde_json::from_str(body).unwrap();
            let ids = request["methodCalls"][0][1]["ids"].as_array().unwrap().len();
            assert!(ids <= 50, "{ids} ids in one Email/get");
            json!([["Email/get", { "list": [] }, "g"]])
        };
        (200, json!({ "methodResponses": answer }).to_string())
    })
    .await;
    let client = client(&base);
    let api = client.session().await.unwrap().api_url;
    let mut cursor = known_cursor();
    client.new_mail(&api, &mut cursor).await.unwrap();
    assert_eq!(cursor.state.as_deref(), Some("s2"));
}

#[test]
fn an_unauthorized_error_is_recognised() {
    let error = crate::i18n::tr(Lang::Fr, "err.unauthorized");
    assert!(crate::jmap::is_unauthorized(Lang::Fr, error));
    assert!(!crate::jmap::is_unauthorized(Lang::Fr, "HTTP 500"));
}
