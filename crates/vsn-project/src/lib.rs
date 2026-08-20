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
        let upper = env.to_ascii_uppercase();
        for (needle, id) in [
            ("DB_CONNECTION=MYSQL", "mysql"),
            ("DB_CONNECTION=PGSQL", "postgresql"),
            ("DB_CONNECTION=SQLITE", "sqlite"),
            ("MONGODB", "mongodb"),
            ("REDIS_HOST", "redis"),
        ] {
            if upper.contains(needle) && !dbs.contains(&id.to_string()) {
                dbs.push(id.into());
            }
        }
        if upper.contains("REDIS_") {
            services.push("redis".into());
        }
    }
    if let Ok(compose) = fs::read_to_string(path.join("docker-compose.yml"))
        .or_else(|_| fs::read_to_string(path.join("compose.yml")))
    {
        let lower = compose.to_ascii_lowercase();
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

    if let Ok(raw) = fs::read_to_string(path.join("composer.json")) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
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
    }
    if let Ok(raw) = fs::read_to_string(path.join("package.json")) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
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
        remediation.push(RemediationStep {
            category: "dependency".into(),
            description: format!("Install {manager} dependencies"),
            command: Some(vec![
                manager.into(),
                if manager == "npm" {
                    "ci".into()
                } else {
                    "install".into()
                },
            ]),
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
    if destination.exists() {
        if !destination.is_dir() {
            return Err(ProjectError::Invalid(
                "bootstrap destination must be a directory".into(),
            ));
        }
        if destination.read_dir()?.next().is_some() {
            return Err(ProjectError::Invalid(
                "bootstrap destination must be empty".into(),
            ));
        }
    }
    let mut command = Command::new(&plan.program);
    match plan.template.as_str() {
        "laravel" | "rust" => {
            command.current_dir(parent);
        }
        "node" | "django" | "go" => {
            fs::create_dir_all(destination)?;
            command.current_dir(destination);
        }
        _ => {
            return Err(ProjectError::Invalid(
                "bootstrap template is not executable".into(),
            ))
        }
    }
    command
        .args(&plan.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| ProjectError::Command(format!("failed to start {}: {e}", plan.program)))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ProjectError::Command("bootstrap stdout unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ProjectError::Command("bootstrap stderr unavailable".into()))?;
    let out_thread = std::thread::spawn(move || read_bootstrap_output(stdout, 4 * 1024 * 1024));
    let err_thread = std::thread::spawn(move || read_bootstrap_output(stderr, 2 * 1024 * 1024));
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| ProjectError::Command(e.to_string()))?
        {
            break status;
        }
        if started.elapsed() > Duration::from_secs(15 * 60) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ProjectError::Command(
                "bootstrap exceeded 15 minute timeout".into(),
            ));
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let stdout = out_thread
        .join()
        .map_err(|_| ProjectError::Command("bootstrap stdout reader panicked".into()))??;
    let stderr = err_thread
        .join()
        .map_err(|_| ProjectError::Command("bootstrap stderr reader panicked".into()))??;
    Ok(BootstrapResult {
        template: plan.template.clone(),
        destination: destination.clone(),
        status_code: status.code(),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}
fn read_bootstrap_output<R: Read>(reader: R, max: usize) -> Result<Vec<u8>, ProjectError> {
    let mut out = Vec::new();
    reader.take(max as u64 + 1).read_to_end(&mut out)?;
    if out.len() > max {
        return Err(ProjectError::Command(
            "bootstrap output exceeded safety limit".into(),
        ));
    }
    Ok(out)
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
