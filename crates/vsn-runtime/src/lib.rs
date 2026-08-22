use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Mutex, MutexGuard, OnceLock},
    time::{Duration, Instant},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("runtime not found: {0}")]
    NotFound(String),
    #[error("runtime command failed: {0}")]
    Command(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("runtime metadata error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid runtime input: {0}")]
    Invalid(String),
    #[error("artifact checksum mismatch")]
    ChecksumMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeDescriptor {
    pub id: String,
    pub display_name: String,
    pub executables: Vec<String>,
    pub version_args: Vec<String>,
    pub project_markers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeDetection {
    pub id: String,
    pub installed: bool,
    pub executable: Option<String>,
    pub version: Option<String>,
}

fn unavailable_detection(runtime: &RuntimeDescriptor) -> RuntimeDetection {
    RuntimeDetection {
        id: runtime.id.clone(),
        installed: false,
        executable: None,
        version: None,
    }
}

fn detect_many(runtimes: Vec<RuntimeDescriptor>) -> Vec<RuntimeDetection> {
    let jobs = runtimes
        .into_iter()
        .map(|runtime| {
            let fallback = unavailable_detection(&runtime);
            let name = format!("vsn-runtime-probe-{}", runtime.id);
            let handle = std::thread::Builder::new()
                .name(name)
                .spawn(move || detect(&runtime));
            (fallback, handle)
        })
        .collect::<Vec<_>>();

    jobs.into_iter()
        .map(|(fallback, handle)| match handle {
            Ok(handle) => handle.join().unwrap_or(fallback),
            Err(_) => fallback,
        })
        .collect()
}

pub const RUNTIME_PROVIDER_SDK_VERSION: u32 = 1;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeProviderDescriptor {
    pub id: String,
    pub sdk_version: u32,
    pub runtime_ids: Vec<String>,
    pub supports_detection: bool,
    pub supports_project_markers: bool,
    pub supports_catalog_install: bool,
    pub supports_activation: bool,
    pub supports_repair: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeProviderConformanceReport {
    pub provider_id: String,
    pub sdk_version: u32,
    pub valid: bool,
    pub issues: Vec<String>,
    pub runtime_count: usize,
}

pub trait RuntimeProvider {
    fn descriptor(&self) -> RuntimeProviderDescriptor;
    fn runtimes(&self) -> Vec<RuntimeDescriptor>;
    fn detect_all(&self) -> Vec<RuntimeDetection> {
        detect_many(self.runtimes())
    }
    fn project_runtimes(&self, path: &Path) -> Vec<String> {
        self.runtimes()
            .into_iter()
            .filter(|r| r.project_markers.iter().any(|m| marker_matches(path, m)))
            .map(|r| r.id)
            .collect()
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct BuiltinRuntimeProvider;
impl RuntimeProvider for BuiltinRuntimeProvider {
    fn descriptor(&self) -> RuntimeProviderDescriptor {
        let runtimes = builtins();
        RuntimeProviderDescriptor {
            id: "vsn.builtin".into(),
            sdk_version: RUNTIME_PROVIDER_SDK_VERSION,
            runtime_ids: runtimes.iter().map(|r| r.id.clone()).collect(),
            supports_detection: true,
            supports_project_markers: true,
            supports_catalog_install: true,
            supports_activation: true,
            supports_repair: true,
        }
    }
    fn runtimes(&self) -> Vec<RuntimeDescriptor> {
        builtins()
    }
}
pub fn validate_provider_descriptor(
    d: &RuntimeProviderDescriptor,
) -> RuntimeProviderConformanceReport {
    let mut issues = Vec::new();
    if d.sdk_version != RUNTIME_PROVIDER_SDK_VERSION {
        issues.push(format!("unsupported SDK version {}", d.sdk_version));
    }
    if d.id.len() < 2
        || d.id.len() > 96
        || !d
            .id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        issues.push("provider id is invalid".into());
    }
    if d.runtime_ids.is_empty() {
        issues.push("provider exposes no runtimes".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for id in &d.runtime_ids {
        if validate_runtime_id(id).is_err() {
            issues.push(format!("invalid runtime id: {id}"));
        }
        if !seen.insert(id) {
            issues.push(format!("duplicate runtime id: {id}"));
        }
    }
    if !d.supports_detection {
        issues.push("runtime provider must support detection".into());
    }
    if !d.supports_catalog_install {
        issues.push("runtime provider must support catalog installation".into());
    }
    RuntimeProviderConformanceReport {
        provider_id: d.id.clone(),
        sdk_version: d.sdk_version,
        valid: issues.is_empty(),
        issues,
        runtime_count: d.runtime_ids.len(),
    }
}
pub fn builtin_provider_conformance() -> RuntimeProviderConformanceReport {
    validate_provider_descriptor(&BuiltinRuntimeProvider.descriptor())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeCatalog {
    pub schema_version: u32,
    pub provider: String,
    pub runtimes: Vec<RuntimeRelease>,
    #[serde(default)]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RuntimeCatalogTrust {
    pub public_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeRelease {
    pub runtime: String,
    pub version: String,
    pub artifacts: Vec<RuntimeArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeArtifact {
    pub os: String,
    pub arch: String,
    pub url: String,
    pub sha256: String,
    pub archive: String,
    pub executable_relpath: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeInstallPlan {
    pub runtime: String,
    pub version: String,
    pub target: String,
    pub url: String,
    pub sha256: String,
    pub archive: String,
    pub install_dir: PathBuf,
    pub executable_relpath: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledRuntime {
    pub runtime: String,
    pub version: String,
    pub install_dir: PathBuf,
    pub executable: PathBuf,
    pub source_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RuntimeRegistry {
    pub installed: Vec<InstalledRuntime>,
    pub project_activation: BTreeMap<String, BTreeMap<String, String>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeRepairReport {
    pub removed_missing: Vec<String>,
    pub fixed_executable_paths: Vec<String>,
    pub remaining_installed: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeAuditSeverity {
    Info,
    Warning,
    Error,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeAuditIssue {
    pub severity: RuntimeAuditSeverity,
    pub code: String,
    pub runtime: Option<String>,
    pub version: Option<String>,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeAuditReport {
    pub installed: usize,
    pub activations: usize,
    pub healthy: bool,
    pub issues: Vec<RuntimeAuditIssue>,
}
static RUNTIME_MUTATION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
fn runtime_guard() -> Result<MutexGuard<'static, ()>, RuntimeError> {
    RUNTIME_MUTATION_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| RuntimeError::Invalid("runtime mutation lock poisoned".into()))
}

pub fn builtins() -> Vec<RuntimeDescriptor> {
    vec![
        descriptor("php", "PHP", &["php"], &["--version"], &["composer.json"]),
        descriptor(
            "node",
            "Node.js",
            &["node"],
            &["--version"],
            &["package.json"],
        ),
        descriptor(
            "python",
            "Python",
            &["python", "python3"],
            &["--version"],
            &["pyproject.toml", "requirements.txt"],
        ),
        descriptor("go", "Go", &["go"], &["version"], &["go.mod"]),
        descriptor("rust", "Rust", &["rustc"], &["--version"], &["Cargo.toml"]),
        descriptor(
            "java",
            "Java",
            &["java"],
            &["--version"],
            &["pom.xml", "build.gradle", "build.gradle.kts"],
        ),
        descriptor(
            "dotnet",
            ".NET",
            &["dotnet"],
            &["--version"],
            &["*.csproj", "*.fsproj"],
        ),
        descriptor("ruby", "Ruby", &["ruby"], &["--version"], &["Gemfile"]),
        descriptor(
            "bun",
            "Bun",
            &["bun"],
            &["--version"],
            &["bun.lock", "bun.lockb"],
        ),
        descriptor(
            "deno",
            "Deno",
            &["deno"],
            &["--version"],
            &["deno.json", "deno.jsonc"],
        ),
    ]
}

fn descriptor(
    id: &str,
    name: &str,
    executables: &[&str],
    version_args: &[&str],
    markers: &[&str],
) -> RuntimeDescriptor {
    RuntimeDescriptor {
        id: id.into(),
        display_name: name.into(),
        executables: executables.iter().map(|v| (*v).into()).collect(),
        version_args: version_args.iter().map(|v| (*v).into()).collect(),
        project_markers: markers.iter().map(|v| (*v).into()).collect(),
    }
}

const RUNTIME_PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const RUNTIME_PROBE_CAPTURE_BYTES: usize = 64 * 1024;

fn read_probe_output<R: Read>(mut reader: R) -> std::io::Result<Vec<u8>> {
    let mut out = Vec::with_capacity(8 * 1024);
    let mut buffer = [0u8; 8 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        if out.len() < RUNTIME_PROBE_CAPTURE_BYTES {
            let remaining = RUNTIME_PROBE_CAPTURE_BYTES - out.len();
            out.extend_from_slice(&buffer[..read.min(remaining)]);
        }
    }
    Ok(out)
}

fn run_version_probe(executable: &str, args: &[String]) -> Option<String> {
    let mut child = Command::new(executable)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    let stderr = child.stderr.take()?;
    let stdout_thread = std::thread::spawn(move || read_probe_output(stdout));
    let stderr_thread = std::thread::spawn(move || read_probe_output(stderr));
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < RUNTIME_PROBE_TIMEOUT => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                // Do not join readers on a timed-out probe: a hostile descendant may have
                // inherited the pipes. Dropping the handles keeps inventory latency bounded.
                return None;
            }
        }
    };
    let stdout = stdout_thread.join().ok()?.ok()?;
    let stderr = stderr_thread.join().ok()?.ok()?;
    if !status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&stderr).trim().to_string();
    let version = if stdout.is_empty() { stderr } else { stdout };
    Some(version.lines().next().unwrap_or_default().to_string())
}

pub fn detect_all() -> Vec<RuntimeDetection> {
    detect_many(builtins())
}

pub fn detect(runtime: &RuntimeDescriptor) -> RuntimeDetection {
    for executable in &runtime.executables {
        if let Some(version) = run_version_probe(executable, &runtime.version_args) {
            return RuntimeDetection {
                id: runtime.id.clone(),
                installed: true,
                executable: Some(executable.clone()),
                version: Some(version),
            };
        }
    }
    unavailable_detection(runtime)
}

pub fn runtimes_for_project(path: &Path) -> Vec<String> {
    builtins()
        .into_iter()
        .filter(|runtime| {
            runtime
                .project_markers
                .iter()
                .any(|marker| marker_matches(path, marker))
        })
        .map(|runtime| runtime.id)
        .collect()
}

pub fn load_catalog(path: &Path) -> Result<RuntimeCatalog, RuntimeError> {
    let catalog: RuntimeCatalog = serde_json::from_slice(&fs::read(path)?)?;
    if catalog.schema_version != 1 || catalog.provider.trim().is_empty() {
        return Err(RuntimeError::Invalid(
            "unsupported or invalid runtime catalog".into(),
        ));
    }
    for release in &catalog.runtimes {
        validate_runtime_id(&release.runtime)?;
        validate_version(&release.version)?;
        if release.artifacts.is_empty() {
            return Err(RuntimeError::Invalid(format!(
                "{} {} has no artifacts",
                release.runtime, release.version
            )));
        }
    }
    Ok(catalog)
}

pub fn load_catalog_verified(
    path: &Path,
    trust_path: &Path,
) -> Result<(RuntimeCatalog, String), RuntimeError> {
    let catalog = load_catalog(path)?;
    let trust: RuntimeCatalogTrust = serde_json::from_slice(&fs::read(trust_path)?)?;
    let signature = catalog
        .signature
        .as_deref()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| RuntimeError::Invalid("runtime catalog is unsigned".into()))?;
    let mut unsigned = catalog.clone();
    unsigned.signature = None;
    let bytes = serde_json::to_vec(&unsigned)?;
    for key in &trust.public_keys {
        if vsn_security::verify_signature(key, &bytes, signature).is_ok() {
            return Ok((catalog, key.clone()));
        }
    }
    Err(RuntimeError::Invalid(
        "runtime catalog signature is not trusted".into(),
    ))
}

pub fn target_triple_parts() -> (String, String) {
    (
        normalize_os(std::env::consts::OS),
        normalize_arch(std::env::consts::ARCH),
    )
}

pub fn install_plan(
    catalog: &RuntimeCatalog,
    runtime: &str,
    version: &str,
    root: &Path,
) -> Result<RuntimeInstallPlan, RuntimeError> {
    validate_runtime_id(runtime)?;
    validate_version(version)?;
    let (os, arch) = target_triple_parts();
    let release = catalog
        .runtimes
        .iter()
        .find(|r| r.runtime == runtime && r.version == version)
        .ok_or_else(|| RuntimeError::NotFound(format!("{runtime}@{version}")))?;
    let artifact = release
        .artifacts
        .iter()
        .find(|a| normalize_os(&a.os) == os && normalize_arch(&a.arch) == arch)
        .ok_or_else(|| {
            RuntimeError::NotFound(format!("artifact for {runtime}@{version} on {os}/{arch}"))
        })?;
    if !artifact.url.starts_with("https://") && !artifact.url.starts_with("file://") {
        return Err(RuntimeError::Invalid(
            "runtime artifacts must use HTTPS or file://".into(),
        ));
    }
    if artifact.sha256.len() != 64 || !artifact.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(RuntimeError::Invalid(
            "artifact sha256 must be 64 hexadecimal characters".into(),
        ));
    }
    if artifact.executable_relpath.contains("..")
        || Path::new(&artifact.executable_relpath).is_absolute()
    {
        return Err(RuntimeError::Invalid(
            "runtime executable_relpath must stay inside install directory".into(),
        ));
    }
    Ok(RuntimeInstallPlan {
        runtime: runtime.into(),
        version: version.into(),
        target: format!("{os}/{arch}"),
        url: artifact.url.clone(),
        sha256: artifact.sha256.to_ascii_lowercase(),
        archive: artifact.archive.clone(),
        install_dir: root.join(runtime).join(version),
        executable_relpath: artifact.executable_relpath.clone(),
    })
}

pub fn download_artifact(
    plan: &RuntimeInstallPlan,
    cache_dir: &Path,
) -> Result<PathBuf, RuntimeError> {
    fs::create_dir_all(cache_dir)?;
    let ext = archive_extension(&plan.archive);
    let path = cache_dir.join(format!("{}-{}{}", plan.runtime, plan.version, ext));
    if let Some(source) = plan.url.strip_prefix("file://") {
        fs::copy(source, &path)?;
    } else {
        let status = Command::new("curl")
            .args([
                "--fail",
                "--location",
                "--proto",
                "=https",
                "--tlsv1.2",
                "--output",
            ])
            .arg(&path)
            .arg(&plan.url)
            .status()
            .map_err(|e| RuntimeError::Command(format!("curl unavailable: {e}")))?;
        if !status.success() {
            return Err(RuntimeError::Command(format!(
                "artifact download failed with status {status}"
            )));
        }
    }
    verify_sha256(&path, &plan.sha256)?;
    Ok(path)
}

pub fn install_from_artifact(
    plan: &RuntimeInstallPlan,
    artifact: &Path,
) -> Result<InstalledRuntime, RuntimeError> {
    let _guard = runtime_guard()?;
    verify_sha256(artifact, &plan.sha256)?;
    let parent = plan
        .install_dir
        .parent()
        .ok_or_else(|| RuntimeError::Invalid("runtime install directory has no parent".into()))?;
    fs::create_dir_all(parent)?;
    let name = plan
        .install_dir
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| RuntimeError::Invalid("runtime install directory name is invalid".into()))?;
    let staging = parent.join(format!(".{name}.staging"));
    let backup = parent.join(format!(".{name}.backup"));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    if backup.exists() && !plan.install_dir.exists() {
        fs::rename(&backup, &plan.install_dir)?;
    }
    if backup.exists() {
        fs::remove_dir_all(&backup)?;
    }
    fs::create_dir_all(&staging)?;
    if let Err(error) = extract_archive(artifact, &plan.archive, &staging, &plan.executable_relpath)
    {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    let staged_executable = staging.join(&plan.executable_relpath);
    if !staged_executable.is_file() {
        let _ = fs::remove_dir_all(&staging);
        return Err(RuntimeError::Invalid(format!(
            "installed executable missing: {}",
            staged_executable.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&staged_executable)?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(permissions.mode() | 0o111);
        fs::set_permissions(&staged_executable, permissions)?;
    }
    if plan.install_dir.exists() {
        fs::rename(&plan.install_dir, &backup)?;
    }
    if let Err(error) = fs::rename(&staging, &plan.install_dir) {
        if backup.exists() {
            let _ = fs::rename(&backup, &plan.install_dir);
        }
        return Err(RuntimeError::Io(error));
    }
    if backup.exists() {
        fs::remove_dir_all(&backup)?;
    }
    let executable = plan.install_dir.join(&plan.executable_relpath);
    Ok(InstalledRuntime {
        runtime: plan.runtime.clone(),
        version: plan.version.clone(),
        install_dir: plan.install_dir.clone(),
        executable,
        source_sha256: plan.sha256.clone(),
    })
}

pub fn load_registry(path: &Path) -> Result<RuntimeRegistry, RuntimeError> {
    if !path.exists() {
        return Ok(RuntimeRegistry::default());
    }
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

pub fn save_registry(path: &Path, registry: &RuntimeRegistry) -> Result<(), RuntimeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    let mut bytes = serde_json::to_vec_pretty(registry)?;
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
    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(&tmp, path)?;
    if let Some(parent) = path.parent() {
        if let Ok(dir) = fs::File::open(parent) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

pub fn register_runtime(
    path: &Path,
    installed: InstalledRuntime,
) -> Result<RuntimeRegistry, RuntimeError> {
    let mut registry = load_registry(path)?;
    registry
        .installed
        .retain(|r| !(r.runtime == installed.runtime && r.version == installed.version));
    registry.installed.push(installed);
    registry
        .installed
        .sort_by(|a, b| (&a.runtime, &a.version).cmp(&(&b.runtime, &b.version)));
    save_registry(path, &registry)?;
    Ok(registry)
}

pub fn uninstall_runtime(
    path: &Path,
    runtime: &str,
    version: &str,
) -> Result<RuntimeRegistry, RuntimeError> {
    let _guard = runtime_guard()?;
    validate_runtime_id(runtime)?;
    validate_version(version)?;
    let previous = load_registry(path)?;
    let installed = previous
        .installed
        .iter()
        .find(|r| r.runtime == runtime && r.version == version)
        .cloned()
        .ok_or_else(|| RuntimeError::NotFound(format!("installed {runtime}@{version}")))?;
    let tombstone = installed.install_dir.with_extension("vsn-removing");
    if tombstone.exists() && !installed.install_dir.exists() {
        fs::remove_dir_all(&tombstone)?;
    }
    if installed.install_dir.exists() {
        if tombstone.exists() {
            fs::remove_dir_all(&tombstone)?;
        }
        fs::rename(&installed.install_dir, &tombstone)?;
    }
    let mut registry = previous.clone();
    registry
        .installed
        .retain(|r| !(r.runtime == runtime && r.version == version));
    for active in registry.project_activation.values_mut() {
        if active.get(runtime).is_some_and(|v| v == version) {
            active.remove(runtime);
        }
    }
    registry
        .project_activation
        .retain(|_, active| !active.is_empty());
    if let Err(error) = save_registry(path, &registry) {
        if tombstone.exists() && !installed.install_dir.exists() {
            let _ = fs::rename(&tombstone, &installed.install_dir);
        }
        return Err(error);
    }
    if tombstone.exists() {
        fs::remove_dir_all(&tombstone)?;
    }
    Ok(registry)
}

pub fn audit_registry(path: &Path) -> Result<RuntimeAuditReport, RuntimeError> {
    let registry = load_registry(path)?;
    let runtime_root = path
        .parent()
        .ok_or_else(|| RuntimeError::Invalid("runtime registry has no managed root".into()))?;
    let canonical_runtime_root = if runtime_root.exists() {
        runtime_root.canonicalize()?
    } else {
        runtime_root.to_path_buf()
    };
    let provider_runtime_ids = builtins()
        .into_iter()
        .map(|runtime| runtime.id)
        .collect::<std::collections::HashSet<_>>();
    let mut issues = Vec::new();
    let mut known = std::collections::HashSet::new();
    let mut registrations = std::collections::HashSet::new();
    for item in &registry.installed {
        let key = (item.runtime.clone(), item.version.clone());
        if !registrations.insert(key.clone()) {
            issues.push(RuntimeAuditIssue {
                severity: RuntimeAuditSeverity::Error,
                code: "duplicate_registration".into(),
                runtime: Some(item.runtime.clone()),
                version: Some(item.version.clone()),
                message: "runtime registry contains a duplicate runtime/version registration".into(),
            });
        }
        known.insert(key);
        let runtime = Some(item.runtime.clone());
        let version = Some(item.version.clone());
        if validate_runtime_id(&item.runtime).is_err() || validate_version(&item.version).is_err() {
            issues.push(RuntimeAuditIssue {
                severity: RuntimeAuditSeverity::Error,
                code: "invalid_registration".into(),
                runtime: runtime.clone(),
                version: version.clone(),
                message: "runtime registry contains unsafe runtime/version metadata".into(),
            });
        }
        if !provider_runtime_ids.contains(&item.runtime) {
            issues.push(RuntimeAuditIssue {
                severity: RuntimeAuditSeverity::Error,
                code: "unknown_runtime".into(),
                runtime: runtime.clone(),
                version: version.clone(),
                message: "runtime registry references an ID not reported by the active provider".into(),
            });
        }
        if !item.install_dir.is_dir() {
            issues.push(RuntimeAuditIssue {
                severity: RuntimeAuditSeverity::Error,
                code: "missing_install_dir".into(),
                runtime: runtime.clone(),
                version: version.clone(),
                message: format!(
                    "install directory is missing: {}",
                    item.install_dir.display()
                ),
            });
            continue;
        }
        let canonical_dir = item.install_dir.canonicalize()?;
        if !canonical_dir.starts_with(&canonical_runtime_root) {
            issues.push(RuntimeAuditIssue {
                severity: RuntimeAuditSeverity::Error,
                code: "install_dir_escape".into(),
                runtime: runtime.clone(),
                version: version.clone(),
                message: "registered install directory escapes the VSN-managed runtime root".into(),
            });
        }
        if !item.executable.is_file() {
            issues.push(RuntimeAuditIssue {
                severity: RuntimeAuditSeverity::Error,
                code: "missing_executable".into(),
                runtime: runtime.clone(),
                version: version.clone(),
                message: format!(
                    "runtime executable is missing: {}",
                    item.executable.display()
                ),
            });
        } else {
            let executable = item.executable.canonicalize()?;
            if !executable.starts_with(&canonical_dir) {
                issues.push(RuntimeAuditIssue {
                    severity: RuntimeAuditSeverity::Error,
                    code: "executable_path_escape".into(),
                    runtime: runtime.clone(),
                    version: version.clone(),
                    message: "registered executable escapes install directory".into(),
                });
            }
        }
        if item.source_sha256.len() != 64
            || !item.source_sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            issues.push(RuntimeAuditIssue {
                severity: RuntimeAuditSeverity::Warning,
                code: "invalid_source_digest".into(),
                runtime: runtime.clone(),
                version: version.clone(),
                message: "installed runtime source SHA-256 metadata is invalid".into(),
            });
        }
    }
    let mut activations = 0usize;
    for (project, map) in &registry.project_activation {
        for (runtime, version) in map {
            activations += 1;
            if !known.contains(&(runtime.clone(), version.clone())) {
                issues.push(RuntimeAuditIssue {
                    severity: RuntimeAuditSeverity::Error,
                    code: "dangling_activation".into(),
                    runtime: Some(runtime.clone()),
                    version: Some(version.clone()),
                    message: format!("project activation references missing runtime: {project}"),
                });
            }
        }
    }
    let healthy = !issues
        .iter()
        .any(|i| matches!(i.severity, RuntimeAuditSeverity::Error));
    Ok(RuntimeAuditReport {
        installed: registry.installed.len(),
        activations,
        healthy,
        issues,
    })
}

pub fn repair_registry(path: &Path) -> Result<RuntimeRepairReport, RuntimeError> {
    let _guard = runtime_guard()?;
    let mut registry = load_registry(path)?;
    let mut removed = Vec::new();
    let mut fixed = Vec::new();
    registry.installed.retain_mut(|item| {
        if !item.install_dir.is_dir() {
            removed.push(format!("{}@{}", item.runtime, item.version));
            return false;
        }
        if !item.executable.is_file() {
            let candidate = item
                .install_dir
                .join(item.executable.file_name().unwrap_or_default());
            if candidate.is_file() {
                item.executable = candidate;
                fixed.push(format!("{}@{}", item.runtime, item.version));
                return true;
            }
            removed.push(format!("{}@{}", item.runtime, item.version));
            return false;
        }
        true
    });
    let valid = registry
        .installed
        .iter()
        .map(|r| (r.runtime.clone(), r.version.clone()))
        .collect::<std::collections::HashSet<_>>();
    for active in registry.project_activation.values_mut() {
        active.retain(|runtime, version| valid.contains(&(runtime.clone(), version.clone())));
    }
    registry.project_activation.retain(|_, v| !v.is_empty());
    save_registry(path, &registry)?;
    Ok(RuntimeRepairReport {
        removed_missing: removed,
        fixed_executable_paths: fixed,
        remaining_installed: registry.installed.len(),
    })
}

pub fn activate_for_project(
    path: &Path,
    project: &Path,
    runtime: &str,
    version: &str,
) -> Result<RuntimeRegistry, RuntimeError> {
    validate_runtime_id(runtime)?;
    validate_version(version)?;
    let mut registry = load_registry(path)?;
    if !registry
        .installed
        .iter()
        .any(|r| r.runtime == runtime && r.version == version)
    {
        return Err(RuntimeError::NotFound(format!(
            "installed {runtime}@{version}"
        )));
    }
    let key = project
        .canonicalize()
        .unwrap_or_else(|_| project.to_path_buf())
        .to_string_lossy()
        .to_string();
    registry
        .project_activation
        .entry(key)
        .or_default()
        .insert(runtime.into(), version.into());
    save_registry(path, &registry)?;
    Ok(registry)
}

pub fn write_shim(shim_dir: &Path, name: &str, executable: &Path) -> Result<PathBuf, RuntimeError> {
    validate_runtime_id(name)?;
    fs::create_dir_all(shim_dir)?;
    #[cfg(windows)]
    {
        let path = shim_dir.join(format!("{name}.cmd"));
        let tmp = shim_dir.join(format!(".{name}.cmd.tmp"));
        fs::write(
            &tmp,
            format!("@echo off\r\n\"{}\" %*\r\n", executable.display()),
        )?;
        if path.exists() {
            fs::remove_file(&path)?;
        }
        fs::rename(tmp, &path)?;
        Ok(path)
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let path = shim_dir.join(name);
        let tmp = shim_dir.join(format!(".{name}.tmp"));
        fs::write(
            &tmp,
            format!("#!/bin/sh\nexec \"{}\" \"$@\"\n", executable.display()),
        )?;
        let mut perm = fs::metadata(&tmp)?.permissions();
        perm.set_mode(0o755);
        fs::set_permissions(&tmp, perm)?;
        fs::rename(tmp, &path)?;
        Ok(path)
    }
}

pub fn verify_sha256(path: &Path, expected: &str) -> Result<(), RuntimeError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(RuntimeError::ChecksumMismatch)
    }
}

fn extract_archive(
    artifact: &Path,
    archive: &str,
    destination: &Path,
    executable_relpath: &str,
) -> Result<(), RuntimeError> {
    if archive != "binary" {
        validate_archive_before_extract(artifact, archive)?;
    }
    let mut command = if archive == "zip" {
        #[cfg(windows)]
        {
            let mut c = Command::new("powershell.exe");
            c.args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Expand-Archive",
                "-LiteralPath",
            ])
            .arg(artifact)
            .args(["-DestinationPath"])
            .arg(destination)
            .arg("-Force");
            c
        }
        #[cfg(not(windows))]
        {
            let mut c = Command::new("unzip");
            c.arg("-q").arg(artifact).arg("-d").arg(destination);
            c
        }
    } else if matches!(archive, "tar.gz" | "tgz" | "tar.xz" | "tar") {
        let mut c = Command::new("tar");
        c.arg("-xf").arg(artifact).arg("-C").arg(destination);
        c
    } else if archive == "binary" {
        let target = destination.join(executable_relpath);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(artifact, target)?;
        return Ok(());
    } else {
        return Err(RuntimeError::Invalid(format!(
            "unsupported archive type: {archive}"
        )));
    };
    let output = command
        .output()
        .map_err(|e| RuntimeError::Command(e.to_string()))?;
    if !output.status.success() {
        return Err(RuntimeError::Command(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    reject_extracted_symlinks(destination)?;
    Ok(())
}

fn validate_archive_before_extract(artifact: &Path, archive: &str) -> Result<(), RuntimeError> {
    let names = if archive == "zip" {
        #[cfg(windows)]
        {
            let script = r#"& { param($p) Add-Type -AssemblyName System.IO.Compression.FileSystem; $z=[System.IO.Compression.ZipFile]::OpenRead($p); try { foreach($e in $z.Entries) { $mode=($e.ExternalAttributes -shr 16) -band 0xF000; if($mode -eq 0xA000){ Write-Output ('SYMLINK:'+$e.FullName) } else { Write-Output $e.FullName } } } finally { $z.Dispose() } }"#;
            let out = Command::new("powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-Command", script])
                .arg(artifact)
                .output()
                .map_err(|e| {
                    RuntimeError::Command(format!("PowerShell archive inspection unavailable: {e}"))
                })?;
            if !out.status.success() {
                return Err(RuntimeError::Command(format!(
                    "zip inspection failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                )));
            }
            String::from_utf8_lossy(&out.stdout).into_owned()
        }
        #[cfg(not(windows))]
        {
            let out = Command::new("unzip")
                .arg("-Z1")
                .arg(artifact)
                .output()
                .map_err(|e| RuntimeError::Command(format!("unzip inspection unavailable: {e}")))?;
            if !out.status.success() {
                return Err(RuntimeError::Command(format!(
                    "zip inspection failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                )));
            }
            let verbose = Command::new("unzip")
                .args(["-Z", "-l"])
                .arg(artifact)
                .output()
                .map_err(|e| {
                    RuntimeError::Command(format!("unzip symlink inspection unavailable: {e}"))
                })?;
            if verbose.status.success()
                && String::from_utf8_lossy(&verbose.stdout)
                    .lines()
                    .any(|line| line.trim_start().starts_with('l'))
            {
                return Err(RuntimeError::Invalid(
                    "runtime zip may not contain symbolic links".into(),
                ));
            }
            String::from_utf8_lossy(&out.stdout).into_owned()
        }
    } else if matches!(archive, "tar.gz" | "tgz" | "tar.xz" | "tar") {
        let out = Command::new("tar")
            .arg("-tf")
            .arg(artifact)
            .output()
            .map_err(|e| RuntimeError::Command(format!("tar inspection unavailable: {e}")))?;
        if !out.status.success() {
            return Err(RuntimeError::Command(format!(
                "tar inspection failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        let verbose = Command::new("tar")
            .arg("-tvf")
            .arg(artifact)
            .output()
            .map_err(|e| RuntimeError::Command(format!("tar type inspection unavailable: {e}")))?;
        if !verbose.status.success() {
            return Err(RuntimeError::Command(format!(
                "tar type inspection failed: {}",
                String::from_utf8_lossy(&verbose.stderr).trim()
            )));
        }
        if String::from_utf8_lossy(&verbose.stdout)
            .lines()
            .any(|line| matches!(line.trim_start().chars().next(), Some('l' | 'h')))
        {
            return Err(RuntimeError::Invalid(
                "runtime tar may not contain symbolic or hard links".into(),
            ));
        }
        String::from_utf8_lossy(&out.stdout).into_owned()
    } else {
        return Err(RuntimeError::Invalid(format!(
            "unsupported archive type: {archive}"
        )));
    };
    for line in names.lines() {
        let name = line.strip_prefix("SYMLINK:").unwrap_or(line).trim();
        if line.starts_with("SYMLINK:") {
            return Err(RuntimeError::Invalid(
                "runtime archive may not contain symbolic links".into(),
            ));
        }
        validate_archive_entry(name)?;
    }
    Ok(())
}
fn validate_archive_entry(name: &str) -> Result<(), RuntimeError> {
    let normalized = name.replace('\\', "/");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.starts_with("//")
        || normalized.as_bytes().get(1) == Some(&b':')
    {
        return Err(RuntimeError::Invalid(format!(
            "unsafe archive entry: {name}"
        )));
    }
    if normalized.split('/').any(|part| part == "..") {
        return Err(RuntimeError::Invalid(format!(
            "archive entry escapes destination: {name}"
        )));
    }
    Ok(())
}
fn reject_extracted_symlinks(root: &Path) -> Result<(), RuntimeError> {
    fn walk(path: &Path) -> Result<(), RuntimeError> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let meta = fs::symlink_metadata(entry.path())?;
            if meta.file_type().is_symlink() {
                return Err(RuntimeError::Invalid(format!(
                    "runtime extraction produced a symlink: {}",
                    entry.path().display()
                )));
            }
            if meta.is_dir() {
                walk(&entry.path())?;
            }
        }
        Ok(())
    }
    walk(root)
}

fn archive_extension(kind: &str) -> &'static str {
    match kind {
        "zip" => ".zip",
        "tar.gz" | "tgz" => ".tar.gz",
        "tar.xz" => ".tar.xz",
        "tar" => ".tar",
        _ => ".bin",
    }
}

fn marker_matches(path: &Path, marker: &str) -> bool {
    if !marker.contains('*') {
        return path.join(marker).exists();
    }
    let suffix = marker.trim_start_matches('*');
    fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| entry.file_name().to_string_lossy().ends_with(suffix))
}

fn validate_runtime_id(value: &str) -> Result<(), RuntimeError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_'))
    {
        return Err(RuntimeError::Invalid("unsafe runtime id".into()));
    }
    Ok(())
}

fn validate_version(value: &str) -> Result<(), RuntimeError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+'))
    {
        return Err(RuntimeError::Invalid("unsafe runtime version".into()));
    }
    Ok(())
}

fn normalize_os(value: &str) -> String {
    match value.to_ascii_lowercase().as_str() {
        "win32" | "windows" => "windows".into(),
        "darwin" | "macos" => "macos".into(),
        "linux" => "linux".into(),
        other => other.into(),
    }
}

fn normalize_arch(value: &str) -> String {
    match value.to_ascii_lowercase().as_str() {
        "x86_64" | "amd64" => "x86_64".into(),
        "aarch64" | "arm64" => "aarch64".into(),
        other => other.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_ids_are_unique() {
        let mut ids: Vec<_> = builtins().into_iter().map(|v| v.id).collect();
        let len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(len, ids.len());
    }

    #[test]
    fn parallel_detection_preserves_provider_order() {
        let runtimes = vec![
            descriptor(
                "missing-a",
                "Missing A",
                &["__vsn_missing_runtime_a__"],
                &["--version"],
                &[],
            ),
            descriptor(
                "missing-b",
                "Missing B",
                &["__vsn_missing_runtime_b__"],
                &["--version"],
                &[],
            ),
        ];
        let detections = detect_many(runtimes);
        assert_eq!(
            detections.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(),
            vec!["missing-a", "missing-b"]
        );
        assert!(detections.iter().all(|item| !item.installed));
    }

    #[test]
    fn uninstall_cleans_registry_and_project_activation() {
        let root = std::env::temp_dir().join(format!("vsn-runtime-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let registry_path = root.join("registry.json");
        let install_dir = root.join("fake").join("1.0");
        fs::create_dir_all(&install_dir).unwrap();
        let exe = install_dir.join("fake");
        fs::write(&exe, b"x").unwrap();
        register_runtime(
            &registry_path,
            InstalledRuntime {
                runtime: "fake".into(),
                version: "1.0".into(),
                install_dir: install_dir.clone(),
                executable: exe,
                source_sha256: "0".repeat(64),
            },
        )
        .unwrap();
        activate_for_project(&registry_path, &root, "fake", "1.0").unwrap();
        let registry = uninstall_runtime(&registry_path, "fake", "1.0").unwrap();
        assert!(registry.installed.is_empty());
        assert!(registry.project_activation.is_empty());
        assert!(!install_dir.exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn audit_empty_missing_registry_is_healthy() {
        let root = std::env::temp_dir().join(format!(
            "vsn-runtime-empty-audit-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let audit = audit_registry(&root.join("runtimes").join("registry.json")).unwrap();
        assert!(audit.healthy);
        assert_eq!(audit.installed, 0);
        assert_eq!(audit.activations, 0);
        assert!(audit.issues.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn audit_flags_duplicate_unknown_and_install_root_escape() {
        let root = std::env::temp_dir().join(format!(
            "vsn-runtime-audit-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let runtime_root = root.join("runtimes");
        let outside = root.join("outside");
        fs::create_dir_all(&runtime_root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let executable = outside.join("evil");
        fs::write(&executable, b"x").unwrap();
        let registry_path = runtime_root.join("registry.json");
        let node = InstalledRuntime {
            runtime: "node".into(),
            version: "20.0.0".into(),
            install_dir: outside.clone(),
            executable: executable.clone(),
            source_sha256: "0".repeat(64),
        };
        let registry = RuntimeRegistry {
            installed: vec![
                node.clone(),
                node,
                InstalledRuntime {
                    runtime: "unknown-runtime".into(),
                    version: "1.0.0".into(),
                    install_dir: outside.clone(),
                    executable,
                    source_sha256: "0".repeat(64),
                },
            ],
            project_activation: BTreeMap::from([(
                root.join("project").display().to_string(),
                BTreeMap::from([("missing-runtime".into(), "9.9.9".into())]),
            )]),
        };
        save_registry(&registry_path, &registry).unwrap();
        let audit = audit_registry(&registry_path).unwrap();
        assert!(!audit.healthy);
        let codes = audit
            .issues
            .iter()
            .map(|issue| issue.code.as_str())
            .collect::<std::collections::HashSet<_>>();
        assert!(codes.contains("duplicate_registration"));
        assert!(codes.contains("unknown_runtime"));
        assert!(codes.contains("install_dir_escape"));
        assert!(codes.contains("dangling_activation"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_archive_path_traversal() {
        assert!(validate_archive_entry("../evil").is_err());
        assert!(validate_archive_entry("safe/bin/tool").is_ok());
        assert!(validate_archive_entry("/absolute/tool").is_err());
        assert!(validate_archive_entry("C:/evil.exe").is_err());
    }

    #[test]
    fn rejects_insecure_catalog_urls() {
        let c = RuntimeCatalog {
            schema_version: 1,
            provider: "t".into(),
            runtimes: vec![RuntimeRelease {
                runtime: "php".into(),
                version: "8.4.0".into(),
                artifacts: vec![RuntimeArtifact {
                    os: std::env::consts::OS.into(),
                    arch: std::env::consts::ARCH.into(),
                    url: "http://example.com/a.zip".into(),
                    sha256: "0".repeat(64),
                    archive: "zip".into(),
                    executable_relpath: "php".into(),
                }],
            }],
            signature: None,
        };
        assert!(install_plan(&c, "php", "8.4.0", Path::new("/tmp")).is_err());
    }
}
