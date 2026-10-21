use super::registry_app::RegistryApp;
use serde_json::Value;

pub(super) enum AppRead {
    Listed(Box<RegistryApp>),
    Skipped(Value),
    Excluded,
}
