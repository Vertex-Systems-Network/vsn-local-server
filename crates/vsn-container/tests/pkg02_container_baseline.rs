use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{self, Command},
    sync::{Mutex, MutexGuard, OnceLock},
    time::{Duration, Instant},
};
use vsn_container::{
    container_action, container_inspect, container_logs, container_stats, detect_all,
    list_containers, list_images, list_networks, list_volumes, ContainerError,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());
static FAKE_ROOT: OnceLock<PathBuf> = OnceLock::new();

struct PathGuard {
    path: Option<OsString>,
}

impl PathGuard {
    fn install(path: &Path) -> Self {
        let guard = Self {
            path: env::var_os("PATH"),
        };
        env::set_var("PATH", path);
        guard
    }
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        match &self.path {
            Some(value) => env::set_var("PATH", value),
            None => env::remove_var("PATH"),
        }
    }
}

fn lock_environment() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn fake_root() -> &'static PathBuf {
    FAKE_ROOT.get_or_init(|| {
        let root = env::temp_dir().join(format!("vsn-pkg02-0215-test-{}", process::id()));
        fs::create_dir_all(&root).expect("create fake backend root");
        let source = root.join("fake_container_backend.rs");
        fs::write(
            &source,
            r#"
use std::{env, process, thread, time::Duration};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let exe = env::current_exe().expect("resolve fake backend path");
    let mode = exe
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("healthy");

    if args.first().map(String::as_str) == Some("--version") {
        if mode == "hang-detect" {
            thread::sleep(Duration::from_secs(10));
        }
        println!("Container Engine version 99.0.0, build vsn-test");
        return;
    }
    if args.first().map(String::as_str) == Some("info") {
        if mode == "daemon-down" {
            eprintln!("daemon unavailable");
            process::exit(125);
        }
        println!("99.0.0");
        return;
    }
    if args.first().map(String::as_str) == Some("ps") {
        if mode == "daemon-down" {
            eprintln!("daemon unavailable");
            process::exit(125);
        }
        if mode == "huge-read" {
            print!("{}", "x".repeat(256 * 1024));
            return;
        }
        println!("abc123\tvsn-demo\timage:test\tUp 1 minute\t127.0.0.1:8080->80/tcp");
        return;
    }
    if args.first().map(String::as_str) == Some("image") && args.get(1).map(String::as_str) == Some("ls") {
        println!("img123\timage:test\t10MB");
        return;
    }
    if args.first().map(String::as_str) == Some("volume") && args.get(1).map(String::as_str) == Some("ls") {
        println!("vol1\tvol1\tlocal");
        return;
    }
    if args.first().map(String::as_str) == Some("network") && args.get(1).map(String::as_str) == Some("ls") {
        println!("net1\tbridge\tbridge");
        return;
    }
    if args.first().map(String::as_str) == Some("logs") {
        println!("line-one");
        println!("line-two");
        return;
    }
    if args.first().map(String::as_str) == Some("inspect") {
        println!("[{{\"Id\":\"abc123\",\"Name\":\"vsn-demo\"}}]");
        return;
    }
    if args.first().map(String::as_str) == Some("stats") {
        println!("vsn-demo\t1.00%\t10MiB / 1GiB\t1kB / 2kB\t3kB / 4kB\t5");
        return;
    }
    if matches!(args.first().map(String::as_str), Some("start" | "stop" | "restart" | "pause" | "unpause")) {
        if mode == "action-fail" {
            eprintln!("lifecycle failed");
            process::exit(42);
        }
        if mode == "hang-action" {
            thread::sleep(Duration::from_secs(10));
        }
        if mode == "huge-action" {
            print!("{}", "x".repeat(256 * 1024));
            return;
        }
        println!("{}", args.get(1).map(String::as_str).unwrap_or("vsn-demo"));
        return;
    }

    eprintln!("unsupported fake container args: {:?}", args);
    process::exit(97);
}
"#,
        )
        .expect("write fake backend source");

        let executable = root.join(if cfg!(windows) {
            "fake-container.exe"
        } else {
            "fake-container"
        });
        let status = Command::new("rustc")
            .arg(&source)
            .arg("-O")
            .arg("-o")
            .arg(&executable)
            .status()
            .expect("run rustc for fake backend");
        assert!(status.success(), "fake backend compilation failed");
        root
    })
}

fn fake_bin(mode: &str) -> PathBuf {
    let root = fake_root();
    let mode_dir = root.join(mode);
    fs::create_dir_all(&mode_dir).expect("create fake backend mode directory");
    let executable = root.join(if cfg!(windows) {
        "fake-container.exe"
    } else {
        "fake-container"
    });
    let docker = mode_dir.join(if cfg!(windows) { "docker.exe" } else { "docker" });
    let podman = mode_dir.join(if cfg!(windows) { "podman.exe" } else { "podman" });
    fs::copy(&executable, &docker).expect("copy fake docker backend");
    fs::copy(&executable, &podman).expect("copy fake podman backend");
    let permissions = fs::metadata(&executable)
        .expect("fake backend metadata")
        .permissions();
    fs::set_permissions(&docker, permissions.clone()).expect("set fake docker permissions");
    fs::set_permissions(&podman, permissions).expect("set fake podman permissions");
    mode_dir
}

#[test]
fn healthy_discovery_and_normal_reads_are_deterministic() {
    let _lock = lock_environment();
    let bin = fake_bin("healthy");
    let _guard = PathGuard::install(&bin);

    let backends = detect_all();
    assert_eq!(backends.len(), 2);
    assert_eq!(backends[0].id, "docker");
    assert_eq!(backends[1].id, "podman");
    assert!(backends.iter().all(|backend| backend.installed));
    assert!(backends
        .iter()
        .all(|backend| backend.daemon_reachable == Some(true)));

    let containers = list_containers("docker", true).expect("list containers");
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].name, "vsn-demo");
    assert_eq!(list_images("docker").expect("list images").len(), 1);
    assert_eq!(list_volumes("docker").expect("list volumes").len(), 1);
    assert_eq!(list_networks("docker").expect("list networks").len(), 1);
    assert!(container_logs("docker", "vsn-demo", 200)
        .expect("container logs")
        .contains("line-two"));
    assert!(container_inspect("docker", "vsn-demo")
        .expect("container inspect")
        .contains("abc123"));
    let stats = container_stats("docker", "vsn-demo").expect("container stats");
    assert_eq!(stats.name, "vsn-demo");
    assert_eq!(stats.pids, "5");
}

#[test]
fn missing_backend_and_unavailable_daemon_fail_closed() {
    let _lock = lock_environment();
    let empty = env::temp_dir().join(format!("vsn-pkg02-0215-empty-{}", process::id()));
    let _ = fs::remove_dir_all(&empty);
    fs::create_dir_all(&empty).expect("create empty PATH");
    {
        let _guard = PathGuard::install(&empty);
        let backends = detect_all();
        assert!(backends.iter().all(|backend| !backend.installed));
        assert!(backends
            .iter()
            .all(|backend| backend.daemon_reachable.is_none()));
    }
    let _ = fs::remove_dir_all(&empty);

    let bin = fake_bin("daemon-down");
    let _guard = PathGuard::install(&bin);
    let backends = detect_all();
    assert!(backends.iter().all(|backend| backend.installed));
    assert!(backends
        .iter()
        .all(|backend| backend.daemon_reachable == Some(false)));
    assert!(list_containers("docker", true).is_err());
    assert!(container_action("docker", "start", "vsn-demo").is_err());
}

#[test]
fn discovery_and_reads_are_bounded() {
    let _lock = lock_environment();
    {
        let bin = fake_bin("hang-detect");
        let _guard = PathGuard::install(&bin);
        let started = Instant::now();
        let backends = detect_all();
        assert!(started.elapsed() < Duration::from_secs(4));
        assert!(backends.iter().all(|backend| !backend.installed));
    }
    {
        let bin = fake_bin("huge-read");
        let _guard = PathGuard::install(&bin);
        let started = Instant::now();
        assert!(list_containers("docker", true).is_err());
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}

#[test]
fn lifecycle_success_failure_timeout_and_oversize_are_bounded() {
    let _lock = lock_environment();
    {
        let bin = fake_bin("healthy");
        let _guard = PathGuard::install(&bin);
        let result = container_action("docker", "restart", "vsn-demo").expect("restart");
        assert_eq!(result.action, "restart");
        assert_eq!(result.output, "vsn-demo");
    }
    {
        let bin = fake_bin("action-fail");
        let _guard = PathGuard::install(&bin);
        assert!(container_action("docker", "start", "vsn-demo").is_err());
    }
    {
        let bin = fake_bin("hang-action");
        let _guard = PathGuard::install(&bin);
        let started = Instant::now();
        assert!(container_action("docker", "stop", "vsn-demo").is_err());
        assert!(started.elapsed() < Duration::from_secs(5));
    }
    {
        let bin = fake_bin("huge-action");
        let _guard = PathGuard::install(&bin);
        let started = Instant::now();
        assert!(container_action("docker", "restart", "vsn-demo").is_err());
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}

#[test]
fn invalid_backend_and_target_are_rejected_before_execution() {
    assert!(matches!(
        list_containers("sh", true),
        Err(ContainerError::Unsupported(_))
    ));
    assert!(matches!(
        container_action("docker", "start", "bad target"),
        Err(ContainerError::Invalid(_))
    ));
}
