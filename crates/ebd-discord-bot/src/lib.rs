//! Adapter over Discord's official, documented bot REST API (v10).
//!
//! This is a *bot* account, not a client mod: it never touches the Discord
//! app. Reading message content requires the MESSAGE_CONTENT privileged intent
//! enabled for the bot in the developer portal. `open_external_url` is not
//! supported (a bot has no local OS to hand URLs to).

use std::fmt;
use std::time::Duration;

use ebd_contract::{
    AdapterError, Capability, Channel, ClientAdapter, ClientInfo, Guild, Message, User,
};
use serde_json::{json, Value};

pub const DEFAULT_BASE_URL: &str = "https://discord.com/api/v10";
/// Discord's hard limits.
const MAX_CONTENT_CHARS: usize = 2000;
const MAX_MESSAGES: usize = 100;
/// Guild text channel type in Discord's channel object.
const GUILD_TEXT: i64 = 0;

pub struct DiscordBot {
    token: String,
    base_url: String,
    agent: ureq::Agent,
}

impl fmt::Debug for DiscordBot {
    // Never print the token.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DiscordBot")
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
}

impl DiscordBot {
    pub fn new(token: impl Into<String>) -> Self {
        Self::with_base_url(token, DEFAULT_BASE_URL)
    }

    /// Points the adapter at another API root (used by tests).
    pub fn with_base_url(token: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            agent: ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(10))
                .user_agent("EvenBetterDiscord-bot (adapter 0.1.0)")
                .build(),
        }
    }

    fn request(&self, method: &str, path: &str) -> ureq::Request {
        self.agent
            .request(method, &format!("{}{path}", self.base_url))
            .set("Authorization", &format!("Bot {}", self.token))
    }

    fn send(&self, request: ureq::Request, body: Option<Value>) -> Result<Value, AdapterError> {
        let response = match body {
            Some(body) => request.send_json(body),
            None => request.call(),
        };
        match response {
            Ok(response) => response
                .into_json()
                .map_err(|error| failed(format!("invalid JSON from Discord: {error}"))),
            Err(ureq::Error::Status(code, _)) => {
                Err(failed(format!("Discord returned HTTP {code}")))
            }
            Err(error) => Err(failed(format!("request failed: {error}"))),
        }
    }
}

fn failed(message: impl Into<String>) -> AdapterError {
    AdapterError::Operation(message.into())
}

/// Snowflakes are decimal digits; refusing anything else blocks path injection.
fn snowflake(id: &str) -> Result<&str, AdapterError> {
    if !id.is_empty() && id.len() <= 20 && id.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(id)
    } else {
        Err(failed(format!("`{id}` is not a Discord id")))
    }
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, AdapterError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| failed(format!("Discord response is missing `{key}`")))
}

fn list(value: Value) -> Result<Vec<Value>, AdapterError> {
    match value {
        Value::Array(items) => Ok(items),
        _ => Err(failed("expected a JSON array from Discord")),
    }
}

fn message(value: &Value) -> Result<Message, AdapterError> {
    Ok(Message {
        id: text(value, "id")?.into(),
        channel_id: text(value, "channel_id")?.into(),
        author_id: text(value.get("author").unwrap_or(&Value::Null), "id")?.into(),
        content: text(value, "content")?.into(),
    })
}

impl ClientAdapter for DiscordBot {
    fn info(&self) -> ClientInfo {
        ClientInfo {
            client_name: "Discord (bot API)".into(),
            client_version: "v10".into(),
            adapter_version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::ReadCurrentUser,
            Capability::ReadGuilds,
            Capability::ReadChannels,
            Capability::ReadMessages,
            Capability::SendMessages,
        ]
    }

    fn current_user(&self) -> Result<User, AdapterError> {
        let user = self.send(self.request("GET", "/users/@me"), None)?;
        Ok(User {
            id: text(&user, "id")?.into(),
            username: text(&user, "username")?.into(),
        })
    }

    fn guilds(&self) -> Result<Vec<Guild>, AdapterError> {
        let guilds = list(self.send(self.request("GET", "/users/@me/guilds"), None)?)?;
        guilds
            .iter()
            .map(|guild| {
                Ok(Guild {
                    id: text(guild, "id")?.into(),
                    name: text(guild, "name")?.into(),
                })
            })
            .collect()
    }

    fn channels(&self, guild_id: &str) -> Result<Vec<Channel>, AdapterError> {
        let path = format!("/guilds/{}/channels", snowflake(guild_id)?);
        list(self.send(self.request("GET", &path), None)?)?
            .iter()
            .filter(|channel| channel.get("type").and_then(Value::as_i64) == Some(GUILD_TEXT))
            .map(|channel| {
                Ok(Channel {
                    id: text(channel, "id")?.into(),
                    guild_id: Some(guild_id.into()),
                    name: text(channel, "name")?.into(),
                })
            })
            .collect()
    }

    fn messages(&self, channel_id: &str, limit: usize) -> Result<Vec<Message>, AdapterError> {
        let path = format!(
            "/channels/{}/messages?limit={}",
            snowflake(channel_id)?,
            limit.clamp(1, MAX_MESSAGES)
        );
        list(self.send(self.request("GET", &path), None)?)?
            .iter()
            .map(message)
            .collect()
    }

    fn send_message(&self, channel_id: &str, content: &str) -> Result<Message, AdapterError> {
        let path = format!("/channels/{}/messages", snowflake(channel_id)?);
        if content.trim().is_empty() || content.chars().count() > MAX_CONTENT_CHARS {
            return Err(failed(format!(
                "message must be 1-{MAX_CONTENT_CHARS} characters"
            )));
        }
        let sent = self.send(
            self.request("POST", &path),
            Some(json!({ "content": content })),
        )?;
        message(&sent)
    }

    fn open_external_url(&self, _url: &str) -> Result<(), AdapterError> {
        Err(AdapterError::Unsupported(Capability::OpenExternalUrl))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    /// One-shot local HTTP server: replies with `(status, body)` and reports
    /// the raw request head plus body.
    fn serve(status: u16, body: &str) -> (String, mpsc::Receiver<(String, String)>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!("http://{}", listener.local_addr().expect("addr"));
        let (tx, rx) = mpsc::channel();
        let body = body.to_owned();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut head = String::new();
            let mut length = 0usize;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).expect("line");
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().expect("length");
                }
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                head.push_str(&line);
            }
            let mut payload = vec![0u8; length];
            reader.read_exact(&mut payload).expect("body");
            write!(
                stream,
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .expect("reply");
            tx.send((head, String::from_utf8(payload).expect("utf8")))
                .expect("report");
        });
        (url, rx)
    }

    #[test]
    fn conforms_to_the_contract_without_touching_the_network() {
        // Unreachable port: check_gating only calls undeclared methods.
        let bot = DiscordBot::with_base_url("t", "http://127.0.0.1:1");
        assert!(ebd_contract::conformance::check_gating(&bot).is_empty());
    }

    #[test]
    fn debug_never_reveals_the_token() {
        assert!(!format!("{:?}", DiscordBot::new("super-secret")).contains("super-secret"));
    }

    #[test]
    fn current_user_authenticates_as_a_bot() {
        let (url, rx) = serve(200, r#"{"id":"1","username":"botty"}"#);
        let user = DiscordBot::with_base_url("tok", url)
            .current_user()
            .expect("user");
        assert_eq!((user.id.as_str(), user.username.as_str()), ("1", "botty"));
        let (head, _) = rx.recv().expect("request");
        assert!(head.starts_with("GET /users/@me "), "{head}");
        assert!(head.contains("authorization: Bot tok") || head.contains("Authorization: Bot tok"));
    }

    #[test]
    fn channels_keep_only_text_channels() {
        let (url, _rx) = serve(
            200,
            r#"[{"id":"10","name":"general","type":0},{"id":"11","name":"voice","type":2}]"#,
        );
        let channels = DiscordBot::with_base_url("t", url)
            .channels("5")
            .expect("channels");
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].name, "general");
        assert_eq!(channels[0].guild_id.as_deref(), Some("5"));
    }

    #[test]
    fn messages_parse_and_clamp_the_limit() {
        let (url, rx) = serve(
            200,
            r#"[{"id":"2","channel_id":"9","author":{"id":"3"},"content":"yo"}]"#,
        );
        let messages = DiscordBot::with_base_url("t", url)
            .messages("9", 5000)
            .expect("messages");
        assert_eq!(messages[0].content, "yo");
        assert_eq!(messages[0].author_id, "3");
        assert!(rx.recv().expect("request").0.contains("limit=100"));
    }

    #[test]
    fn send_message_posts_json_content() {
        let (url, rx) = serve(
            200,
            r#"{"id":"7","channel_id":"9","author":{"id":"1"},"content":"hello"}"#,
        );
        let sent = DiscordBot::with_base_url("t", url)
            .send_message("9", "hello")
            .expect("send");
        assert_eq!(sent.id, "7");
        let (head, body) = rx.recv().expect("request");
        assert!(head.starts_with("POST /channels/9/messages "), "{head}");
        assert_eq!(body, r#"{"content":"hello"}"#);
    }

    #[test]
    fn bad_input_is_rejected_before_any_request() {
        let bot = DiscordBot::with_base_url("t", "http://127.0.0.1:1");
        assert!(bot.channels("1/../../x").is_err());
        assert!(bot.messages("abc", 1).is_err());
        assert!(bot.send_message("1", "  ").is_err());
        assert!(bot.send_message("1", &"x".repeat(2001)).is_err());
    }

    #[test]
    fn http_errors_become_operation_errors_without_leaking_the_token() {
        let (url, _rx) = serve(403, r#"{"message":"Missing Access"}"#);
        let error = DiscordBot::with_base_url("secret-token", url)
            .guilds()
            .expect_err("403");
        let shown = error.to_string();
        assert!(
            shown.contains("403") && !shown.contains("secret-token"),
            "{shown}"
        );
    }
}
