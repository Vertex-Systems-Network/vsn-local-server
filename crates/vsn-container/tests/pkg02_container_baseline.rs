use std::{
    env, fs,
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

struct BackendAliases {
    docker: PathBuf,
    podman: PathBuf,
}

impl BackendAliases {
    fn paths() -> (PathBuf, PathBuf) {
        let exe = env::current_exe().expect("resolve test executable");
        let dir = exe.parent().expect("resolve test executable directory");
        (
            dir.join(if cfg!(windows) { "docker.exe" } else { "docker" }),
            dir.join(if cfg!(windows) { "podman.exe" } else { "podman" }),
        )
    }

    fn clear() {
        let (docker, podman) = Self::paths();
        let _ = fs::remove_file(docker);
        let _ = fs::remove_file(podman);
    }

    fn install(source: &Path) -> Self {
        let (docker, podman) = Self::paths();
        let _ = fs::remove_file(&docker);
        let _ = fs::remove_file(&podman);
        fs::copy(source, &docker).expect("copy fake docker beside test executable");
        fs::copy(source, &podman).expect("copy fake podman beside test executable");
        let permissions = fs::metadata(source)
            .expect("fake backend metadata")
            .permissions();
        fs::set_permissions(&docker, permissions.clone()).expect("set fake docker permissions");
        fs::set_permissions(&podman, permissions).expect("set fake podman permissions");
        Self { docker, podman }
    }
}

impl Drop for BackendAliases {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.docker);
        let _ = fs::remove_file(&self.podman);
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
    let mode = env::var("VSN_FAKE_CONTAINER_MODE").unwrap_or_else(|_| "healthy".into());

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
        if mode == "daemon-down" || mode == "action-fail" {
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

fn fake_executable() -> PathBuf {
    fake_root().join(if cfg!(windows) {
        "fake-container.exe"
    } else {
        "fake-container"
    })
}

fn run_scenario(scenario: &str) {
    let test_exe = env::current_exe().expect("resolve current test executable");
    let test_dir = test_exe
        .parent()
        .expect("resolve current test directory")
        .to_path_buf();
    let empty = fake_root().join("empty");
    fs::create_dir_all(&empty).expect("create empty backend search directory");
    let search_dir = if scenario == "missing" {
        &empty
    } else {
        &test_dir
    };

    let output = Command::new(&test_exe)
        .args(["--exact", "fixture_child", "--nocapture"])
        .current_dir(search_dir)
        .env("PATH", search_dir)
        .env_remove("NoDefaultCurrentDirectoryInExePath")
        .env("VSN_0215_CHILD_SCENARIO", scenario)
        .env("VSN_0215_FAKE_EXE", fake_executable())
        .env("VSN_FAKE_CONTAINER_MODE", scenario)
        .output()
        .expect("spawn isolated container fixture child");

    assert!(
        output.status.success(),
        "fixture scenario {scenario} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn fixture_child() {
    let Ok(scenario) = env::var("VSN_0215_CHILD_SCENARIO") else {
        return;
    };

    let aliases = if scenario == "missing" {
        BackendAliases::clear();
        None
    } else {
        let source = PathBuf::from(
            env::var_os("VSN_0215_FAKE_EXE").expect("fake backend executable path"),
        );
        Some(BackendAliases::install(&source))
    };

    match scenario.as_str() {
        "healthy" => {
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
            let action =
                container_action("docker", "restart", "vsn-demo").expect("restart container");
            assert_eq!(action.action, "restart");
            assert_eq!(action.output, "vsn-demo");
        }
        "missing" => {
            let backends = detect_all();
            assert!(backends.iter().all(|backend| !backend.installed));
            assert!(backends
                .iter()
                .all(|backend| backend.daemon_reachable.is_none()));
        }
        "daemon-down" => {
            let backends = detect_all();
            assert!(backends.iter().all(|backend| backend.installed));
            assert!(backends
                .iter()
                .all(|backend| backend.daemon_reachable == Some(false)));
            assert!(list_containers("docker", true).is_err());
            assert!(container_action("docker", "start", "vsn-demo").is_err());
        }
        "hang-detect" => {
            let started = Instant::now();
            let backends = detect_all();
            assert!(started.elapsed() < Duration::from_secs(4));
            assert!(backends.iter().all(|backend| !backend.installed));
        }
        "huge-read" => {
            let started = Instant::now();
            assert!(list_containers("docker", true).is_err());
            assert!(started.elapsed() < Duration::from_secs(5));
        }
        "action-fail" => {
            assert!(container_action("docker", "start", "vsn-demo").is_err());
        }
        "hang-action" => {
            let started = Instant::now();
            assert!(container_action("docker", "stop", "vsn-demo").is_err());
            assert!(started.elapsed() < Duration::from_secs(5));
        }
        "huge-action" => {
            let started = Instant::now();
            assert!(container_action("docker", "restart", "vsn-demo").is_err());
            assert!(started.elapsed() < Duration::from_secs(5));
        }
        other => panic!("unknown fixture scenario: {other}"),
    }

    drop(aliases);
}

#[test]
fn healthy_discovery_and_normal_reads_are_deterministic() {
    let _lock = lock_environment();
    run_scenario("healthy");
}

#[test]
fn missing_backend_and_unavailable_daemon_fail_closed() {
    let _lock = lock_environment();
    run_scenario("missing");
    run_scenario("daemon-down");
}

#[test]
fn discovery_and_reads_are_bounded() {
    let _lock = lock_environment();
    run_scenario("hang-detect");
    run_scenario("huge-read");
}

#[test]
fn lifecycle_success_failure_timeout_and_oversize_are_bounded() {
    let _lock = lock_environment();
    run_scenario("action-fail");
    run_scenario("hang-action");
    run_scenario("huge-action");
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
