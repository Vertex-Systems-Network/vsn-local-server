use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use directories::ProjectDirs;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hmac::{Hmac, Mac};
use keyring::Entry;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

const KEYRING_SERVICE: &str = "vsn-agent";
const DEVICE_KEY_ENTRY: &str = "device-ed25519-v1";
#[cfg(not(windows))]
const IPC_KEY_ENTRY: &str = "local-ipc-hmac-v1";

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("OS secure store error: {0}")]
    SecureStore(String),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("identity metadata error: {0}")]
    Metadata(#[from] serde_json::Error),
    #[error("invalid stored secret: {0}")]
    InvalidSecret(String),
    #[error("device public/private key mismatch")]
    IdentityMismatch,
    #[error("signature verification failed")]
    SignatureInvalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceIdentityMetadata {
    pub version: u32,
    pub device_id: String,
    pub public_key: String,
    pub display_name: String,
    pub os: String,
    pub created_at_unix: u64,
}

#[derive(Debug, Clone)]
pub struct DeviceIdentity {
    metadata: DeviceIdentityMetadata,
    signing_key: SigningKey,
}

impl DeviceIdentity {
    pub fn load_or_create() -> Result<Self, SecurityError> {
        let secret = load_or_create_secret(DEVICE_KEY_ENTRY, 32)?;
        let key_bytes: [u8; 32] = secret
            .as_slice()
            .try_into()
            .map_err(|_| SecurityError::InvalidSecret("device key must be 32 bytes".into()))?;
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key = signing_key.verifying_key();
        let public_key = B64.encode(verifying_key.to_bytes());
        let device_id = device_id_from_public_key(&verifying_key);

        let path = identity_metadata_path()?;
        let metadata = if path.exists() {
            let data = fs::read(&path)?;
            let stored: DeviceIdentityMetadata = serde_json::from_slice(&data)?;
            if stored.public_key != public_key || stored.device_id != device_id {
                return Err(SecurityError::IdentityMismatch);
            }
            stored
        } else {
            let metadata = DeviceIdentityMetadata {
                version: 1,
                device_id,
                public_key,
                display_name: machine_name(),
                os: std::env::consts::OS.to_string(),
                created_at_unix: unix_timestamp(),
            };
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut encoded = serde_json::to_vec_pretty(&metadata)?;
            encoded.push(b'\n');
            atomic_write(&path, &encoded)?;
            metadata
        };

        Ok(Self {
            metadata,
            signing_key,
        })
    }

    pub fn metadata(&self) -> &DeviceIdentityMetadata {
        &self.metadata
    }

    pub fn sign(&self, message: &[u8]) -> String {
        B64.encode(self.signing_key.sign(message).to_bytes())
    }

    pub fn verify(&self, message: &[u8], signature_b64: &str) -> Result<(), SecurityError> {
        verify_signature(&self.metadata.public_key, message, signature_b64)
    }

    pub fn verify_with_public_key(
        public_key_b64: &str,
        message: &[u8],
        signature_b64: &str,
    ) -> Result<(), SecurityError> {
        verify_signature(public_key_b64, message, signature_b64)
    }
}

pub fn device_id_from_public_key_b64(public_key_b64: &str) -> Result<String, SecurityError> {
    let public = B64
        .decode(public_key_b64)
        .map_err(|e| SecurityError::InvalidSecret(format!("invalid public key: {e}")))?;
    let public: [u8; 32] = public
        .as_slice()
        .try_into()
        .map_err(|_| SecurityError::InvalidSecret("public key must be 32 bytes".into()))?;
    let verifying_key = VerifyingKey::from_bytes(&public)
        .map_err(|e| SecurityError::InvalidSecret(format!("invalid verifying key: {e}")))?;
    Ok(device_id_from_public_key(&verifying_key))
}

pub fn verify_signature(
    public_key_b64: &str,
    message: &[u8],
    signature_b64: &str,
) -> Result<(), SecurityError> {
    let public = B64
        .decode(public_key_b64)
        .map_err(|e| SecurityError::InvalidSecret(format!("invalid public key: {e}")))?;
    let public: [u8; 32] = public
        .as_slice()
        .try_into()
        .map_err(|_| SecurityError::InvalidSecret("public key must be 32 bytes".into()))?;
    let verifying_key = VerifyingKey::from_bytes(&public)
        .map_err(|e| SecurityError::InvalidSecret(format!("invalid verifying key: {e}")))?;

    let signature = B64
        .decode(signature_b64)
        .map_err(|e| SecurityError::InvalidSecret(format!("invalid signature encoding: {e}")))?;
    let signature: [u8; 64] = signature
        .as_slice()
        .try_into()
        .map_err(|_| SecurityError::InvalidSecret("signature must be 64 bytes".into()))?;
    let signature = Signature::from_bytes(&signature);

    verifying_key
        .verify(message, &signature)
        .map_err(|_| SecurityError::SignatureInvalid)
}

#[derive(Debug, Clone)]
pub struct IpcAuthenticator {
    key: [u8; 32],
}

impl IpcAuthenticator {
    pub fn load_or_create() -> Result<Self, SecurityError> {
        let secret = load_or_create_ipc_secret()?;
        let key: [u8; 32] = secret
            .as_slice()
            .try_into()
            .map_err(|_| SecurityError::InvalidSecret("IPC key must be 32 bytes".into()))?;
        Ok(Self { key })
    }

    pub fn sign(&self, canonical_message: &[u8]) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).expect("HMAC accepts 32-byte keys");
        mac.update(canonical_message);
        B64.encode(mac.finalize().into_bytes())
    }

    pub fn verify(&self, canonical_message: &[u8], supplied_mac_b64: &str) -> bool {
        let supplied = match B64.decode(supplied_mac_b64) {
            Ok(value) => value,
            Err(_) => return false,
        };
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).expect("HMAC accepts 32-byte keys");
        mac.update(canonical_message);
        mac.verify_slice(&supplied).is_ok()
    }
}

pub fn secure_store_name() -> &'static str {
    #[cfg(windows)]
    {
        "os-credential-store + ACL-protected machine IPC secret"
    }
    #[cfg(not(windows))]
    {
        "os-credential-store"
    }
}

pub fn data_dir() -> Result<PathBuf, SecurityError> {
    let dirs = ProjectDirs::from("dev", "VSN", "VSN Platform").ok_or_else(|| {
        SecurityError::InvalidSecret("unable to resolve application data directory".into())
    })?;
    Ok(dirs.data_local_dir().to_path_buf())
}

fn identity_metadata_path() -> Result<PathBuf, SecurityError> {
    Ok(data_dir()?.join("security").join("device.json"))
}

fn load_or_create_ipc_secret() -> Result<Vec<u8>, SecurityError> {
    #[cfg(windows)]
    {
        load_or_create_windows_ipc_secret()
    }
    #[cfg(not(windows))]
    {
        load_or_create_secret(IPC_KEY_ENTRY, 32)
    }
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowsIpcAclPrincipal {
    System,
    Administrators,
    LocalService,
    OrdinaryCreator,
}

#[cfg(windows)]
fn windows_ipc_creator_principal(sid: &str) -> WindowsIpcAclPrincipal {
    match sid {
        "S-1-5-18" => WindowsIpcAclPrincipal::System,
        "S-1-5-32-544" => WindowsIpcAclPrincipal::Administrators,
        "S-1-5-19" => WindowsIpcAclPrincipal::LocalService,
        _ => WindowsIpcAclPrincipal::OrdinaryCreator,
    }
}

#[cfg(windows)]
fn windows_ipc_file_grants(sid: &str) -> Vec<String> {
    let mut grants = vec![
        "*S-1-5-18:(F)".to_string(),
        "*S-1-5-32-544:(F)".to_string(),
        "*S-1-5-19:(R)".to_string(),
    ];
    if windows_ipc_creator_principal(sid) == WindowsIpcAclPrincipal::OrdinaryCreator {
        grants.push(format!("*{sid}:(R)"));
    }
    grants
}

#[cfg(windows)]
fn windows_ipc_directory_grants(sid: &str) -> Vec<String> {
    let mut grants = vec![
        "*S-1-5-18:(OI)(CI)(F)".to_string(),
        "*S-1-5-32-544:(OI)(CI)(F)".to_string(),
        "*S-1-5-19:(OI)(CI)(R)".to_string(),
    ];
    if windows_ipc_creator_principal(sid) == WindowsIpcAclPrincipal::OrdinaryCreator {
        grants.push(format!("*{sid}:(OI)(CI)(F)"));
    }
    grants
}

#[cfg(windows)]
fn load_or_create_windows_ipc_secret() -> Result<Vec<u8>, SecurityError> {
    use std::{fs::OpenOptions, io::Write, process::Command};

    let program_data = std::env::var_os("PROGRAMDATA")
        .map(PathBuf::from)
        .ok_or_else(|| SecurityError::SecureStore("PROGRAMDATA is not available".into()))?;
    let directory = program_data.join("VSN").join("security");
    let path = directory.join("ipc.key");

    fs::create_dir_all(&directory)?;

    // Existing credentials are read under their existing ACL. Do not recalculate the
    // directory ACL from the current account: the Windows service runs as LocalService
    // while the CLI runs as the installing user, and both need stable access.
    match fs::read_to_string(&path) {
        Ok(encoded) => {
            return B64.decode(encoded.trim()).map_err(|e| {
                SecurityError::InvalidSecret(format!("shared IPC secret is corrupted: {e}"))
            });
        }
        Err(error) if error.kind() != io::ErrorKind::NotFound => {
            return Err(SecurityError::Io(error))
        }
        Err(_) => {}
    }

    let sid = current_windows_user_sid()?;
    secure_windows_ipc_directory(&directory, &sid)?;

    let mut secret = vec![0u8; 32];
    OsRng.fill_bytes(&mut secret);
    let encoded = B64.encode(&secret);

    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(mut file) => {
            file.write_all(encoded.as_bytes())?;
            file.sync_all()?;
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            let encoded = fs::read_to_string(&path)?;
            return B64.decode(encoded.trim()).map_err(|e| {
                SecurityError::InvalidSecret(format!("shared IPC secret is corrupted: {e}"))
            });
        }
        Err(error) => return Err(SecurityError::Io(error)),
    }

    // Tighten the file after creation without allowing a creator SID that aliases a
    // baseline principal to replace the mandatory SYSTEM/Admin/LocalService floor.
    let grants = windows_ipc_file_grants(&sid);
    let mut command = Command::new("icacls.exe");
    command.arg(&path).args(["/inheritance:r", "/grant:r"]);
    for grant in &grants {
        command.arg(grant);
    }
    let output = command.output().map_err(SecurityError::Io)?;

    if !output.status.success() {
        let _ = fs::remove_file(&path);
        return Err(SecurityError::SecureStore(format!(
            "failed to secure IPC secret ACL: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(secret)
}

#[cfg(windows)]
fn secure_windows_ipc_directory(path: &std::path::Path, sid: &str) -> Result<(), SecurityError> {
    use std::process::Command;
    let grants = windows_ipc_directory_grants(sid);
    let mut command = Command::new("icacls.exe");
    command.arg(path).args(["/inheritance:r", "/grant:r"]);
    for grant in &grants {
        command.arg(grant);
    }
    let output = command.output().map_err(SecurityError::Io)?;
    if !output.status.success() {
        return Err(SecurityError::SecureStore(format!(
            "failed to secure IPC directory ACL: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn current_windows_user_sid() -> Result<String, SecurityError> {
    use std::process::Command;
    let output = Command::new("whoami.exe")
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
        .map_err(SecurityError::Io)?;
    if !output.status.success() {
        return Err(SecurityError::SecureStore(
            "unable to resolve current Windows user SID".into(),
        ));
    }
    let line = String::from_utf8_lossy(&output.stdout);
    let fields: Vec<&str> = line.trim().split("\",\"").collect();
    let sid = fields
        .get(1)
        .map(|value| value.trim().trim_matches('"').to_string())
        .filter(|value| value.starts_with("S-1-"))
        .ok_or_else(|| {
            SecurityError::SecureStore("unable to parse current Windows user SID".into())
        })?;
    Ok(sid)
}

fn load_or_create_secret(entry_name: &str, bytes: usize) -> Result<Vec<u8>, SecurityError> {
    let entry = Entry::new(KEYRING_SERVICE, entry_name)
        .map_err(|e| SecurityError::SecureStore(e.to_string()))?;

    match entry.get_password() {
        Ok(encoded) => B64.decode(encoded).map_err(|e| {
            SecurityError::InvalidSecret(format!("secure store secret is corrupted: {e}"))
        }),
        Err(keyring::Error::NoEntry) => {
            let mut secret = vec![0u8; bytes];
            OsRng.fill_bytes(&mut secret);
            entry
                .set_password(&B64.encode(&secret))
                .map_err(|e| SecurityError::SecureStore(e.to_string()))?;
            Ok(secret)
        }
        Err(e) => Err(SecurityError::SecureStore(e.to_string())),
    }
}

fn device_id_from_public_key(key: &VerifyingKey) -> String {
    let digest = Sha256::digest(key.as_bytes());
    let mut out = String::from("dev_");
    for byte in &digest[..20] {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

fn machine_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-machine".to_string())
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn atomic_write(path: &PathBuf, bytes: &[u8]) -> Result<(), io::Error> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_key_device_id_is_stable() {
        let secret = [7u8; 32];
        let key = SigningKey::from_bytes(&secret);
        assert_eq!(
            device_id_from_public_key(&key.verifying_key()),
            device_id_from_public_key(&key.verifying_key())
        );
    }

    #[test]
    fn hmac_rejects_wrong_message() {
        let auth = IpcAuthenticator { key: [1u8; 32] };
        let mac = auth.sign(b"hello");
        assert!(auth.verify(b"hello", &mac));
        assert!(!auth.verify(b"goodbye", &mac));
    }

    #[cfg(windows)]
    #[test]
    fn windows_ipc_system_creator_preserves_full_control() {
        let file = windows_ipc_file_grants("S-1-5-18");
        let directory = windows_ipc_directory_grants("S-1-5-18");
        assert_eq!(
            file.iter().filter(|grant| grant.starts_with("*S-1-5-18:")).count(),
            1
        );
        assert!(file.iter().any(|grant| grant == "*S-1-5-18:(F)"));
        assert_eq!(
            directory
                .iter()
                .filter(|grant| grant.starts_with("*S-1-5-18:"))
                .count(),
            1
        );
        assert!(directory
            .iter()
            .any(|grant| grant == "*S-1-5-18:(OI)(CI)(F)"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_ipc_local_service_creator_does_not_gain_write() {
        let file = windows_ipc_file_grants("S-1-5-19");
        let directory = windows_ipc_directory_grants("S-1-5-19");
        assert_eq!(
            file.iter().filter(|grant| grant.starts_with("*S-1-5-19:")).count(),
            1
        );
        assert!(file.iter().any(|grant| grant == "*S-1-5-19:(R)"));
        assert_eq!(
            directory
                .iter()
                .filter(|grant| grant.starts_with("*S-1-5-19:"))
                .count(),
            1
        );
        assert!(directory
            .iter()
            .any(|grant| grant == "*S-1-5-19:(OI)(CI)(R)"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_ipc_ordinary_creator_retains_expected_rights() {
        let sid = "S-1-5-21-1000-2000-3000-4000";
        let file = windows_ipc_file_grants(sid);
        let directory = windows_ipc_directory_grants(sid);
        assert!(file.iter().any(|grant| grant == &format!("*{sid}:(R)")));
        assert!(directory
            .iter()
            .any(|grant| grant == &format!("*{sid}:(OI)(CI)(F)")));
    }
}
