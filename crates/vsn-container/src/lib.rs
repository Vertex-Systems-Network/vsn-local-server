use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ContainerError {
    #[error("unsupported container backend: {0}")]
    Unsupported(String),
    #[error("container command failed: {0}")]
    Command(String),
    #[error("invalid container input: {0}")]
    Invalid(String),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContainerBackend {
    pub id: String,
    pub installed: bool,
    pub version: Option<String>,
    pub daemon_reachable: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub ports: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContainerActionResult {
    pub backend: String,
    pub target: String,
    pub action: String,
    pub output: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContainerResource {
    pub id: String,
    pub name: String,
    pub detail: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContainerBuildRequest {
    pub backend: String,
    pub context: PathBuf,
    pub tag: String,
    #[serde(default)]
    pub dockerfile: Option<PathBuf>,
    #[serde(default)]
    pub pull: bool,
    #[serde(default)]
    pub no_cache: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContainerExecRequest {
    pub backend: String,
    pub target: String,
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContainerStats {
    pub name: String,
    pub cpu_percent: String,
    pub memory: String,
    pub net_io: String,
    pub block_io: String,
    pub pids: String,
}

const BACKEND_VERSION_TIMEOUT: Duration = Duration::from_secs(3);
const BACKEND_INFO_TIMEOUT: Duration = Duration::from_secs(5);
const BASELINE_READ_TIMEOUT: Duration = Duration::from_secs(30);
const BASELINE_ACTION_TIMEOUT: Duration = Duration::from_secs(120);
const BACKEND_PROBE_OUTPUT_BYTES: usize = 64 * 1024;
const BASELINE_LIST_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const BASELINE_LOG_OUTPUT_BYTES: usize = 2 * 1024 * 1024;
const BASELINE_ACTION_OUTPUT_BYTES: usize = 2 * 1024 * 1024;

fn unavailable_backend(id: &str) -> ContainerBackend {
    ContainerBackend {
        id: id.into(),
        installed: false,
        version: None,
        daemon_reachable: None,
    }
}

pub fn detect_all() -> Vec<ContainerBackend> {
    let docker = std::thread::Builder::new()
        .name("vsn-container-detect-docker".into())
        .spawn(|| detect("docker"));
    let podman = std::thread::Builder::new()
        .name("vsn-container-detect-podman".into())
        .spawn(|| detect("podman"));
    vec![
        docker
            .ok()
            .and_then(|handle| handle.join().ok())
            .unwrap_or_else(|| unavailable_backend("docker")),
        podman
            .ok()
            .and_then(|handle| handle.join().ok())
            .unwrap_or_else(|| unavailable_backend("podman")),
    ]
}
fn detect(id: &str) -> ContainerBackend {
    let version = run_bounded(
        id,
        &["--version"],
        BACKEND_VERSION_TIMEOUT,
        BACKEND_PROBE_OUTPUT_BYTES,
    )
    .ok()
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty());
    let installed = version.is_some();
    let daemon_reachable = if installed {
        Some(
            run_bounded(
                id,
                &["info", "--format", "{{.ServerVersion}}"],
                BACKEND_INFO_TIMEOUT,
                BACKEND_PROBE_OUTPUT_BYTES,
            )
            .is_ok(),
        )
    } else {
        None
    };
    ContainerBackend {
        id: id.into(),
        installed,
        version,
        daemon_reachable,
    }
}

pub fn list_containers(backend: &str, all: bool) -> Result<Vec<ContainerInfo>, ContainerError> {
    validate_backend(backend)?;
    let mut args = vec!["ps"];
    if all {
        args.push("-a");
    }
    args.extend([
        "--format",
        "{{.ID}}\t{{.Names}}\t{{.Image}}\t{{.Status}}\t{{.Ports}}",
    ]);
    let output = run_bounded(
        backend,
        &args,
        BASELINE_READ_TIMEOUT,
        BASELINE_LIST_OUTPUT_BYTES,
    )?;
    Ok(output
        .lines()
        .filter_map(|line| {
            let p: Vec<&str> = line.split('\t').collect();
            if p.len() < 5 {
                None
            } else {
                Some(ContainerInfo {
                    id: p[0].into(),
                    name: p[1].into(),
                    image: p[2].into(),
                    status: p[3].into(),
                    ports: p[4].into(),
                })
            }
        })
        .collect())
}
pub fn list_images(backend: &str) -> Result<Vec<ContainerResource>, ContainerError> {
    list_resource(
        backend,
        &[
            "image",
            "ls",
            "--format",
            "{{.ID}}\t{{.Repository}}:{{.Tag}}\t{{.Size}}",
        ],
    )
}
pub fn list_volumes(backend: &str) -> Result<Vec<ContainerResource>, ContainerError> {
    list_resource(
        backend,
        &[
            "volume",
            "ls",
            "--format",
            "{{.Name}}\t{{.Name}}\t{{.Driver}}",
        ],
    )
}
pub fn list_networks(backend: &str) -> Result<Vec<ContainerResource>, ContainerError> {
    list_resource(
        backend,
        &[
            "network",
            "ls",
            "--format",
            "{{.ID}}\t{{.Name}}\t{{.Driver}}",
        ],
    )
}
pub fn container_logs(backend: &str, target: &str, tail: u32) -> Result<String, ContainerError> {
    validate_backend(backend)?;
    validate_target(target)?;
    let tail = tail.clamp(1, 5000).to_string();
    run_bounded(
        backend,
        &["logs", "--tail", &tail, target],
        BASELINE_READ_TIMEOUT,
        BASELINE_LOG_OUTPUT_BYTES,
    )
}
pub fn container_inspect(backend: &str, target: &str) -> Result<String, ContainerError> {
    validate_backend(backend)?;
    validate_target(target)?;
    run_bounded(
        backend,
        &["inspect", target],
        BASELINE_READ_TIMEOUT,
        4 * 1024 * 1024,
    )
}
pub fn container_stats(backend: &str, target: &str) -> Result<ContainerStats, ContainerError> {
    validate_backend(backend)?;
    validate_target(target)?;
    let raw = run_bounded(
        backend,
        &[
            "stats",
            "--no-stream",
            "--format",
            "{{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}\t{{.BlockIO}}\t{{.PIDs}}",
            target,
        ],
        BASELINE_READ_TIMEOUT,
        1024 * 1024,
    )?;
    let line = raw
        .lines()
        .next()
        .ok_or_else(|| ContainerError::Command("container stats returned no row".into()))?;
    let p = line.split('\t').collect::<Vec<_>>();
    if p.len() < 6 {
        return Err(ContainerError::Command(
            "container stats row format is incomplete".into(),
        ));
    }
    Ok(ContainerStats {
        name: p[0].into(),
        cpu_percent: p[1].into(),
        memory: p[2].into(),
        net_io: p[3].into(),
        block_io: p[4].into(),
        pids: p[5].into(),
    })
}
pub fn container_exec(
    request: &ContainerExecRequest,
) -> Result<ContainerActionResult, ContainerError> {
    validate_backend(&request.backend)?;
    validate_target(&request.target)?;
    validate_exec_program(&request.program)?;
    if request.args.len() > 128
        || request
            .args
            .iter()
            .any(|a| a.len() > 16_384 || a.chars().any(|c| c == '\0'))
    {
        return Err(ContainerError::Invalid(
            "container exec argument limit exceeded".into(),
        ));
    }
    let mut owned = vec![
        "exec".to_string(),
        request.target.clone(),
        request.program.clone(),
    ];
    owned.extend(request.args.clone());
    let refs = owned.iter().map(String::as_str).collect::<Vec<_>>();
    let output = run_bounded(
        &request.backend,
        &refs,
        BASELINE_ACTION_TIMEOUT,
        4 * 1024 * 1024,
    )?;
    Ok(ContainerActionResult {
        backend: request.backend.clone(),
        target: request.target.clone(),
        action: "container.exec".into(),
        output,
    })
}
fn list_resource(backend: &str, args: &[&str]) -> Result<Vec<ContainerResource>, ContainerError> {
    validate_backend(backend)?;
    let output = run_bounded(
        backend,
        args,
        BASELINE_READ_TIMEOUT,
        BASELINE_LIST_OUTPUT_BYTES,
    )?;
    Ok(output
        .lines()
        .filter_map(|line| {
            let p: Vec<&str> = line.split('\t').collect();
            if p.len() < 3 {
                None
            } else {
                Some(ContainerResource {
                    id: p[0].into(),
                    name: p[1].into(),
                    detail: p[2..].join(" · "),
                })
            }
        })
        .collect())
}
pub fn container_action(
    backend: &str,
    action: &str,
    target: &str,
) -> Result<ContainerActionResult, ContainerError> {
    validate_backend(backend)?;
    if !matches!(action, "start" | "stop" | "restart" | "pause" | "unpause") {
        return Err(ContainerError::Invalid(
            "unsupported container action".into(),
        ));
    }
    validate_target(target)?;
    let output = run_bounded(
        backend,
        &[action, target],
        BASELINE_ACTION_TIMEOUT,
        BASELINE_ACTION_OUTPUT_BYTES,
    )?;
    Ok(ContainerActionResult {
        backend: backend.into(),
        target: target.into(),
        action: action.into(),
        output: output.trim().into(),
    })
}
pub fn image_pull(backend: &str, image: &str) -> Result<ContainerActionResult, ContainerError> {
    validate_backend(backend)?;
    validate_target(image)?;
    let output = run_bounded(
        backend,
        &["image", "pull", image],
        Duration::from_secs(600),
        8 * 1024 * 1024,
    )?;
    Ok(ContainerActionResult {
        backend: backend.into(),
        target: image.into(),
        action: "image.pull".into(),
        output,
    })
}
pub fn image_build(
    request: &ContainerBuildRequest,
) -> Result<ContainerActionResult, ContainerError> {
    validate_backend(&request.backend)?;
    validate_target(&request.tag)?;
    let context = request
        .context
        .canonicalize()
        .map_err(|_| ContainerError::Invalid("container build context does not exist".into()))?;
    if !context.is_dir() {
        return Err(ContainerError::Invalid(
            "container build context must be a directory".into(),
        ));
    }
    let mut owned = vec!["build".to_string(), "--tag".into(), request.tag.clone()];
    if request.pull {
        owned.push("--pull".into());
    }
    if request.no_cache {
        owned.push("--no-cache".into());
    }
    if let Some(file) = &request.dockerfile {
        let file = file.canonicalize().map_err(|_| {
            ContainerError::Invalid("Dockerfile/Containerfile does not exist".into())
        })?;
        if !file.starts_with(&context) || !file.is_file() {
            return Err(ContainerError::Invalid(
                "build file must remain inside the build context".into(),
            ));
        }
        owned.push("--file".into());
        owned.push(file.display().to_string());
    }
    owned.push(context.display().to_string());
    let refs = owned.iter().map(String::as_str).collect::<Vec<_>>();
    let output = run_bounded(
        &request.backend,
        &refs,
        Duration::from_secs(900),
        8 * 1024 * 1024,
    )?;
    Ok(ContainerActionResult {
        backend: request.backend.clone(),
        target: request.tag.clone(),
        action: "image.build".into(),
        output,
    })
}
pub fn remove_resource(
    backend: &str,
    kind: &str,
    target: &str,
    force: bool,
) -> Result<ContainerActionResult, ContainerError> {
    validate_backend(backend)?;
    validate_target(target)?;
    let mut args = match kind {
        "container" => vec!["container", "rm"],
        "image" => vec!["image", "rm"],
        "volume" => vec!["volume", "rm"],
        "network" => vec!["network", "rm"],
        _ => {
            return Err(ContainerError::Invalid(
                "remove kind must be container, image, volume, or network".into(),
            ))
        }
    };
    if force && matches!(kind, "container" | "image") {
        args.push("--force");
    } else if force {
        return Err(ContainerError::Invalid(
            "force removal is only allowed for containers/images".into(),
        ));
    }
    args.push(target);
    let output = run_bounded(backend, &args, BASELINE_ACTION_TIMEOUT, 2 * 1024 * 1024)?;
    Ok(ContainerActionResult {
        backend: backend.into(),
        target: target.into(),
        action: format!("{kind}.remove"),
        output,
    })
}
fn run_bounded(
    program: &str,
    args: &[&str],
    timeout: Duration,
    max_output: usize,
) -> Result<String, ContainerError> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ContainerError::Command(e.to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ContainerError::Command("container stdout unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ContainerError::Command("container stderr unavailable".into()))?;
    let out_thread = std::thread::spawn(move || read_limited(stdout, max_output));
    let err_cap = max_output.min(1024 * 1024);
    let err_thread = std::thread::spawn(move || read_limited(stderr, err_cap));
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| ContainerError::Command(e.to_string()))?
        {
            break status;
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ContainerError::Command(format!(
                "container command timed out after {} ms",
                timeout.as_millis()
            )));
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let stdout = out_thread
        .join()
        .map_err(|_| ContainerError::Command("container stdout reader panicked".into()))??;
    let stderr = err_thread
        .join()
        .map_err(|_| ContainerError::Command("container stderr reader panicked".into()))??;
    let mut text = String::from_utf8_lossy(&stdout).into_owned();
    if !stderr.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&String::from_utf8_lossy(&stderr));
    }
    if !status.success() {
        return Err(ContainerError::Command(text.chars().take(8192).collect()));
    }
    Ok(text)
}
fn read_limited<R: Read>(reader: R, max: usize) -> Result<Vec<u8>, ContainerError> {
    let mut out = Vec::new();
    reader
        .take(max as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|e| ContainerError::Command(e.to_string()))?;
    if out.len() > max {
        return Err(ContainerError::Command(
            "container command output exceeded safety limit".into(),
        ));
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryPushRequest {
    pub backend: String,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub push: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryPushResult {
    pub backend: String,
    pub source: String,
    pub target: String,
    pub tagged: bool,
    pub pushed: bool,
    pub output: String,
}
pub fn tag_and_push(request: &RegistryPushRequest) -> Result<RegistryPushResult, ContainerError> {
    validate_backend(&request.backend)?;
    validate_target(&request.source)?;
    validate_target(&request.target)?;
    if !request.target.contains('/') && !request.target.contains(':') {
        return Err(ContainerError::Invalid(
            "registry target must include a registry/repository or explicit tag".into(),
        ));
    }
    let tag_output = run_bounded(
        &request.backend,
        &["image", "tag", &request.source, &request.target],
        Duration::from_secs(60),
        1024 * 1024,
    )?;
    let mut output = tag_output;
    let mut pushed = false;
    if request.push {
        let push_output = run_bounded(
            &request.backend,
            &["image", "push", &request.target],
            Duration::from_secs(900),
            8 * 1024 * 1024,
        )?;
        if !output.is_empty() && !push_output.is_empty() {
            output.push('\n');
        }
        output.push_str(&push_output);
        pushed = true;
    }
    Ok(RegistryPushResult {
        backend: request.backend.clone(),
        source: request.source.clone(),
        target: request.target.clone(),
        tagged: true,
        pushed,
        output,
    })
}

pub fn compose_action(
    backend: &str,
    project_dir: &Path,
    action: &str,
) -> Result<ContainerActionResult, ContainerError> {
    validate_backend(backend)?;
    if !project_dir.is_dir() {
        return Err(ContainerError::Invalid(
            "compose project directory does not exist".into(),
        ));
    }
    let args: &[&str] = match action {
        "up" => &["compose", "up", "-d"],
        "down" => &["compose", "down"],
        "stop" => &["compose", "stop"],
        "start" => &["compose", "start"],
        "restart" => &["compose", "restart"],
        "pull" => &["compose", "pull"],
        "build" => &["compose", "build"],
        "ps" => &["compose", "ps"],
        "logs" => &["compose", "logs", "--tail", "500"],
        _ => {
            return Err(ContainerError::Invalid(
                "compose action must be up, down, start, stop, restart, pull, build, ps, or logs"
                    .into(),
            ))
        }
    };
    let output = run_bounded_in_dir(
        backend,
        args,
        project_dir,
        Duration::from_secs(if matches!(action, "pull" | "build" | "up") {
            900
        } else {
            180
        }),
        8 * 1024 * 1024,
    )?;
    Ok(ContainerActionResult {
        backend: backend.into(),
        target: project_dir.display().to_string(),
        action: format!("compose.{action}"),
        output,
    })
}
fn run_bounded_in_dir(
    program: &str,
    args: &[&str],
    cwd: &Path,
    timeout: Duration,
    max_output: usize,
) -> Result<String, ContainerError> {
    let mut child = Command::new(program)
        .current_dir(cwd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ContainerError::Command(e.to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ContainerError::Command("container stdout unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ContainerError::Command("container stderr unavailable".into()))?;
    let out_thread = std::thread::spawn(move || read_limited(stdout, max_output));
    let err_thread = std::thread::spawn(move || read_limited(stderr, max_output.min(1024 * 1024)));
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| ContainerError::Command(e.to_string()))?
        {
            break status;
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ContainerError::Command(format!(
                "container command timed out after {} ms",
                timeout.as_millis()
            )));
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let stdout = out_thread
        .join()
        .map_err(|_| ContainerError::Command("container stdout reader panicked".into()))??;
    let stderr = err_thread
        .join()
        .map_err(|_| ContainerError::Command("container stderr reader panicked".into()))??;
    let mut text = String::from_utf8_lossy(&stdout).into_owned();
    if !stderr.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&String::from_utf8_lossy(&stderr));
    }
    if !status.success() {
        return Err(ContainerError::Command(text.chars().take(8192).collect()));
    }
    Ok(text)
}
fn validate_exec_program(value: &str) -> Result<(), ContainerError> {
    if value.is_empty() || value.len() > 512 || value.chars().any(|c| c.is_control() || c == '\0') {
        Err(ContainerError::Invalid(
            "unsafe container exec program".into(),
        ))
    } else {
        Ok(())
    }
}
fn validate_backend(value: &str) -> Result<(), ContainerError> {
    if matches!(value, "docker" | "podman") {
        Ok(())
    } else {
        Err(ContainerError::Unsupported(value.into()))
    }
}
fn validate_target(value: &str) -> Result<(), ContainerError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':' | b'/'))
    {
        Err(ContainerError::Invalid("unsafe container target".into()))
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_target_must_be_structured() {
        let r = RegistryPushRequest {
            backend: "sh".into(),
            source: "a".into(),
            target: "b".into(),
            push: false,
        };
        assert!(tag_and_push(&r).is_err());
    }
    #[test]
    fn exec_program_rejects_control_chars() {
        assert!(validate_exec_program("/bin/echo").is_ok());
        assert!(validate_exec_program("bad\ncmd").is_err());
    }
    #[test]
    fn removal_kind_is_allowlisted() {
        assert!(remove_resource("sh", "image", "x", false).is_err());
    }
    #[test]
    fn known_backends_are_stable() {
        let ids: Vec<_> = detect_all().into_iter().map(|v| v.id).collect();
        assert_eq!(ids, vec!["docker", "podman"]);
    }
    #[test]
    fn backend_is_allowlisted() {
        assert!(validate_backend("docker").is_ok());
        assert!(validate_backend("sh").is_err());
    }
}
