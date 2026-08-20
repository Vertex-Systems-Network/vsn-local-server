use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use keyring::Entry;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, OnceLock},
};
use thiserror::Error;

const KEYRING_SERVICE: &str = "VSN Platform Vault";
const MASTER_ENTRY: &str = "vault-master-v1";
const CURRENT_FORMAT_VERSION: u32 = 2;
#[derive(Debug, Error)]
pub enum VaultError {
    #[error("vault I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("vault JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("secure store error: {0}")]
    SecureStore(String),
    #[error("vault cryptographic operation failed")]
    Crypto,
    #[error("invalid secret name")]
    InvalidName,
    #[error("secret not found: {0}")]
    NotFound(String),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct VaultFile {
    version: u32,
    #[serde(default)]
    key_id: Option<String>,
    entries: Vec<EncryptedEntry>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct EncryptedEntry {
    name: String,
    nonce: String,
    ciphertext: String,
    updated_at_unix: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecretMetadata {
    pub name: String,
    pub updated_at_unix: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultStatus {
    pub format_version: u32,
    pub key_id: String,
    pub entries: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultRotationResult {
    pub previous_key_id: String,
    pub current_key_id: String,
    pub entries_reencrypted: usize,
    pub old_key_retained_for_recovery: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultKeyRecord {
    pub key_id: String,
    pub state: String,
    pub recorded_at_unix: u64,
    pub backup_file: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
struct VaultKeyHistory {
    records: Vec<VaultKeyRecord>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultRecoveryResult {
    pub restored_key_id: String,
    pub previous_current_key_id: String,
    pub entries_restored: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultRetirementResult {
    pub retired_key_id: String,
    pub backup_removed: bool,
    pub secure_store_removed: bool,
}
impl Default for VaultFile {
    fn default() -> Self {
        Self {
            version: CURRENT_FORMAT_VERSION,
            key_id: Some(MASTER_ENTRY.into()),
            entries: vec![],
        }
    }
}

pub fn default_path() -> Result<PathBuf, VaultError> {
    Ok(vsn_security::data_dir()?.join("vault").join("secrets.json"))
}
fn history_path() -> Result<PathBuf, VaultError> {
    Ok(vsn_security::data_dir()?.join("vault").join("keys.json"))
}
fn recovery_dir() -> Result<PathBuf, VaultError> {
    Ok(vsn_security::data_dir()?.join("vault").join("recovery"))
}
fn recovery_backup_path(key_id: &str) -> Result<PathBuf, VaultError> {
    validate_key_id(key_id)?;
    Ok(recovery_dir()?.join(format!("{key_id}.vault.json")))
}
fn load_history() -> Result<VaultKeyHistory, VaultError> {
    let p = history_path()?;
    if !p.exists() {
        return Ok(VaultKeyHistory::default());
    }
    Ok(serde_json::from_slice(&fs::read(p)?)?)
}
fn save_history(history: &VaultKeyHistory) -> Result<(), VaultError> {
    let p = history_path()?;
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
        set_private_dir(parent)?;
    }
    let tmp = p.with_extension("tmp");
    let mut bytes = serde_json::to_vec_pretty(history)?;
    bytes.push(b'\n');
    {
        let mut f = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
    }
    set_private_file(&tmp)?;
    fs::rename(tmp, p)?;
    Ok(())
}
pub fn key_history() -> Result<Vec<VaultKeyRecord>, VaultError> {
    let _guard = vault_guard()?;
    let mut h = load_history()?.records;
    h.sort_by(|a, b| b.recorded_at_unix.cmp(&a.recorded_at_unix));
    Ok(h)
}
pub fn list() -> Result<Vec<SecretMetadata>, VaultError> {
    let _guard = vault_guard()?;
    let file = load(&default_path()?)?;
    let mut out = file
        .entries
        .into_iter()
        .map(|e| SecretMetadata {
            name: e.name,
            updated_at_unix: e.updated_at_unix,
        })
        .collect::<Vec<_>>();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}
pub fn set(name: &str, value: &str) -> Result<SecretMetadata, VaultError> {
    let _guard = vault_guard()?;
    validate_name(name)?;
    if value.len() > 1024 * 1024 {
        return Err(VaultError::Crypto);
    }
    let path = default_path()?;
    let mut file = load(&path)?;
    let key_id = file.key_id.clone().unwrap_or_else(|| MASTER_ENTRY.into());
    let key = master_key_for(&key_id, true)?;
    file.version = CURRENT_FORMAT_VERSION;
    file.key_id = Some(key_id);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), value.as_bytes())
        .map_err(|_| VaultError::Crypto)?;
    let updated_at_unix = now();
    let entry = EncryptedEntry {
        name: name.into(),
        nonce: B64.encode(nonce_bytes),
        ciphertext: B64.encode(ciphertext),
        updated_at_unix,
    };
    file.entries.retain(|e| e.name != name);
    file.entries.push(entry);
    save(&path, &file)?;
    Ok(SecretMetadata {
        name: name.into(),
        updated_at_unix,
    })
}
pub fn reveal(name: &str) -> Result<String, VaultError> {
    let _guard = vault_guard()?;
    validate_name(name)?;
    let file = load(&default_path()?)?;
    let entry = file
        .entries
        .iter()
        .find(|e| e.name == name)
        .ok_or_else(|| VaultError::NotFound(name.into()))?;
    let nonce = B64.decode(&entry.nonce).map_err(|_| VaultError::Crypto)?;
    let ciphertext = B64
        .decode(&entry.ciphertext)
        .map_err(|_| VaultError::Crypto)?;
    if nonce.len() != 12 {
        return Err(VaultError::Crypto);
    }
    let key_id = file.key_id.as_deref().unwrap_or(MASTER_ENTRY);
    let key = master_key_for(key_id, false)?;
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    let plain = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| VaultError::Crypto)?;
    String::from_utf8(plain).map_err(|_| VaultError::Crypto)
}
pub fn delete(name: &str) -> Result<bool, VaultError> {
    let _guard = vault_guard()?;
    validate_name(name)?;
    let path = default_path()?;
    let mut file = load(&path)?;
    let before = file.entries.len();
    file.entries.retain(|e| e.name != name);
    let changed = before != file.entries.len();
    if changed {
        save(&path, &file)?;
    }
    Ok(changed)
}
pub fn exists(name: &str) -> Result<bool, VaultError> {
    let _guard = vault_guard()?;
    validate_name(name)?;
    Ok(load(&default_path()?)?
        .entries
        .iter()
        .any(|e| e.name == name))
}

fn master_key_for(key_id: &str, create_if_missing: bool) -> Result<[u8; 32], VaultError> {
    validate_key_id(key_id)?;
    let entry =
        Entry::new(KEYRING_SERVICE, key_id).map_err(|e| VaultError::SecureStore(e.to_string()))?;
    let bytes = match entry.get_password() {
        Ok(v) => B64.decode(v).map_err(|_| VaultError::Crypto)?,
        Err(keyring::Error::NoEntry) if create_if_missing => {
            let mut k = [0u8; 32];
            OsRng.fill_bytes(&mut k);
            entry
                .set_password(&B64.encode(k))
                .map_err(|e| VaultError::SecureStore(e.to_string()))?;
            k.to_vec()
        }
        Err(keyring::Error::NoEntry) => {
            return Err(VaultError::SecureStore(format!(
                "vault key {key_id} is missing from secure storage"
            )))
        }
        Err(e) => return Err(VaultError::SecureStore(e.to_string())),
    };
    bytes.as_slice().try_into().map_err(|_| VaultError::Crypto)
}
fn validate_key_id(value: &str) -> Result<(), VaultError> {
    if value.len() < 8
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        Err(VaultError::SecureStore(
            "invalid vault key identifier".into(),
        ))
    } else {
        Ok(())
    }
}
pub fn status() -> Result<VaultStatus, VaultError> {
    let _guard = vault_guard()?;
    let file = load(&default_path()?)?;
    Ok(VaultStatus {
        format_version: file.version,
        key_id: file.key_id.unwrap_or_else(|| MASTER_ENTRY.into()),
        entries: file.entries.len(),
    })
}
pub fn rotate_master_key() -> Result<VaultRotationResult, VaultError> {
    let _guard = vault_guard()?;
    let path = default_path()?;
    let file = load(&path)?;
    let previous_key_id = file.key_id.clone().unwrap_or_else(|| MASTER_ENTRY.into());
    let old_key = master_key_for(&previous_key_id, file.entries.is_empty())?;
    let old_cipher = ChaCha20Poly1305::new(Key::from_slice(&old_key));
    let mut plaintext = Vec::with_capacity(file.entries.len());
    for entry in &file.entries {
        let nonce = B64.decode(&entry.nonce).map_err(|_| VaultError::Crypto)?;
        let ciphertext = B64
            .decode(&entry.ciphertext)
            .map_err(|_| VaultError::Crypto)?;
        if nonce.len() != 12 {
            return Err(VaultError::Crypto);
        }
        let plain = old_cipher
            .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
            .map_err(|_| VaultError::Crypto)?;
        plaintext.push((entry.name.clone(), plain, entry.updated_at_unix));
    }
    let recovery = recovery_backup_path(&previous_key_id)?;
    if let Some(parent) = recovery.parent() {
        fs::create_dir_all(parent)?;
        set_private_dir(parent)?;
    }
    let mut backup_bytes = serde_json::to_vec_pretty(&file)?;
    backup_bytes.push(b'\n');
    let backup_tmp = recovery.with_extension("tmp");
    {
        let mut f = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&backup_tmp)?;
        f.write_all(&backup_bytes)?;
        f.sync_all()?;
    }
    set_private_file(&backup_tmp)?;
    fs::rename(&backup_tmp, &recovery)?;
    let mut rotation_suffix = [0u8; 8];
    OsRng.fill_bytes(&mut rotation_suffix);
    let suffix = rotation_suffix
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let current_key_id = format!("vault-master-v2-{}-{suffix}", now());
    let new_key = master_key_for(&current_key_id, true)?;
    let new_cipher = ChaCha20Poly1305::new(Key::from_slice(&new_key));
    let mut entries = Vec::with_capacity(plaintext.len());
    for (name, plain, updated_at_unix) in plaintext {
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let ciphertext = new_cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), plain.as_ref())
            .map_err(|_| VaultError::Crypto)?;
        entries.push(EncryptedEntry {
            name,
            nonce: B64.encode(nonce_bytes),
            ciphertext: B64.encode(ciphertext),
            updated_at_unix,
        });
    }
    let rotated = VaultFile {
        version: CURRENT_FORMAT_VERSION,
        key_id: Some(current_key_id.clone()),
        entries,
    };
    save(&path, &rotated)?;
    let mut history = load_history()?;
    for r in &mut history.records {
        if r.state == "current" {
            r.state = "recovery".into();
        }
    }
    if let Some(existing) = history
        .records
        .iter_mut()
        .find(|r| r.key_id == previous_key_id)
    {
        existing.state = "recovery".into();
        existing.backup_file = Some(recovery.display().to_string());
        existing.recorded_at_unix = now();
    } else {
        history.records.push(VaultKeyRecord {
            key_id: previous_key_id.clone(),
            state: "recovery".into(),
            recorded_at_unix: now(),
            backup_file: Some(recovery.display().to_string()),
        });
    }
    history.records.push(VaultKeyRecord {
        key_id: current_key_id.clone(),
        state: "current".into(),
        recorded_at_unix: now(),
        backup_file: None,
    });
    save_history(&history)?;
    Ok(VaultRotationResult {
        previous_key_id,
        current_key_id,
        entries_reencrypted: rotated.entries.len(),
        old_key_retained_for_recovery: true,
    })
}

pub fn restore_recovery_key(
    key_id: &str,
    confirm: bool,
) -> Result<VaultRecoveryResult, VaultError> {
    let _guard = vault_guard()?;
    if !confirm {
        return Err(VaultError::SecureStore(
            "vault recovery restore requires explicit confirmation".into(),
        ));
    }
    validate_key_id(key_id)?;
    let path = default_path()?;
    let current = load(&path)?;
    let current_key_id = current
        .key_id
        .clone()
        .unwrap_or_else(|| MASTER_ENTRY.into());
    if key_id == current_key_id {
        return Err(VaultError::SecureStore(
            "requested recovery key is already current".into(),
        ));
    }
    let backup_path = recovery_backup_path(key_id)?;
    if !backup_path.is_file() {
        return Err(VaultError::SecureStore(
            "recovery vault snapshot is unavailable".into(),
        ));
    }
    let backup: VaultFile = serde_json::from_slice(&fs::read(&backup_path)?)?;
    if backup.key_id.as_deref().unwrap_or(MASTER_ENTRY) != key_id {
        return Err(VaultError::Crypto);
    }
    let key = master_key_for(key_id, false)?;
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    for entry in &backup.entries {
        let nonce = B64.decode(&entry.nonce).map_err(|_| VaultError::Crypto)?;
        let ciphertext = B64
            .decode(&entry.ciphertext)
            .map_err(|_| VaultError::Crypto)?;
        cipher
            .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
            .map_err(|_| VaultError::Crypto)?;
    }
    let current_backup = recovery_backup_path(&current_key_id)?;
    if let Some(parent) = current_backup.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&current_backup, serde_json::to_vec_pretty(&current)?)?;
    set_private_file(&current_backup)?;
    save(&path, &backup)?;
    let mut history = load_history()?;
    for r in &mut history.records {
        if r.key_id == current_key_id {
            r.state = "recovery".into();
            r.backup_file = Some(current_backup.display().to_string());
        }
        if r.key_id == key_id {
            r.state = "current".into();
        }
    }
    save_history(&history)?;
    Ok(VaultRecoveryResult {
        restored_key_id: key_id.into(),
        previous_current_key_id: current_key_id,
        entries_restored: backup.entries.len(),
    })
}

pub fn retire_recovery_key(
    key_id: &str,
    confirm: bool,
) -> Result<VaultRetirementResult, VaultError> {
    let _guard = vault_guard()?;
    if !confirm {
        return Err(VaultError::SecureStore(
            "vault key retirement requires explicit confirmation".into(),
        ));
    }
    validate_key_id(key_id)?;
    let file = load(&default_path()?)?;
    let current = file.key_id.as_deref().unwrap_or(MASTER_ENTRY);
    if current == key_id {
        return Err(VaultError::SecureStore(
            "current vault key cannot be retired".into(),
        ));
    }
    let current_key = master_key_for(current, file.entries.is_empty())?;
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&current_key));
    for entry in &file.entries {
        let nonce = B64.decode(&entry.nonce).map_err(|_| VaultError::Crypto)?;
        let ciphertext = B64
            .decode(&entry.ciphertext)
            .map_err(|_| VaultError::Crypto)?;
        cipher
            .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
            .map_err(|_| VaultError::Crypto)?;
    }
    let mut history = load_history()?;
    let record = history
        .records
        .iter_mut()
        .find(|r| r.key_id == key_id)
        .ok_or_else(|| VaultError::SecureStore("vault recovery key is not tracked".into()))?;
    if record.state != "recovery" {
        return Err(VaultError::SecureStore(
            "only recovery keys may be retired".into(),
        ));
    }
    let backup = recovery_backup_path(key_id)?;
    let backup_removed = if backup.exists() {
        fs::remove_file(&backup)?;
        true
    } else {
        false
    };
    let entry =
        Entry::new(KEYRING_SERVICE, key_id).map_err(|e| VaultError::SecureStore(e.to_string()))?;
    let secure_store_removed = match entry.delete_credential() {
        Ok(()) => true,
        Err(keyring::Error::NoEntry) => false,
        Err(e) => return Err(VaultError::SecureStore(e.to_string())),
    };
    record.state = "retired".into();
    record.backup_file = None;
    record.recorded_at_unix = now();
    save_history(&history)?;
    Ok(VaultRetirementResult {
        retired_key_id: key_id.into(),
        backup_removed,
        secure_store_removed,
    })
}

static VAULT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
fn vault_guard() -> Result<MutexGuard<'static, ()>, VaultError> {
    VAULT_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| VaultError::SecureStore("vault lock poisoned".into()))
}
fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("bak")
}
fn load(path: &Path) -> Result<VaultFile, VaultError> {
    let backup = backup_path(path);
    if !path.exists() && !backup.exists() {
        return Ok(VaultFile::default());
    }
    let source = if path.exists() {
        path
    } else {
        backup.as_path()
    };
    let file: VaultFile = serde_json::from_slice(&fs::read(source)?)?;
    if !(1..=CURRENT_FORMAT_VERSION).contains(&file.version) {
        return Err(VaultError::Crypto);
    }
    if source == backup.as_path() && !path.exists() {
        let _ = fs::rename(&backup, path);
    }
    Ok(file)
}
fn save(path: &Path, file: &VaultFile) -> Result<(), VaultError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        set_private_dir(parent)?;
    }
    let tmp = path.with_extension("tmp");
    let backup = backup_path(path);
    let mut bytes = serde_json::to_vec_pretty(file)?;
    bytes.push(b'\n');
    {
        let mut handle = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)?;
        handle.write_all(&bytes)?;
        handle.sync_all()?;
    }
    set_private_file(&tmp)?;
    if backup.exists() {
        let _ = fs::remove_file(&backup);
    }
    if path.exists() {
        fs::rename(path, &backup)?;
    }
    if let Err(err) = fs::rename(&tmp, path) {
        if backup.exists() {
            let _ = fs::rename(&backup, path);
        }
        return Err(VaultError::Io(err));
    }
    if backup.exists() {
        fs::remove_file(&backup)?;
    }
    if let Some(parent) = path.parent() {
        if let Ok(dir) = fs::File::open(parent) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}
#[cfg(unix)]
fn set_private_file(path: &Path) -> Result<(), io::Error> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}
#[cfg(not(unix))]
fn set_private_file(_path: &Path) -> Result<(), io::Error> {
    Ok(())
}
#[cfg(unix)]
fn set_private_dir(path: &Path) -> Result<(), io::Error> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}
#[cfg(not(unix))]
fn set_private_dir(_path: &Path) -> Result<(), io::Error> {
    Ok(())
}
fn validate_name(value: &str) -> Result<(), VaultError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':' | b'/'))
    {
        Err(VaultError::InvalidName)
    } else {
        Ok(())
    }
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
impl From<vsn_security::SecurityError> for VaultError {
    fn from(e: vsn_security::SecurityError) -> Self {
        VaultError::SecureStore(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn name_validation_blocks_shell_like_values() {
        assert!(validate_name("db.password").is_ok());
        assert!(validate_name("x;whoami").is_err());
    }
    #[test]
    fn key_ids_are_bounded() {
        assert!(validate_key_id(MASTER_ENTRY).is_ok());
        assert!(validate_key_id("../bad").is_err());
    }
}
