//! JMAP push over EventSource (RFC 8620 §7.3): the server says as soon as an account's state
//! moves, and the poller checks right away instead of at its next round.
//!
//! The push only wakes the poller up; the poller still does all the reading, and keeps its own
//! interval as a fallback, so a push that is lost or a server without push changes nothing.
//!
//! pl_mail holds one PHP worker per open stream and closes each one after five minutes on
//! purpose; the client reconnects then, and backs off when the server cannot be reached.

use std::{collections::HashMap, sync::Arc, time::Duration};

use serde_json::Value;
use tokio::sync::Notify;

use crate::jmap::Client;

/// Object types worth a check: a new message, or a message read elsewhere (the unread count).
const WATCHED: [&str; 2] = ["Email", "Mailbox"];
/// How often the server is asked to send a keep-alive; the stream is dropped after a few missed.
const PING_SECS: u64 = 30;
const FIRST_RETRY: Duration = Duration::from_secs(5);
const MAX_RETRY: Duration = Duration::from_secs(5 * 60);

/// Fills the session's `eventSourceUrl` template.
pub fn event_source_url(template: &str) -> String {
    template
        .replace("{types}", &WATCHED.join(","))
        .replace("{closeafter}", "no")
        .replace("{ping}", &PING_SECS.to_string())
}

/// One server-sent event.
#[derive(Debug, PartialEq)]
pub struct Event {
    pub name: String,
    pub data: String,
}

/// Cuts a `text/event-stream` into events, whatever the chunks it arrives in.
#[derive(Default)]
pub struct Parser {
    pending: String,
    name: String,
    data: Vec<String>,
}

impl Parser {
    pub fn feed(&mut self, chunk: &str) -> Vec<Event> {
        self.pending.push_str(chunk);
        let mut events = vec![];
        while let Some(end) = self.pending.find('\n') {
            let line: String = self.pending.drain(..=end).collect();
            let line = line.trim_end_matches(['\n', '\r']);
            if line.is_empty() {
                // A blank line ends the event.
                if !self.data.is_empty() {
                    events.push(Event {
                        name: if self.name.is_empty() {
                            "message".to_string()
                        } else {
                            std::mem::take(&mut self.name)
                        },
                        data: self.data.join("\n"),
                    });
                }
                self.name.clear();
                self.data.clear();
                continue;
            }
            if line.starts_with(':') {
                continue; // a comment
            }
            let (field, value) = line.split_once(':').unwrap_or((line, ""));
            let value = value.strip_prefix(' ').unwrap_or(value);
            match field {
                "event" => self.name = value.to_string(),
                "data" => self.data.push(value.to_string()),
                _ => {}
            }
        }
        events
    }
}

/// The states we last heard of: account → type → state.
pub type Known = HashMap<String, HashMap<String, String>>;

/// Takes in a `StateChange` and says whether the mail of one of the watched accounts moved.
/// The first news of an account only records where it stands: right after (re)connecting, the
/// server repeats the current states, which is not a change.
pub fn wants_check(known: &mut Known, data: &str, accounts: &[String]) -> bool {
    let Ok(change) = serde_json::from_str::<Value>(data) else {
        return false;
    };
    let Some(changed) = change["changed"].as_object() else {
        return false;
    };
    let mut moved = false;
    for (account, types) in changed {
        if !accounts.contains(account) {
            continue;
        }
        let Some(types) = types.as_object() else {
            continue;
        };
        let seen = known.entry(account.clone()).or_default();
        for name in WATCHED {
            let Some(state) = types.get(name).and_then(Value::as_str) else {
                continue;
            };
            if let Some(old) = seen.insert(name.to_string(), state.to_string()) {
                moved |= old != state;
            }
        }
    }
    moved
}

/// Listens to the server's push for as long as the application runs, waking `check_now` when
/// mail moves. Returns at once when the server offers no push.
pub async fn run(client: Client, http: reqwest::Client, token: String, check_now: Arc<Notify>) {
    let mut known = Known::new();
    let mut retry = FIRST_RETRY;
    loop {
        match listen(&client, &http, &token, &check_now, &mut known).await {
            Ok(Some(())) => {
                // The server closed the stream, as pl_mail does every five minutes.
                retry = FIRST_RETRY;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Ok(None) => {
                log::info!("the server offers no push; checking at the set interval only");
                return;
            }
            Err(e) => {
                log::info!(
                    "push unavailable, trying again in {}s: {e}",
                    retry.as_secs()
                );
                tokio::time::sleep(retry).await;
                retry = (retry * 2).min(MAX_RETRY);
            }
        }
    }
}

/// One connection: `Ok(None)` when there is no push to listen to, `Ok(Some(()))` when the
/// server ended the stream.
async fn listen(
    client: &Client,
    http: &reqwest::Client,
    token: &str,
    check_now: &Notify,
    known: &mut Known,
) -> Result<Option<()>, String> {
    let session = client.session().await?;
    let Some(template) = session.event_source_url else {
        return Ok(None);
    };
    let accounts: Vec<String> = session.mail_accounts.into_iter().map(|a| a.id).collect();
    let mut response = http
        .get(event_source_url(&template))
        .bearer_auth(token)
        .header("Accept", "text/event-stream")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status().as_u16()));
    }
    let mut parser = Parser::default();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        for event in parser.feed(&String::from_utf8_lossy(&chunk)) {
            if event.name == "state" && wants_check(known, &event.data, &accounts) {
                check_now.notify_one();
            }
        }
    }
    Ok(Some(()))
}

/// A client for the stream: no overall time limit (the stream lasts minutes), but one on each
/// read, longer than a few missed pings.
pub fn http_client(accept_invalid_certs: bool) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(accept_invalid_certs)
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(PING_SECS * 3))
        .build()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_template_is_filled() {
        assert_eq!(
            event_source_url(
                "https://mail.example.com/jmap/eventsource?types={types}&closeafter={closeafter}&ping={ping}"
            ),
            "https://mail.example.com/jmap/eventsource?types=Email,Mailbox&closeafter=no&ping=30"
        );
    }

    #[test]
    fn events_are_cut_out_of_any_chunks() {
        let mut parser = Parser::default();
        assert!(parser.feed("event: state\r\nda").is_empty());
        let events = parser.feed("ta: {\"a\":1}\r\n\r\n: comment\n\nevent: ping\ndata: {}\n\n");
        assert_eq!(
            events,
            vec![
                Event {
                    name: "state".into(),
                    data: "{\"a\":1}".into()
                },
                Event {
                    name: "ping".into(),
                    data: "{}".into()
                },
            ]
        );
        let events = parser.feed("data: a\ndata: b\n\n");
        assert_eq!(events[0].name, "message");
        assert_eq!(events[0].data, "a\nb");
    }

    fn change(account: &str, email: &str) -> String {
        format!(
            r#"{{"@type":"StateChange","changed":{{"{account}":{{"Email":"{email}","Thread":"x"}}}}}}"#
        )
    }

    #[test]
    fn only_a_moving_mail_state_of_a_watched_account_asks_for_a_check() {
        let accounts = vec!["7".to_string()];
        let mut known = Known::new();
        // Right after connecting: the current state, not a change.
        assert!(!wants_check(&mut known, &change("7", "s1"), &accounts));
        assert!(!wants_check(&mut known, &change("7", "s1"), &accounts));
        assert!(wants_check(&mut known, &change("7", "s2"), &accounts));
        // The calendar account, or anything that is not mail, is not our business.
        assert!(!wants_check(&mut known, &change("9", "s1"), &accounts));
        assert!(!wants_check(&mut known, &change("9", "s5"), &accounts));
        assert!(!wants_check(
            &mut known,
            r#"{"changed":{"7":{"CalendarEvent":"c2"}}}"#,
            &accounts
        ));
        assert!(!wants_check(&mut known, "not json", &accounts));
    }

    /// A server whose session offers push, and whose stream sends the current state, a ping,
    /// a state that moved, then closes, as pl_mail does after its five minutes.
    async fn push_server() -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let origin = base.clone();
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let origin = origin.clone();
                tokio::spawn(async move {
                    let mut head = vec![0u8; 4096];
                    let n = socket.read(&mut head).await.unwrap_or(0);
                    let request = String::from_utf8_lossy(&head[..n]).to_string();
                    let reply = if request.starts_with("GET /.well-known/jmap") {
                        let body = serde_json::json!({
                            "apiUrl": format!("{origin}/jmap/api"),
                            "eventSourceUrl": format!("{origin}/jmap/eventsource?types={{types}}&closeafter={{closeafter}}&ping={{ping}}"),
                            "accounts": { "7": { "name": "me", "accountCapabilities": { "urn:ietf:params:jmap:mail": {} } } }
                        })
                        .to_string();
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                            body.len()
                        )
                    } else {
                        assert!(request.contains("types=Email,Mailbox&closeafter=no&ping=30"));
                        let state = |s: &str| {
                            format!(
                                "event: state\ndata: {{\"changed\":{{\"7\":{{\"Email\":\"{s}\"}}}}}}\n\n"
                            )
                        };
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n{}event: ping\ndata: {{}}\n\n{}",
                            state("s1"),
                            state("s2")
                        )
                    };
                    let _ = socket.write_all(reply.as_bytes()).await;
                });
            }
        });
        base
    }

    #[tokio::test]
    async fn a_moving_state_on_the_stream_wakes_the_poller_once() {
        let base = push_server().await;
        let client = Client::new(&base, "token", false, crate::i18n::Lang::En).unwrap();
        let wake = Notify::new();
        let mut known = Known::new();
        let ended = listen(
            &client,
            &http_client(false).unwrap(),
            "token",
            &wake,
            &mut known,
        )
        .await
        .unwrap();
        assert_eq!(ended, Some(()), "the server closed the stream");
        // s1 was where things stood, s2 is news: exactly one wake-up is waiting.
        tokio::time::timeout(Duration::from_secs(1), wake.notified())
            .await
            .expect("woken up");
        assert_eq!(known["7"]["Email"], "s2");
    }

    #[test]
    fn a_reconnection_does_not_count_as_news() {
        let accounts = vec!["7".to_string()];
        let mut known = Known::new();
        wants_check(&mut known, &change("7", "s1"), &accounts);
        // New stream, same state.
        assert!(!wants_check(&mut known, &change("7", "s1"), &accounts));
        // New stream after mail arrived while disconnected: that is news.
        assert!(wants_check(&mut known, &change("7", "s3"), &accounts));
    }
}
