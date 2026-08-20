use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("invalid update manifest: {0}")]
    Invalid(String),
    #[error("signature verification failed")]
    Signature,
    #[error("artifact checksum mismatch")]
    Checksum,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateArtifact {
    pub os: String,
    pub arch: String,
    pub url: String,
    pub sha256: String,
    pub bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateManifest {
    pub version: u32,
    pub product: String,
    pub release: String,
    pub channel: String,
    pub published_at_unix_ms: u128,
    pub artifacts: Vec<UpdateArtifact>,
    pub signature: String,
}
#[derive(Serialize)]
struct UnsignedManifest<'a> {
    version: u32,
    product: &'a str,
    release: &'a str,
    channel: &'a str,
    published_at_unix_ms: u128,
    artifacts: &'a [UpdateArtifact],
}

pub fn verify_manifest(manifest: &UpdateManifest, public_key_b64: &str) -> Result<(), UpdateError> {
    if manifest.version != 1
        || manifest.product != "vsn-platform"
        || manifest.release.trim().is_empty()
        || manifest.artifacts.is_empty()
    {
        return Err(UpdateError::Invalid(
            "unsupported or incomplete manifest".into(),
        ));
    }
    for a in &manifest.artifacts {
        if !a.url.starts_with("https://")
            || a.sha256.len() != 64
            || !a.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(UpdateError::Invalid(
                "artifacts require HTTPS and SHA-256".into(),
            ));
        }
    }
    let key_bytes = B64
        .decode(public_key_b64)
        .map_err(|_| UpdateError::Signature)?;
    let key_arr: [u8; 32] = key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| UpdateError::Signature)?;
    let key = VerifyingKey::from_bytes(&key_arr).map_err(|_| UpdateError::Signature)?;
    let sig_bytes = B64
        .decode(&manifest.signature)
        .map_err(|_| UpdateError::Signature)?;
    let sig = Signature::from_slice(&sig_bytes).map_err(|_| UpdateError::Signature)?;
    let unsigned = UnsignedManifest {
        version: manifest.version,
        product: &manifest.product,
        release: &manifest.release,
        channel: &manifest.channel,
        published_at_unix_ms: manifest.published_at_unix_ms,
        artifacts: &manifest.artifacts,
    };
    let bytes = serde_json::to_vec(&unsigned)?;
    key.verify(&bytes, &sig).map_err(|_| UpdateError::Signature)
}

pub fn verify_artifact(path: &Path, expected_sha256: &str) -> Result<(), UpdateError> {
    let bytes = fs::read(path)?;
    let digest = Sha256::digest(bytes);
    let actual = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    if actual.eq_ignore_ascii_case(expected_sha256) {
        Ok(())
    } else {
        Err(UpdateError::Checksum)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplyFileRequest {
    pub install_root: std::path::PathBuf,
    pub target_relative: std::path::PathBuf,
    pub staged_artifact: std::path::PathBuf,
    pub expected_sha256: String,
    pub release: String,
    pub confirm_apply: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplyFileResult {
    pub target: std::path::PathBuf,
    pub release: String,
    pub backup_created: bool,
    pub rollback_available: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FileInstallState {
    target_relative: std::path::PathBuf,
    current_release: String,
    previous_release: Option<String>,
}

fn safe_release(value: &str) -> Result<(), UpdateError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+'))
    {
        Err(UpdateError::Invalid(
            "release must be a bounded safe identifier".into(),
        ))
    } else {
        Ok(())
    }
}
fn safe_relative(path: &Path) -> Result<(), UpdateError> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        Err(UpdateError::Invalid(
            "update target must be a safe relative path".into(),
        ))
    } else {
        Ok(())
    }
}
fn fsync_file(path: &Path) -> Result<(), UpdateError> {
    let f = fs::OpenOptions::new().read(true).open(path)?;
    f.sync_all()?;
    Ok(())
}
fn fsync_dir(path: &Path) -> Result<(), UpdateError> {
    #[cfg(unix)]
    {
        let f = fs::File::open(path)?;
        f.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}
fn update_state_path(root: &Path) -> std::path::PathBuf {
    root.join(".vsn-update").join("state.json")
}
fn backup_path(root: &Path, target_relative: &Path) -> std::path::PathBuf {
    let name = target_relative
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("target");
    root.join(".vsn-update")
        .join("previous")
        .join(format!("{name}.previous"))
}

/// Apply a pre-downloaded, checksum-pinned single-file update. This function is designed for
/// an out-of-process updater helper. It never downloads or executes the staged artifact.
pub fn apply_verified_file(request: &ApplyFileRequest) -> Result<ApplyFileResult, UpdateError> {
    if !request.confirm_apply {
        return Err(UpdateError::Invalid(
            "update apply requires confirm_apply=true".into(),
        ));
    }
    safe_release(&request.release)?;
    safe_relative(&request.target_relative)?;
    verify_artifact(&request.staged_artifact, &request.expected_sha256)?;
    let root = request
        .install_root
        .canonicalize()
        .map_err(|e| UpdateError::Invalid(format!("install root unavailable: {e}")))?;
    let target = root.join(&request.target_relative);
    let parent = target
        .parent()
        .ok_or_else(|| UpdateError::Invalid("update target has no parent".into()))?;
    let parent_canon = parent
        .canonicalize()
        .map_err(|e| UpdateError::Invalid(format!("update target parent unavailable: {e}")))?;
    if !parent_canon.starts_with(&root) {
        return Err(UpdateError::Invalid(
            "update target escapes install root".into(),
        ));
    }
    let control = root.join(".vsn-update");
    let pending_dir = control.join("pending");
    let previous_dir = control.join("previous");
    fs::create_dir_all(&pending_dir)?;
    fs::create_dir_all(&previous_dir)?;
    let pending = pending_dir.join(format!("{}.pending", request.release));
    let _ = fs::remove_file(&pending);
    fs::copy(&request.staged_artifact, &pending)?;
    if let Ok(meta) = fs::metadata(&target) {
        let _ = fs::set_permissions(&pending, meta.permissions());
    }
    fsync_file(&pending)?;
    verify_artifact(&pending, &request.expected_sha256)?;
    let state_path = update_state_path(&root);
    let previous_state = fs::read(&state_path)
        .ok()
        .and_then(|b| serde_json::from_slice::<FileInstallState>(&b).ok());
    let backup = backup_path(&root, &request.target_relative);
    let mut backup_created = false;
    if target.exists() {
        let _ = fs::remove_file(&backup);
        fs::rename(&target,&backup).map_err(|e|UpdateError::Invalid(format!("target could not be moved to backup; run from an out-of-process helper and ensure file is not locked: {e}")))?;
        backup_created = true;
    }
    if let Err(error) = fs::rename(&pending, &target) {
        if backup_created {
            let _ = fs::rename(&backup, &target);
        }
        return Err(UpdateError::Invalid(format!("pending update could not replace target; previous target restoration was attempted: {error}")));
    }
    fsync_file(&target)?;
    let state = FileInstallState {
        target_relative: request.target_relative.clone(),
        current_release: request.release.clone(),
        previous_release: previous_state.map(|s| s.current_release),
    };
    let state_tmp = control.join("state.json.tmp");
    fs::write(&state_tmp, serde_json::to_vec_pretty(&state)?)?;
    fsync_file(&state_tmp)?;
    fs::rename(&state_tmp, &state_path)?;
    fsync_dir(&control)?;
    Ok(ApplyFileResult {
        target,
        release: request.release.clone(),
        backup_created,
        rollback_available: backup.exists(),
    })
}

pub fn rollback_verified_file(
    install_root: &Path,
    confirm_rollback: bool,
) -> Result<ApplyFileResult, UpdateError> {
    if !confirm_rollback {
        return Err(UpdateError::Invalid(
            "rollback requires confirm_rollback=true".into(),
        ));
    }
    let root = install_root
        .canonicalize()
        .map_err(|e| UpdateError::Invalid(format!("install root unavailable: {e}")))?;
    let state_path = update_state_path(&root);
    let state: FileInstallState = serde_json::from_slice(&fs::read(&state_path)?)?;
    safe_relative(&state.target_relative)?;
    let target = root.join(&state.target_relative);
    let backup = backup_path(&root, &state.target_relative);
    if !backup.is_file() {
        return Err(UpdateError::Invalid(
            "no previous update backup is available".into(),
        ));
    }
    let control = root.join(".vsn-update");
    let failed = control.join("failed-current");
    fs::create_dir_all(&failed)?;
    let failed_target = failed.join(format!(
        "{}.{}",
        target
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("target"),
        state.current_release
    ));
    let _ = fs::remove_file(&failed_target);
    if target.exists() {
        fs::rename(&target, &failed_target).map_err(|e| {
            UpdateError::Invalid(format!(
                "current target could not be staged for rollback: {e}"
            ))
        })?;
    }
    if let Err(error) = fs::rename(&backup, &target) {
        if failed_target.exists() {
            let _ = fs::rename(&failed_target, &target);
        }
        return Err(UpdateError::Invalid(format!(
            "rollback replacement failed; current target restoration was attempted: {error}"
        )));
    }
    fsync_file(&target)?;
    let new_release = state
        .previous_release
        .clone()
        .unwrap_or_else(|| "previous".into());
    let next = FileInstallState {
        target_relative: state.target_relative.clone(),
        current_release: new_release.clone(),
        previous_release: None,
    };
    let tmp = control.join("state.json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(&next)?)?;
    fsync_file(&tmp)?;
    fs::rename(&tmp, &state_path)?;
    fsync_dir(&control)?;
    Ok(ApplyFileResult {
        target,
        release: new_release,
        backup_created: false,
        rollback_available: false,
    })
}

#[cfg(test)]
mod apply_tests {
    use super::*;
    #[test]
    fn unsafe_target_rejected() {
        assert!(safe_relative(Path::new("../vsn-agent")).is_err());
        assert!(safe_relative(Path::new("bin/vsn-agent")).is_ok());
    }
    #[test]
    fn apply_requires_confirmation() {
        let r = ApplyFileRequest {
            install_root: ".".into(),
            target_relative: "bin/x".into(),
            staged_artifact: "x".into(),
            expected_sha256: "0".repeat(64),
            release: "0.38.1".into(),
            confirm_apply: false,
        };
        assert!(apply_verified_file(&r).is_err());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateLockInfo {
    pub pid: u32,
    pub created_at_unix_ms: u128,
    pub helper_version: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateStatus {
    pub install_root: std::path::PathBuf,
    pub locked: bool,
    pub lock: Option<UpdateLockInfo>,
    pub current_release: Option<String>,
    pub previous_release: Option<String>,
    pub target_relative: Option<std::path::PathBuf>,
    pub rollback_available: bool,
}
pub struct UpdateLockGuard {
    path: std::path::PathBuf,
    active: bool,
}
impl Drop for UpdateLockGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = fs::remove_file(&self.path);
            if let Some(parent) = self.path.parent() {
                let _ = fsync_dir(parent);
            }
        }
    }
}

pub fn apply_verified_file_locked(
    request: &ApplyFileRequest,
) -> Result<ApplyFileResult, UpdateError> {
    let root = canonical_install_root(Path::new(&request.install_root))?;
    let _lock = acquire_update_lock(&root)?;
    apply_verified_file(request)
}
pub fn rollback_verified_file_locked(
    install_root: &Path,
    confirm_rollback: bool,
) -> Result<ApplyFileResult, UpdateError> {
    let root = canonical_install_root(install_root)?;
    let _lock = acquire_update_lock(&root)?;
    rollback_verified_file(&root, confirm_rollback)
}
fn canonical_install_root(path: &Path) -> Result<PathBuf, UpdateError> {
    let root = path
        .canonicalize()
        .map_err(|e| UpdateError::Invalid(format!("install root unavailable: {e}")))?;
    if !root.is_dir() {
        return Err(UpdateError::Invalid(
            "install root must be a directory".into(),
        ));
    }
    Ok(root)
}

pub fn acquire_update_lock(install_root: &Path) -> Result<UpdateLockGuard, UpdateError> {
    let root = install_root
        .canonicalize()
        .map_err(|e| UpdateError::Invalid(format!("install root unavailable: {e}")))?;
    let control = root.join(".vsn-update");
    fs::create_dir_all(&control)?;
    let path = control.join("apply.lock");
    let info = UpdateLockInfo {
        pid: std::process::id(),
        created_at_unix_ms: now_ms(),
        helper_version: env!("CARGO_PKG_VERSION").into(),
    };
    let bytes = serde_json::to_vec_pretty(&info)?;
    let mut file=fs::OpenOptions::new().create_new(true).write(true).open(&path).map_err(|e|if e.kind()==std::io::ErrorKind::AlreadyExists{UpdateError::Invalid("another updater helper already owns .vsn-update/apply.lock; inspect status or explicitly recover a stale lock".into())}else{UpdateError::Io(e)})?;
    use std::io::Write as _;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    fsync_dir(&control)?;
    Ok(UpdateLockGuard { path, active: true })
}

pub fn update_status(install_root: &Path) -> Result<UpdateStatus, UpdateError> {
    let root = install_root
        .canonicalize()
        .map_err(|e| UpdateError::Invalid(format!("install root unavailable: {e}")))?;
    let control = root.join(".vsn-update");
    let lock_path = control.join("apply.lock");
    let lock = if lock_path.is_file() {
        fs::read(&lock_path)
            .ok()
            .and_then(|b| serde_json::from_slice::<UpdateLockInfo>(&b).ok())
    } else {
        None
    };
    let state_path = update_state_path(&root);
    let state = if state_path.is_file() {
        Some(serde_json::from_slice::<FileInstallState>(&fs::read(
            &state_path,
        )?)?)
    } else {
        None
    };
    let (target_relative, current_release, previous_release, rollback_available) =
        if let Some(s) = state {
            let backup = backup_path(&root, &s.target_relative);
            (
                Some(s.target_relative),
                Some(s.current_release),
                s.previous_release,
                backup.is_file(),
            )
        } else {
            (None, None, None, false)
        };
    Ok(UpdateStatus {
        install_root: root,
        locked: lock_path.exists(),
        lock,
        current_release,
        previous_release,
        target_relative,
        rollback_available,
    })
}

pub fn recover_stale_update_lock(
    install_root: &Path,
    confirm_recover: bool,
) -> Result<bool, UpdateError> {
    if !confirm_recover {
        return Err(UpdateError::Invalid(
            "stale lock recovery requires confirm_recover=true".into(),
        ));
    }
    let root = install_root
        .canonicalize()
        .map_err(|e| UpdateError::Invalid(format!("install root unavailable: {e}")))?;
    let control = root.join(".vsn-update");
    let lock_path = control.join("apply.lock");
    if !lock_path.exists() {
        return Ok(false);
    }
    let info: UpdateLockInfo = serde_json::from_slice(&fs::read(&lock_path)?).map_err(|_| {
        UpdateError::Invalid("update lock is malformed; refuse automatic recovery".into())
    })?;
    // A fresh lock is never auto-removed. This is a conservative time-based stale-lock
    // recovery because cross-platform process liveness checks are not uniformly reliable.
    let age = now_ms().saturating_sub(info.created_at_unix_ms);
    if age < 10 * 60 * 1000 {
        return Err(UpdateError::Invalid(
            "update lock is younger than 10 minutes; refusing stale-lock recovery".into(),
        ));
    }
    fs::remove_file(&lock_path)?;
    fsync_dir(&control)?;
    Ok(true)
}
fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod helper_lock_tests {
    use super::*;
    #[test]
    fn status_without_state_is_safe() {
        let dir = std::env::temp_dir().join(format!("vsn-update-test-{}", now_ms()));
        fs::create_dir_all(&dir).unwrap();
        let s = update_status(&dir).unwrap();
        assert!(!s.locked);
        assert!(s.current_release.is_none());
        let _ = fs::remove_dir_all(dir);
    }
    #[test]
    fn stale_recovery_requires_confirmation() {
        let dir = std::env::temp_dir().join(format!("vsn-update-lock-test-{}", now_ms()));
        fs::create_dir_all(&dir).unwrap();
        assert!(recover_stale_update_lock(&dir, false).is_err());
        let _ = fs::remove_dir_all(dir);
    }
}
