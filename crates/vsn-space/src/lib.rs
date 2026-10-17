//! VSN Devify spaces (DVF-01): a project folder plus a `.devify/space.toml` that pins
//! runtime, package-manager and service versions for that project.
//!
//! An **isolated** space resolves its tools from the shared VSN runtime store and keeps
//! its own caches, data and logs under `.devify/`, so two spaces never conflict even on
//! different versions. A **global** space uses whatever is installed on the machine.
//! A **container** space is rendered to Compose by the container layer.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const SPACE_DIR: &str = ".devify";
pub const SPACE_FILE: &str = "space.toml";
pub const SPACE_FORMAT_VERSION: u32 = 1;
/// Per-space folders created by `init`; all are git-ignored except `space.toml` and `api/`.
pub const SPACE_SUBDIRS: [&str; 4] = ["data", "cache", "logs", "api"];

#[derive(Debug, thiserror::Error)]
pub enum SpaceError {
    #[error("invalid space: {0}")]
    Invalid(String),
    #[error("space io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("space.toml could not be read: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("space.toml could not be written: {0}")]
    Write(#[from] toml::ser::Error),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SpaceMode {
    #[default]
    Isolated,
    Global,
    Container,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceSpec {
    /// mysql, mariadb, postgres, redis, memcached, mongodb, mailpit …
    pub kind: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpaceSpec {
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub mode: SpaceMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Tool id → version, e.g. php = "8.3", node = "22", pnpm = "9".
    #[serde(default)]
    pub tools: BTreeMap<String, String>,
    #[serde(default)]
    pub services: Vec<ServiceSpec>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

impl SpaceSpec {
    pub fn new(name: &str, mode: SpaceMode) -> Self {
        Self {
            version: SPACE_FORMAT_VERSION,
            name: name.to_string(),
            mode,
            domain: None,
            tools: BTreeMap::new(),
            services: Vec::new(),
            env: BTreeMap::new(),
        }
    }
}

fn valid_id(s: &str, max: usize) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// Reject anything that could escape the space or confuse the shell/proxy layers.
pub fn validate(spec: &SpaceSpec) -> Result<(), SpaceError> {
    if spec.version != SPACE_FORMAT_VERSION {
        return Err(SpaceError::Invalid(format!(
            "unsupported space format version {}",
            spec.version
        )));
    }
    if !valid_id(&spec.name, 64) || spec.name.starts_with('.') {
        return Err(SpaceError::Invalid(
            "name must be 1-64 letters, digits, '-', '_' or '.'".into(),
        ));
    }
    if let Some(domain) = &spec.domain {
        let ok = domain.len() <= 253
            && domain.contains('.')
            && domain.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && label
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            });
        if !ok {
            return Err(SpaceError::Invalid(format!("invalid domain {domain}")));
        }
    }
    for (tool, version) in &spec.tools {
        if !valid_id(tool, 32) || !valid_id(version, 32) {
            return Err(SpaceError::Invalid(format!(
                "invalid tool pin {tool}={version}"
            )));
        }
    }
    let mut ports = BTreeSet::new();
    for service in &spec.services {
        if !valid_id(&service.kind, 32) || !valid_id(&service.version, 32) {
            return Err(SpaceError::Invalid(format!(
                "invalid service {}={}",
                service.kind, service.version
            )));
        }
        if let Some(port) = service.port {
            if port < 1024 || !ports.insert(port) {
                return Err(SpaceError::Invalid(format!(
                    "service port {port} must be >= 1024 and unique"
                )));
            }
        }
    }
    for (key, value) in &spec.env {
        let key_ok = !key.is_empty()
            && key.len() <= 128
            && key
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
            && !key.as_bytes()[0].is_ascii_digit();
        if !key_ok || value.contains('\0') || value.len() > 4096 {
            return Err(SpaceError::Invalid(format!("invalid env var {key}")));
        }
        if matches!(key.as_str(), "PATH" | "PATHEXT" | "SYSTEMROOT" | "COMSPEC") {
            return Err(SpaceError::Invalid(format!(
                "{key} is managed by VSN and cannot be overridden"
            )));
        }
    }
    Ok(())
}

pub fn space_file(project: &Path) -> PathBuf {
    project.join(SPACE_DIR).join(SPACE_FILE)
}

pub fn parse(text: &str) -> Result<SpaceSpec, SpaceError> {
    let spec: SpaceSpec = toml::from_str(text)?;
    validate(&spec)?;
    Ok(spec)
}

pub fn render(spec: &SpaceSpec) -> Result<String, SpaceError> {
    validate(spec)?;
    Ok(format!(
        "# VSN Devify space. Edit in the app or by hand; VSN validates it on load.\n{}",
        toml::to_string_pretty(spec)?
    ))
}

pub fn load(project: &Path) -> Result<SpaceSpec, SpaceError> {
    parse(&fs::read_to_string(space_file(project))?)
}

pub fn exists(project: &Path) -> bool {
    space_file(project).is_file()
}

/// Write `space.toml` and create the per-space folders. Never touches project code.
pub fn init(project: &Path, spec: &SpaceSpec) -> Result<(), SpaceError> {
    if !project.is_dir() {
        return Err(SpaceError::Invalid(format!(
            "project folder {} does not exist",
            project.display()
        )));
    }
    let text = render(spec)?;
    let dir = project.join(SPACE_DIR);
    fs::create_dir_all(&dir)?;
    for sub in SPACE_SUBDIRS {
        fs::create_dir_all(dir.join(sub))?;
    }
    let ignore = dir.join(".gitignore");
    if !ignore.exists() {
        fs::write(&ignore, "data/\ncache/\nlogs/\n")?;
    }
    let tmp = dir.join("space.toml.tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, space_file(project))?;
    Ok(())
}

/// A runtime available in the shared store (mirrors `vsn_runtime::InstalledRuntime`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvailableTool {
    pub tool: String,
    pub version: String,
    pub executable: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MissingTool {
    pub tool: String,
    pub version: String,
}

/// Process environment for commands run inside a space.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SpaceEnv {
    /// Directories to put in front of PATH, in order.
    pub path_prepend: Vec<PathBuf>,
    pub vars: BTreeMap<String, String>,
    /// Pinned tools that are not installed yet (Devify downloads them).
    pub missing: Vec<MissingTool>,
}

/// `pin` matches an installed version when equal or a dotted prefix ("8.3" → "8.3.12").
pub fn version_matches(pin: &str, installed: &str) -> bool {
    installed == pin
        || installed
            .strip_prefix(pin)
            .is_some_and(|rest| rest.starts_with('.'))
}

/// Highest installed version that satisfies the pin.
pub fn pick<'a>(
    pin_tool: &str,
    pin: &str,
    available: &'a [AvailableTool],
) -> Option<&'a AvailableTool> {
    available
        .iter()
        .filter(|t| t.tool == pin_tool && version_matches(pin, &t.version))
        .max_by(|a, b| compare_versions(&a.version, &b.version))
}

fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parts = |v: &str| -> Vec<u64> { v.split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    parts(a).cmp(&parts(b))
}

pub fn resolve_env(project: &Path, spec: &SpaceSpec, available: &[AvailableTool]) -> SpaceEnv {
    let mut env = SpaceEnv::default();
    if spec.mode == SpaceMode::Isolated {
        for (tool, pin) in &spec.tools {
            match pick(tool, pin, available) {
                Some(found) => {
                    if let Some(dir) = found.executable.parent() {
                        let dir = dir.to_path_buf();
                        if !env.path_prepend.contains(&dir) {
                            env.path_prepend.push(dir);
                        }
                    }
                }
                None => env.missing.push(MissingTool {
                    tool: tool.clone(),
                    version: pin.clone(),
                }),
            }
        }
        // Package-manager caches live inside the space so versions never leak between spaces.
        let cache = project.join(SPACE_DIR).join("cache");
        let set = |env: &mut SpaceEnv, key: &str, sub: &str| {
            env.vars
                .insert(key.into(), cache.join(sub).to_string_lossy().into_owned());
        };
        set(&mut env, "npm_config_cache", "npm");
        set(&mut env, "PNPM_HOME", "pnpm");
        set(&mut env, "YARN_CACHE_FOLDER", "yarn");
        set(&mut env, "COMPOSER_HOME", "composer");
        set(&mut env, "PIP_CACHE_DIR", "pip");
        set(&mut env, "GOMODCACHE", "go-mod");
        set(&mut env, "CARGO_HOME", "cargo");
    }
    env.vars.insert("DEVIFY_SPACE".into(), spec.name.clone());
    env.vars.insert(
        "DEVIFY_SPACE_ROOT".into(),
        project.to_string_lossy().into_owned(),
    );
    for (k, v) in &spec.env {
        env.vars.insert(k.clone(), v.clone());
    }
    env
}

/// Conventional base port per service kind; Devify allocates upward from here.
pub fn default_port(kind: &str) -> u16 {
    match kind {
        "mysql" | "mariadb" => 33060,
        "postgres" => 54320,
        "redis" | "valkey" => 63790,
        "memcached" => 11220,
        "mongodb" => 27020,
        "mailpit" => 10250,
        _ => 20000,
    }
}

/// Give every service without a port a free one, skipping ports already used on the
/// machine or by other spaces. Returns the services with ports filled in.
pub fn allocate_ports(spec: &SpaceSpec, used: &BTreeSet<u16>) -> Vec<ServiceSpec> {
    let mut taken: BTreeSet<u16> = used.clone();
    taken.extend(spec.services.iter().filter_map(|s| s.port));
    spec.services
        .iter()
        .map(|s| {
            let mut s = s.clone();
            if s.port.is_none() {
                let mut port = default_port(&s.kind);
                while taken.contains(&port) && port < u16::MAX {
                    port += 1;
                }
                taken.insert(port);
                s.port = Some(port);
            }
            s
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SpaceSpec {
        let mut s = SpaceSpec::new("shop", SpaceMode::Isolated);
        s.domain = Some("shop.test".into());
        s.tools.insert("php".into(), "8.3".into());
        s.tools.insert("node".into(), "22".into());
        s.services.push(ServiceSpec {
            kind: "mysql".into(),
            version: "8.4".into(),
            port: None,
        });
        s.env.insert("APP_ENV".into(), "local".into());
        s
    }

    #[test]
    fn render_parse_round_trip() {
        let spec = sample();
        let text = render(&spec).unwrap();
        assert!(text.contains("mode = \"isolated\""));
        assert_eq!(parse(&text).unwrap(), spec);
    }

    #[test]
    fn rejects_unsafe_values() {
        let mut s = sample();
        s.name = "../etc".into();
        assert!(validate(&s).is_err());
        let mut s = sample();
        s.domain = Some("Shop.TEST".into());
        assert!(validate(&s).is_err());
        let mut s = sample();
        s.env.insert("PATH".into(), "x".into());
        assert!(validate(&s).is_err());
        let mut s = sample();
        s.tools.insert("php".into(), "8.3; rm -rf /".into());
        assert!(validate(&s).is_err());
        let mut s = sample();
        s.services.push(ServiceSpec {
            kind: "redis".into(),
            version: "7".into(),
            port: Some(80),
        });
        assert!(validate(&s).is_err());
    }

    #[test]
    fn version_pin_matching() {
        assert!(version_matches("8.3", "8.3.12"));
        assert!(version_matches("8.3.12", "8.3.12"));
        assert!(!version_matches("8.3", "8.30.1"));
        assert!(!version_matches("22", "2.2"));
    }

    #[test]
    fn isolated_env_picks_highest_match_and_reports_missing() {
        let project = Path::new("/w/shop");
        let available = vec![
            AvailableTool {
                tool: "php".into(),
                version: "8.3.4".into(),
                executable: "/s/php/8.3.4/php".into(),
            },
            AvailableTool {
                tool: "php".into(),
                version: "8.3.12".into(),
                executable: "/s/php/8.3.12/php".into(),
            },
            AvailableTool {
                tool: "php".into(),
                version: "8.4.1".into(),
                executable: "/s/php/8.4.1/php".into(),
            },
        ];
        let env = resolve_env(project, &sample(), &available);
        assert_eq!(env.path_prepend, vec![PathBuf::from("/s/php/8.3.12")]);
        assert_eq!(
            env.missing,
            vec![MissingTool {
                tool: "node".into(),
                version: "22".into()
            }]
        );
        assert!(env.vars["npm_config_cache"].ends_with("npm"));
        assert_eq!(env.vars["APP_ENV"], "local");
        assert_eq!(env.vars["DEVIFY_SPACE"], "shop");
    }

    #[test]
    fn global_env_does_not_touch_path_or_caches() {
        let mut spec = sample();
        spec.mode = SpaceMode::Global;
        let env = resolve_env(Path::new("/w/shop"), &spec, &[]);
        assert!(env.path_prepend.is_empty() && env.missing.is_empty());
        assert!(!env.vars.contains_key("npm_config_cache"));
    }

    #[test]
    fn ports_are_allocated_without_collisions() {
        let mut spec = sample();
        spec.services.push(ServiceSpec {
            kind: "mysql".into(),
            version: "5.7".into(),
            port: None,
        });
        let used: BTreeSet<u16> = [33060].into_iter().collect();
        let got = allocate_ports(&spec, &used);
        assert_eq!(got[0].port, Some(33061));
        assert_eq!(got[1].port, Some(33062));
    }

    #[test]
    fn init_writes_space_and_folders_only() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path(), &sample()).unwrap();
        assert!(exists(dir.path()));
        for sub in SPACE_SUBDIRS {
            assert!(dir.path().join(SPACE_DIR).join(sub).is_dir());
        }
        assert_eq!(load(dir.path()).unwrap(), sample());
        assert!(init(&dir.path().join("missing"), &sample()).is_err());
    }
}
