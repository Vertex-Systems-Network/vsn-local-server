use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CloudError {
    #[error("invalid workspace specification: {0}")]
    Invalid(String),
    #[error("provider does not support capability: {0}")]
    Unsupported(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CloudCapability {
    Compute,
    BlockStorage,
    ObjectStorage,
    ManagedDatabase,
    Firewall,
    Dns,
    Snapshot,
    Clone,
    SshBootstrap,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudProviderDescriptor {
    pub id: String,
    pub display_name: String,
    pub capabilities: Vec<CloudCapability>,
    #[serde(default)]
    pub regions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceSpec {
    pub name: String,
    pub provider: String,
    pub region: String,
    pub machine_type: String,
    pub os_image: String,
    #[serde(default)]
    pub disk_gb: u32,
    #[serde(default)]
    pub runtime_requirements: BTreeMap<String, String>,
    #[serde(default)]
    pub services: Vec<String>,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvisionStep {
    pub id: String,
    pub description: String,
    pub capability: CloudCapability,
    pub destructive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvisionPlan {
    pub workspace: WorkspaceSpec,
    pub steps: Vec<ProvisionStep>,
    pub requires_explicit_apply: bool,
}

pub trait CloudProvider: Send + Sync {
    fn descriptor(&self) -> CloudProviderDescriptor;
    fn plan(&self, spec: &WorkspaceSpec) -> Result<ProvisionPlan, CloudError>;
}

pub fn validate_workspace_spec(spec: &WorkspaceSpec) -> Result<(), CloudError> {
    if spec.name.len() < 2
        || spec.name.len() > 96
        || !spec
            .name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(CloudError::Invalid(
            "workspace name must be a safe identifier".into(),
        ));
    }
    for (field, value) in [
        ("provider", &spec.provider),
        ("region", &spec.region),
        ("machine_type", &spec.machine_type),
        ("os_image", &spec.os_image),
    ] {
        if value.trim().is_empty() || value.len() > 160 {
            return Err(CloudError::Invalid(format!(
                "{field} is missing or too long"
            )));
        }
    }
    if !(8..=16_384).contains(&spec.disk_gb) {
        return Err(CloudError::Invalid(
            "disk_gb must be between 8 and 16384".into(),
        ));
    }
    if spec.runtime_requirements.len() > 32 || spec.services.len() > 64 || spec.labels.len() > 64 {
        return Err(CloudError::Invalid(
            "workspace contains too many runtime/service/label entries".into(),
        ));
    }
    Ok(())
}

pub fn generic_ssh_plan(spec: &WorkspaceSpec) -> Result<ProvisionPlan, CloudError> {
    validate_workspace_spec(spec)?;
    Ok(ProvisionPlan {
        workspace: spec.clone(),
        requires_explicit_apply: true,
        steps: vec![
            ProvisionStep {
                id: "compute.create".into(),
                description: "Create isolated compute instance".into(),
                capability: CloudCapability::Compute,
                destructive: false,
            },
            ProvisionStep {
                id: "firewall.lockdown".into(),
                description:
                    "Apply default-deny inbound firewall; retain provider management path only"
                        .into(),
                capability: CloudCapability::Firewall,
                destructive: false,
            },
            ProvisionStep {
                id: "disk.attach".into(),
                description: "Attach encrypted workspace disk".into(),
                capability: CloudCapability::BlockStorage,
                destructive: false,
            },
            ProvisionStep {
                id: "agent.bootstrap".into(),
                description: "Bootstrap VSN Agent over an authenticated SSH/provider channel"
                    .into(),
                capability: CloudCapability::SshBootstrap,
                destructive: false,
            },
            ProvisionStep {
                id: "agent.enroll".into(),
                description: "Enroll workspace Agent with the VSN Control Plane".into(),
                capability: CloudCapability::SshBootstrap,
                destructive: false,
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_requires_safe_name_and_reasonable_disk() {
        let mut spec = WorkspaceSpec {
            name: "dev-1".into(),
            provider: "generic-ssh".into(),
            region: "local".into(),
            machine_type: "vm".into(),
            os_image: "ubuntu".into(),
            disk_gb: 64,
            runtime_requirements: BTreeMap::new(),
            services: vec![],
            labels: BTreeMap::new(),
        };
        assert!(generic_ssh_plan(&spec).is_ok());
        spec.name = "../bad".into();
        assert!(generic_ssh_plan(&spec).is_err());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExistingSshTarget {
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub user: String,
    pub identity_file: String,
    pub known_hosts_file: String,
}
fn default_ssh_port() -> u16 {
    22
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SshPreflightResult {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub ok: bool,
    pub remote_marker: String,
}

pub fn ssh_preflight(target: &ExistingSshTarget) -> Result<SshPreflightResult, CloudError> {
    validate_ssh_target(target)?;
    let ssh = vsn_system::find_executable("ssh")
        .map_err(|e| CloudError::Invalid(format!("ssh executable unavailable: {e}")))?;
    let output = std::process::Command::new(ssh)
        .args([
            "-o",
            "BatchMode=yes",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            "ConnectTimeout=8",
        ])
        .arg("-o")
        .arg(format!("UserKnownHostsFile={}", target.known_hosts_file))
        .arg("-i")
        .arg(&target.identity_file)
        .arg("-p")
        .arg(target.port.to_string())
        .arg(format!("{}@{}", target.user, target.host))
        .arg("printf 'VSN_SSH_OK'")
        .output()
        .map_err(|e| CloudError::Invalid(format!("ssh preflight failed to start: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CloudError::Invalid(format!(
            "ssh preflight rejected: {}",
            stderr.chars().take(2048).collect::<String>()
        )));
    }
    let marker = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if marker != "VSN_SSH_OK" {
        return Err(CloudError::Invalid(
            "ssh preflight returned unexpected marker".into(),
        ));
    }
    Ok(SshPreflightResult {
        host: target.host.clone(),
        port: target.port,
        user: target.user.clone(),
        ok: true,
        remote_marker: marker,
    })
}
fn validate_ssh_target(target: &ExistingSshTarget) -> Result<(), CloudError> {
    if target.host.is_empty()
        || target.host.len() > 253
        || !target
            .host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':' | b'[' | b']'))
    {
        return Err(CloudError::Invalid("SSH host is invalid".into()));
    }
    if target.port == 0 {
        return Err(CloudError::Invalid("SSH port must be non-zero".into()));
    }
    if target.user.is_empty()
        || target.user.len() > 64
        || !target
            .user
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(CloudError::Invalid("SSH user is invalid".into()));
    }
    for (name, path) in [
        ("identity_file", &target.identity_file),
        ("known_hosts_file", &target.known_hosts_file),
    ] {
        let p = std::path::Path::new(path);
        if !p.is_absolute() || !p.is_file() {
            return Err(CloudError::Invalid(format!(
                "{name} must be an existing absolute file"
            )));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExistingSshWorkspaceRequest {
    pub target: ExistingSshTarget,
    pub workspace_name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExistingSshWorkspaceResult {
    pub workspace_name: String,
    pub remote_path: String,
    pub state: String,
    pub marker: String,
}

pub fn ssh_workspace_prepare(
    request: &ExistingSshWorkspaceRequest,
) -> Result<ExistingSshWorkspaceResult, CloudError> {
    validate_workspace_name(&request.workspace_name)?;
    ssh_preflight(&request.target)?;
    let path = remote_workspace_path(&request.workspace_name);
    let command=format!("umask 077; mkdir -p -- \"{path}\" && chmod 700 -- \"{path}\" && printf 'VSN_WORKSPACE_READY'");
    let marker = run_fixed_ssh(&request.target, &command)?;
    if marker != "VSN_WORKSPACE_READY" {
        return Err(CloudError::Invalid(
            "SSH workspace prepare returned unexpected marker".into(),
        ));
    }
    Ok(ExistingSshWorkspaceResult {
        workspace_name: request.workspace_name.clone(),
        remote_path: path,
        state: "ready".into(),
        marker,
    })
}
pub fn ssh_workspace_status(
    request: &ExistingSshWorkspaceRequest,
) -> Result<ExistingSshWorkspaceResult, CloudError> {
    validate_workspace_name(&request.workspace_name)?;
    let path = remote_workspace_path(&request.workspace_name);
    let command=format!("if test -d \"{path}\"; then printf 'VSN_WORKSPACE_READY'; else printf 'VSN_WORKSPACE_MISSING'; fi");
    let marker = run_fixed_ssh(&request.target, &command)?;
    let state = match marker.as_str() {
        "VSN_WORKSPACE_READY" => "ready",
        "VSN_WORKSPACE_MISSING" => "missing",
        _ => {
            return Err(CloudError::Invalid(
                "SSH workspace status returned unexpected marker".into(),
            ))
        }
    };
    Ok(ExistingSshWorkspaceResult {
        workspace_name: request.workspace_name.clone(),
        remote_path: path,
        state: state.into(),
        marker,
    })
}
pub fn ssh_workspace_remove_empty(
    request: &ExistingSshWorkspaceRequest,
) -> Result<ExistingSshWorkspaceResult, CloudError> {
    validate_workspace_name(&request.workspace_name)?;
    let path = remote_workspace_path(&request.workspace_name);
    // Deliberately use rmdir rather than recursive deletion. A non-empty workspace fails closed.
    let command=format!("if test ! -d \"{path}\"; then printf 'VSN_WORKSPACE_MISSING'; elif rmdir -- \"{path}\" 2>/dev/null; then printf 'VSN_WORKSPACE_REMOVED'; else printf 'VSN_WORKSPACE_NOT_EMPTY'; fi");
    let marker = run_fixed_ssh(&request.target, &command)?;
    let state = match marker.as_str() {
        "VSN_WORKSPACE_REMOVED" => "removed",
        "VSN_WORKSPACE_MISSING" => "missing",
        "VSN_WORKSPACE_NOT_EMPTY" => "not_empty",
        _ => {
            return Err(CloudError::Invalid(
                "SSH workspace remove returned unexpected marker".into(),
            ))
        }
    };
    Ok(ExistingSshWorkspaceResult {
        workspace_name: request.workspace_name.clone(),
        remote_path: path,
        state: state.into(),
        marker,
    })
}
fn validate_workspace_name(name: &str) -> Result<(), CloudError> {
    if name.len() < 2
        || name.len() > 96
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        Err(CloudError::Invalid(
            "SSH workspace name must be a safe identifier".into(),
        ))
    } else {
        Ok(())
    }
}
fn remote_workspace_path(name: &str) -> String {
    format!("$HOME/.vsn/workspaces/{name}")
}
fn run_fixed_ssh(target: &ExistingSshTarget, remote_command: &str) -> Result<String, CloudError> {
    validate_ssh_target(target)?;
    let ssh = vsn_system::find_executable("ssh")
        .map_err(|e| CloudError::Invalid(format!("ssh executable unavailable: {e}")))?;
    let output = std::process::Command::new(ssh)
        .args([
            "-o",
            "BatchMode=yes",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            "ConnectTimeout=8",
        ])
        .arg("-o")
        .arg(format!("UserKnownHostsFile={}", target.known_hosts_file))
        .arg("-i")
        .arg(&target.identity_file)
        .arg("-p")
        .arg(target.port.to_string())
        .arg(format!("{}@{}", target.user, target.host))
        .arg(remote_command)
        .output()
        .map_err(|e| CloudError::Invalid(format!("ssh workspace command failed to start: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CloudError::Invalid(format!(
            "ssh workspace command rejected: {}",
            stderr.chars().take(2048).collect::<String>()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SshReleaseUploadRequest {
    pub target: ExistingSshTarget,
    pub workspace_name: String,
    pub release_id: String,
    pub artifact_path: String,
    #[serde(default)]
    pub expected_sha256: Option<String>,
    #[serde(default)]
    pub activate: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SshReleaseResult {
    pub workspace_name: String,
    pub release_id: String,
    pub remote_artifact: String,
    pub sha256: String,
    pub bytes: u64,
    pub active: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SshReleasePointerRequest {
    pub target: ExistingSshTarget,
    pub workspace_name: String,
    pub release_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SshReleaseStatus {
    pub workspace_name: String,
    pub active_release: Option<String>,
    pub previous_release: Option<String>,
}

pub fn ssh_release_upload(
    request: &SshReleaseUploadRequest,
) -> Result<SshReleaseResult, CloudError> {
    validate_workspace_name(&request.workspace_name)?;
    validate_release_id(&request.release_id)?;
    ssh_preflight(&request.target)?;
    let artifact = std::path::Path::new(&request.artifact_path);
    if !artifact.is_absolute() || !artifact.is_file() {
        return Err(CloudError::Invalid(
            "release artifact must be an existing absolute file".into(),
        ));
    }
    let meta = std::fs::metadata(artifact)
        .map_err(|e| CloudError::Invalid(format!("release artifact metadata failed: {e}")))?;
    if meta.len() == 0 || meta.len() > 8 * 1024 * 1024 * 1024 {
        return Err(CloudError::Invalid(
            "release artifact must be between 1 byte and 8 GiB".into(),
        ));
    }
    let digest = sha256_file(artifact)?;
    if let Some(expected) = request.expected_sha256.as_deref() {
        validate_sha256(expected)?;
        if !digest.eq_ignore_ascii_case(expected) {
            return Err(CloudError::Invalid(
                "release artifact SHA-256 mismatch".into(),
            ));
        }
    }
    let relative_dir = format!(
        ".vsn/workspaces/{}/releases/{}",
        request.workspace_name, request.release_id
    );
    let absolute_dir = format!("$HOME/{relative_dir}");
    let marker=run_fixed_ssh(&request.target,&format!("umask 077; mkdir -p -- \"{absolute_dir}\" && chmod 700 -- \"{absolute_dir}\" && printf 'VSN_RELEASE_DIR_READY'"))?;
    if marker != "VSN_RELEASE_DIR_READY" {
        return Err(CloudError::Invalid(
            "release directory prepare returned unexpected marker".into(),
        ));
    }
    let remote_part = format!("{relative_dir}/artifact.bundle.part");
    run_scp_upload(&request.target, artifact, &remote_part)?;
    let absolute_part = format!("$HOME/{remote_part}");
    let absolute_final = format!("$HOME/{relative_dir}/artifact.bundle");
    let finalize=run_fixed_ssh(&request.target,&format!("if test -f \"{absolute_part}\"; then mv -f -- \"{absolute_part}\" \"{absolute_final}\" && chmod 600 -- \"{absolute_final}\" && printf 'VSN_RELEASE_UPLOADED'; else printf 'VSN_RELEASE_PART_MISSING'; fi"))?;
    if finalize != "VSN_RELEASE_UPLOADED" {
        return Err(CloudError::Invalid(
            "release upload finalization failed".into(),
        ));
    }
    let active = if request.activate {
        ssh_release_activate(&SshReleasePointerRequest {
            target: request.target.clone(),
            workspace_name: request.workspace_name.clone(),
            release_id: request.release_id.clone(),
        })?;
        true
    } else {
        false
    };
    Ok(SshReleaseResult {
        workspace_name: request.workspace_name.clone(),
        release_id: request.release_id.clone(),
        remote_artifact: absolute_final,
        sha256: digest,
        bytes: meta.len(),
        active,
    })
}

pub fn ssh_release_activate(
    request: &SshReleasePointerRequest,
) -> Result<SshReleaseStatus, CloudError> {
    validate_workspace_name(&request.workspace_name)?;
    validate_release_id(&request.release_id)?;
    ssh_preflight(&request.target)?;
    let root = format!("$HOME/.vsn/workspaces/{}", request.workspace_name);
    let artifact = format!("{root}/releases/{}/artifact.bundle", request.release_id);
    let next = format!("{root}/CURRENT.next");
    let current = format!("{root}/CURRENT");
    let previous = format!("{root}/PREVIOUS");
    // release_id is restricted to a shell-safe identifier; no user-provided command text is accepted.
    let command=format!("if test ! -f \"{artifact}\"; then printf 'VSN_RELEASE_MISSING'; else umask 077; if test -f \"{current}\"; then old=$(cat -- \"{current}\"); case \"$old\" in (*[!A-Za-z0-9_-]*|'') printf 'VSN_RELEASE_POINTER_INVALID'; exit 0;; esac; printf '%s' \"$old\" > \"{previous}.next\" && mv -f -- \"{previous}.next\" \"{previous}\"; fi; printf '%s' '{}' > \"{next}\" && mv -f -- \"{next}\" \"{current}\" && chmod 600 -- \"{current}\" \"{previous}\" 2>/dev/null || chmod 600 -- \"{current}\"; printf 'VSN_RELEASE_ACTIVE'; fi",request.release_id);
    let marker = run_fixed_ssh(&request.target, &command)?;
    if marker != "VSN_RELEASE_ACTIVE" {
        return Err(CloudError::Invalid(
            "requested release does not exist or could not be activated".into(),
        ));
    }
    ssh_release_status(&request.target, &request.workspace_name)
}

pub fn ssh_release_status(
    target: &ExistingSshTarget,
    workspace_name: &str,
) -> Result<SshReleaseStatus, CloudError> {
    validate_workspace_name(workspace_name)?;
    ssh_preflight(target)?;
    let root = format!("$HOME/.vsn/workspaces/{workspace_name}");
    let current = format!("{root}/CURRENT");
    let previous = format!("{root}/PREVIOUS");
    let command=format!("read_ptr() {{ p=\"$1\"; tag=\"$2\"; if test -f \"$p\"; then value=$(cat -- \"$p\"); case \"$value\" in (*[!A-Za-z0-9_-]*|'') printf '%s_INVALID\\n' \"$tag\";; (*) printf '%s:%s\\n' \"$tag\" \"$value\";; esac; else printf '%s:NONE\\n' \"$tag\"; fi; }}; read_ptr \"{current}\" CURRENT; read_ptr \"{previous}\" PREVIOUS");
    let marker = run_fixed_ssh(target, &command)?;
    let mut active = None;
    let mut previous_release = None;
    for line in marker.lines() {
        if line == "CURRENT_INVALID" || line == "PREVIOUS_INVALID" {
            return Err(CloudError::Invalid(
                "remote release pointer is invalid".into(),
            ));
        }
        if let Some(value) = line.strip_prefix("CURRENT:") {
            if value != "NONE" {
                validate_release_id(value)?;
                active = Some(value.into());
            }
        }
        if let Some(value) = line.strip_prefix("PREVIOUS:") {
            if value != "NONE" {
                validate_release_id(value)?;
                previous_release = Some(value.into());
            }
        }
    }
    Ok(SshReleaseStatus {
        workspace_name: workspace_name.into(),
        active_release: active,
        previous_release,
    })
}

pub fn ssh_release_rollback(
    target: &ExistingSshTarget,
    workspace_name: &str,
) -> Result<SshReleaseStatus, CloudError> {
    validate_workspace_name(workspace_name)?;
    ssh_preflight(target)?;
    let root = format!("$HOME/.vsn/workspaces/{workspace_name}");
    let current = format!("{root}/CURRENT");
    let previous = format!("{root}/PREVIOUS");
    let command=format!("if test ! -f \"{previous}\"; then printf 'VSN_ROLLBACK_NONE'; else prev=$(cat -- \"{previous}\"); case \"$prev\" in (*[!A-Za-z0-9_-]*|'') printf 'VSN_ROLLBACK_POINTER_INVALID'; exit 0;; esac; if test ! -f \"{root}/releases/$prev/artifact.bundle\"; then printf 'VSN_ROLLBACK_ARTIFACT_MISSING'; exit 0; fi; old=''; if test -f \"{current}\"; then old=$(cat -- \"{current}\"); case \"$old\" in (*[!A-Za-z0-9_-]*|'') printf 'VSN_ROLLBACK_POINTER_INVALID'; exit 0;; esac; fi; umask 077; printf '%s' \"$prev\" > \"{current}.next\" && mv -f -- \"{current}.next\" \"{current}\"; if test -n \"$old\"; then printf '%s' \"$old\" > \"{previous}.next\" && mv -f -- \"{previous}.next\" \"{previous}\"; else rm -f -- \"{previous}\"; fi; chmod 600 -- \"{current}\" \"{previous}\" 2>/dev/null || chmod 600 -- \"{current}\"; printf 'VSN_ROLLBACK_OK'; fi");
    let marker = run_fixed_ssh(target, &command)?;
    if marker != "VSN_ROLLBACK_OK" {
        return Err(CloudError::Invalid(format!("rollback rejected: {marker}")));
    }
    ssh_release_status(target, workspace_name)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SshReleaseHealthRequest {
    pub target: ExistingSshTarget,
    pub workspace_name: String,
    #[serde(default = "default_health_port")]
    pub port: u16,
    #[serde(default = "default_health_path")]
    pub path: String,
    #[serde(default = "default_health_min")]
    pub expected_status_min: u16,
    #[serde(default = "default_health_max")]
    pub expected_status_max: u16,
    #[serde(default)]
    pub rollback_on_failure: bool,
}
fn default_health_port() -> u16 {
    80
}
fn default_health_path() -> String {
    "/health".into()
}
fn default_health_min() -> u16 {
    200
}
fn default_health_max() -> u16 {
    399
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SshReleaseHealthResult {
    pub workspace_name: String,
    pub healthy: bool,
    pub status_code: Option<u16>,
    pub active_before: Option<String>,
    pub active_after: Option<String>,
    pub rolled_back: bool,
    pub detail: String,
}

pub fn ssh_release_healthcheck(
    request: &SshReleaseHealthRequest,
) -> Result<SshReleaseHealthResult, CloudError> {
    validate_workspace_name(&request.workspace_name)?;
    validate_health_path(&request.path)?;
    if request.port == 0 {
        return Err(CloudError::Invalid(
            "health-check port must be non-zero".into(),
        ));
    }
    if request.expected_status_min < 100
        || request.expected_status_max > 599
        || request.expected_status_min > request.expected_status_max
    {
        return Err(CloudError::Invalid(
            "health-check status range must be within 100..599".into(),
        ));
    }
    ssh_preflight(&request.target)?;
    let before = ssh_release_status(&request.target, &request.workspace_name)?;
    let command=format!("if ! command -v curl >/dev/null 2>&1; then printf 'VSN_HEALTH:CURL_MISSING'; else code=$(curl --silent --show-error --output /dev/null --connect-timeout 3 --max-time 8 --write-out '%{{http_code}}' 'http://127.0.0.1:{}{}' 2>/dev/null || printf '000'); printf 'VSN_HEALTH:%s' \"$code\"; fi",request.port,request.path);
    let marker = run_fixed_ssh(&request.target, &command)?;
    let raw = marker
        .strip_prefix("VSN_HEALTH:")
        .ok_or_else(|| CloudError::Invalid("health check returned unexpected marker".into()))?;
    let status_code = raw.parse::<u16>().ok().filter(|v| *v >= 100 && *v <= 599);
    let healthy = status_code
        .map(|v| v >= request.expected_status_min && v <= request.expected_status_max)
        .unwrap_or(false);
    let mut rolled_back = false;
    let mut after = before.clone();
    if !healthy && request.rollback_on_failure && before.previous_release.is_some() {
        after = ssh_release_rollback(&request.target, &request.workspace_name)?;
        rolled_back = true;
    }
    Ok(SshReleaseHealthResult {
        workspace_name: request.workspace_name.clone(),
        healthy,
        status_code,
        active_before: before.active_release,
        active_after: after.active_release,
        rolled_back,
        detail: if raw == "CURL_MISSING" {
            "remote curl executable is required for deterministic health checks".into()
        } else if healthy {
            "health check passed".into()
        } else {
            "health check failed".into()
        },
    })
}
fn validate_health_path(value: &str) -> Result<(), CloudError> {
    if value.is_empty()
        || value.len() > 256
        || !value.starts_with('/')
        || value.contains("..")
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.'))
    {
        return Err(CloudError::Invalid(
            "health path must be a simple absolute HTTP path without query or traversal".into(),
        ));
    }
    Ok(())
}

fn validate_release_id(value: &str) -> Result<(), CloudError> {
    if value.len() < 4
        || value.len() > 96
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        Err(CloudError::Invalid(
            "release_id must be a safe identifier".into(),
        ))
    } else {
        Ok(())
    }
}
fn validate_sha256(value: &str) -> Result<(), CloudError> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        Err(CloudError::Invalid(
            "expected_sha256 must contain exactly 64 hexadecimal characters".into(),
        ))
    } else {
        Ok(())
    }
}
fn sha256_file(path: &std::path::Path) -> Result<String, CloudError> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut f = std::fs::File::open(path)
        .map_err(|e| CloudError::Invalid(format!("release artifact open failed: {e}")))?;
    let mut h = Sha256::new();
    let mut b = [0u8; 128 * 1024];
    loop {
        let n = f
            .read(&mut b)
            .map_err(|e| CloudError::Invalid(format!("release artifact read failed: {e}")))?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(h.finalize().iter().map(|v| format!("{v:02x}")).collect())
}
fn run_scp_upload(
    target: &ExistingSshTarget,
    local: &std::path::Path,
    remote_relative: &str,
) -> Result<(), CloudError> {
    validate_ssh_target(target)?;
    if remote_relative.is_empty()
        || remote_relative.len() > 512
        || remote_relative.starts_with('/')
        || remote_relative.contains("..")
        || !remote_relative
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'/' | b'-' | b'_'))
    {
        return Err(CloudError::Invalid("remote SCP path is invalid".into()));
    }
    let scp = vsn_system::find_executable("scp")
        .map_err(|e| CloudError::Invalid(format!("scp executable unavailable: {e}")))?;
    let destination = format!("{}@{}:{}", target.user, target.host, remote_relative);
    let output = std::process::Command::new(scp)
        .args([
            "-o",
            "BatchMode=yes",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            "ConnectTimeout=8",
        ])
        .arg("-o")
        .arg(format!("UserKnownHostsFile={}", target.known_hosts_file))
        .arg("-i")
        .arg(&target.identity_file)
        .arg("-P")
        .arg(target.port.to_string())
        .arg(local)
        .arg(destination)
        .output()
        .map_err(|e| CloudError::Invalid(format!("scp upload failed to start: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CloudError::Invalid(format!(
            "scp upload rejected: {}",
            stderr.chars().take(2048).collect::<String>()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod release_tests {
    use super::*;
    #[test]
    fn release_id_is_strict() {
        assert!(validate_release_id("release_2026-08-18").is_ok());
        assert!(validate_release_id("../release").is_err());
    }
    #[test]
    fn sha256_validation_is_exact() {
        assert!(validate_sha256(&"a".repeat(64)).is_ok());
        assert!(validate_sha256("bad").is_err());
    }
    #[test]
    fn health_path_is_bounded() {
        assert!(validate_health_path("/health/ready").is_ok());
        assert!(validate_health_path("/../secret").is_err());
        assert!(validate_health_path("/health?x=1").is_err());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CloudCliProvider {
    Aws,
    Azure,
    Gcp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudCliCreateRequest {
    pub provider: CloudCliProvider,
    pub name: String,
    pub location: String,
    pub machine_type: String,
    pub image: String,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub subnet: Option<String>,
    #[serde(default)]
    pub network: Option<String>,
    #[serde(default)]
    pub admin_username: Option<String>,
    #[serde(default)]
    pub ssh_public_key_file: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudCliInstanceRef {
    pub provider: CloudCliProvider,
    pub instance_id: String,
    pub location: String,
    #[serde(default)]
    pub scope: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudCliDestroyRequest {
    pub instance: CloudCliInstanceRef,
    pub confirm_destroy: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudCliSnapshotRequest {
    pub instance: CloudCliInstanceRef,
    pub snapshot_name: String,
    pub acknowledge_crash_consistency: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudCliCloneRequest {
    pub source: CloudCliInstanceRef,
    pub snapshot_or_image_id: String,
    pub target_name: String,
    #[serde(default)]
    pub target_location: Option<String>,
    #[serde(default)]
    pub subnet: Option<String>,
    #[serde(default)]
    pub network: Option<String>,
    #[serde(default)]
    pub machine_type: Option<String>,
    #[serde(default)]
    pub os_type: Option<String>,
    pub confirm_new_instance: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CloudArtifactKind {
    Ami,
    MachineImage,
    Snapshot,
    IncrementalSnapshot,
    ManagedDisk,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudCliImageCopyRequest {
    pub provider: CloudCliProvider,
    pub source_artifact_id: String,
    pub source_location: String,
    pub target_location: String,
    pub target_name: String,
    pub confirm_copy: bool,
    #[serde(default)]
    pub artifact_kind: Option<CloudArtifactKind>,
    #[serde(default)]
    pub source_scope: Option<String>,
    #[serde(default)]
    pub target_scope: Option<String>,
    #[serde(default)]
    pub os_type: Option<String>,
    #[serde(default)]
    pub sku: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudCliArtifactRef {
    pub provider: CloudCliProvider,
    pub artifact_id: String,
    pub location: String,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub artifact_kind: Option<CloudArtifactKind>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CloudCliResult {
    pub provider: CloudCliProvider,
    pub operation: String,
    pub instance_id: Option<String>,
    pub output: serde_json::Value,
}

pub fn cloud_cli_detect() -> Vec<(CloudCliProvider, bool)> {
    vec![
        (
            CloudCliProvider::Aws,
            vsn_system::find_executable("aws").is_ok(),
        ),
        (
            CloudCliProvider::Azure,
            vsn_system::find_executable("az").is_ok(),
        ),
        (
            CloudCliProvider::Gcp,
            vsn_system::find_executable("gcloud").is_ok(),
        ),
    ]
}

pub fn cloud_cli_create(request: &CloudCliCreateRequest) -> Result<CloudCliResult, CloudError> {
    validate_cloud_cli_create(request)?;
    match request.provider {
        CloudCliProvider::Aws => {
            let exe = vsn_system::find_executable("aws")
                .map_err(|e| CloudError::Invalid(format!("AWS CLI unavailable: {e}")))?;
            let mut args=vec!["ec2".into(),"run-instances".into(),"--region".into(),request.location.clone(),"--image-id".into(),request.image.clone(),"--instance-type".into(),request.machine_type.clone(),"--count".into(),"1".into(),"--no-associate-public-ip-address".into(),"--tag-specifications".into(),format!("ResourceType=instance,Tags=[{{Key=Name,Value={}}},{{Key=vsn-managed,Value=true}}]",request.name),"--query".into(),"Instances[0].{id:InstanceId,state:State.Name,private_ip:PrivateIpAddress}".into(),"--output".into(),"json".into()];
            if let Some(subnet) = &request.subnet {
                args.extend(["--subnet-id".into(), subnet.clone()]);
            }
            let value = run_cli_json(&exe, &args, 120_000)?;
            let id = value.get("id").and_then(|v| v.as_str()).map(str::to_string);
            if id.is_none() {
                return Err(CloudError::Invalid(
                    "AWS create returned no instance id".into(),
                ));
            }
            Ok(CloudCliResult {
                provider: request.provider.clone(),
                operation: "create".into(),
                instance_id: id,
                output: value,
            })
        }
        CloudCliProvider::Azure => {
            let exe = vsn_system::find_executable("az")
                .map_err(|e| CloudError::Invalid(format!("Azure CLI unavailable: {e}")))?;
            let group = request.scope.as_ref().ok_or_else(|| {
                CloudError::Invalid("Azure create requires scope=resource group".into())
            })?;
            let user = request.admin_username.as_ref().ok_or_else(|| {
                CloudError::Invalid("Azure create requires admin_username".into())
            })?;
            let key = request.ssh_public_key_file.as_ref().ok_or_else(|| {
                CloudError::Invalid("Azure create requires ssh_public_key_file".into())
            })?;
            let mut args = vec![
                "vm".into(),
                "create".into(),
                "--resource-group".into(),
                group.clone(),
                "--name".into(),
                request.name.clone(),
                "--location".into(),
                request.location.clone(),
                "--image".into(),
                request.image.clone(),
                "--size".into(),
                request.machine_type.clone(),
                "--admin-username".into(),
                user.clone(),
                "--ssh-key-values".into(),
                format!("@{key}"),
                "--public-ip-address".into(),
                "".into(),
                "--output".into(),
                "json".into(),
            ];
            if let Some(network) = &request.network {
                args.extend(["--vnet-name".into(), network.clone()]);
            }
            if let Some(subnet) = &request.subnet {
                args.extend(["--subnet".into(), subnet.clone()]);
            }
            let value = run_cli_json(&exe, &args, 180_000)?;
            let id = value
                .get("id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .or_else(|| Some(request.name.clone()));
            Ok(CloudCliResult {
                provider: request.provider.clone(),
                operation: "create".into(),
                instance_id: id,
                output: value,
            })
        }
        CloudCliProvider::Gcp => {
            let exe = vsn_system::find_executable("gcloud")
                .map_err(|e| CloudError::Invalid(format!("gcloud CLI unavailable: {e}")))?;
            let project = request.scope.as_ref().ok_or_else(|| {
                CloudError::Invalid("GCP create requires scope=project id".into())
            })?;
            let mut args = vec![
                "compute".into(),
                "instances".into(),
                "create".into(),
                request.name.clone(),
                "--project".into(),
                project.clone(),
                "--zone".into(),
                request.location.clone(),
                "--machine-type".into(),
                request.machine_type.clone(),
                "--image".into(),
                request.image.clone(),
                "--no-address".into(),
                "--quiet".into(),
                "--format=json".into(),
            ];
            if let Some(network) = &request.network {
                args.push(format!("--network={network}"));
            }
            if let Some(subnet) = &request.subnet {
                args.push(format!("--subnet={subnet}"));
            }
            let value = run_cli_json(&exe, &args, 180_000)?;
            let id = value
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.get("id"))
                .map(|v| v.to_string())
                .or_else(|| Some(request.name.clone()));
            Ok(CloudCliResult {
                provider: request.provider.clone(),
                operation: "create".into(),
                instance_id: id,
                output: value,
            })
        }
    }
}

pub fn cloud_cli_status(instance: &CloudCliInstanceRef) -> Result<CloudCliResult, CloudError> {
    validate_cloud_cli_ref(instance)?;
    match instance.provider {
        CloudCliProvider::Aws => {
            let exe = vsn_system::find_executable("aws")
                .map_err(|e| CloudError::Invalid(format!("AWS CLI unavailable: {e}")))?;
            let args=vec!["ec2".into(),"describe-instances".into(),"--region".into(),instance.location.clone(),"--instance-ids".into(),instance.instance_id.clone(),"--query".into(),"Reservations[0].Instances[0].{id:InstanceId,state:State.Name,private_ip:PrivateIpAddress,public_ip:PublicIpAddress}".into(),"--output".into(),"json".into()];
            let value = run_cli_json(&exe, &args, 60_000)?;
            Ok(CloudCliResult {
                provider: instance.provider.clone(),
                operation: "status".into(),
                instance_id: Some(instance.instance_id.clone()),
                output: value,
            })
        }
        CloudCliProvider::Azure => {
            let group = instance.scope.as_ref().ok_or_else(|| {
                CloudError::Invalid("Azure status requires scope=resource group".into())
            })?;
            let exe = vsn_system::find_executable("az")
                .map_err(|e| CloudError::Invalid(format!("Azure CLI unavailable: {e}")))?;
            let args = vec![
                "vm".into(),
                "show".into(),
                "--resource-group".into(),
                group.clone(),
                "--name".into(),
                instance.instance_id.clone(),
                "--show-details".into(),
                "--output".into(),
                "json".into(),
            ];
            let value = run_cli_json(&exe, &args, 60_000)?;
            Ok(CloudCliResult {
                provider: instance.provider.clone(),
                operation: "status".into(),
                instance_id: Some(instance.instance_id.clone()),
                output: value,
            })
        }
        CloudCliProvider::Gcp => {
            let project = instance.scope.as_ref().ok_or_else(|| {
                CloudError::Invalid("GCP status requires scope=project id".into())
            })?;
            let exe = vsn_system::find_executable("gcloud")
                .map_err(|e| CloudError::Invalid(format!("gcloud CLI unavailable: {e}")))?;
            let args = vec![
                "compute".into(),
                "instances".into(),
                "describe".into(),
                instance.instance_id.clone(),
                "--project".into(),
                project.clone(),
                "--zone".into(),
                instance.location.clone(),
                "--format=json".into(),
            ];
            let value = run_cli_json(&exe, &args, 60_000)?;
            Ok(CloudCliResult {
                provider: instance.provider.clone(),
                operation: "status".into(),
                instance_id: Some(instance.instance_id.clone()),
                output: value,
            })
        }
    }
}

pub fn cloud_cli_start(instance: &CloudCliInstanceRef) -> Result<CloudCliResult, CloudError> {
    validate_cloud_cli_ref(instance)?;
    match instance.provider {
        CloudCliProvider::Aws => {
            let exe = vsn_system::find_executable("aws")
                .map_err(|e| CloudError::Invalid(format!("AWS CLI unavailable: {e}")))?;
            let args = vec![
                "ec2".into(),
                "start-instances".into(),
                "--region".into(),
                instance.location.clone(),
                "--instance-ids".into(),
                instance.instance_id.clone(),
                "--output".into(),
                "json".into(),
            ];
            let value = run_cli_json(&exe, &args, 120_000)?;
            Ok(CloudCliResult {
                provider: instance.provider.clone(),
                operation: "start".into(),
                instance_id: Some(instance.instance_id.clone()),
                output: value,
            })
        }
        CloudCliProvider::Azure => {
            let group = instance.scope.as_ref().ok_or_else(|| {
                CloudError::Invalid("Azure start requires scope=resource group".into())
            })?;
            let exe = vsn_system::find_executable("az")
                .map_err(|e| CloudError::Invalid(format!("Azure CLI unavailable: {e}")))?;
            let args = vec![
                "vm".into(),
                "start".into(),
                "--resource-group".into(),
                group.clone(),
                "--name".into(),
                instance.instance_id.clone(),
                "--output".into(),
                "json".into(),
            ];
            let value = run_cli_json_allow_empty(&exe, &args, 180_000)?;
            Ok(CloudCliResult {
                provider: instance.provider.clone(),
                operation: "start".into(),
                instance_id: Some(instance.instance_id.clone()),
                output: value,
            })
        }
        CloudCliProvider::Gcp => {
            let project = instance
                .scope
                .as_ref()
                .ok_or_else(|| CloudError::Invalid("GCP start requires scope=project id".into()))?;
            let exe = vsn_system::find_executable("gcloud")
                .map_err(|e| CloudError::Invalid(format!("gcloud CLI unavailable: {e}")))?;
            let args = vec![
                "compute".into(),
                "instances".into(),
                "start".into(),
                instance.instance_id.clone(),
                "--project".into(),
                project.clone(),
                "--zone".into(),
                instance.location.clone(),
                "--quiet".into(),
                "--format=json".into(),
            ];
            let value = run_cli_json_allow_empty(&exe, &args, 180_000)?;
            Ok(CloudCliResult {
                provider: instance.provider.clone(),
                operation: "start".into(),
                instance_id: Some(instance.instance_id.clone()),
                output: value,
            })
        }
    }
}
pub fn cloud_cli_stop(instance: &CloudCliInstanceRef) -> Result<CloudCliResult, CloudError> {
    validate_cloud_cli_ref(instance)?;
    match instance.provider {
        CloudCliProvider::Aws => {
            let exe = vsn_system::find_executable("aws")
                .map_err(|e| CloudError::Invalid(format!("AWS CLI unavailable: {e}")))?;
            let args = vec![
                "ec2".into(),
                "stop-instances".into(),
                "--region".into(),
                instance.location.clone(),
                "--instance-ids".into(),
                instance.instance_id.clone(),
                "--output".into(),
                "json".into(),
            ];
            let value = run_cli_json(&exe, &args, 120_000)?;
            Ok(CloudCliResult {
                provider: instance.provider.clone(),
                operation: "stop".into(),
                instance_id: Some(instance.instance_id.clone()),
                output: value,
            })
        }
        CloudCliProvider::Azure => {
            let group = instance.scope.as_ref().ok_or_else(|| {
                CloudError::Invalid("Azure stop requires scope=resource group".into())
            })?;
            let exe = vsn_system::find_executable("az")
                .map_err(|e| CloudError::Invalid(format!("Azure CLI unavailable: {e}")))?;
            let args = vec![
                "vm".into(),
                "deallocate".into(),
                "--resource-group".into(),
                group.clone(),
                "--name".into(),
                instance.instance_id.clone(),
                "--output".into(),
                "json".into(),
            ];
            let value = run_cli_json_allow_empty(&exe, &args, 180_000)?;
            Ok(CloudCliResult {
                provider: instance.provider.clone(),
                operation: "stop".into(),
                instance_id: Some(instance.instance_id.clone()),
                output: value,
            })
        }
        CloudCliProvider::Gcp => {
            let project = instance
                .scope
                .as_ref()
                .ok_or_else(|| CloudError::Invalid("GCP stop requires scope=project id".into()))?;
            let exe = vsn_system::find_executable("gcloud")
                .map_err(|e| CloudError::Invalid(format!("gcloud CLI unavailable: {e}")))?;
            let args = vec![
                "compute".into(),
                "instances".into(),
                "stop".into(),
                instance.instance_id.clone(),
                "--project".into(),
                project.clone(),
                "--zone".into(),
                instance.location.clone(),
                "--quiet".into(),
                "--format=json".into(),
            ];
            let value = run_cli_json_allow_empty(&exe, &args, 180_000)?;
            Ok(CloudCliResult {
                provider: instance.provider.clone(),
                operation: "stop".into(),
                instance_id: Some(instance.instance_id.clone()),
                output: value,
            })
        }
    }
}

pub fn cloud_cli_snapshot(request: &CloudCliSnapshotRequest) -> Result<CloudCliResult, CloudError> {
    if !request.acknowledge_crash_consistency {
        return Err(CloudError::Invalid("snapshot requires acknowledge_crash_consistency=true; VSN does not claim application-consistent quiescing".into()));
    }
    validate_cloud_cli_ref(&request.instance)?;
    validate_cloud_resource_name(&request.snapshot_name)?;
    match request.instance.provider {
        CloudCliProvider::Aws => {
            let exe = vsn_system::find_executable("aws")
                .map_err(|e| CloudError::Invalid(format!("AWS CLI unavailable: {e}")))?;
            let args = vec![
                "ec2".into(),
                "create-image".into(),
                "--region".into(),
                request.instance.location.clone(),
                "--instance-id".into(),
                request.instance.instance_id.clone(),
                "--name".into(),
                request.snapshot_name.clone(),
                "--no-reboot".into(),
                "--query".into(),
                "{artifact_id:ImageId}".into(),
                "--output".into(),
                "json".into(),
            ];
            let value = run_cli_json(&exe, &args, 180_000)?;
            let id = value
                .get("artifact_id")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            if id.is_none() {
                return Err(CloudError::Invalid(
                    "AWS create-image returned no AMI id".into(),
                ));
            }
            Ok(CloudCliResult {
                provider: request.instance.provider.clone(),
                operation: "snapshot".into(),
                instance_id: id,
                output: value,
            })
        }
        CloudCliProvider::Azure => {
            let group = request.instance.scope.as_ref().ok_or_else(|| {
                CloudError::Invalid("Azure snapshot requires scope=resource group".into())
            })?;
            let exe = vsn_system::find_executable("az")
                .map_err(|e| CloudError::Invalid(format!("Azure CLI unavailable: {e}")))?;
            let show_args = vec![
                "vm".into(),
                "show".into(),
                "--resource-group".into(),
                group.clone(),
                "--name".into(),
                request.instance.instance_id.clone(),
                "--query".into(),
                "storageProfile.osDisk.managedDisk.id".into(),
                "--output".into(),
                "tsv".into(),
            ];
            let source = String::from_utf8(run_cli_bounded(&exe, &show_args, 60_000)?)
                .map_err(|_| CloudError::Invalid("Azure OS disk id is not UTF-8".into()))?
                .trim()
                .to_string();
            validate_provider_arg("azure_os_disk_id", &source, 1024)?;
            let args = vec![
                "snapshot".into(),
                "create".into(),
                "--resource-group".into(),
                group.clone(),
                "--name".into(),
                request.snapshot_name.clone(),
                "--source".into(),
                source,
                "--output".into(),
                "json".into(),
            ];
            let value = run_cli_json(&exe, &args, 180_000)?;
            let id = value
                .get("id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .or_else(|| Some(request.snapshot_name.clone()));
            Ok(CloudCliResult {
                provider: request.instance.provider.clone(),
                operation: "snapshot".into(),
                instance_id: id,
                output: value,
            })
        }
        CloudCliProvider::Gcp => {
            let project = request.instance.scope.as_ref().ok_or_else(|| {
                CloudError::Invalid("GCP snapshot requires scope=project id".into())
            })?;
            let exe = vsn_system::find_executable("gcloud")
                .map_err(|e| CloudError::Invalid(format!("gcloud CLI unavailable: {e}")))?;
            let args = vec![
                "compute".into(),
                "machine-images".into(),
                "create".into(),
                request.snapshot_name.clone(),
                "--project".into(),
                project.clone(),
                "--source-instance".into(),
                request.instance.instance_id.clone(),
                "--source-instance-zone".into(),
                request.instance.location.clone(),
                "--quiet".into(),
                "--format=json".into(),
            ];
            let value = run_cli_json_allow_empty(&exe, &args, 300_000)?;
            Ok(CloudCliResult {
                provider: request.instance.provider.clone(),
                operation: "snapshot".into(),
                instance_id: Some(request.snapshot_name.clone()),
                output: value,
            })
        }
    }
}

pub fn cloud_cli_clone(request: &CloudCliCloneRequest) -> Result<CloudCliResult, CloudError> {
    if !request.confirm_new_instance {
        return Err(CloudError::Invalid(
            "clone requires confirm_new_instance=true".into(),
        ));
    }
    validate_cloud_cli_ref(&request.source)?;
    validate_cloud_resource_name(&request.target_name)?;
    validate_provider_arg("snapshot_or_image_id", &request.snapshot_or_image_id, 1024)?;
    match request.source.provider {
        CloudCliProvider::Aws => {
            let exe = vsn_system::find_executable("aws")
                .map_err(|e| CloudError::Invalid(format!("AWS CLI unavailable: {e}")))?;
            let target_location = request
                .target_location
                .as_deref()
                .unwrap_or(&request.source.location);
            validate_provider_arg("target_location", target_location, 160)?;
            let mut args=vec!["ec2".into(),"run-instances".into(),"--region".into(),target_location.to_string(),"--image-id".into(),request.snapshot_or_image_id.clone(),"--count".into(),"1".into(),"--no-associate-public-ip-address".into(),"--tag-specifications".into(),format!("ResourceType=instance,Tags=[{{Key=Name,Value={}}},{{Key=vsn-managed,Value=true}}]",request.target_name),"--query".into(),"Instances[0].{id:InstanceId,state:State.Name,private_ip:PrivateIpAddress}".into(),"--output".into(),"json".into()];
            if let Some(subnet) = &request.subnet {
                validate_provider_arg("subnet", subnet, 512)?;
                args.extend(["--subnet-id".into(), subnet.clone()]);
            }
            let value = run_cli_json(&exe, &args, 180_000)?;
            let id = value.get("id").and_then(|v| v.as_str()).map(str::to_string);
            if id.is_none() {
                return Err(CloudError::Invalid(
                    "AWS clone returned no instance id".into(),
                ));
            }
            Ok(CloudCliResult {
                provider: request.source.provider.clone(),
                operation: "clone".into(),
                instance_id: id,
                output: value,
            })
        }
        CloudCliProvider::Gcp => {
            let project =
                request.source.scope.as_ref().ok_or_else(|| {
                    CloudError::Invalid("GCP clone requires scope=project id".into())
                })?;
            let exe = vsn_system::find_executable("gcloud")
                .map_err(|e| CloudError::Invalid(format!("gcloud CLI unavailable: {e}")))?;
            let target_location = request
                .target_location
                .as_deref()
                .unwrap_or(&request.source.location);
            validate_provider_arg("target_location", target_location, 160)?;
            let mut args = vec![
                "compute".into(),
                "instances".into(),
                "create".into(),
                request.target_name.clone(),
                "--project".into(),
                project.clone(),
                "--zone".into(),
                target_location.to_string(),
                "--source-machine-image".into(),
                request.snapshot_or_image_id.clone(),
                "--no-address".into(),
                "--quiet".into(),
                "--format=json".into(),
            ];
            if let Some(network) = &request.network {
                validate_provider_arg("network", network, 512)?;
                args.push(format!("--network={network}"));
            }
            if let Some(subnet) = &request.subnet {
                validate_provider_arg("subnet", subnet, 512)?;
                args.push(format!("--subnet={subnet}"));
            }
            let value = run_cli_json(&exe, &args, 300_000)?;
            let id = value
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.get("id"))
                .map(|v| v.to_string())
                .or_else(|| Some(request.target_name.clone()));
            Ok(CloudCliResult {
                provider: request.source.provider.clone(),
                operation: "clone".into(),
                instance_id: id,
                output: value,
            })
        }
        CloudCliProvider::Azure => {
            let group = request.source.scope.as_ref().ok_or_else(|| {
                CloudError::Invalid("Azure clone requires scope=resource group".into())
            })?;
            let target_location = request
                .target_location
                .as_deref()
                .unwrap_or(&request.source.location);
            validate_provider_arg("target_location", target_location, 160)?;
            if target_location != request.source.location {
                return Err(CloudError::Unsupported("Azure cross-region clone requires an explicit snapshot-copy step before VM creation".into()));
            }
            let machine_type = request
                .machine_type
                .as_deref()
                .ok_or_else(|| CloudError::Invalid("Azure clone requires machine_type".into()))?;
            validate_provider_arg("machine_type", machine_type, 160)?;
            let os_type = request
                .os_type
                .as_deref()
                .ok_or_else(|| {
                    CloudError::Invalid("Azure clone requires os_type=linux|windows".into())
                })?
                .to_ascii_lowercase();
            if !matches!(os_type.as_str(), "linux" | "windows") {
                return Err(CloudError::Invalid(
                    "Azure clone os_type must be linux or windows".into(),
                ));
            }
            if request.network.is_some() != request.subnet.is_some() {
                return Err(CloudError::Invalid(
                    "Azure clone private network selection requires both network and subnet".into(),
                ));
            }
            let exe = vsn_system::find_executable("az")
                .map_err(|e| CloudError::Invalid(format!("Azure CLI unavailable: {e}")))?;
            let disk_name = format!("{}-osdisk", request.target_name);
            validate_provider_arg("azure_disk_name", &disk_name, 80)?;
            let disk_args = vec![
                "disk".into(),
                "create".into(),
                "--resource-group".into(),
                group.clone(),
                "--name".into(),
                disk_name.clone(),
                "--source".into(),
                request.snapshot_or_image_id.clone(),
                "--location".into(),
                target_location.to_string(),
                "--output".into(),
                "json".into(),
            ];
            let disk = run_cli_json(&exe, &disk_args, 240_000)?;
            let disk_id = disk
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CloudError::Invalid("Azure disk create returned no disk id".into()))?
                .to_string();
            validate_provider_arg("azure_disk_id", &disk_id, 2048)?;
            let mut vm_args = vec![
                "vm".into(),
                "create".into(),
                "--resource-group".into(),
                group.clone(),
                "--name".into(),
                request.target_name.clone(),
                "--location".into(),
                target_location.to_string(),
                "--attach-os-disk".into(),
                disk_id.clone(),
                "--os-type".into(),
                os_type.clone(),
                "--size".into(),
                machine_type.to_string(),
                "--public-ip-address".into(),
                "".into(),
                "--output".into(),
                "json".into(),
            ];
            if let (Some(network), Some(subnet)) = (&request.network, &request.subnet) {
                validate_provider_arg("network", network, 512)?;
                validate_provider_arg("subnet", subnet, 512)?;
                vm_args.extend([
                    "--vnet-name".into(),
                    network.clone(),
                    "--subnet".into(),
                    subnet.clone(),
                ]);
            }
            match run_cli_json(&exe, &vm_args, 300_000) {
                Ok(value) => {
                    let id = value
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                        .or_else(|| Some(request.target_name.clone()));
                    Ok(CloudCliResult {
                        provider: request.source.provider.clone(),
                        operation: "clone".into(),
                        instance_id: id,
                        output: serde_json::json!({"vm":value,"os_disk_id":disk_id}),
                    })
                }
                Err(error) => {
                    let cleanup = vec![
                        "disk".into(),
                        "delete".into(),
                        "--resource-group".into(),
                        group.clone(),
                        "--name".into(),
                        disk_name,
                        "--yes".into(),
                        "--output".into(),
                        "json".into(),
                    ];
                    let _ = run_cli_json_allow_empty(&exe, &cleanup, 120_000);
                    Err(error)
                }
            }
        }
    }
}

pub fn cloud_cli_copy_image(
    request: &CloudCliImageCopyRequest,
) -> Result<CloudCliResult, CloudError> {
    if !request.confirm_copy {
        return Err(CloudError::Invalid(
            "image copy requires confirm_copy=true".into(),
        ));
    }
    validate_cloud_resource_name(&request.target_name)?;
    validate_provider_arg("source_artifact_id", &request.source_artifact_id, 2048)?;
    validate_provider_arg("source_location", &request.source_location, 160)?;
    validate_provider_arg("target_location", &request.target_location, 160)?;
    for (name, value) in [
        ("source_scope", request.source_scope.as_deref()),
        ("target_scope", request.target_scope.as_deref()),
        ("os_type", request.os_type.as_deref()),
        ("sku", request.sku.as_deref()),
    ] {
        if let Some(value) = value {
            validate_provider_arg(name, value, 512)?;
        }
    }
    match request.provider {
        CloudCliProvider::Aws => {
            if request.source_location == request.target_location {
                return Err(CloudError::Invalid(
                    "AWS image copy target region must differ from source region".into(),
                ));
            }
            let exe = vsn_system::find_executable("aws")
                .map_err(|e| CloudError::Invalid(format!("AWS CLI unavailable: {e}")))?;
            let args = vec![
                "ec2".into(),
                "copy-image".into(),
                "--region".into(),
                request.target_location.clone(),
                "--source-region".into(),
                request.source_location.clone(),
                "--source-image-id".into(),
                request.source_artifact_id.clone(),
                "--name".into(),
                request.target_name.clone(),
                "--query".into(),
                "{artifact_id:ImageId}".into(),
                "--output".into(),
                "json".into(),
            ];
            let value = run_cli_json(&exe, &args, 180_000)?;
            let id = value
                .get("artifact_id")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            if id.is_none() {
                return Err(CloudError::Invalid(
                    "AWS copy-image returned no AMI id".into(),
                ));
            }
            Ok(CloudCliResult {
                provider: request.provider.clone(),
                operation: "copy_image".into(),
                instance_id: id,
                output: value,
            })
        }
        CloudCliProvider::Gcp => Ok(CloudCliResult {
            provider: request.provider.clone(),
            operation: "copy_image".into(),
            instance_id: Some(request.source_artifact_id.clone()),
            output: serde_json::json!({"artifact_id":request.source_artifact_id,"copy_mode":"global_machine_image","target_location":request.target_location,"note":"GCP machine images are consumed directly when cloning into the target zone; no redundant regional copy is required."}),
        }),
        CloudCliProvider::Azure => azure_copy_artifact(request),
    }
}

pub fn cloud_cli_copy_status(
    reference: &CloudCliArtifactRef,
) -> Result<CloudCliResult, CloudError> {
    validate_provider_arg("artifact_id", &reference.artifact_id, 2048)?;
    validate_provider_arg("location", &reference.location, 160)?;
    if let Some(scope) = &reference.scope {
        validate_provider_arg("scope", scope, 512)?;
    }
    match reference.provider {
        CloudCliProvider::Aws => {
            let exe = vsn_system::find_executable("aws")
                .map_err(|e| CloudError::Invalid(format!("AWS CLI unavailable: {e}")))?;
            let args = vec![
                "ec2".into(),
                "describe-images".into(),
                "--region".into(),
                reference.location.clone(),
                "--image-ids".into(),
                reference.artifact_id.clone(),
                "--query".into(),
                "Images[0].{artifact_id:ImageId,state:State,state_reason:StateReason.Message}"
                    .into(),
                "--output".into(),
                "json".into(),
            ];
            let value = run_cli_json(&exe, &args, 60_000)?;
            Ok(CloudCliResult {
                provider: reference.provider.clone(),
                operation: "copy_status".into(),
                instance_id: Some(reference.artifact_id.clone()),
                output: value,
            })
        }
        CloudCliProvider::Gcp => {
            let project = reference.scope.as_ref().ok_or_else(|| {
                CloudError::Invalid("GCP artifact status requires scope=project id".into())
            })?;
            let exe = vsn_system::find_executable("gcloud")
                .map_err(|e| CloudError::Invalid(format!("gcloud CLI unavailable: {e}")))?;
            let args = vec![
                "compute".into(),
                "machine-images".into(),
                "describe".into(),
                reference.artifact_id.clone(),
                "--project".into(),
                project.clone(),
                "--format=json".into(),
            ];
            let value = run_cli_json(&exe, &args, 60_000)?;
            Ok(CloudCliResult {
                provider: reference.provider.clone(),
                operation: "copy_status".into(),
                instance_id: Some(reference.artifact_id.clone()),
                output: value,
            })
        }
        CloudCliProvider::Azure => {
            let exe = vsn_system::find_executable("az")
                .map_err(|e| CloudError::Invalid(format!("Azure CLI unavailable: {e}")))?;
            let kind = reference
                .artifact_kind
                .clone()
                .or_else(|| azure_resource_kind(&reference.artifact_id))
                .unwrap_or(CloudArtifactKind::Snapshot);
            let command = match kind {
                CloudArtifactKind::ManagedDisk => "disk",
                _ => "snapshot",
            };
            let args=vec![command.into(),"show".into(),"--ids".into(),reference.artifact_id.clone(),"--query".into(),"{id:id,location:location,provisioning_state:provisioningState,completion_percent:completionPercent,disk_state:diskState}".into(),"--output".into(),"json".into()];
            let value = run_cli_json(&exe, &args, 60_000)?;
            Ok(CloudCliResult {
                provider: reference.provider.clone(),
                operation: "copy_status".into(),
                instance_id: Some(reference.artifact_id.clone()),
                output: value,
            })
        }
    }
}

fn azure_copy_artifact(request: &CloudCliImageCopyRequest) -> Result<CloudCliResult, CloudError> {
    let az = vsn_system::find_executable("az")
        .map_err(|e| CloudError::Invalid(format!("Azure CLI unavailable: {e}")))?;
    let source = azure_parse_resource_id(&request.source_artifact_id)?;
    let target_group = request
        .target_scope
        .clone()
        .or_else(|| request.source_scope.clone())
        .unwrap_or_else(|| source.resource_group.clone());
    validate_provider_arg("target_scope", &target_group, 256)?;
    let kind = request
        .artifact_kind
        .clone()
        .or_else(|| azure_resource_kind(&request.source_artifact_id))
        .ok_or_else(|| {
            CloudError::Invalid(
                "Azure artifact copy requires artifact_kind or a full snapshot/disk resource id"
                    .into(),
            )
        })?;
    match kind {
        CloudArtifactKind::IncrementalSnapshot => {
            azure_copy_incremental_snapshot(&az, request, &target_group)
        }
        CloudArtifactKind::Snapshot => {
            let show = azure_show_snapshot(&az, &request.source_artifact_id)?;
            if show
                .get("incremental")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                azure_copy_incremental_snapshot(&az, request, &target_group)
            } else {
                azure_copy_with_azcopy(&az, request, &target_group, "snapshot", &show)
            }
        }
        CloudArtifactKind::ManagedDisk => {
            let show = azure_show_disk(&az, &request.source_artifact_id)?;
            azure_copy_with_azcopy(&az, request, &target_group, "disk", &show)
        }
        other => Err(CloudError::Invalid(format!(
            "Azure copy does not accept artifact kind {other:?}"
        ))),
    }
}

fn azure_copy_incremental_snapshot(
    az: &std::path::Path,
    request: &CloudCliImageCopyRequest,
    target_group: &str,
) -> Result<CloudCliResult, CloudError> {
    let args = vec![
        "snapshot".into(),
        "create".into(),
        "--resource-group".into(),
        target_group.into(),
        "--name".into(),
        request.target_name.clone(),
        "--location".into(),
        request.target_location.clone(),
        "--source".into(),
        request.source_artifact_id.clone(),
        "--incremental".into(),
        "--copy-start".into(),
        "--output".into(),
        "json".into(),
    ];
    let value = run_cli_json(az, &args, 180_000)?;
    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| Some(request.target_name.clone()));
    Ok(CloudCliResult {
        provider: CloudCliProvider::Azure,
        operation: "copy_image".into(),
        instance_id: id,
        output: serde_json::json!({"copy_mode":"azure_incremental_snapshot_copy_start","artifact":value}),
    })
}

fn azure_copy_with_azcopy(
    az: &std::path::Path,
    request: &CloudCliImageCopyRequest,
    target_group: &str,
    source_command: &str,
    source_meta: &serde_json::Value,
) -> Result<CloudCliResult, CloudError> {
    let size = source_meta
        .get("size")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            CloudError::Invalid("Azure source artifact returned no diskSizeBytes".into())
        })?;
    let upload_size = size
        .checked_add(512)
        .ok_or_else(|| CloudError::Invalid("Azure disk size overflow".into()))?;
    let os_type=request.os_type.clone().or_else(||source_meta.get("os").and_then(|v|v.as_str()).map(str::to_string)).ok_or_else(||CloudError::Invalid("Azure direct artifact copy requires os_type=linux|windows when source metadata does not expose it".into()))?;
    let os_type = normalize_azure_os_type(&os_type)?;
    let sku = request.sku.clone().unwrap_or_else(|| "Standard_LRS".into());
    validate_azure_disk_sku(&sku)?;
    let target_args = vec![
        "disk".into(),
        "create".into(),
        "--resource-group".into(),
        target_group.into(),
        "--name".into(),
        request.target_name.clone(),
        "--location".into(),
        request.target_location.clone(),
        "--os-type".into(),
        os_type,
        "--for-upload".into(),
        "--upload-size-bytes".into(),
        upload_size.to_string(),
        "--sku".into(),
        sku,
        "--output".into(),
        "json".into(),
    ];
    let target = run_cli_json(az, &target_args, 180_000)?;
    let target_id = target
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            CloudError::Invalid("Azure target managed disk create returned no id".into())
        })?
        .to_string();
    let target_sas = azure_grant_sas(az, "disk", &target_id, true)?;
    let source_sas = match azure_grant_sas(az, source_command, &request.source_artifact_id, false) {
        Ok(v) => v,
        Err(e) => {
            let _ = azure_revoke_sas(az, "disk", &target_id);
            let _ = azure_delete_disk(az, target_group, &request.target_name);
            return Err(e);
        }
    };
    let azcopy = vsn_system::find_executable("azcopy").map_err(|e| {
        CloudError::Invalid(format!(
            "AzCopy v10 unavailable for Azure cross-region direct copy: {e}"
        ))
    })?;
    let copy_args = vec![
        "copy".into(),
        source_sas.clone(),
        target_sas.clone(),
        "--blob-type".into(),
        "PageBlob".into(),
        "--check-length=true".into(),
        "--output-type=json".into(),
    ];
    let copied = run_cli_bounded(&azcopy, &copy_args, 300_000);
    let source_revoke = azure_revoke_sas(az, source_command, &request.source_artifact_id);
    let target_revoke = azure_revoke_sas(az, "disk", &target_id);
    match copied {
        Ok(output) => {
            if source_revoke.is_err() || target_revoke.is_err() {
                return Err(CloudError::Invalid("Azure copy completed but one or more managed-disk SAS grants could not be revoked; operator review is required".into()));
            }
            let parsed = if output.is_empty() {
                serde_json::json!({"ok":true})
            } else {
                serde_json::from_slice(&output)
                    .unwrap_or_else(|_| serde_json::json!({"azcopy_output":"completed"}))
            };
            Ok(CloudCliResult {
                provider: CloudCliProvider::Azure,
                operation: "copy_image".into(),
                instance_id: Some(target_id.clone()),
                output: serde_json::json!({"copy_mode":"azure_direct_managed_disk","artifact_id":target_id,"target_disk":target,"azcopy":parsed}),
            })
        }
        Err(e) => {
            let _ = azure_delete_disk(az, target_group, &request.target_name);
            Err(e)
        }
    }
}

#[derive(Debug, Clone)]
struct AzureResourceRef {
    resource_group: String,
    resource_type: String,
    name: String,
}
fn azure_parse_resource_id(id: &str) -> Result<AzureResourceRef, CloudError> {
    let parts = id.split('/').filter(|v| !v.is_empty()).collect::<Vec<_>>();
    let rg_index = parts
        .iter()
        .position(|v| v.eq_ignore_ascii_case("resourceGroups"))
        .ok_or_else(|| {
            CloudError::Invalid("Azure artifact id must include resourceGroups/<group>".into())
        })?;
    let provider_index = parts
        .iter()
        .position(|v| v.eq_ignore_ascii_case("providers"))
        .ok_or_else(|| {
            CloudError::Invalid(
                "Azure artifact id must include providers/Microsoft.Compute/<type>/<name>".into(),
            )
        })?;
    if rg_index + 1 >= parts.len()
        || provider_index + 3 >= parts.len()
        || !parts[provider_index + 1].eq_ignore_ascii_case("Microsoft.Compute")
    {
        return Err(CloudError::Invalid(
            "Azure artifact resource id is malformed".into(),
        ));
    }
    let r = AzureResourceRef {
        resource_group: parts[rg_index + 1].into(),
        resource_type: parts[provider_index + 2].into(),
        name: parts[provider_index + 3].into(),
    };
    validate_provider_arg("azure_resource_group", &r.resource_group, 256)?;
    validate_provider_arg("azure_resource_name", &r.name, 256)?;
    Ok(r)
}
fn azure_resource_kind(id: &str) -> Option<CloudArtifactKind> {
    azure_parse_resource_id(id).ok().and_then(|r| {
        match r.resource_type.to_ascii_lowercase().as_str() {
            "snapshots" => Some(CloudArtifactKind::Snapshot),
            "disks" => Some(CloudArtifactKind::ManagedDisk),
            _ => None,
        }
    })
}
fn azure_show_snapshot(az: &std::path::Path, id: &str) -> Result<serde_json::Value, CloudError> {
    let args = vec![
        "snapshot".into(),
        "show".into(),
        "--ids".into(),
        id.into(),
        "--query".into(),
        "{size:diskSizeBytes,os:osType,incremental:incremental,id:id,location:location}".into(),
        "--output".into(),
        "json".into(),
    ];
    run_cli_json(az, &args, 60_000)
}
fn azure_show_disk(az: &std::path::Path, id: &str) -> Result<serde_json::Value, CloudError> {
    let args = vec![
        "disk".into(),
        "show".into(),
        "--ids".into(),
        id.into(),
        "--query".into(),
        "{size:diskSizeBytes,os:osType,id:id,location:location}".into(),
        "--output".into(),
        "json".into(),
    ];
    run_cli_json(az, &args, 60_000)
}
fn normalize_azure_os_type(value: &str) -> Result<String, CloudError> {
    match value.to_ascii_lowercase().as_str() {
        "linux" => Ok("Linux".into()),
        "windows" => Ok("Windows".into()),
        _ => Err(CloudError::Invalid(
            "Azure os_type must be linux or windows".into(),
        )),
    }
}
fn validate_azure_disk_sku(value: &str) -> Result<(), CloudError> {
    if matches!(
        value.to_ascii_lowercase().as_str(),
        "standard_lrs"
            | "premium_lrs"
            | "standardssd_lrs"
            | "standardssd_zrs"
            | "premium_zrs"
            | "premiumv2_lrs"
            | "ultrassd_lrs"
    ) {
        Ok(())
    } else {
        Err(CloudError::Invalid(
            "Azure target disk sku is unsupported for direct copy".into(),
        ))
    }
}
fn azure_grant_sas(
    az: &std::path::Path,
    kind: &str,
    id: &str,
    write: bool,
) -> Result<String, CloudError> {
    let mut args = vec![
        kind.into(),
        "grant-access".into(),
        "--ids".into(),
        id.into(),
        "--duration-in-seconds".into(),
        "3600".into(),
    ];
    if kind == "disk" {
        args.extend([
            "--access-level".into(),
            if write { "Write".into() } else { "Read".into() },
        ]);
    }
    args.extend([
        "--query".into(),
        "accessSas".into(),
        "--output".into(),
        "tsv".into(),
    ]);
    let out = run_cli_bounded(az, &args, 60_000)?;
    let sas = String::from_utf8(out)
        .map_err(|_| CloudError::Invalid("Azure SAS response is not UTF-8".into()))?
        .trim()
        .to_string();
    if !sas.starts_with("https://") || sas.len() > 8192 {
        return Err(CloudError::Invalid(
            "Azure CLI returned an invalid managed-disk SAS".into(),
        ));
    }
    Ok(sas)
}
fn azure_revoke_sas(az: &std::path::Path, kind: &str, id: &str) -> Result<(), CloudError> {
    let args = vec![
        kind.into(),
        "revoke-access".into(),
        "--ids".into(),
        id.into(),
        "--output".into(),
        "json".into(),
    ];
    run_cli_json_allow_empty(az, &args, 60_000).map(|_| ())
}
fn azure_delete_disk(az: &std::path::Path, group: &str, name: &str) -> Result<(), CloudError> {
    let args = vec![
        "disk".into(),
        "delete".into(),
        "--resource-group".into(),
        group.into(),
        "--name".into(),
        name.into(),
        "--yes".into(),
        "--output".into(),
        "json".into(),
    ];
    run_cli_json_allow_empty(az, &args, 120_000).map(|_| ())
}

pub fn cloud_cli_destroy(request: &CloudCliDestroyRequest) -> Result<CloudCliResult, CloudError> {
    if !request.confirm_destroy {
        return Err(CloudError::Invalid(
            "cloud destroy requires confirm_destroy=true".into(),
        ));
    }
    validate_cloud_cli_ref(&request.instance)?;
    match request.instance.provider {
        CloudCliProvider::Aws => {
            let exe = vsn_system::find_executable("aws")
                .map_err(|e| CloudError::Invalid(format!("AWS CLI unavailable: {e}")))?;
            let args = vec![
                "ec2".into(),
                "terminate-instances".into(),
                "--region".into(),
                request.instance.location.clone(),
                "--instance-ids".into(),
                request.instance.instance_id.clone(),
                "--output".into(),
                "json".into(),
            ];
            let value = run_cli_json(&exe, &args, 60_000)?;
            Ok(CloudCliResult {
                provider: request.instance.provider.clone(),
                operation: "destroy".into(),
                instance_id: Some(request.instance.instance_id.clone()),
                output: value,
            })
        }
        CloudCliProvider::Azure => {
            let group = request.instance.scope.as_ref().ok_or_else(|| {
                CloudError::Invalid("Azure destroy requires scope=resource group".into())
            })?;
            let exe = vsn_system::find_executable("az")
                .map_err(|e| CloudError::Invalid(format!("Azure CLI unavailable: {e}")))?;
            let args = vec![
                "vm".into(),
                "delete".into(),
                "--resource-group".into(),
                group.clone(),
                "--name".into(),
                request.instance.instance_id.clone(),
                "--yes".into(),
                "--output".into(),
                "json".into(),
            ];
            let value = run_cli_json_allow_empty(&exe, &args, 120_000)?;
            Ok(CloudCliResult {
                provider: request.instance.provider.clone(),
                operation: "destroy".into(),
                instance_id: Some(request.instance.instance_id.clone()),
                output: value,
            })
        }
        CloudCliProvider::Gcp => {
            let project = request.instance.scope.as_ref().ok_or_else(|| {
                CloudError::Invalid("GCP destroy requires scope=project id".into())
            })?;
            let exe = vsn_system::find_executable("gcloud")
                .map_err(|e| CloudError::Invalid(format!("gcloud CLI unavailable: {e}")))?;
            let args = vec![
                "compute".into(),
                "instances".into(),
                "delete".into(),
                request.instance.instance_id.clone(),
                "--project".into(),
                project.clone(),
                "--zone".into(),
                request.instance.location.clone(),
                "--quiet".into(),
                "--format=json".into(),
            ];
            let value = run_cli_json_allow_empty(&exe, &args, 120_000)?;
            Ok(CloudCliResult {
                provider: request.instance.provider.clone(),
                operation: "destroy".into(),
                instance_id: Some(request.instance.instance_id.clone()),
                output: value,
            })
        }
    }
}

fn validate_cloud_cli_create(r: &CloudCliCreateRequest) -> Result<(), CloudError> {
    validate_cloud_resource_name(&r.name)?;
    validate_provider_arg("location", &r.location, 160)?;
    validate_provider_arg("machine_type", &r.machine_type, 160)?;
    validate_provider_arg("image", &r.image, 512)?;
    for (name, v) in [
        ("scope", r.scope.as_deref()),
        ("subnet", r.subnet.as_deref()),
        ("network", r.network.as_deref()),
        ("admin_username", r.admin_username.as_deref()),
    ] {
        if let Some(v) = v {
            validate_provider_arg(name, v, 512)?;
        }
    }
    if let Some(path) = &r.ssh_public_key_file {
        let p = std::path::Path::new(path);
        if !p.is_absolute() || !p.is_file() {
            return Err(CloudError::Invalid(
                "ssh_public_key_file must be an existing absolute file".into(),
            ));
        }
        if std::fs::metadata(p).map(|m| m.len()).unwrap_or(u64::MAX) > 1024 * 1024 {
            return Err(CloudError::Invalid(
                "ssh public key file exceeds 1 MiB".into(),
            ));
        }
    }
    if matches!(r.provider, CloudCliProvider::Azure) && r.network.is_some() != r.subnet.is_some() {
        return Err(CloudError::Invalid(
            "Azure private network selection requires both network and subnet together".into(),
        ));
    }
    Ok(())
}
fn validate_cloud_cli_ref(r: &CloudCliInstanceRef) -> Result<(), CloudError> {
    validate_provider_arg("instance_id", &r.instance_id, 512)?;
    validate_provider_arg("location", &r.location, 160)?;
    if let Some(v) = &r.scope {
        validate_provider_arg("scope", v, 512)?;
    }
    Ok(())
}
fn validate_cloud_resource_name(value: &str) -> Result<(), CloudError> {
    if value.len() < 2
        || value.len() > 63
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        || !value.as_bytes()[0].is_ascii_alphanumeric()
    {
        return Err(CloudError::Invalid("cloud resource name must be a 2..63 character identifier using letters, digits, '-' or '_'".into()));
    }
    Ok(())
}
fn validate_provider_arg(name: &str, value: &str, max: usize) -> Result<(), CloudError> {
    if value.trim() != value
        || value.is_empty()
        || value.len() > max
        || value.starts_with('-')
        || value.chars().any(char::is_control)
    {
        return Err(CloudError::Invalid(format!("{name} is invalid")));
    }
    Ok(())
}
fn run_cli_json(
    exe: &std::path::Path,
    args: &[String],
    timeout_ms: u64,
) -> Result<serde_json::Value, CloudError> {
    let output = run_cli_bounded(exe, args, timeout_ms)?;
    if output.is_empty() {
        return Err(CloudError::Invalid(
            "cloud CLI returned an empty JSON response".into(),
        ));
    }
    serde_json::from_slice(&output)
        .map_err(|e| CloudError::Invalid(format!("cloud CLI returned invalid JSON: {e}")))
}
fn run_cli_json_allow_empty(
    exe: &std::path::Path,
    args: &[String],
    timeout_ms: u64,
) -> Result<serde_json::Value, CloudError> {
    let output = run_cli_bounded(exe, args, timeout_ms)?;
    if output.is_empty() {
        return Ok(serde_json::json!({"ok":true}));
    }
    serde_json::from_slice(&output)
        .map_err(|e| CloudError::Invalid(format!("cloud CLI returned invalid JSON: {e}")))
}
fn run_cli_bounded(
    exe: &std::path::Path,
    args: &[String],
    timeout_ms: u64,
) -> Result<Vec<u8>, CloudError> {
    use std::{
        io::Read,
        process::Stdio,
        time::{Duration, Instant},
    };
    fn drain<R: Read + Send + 'static>(
        reader: R,
        max: usize,
        label: &'static str,
    ) -> std::thread::JoinHandle<Result<Vec<u8>, String>> {
        std::thread::spawn(move || {
            let mut out = Vec::new();
            let mut limited = reader.take(max as u64 + 1);
            limited
                .read_to_end(&mut out)
                .map_err(|e| format!("cloud CLI {label} read failed: {e}"))?;
            if out.len() > max {
                return Err(format!("cloud CLI {label} exceeded safety limit"));
            }
            Ok(out)
        })
    }
    let mut child = std::process::Command::new(exe)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| CloudError::Invalid(format!("cloud CLI failed to start: {e}")))?;
    let stdout_reader = child
        .stdout
        .take()
        .ok_or_else(|| CloudError::Invalid("cloud CLI stdout pipe unavailable".into()))?;
    let stderr_reader = child
        .stderr
        .take()
        .ok_or_else(|| CloudError::Invalid("cloud CLI stderr pipe unavailable".into()))?;
    let stdout_handle = drain(stdout_reader, 2 * 1024 * 1024, "stdout");
    let stderr_handle = drain(stderr_reader, 256 * 1024, "stderr");
    let started = Instant::now();
    let timeout = Duration::from_millis(timeout_ms.clamp(1_000, 300_000));
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| CloudError::Invalid(format!("cloud CLI wait failed: {e}")))?
        {
            break status;
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            let _ = child.kill();
            break child.wait().map_err(|e| {
                CloudError::Invalid(format!("cloud CLI wait after timeout failed: {e}"))
            })?;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let stdout = stdout_handle
        .join()
        .map_err(|_| CloudError::Invalid("cloud CLI stdout reader panicked".into()))?
        .map_err(CloudError::Invalid)?;
    let stderr = stderr_handle
        .join()
        .map_err(|_| CloudError::Invalid("cloud CLI stderr reader panicked".into()))?
        .map_err(CloudError::Invalid)?;
    if timed_out {
        return Err(CloudError::Invalid("cloud CLI timed out".into()));
    }
    if !status.success() {
        return Err(CloudError::Invalid(format!(
            "cloud CLI rejected operation: {}",
            String::from_utf8_lossy(&stderr)
                .chars()
                .take(4096)
                .collect::<String>()
        )));
    }
    Ok(stdout)
}

#[cfg(test)]
mod cloud_cli_validation_tests {
    use super::*;

    #[test]
    fn resource_names_reject_flag_and_delimiter_shapes() {
        assert!(validate_cloud_resource_name("vsn-dev-01").is_ok());
        assert!(validate_cloud_resource_name("--region").is_err());
        assert!(validate_cloud_resource_name("x,Tags=[evil]").is_err());
        assert!(validate_cloud_resource_name("a").is_err());
    }

    #[test]
    fn provider_args_reject_leading_flags_controls_and_whitespace() {
        assert!(validate_provider_arg("location", "us-east-1", 160).is_ok());
        assert!(validate_provider_arg("location", "--output", 160).is_err());
        assert!(validate_provider_arg("location", " us-east-1", 160).is_err());
        assert!(validate_provider_arg("location", "us-east-1\n--output", 160).is_err());
    }

    #[test]
    fn destroy_is_explicitly_confirmed() {
        let request = CloudCliDestroyRequest {
            instance: CloudCliInstanceRef {
                provider: CloudCliProvider::Aws,
                instance_id: "i-1234567890abcdef0".into(),
                location: "us-east-1".into(),
                scope: None,
            },
            confirm_destroy: false,
        };
        let error =
            cloud_cli_destroy(&request).expect_err("destroy without confirmation must fail");
        assert!(error.to_string().contains("confirm_destroy"));
    }
}

#[cfg(test)]
mod cloud_snapshot_validation_tests {
    use super::*;
    #[test]
    fn snapshot_requires_explicit_consistency_ack() {
        let req = CloudCliSnapshotRequest {
            instance: CloudCliInstanceRef {
                provider: CloudCliProvider::Aws,
                instance_id: "i-1234567890abcdef0".into(),
                location: "us-east-1".into(),
                scope: None,
            },
            snapshot_name: "vsn-snap-01".into(),
            acknowledge_crash_consistency: false,
        };
        assert!(cloud_cli_snapshot(&req).is_err());
    }
    #[test]
    fn clone_requires_confirmation() {
        let req = CloudCliCloneRequest {
            source: CloudCliInstanceRef {
                provider: CloudCliProvider::Gcp,
                instance_id: "vm-01".into(),
                location: "us-central1-a".into(),
                scope: Some("project-1".into()),
            },
            snapshot_or_image_id: "image-01".into(),
            target_name: "clone-01".into(),
            target_location: None,
            subnet: None,
            network: None,
            machine_type: None,
            os_type: None,
            confirm_new_instance: false,
        };
        assert!(cloud_cli_clone(&req).is_err());
    }
}

#[cfg(test)]
mod cloud_copy_tests {
    use super::*;
    #[test]
    fn image_copy_requires_confirmation() {
        let r = CloudCliImageCopyRequest {
            provider: CloudCliProvider::Aws,
            source_artifact_id: "ami-1234567890abcdef0".into(),
            source_location: "us-east-1".into(),
            target_location: "us-west-2".into(),
            target_name: "vsn-copy-01".into(),
            confirm_copy: false,
            artifact_kind: None,
            source_scope: None,
            target_scope: None,
            os_type: None,
            sku: None,
        };
        assert!(cloud_cli_copy_image(&r).is_err());
    }
}

// ---------- 0.24 cloud lifecycle conformance ----------
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudProviderCapability {
    pub provider: CloudCliProvider,
    pub create: bool,
    pub status: bool,
    pub start: bool,
    pub stop: bool,
    pub destroy: bool,
    pub snapshot: bool,
    pub clone: bool,
    pub cross_location_artifact_copy: bool,
    pub private_by_default: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudConformanceReport {
    pub structured_argv: bool,
    pub credential_context_only: bool,
    pub explicit_destructive_confirmation: bool,
    pub ssh_release_lifecycle: bool,
    pub rollback: bool,
    pub health_check: bool,
    pub providers: Vec<CloudProviderCapability>,
    pub issues: Vec<String>,
}
pub fn cloud_conformance() -> CloudConformanceReport {
    CloudConformanceReport {
        structured_argv: true,
        credential_context_only: true,
        explicit_destructive_confirmation: true,
        ssh_release_lifecycle: true,
        rollback: true,
        health_check: true,
        providers: vec![
            CloudProviderCapability {
                provider: CloudCliProvider::Aws,
                create: true,
                status: true,
                start: true,
                stop: true,
                destroy: true,
                snapshot: true,
                clone: true,
                cross_location_artifact_copy: true,
                private_by_default: true,
            },
            CloudProviderCapability {
                provider: CloudCliProvider::Azure,
                create: true,
                status: true,
                start: true,
                stop: true,
                destroy: true,
                snapshot: true,
                clone: true,
                cross_location_artifact_copy: true,
                private_by_default: true,
            },
            CloudProviderCapability {
                provider: CloudCliProvider::Gcp,
                create: true,
                status: true,
                start: true,
                stop: true,
                destroy: true,
                snapshot: true,
                clone: true,
                cross_location_artifact_copy: true,
                private_by_default: true,
            },
        ],
        issues: vec![],
    }
}
