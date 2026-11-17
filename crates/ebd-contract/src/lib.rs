//! Stable boundary types shared by EBD Core and client adapters.
use serde::{Deserialize, Serialize};
use std::fmt;

pub const CONTRACT_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    ReadCurrentUser,
    ReadGuilds,
    ReadChannels,
    ReadMessages,
    SendMessages,
    OpenExternalUrl,
}

impl Capability {
    /// Every capability defined by this contract, in declaration order.
    pub const ALL: [Capability; 6] = [
        Capability::ReadCurrentUser,
        Capability::ReadGuilds,
        Capability::ReadChannels,
        Capability::ReadMessages,
        Capability::SendMessages,
        Capability::OpenExternalUrl,
    ];

    /// Stable identifier used on the wire and in user-facing output.
    pub const fn as_str(self) -> &'static str {
        match self {
            Capability::ReadCurrentUser => "read_current_user",
            Capability::ReadGuilds => "read_guilds",
            Capability::ReadChannels => "read_channels",
            Capability::ReadMessages => "read_messages",
            Capability::SendMessages => "send_messages",
            Capability::OpenExternalUrl => "open_external_url",
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientInfo {
    pub client_name: String,
    pub client_version: String,
    pub adapter_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Guild {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub guild_id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub channel_id: String,
    pub author_id: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AdapterError {
    /// The adapter does not declare the capability required for this operation.
    #[error("capability is not supported: {0}")]
    Unsupported(Capability),
    /// The adapter declares the capability but the operation itself failed.
    #[error("adapter operation failed: {0}")]
    Operation(String),
}

/// The only surface EBD Core is allowed to use to talk to a client.
///
/// Implementations must honour [`ClientAdapter::capabilities`]: a method whose
/// capability is not declared should return [`AdapterError::Unsupported`] rather
/// than pretending to succeed. Every method names the capability it needs.
pub trait ClientAdapter: Send + Sync + fmt::Debug {
    fn info(&self) -> ClientInfo;

    /// Capabilities this adapter implements. Callers must check this before use.
    fn capabilities(&self) -> Vec<Capability>;

    /// Requires [`Capability::ReadCurrentUser`].
    fn current_user(&self) -> Result<User, AdapterError>;

    /// Requires [`Capability::ReadGuilds`].
    fn guilds(&self) -> Result<Vec<Guild>, AdapterError>;

    /// Requires [`Capability::ReadChannels`].
    fn channels(&self, guild_id: &str) -> Result<Vec<Channel>, AdapterError>;

    /// Returns at most `limit` messages, newest first. Requires [`Capability::ReadMessages`].
    fn messages(&self, channel_id: &str, limit: usize) -> Result<Vec<Message>, AdapterError>;

    /// Requires [`Capability::SendMessages`].
    fn send_message(&self, channel_id: &str, content: &str) -> Result<Message, AdapterError>;

    /// Asks the client to hand a URL to the operating system. Requires
    /// [`Capability::OpenExternalUrl`].
    fn open_external_url(&self, url: &str) -> Result<(), AdapterError>;

    /// Returns `true` when the adapter declares `capability`.
    fn supports(&self, capability: Capability) -> bool {
        self.capabilities().contains(&capability)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_list_has_no_duplicates() {
        let mut sorted = Capability::ALL;
        sorted.sort();
        for pair in sorted.windows(2) {
            assert_ne!(pair[0], pair[1], "duplicate capability: {}", pair[0]);
        }
    }

    #[test]
    fn capability_names_match_the_wire_format() {
        for capability in Capability::ALL {
            let json = serde_json::to_string(&capability).expect("serialise capability");
            assert_eq!(json, format!("\"{}\"", capability.as_str()));
            assert_eq!(capability.to_string(), capability.as_str());
        }
    }

    #[test]
    fn message_round_trips_through_json() {
        let message = Message {
            id: "m1".into(),
            channel_id: "c1".into(),
            author_id: "u1".into(),
            content: "hello".into(),
        };

        let json = serde_json::to_string(&message).expect("serialise message");
        let decoded: Message = serde_json::from_str(&json).expect("deserialise message");
        assert_eq!(decoded, message);
    }

    #[test]
    fn unsupported_error_names_the_capability() {
        let error = AdapterError::Unsupported(Capability::ReadMessages);
        assert_eq!(
            error.to_string(),
            "capability is not supported: read_messages"
        );
    }
}
