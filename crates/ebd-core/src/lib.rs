//! Client-independent EBD orchestration.
use ebd_contract::{AdapterError, Capability, ClientAdapter, ClientInfo};

#[derive(Debug, Clone)]
pub struct CompatibilityReport {
    pub client: ClientInfo,
    pub contract_version: &'static str,
    pub capabilities: Vec<Capability>,
}

impl CompatibilityReport {
    /// Returns `true` when the attached adapter declares `capability`.
    pub fn supports(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }

    /// Capabilities defined by the contract that this adapter does not implement.
    pub fn missing(&self) -> Vec<Capability> {
        Capability::ALL
            .iter()
            .copied()
            .filter(|capability| !self.supports(*capability))
            .collect()
    }
}

#[derive(Debug, Default)]
pub struct AdapterManager {
    adapter: Option<Box<dyn ClientAdapter>>,
}

impl AdapterManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attaches an adapter, replacing any adapter already attached.
    pub fn attach(&mut self, adapter: Box<dyn ClientAdapter>) {
        self.adapter = Some(adapter);
    }

    /// Detaches the current adapter and returns it, if any.
    pub fn detach(&mut self) -> Option<Box<dyn ClientAdapter>> {
        self.adapter.take()
    }

    pub fn is_attached(&self) -> bool {
        self.adapter.is_some()
    }

    pub fn report(&self) -> Option<CompatibilityReport> {
        self.adapter.as_ref().map(|adapter| CompatibilityReport {
            client: adapter.info(),
            contract_version: ebd_contract::CONTRACT_VERSION,
            capabilities: adapter.capabilities(),
        })
    }

    /// Returns the attached adapter only when it declares `capability`.
    ///
    /// Fails with [`AdapterError::Operation`] when nothing is attached and with
    /// [`AdapterError::Unsupported`] when the adapter does not declare the
    /// capability.
    pub fn adapter_for(&self, capability: Capability) -> Result<&dyn ClientAdapter, AdapterError> {
        let adapter = self
            .adapter
            .as_deref()
            .ok_or_else(|| AdapterError::Operation("no client adapter is attached".to_owned()))?;

        if adapter.supports(capability) {
            Ok(adapter)
        } else {
            Err(AdapterError::Unsupported(capability))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ebd_contract::{Channel, Guild, Message, User};

    /// Declares only reads, so capability gating can be observed.
    #[derive(Debug)]
    struct ReadOnlyAdapter;

    impl ClientAdapter for ReadOnlyAdapter {
        fn info(&self) -> ClientInfo {
            ClientInfo {
                client_name: "Read Only".into(),
                client_version: "0.1.0".into(),
                adapter_version: "0.1.0".into(),
            }
        }

        fn capabilities(&self) -> Vec<Capability> {
            vec![Capability::ReadCurrentUser]
        }

        fn current_user(&self) -> Result<User, AdapterError> {
            Ok(User {
                id: "u1".into(),
                username: "reader".into(),
            })
        }

        fn guilds(&self) -> Result<Vec<Guild>, AdapterError> {
            Err(AdapterError::Unsupported(Capability::ReadGuilds))
        }

        fn channels(&self, _guild_id: &str) -> Result<Vec<Channel>, AdapterError> {
            Err(AdapterError::Unsupported(Capability::ReadChannels))
        }

        fn messages(&self, _channel_id: &str, _limit: usize) -> Result<Vec<Message>, AdapterError> {
            Err(AdapterError::Unsupported(Capability::ReadMessages))
        }

        fn send_message(&self, _channel_id: &str, _content: &str) -> Result<Message, AdapterError> {
            Err(AdapterError::Unsupported(Capability::SendMessages))
        }

        fn open_external_url(&self, _url: &str) -> Result<(), AdapterError> {
            Err(AdapterError::Unsupported(Capability::OpenExternalUrl))
        }
    }

    fn manager_with_read_only_adapter() -> AdapterManager {
        let mut manager = AdapterManager::new();
        manager.attach(Box::new(ReadOnlyAdapter));
        manager
    }

    #[test]
    fn empty_manager_has_no_report() {
        assert!(AdapterManager::new().report().is_none());
        assert!(!AdapterManager::new().is_attached());
    }

    #[test]
    fn report_reports_contract_version_and_capabilities() {
        let report = manager_with_read_only_adapter()
            .report()
            .expect("adapter attached");

        assert_eq!(report.contract_version, ebd_contract::CONTRACT_VERSION);
        assert_eq!(report.capabilities, vec![Capability::ReadCurrentUser]);
        assert!(report.supports(Capability::ReadCurrentUser));
        assert!(!report.supports(Capability::SendMessages));
    }

    #[test]
    fn missing_lists_only_undeclared_capabilities() {
        let report = manager_with_read_only_adapter()
            .report()
            .expect("adapter attached");

        let missing = report.missing();
        assert!(!missing.contains(&Capability::ReadCurrentUser));
        assert_eq!(missing.len(), Capability::ALL.len() - 1);
    }

    #[test]
    fn adapter_for_rejects_capabilities_the_adapter_did_not_declare() {
        let manager = manager_with_read_only_adapter();

        assert!(manager.adapter_for(Capability::ReadCurrentUser).is_ok());
        assert_eq!(
            manager.adapter_for(Capability::SendMessages).err(),
            Some(AdapterError::Unsupported(Capability::SendMessages))
        );
    }

    #[test]
    fn adapter_for_fails_when_nothing_is_attached() {
        let manager = AdapterManager::new();

        assert!(matches!(
            manager.adapter_for(Capability::ReadCurrentUser),
            Err(AdapterError::Operation(_))
        ));
    }

    #[test]
    fn detach_removes_the_adapter() {
        let mut manager = manager_with_read_only_adapter();

        assert!(manager.detach().is_some());
        assert!(!manager.is_attached());
        assert!(manager.report().is_none());
        assert!(manager.detach().is_none());
    }
}
