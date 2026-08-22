use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use vsn_runtime::{install_from_artifact, register_runtime, write_shim, RuntimeInstallPlan};

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "vsn-pkg02-0211-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn artifact_fixture(root: &Path, bytes: &[u8]) -> (PathBuf, String) {
    let artifact = root.join("runtime.bin");
    fs::write(&artifact, bytes).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    (artifact, format!("{:x}", hasher.finalize()))
}

fn install_plan(root: &Path, runtime: &str, sha256: String) -> RuntimeInstallPlan {
    RuntimeInstallPlan {
        runtime: runtime.into(),
        version: "1.0.0".into(),
        target: "test/test".into(),
        url: "file://fixture".into(),
        sha256,
        archive: "binary".into(),
        install_dir: root.join(runtime).join("1.0.0"),
        executable_relpath: "bin/tool".into(),
    }
}

fn expected_shim_path(root: &Path, runtime: &str) -> PathBuf {
    #[cfg(windows)]
    {
        root.join("shims").join(format!("{runtime}.cmd"))
    }
    #[cfg(not(windows))]
    {
        root.join("shims").join(runtime)
    }
}

#[test]
fn stale_install_transaction_rejects_paths_outside_managed_runtime_root() {
    let root = unique_temp_dir("managed");
    let outside = unique_temp_dir("outside");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&outside).unwrap();

    let (artifact, sha256) = artifact_fixture(&root, b"vsn-runtime-transaction-safety");
    let plan = install_plan(&root, "txn-safety", sha256);

    let outside_registry = outside.join("registry-sentinel.json");
    let outside_shim = outside.join("shim-sentinel");
    fs::write(&outside_registry, b"registry sentinel").unwrap();
    fs::write(&outside_shim, b"shim sentinel").unwrap();

    let transaction_dir = root.join(".install-transactions").join("txn-safety");
    fs::create_dir_all(&transaction_dir).unwrap();
    let transaction = serde_json::json!({
        "runtime": "txn-safety",
        "version": "1.0.0",
        "install_dir": plan.install_dir.to_string_lossy(),
        "registry_path": outside_registry.to_string_lossy(),
        "shim_path": outside_shim.to_string_lossy(),
        "previous_install": false,
        "previous_registry": false,
        "previous_shim": "missing"
    });
    fs::write(
        transaction_dir.join("transaction.json"),
        serde_json::to_vec_pretty(&transaction).unwrap(),
    )
    .unwrap();

    let result = install_from_artifact(&plan, &artifact);
    let registry_after = fs::read(&outside_registry).ok();
    let shim_after = fs::read(&outside_shim).ok();

    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&outside);

    assert!(
        result.is_err(),
        "a stale transaction with paths outside the managed runtime root must fail closed"
    );
    assert_eq!(
        registry_after.as_deref(),
        Some(&b"registry sentinel"[..]),
        "transaction recovery must never delete an outside registry path"
    );
    assert_eq!(
        shim_after.as_deref(),
        Some(&b"shim sentinel"[..]),
        "transaction recovery must never delete an outside shim path"
    );
}

#[test]
fn valid_stale_install_transaction_is_recovered_before_reinstall() {
    let root = unique_temp_dir("valid-recovery");
    fs::create_dir_all(&root).unwrap();

    let (artifact, sha256) = artifact_fixture(&root, b"vsn-runtime-valid-recovery");
    let plan = install_plan(&root, "txn-recover", sha256);
    let registry_path = root.join("registry.json");
    let shim_path = expected_shim_path(&root, "txn-recover");
    let transaction_dir = root.join(".install-transactions").join("txn-recover");
    fs::create_dir_all(&transaction_dir).unwrap();
    let transaction = serde_json::json!({
        "runtime": "txn-recover",
        "version": "1.0.0",
        "install_dir": plan.install_dir.to_string_lossy(),
        "registry_path": registry_path.to_string_lossy(),
        "shim_path": shim_path.to_string_lossy(),
        "previous_install": false,
        "previous_registry": false,
        "previous_shim": "missing"
    });
    fs::write(
        transaction_dir.join("transaction.json"),
        serde_json::to_vec_pretty(&transaction).unwrap(),
    )
    .unwrap();

    let installed = install_from_artifact(&plan, &artifact).unwrap();
    let registry = register_runtime(&registry_path, installed.clone()).unwrap();
    let shim = write_shim(&root.join("shims"), "txn-recover", &installed.executable).unwrap();

    assert!(installed.executable.is_file());
    assert_eq!(registry.installed.len(), 1);
    assert_eq!(registry.installed[0].runtime, "txn-recover");
    assert_eq!(shim, shim_path);
    assert!(shim.is_file());
    assert!(!transaction_dir.exists());

    let _ = fs::remove_dir_all(&root);
}
