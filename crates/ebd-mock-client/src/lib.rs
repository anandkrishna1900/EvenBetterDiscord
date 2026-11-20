//! Deterministic offline adapter for development and tests.
//!
//! The mock never touches the network. It exists so EBD Core, the CLI, and future
//! UI layers can be developed and tested before any real client is available.
use ebd_contract::*;

pub const MOCK_CLIENT_NAME: &str = "EBD Mock Client";
pub const MOCK_USER_ID: &str = "mock-user";
pub const MOCK_GUILD_ID: &str = "mock-guild";
pub const MOCK_CHANNEL_ID: &str = "mock-general";

/// Chronological seed conversation, oldest first.
const MOCK_MESSAGES: [&str; 2] = [
    "Welcome to the EBD mock client.",
    "This channel exists entirely offline.",
];

const MOCK_CAPABILITIES: [Capability; 6] = Capability::ALL;

#[derive(Debug, Default)]
pub struct MockClient;

impl MockClient {
    fn unknown_channel(channel_id: &str) -> AdapterError {
        AdapterError::Operation(format!("unknown mock channel: {channel_id}"))
    }
}

impl ClientAdapter for MockClient {
    fn info(&self) -> ClientInfo {
        ClientInfo {
            client_name: MOCK_CLIENT_NAME.into(),
            // Keep these in sync with the crate version so a release bump does
            // not silently report a stale version.
            client_version: env!("CARGO_PKG_VERSION").into(),
            adapter_version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn capabilities(&self) -> Vec<Capability> {
        MOCK_CAPABILITIES.to_vec()
    }

    fn current_user(&self) -> Result<User, AdapterError> {
        Ok(User {
            id: MOCK_USER_ID.into(),
            username: "EBD Developer".into(),
        })
    }

    fn guilds(&self) -> Result<Vec<Guild>, AdapterError> {
        Ok(vec![Guild {
            id: MOCK_GUILD_ID.into(),
            name: "Example Community".into(),
        }])
    }

    fn channels(&self, guild_id: &str) -> Result<Vec<Channel>, AdapterError> {
        if guild_id != MOCK_GUILD_ID {
            return Err(AdapterError::Operation(format!(
                "unknown mock guild: {guild_id}"
            )));
        }

        Ok(vec![Channel {
            id: MOCK_CHANNEL_ID.into(),
            guild_id: Some(MOCK_GUILD_ID.into()),
            name: "general".into(),
        }])
    }

    fn messages(&self, channel_id: &str, limit: usize) -> Result<Vec<Message>, AdapterError> {
        if channel_id != MOCK_CHANNEL_ID {
            return Err(Self::unknown_channel(channel_id));
        }

        Ok(MOCK_MESSAGES
            .iter()
            .enumerate()
            .rev()
            .take(limit)
            .map(|(index, content)| Message {
                id: format!("mock-message-{}", index + 1),
                channel_id: MOCK_CHANNEL_ID.into(),
                author_id: MOCK_USER_ID.into(),
                content: (*content).into(),
            })
            .collect())
    }

    fn send_message(&self, channel_id: &str, content: &str) -> Result<Message, AdapterError> {
        if channel_id != MOCK_CHANNEL_ID {
            return Err(Self::unknown_channel(channel_id));
        }

        if content.trim().is_empty() {
            return Err(AdapterError::Operation(
                "message content must not be empty".into(),
            ));
        }

        Ok(Message {
            id: "mock-message-sent".into(),
            channel_id: channel_id.into(),
            author_id: MOCK_USER_ID.into(),
            content: content.into(),
        })
    }

    fn open_external_url(&self, url: &str) -> Result<(), AdapterError> {
        // Deny-by-default: only absolute https URLs with a real host are handed
        // to the OS. This is deliberately stricter than the URL grammar: anything
        // ambiguous is refused rather than guessed at.
        if is_absolute_https(url) {
            Ok(())
        } else {
            Err(AdapterError::Operation(format!(
                "refusing to open a URL that is not absolute https: {url}"
            )))
        }
    }
}

/// Accepts `https://<host>[/path][?query][#fragment]` where `<host>` is
/// non-empty and free of whitespace, control characters, and empty ports.
///
/// This is a guard for a mock adapter, not a URL parser.
fn is_absolute_https(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };

    if rest
        .chars()
        .any(|character| character.is_whitespace() || character.is_control())
    {
        return false;
    }

    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority.rsplit('@').next().unwrap_or_default();

    if host.starts_with('[') {
        // Bracketed IPv6 literal, possibly with a port: `[::1]:8443`.
        let Some((literal, remainder)) = host.split_once(']') else {
            return false;
        };
        return literal.len() > 1 && (remainder.is_empty() || remainder.starts_with(':'));
    }

    let (host, port) = match host.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (host, None),
    };

    if host.is_empty() || host == "." {
        return false;
    }
    if let Some(port) = port {
        if port.is_empty() || !port.chars().all(|character| character.is_ascii_digit()) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> MockClient {
        MockClient
    }

    #[test]
    fn declares_every_contract_capability() {
        let capabilities = client().capabilities();

        for capability in Capability::ALL {
            assert!(capabilities.contains(&capability), "missing {capability}");
        }
        assert_eq!(capabilities.len(), Capability::ALL.len());
    }

    #[test]
    fn reports_its_own_identity() {
        let info = client().info();

        assert_eq!(info.client_name, MOCK_CLIENT_NAME);
        assert_eq!(info.client_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.adapter_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn returns_a_deterministic_user() {
        let user = client().current_user().expect("mock user");

        assert_eq!(user.id, MOCK_USER_ID);
        assert!(!user.username.is_empty());
    }

    #[test]
    fn channels_require_a_known_guild() {
        assert_eq!(
            client().channels(MOCK_GUILD_ID).expect("known guild").len(),
            1
        );
        assert!(client().channels("other-guild").is_err());
    }

    #[test]
    fn messages_are_newest_first_and_respect_the_limit() {
        let messages = client().messages(MOCK_CHANNEL_ID, 5).expect("messages");

        assert_eq!(messages.len(), MOCK_MESSAGES.len());
        assert_eq!(messages[0].content, MOCK_MESSAGES[MOCK_MESSAGES.len() - 1]);
        assert!(messages.iter().all(|m| m.channel_id == MOCK_CHANNEL_ID));

        assert_eq!(client().messages(MOCK_CHANNEL_ID, 1).expect("one").len(), 1);
        assert!(client()
            .messages(MOCK_CHANNEL_ID, 0)
            .expect("none")
            .is_empty());
        assert!(client().messages("other-channel", 1).is_err());
    }

    #[test]
    fn send_message_echoes_valid_content() {
        let message = client()
            .send_message(MOCK_CHANNEL_ID, "hello")
            .expect("send");

        assert_eq!(message.content, "hello");
        assert_eq!(message.channel_id, MOCK_CHANNEL_ID);
        assert_eq!(message.author_id, MOCK_USER_ID);
    }

    #[test]
    fn send_message_rejects_unknown_channels_and_blank_content() {
        assert!(client().send_message("other-channel", "hello").is_err());
        assert!(client().send_message(MOCK_CHANNEL_ID, "   \n").is_err());
    }

    #[test]
    fn external_urls_must_be_absolute_https() {
        for url in [
            "https://example.com/docs",
            "https://example.com:8443/a?b#c",
            "https://user:pass@example.com/x",
            "https://[::1]:8443/",
        ] {
            assert!(
                client().open_external_url(url).is_ok(),
                "should accept {url}"
            );
        }

        for url in [
            "http://example.com",
            "file:///C:/Windows/System32/calc.exe",
            "javascript:alert(1)",
            "https://",
            "https:// foo",
            "https://example.com:abc",
            "https://:443",
            "https://[::1",
            "example.com",
            "",
        ] {
            assert!(
                client().open_external_url(url).is_err(),
                "should reject {url}"
            );
        }
    }

    #[test]
    fn mock_passes_conformance() {
        assert!(ebd_contract::conformance::check_gating(&MockClient).is_empty());
    }
}
