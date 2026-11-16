//! Client-independent EBD orchestration.
use ebd_contract::{Capability, ClientAdapter, ClientInfo};

#[derive(Debug, Clone)]
pub struct CompatibilityReport {
    pub client: ClientInfo,
    pub contract_version: &'static str,
    pub capabilities: Vec<Capability>,
}

#[derive(Default)]
pub struct AdapterManager {
    adapter: Option<Box<dyn ClientAdapter>>,
}

impl AdapterManager {
    pub fn new() -> Self { Self::default() }

    pub fn attach(&mut self, adapter: Box<dyn ClientAdapter>) {
        self.adapter = Some(adapter);
    }

    pub fn detach(&mut self) { self.adapter = None; }

    pub fn report(&self) -> Option<CompatibilityReport> {
        self.adapter.as_ref().map(|adapter| CompatibilityReport {
            client: adapter.info(),
            contract_version: ebd_contract::CONTRACT_VERSION,
            capabilities: adapter.capabilities(),
        })
    }

    pub fn adapter(&self) -> Option<&dyn ClientAdapter> {
        self.adapter.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_manager_has_no_report() {
        assert!(AdapterManager::new().report().is_none());
    }
}
