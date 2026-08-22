use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use vsn_runtime::{install_from_artifact, RuntimeInstallPlan};

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

#[test]
fn stale_install_transaction_rejects_paths_outside_managed_runtime_root() {
    let root = unique_temp_dir("managed");
    let outside = unique_temp_dir("outside");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&outside).unwrap();

    let artifact = root.join("runtime.bin");
    let artifact_bytes = b"vsn-runtime-transaction-safety";
    fs::write(&artifact, artifact_bytes).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(artifact_bytes);
    let sha256 = format!("{:x}", hasher.finalize());

    let plan = RuntimeInstallPlan {
        runtime: "txn-safety".into(),
        version: "1.0.0".into(),
        target: "test/test".into(),
        url: "file://fixture".into(),
        sha256,
        archive: "binary".into(),
        install_dir: root.join("txn-safety").join("1.0.0"),
        executable_relpath: "bin/tool".into(),
    };

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
        Some(b"registry sentinel".as_slice()),
        "transaction recovery must never delete an outside registry path"
    );
    assert_eq!(
        shim_after.as_deref(),
        Some(b"shim sentinel".as_slice()),
        "transaction recovery must never delete an outside shim path"
    );
}
