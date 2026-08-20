use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    MachineView,
    MachineManage,
    ProjectView,
    ProjectEdit,
    RuntimeView,
    RuntimeManage,
    ServiceView,
    ServiceManage,
    NetworkView,
    NetworkManage,
    RemoteView,
    RemoteManage,
    TerminalView,
    TerminalExecute,
    TerminalAdmin,
    FilesRead,
    FilesWrite,
    DatabaseView,
    DatabaseQuery,
    DatabaseWrite,
    DatabaseDestructive,
    SecretsUse,
    SecretsManage,
    SecretsReveal,
    SecurityAuditView,
}

impl Permission {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MachineView => "machine.view",
            Self::MachineManage => "machine.manage",
            Self::ProjectView => "project.view",
            Self::ProjectEdit => "project.edit",
            Self::RuntimeView => "runtime.view",
            Self::RuntimeManage => "runtime.manage",
            Self::ServiceView => "service.view",
            Self::ServiceManage => "service.manage",
            Self::NetworkView => "network.view",
            Self::NetworkManage => "network.manage",
            Self::RemoteView => "remote.view",
            Self::RemoteManage => "remote.manage",
            Self::TerminalView => "terminal.view",
            Self::TerminalExecute => "terminal.execute",
            Self::TerminalAdmin => "terminal.admin",
            Self::FilesRead => "files.read",
            Self::FilesWrite => "files.write",
            Self::DatabaseView => "database.view",
            Self::DatabaseQuery => "database.query",
            Self::DatabaseWrite => "database.write",
            Self::DatabaseDestructive => "database.destructive",
            Self::SecretsUse => "secrets.use",
            Self::SecretsManage => "secrets.manage",
            Self::SecretsReveal => "secrets.reveal",
            Self::SecurityAuditView => "security.audit.view",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        Some(match value {
            "machine.view" => Self::MachineView,
            "machine.manage" => Self::MachineManage,
            "project.view" => Self::ProjectView,
            "project.edit" => Self::ProjectEdit,
            "runtime.view" => Self::RuntimeView,
            "runtime.manage" => Self::RuntimeManage,
            "service.view" => Self::ServiceView,
            "service.manage" => Self::ServiceManage,
            "network.view" => Self::NetworkView,
            "network.manage" => Self::NetworkManage,
            "remote.view" => Self::RemoteView,
            "remote.manage" => Self::RemoteManage,
            "terminal.view" => Self::TerminalView,
            "terminal.execute" => Self::TerminalExecute,
            "terminal.admin" => Self::TerminalAdmin,
            "files.read" => Self::FilesRead,
            "files.write" => Self::FilesWrite,
            "database.view" => Self::DatabaseView,
            "database.query" => Self::DatabaseQuery,
            "database.write" => Self::DatabaseWrite,
            "database.destructive" => Self::DatabaseDestructive,
            "secrets.use" => Self::SecretsUse,
            "secrets.manage" => Self::SecretsManage,
            "secrets.reveal" => Self::SecretsReveal,
            "security.audit.view" => Self::SecurityAuditView,
            _ => return None,
        })
    }

    pub fn is_high_risk(self) -> bool {
        matches!(
            self,
            Self::MachineManage
                | Self::NetworkManage
                | Self::TerminalAdmin
                | Self::DatabaseDestructive
                | Self::SecretsReveal
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principal {
    pub id: String,
    pub kind: String,
    pub permissions: BTreeSet<Permission>,
}

impl Principal {
    pub fn local_authenticated() -> Self {
        use Permission::*;
        Self {
            id: "local-authenticated-user".into(),
            kind: "local_ipc".into(),
            permissions: [
                MachineView,
                ProjectView,
                ProjectEdit,
                RuntimeView,
                RuntimeManage,
                ServiceView,
                ServiceManage,
                NetworkView,
                RemoteView,
                RemoteManage,
                TerminalView,
                TerminalExecute,
                FilesRead,
                FilesWrite,
                DatabaseView,
                DatabaseQuery,
                DatabaseWrite,
                SecretsUse,
                SecretsManage,
                SecurityAuditView,
            ]
            .into_iter()
            .collect(),
        }
    }

    pub fn local_network_admin() -> Self {
        use Permission::*;
        Self { id: "local-elevated-network-admin".into(), kind: "local_os_elevation".into(), permissions: [MachineView, NetworkView, NetworkManage, ServiceView, ServiceManage].into_iter().collect() }
    }

    /// Builds a deliberately narrow remote principal. The control plane may only
    /// delegate one permission per signed command and high-risk permissions stay
    /// blocked until the later approval/MFA policy service exists.
    pub fn remote_delegated(id: impl Into<String>, permission: Permission) -> Result<Self, PolicyError> {
        if permission.is_high_risk() {
            return Err(PolicyError::RemoteHighRisk(permission.as_str()));
        }
        Ok(Self {
            id: id.into(),
            kind: "remote_signed_command".into(),
            permissions: [permission].into_iter().collect(),
        })
    }

    pub fn remote_stream(id: impl Into<String>, permissions: impl IntoIterator<Item=Permission>) -> Result<Self, PolicyError> {
        let permissions: BTreeSet<Permission> = permissions.into_iter().collect();
        if permissions.is_empty() { return Err(PolicyError::Denied("remote stream has no permissions")); }
        if let Some(high)=permissions.iter().copied().find(|p|p.is_high_risk()) {
            return Err(PolicyError::RemoteHighRisk(high.as_str()));
        }
        Ok(Self { id:id.into(), kind:"remote_stream".into(), permissions })
    }
}

#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("permission denied: {0}")]
    Denied(&'static str),
    #[error("remote high-risk permission requires an approval workflow: {0}")]
    RemoteHighRisk(&'static str),
}

pub fn require(principal: &Principal, permission: Permission) -> Result<(), PolicyError> {
    if principal.permissions.contains(&permission) {
        Ok(())
    } else {
        Err(PolicyError::Denied(permission.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_principal_is_default_deny_for_high_risk() {
        let p = Principal::local_authenticated();
        assert!(require(&p, Permission::ServiceManage).is_ok());
        assert!(require(&p, Permission::RemoteManage).is_ok());
        assert!(require(&p, Permission::TerminalAdmin).is_err());
        assert!(require(&p, Permission::DatabaseDestructive).is_err());
        assert!(require(&p, Permission::NetworkManage).is_err());
    }

    #[test]
    fn remote_principal_rejects_high_risk_delegation() {
        assert!(Principal::remote_delegated("u1", Permission::MachineView).is_ok());
        assert!(Principal::remote_delegated("u1", Permission::SecretsReveal).is_err());
    }
}
