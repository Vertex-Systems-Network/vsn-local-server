use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;

pub const CURRENT_CONFIG_VERSION:u32=3;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("unable to resolve VSN config directory")]
    NoConfigDir,
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("config JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid config value: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteConfig {
    pub enabled: bool,
    pub control_plane_url: Option<String>,
    pub control_plane_public_key: Option<String>,
    pub poll_interval_ms: u64,
    #[serde(default)] pub allow_remote_terminal: bool,
    #[serde(default)] pub allow_remote_file_write: bool,
    #[serde(default)] pub allow_remote_database_query: bool,
    #[serde(default)] pub allow_remote_preview_interactive: bool,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            control_plane_url: None,
            control_plane_public_key: None,
            poll_interval_ms: 2_500,
            allow_remote_terminal: false,
            allow_remote_file_write: false,
            allow_remote_database_query: false,
            allow_remote_preview_interactive: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub version: u32,
    pub workspace_roots: Vec<PathBuf>,
    pub default_domain_suffix: String,
    pub default_execution_backend: String,
    pub telemetry_enabled: bool,
    #[serde(default)]
    pub remote: RemoteConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CURRENT_CONFIG_VERSION,
            workspace_roots: vec![],
            default_domain_suffix: ".test".into(),
            default_execution_backend: "native".into(),
            telemetry_enabled: false,
            remote: RemoteConfig::default(),
        }
    }
}

pub fn default_path() -> Result<PathBuf, ConfigError> {
    let dirs = ProjectDirs::from("dev", "VSN", "VSN Platform").ok_or(ConfigError::NoConfigDir)?;
    Ok(dirs.config_dir().join("config.json"))
}

pub fn load_or_default() -> Result<AppConfig, ConfigError> {
    let path = default_path()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    recover_atomic_config(&path)?;
    let mut config: AppConfig = serde_json::from_slice(&fs::read(&path)?)?;
    if config.version < CURRENT_CONFIG_VERSION { config.version = CURRENT_CONFIG_VERSION; save_to(&path,&config)?; }
    validate(&config)?;
    Ok(config)
}

pub fn save(config: &AppConfig) -> Result<(), ConfigError> {
    validate(config)?;
    save_to(&default_path()?, config)
}

pub fn update_remote(remote: RemoteConfig) -> Result<AppConfig, ConfigError> {
    let mut config = load_or_default()?;
    config.remote = remote;
    config.version = CURRENT_CONFIG_VERSION;
    save(&config)?;
    Ok(config)
}


pub fn update_workspace_roots(roots: Vec<PathBuf>) -> Result<AppConfig, ConfigError> {
    let mut config = load_or_default()?;
    let mut normalized = Vec::new();
    for root in roots {
        if !root.is_dir() { return Err(ConfigError::Invalid(format!("workspace root is not a directory: {}", root.display()))); }
        let canonical = root.canonicalize()?;
        if !normalized.contains(&canonical) { normalized.push(canonical); }
    }
    config.workspace_roots = normalized;
    save(&config)?;
    Ok(config)
}

pub fn add_workspace_root(root: &Path) -> Result<AppConfig, ConfigError> {
    if !root.is_dir() { return Err(ConfigError::Invalid(format!("workspace root is not a directory: {}", root.display()))); }
    let canonical = root.canonicalize()?;
    let mut config = load_or_default()?;
    if !config.workspace_roots.iter().any(|v| v == &canonical) { config.workspace_roots.push(canonical); }
    save(&config)?;
    Ok(config)
}

pub fn remove_workspace_root(root: &Path) -> Result<AppConfig, ConfigError> {
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut config = load_or_default()?;
    config.workspace_roots.retain(|v| v != &canonical);
    save(&config)?;
    Ok(config)
}

pub fn save_to(path: &Path, config: &AppConfig) -> Result<(), ConfigError> {
    validate(config)?;if let Some(parent)=path.parent(){fs::create_dir_all(parent)?;}recover_atomic_config(path)?;let tmp=path.with_extension("json.tmp");let bak=path.with_extension("json.bak");let mut bytes=serde_json::to_vec_pretty(config)?;bytes.push(b'\n');{let mut file=fs::File::create(&tmp)?;file.write_all(&bytes)?;file.sync_all()?;}if bak.exists(){fs::remove_file(&bak)?;}if path.exists(){fs::rename(path,&bak)?;}if let Err(error)=fs::rename(&tmp,path){if bak.exists()&&!path.exists(){let _=fs::rename(&bak,path);}return Err(ConfigError::Io(error));}if let Some(parent)=path.parent(){sync_dir(parent)?;}if bak.exists(){fs::remove_file(&bak)?;}Ok(())
}
fn recover_atomic_config(path:&Path)->Result<(),ConfigError>{let tmp=path.with_extension("json.tmp");let bak=path.with_extension("json.bak");if path.exists(){if tmp.exists(){let _=fs::remove_file(&tmp);}return Ok(());}if tmp.exists(){if serde_json::from_slice::<AppConfig>(&fs::read(&tmp)?).is_ok(){fs::rename(&tmp,path)?;if let Some(parent)=path.parent(){sync_dir(parent)?;}return Ok(());}let _=fs::remove_file(&tmp);}if bak.exists(){fs::rename(&bak,path)?;if let Some(parent)=path.parent(){sync_dir(parent)?;}}Ok(())}
#[cfg(unix)]fn sync_dir(path:&Path)->Result<(),ConfigError>{fs::File::open(path)?.sync_all()?;Ok(())}
#[cfg(not(unix))]fn sync_dir(_path:&Path)->Result<(),ConfigError>{Ok(())}

fn validate(config: &AppConfig) -> Result<(), ConfigError> {
    if config.version!=CURRENT_CONFIG_VERSION{return Err(ConfigError::Invalid(format!("config version must be {CURRENT_CONFIG_VERSION}")));}
    if config.default_domain_suffix != ".test" {
        return Err(ConfigError::Invalid(
            "baseline local domain suffix must remain .test".into(),
        ));
    }
    if !matches!(config.default_execution_backend.as_str(), "native" | "container" | "remote") {
        return Err(ConfigError::Invalid(
            "default_execution_backend must be native, container, or remote".into(),
        ));
    }
    if !(500..=60_000).contains(&config.remote.poll_interval_ms) {
        return Err(ConfigError::Invalid(
            "remote poll_interval_ms must be between 500 and 60000".into(),
        ));
    }
    if config.remote.enabled {
        let url = config
            .remote
            .control_plane_url
            .as_deref()
            .ok_or_else(|| ConfigError::Invalid("remote control plane URL is required".into()))?;
        if !url.starts_with("https://") && !url.starts_with("http://127.0.0.1") && !url.starts_with("http://localhost") {
            return Err(ConfigError::Invalid(
                "remote control plane must use HTTPS except for loopback development".into(),
            ));
        }
        if config
            .remote
            .control_plane_public_key
            .as_deref()
            .filter(|v| !v.trim().is_empty())
            .is_none()
        {
            return Err(ConfigError::Invalid(
                "remote control plane public key is required".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_local_first_and_private() {
        let c = AppConfig::default();
        assert_eq!(c.default_execution_backend, "native");
        assert!(!c.telemetry_enabled);
        assert!(!c.remote.enabled);
    }

    #[test]
    fn current_config_version_is_stable(){assert_eq!(AppConfig::default().version,CURRENT_CONFIG_VERSION);}

    #[test]
    fn public_plain_http_remote_is_rejected() {
        let mut c = AppConfig::default();
        c.remote.enabled = true;
        c.remote.control_plane_url = Some("http://example.com".into());
        c.remote.control_plane_public_key = Some("abc".into());
        assert!(validate(&c).is_err());
    }
}
