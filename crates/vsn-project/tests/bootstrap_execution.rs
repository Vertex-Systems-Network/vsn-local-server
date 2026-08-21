use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use vsn_project::{execute_bootstrap, BootstrapPlan};

fn fixture(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vsn-bootstrap-integration-{name}-{nonce}"));
    fs::create_dir_all(&root).expect("create fixture");
    root
}

fn helper_plan(destination: &Path) -> BootstrapPlan {
    BootstrapPlan {
        template: "node".into(),
        destination: destination.to_path_buf(),
        program: std::env::current_exe()
            .expect("current test executable")
            .display()
            .to_string(),
        args: vec![
            "--exact".into(),
            "bootstrap_child_helper".into(),
            "--ignored".into(),
            "--nocapture".into(),
        ],
        requires_network: false,
    }
}

#[test]
#[ignore = "invoked as an isolated child process by bootstrap execution tests"]
fn bootstrap_child_helper() {
    let cwd = std::env::current_dir().expect("child cwd");
    let name = cwd
        .file_name()
        .and_then(|value| value.to_str())
        .expect("child destination name");
    match name {
        "success-app" => {
            fs::write(cwd.join("created.txt"), "created\n").expect("success marker");
            println!("bootstrap-child-success");
        }
        "verbose-success-app" => {
            fs::write(cwd.join("created.txt"), "created\n").expect("success marker");
            print!("{}", "x".repeat(128 * 1024));
        }
        "fail-new" | "fail-existing" => {
            fs::write(cwd.join("partial.txt"), "partial\n").expect("partial marker");
            eprintln!("controlled bootstrap failure");
            std::process::exit(42);
        }
        "verbose-failure-app" => {
            fs::write(cwd.join("partial.txt"), "partial\n").expect("partial marker");
            eprint!("{}", "e".repeat(96 * 1024));
            std::process::exit(42);
        }
        other => panic!("unexpected bootstrap child destination: {other}"),
    }
}

#[test]
fn successful_bootstrap_returns_zero_and_keeps_created_destination() {
    let root = fixture("success");
    let destination = root.join("success-app");
    let result = execute_bootstrap(&helper_plan(&destination)).expect("bootstrap success");

    assert_eq!(result.status_code, Some(0));
    assert!(result.stdout.contains("bootstrap-child-success"));
    assert!(!result.stdout_truncated);
    assert!(!result.stderr_truncated);
    assert!(destination.join("created.txt").is_file());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn verbose_success_is_bounded_without_turning_into_failure() {
    let root = fixture("verbose-success");
    let destination = root.join("verbose-success-app");
    let result = execute_bootstrap(&helper_plan(&destination)).expect("verbose bootstrap success");

    assert_eq!(result.status_code, Some(0));
    assert_eq!(result.stdout.len(), 64 * 1024);
    assert!(
        result.stdout.bytes().filter(|byte| *byte == b'x').count() > 60 * 1024,
        "bounded tail should retain the verbose child output"
    );
    assert!(result.stdout_truncated);
    assert!(!result.stderr_truncated);
    assert!(destination.join("created.txt").is_file());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn nonzero_child_rolls_back_new_destination_and_is_retry_safe() {
    let root = fixture("fail-new");
    let destination = root.join("fail-new");

    for _ in 0..2 {
        assert!(!destination.exists());
        let error = execute_bootstrap(&helper_plan(&destination))
            .expect_err("non-zero child must fail the bootstrap");
        let message = error.to_string();
        assert!(message.contains("status 42"), "unexpected error: {message}");
        assert!(
            message.contains("controlled bootstrap failure"),
            "unexpected error: {message}"
        );
        assert!(!destination.exists());
    }

    let _ = fs::remove_dir_all(root);
}

#[test]
fn verbose_nonzero_stderr_is_bounded_and_rolls_back() {
    let root = fixture("verbose-failure");
    let destination = root.join("verbose-failure-app");

    let error = execute_bootstrap(&helper_plan(&destination))
        .expect_err("verbose non-zero child must fail the bootstrap");
    let message = error.to_string();
    assert!(message.contains("status 42"), "unexpected error: {message}");
    assert!(
        message.contains("stderr truncated"),
        "unexpected error: {message}"
    );
    assert!(message.len() < 40 * 1024, "failure must remain IPC-bounded");
    assert!(!destination.exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn nonzero_child_restores_preexisting_empty_destination() {
    let root = fixture("fail-existing");
    let destination = root.join("fail-existing");
    fs::create_dir(&destination).expect("existing empty destination");

    for _ in 0..2 {
        let error = execute_bootstrap(&helper_plan(&destination))
            .expect_err("non-zero child must fail the bootstrap");
        assert!(error.to_string().contains("status 42"));
        assert!(destination.is_dir());
        assert!(destination
            .read_dir()
            .expect("read restored destination")
            .next()
            .is_none());
    }

    let _ = fs::remove_dir_all(root);
}

#[test]
fn nonempty_destination_is_rejected_without_mutation() {
    let root = fixture("nonempty");
    let destination = root.join("nonempty-app");
    fs::create_dir(&destination).expect("nonempty destination");
    let sentinel = destination.join("keep.txt");
    fs::write(&sentinel, "keep\n").expect("sentinel");

    let error = execute_bootstrap(&helper_plan(&destination))
        .expect_err("nonempty destination must be rejected");
    assert!(error.to_string().contains("destination must be empty"));
    assert_eq!(fs::read_to_string(&sentinel).expect("sentinel remains"), "keep\n");
    assert_eq!(
        destination
            .read_dir()
            .expect("read destination")
            .count(),
        1
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn spawn_failure_rolls_back_automatically_created_destination() {
    let root = fixture("spawn-failure");
    let destination = root.join("spawn-failure-app");
    let plan = BootstrapPlan {
        template: "node".into(),
        destination: destination.clone(),
        program: "__vsn_missing_bootstrap_program_for_test__".into(),
        args: vec![],
        requires_network: false,
    };

    let error = execute_bootstrap(&plan).expect_err("missing child program must fail");
    assert!(error.to_string().contains("failed to start"));
    assert!(!destination.exists());

    let _ = fs::remove_dir_all(root);
}
