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
    pub mail_accounts: Vec<String>,
}

pub struct NewMail {
    pub from: String,
    pub subject: String,
    pub preview: String,
}

/// Where one account stands: the Email state we last saw and its Inbox binding id.
pub struct AccountCursor {
    pub account_id: String,
    pub state: Option<String>,
    pub inbox_id: Option<String>,
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
                    .map(|(id, _)| id.clone())
                    .collect()
            })
            .unwrap_or_default();
        Ok(Session {
            api_url,
            mail_accounts,
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
                    json!(["Mailbox/get", { "accountId": id, "properties": ["role"] }, "m"]),
                ],
            )
            .await?;
        cur.state = resp
            .iter()
            .find(|r| r[0] == "Email/get")
            .and_then(|r| r[1]["state"].as_str())
            .map(String::from);
        cur.inbox_id = resp
            .iter()
            .find(|r| r[0] == "Mailbox/get")
            .and_then(|r| r[1]["list"].as_array())
            .and_then(|l| l.iter().find(|m| m["role"] == "inbox"))
            .and_then(|m| m["id"].as_str())
            .map(String::from);
        Ok(())
    }

    /// Unread conversations in the Inbox, as the server counts them.
    pub async fn unread(&self, api_url: &str, cur: &AccountCursor) -> Result<u64, String> {
        let Some(inbox) = cur.inbox_id.as_deref() else {
            return Ok(0);
        };
        let resp = self
            .call(
                api_url,
                vec![json!(["Mailbox/get", {
                    "accountId": cur.account_id,
                    "ids": [inbox],
                    "properties": ["unreadEmails"]
                }, "u"])],
            )
            .await?;
        Ok(resp
            .first()
            .and_then(|r| r[1]["list"][0]["unreadEmails"].as_u64())
            .unwrap_or(0))
    }

    /// Unread mail that landed in the Inbox since the last call.
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
        loop {
            let resp = self
                .call(
                    api_url,
                    vec![json!(["Email/changes", { "accountId": id, "sinceState": since, "maxChanges": 50 }, "c"])],
                )
                .await?;
            let Some(r) = resp.first() else {
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
            since = r[1]["newState"].as_str().unwrap_or(&since).to_string();
            cur.state = Some(since.clone());
            if r[1]["hasMoreChanges"] != true {
                break;
            }
        }
        if created.is_empty() {
            return Ok(vec![]);
        }

        let resp = self
            .call(
                api_url,
                vec![json!(["Email/get", {
                    "accountId": id,
                    "ids": created,
                    "properties": ["from", "subject", "preview", "keywords", "mailboxIds"]
                }, "g"])],
            )
            .await?;
        let list = resp
            .first()
            .and_then(|r| r[1]["list"].as_array())
            .cloned()
            .unwrap_or_default();
        let inbox = cur.inbox_id.as_deref();
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
                    from: name
                        .or_else(|| from["email"].as_str())
                        .unwrap_or_else(|| tr(self.lang, "toast.unknown_sender"))
                        .to_string(),
                    subject: m["subject"]
                        .as_str()
                        .unwrap_or_else(|| tr(self.lang, "toast.no_subject"))
                        .to_string(),
                    preview: m["preview"]
                        .as_str()
                        .unwrap_or("")
                        .chars()
                        .take(140)
                        .collect(),
                }
            })
            .collect())
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
