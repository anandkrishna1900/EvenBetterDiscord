//! Deterministic offline adapter for development and tests.
use ebd_contract::*;

#[derive(Debug, Default)]
pub struct MockClient;

impl ClientAdapter for MockClient {
    fn info(&self) -> ClientInfo {
        ClientInfo { client_name: "EBD Mock Client".into(), client_version: "0.1.0".into(), adapter_version: "0.1.0".into() }
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::ReadCurrentUser, Capability::ReadGuilds, Capability::ReadChannels, Capability::SendMessages]
    }

    fn current_user(&self) -> Result<User, AdapterError> {
        Ok(User { id: "mock-user".into(), username: "EBD Developer".into() })
    }

    fn guilds(&self) -> Result<Vec<Guild>, AdapterError> {
        Ok(vec![Guild { id: "mock-guild".into(), name: "Example Community".into() }])
    }

    fn channels(&self, guild_id: &str) -> Result<Vec<Channel>, AdapterError> {
        if guild_id != "mock-guild" { return Ok(Vec::new()); }
        Ok(vec![Channel { id: "mock-general".into(), guild_id: Some(guild_id.into()), name: "general".into() }])
    }

    fn send_message(&self, channel_id: &str, content: &str) -> Result<Message, AdapterError> {
        if channel_id != "mock-general" {
            return Err(AdapterError::Operation("unknown mock channel".into()));
        }
        Ok(Message { id: "mock-message-1".into(), channel_id: channel_id.into(), author_id: "mock-user".into(), content: content.into() })
    }
}
