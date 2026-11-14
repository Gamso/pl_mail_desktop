use serde_json::{Value, json};

use crate::i18n::{Lang, fill, tr};

const USING: [&str; 2] = ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"];
const MAIL_CAP: &str = "urn:ietf:params:jmap:mail";

pub struct Client {
    http: reqwest::Client,
    base: String,
    token: String,
    lang: Lang,
}

pub struct Session {
    pub api_url: String,
    pub mail_accounts: Vec<MailAccount>,
    /// Where the server pushes its changes, as a template (RFC 8620 §7.3), when it does.
    pub event_source_url: Option<String>,
}

/// One connected mail account, as the session lists it.
pub struct MailAccount {
    pub id: String,
    /// What the account is called: its address.
    pub name: String,
}

pub struct NewMail {
    /// The JMAP Email id, which is also the number in pl_mail's `/mail/message/{id}` address.
    pub id: String,
    pub account_id: String,
    pub account_name: String,
    pub from: String,
    pub subject: String,
}

/// Where one account stands: the Email state we last saw, its Inbox id and its unread count.
#[derive(Debug)]
pub struct AccountCursor {
    pub account_id: String,
    pub account_name: String,
    pub state: Option<String>,
    pub inbox_id: Option<String>,
    /// Unread messages in the Inbox at the last successful check.
    pub unread: u64,
}

impl AccountCursor {
    pub fn new(account: MailAccount) -> Self {
        Self {
            account_id: account.id,
            account_name: account.name,
            state: None,
            inbox_id: None,
            unread: 0,
        }
    }
}

/// Lines up the accounts a fresh session lists with the cursors we already had: an account we
/// knew keeps its Email state, so mail that arrived while the server could not be reached is
/// still reported; a new account starts from now; an account that is gone is dropped.
pub fn merge_cursors(
    mut old: Vec<AccountCursor>,
    accounts: Vec<MailAccount>,
) -> Vec<AccountCursor> {
    accounts
        .into_iter()
        .map(
            |account| match old.iter().position(|c| c.account_id == account.id) {
                Some(i) => {
                    let mut cur = old.swap_remove(i);
                    cur.account_name = account.name;
                    cur
                }
                None => AccountCursor::new(account),
            },
        )
        .collect()
}

/// Is this error the server refusing the app password?
pub fn is_unauthorized(lang: Lang, error: &str) -> bool {
    error == tr(lang, "err.unauthorized")
}

/// `Email/get` asks for at most this many messages at once (servers cap `maxObjectsInGet`).
const GET_CHUNK: usize = 50;

/// The answer to the call with this id, out of a JMAP response.
fn answer<'a>(resp: &'a [Value], call_id: &str) -> Option<&'a Value> {
    resp.iter().find(|r| r[2] == call_id)
}

impl Client {
    pub fn new(
        base: &str,
        token: &str,
        accept_invalid_certs: bool,
        lang: Lang,
    ) -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(accept_invalid_certs)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            http,
            base: base.trim_end_matches('/').to_string(),
            token: token.to_string(),
            lang,
        })
    }

    pub async fn session(&self) -> Result<Session, String> {
        let url = format!("{}/.well-known/jmap", self.base);
        let res = self
            .http
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| {
                fill(
                    tr(self.lang, "err.unreachable"),
                    &[("base", &self.base), ("e", &e.to_string())],
                )
            })?;
        match res.status().as_u16() {
            200 => {}
            401 | 403 => return Err(tr(self.lang, "err.unauthorized").to_string()),
            code => {
                return Err(fill(
                    tr(self.lang, "err.http"),
                    &[("code", &code.to_string())],
                ));
            }
        }
        let body: Value = res
            .json()
            .await
            .map_err(|_| tr(self.lang, "err.not_jmap").to_string())?;
        let api_url = body["apiUrl"]
            .as_str()
            .ok_or_else(|| tr(self.lang, "err.no_api_url").to_string())?
            .to_string();
        let mail_accounts = body["accounts"]
            .as_object()
            .map(|accounts| {
                accounts
                    .iter()
                    .filter(|(_, a)| a["accountCapabilities"].get(MAIL_CAP).is_some())
                    .map(|(id, a)| MailAccount {
                        id: id.clone(),
                        name: a["name"].as_str().unwrap_or(id).to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Session {
            api_url,
            mail_accounts,
            event_source_url: body["eventSourceUrl"]
                .as_str()
                .filter(|u| !u.is_empty())
                .map(String::from),
        })
    }

    async fn call(&self, api_url: &str, calls: Vec<Value>) -> Result<Vec<Value>, String> {
        let res = self
            .http
            .post(api_url)
            .bearer_auth(&self.token)
            .json(&json!({ "using": USING, "methodCalls": calls }))
            .send()
            .await
            .map_err(|e| {
                fill(
                    tr(self.lang, "err.unreachable_short"),
                    &[("e", &e.to_string())],
                )
            })?;
        if res.status().as_u16() == 401 {
            return Err(tr(self.lang, "err.unauthorized").to_string());
        }
        if !res.status().is_success() {
            return Err(fill(
                tr(self.lang, "err.jmap"),
                &[("code", &res.status().as_u16().to_string())],
            ));
        }
        let body: Value = res.json().await.map_err(|e| e.to_string())?;
        Ok(body["methodResponses"]
            .as_array()
            .cloned()
            .unwrap_or_default())
    }

    /// Records the current Email state and the Inbox id without reporting anything:
    /// mail that was already there before we started is not news.
    pub async fn init(&self, api_url: &str, cur: &mut AccountCursor) -> Result<(), String> {
        let id = cur.account_id.as_str();
        let resp = self
            .call(
                api_url,
                vec![
                    json!(["Email/get", { "accountId": id, "ids": [] }, "e"]),
                    json!(["Mailbox/get", { "accountId": id, "properties": ["role", "unreadEmails"] }, "m"]),
                ],
            )
            .await?;
        cur.state = answer(&resp, "e")
            .and_then(|r| r[1]["state"].as_str())
            .map(String::from);
        let inbox = answer(&resp, "m")
            .and_then(|r| r[1]["list"].as_array())
            .and_then(|l| l.iter().find(|m| m["role"] == "inbox"));
        cur.inbox_id = inbox.and_then(|m| m["id"].as_str()).map(String::from);
        cur.unread = inbox.and_then(|m| m["unreadEmails"].as_u64()).unwrap_or(0);
        Ok(())
    }

    /// Unread mail that landed in the Inbox since the last call. The Inbox's unread count is
    /// read in the same request and kept in the cursor; when the request fails, the cursor keeps
    /// the count and the state it had.
    pub async fn new_mail(
        &self,
        api_url: &str,
        cur: &mut AccountCursor,
    ) -> Result<Vec<NewMail>, String> {
        let Some(mut since) = cur.state.clone() else {
            self.init(api_url, cur).await?;
            return Ok(vec![]);
        };
        let id = cur.account_id.clone();
        let mut created: Vec<String> = vec![];
        let mut first = true;
        loop {
            let mut calls = vec![
                json!(["Email/changes", { "accountId": id, "sinceState": since, "maxChanges": 50 }, "c"]),
            ];
            if let Some(inbox) = cur.inbox_id.as_deref().filter(|_| first) {
                calls.push(json!(["Mailbox/get", {
                    "accountId": id,
                    "ids": [inbox],
                    "properties": ["unreadEmails"]
                }, "u"]));
            }
            let resp = self.call(api_url, calls).await?;
            if let Some(unread) = answer(&resp, "u")
                .filter(|r| r[0] == "Mailbox/get")
                .and_then(|r| r[1]["list"][0]["unreadEmails"].as_u64())
            {
                cur.unread = unread;
            }
            first = false;
            let Some(r) = answer(&resp, "c") else {
                return Ok(vec![]);
            };
            if r[0] == "error" {
                // The server cannot replay that far back; start over from now.
                self.init(api_url, cur).await?;
                return Ok(vec![]);
            }
            created.extend(
                r[1]["created"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|v| v.as_str().map(String::from)),
            );
            // A state that does not move would ask the same question forever.
            let Some(next) = r[1]["newState"].as_str().filter(|s| *s != since) else {
                break;
            };
            since = next.to_string();
            if r[1]["hasMoreChanges"] != true {
                break;
            }
        }
        if created.is_empty() {
            cur.state = Some(since);
            return Ok(vec![]);
        }

        let mut list: Vec<Value> = vec![];
        for ids in created.chunks(GET_CHUNK) {
            let resp = self
                .call(
                    api_url,
                    vec![json!(["Email/get", {
                        "accountId": id,
                        "ids": ids,
                        "properties": ["from", "subject", "keywords", "mailboxIds"]
                    }, "g"])],
                )
                .await?;
            list.extend(
                answer(&resp, "g")
                    .and_then(|r| r[1]["list"].as_array())
                    .cloned()
                    .unwrap_or_default(),
            );
        }
        // Only now: a request that failed above leaves the state where it was, so the same
        // messages are asked about again at the next check instead of being lost.
        cur.state = Some(since);
        let inbox = cur.inbox_id.as_deref();
        let (account_id, account_name) = (cur.account_id.clone(), cur.account_name.clone());
        Ok(list
            .iter()
            .filter(|m| {
                m["keywords"].get("$seen").is_none() && m["keywords"].get("$draft").is_none()
            })
            .filter(|m| match inbox {
                Some(i) => m["mailboxIds"].get(i).is_some(),
                None => true,
            })
            .map(|m| {
                let from = &m["from"][0];
                let name = from["name"].as_str().filter(|n| !n.is_empty());
                NewMail {
                    id: m["id"].as_str().unwrap_or_default().to_string(),
                    account_id: account_id.clone(),
                    account_name: account_name.clone(),
                    from: name
                        .or_else(|| from["email"].as_str())
                        .unwrap_or_else(|| tr(self.lang, "toast.unknown_sender"))
                        .to_string(),
                    subject: m["subject"]
                        .as_str()
                        .unwrap_or_else(|| tr(self.lang, "toast.no_subject"))
                        .to_string(),
                }
            })
            .collect())
    }
}

impl Client {
    /// Marks one message as read. A message that is already gone is not an error.
    pub async fn mark_read(
        &self,
        api_url: &str,
        account_id: &str,
        email_id: &str,
    ) -> Result<(), String> {
        let resp = self
            .call(
                api_url,
                vec![json!(["Email/set", {
                    "accountId": account_id,
                    "update": { email_id: { "keywords/$seen": true } }
                }, "r"])],
            )
            .await?;
        set_outcome(&resp, email_id, "updated", "notUpdated")
    }

    /// Moves one message to the Trash: in pl_mail, destroying an Email is a move to the Trash and
    /// never a deletion. A message that is already gone is not an error.
    pub async fn trash(
        &self,
        api_url: &str,
        account_id: &str,
        email_id: &str,
    ) -> Result<(), String> {
        let resp = self
            .call(
                api_url,
                vec![json!(["Email/set", { "accountId": account_id, "destroy": [email_id] }, "d"])],
            )
            .await?;
        set_outcome(&resp, email_id, "destroyed", "notDestroyed")
    }
}

/// Reads the answer to an `Email/set` about one message: done, already gone (fine), or a real error.
fn set_outcome(resp: &[Value], id: &str, done: &str, failed: &str) -> Result<(), String> {
    let Some(r) = resp.first() else {
        return Err("Empty answer from the server.".to_string());
    };
    if r[0] == "error" {
        return Err(r[1]["type"].as_str().unwrap_or("error").to_string());
    }
    let done = match &r[1][done] {
        Value::Object(map) => map.contains_key(id),
        Value::Array(list) => list.iter().any(|v| v.as_str() == Some(id)),
        _ => false,
    };
    if done {
        return Ok(());
    }
    let failure = &r[1][failed][id];
    match failure["type"].as_str() {
        Some("notFound") => Ok(()),
        Some(kind) => Err(match failure["description"].as_str() {
            Some(why) => format!("{kind}: {why}"),
            None => kind.to_string(),
        }),
        None => Err("The server did not confirm the change.".to_string()),
    }
}

/// Is anything answering at this address? Any HTTP reply counts, except the
/// gateway errors a reverse proxy gives when the app behind it is down.
pub async fn probe(base: &str, accept_invalid_certs: bool, lang: Lang) -> Result<(), String> {
    let http = reqwest::Client::builder()
        .danger_accept_invalid_certs(accept_invalid_certs)
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;
    match http.get(base).send().await {
        Ok(r) if (502..=504).contains(&r.status().as_u16()) => Err(fill(
            tr(lang, "err.app_down"),
            &[("code", &r.status().as_u16().to_string())],
        )),
        Ok(_) => Ok(()),
        Err(e) if e.is_timeout() => Err(tr(lang, "err.timeout").to_string()),
        Err(e) if e.is_connect() => Err(tr(lang, "err.refused").to_string()),
        Err(e) => Err(e.to_string()),
    }
}
