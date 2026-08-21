use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("project path does not exist: {0}")]
    Missing(String),
    #[error("invalid project request: {0}")]
    Invalid(String),
    #[error("project command failed: {0}")]
    Command(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectDetection {
    pub path: PathBuf,
    pub project_type: String,
    pub frameworks: Vec<String>,
    pub runtimes: Vec<String>,
    pub package_managers: Vec<String>,
    pub databases: Vec<String>,
    pub services: Vec<String>,
    pub evidence: Vec<String>,
}

fn active_env_entries(raw: &str) -> Vec<(String, String)> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let line = line.strip_prefix("export ").unwrap_or(line).trim();
            let (key, value) = line.split_once('=')?;
            let key = key.trim();
            if key.is_empty() {
                return None;
            }
            let value = value.trim().trim_matches(|c| c == '\'' || c == '"');
            Some((key.to_ascii_uppercase(), value.to_ascii_lowercase()))
        })
        .collect()
}

fn compose_active_text(raw: &str) -> String {
    raw.lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            Some(line.split(" #").next().unwrap_or(line))
        })
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase()
}

fn read_json_manifest(
    project: &Path,
    name: &str,
) -> Result<Option<serde_json::Value>, ProjectError> {
    let path = project.join(name);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)?;
    let value = serde_json::from_str::<serde_json::Value>(&raw)
        .map_err(|error| ProjectError::Invalid(format!("invalid {name}: {error}")))?;
    Ok(Some(value))
}

pub fn detect(path: &Path) -> Result<ProjectDetection, ProjectError> {
    if !path.is_dir() {
        return Err(ProjectError::Missing(path.display().to_string()));
    }
    let mut frameworks = Vec::new();
    let mut managers = Vec::new();
    let mut dbs = Vec::new();
    let mut services = Vec::new();
    let mut evidence = Vec::new();
    let exists = |name: &str| path.join(name).exists();

    if exists("artisan") && exists("composer.json") {
        frameworks.push("laravel".into());
        evidence.push("artisan".into());
    }
    if exists("package.json") {
        managers.push(
            if exists("pnpm-lock.yaml") {
                "pnpm"
            } else if exists("yarn.lock") {
                "yarn"
            } else if exists("bun.lock") || exists("bun.lockb") {
                "bun"
            } else {
                "npm"
            }
            .into(),
        );
        evidence.push("package.json".into());
    }
    if exists("composer.json") {
        managers.push("composer".into());
        evidence.push("composer.json".into());
    }
    if exists("pyproject.toml") || exists("requirements.txt") {
        managers.push("python-package".into());
    }
    if exists("manage.py") {
        frameworks.push("django".into());
        evidence.push("manage.py".into());
    }
    if exists("Cargo.toml") {
        managers.push("cargo".into());
        evidence.push("Cargo.toml".into());
    }
    if exists("go.mod") {
        managers.push("go-modules".into());
        evidence.push("go.mod".into());
    }
    if exists("Gemfile") {
        managers.push("bundler".into());
        evidence.push("Gemfile".into());
    }

    if let Ok(env) = fs::read_to_string(path.join(".env")) {
        for (key, value) in active_env_entries(&env) {
            if key == "DB_CONNECTION" {
                let database = match value.as_str() {
                    "mysql" => Some("mysql"),
                    "pgsql" | "postgres" | "postgresql" => Some("postgresql"),
                    "sqlite" => Some("sqlite"),
                    "mongodb" | "mongo" => Some("mongodb"),
                    _ => None,
                };
                if let Some(database) = database {
                    dbs.push(database.into());
                }
            }
            if key.contains("MONGODB") || key.starts_with("MONGO_") {
                dbs.push("mongodb".into());
            }
            if key.starts_with("REDIS_") {
                services.push("redis".into());
            }
        }
    }
    if let Ok(compose) = fs::read_to_string(path.join("docker-compose.yml"))
        .or_else(|_| fs::read_to_string(path.join("compose.yml")))
    {
        let lower = compose_active_text(&compose);
        for id in [
            "mysql",
            "mariadb",
            "postgres",
            "redis",
            "mongodb",
            "elasticsearch",
            "rabbitmq",
            "kafka",
        ] {
            if lower.contains(id) {
                services.push(id.into());
            }
        }
        evidence.push("compose file".into());
    }
    services.sort();
    services.dedup();
    dbs.sort();
    dbs.dedup();
    managers.sort();
    managers.dedup();
    frameworks.sort();
    frameworks.dedup();
    let runtimes = vsn_runtime::runtimes_for_project(path);
    let project_type = frameworks
        .first()
        .cloned()
        .or_else(|| runtimes.first().cloned())
        .unwrap_or_else(|| "generic".into());
    Ok(ProjectDetection {
        path: path.to_path_buf(),
        project_type,
        frameworks,
        runtimes,
        package_managers: managers,
        databases: dbs,
        services,
        evidence,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectRequirement {
    pub kind: String,
    pub name: String,
    pub constraint: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemediationStep {
    pub category: String,
    pub description: String,
    pub command: Option<Vec<String>>,
    pub automatic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectDependencyReport {
    pub requirements: Vec<ProjectRequirement>,
    pub remediation: Vec<RemediationStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootstrapPlan {
    pub template: String,
    pub destination: PathBuf,
    pub program: String,
    pub args: Vec<String>,
    pub requires_network: bool,
}

pub fn dependency_report(path: &Path) -> Result<ProjectDependencyReport, ProjectError> {
    if !path.is_dir() {
        return Err(ProjectError::Missing(path.display().to_string()));
    }
    let mut requirements = Vec::new();
    let mut remediation = Vec::new();

    if let Some(value) = read_json_manifest(path, "composer.json")? {
        if let Some(req) = value.get("require").and_then(|v| v.as_object()) {
            for (name, constraint) in req {
                if name == "php" || name.starts_with("ext-") {
                    requirements.push(ProjectRequirement {
                        kind: if name == "php" {
                            "runtime".into()
                        } else {
                            "extension".into()
                        },
                        name: name.clone(),
                        constraint: constraint.as_str().map(str::to_string),
                        source: "composer.json".into(),
                    });
                }
            }
        }
    }
    if let Some(value) = read_json_manifest(path, "package.json")? {
        if let Some(engines) = value.get("engines").and_then(|v| v.as_object()) {
            for (name, constraint) in engines {
                if matches!(name.as_str(), "node" | "npm" | "pnpm" | "yarn" | "bun") {
                    requirements.push(ProjectRequirement {
                        kind: "runtime_or_tool".into(),
                        name: name.clone(),
                        constraint: constraint.as_str().map(str::to_string),
                        source: "package.json#engines".into(),
                    });
                }
            }
        }
    }
    if path.join("requirements.txt").exists() || path.join("pyproject.toml").exists() {
        requirements.push(ProjectRequirement {
            kind: "runtime".into(),
            name: "python".into(),
            constraint: None,
            source: if path.join("pyproject.toml").exists() {
                "pyproject.toml".into()
            } else {
                "requirements.txt".into()
            },
        });
    }
    if path.join("go.mod").exists() {
        requirements.push(ProjectRequirement {
            kind: "runtime".into(),
            name: "go".into(),
            constraint: None,
            source: "go.mod".into(),
        });
    }
    if path.join("Cargo.toml").exists() {
        requirements.push(ProjectRequirement {
            kind: "runtime".into(),
            name: "rust".into(),
            constraint: None,
            source: "Cargo.toml".into(),
        });
    }

    let detected = vsn_runtime::detect_all();
    for req in &requirements {
        let runtime_name = match req.name.as_str() {
            "php" => Some("php"),
            "node" => Some("node"),
            "python" => Some("python"),
            "go" => Some("go"),
            "rust" => Some("rust"),
            _ => None,
        };
        if let Some(runtime) = runtime_name {
            if !detected.iter().any(|d| d.id == runtime && d.installed) {
                remediation.push(RemediationStep {
                    category: "runtime".into(),
                    description: format!(
                        "Install required runtime: {runtime}{}",
                        req.constraint
                            .as_deref()
                            .map(|v| format!(" ({v})"))
                            .unwrap_or_default()
                    ),
                    command: None,
                    automatic: true,
                });
            }
        }
    }
    if path.join("composer.json").exists() && !path.join("vendor").exists() {
        remediation.push(RemediationStep {
            category: "dependency".into(),
            description: "Install Composer dependencies".into(),
            command: Some(vec![
                "composer".into(),
                "install".into(),
                "--no-interaction".into(),
            ]),
            automatic: true,
        });
    }
    if path.join("package.json").exists() && !path.join("node_modules").exists() {
        let manager = if path.join("pnpm-lock.yaml").exists() {
            "pnpm"
        } else if path.join("yarn.lock").exists() {
            "yarn"
        } else if path.join("bun.lock").exists() || path.join("bun.lockb").exists() {
            "bun"
        } else {
            "npm"
        };
        let action = if manager == "npm"
            && (path.join("package-lock.json").exists()
                || path.join("npm-shrinkwrap.json").exists())
        {
            "ci"
        } else {
            "install"
        };
        remediation.push(RemediationStep {
            category: "dependency".into(),
            description: format!("Install {manager} dependencies"),
            command: Some(vec![manager.into(), action.into()]),
            automatic: true,
        });
    }
    Ok(ProjectDependencyReport {
        requirements,
        remediation,
    })
}

pub fn bootstrap_plan(template: &str, destination: &Path) -> Result<BootstrapPlan, ProjectError> {
    if destination.as_os_str().is_empty() {
        return Err(ProjectError::Missing("destination is empty".into()));
    }
    let (program, args, network) = match template {
        "laravel" => (
            "composer".to_string(),
            vec![
                "create-project".into(),
                "laravel/laravel".into(),
                destination.to_string_lossy().into_owned(),
            ],
            true,
        ),
        "node" => ("npm".to_string(), vec!["init".into(), "-y".into()], false),
        "django" => (
            "python".to_string(),
            vec![
                "-m".into(),
                "django".into(),
                "startproject".into(),
                "config".into(),
                ".".into(),
            ],
            false,
        ),
        "rust" => (
            "cargo".to_string(),
            vec!["init".into(), destination.to_string_lossy().into_owned()],
            false,
        ),
        "go" => (
            "go".to_string(),
            vec![
                "mod".into(),
                "init".into(),
                destination
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or("vsn-app")
                    .into(),
            ],
            false,
        ),
        _ => {
            return Err(ProjectError::Missing(format!(
                "unknown bootstrap template: {template}"
            )))
        }
    };
    Ok(BootstrapPlan {
        template: template.into(),
        destination: destination.to_path_buf(),
        program,
        args,
        requires_network: network,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootstrapResult {
    pub template: String,
    pub destination: PathBuf,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    #[serde(default)]
    pub stdout_truncated: bool,
    #[serde(default)]
    pub stderr_truncated: bool,
}

const BOOTSTRAP_STDOUT_CAPTURE_BYTES: usize = 64 * 1024;
const BOOTSTRAP_STDERR_CAPTURE_BYTES: usize = 32 * 1024;

struct CapturedBootstrapOutput {
    bytes: Vec<u8>,
    truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BootstrapDestinationState {
    Absent,
    ExistingEmptyDirectory,
}

fn bootstrap_destination_state(
    destination: &Path,
) -> Result<BootstrapDestinationState, ProjectError> {
    match fs::symlink_metadata(destination) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(ProjectError::Invalid(
                    "bootstrap destination must be a directory".into(),
                ));
            }
            if destination.read_dir()?.next().is_some() {
                return Err(ProjectError::Invalid(
                    "bootstrap destination must be empty".into(),
                ));
            }
            Ok(BootstrapDestinationState::ExistingEmptyDirectory)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(BootstrapDestinationState::Absent)
        }
        Err(error) => Err(ProjectError::Io(error)),
    }
}

fn remove_path_entry(path: &Path) -> Result<(), ProjectError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(ProjectError::Io(error)),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        fs::remove_file(path)?;
    } else {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

fn rollback_bootstrap_destination(
    destination: &Path,
    state: BootstrapDestinationState,
) -> Result<(), ProjectError> {
    match state {
        BootstrapDestinationState::Absent => remove_path_entry(destination),
        BootstrapDestinationState::ExistingEmptyDirectory => {
            match fs::symlink_metadata(destination) {
                Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                    for entry in fs::read_dir(destination)? {
                        remove_path_entry(&entry?.path())?;
                    }
                    Ok(())
                }
                Ok(_) => {
                    remove_path_entry(destination)?;
                    fs::create_dir(destination)?;
                    Ok(())
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    fs::create_dir(destination)?;
                    Ok(())
                }
                Err(error) => Err(ProjectError::Io(error)),
            }
        }
    }
}

fn bootstrap_failure(
    destination: &Path,
    state: BootstrapDestinationState,
    message: String,
) -> ProjectError {
    match rollback_bootstrap_destination(destination, state) {
        Ok(()) => ProjectError::Command(message),
        Err(error) => {
            ProjectError::Command(format!("{message}; bootstrap rollback failed: {error}"))
        }
    }
}

pub fn execute_bootstrap(plan: &BootstrapPlan) -> Result<BootstrapResult, ProjectError> {
    let destination = &plan.destination;
    let parent = destination.parent().ok_or_else(|| {
        ProjectError::Invalid("bootstrap destination must have a parent directory".into())
    })?;
    if !parent.exists() {
        return Err(ProjectError::Invalid(
            "bootstrap parent directory must already exist".into(),
        ));
    }
    let destination_state = bootstrap_destination_state(destination)?;
    let create_destination = match plan.template.as_str() {
        "laravel" | "rust" => false,
        "node" | "django" | "go" => true,
        _ => {
            return Err(ProjectError::Invalid(
                "bootstrap template is not executable".into(),
            ))
        }
    };
    if create_destination && destination_state == BootstrapDestinationState::Absent {
        fs::create_dir(destination)?;
    }

    let mut command = Command::new(&plan.program);
    if create_destination {
        command.current_dir(destination);
    } else {
        command.current_dir(parent);
    }
    command
        .args(&plan.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            if create_destination && destination_state == BootstrapDestinationState::Absent {
                return Err(bootstrap_failure(
                    destination,
                    destination_state,
                    format!("failed to start {}: {error}", plan.program),
                ));
            }
            return Err(ProjectError::Command(format!(
                "failed to start {}: {error}",
                plan.program
            )));
        }
    };
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(bootstrap_failure(
                destination,
                destination_state,
                "bootstrap stdout unavailable".into(),
            ));
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(bootstrap_failure(
                destination,
                destination_state,
                "bootstrap stderr unavailable".into(),
            ));
        }
    };
    let out_thread =
        std::thread::spawn(move || read_bootstrap_output(stdout, BOOTSTRAP_STDOUT_CAPTURE_BYTES));
    let err_thread =
        std::thread::spawn(move || read_bootstrap_output(stderr, BOOTSTRAP_STDERR_CAPTURE_BYTES));
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if started.elapsed() > Duration::from_secs(15 * 60) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(bootstrap_failure(
                        destination,
                        destination_state,
                        "bootstrap exceeded 15 minute timeout".into(),
                    ));
                }
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(bootstrap_failure(
                    destination,
                    destination_state,
                    format!("failed while waiting for bootstrap: {error}"),
                ));
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let stdout = match out_thread.join() {
        Ok(Ok(stdout)) => stdout,
        Ok(Err(error)) => {
            return Err(bootstrap_failure(
                destination,
                destination_state,
                error.to_string(),
            ))
        }
        Err(_) => {
            return Err(bootstrap_failure(
                destination,
                destination_state,
                "bootstrap stdout reader panicked".into(),
            ))
        }
    };
    let stderr = match err_thread.join() {
        Ok(Ok(stderr)) => stderr,
        Ok(Err(error)) => {
            return Err(bootstrap_failure(
                destination,
                destination_state,
                error.to_string(),
            ))
        }
        Err(_) => {
            return Err(bootstrap_failure(
                destination,
                destination_state,
                "bootstrap stderr reader panicked".into(),
            ))
        }
    };
    if !status.success() {
        let status_text = status
            .code()
            .map(|code| code.to_string())
            .unwrap_or_else(|| "terminated by signal".into());
        let stderr_text = String::from_utf8_lossy(&stderr.bytes);
        let detail = stderr_text.trim();
        let truncation = if stderr.truncated {
            " [stderr truncated]"
        } else {
            ""
        };
        let message = if detail.is_empty() {
            format!(
                "{} exited with status {status_text}{truncation}",
                plan.program
            )
        } else {
            format!(
                "{} exited with status {status_text}{truncation}: {detail}",
                plan.program
            )
        };
        return Err(bootstrap_failure(destination, destination_state, message));
    }
    Ok(BootstrapResult {
        template: plan.template.clone(),
        destination: destination.clone(),
        status_code: status.code(),
        stdout: String::from_utf8_lossy(&stdout.bytes).into_owned(),
        stderr: String::from_utf8_lossy(&stderr.bytes).into_owned(),
        stdout_truncated: stdout.truncated,
        stderr_truncated: stderr.truncated,
    })
}
fn read_bootstrap_output<R: Read>(
    mut reader: R,
    max: usize,
) -> Result<CapturedBootstrapOutput, ProjectError> {
    let mut out = Vec::with_capacity(max.min(8 * 1024));
    let mut buffer = [0u8; 8 * 1024];
    let mut total = 0usize;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total = total.saturating_add(read);
        if max == 0 {
            continue;
        }
        if read >= max {
            out.clear();
            out.extend_from_slice(&buffer[read - max..read]);
            continue;
        }
        let overflow = out.len().saturating_add(read).saturating_sub(max);
        if overflow > 0 {
            out.drain(..overflow);
        }
        out.extend_from_slice(&buffer[..read]);
    }
    Ok(CapturedBootstrapOutput {
        bytes: out,
        truncated: total > max,
    })
}

pub const PROJECT_PROVIDER_SDK_VERSION: u32 = 1;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectProviderDescriptor {
    pub id: String,
    pub sdk_version: u32,
    pub templates: Vec<String>,
    pub detection: bool,
    pub dependencies: bool,
    pub bootstrap: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectProviderConformanceReport {
    pub descriptor: ProjectProviderDescriptor,
    pub valid: bool,
    pub issues: Vec<String>,
}
pub trait ProjectProvider {
    fn descriptor(&self) -> ProjectProviderDescriptor;
    fn detect(&self, path: &Path) -> Result<ProjectDetection, ProjectError>;
    fn dependency_report(&self, path: &Path) -> Result<ProjectDependencyReport, ProjectError>;
    fn bootstrap_plan(
        &self,
        template: &str,
        destination: &Path,
    ) -> Result<BootstrapPlan, ProjectError>;
    fn execute_bootstrap(&self, plan: &BootstrapPlan) -> Result<BootstrapResult, ProjectError>;
}
pub struct BuiltinProjectProvider;
impl ProjectProvider for BuiltinProjectProvider {
    fn descriptor(&self) -> ProjectProviderDescriptor {
        builtin_project_provider_descriptor()
    }
    fn detect(&self, path: &Path) -> Result<ProjectDetection, ProjectError> {
        detect(path)
    }
    fn dependency_report(&self, path: &Path) -> Result<ProjectDependencyReport, ProjectError> {
        dependency_report(path)
    }
    fn bootstrap_plan(
        &self,
        template: &str,
        destination: &Path,
    ) -> Result<BootstrapPlan, ProjectError> {
        bootstrap_plan(template, destination)
    }
    fn execute_bootstrap(&self, plan: &BootstrapPlan) -> Result<BootstrapResult, ProjectError> {
        execute_bootstrap(plan)
    }
}
pub fn builtin_project_templates() -> Vec<String> {
    vec![
        "laravel".into(),
        "node".into(),
        "django".into(),
        "rust".into(),
        "go".into(),
    ]
}
pub fn builtin_project_provider_descriptor() -> ProjectProviderDescriptor {
    ProjectProviderDescriptor {
        id: "builtin".into(),
        sdk_version: PROJECT_PROVIDER_SDK_VERSION,
        templates: builtin_project_templates(),
        detection: true,
        dependencies: true,
        bootstrap: true,
    }
}
pub fn project_provider_conformance(
    descriptor: &ProjectProviderDescriptor,
) -> ProjectProviderConformanceReport {
    let mut issues = Vec::new();
    if descriptor.id.is_empty()
        || descriptor.id.len() > 128
        || !descriptor
            .id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        issues.push("unsafe provider id".into());
    }
    if descriptor.sdk_version != PROJECT_PROVIDER_SDK_VERSION {
        issues.push(format!(
            "unsupported SDK version {}",
            descriptor.sdk_version
        ));
    }
    if !descriptor.detection {
        issues.push("project detection capability is required".into());
    }
    if !descriptor.dependencies {
        issues.push("dependency-report capability is required".into());
    }
    if descriptor.bootstrap && descriptor.templates.is_empty() {
        issues.push("bootstrap provider declares no templates".into());
    }
    if descriptor.templates.len() > 64
        || descriptor.templates.iter().any(|v| {
            v.is_empty()
                || v.len() > 128
                || !v
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        })
    {
        issues.push("unsafe or excessive template list".into());
    }
    ProjectProviderConformanceReport {
        valid: issues.is_empty(),
        descriptor: descriptor.clone(),
        issues,
    }
}
pub fn builtin_project_conformance() -> ProjectProviderConformanceReport {
    project_provider_conformance(&builtin_project_provider_descriptor())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("vsn-project-{name}-{nonce}"));
        fs::create_dir_all(&path).expect("create fixture");
        path
    }

    #[test]
    fn commented_env_and_compose_entries_do_not_create_false_positives() {
        let path = fixture("comments");
        fs::write(
            path.join(".env"),
            "DB_CONNECTION=mysql\nREDIS_HOST=127.0.0.1\n# MONGODB_URI=mongodb://disabled\n",
        )
        .expect("env");
        fs::write(
            path.join("docker-compose.yml"),
            "services:\n  cache:\n    image: redis:7\n  db:\n    image: postgres:16\n# mongodb: disabled example\n",
        )
        .expect("compose");

        let detection = detect(&path).expect("detect");
        assert_eq!(detection.databases, vec!["mysql"]);
        assert_eq!(detection.services, vec!["postgres", "redis"]);
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn malformed_json_manifests_fail_closed() {
        for name in ["package.json", "composer.json"] {
            let path = fixture("malformed");
            fs::write(path.join(name), "{not-json").expect("manifest");
            let error = dependency_report(&path).expect_err("malformed manifest must fail");
            assert!(error.to_string().contains(&format!("invalid {name}")));
            let _ = fs::remove_dir_all(path);
        }
    }

    #[test]
    fn npm_remediation_uses_install_without_lock_and_ci_with_lock() {
        let unlocked = fixture("npm-unlocked");
        fs::write(
            unlocked.join("package.json"),
            "{\"engines\":{\"node\":\">=18\"}}",
        )
        .expect("package");
        let unlocked_report = dependency_report(&unlocked).expect("unlocked report");
        let unlocked_commands: Vec<_> = unlocked_report
            .remediation
            .iter()
            .filter_map(|step| step.command.clone())
            .collect();
        assert!(unlocked_commands.contains(&vec!["npm".into(), "install".into()]));
        assert!(!unlocked_commands.contains(&vec!["npm".into(), "ci".into()]));

        let locked = fixture("npm-locked");
        fs::write(
            locked.join("package.json"),
            "{\"engines\":{\"node\":\">=18\"}}",
        )
        .expect("package");
        fs::write(locked.join("package-lock.json"), "{\"lockfileVersion\":3}").expect("lock");
        let locked_report = dependency_report(&locked).expect("locked report");
        let locked_commands: Vec<_> = locked_report
            .remediation
            .iter()
            .filter_map(|step| step.command.clone())
            .collect();
        assert!(locked_commands.contains(&vec!["npm".into(), "ci".into()]));

        let _ = fs::remove_dir_all(unlocked);
        let _ = fs::remove_dir_all(locked);
    }

    #[test]
    fn bootstrap_rollback_removes_new_destination() {
        let root = fixture("rollback-new");
        let destination = root.join("app");
        let state = bootstrap_destination_state(&destination).expect("state");
        assert_eq!(state, BootstrapDestinationState::Absent);
        fs::create_dir(&destination).expect("destination");
        fs::write(destination.join("partial.txt"), "partial").expect("partial");
        rollback_bootstrap_destination(&destination, state).expect("rollback");
        assert!(!destination.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn bootstrap_rollback_restores_existing_empty_destination() {
        let root = fixture("rollback-existing");
        let destination = root.join("app");
        fs::create_dir(&destination).expect("destination");
        let state = bootstrap_destination_state(&destination).expect("state");
        assert_eq!(state, BootstrapDestinationState::ExistingEmptyDirectory);
        fs::write(destination.join("partial.txt"), "partial").expect("partial");
        fs::create_dir(destination.join("nested")).expect("nested");
        fs::write(destination.join("nested/file.txt"), "partial").expect("nested file");
        rollback_bootstrap_destination(&destination, state).expect("rollback");
        assert!(destination.is_dir());
        assert!(destination
            .read_dir()
            .expect("read destination")
            .next()
            .is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn bootstrap_output_capture_is_bounded_and_marks_truncation() {
        let mut input = vec![b'a'; 16];
        input.extend(vec![b'b'; 24]);
        let output = read_bootstrap_output(std::io::Cursor::new(input), 32).expect("capture");
        assert_eq!(output.bytes.len(), 32);
        assert!(output.truncated);
        assert_eq!(&output.bytes[..8], &[b'a'; 8]);
        assert_eq!(&output.bytes[8..], &[b'b'; 24]);
    }

    #[test]
    fn bootstrap_output_capture_keeps_exact_limit_without_truncation() {
        let output =
            read_bootstrap_output(std::io::Cursor::new(vec![b'x'; 32]), 32).expect("capture");
        assert_eq!(output.bytes, vec![b'x'; 32]);
        assert!(!output.truncated);
    }
}
