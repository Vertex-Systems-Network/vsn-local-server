use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderKind {
    Runtime,
    Database,
    Service,
    Project,
    Container,
    Cloud,
    Os,
    Network,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityState {
    Supported,
    Unsupported,
    RequiresApproval,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineIdentity {
    pub device_id: String,
    pub display_name: String,
    pub os: String,
    pub public_key: String,
    pub created_at_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthStatus {
    pub service: String,
    pub healthy: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityStatus {
    pub device_identity_ready: bool,
    pub ipc_secret_ready: bool,
    pub secure_store: String,
}
