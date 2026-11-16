//! Stable boundary types shared by EBD Core and client adapters.
use serde::{Deserialize, Serialize};
use std::fmt;

pub const CONTRACT_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    ReadCurrentUser,
    ReadGuilds,
    ReadChannels,
    ReadMessages,
    SendMessages,
    OpenExternalUrl,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientInfo {
    pub client_name: String,
    pub client_version: String,
    pub adapter_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Guild {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Channel {
    pub id: String,
    pub guild_id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub id: String,
    pub channel_id: String,
    pub author_id: String,
    pub content: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("capability is not supported: {0}")]
    Unsupported(&'static str),
    #[error("adapter operation failed: {0}")]
    Operation(String),
}

pub trait ClientAdapter: Send + Sync + fmt::Debug {
    fn info(&self) -> ClientInfo;
    fn capabilities(&self) -> Vec<Capability>;
    fn current_user(&self) -> Result<User, AdapterError>;
    fn guilds(&self) -> Result<Vec<Guild>, AdapterError>;
    fn channels(&self, guild_id: &str) -> Result<Vec<Channel>, AdapterError>;
    fn send_message(&self, channel_id: &str, content: &str) -> Result<Message, AdapterError>;
}
