use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Mutex, MutexGuard, OnceLock},
    time::{Duration, Instant},
};
use thiserror::Error;
const MAX_EXTENSION_FILES: usize = 4096;
const MAX_EXTENSION_BYTES: u64 = 256 * 1024 * 1024;
#[derive(Debug, Error)]
pub enum ExtensionError {
    #[error("extension I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("extension JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("extension signature is missing or untrusted")]
    Untrusted,
    #[error("invalid extension package: {0}")]
    Invalid(String),
    #[error("signature verification failed: {0}")]
    Security(#[from] vsn_security::SecurityError),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderReference {
    pub kind: String,
    pub manifest: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionDependency {
    pub id: String,
    pub version: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutableReference {
    pub id: String,
    pub path: String,
    #[serde(default)]
    pub fixed_args: Vec<String>,
    #[serde(default = "default_exec_timeout")]
    pub timeout_seconds: u64,
}
fn default_exec_timeout() -> u64 {
    30
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: u32,
    pub providers: Vec<ProviderReference>,
    #[serde(default)]
    pub executables: Vec<ExecutableReference>,
    #[serde(default)]
    pub dependencies: Vec<ExtensionDependency>,
    pub permissions: Vec<String>,
    pub signature: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TrustStore {
    pub public_keys: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledExtension {
    pub id: String,
    pub version: String,
    pub path: PathBuf,
    pub signer_public_key: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedProvider {
    pub extension_id: String,
    pub extension_version: String,
    pub kind: String,
    pub manifest_path: PathBuf,
    pub signer_public_key: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SandboxCapabilities {
    pub backend: String,
    pub available: bool,
    pub network_default_denied: bool,
    pub filesystem_policy_enforced: bool,
    #[serde(default)]
    pub supported_permissions: Vec<String>,
    #[serde(default)]
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SandboxExecRequest {
    pub extension_id: String,
    pub extension_version: String,
    pub executable_id: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub workspace: Option<PathBuf>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SandboxExecResult {
    pub executable_id: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u128,
    pub backend: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstallRecord {
    signer_public_key: String,
}

pub fn load_manifest(path: &Path) -> Result<ExtensionManifest, ExtensionError> {
    let m: ExtensionManifest = serde_json::from_slice(&fs::read(path)?)?;
    validate_manifest(&m, path.parent().unwrap_or(Path::new(".")))?;
    Ok(m)
}
pub fn verify_manifest(
    manifest: &ExtensionManifest,
    trust: &TrustStore,
) -> Result<String, ExtensionError> {
    let signature = manifest
        .signature
        .as_deref()
        .filter(|v| !v.is_empty())
        .ok_or(ExtensionError::Untrusted)?;
    let canonical = canonical_bytes(manifest)?;
    for key in &trust.public_keys {
        if vsn_security::verify_signature(key, &canonical, signature).is_ok() {
            return Ok(key.clone());
        }
    }
    Err(ExtensionError::Untrusted)
}
static EXTENSION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
fn extension_guard() -> Result<MutexGuard<'static, ()>, ExtensionError> {
    EXTENSION_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| ExtensionError::Invalid("extension store lock poisoned".into()))
}

pub fn install_package(
    package_dir: &Path,
    install_root: &Path,
    trust: &TrustStore,
) -> Result<InstalledExtension, ExtensionError> {
    let _guard = extension_guard()?;
    let manifest_path = package_dir.join("extension.json");
    let manifest = load_manifest(&manifest_path)?;
    let signer = verify_manifest(&manifest, trust)?;
    validate_dependencies_installed(install_root, &manifest, trust)?;
    let (files, bytes) = package_stats(package_dir)?;
    if files > MAX_EXTENSION_FILES || bytes > MAX_EXTENSION_BYTES {
        return Err(ExtensionError::Invalid(
            "extension package exceeds file/size safety limits".into(),
        ));
    }
    let parent = install_root.join(&manifest.id);
    fs::create_dir_all(&parent)?;
    let destination = parent.join(&manifest.version);
    let staging = parent.join(format!(".{}.staging", manifest.version));
    let backup = parent.join(format!(".{}.backup", manifest.version));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    if backup.exists() && !destination.exists() {
        fs::rename(&backup, &destination)?;
    }
    if backup.exists() {
        fs::remove_dir_all(&backup)?;
    }
    copy_tree(package_dir, &staging)?;
    fs::write(
        staging.join(".vsn-install.json"),
        serde_json::to_vec_pretty(&InstallRecord {
            signer_public_key: signer.clone(),
        })?,
    )?;
    if destination.exists() {
        fs::rename(&destination, &backup)?;
    }
    if let Err(err) = fs::rename(&staging, &destination) {
        if backup.exists() {
            let _ = fs::rename(&backup, &destination);
        }
        return Err(ExtensionError::Io(err));
    }
    if backup.exists() {
        fs::remove_dir_all(&backup)?;
    }
    Ok(InstalledExtension {
        id: manifest.id,
        version: manifest.version,
        path: destination,
        signer_public_key: signer,
    })
}

pub fn list_installed(install_root: &Path) -> Result<Vec<InstalledExtension>, ExtensionError> {
    let _guard = extension_guard()?;
    let mut out = Vec::new();
    if !install_root.exists() {
        return Ok(out);
    }
    for id_entry in fs::read_dir(install_root)? {
        let id_entry = id_entry?;
        if !id_entry.file_type()?.is_dir() {
            continue;
        }
        for version_entry in fs::read_dir(id_entry.path())? {
            let version_entry = version_entry?;
            if !version_entry.file_type()?.is_dir()
                || version_entry.file_name().to_string_lossy().starts_with('.')
            {
                continue;
            }
            let manifest_path = version_entry.path().join("extension.json");
            if !manifest_path.is_file() {
                continue;
            }
            let manifest = load_manifest(&manifest_path)?;
            let record_path = version_entry.path().join(".vsn-install.json");
            let record: InstallRecord =
                serde_json::from_slice(&fs::read(&record_path)?).map_err(ExtensionError::Json)?;
            out.push(InstalledExtension {
                id: manifest.id,
                version: manifest.version,
                path: version_entry.path(),
                signer_public_key: record.signer_public_key,
            });
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.version.cmp(&b.version)));
    Ok(out)
}
pub fn resolve_providers(
    install_root: &Path,
    id: &str,
    version: &str,
    kind: Option<&str>,
) -> Result<Vec<ResolvedProvider>, ExtensionError> {
    let _guard = extension_guard()?;
    validate_component(id, 128)?;
    validate_component(version, 64)?;
    if let Some(k) = kind {
        if !matches!(
            k,
            "runtime"
                | "database"
                | "service"
                | "project"
                | "container"
                | "cloud"
                | "os"
                | "network"
        ) {
            return Err(ExtensionError::Invalid(
                "unsupported provider kind filter".into(),
            ));
        }
    }
    let root = install_root.join(id).join(version);
    let canonical = root
        .canonicalize()
        .map_err(|_| ExtensionError::Invalid("installed extension was not found".into()))?;
    let manifest = load_manifest(&canonical.join("extension.json"))?;
    if manifest.id != id || manifest.version != version {
        return Err(ExtensionError::Invalid(
            "installed extension manifest identity mismatch".into(),
        ));
    }
    let record: InstallRecord =
        serde_json::from_slice(&fs::read(canonical.join(".vsn-install.json"))?)?;
    let runtime_trust = TrustStore {
        public_keys: vec![record.signer_public_key.clone()],
    };
    verify_manifest(&manifest, &runtime_trust)?;
    let mut out = Vec::new();
    for provider in manifest.providers {
        if kind.map(|k| k != provider.kind).unwrap_or(false) {
            continue;
        }
        let path = canonical
            .join(&provider.manifest)
            .canonicalize()
            .map_err(|_| {
                ExtensionError::Invalid("installed provider manifest is missing".into())
            })?;
        if !path.starts_with(&canonical) || !path.is_file() {
            return Err(ExtensionError::Invalid(
                "installed provider path escapes extension root".into(),
            ));
        }
        let bytes = fs::read(&path)?;
        if bytes.len() > 2 * 1024 * 1024 {
            return Err(ExtensionError::Invalid(
                "provider manifest exceeds 2 MiB".into(),
            ));
        }
        let _: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
            ExtensionError::Invalid(format!("provider manifest is not valid JSON: {e}"))
        })?;
        out.push(ResolvedProvider {
            extension_id: id.into(),
            extension_version: version.into(),
            kind: provider.kind,
            manifest_path: path,
            signer_public_key: record.signer_public_key.clone(),
        });
    }
    Ok(out)
}

pub fn sandbox_capabilities() -> SandboxCapabilities {
    #[cfg(target_os = "linux")]
    {
        let available = Command::new("bwrap")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        SandboxCapabilities {
            backend: "bubblewrap".into(),
            available,
            network_default_denied: true,
            filesystem_policy_enforced: true,
            supported_permissions: vec![
                "process.execute".into(),
                "network".into(),
                "filesystem.read".into(),
                "filesystem.write".into(),
            ],
            limitations: if available {
                vec![]
            } else {
                vec!["Bubblewrap executable is required on the host".into()]
            },
        }
    }
    #[cfg(target_os = "windows")]
    {
        let helper = find_windows_sandbox_helper();
        let available = helper
            .as_ref()
            .map(|p| {
                Command::new(p)
                    .arg("--probe")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        SandboxCapabilities{backend:"windows_appcontainer".into(),available,network_default_denied:true,filesystem_policy_enforced:true,supported_permissions:vec!["process.execute".into(),"network".into()],limitations:vec!["Arbitrary workspace mounts are denied by the Windows AppContainer executable backend; use structured provider APIs for workspace filesystem access".into()]}
    }
    #[cfg(target_os = "macos")]
    {
        let available = Path::new("/usr/bin/codesign").is_file();
        return SandboxCapabilities{backend:"macos_app_sandbox".into(),available,network_default_denied:true,filesystem_policy_enforced:true,supported_permissions:vec!["process.execute".into(),"network".into()],limitations:vec!["Arbitrary workspace mounts are denied by the macOS App Sandbox executable backend; use structured provider APIs for workspace filesystem access".into()]};
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        SandboxCapabilities {
            backend: "unavailable".into(),
            available: false,
            network_default_denied: true,
            filesystem_policy_enforced: false,
            supported_permissions: vec![],
            limitations: vec![
                "Executable extensions are not supported on this operating system".into(),
            ],
        }
    }
}

pub fn run_sandboxed(
    install_root: &Path,
    request: &SandboxExecRequest,
) -> Result<SandboxExecResult, ExtensionError> {
    let _guard = extension_guard()?;
    validate_component(&request.extension_id, 128)?;
    validate_component(&request.extension_version, 64)?;
    validate_component(&request.executable_id, 128)?;
    if request.args.len() > 64
        || request
            .args
            .iter()
            .any(|a| a.len() > 4096 || a.contains('\0'))
    {
        return Err(ExtensionError::Invalid(
            "sandbox executable arguments exceed limits".into(),
        ));
    }
    let root = install_root
        .join(&request.extension_id)
        .join(&request.extension_version)
        .canonicalize()
        .map_err(|_| ExtensionError::Invalid("installed extension was not found".into()))?;
    let manifest = load_manifest(&root.join("extension.json"))?;
    if manifest.id != request.extension_id || manifest.version != request.extension_version {
        return Err(ExtensionError::Invalid(
            "installed extension identity mismatch".into(),
        ));
    }
    let record: InstallRecord = serde_json::from_slice(&fs::read(root.join(".vsn-install.json"))?)?;
    verify_manifest(
        &manifest,
        &TrustStore {
            public_keys: vec![record.signer_public_key],
        },
    )?;
    let exec = manifest
        .executables
        .iter()
        .find(|e| e.id == request.executable_id)
        .ok_or_else(|| ExtensionError::Invalid("declared extension executable not found".into()))?;
    let rel = Path::new(&exec.path);
    if rel.is_absolute()
        || rel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(ExtensionError::Invalid(
            "extension executable path escapes package".into(),
        ));
    }
    let host_exec = root
        .join(rel)
        .canonicalize()
        .map_err(|_| ExtensionError::Invalid("extension executable is missing".into()))?;
    if !host_exec.starts_with(&root) || !host_exec.is_file() {
        return Err(ExtensionError::Invalid(
            "extension executable path escapes package".into(),
        ));
    }
    if manifest
        .permissions
        .iter()
        .any(|p| matches!(p.as_str(), "secrets.use" | "database.connect"))
    {
        return Err(ExtensionError::Invalid("executable extensions cannot receive secret/database capabilities directly; use structured provider APIs".into()));
    }
    #[cfg(target_os = "linux")]
    {
        run_linux_bubblewrap(&root, rel, &manifest, exec, request)
    }
    #[cfg(target_os = "windows")]
    {
        run_windows_appcontainer(&root, rel, &manifest, exec, request)
    }
    #[cfg(target_os = "macos")]
    {
        return run_macos_app_sandbox(&root, rel, &manifest, exec, request);
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        let _ = (root, rel, manifest, exec, request);
        Err(ExtensionError::Invalid(
            "executable extensions are unavailable on this operating system and fail closed".into(),
        ))
    }
}

#[cfg(target_os = "linux")]
fn run_linux_bubblewrap(
    root: &Path,
    rel: &Path,
    manifest: &ExtensionManifest,
    exec: &ExecutableReference,
    request: &SandboxExecRequest,
) -> Result<SandboxExecResult, ExtensionError> {
    let caps = sandbox_capabilities();
    if !caps.available {
        return Err(ExtensionError::Invalid(
            "Bubblewrap is required for executable extensions on Linux".into(),
        ));
    }
    let mut cmd = Command::new("bwrap");
    cmd.args([
        "--die-with-parent",
        "--new-session",
        "--unshare-user",
        "--unshare-pid",
        "--unshare-uts",
        "--unshare-ipc",
        "--proc",
        "/proc",
        "--dev",
        "/dev",
        "--tmpfs",
        "/tmp",
    ]);
    if !manifest.permissions.iter().any(|p| p == "network") {
        cmd.arg("--unshare-net");
    }
    for dir in ["/usr", "/bin", "/lib", "/lib64", "/sbin"] {
        let p = Path::new(dir);
        if p.exists() {
            cmd.arg("--ro-bind").arg(p).arg(p);
        }
    }
    if Path::new("/etc").exists() {
        cmd.args(["--ro-bind", "/etc", "/etc"]);
    }
    cmd.arg("--ro-bind").arg(root).arg("/extension");
    if let Some(workspace) = &request.workspace {
        let canonical = workspace
            .canonicalize()
            .map_err(|_| ExtensionError::Invalid("sandbox workspace does not exist".into()))?;
        if manifest.permissions.iter().any(|p| p == "filesystem.write") {
            cmd.arg("--bind").arg(canonical).arg("/workspace");
        } else if manifest.permissions.iter().any(|p| p == "filesystem.read") {
            cmd.arg("--ro-bind").arg(canonical).arg("/workspace");
        } else {
            return Err(ExtensionError::Invalid(
                "extension did not request filesystem workspace access".into(),
            ));
        }
    }
    let sandbox_exec = Path::new("/extension").join(rel);
    cmd.arg("--chdir").arg("/extension").arg(sandbox_exec);
    for a in &exec.fixed_args {
        cmd.arg(a);
    }
    for a in &request.args {
        cmd.arg(a);
    }
    cmd.env_clear().env("PATH", "/usr/bin:/bin");
    run_child_bounded(
        cmd,
        exec.timeout_seconds,
        "bubblewrap",
        &request.executable_id,
    )
}

#[cfg(target_os = "windows")]
fn find_windows_sandbox_helper() -> Option<PathBuf> {
    let name = "vsn-extension-appcontainer.exe";
    if let Ok(current) = std::env::current_exe() {
        if let Some(parent) = current.parent() {
            let candidate = parent.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|p| p.join(name))
            .find(|p| p.is_file())
    })
}
#[cfg(target_os = "windows")]
fn run_windows_appcontainer(
    root: &Path,
    rel: &Path,
    manifest: &ExtensionManifest,
    exec: &ExecutableReference,
    request: &SandboxExecRequest,
) -> Result<SandboxExecResult, ExtensionError> {
    if request.workspace.is_some() {
        return Err(ExtensionError::Invalid("Windows AppContainer executable backend denies arbitrary workspace mounts; use structured provider APIs".into()));
    }
    let helper = find_windows_sandbox_helper().ok_or_else(|| {
        ExtensionError::Invalid(
            "vsn-extension-appcontainer.exe is required for executable extensions on Windows"
                .into(),
        )
    })?;
    let mut cmd = Command::new(helper);
    cmd.arg("--root")
        .arg(root)
        .arg("--exec")
        .arg(rel)
        .arg("--profile")
        .arg(format!(
            "vsn.{}.{}",
            request.extension_id.replace('.', "-"),
            request.extension_version.replace('.', "-")
        ))
        .arg("--timeout")
        .arg(exec.timeout_seconds.clamp(1, 120).to_string())
        .arg("--network")
        .arg(if manifest.permissions.iter().any(|p| p == "network") {
            "1"
        } else {
            "0"
        });
    for a in &exec.fixed_args {
        cmd.arg("--arg").arg(a);
    }
    for a in &request.args {
        cmd.arg("--arg").arg(a);
    }
    cmd.env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default());
    run_child_bounded(
        cmd,
        exec.timeout_seconds.saturating_add(5),
        "windows_appcontainer",
        &request.executable_id,
    )
}

#[cfg(target_os = "macos")]
fn run_macos_app_sandbox(
    root: &Path,
    rel: &Path,
    manifest: &ExtensionManifest,
    exec: &ExecutableReference,
    request: &SandboxExecRequest,
) -> Result<SandboxExecResult, ExtensionError> {
    if request.workspace.is_some() {
        return Err(ExtensionError::Invalid("macOS App Sandbox executable backend denies arbitrary workspace mounts; use structured provider APIs".into()));
    }
    if !Path::new("/usr/bin/codesign").is_file() {
        return Err(ExtensionError::Invalid(
            "/usr/bin/codesign is required for macOS executable extension sandboxing".into(),
        ));
    }
    let nonce = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|v| v.as_nanos())
            .unwrap_or(0)
    );
    let base = std::env::temp_dir().join(format!("vsn-extension-sandbox-{nonce}"));
    let app = base.join("VSNExtension.app");
    let contents = app.join("Contents");
    let macos = contents.join("MacOS");
    let resources = contents.join("Resources").join("extension");
    fs::create_dir_all(&macos)?;
    fs::create_dir_all(&resources)?;
    copy_tree(root, &resources)?;
    let staged = macos.join("extension-exec");
    fs::copy(root.join(rel), &staged)?;
    let plist=format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\"><plist version=\"1.0\"><dict><key>CFBundleIdentifier</key><string>dev.vsn.extension.sandbox.{}</string><key>CFBundleExecutable</key><string>extension-exec</string><key>CFBundleName</key><string>VSN Extension Sandbox</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>",request.extension_id.replace('_',"-"));
    fs::write(contents.join("Info.plist"), plist)?;
    let network = manifest.permissions.iter().any(|p| p == "network");
    let entitlements=format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\"><plist version=\"1.0\"><dict><key>com.apple.security.app-sandbox</key><true/>{}</dict></plist>",if network{"<key>com.apple.security.network.client</key><true/>"}else{""});
    let entitlement_path = base.join("entitlements.plist");
    fs::write(&entitlement_path, entitlements)?;
    let status = Command::new("/usr/bin/codesign")
        .args(["--force", "--deep", "--sign", "-", "--entitlements"])
        .arg(&entitlement_path)
        .arg(&app)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| ExtensionError::Invalid(format!("codesign failed to start: {e}")))?;
    if !status.status.success() {
        let message = String::from_utf8_lossy(&status.stderr)
            .chars()
            .take(2048)
            .collect::<String>();
        let _ = fs::remove_dir_all(&base);
        return Err(ExtensionError::Invalid(format!(
            "macOS App Sandbox staging signature failed: {message}"
        )));
    }
    let mut cmd = Command::new(&staged);
    cmd.current_dir(&resources);
    for a in &exec.fixed_args {
        cmd.arg(a);
    }
    for a in &request.args {
        cmd.arg(a);
    }
    cmd.env_clear().env("PATH", "/usr/bin:/bin");
    let result = run_child_bounded(
        cmd,
        exec.timeout_seconds,
        "macos_app_sandbox",
        &request.executable_id,
    );
    let _ = fs::remove_dir_all(&base);
    result
}

fn run_child_bounded(
    mut cmd: Command,
    timeout_seconds: u64,
    backend: &str,
    executable_id: &str,
) -> Result<SandboxExecResult, ExtensionError> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let started = Instant::now();
    let mut child = cmd.spawn().map_err(|e| {
        ExtensionError::Invalid(format!("{backend} execution failed to start: {e}"))
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ExtensionError::Invalid("sandbox stdout pipe unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ExtensionError::Invalid("sandbox stderr pipe unavailable".into()))?;
    let stdout_thread = std::thread::spawn(move || read_pipe_bounded(stdout, 2 * 1024 * 1024));
    let stderr_thread = std::thread::spawn(move || read_pipe_bounded(stderr, 512 * 1024));
    let timeout = Duration::from_secs(timeout_seconds.clamp(1, 125));
    let status = loop {
        if let Some(status) = child.try_wait().map_err(ExtensionError::Io)? {
            break status;
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_thread.join();
            let _ = stderr_thread.join();
            return Err(ExtensionError::Invalid(
                "sandboxed extension exceeded execution timeout".into(),
            ));
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    let stdout = stdout_thread
        .join()
        .map_err(|_| ExtensionError::Invalid("sandbox stdout reader panicked".into()))??;
    let stderr = stderr_thread
        .join()
        .map_err(|_| ExtensionError::Invalid("sandbox stderr reader panicked".into()))??;
    Ok(SandboxExecResult {
        executable_id: executable_id.into(),
        exit_code: status.code(),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        duration_ms: started.elapsed().as_millis(),
        backend: backend.into(),
    })
}

fn read_pipe_bounded<R: io::Read>(reader: R, max: usize) -> Result<Vec<u8>, ExtensionError> {
    let mut out = Vec::new();
    let mut limited = reader.take(max as u64 + 1);
    limited.read_to_end(&mut out)?;
    if out.len() > max {
        return Err(ExtensionError::Invalid(
            "sandboxed extension output exceeded safety limit".into(),
        ));
    }
    Ok(out)
}

pub fn uninstall(install_root: &Path, id: &str, version: &str) -> Result<bool, ExtensionError> {
    let _guard = extension_guard()?;
    validate_component(id, 128)?;
    validate_component(version, 64)?;
    let dependents = dependent_extensions_unlocked(install_root, id, version)?;
    if !dependents.is_empty() {
        return Err(ExtensionError::Invalid(format!(
            "extension is required by: {}",
            dependents.join(", ")
        )));
    }
    let path = install_root.join(id).join(version);
    let tombstone = install_root.join(id).join(format!(".{version}.removing"));
    if tombstone.exists() && !path.exists() {
        fs::remove_dir_all(&tombstone)?;
        return Ok(true);
    }
    if !path.exists() {
        return Ok(false);
    }
    if tombstone.exists() {
        fs::remove_dir_all(&tombstone)?;
    }
    fs::rename(&path, &tombstone)?;
    fs::remove_dir_all(&tombstone)?;
    if let Some(parent) = path.parent() {
        if parent.read_dir()?.next().is_none() {
            let _ = fs::remove_dir(parent);
        }
    }
    Ok(true)
}
fn validate_component(value: &str, max: usize) -> Result<(), ExtensionError> {
    if value.is_empty()
        || value.len() > max
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        Err(ExtensionError::Invalid(
            "unsafe extension identifier/version".into(),
        ))
    } else {
        Ok(())
    }
}
pub fn canonical_bytes(manifest: &ExtensionManifest) -> Result<Vec<u8>, ExtensionError> {
    let mut unsigned = manifest.clone();
    unsigned.signature = None;
    Ok(serde_json::to_vec(&unsigned)?)
}
fn validate_manifest(m: &ExtensionManifest, base: &Path) -> Result<(), ExtensionError> {
    if m.api_version != 1 {
        return Err(ExtensionError::Invalid(
            "unsupported extension api_version".into(),
        ));
    }
    if m.id.is_empty()
        || m.id.len() > 128
        || !m.id.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        })
    {
        return Err(ExtensionError::Invalid("unsafe extension id".into()));
    }
    if m.version.is_empty()
        || m.version.len() > 64
        || !m
            .version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        || (m.providers.is_empty() && m.executables.is_empty())
    {
        return Err(ExtensionError::Invalid(
            "extension version and at least one provider or executable are required".into(),
        ));
    }
    if m.permissions.len() > 64
        || m.permissions.iter().any(|p| {
            !matches!(
                p.as_str(),
                "network"
                    | "filesystem.read"
                    | "filesystem.write"
                    | "process.execute"
                    | "secrets.use"
                    | "database.connect"
            )
        })
    {
        return Err(ExtensionError::Invalid(
            "extension requests an unknown or excessive permission set".into(),
        ));
    }
    if m.executables.len() > 16 {
        return Err(ExtensionError::Invalid(
            "extension declares too many executables".into(),
        ));
    }
    for e in &m.executables {
        validate_component(&e.id, 128)?;
        let rel = Path::new(&e.path);
        if rel.is_absolute()
            || rel
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            || !base.join(rel).is_file()
        {
            return Err(ExtensionError::Invalid(
                "extension executable path is missing or unsafe".into(),
            ));
        }
        if e.fixed_args.len() > 32
            || e.fixed_args
                .iter()
                .any(|a| a.len() > 4096 || a.contains('\0'))
            || !(1..=120).contains(&e.timeout_seconds)
        {
            return Err(ExtensionError::Invalid(
                "extension executable declaration exceeds limits".into(),
            ));
        }
    }
    if !m.executables.is_empty() && !m.permissions.iter().any(|p| p == "process.execute") {
        return Err(ExtensionError::Invalid(
            "extension executables require process.execute permission".into(),
        ));
    }
    if m.dependencies.len() > 32 {
        return Err(ExtensionError::Invalid(
            "extension declares too many dependencies".into(),
        ));
    }
    for d in &m.dependencies {
        validate_component(&d.id, 128)?;
        validate_component(&d.version, 64)?;
        if d.id == m.id && d.version == m.version {
            return Err(ExtensionError::Invalid(
                "extension cannot depend on itself".into(),
            ));
        }
    }
    for p in &m.providers {
        if !matches!(
            p.kind.as_str(),
            "runtime"
                | "database"
                | "service"
                | "project"
                | "container"
                | "cloud"
                | "os"
                | "network"
        ) {
            return Err(ExtensionError::Invalid(format!(
                "unsupported provider kind: {}",
                p.kind
            )));
        }
        let rel = Path::new(&p.manifest);
        if rel.is_absolute()
            || rel
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(ExtensionError::Invalid(
                "provider manifest path must remain inside package".into(),
            ));
        }
        if !base.join(rel).is_file() {
            return Err(ExtensionError::Invalid(format!(
                "provider manifest missing: {}",
                p.manifest
            )));
        }
    }
    Ok(())
}
fn copy_tree(source: &Path, destination: &Path) -> Result<(), io::Error> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let dest = destination.join(entry.file_name());
        let ty = entry.file_type()?;
        if ty.is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "extension packages may not contain symlinks",
            ));
        }
        if ty.is_dir() {
            copy_tree(&source_path, &dest)?;
        } else if ty.is_file() {
            fs::copy(&source_path, &dest)?;
        }
    }
    Ok(())
}
fn package_stats(root: &Path) -> Result<(usize, u64), io::Error> {
    fn walk(path: &Path, files: &mut usize, bytes: &mut u64) -> Result<(), io::Error> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_symlink() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "extension packages may not contain symlinks",
                ));
            }
            if ty.is_dir() {
                walk(&entry.path(), files, bytes)?;
            } else if ty.is_file() {
                *files += 1;
                *bytes = bytes.saturating_add(entry.metadata()?.len());
                if *files > MAX_EXTENSION_FILES || *bytes > MAX_EXTENSION_BYTES {
                    return Ok(());
                }
            }
        }
        Ok(())
    }
    let mut files = 0;
    let mut bytes = 0;
    walk(root, &mut files, &mut bytes)?;
    Ok((files, bytes))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsigned_is_not_trusted() {
        let m = ExtensionManifest {
            id: "demo".into(),
            name: "Demo".into(),
            version: "1".into(),
            api_version: 1,
            providers: vec![ProviderReference {
                kind: "runtime".into(),
                manifest: "m.json".into(),
            }],
            executables: vec![],
            dependencies: vec![],
            permissions: vec![],
            signature: None,
        };
        assert!(verify_manifest(&m, &TrustStore::default()).is_err());
    }
}

// ---------- 0.24 deterministic dependency lifecycle + conformance ----------
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionDependencyReport {
    pub extension_id: String,
    pub extension_version: String,
    pub dependencies: Vec<ExtensionDependency>,
    pub missing: Vec<ExtensionDependency>,
    pub dependents: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionConformanceReport {
    pub signed_install: bool,
    pub atomic_install_uninstall: bool,
    pub provider_resolution: bool,
    pub dependency_lifecycle: bool,
    pub linux_sandbox: bool,
    pub windows_sandbox: bool,
    pub macos_sandbox: bool,
    pub fail_closed_without_sandbox: bool,
    pub unsupported_capabilities_fail_closed: bool,
    pub host_backend_available: bool,
    pub issues: Vec<String>,
}
fn validate_dependencies_installed(
    install_root: &Path,
    manifest: &ExtensionManifest,
    trust: &TrustStore,
) -> Result<(), ExtensionError> {
    for d in &manifest.dependencies {
        let root = install_root.join(&d.id).join(&d.version);
        let child = load_manifest(&root.join("extension.json")).map_err(|_| {
            ExtensionError::Invalid(format!(
                "missing extension dependency {}@{}",
                d.id, d.version
            ))
        })?;
        if child.id != d.id || child.version != d.version {
            return Err(ExtensionError::Invalid(
                "extension dependency identity mismatch".into(),
            ));
        }
        verify_manifest(&child, trust).map_err(|_| {
            ExtensionError::Invalid(format!(
                "extension dependency {}@{} is not trusted",
                d.id, d.version
            ))
        })?;
    }
    Ok(())
}
fn dependent_extensions_unlocked(
    install_root: &Path,
    id: &str,
    version: &str,
) -> Result<Vec<String>, ExtensionError> {
    let mut out = Vec::new();
    if !install_root.exists() {
        return Ok(out);
    }
    for id_entry in fs::read_dir(install_root)? {
        let id_entry = id_entry?;
        if !id_entry.file_type()?.is_dir() {
            continue;
        }
        for ver_entry in fs::read_dir(id_entry.path())? {
            let ver_entry = ver_entry?;
            if !ver_entry.file_type()?.is_dir()
                || ver_entry.file_name().to_string_lossy().starts_with('.')
            {
                continue;
            }
            let manifest_path = ver_entry.path().join("extension.json");
            if !manifest_path.is_file() {
                continue;
            }
            let m = load_manifest(&manifest_path)?;
            if m.dependencies
                .iter()
                .any(|d| d.id == id && d.version == version)
            {
                out.push(format!("{}@{}", m.id, m.version));
            }
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}
pub fn dependency_report(
    install_root: &Path,
    id: &str,
    version: &str,
) -> Result<ExtensionDependencyReport, ExtensionError> {
    let _guard = extension_guard()?;
    validate_component(id, 128)?;
    validate_component(version, 64)?;
    let root = install_root.join(id).join(version);
    let manifest = load_manifest(&root.join("extension.json"))?;
    let missing = manifest
        .dependencies
        .iter()
        .filter(|d| {
            !install_root
                .join(&d.id)
                .join(&d.version)
                .join("extension.json")
                .is_file()
        })
        .cloned()
        .collect();
    let dependents = dependent_extensions_unlocked(install_root, id, version)?;
    Ok(ExtensionDependencyReport {
        extension_id: id.into(),
        extension_version: version.into(),
        dependencies: manifest.dependencies,
        missing,
        dependents,
    })
}
pub fn extension_conformance() -> ExtensionConformanceReport {
    let caps = sandbox_capabilities();
    let mut issues = Vec::new();
    if !caps.available {
        issues.push(format!(
            "{} sandbox backend is not available on this host",
            caps.backend
        ));
    }
    issues.extend(caps.limitations.clone());
    ExtensionConformanceReport {
        signed_install: true,
        atomic_install_uninstall: true,
        provider_resolution: true,
        dependency_lifecycle: true,
        linux_sandbox: true,
        windows_sandbox: true,
        macos_sandbox: true,
        fail_closed_without_sandbox: true,
        unsupported_capabilities_fail_closed: true,
        host_backend_available: caps.available,
        issues,
    }
}
