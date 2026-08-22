use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use vsn_runtime::{
    audit_registry, load_registry, repair_registry, save_registry, uninstall_runtime, write_shim,
    InstalledRuntime, RuntimeRegistry,
};

fn fresh_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("vsn-pkg02-0212-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn installed(root: &Path, runtime: &str, version: &str) -> InstalledRuntime {
    let install_dir = root.join(runtime).join(version);
    fs::create_dir_all(&install_dir).unwrap();
    let executable = install_dir.join(format!("{runtime}.bin"));
    fs::write(&executable, runtime.as_bytes()).unwrap();
    InstalledRuntime {
        runtime: runtime.into(),
        version: version.into(),
        install_dir,
        executable,
        source_sha256: "0".repeat(64),
    }
}

#[test]
fn uninstall_rejects_registry_escape_without_touching_outside_data() {
    let top = fresh_root("escape");
    let root = top.join("runtimes");
    let outside = top.join("outside");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&outside).unwrap();
    let sentinel = outside.join("keep.txt");
    fs::write(&sentinel, b"must-survive").unwrap();
    let registry_path = root.join("registry.json");
    let mut activation = BTreeMap::new();
    activation.insert("node".into(), "20.0.0".into());
    save_registry(
        &registry_path,
        &RuntimeRegistry {
            installed: vec![InstalledRuntime {
                runtime: "node".into(),
                version: "20.0.0".into(),
                install_dir: outside.clone(),
                executable: sentinel.clone(),
                source_sha256: "0".repeat(64),
            }],
            project_activation: BTreeMap::from([("project".into(), activation)]),
        },
    )
    .unwrap();

    assert!(uninstall_runtime(&registry_path, "node", "20.0.0").is_err());
    assert_eq!(fs::read(&sentinel).unwrap(), b"must-survive");
    assert_eq!(load_registry(&registry_path).unwrap().installed.len(), 1);
    let _ = fs::remove_dir_all(top);
}

#[test]
fn repair_drops_unsafe_registration_and_preserves_healthy_runtime() {
    let top = fresh_root("repair");
    let root = top.join("runtimes");
    let outside = top.join("outside");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&outside).unwrap();
    let sentinel = outside.join("keep.txt");
    fs::write(&sentinel, b"must-survive").unwrap();
    let python = installed(&root, "python", "3.12.0");
    let registry_path = root.join("registry.json");
    let mut activation = BTreeMap::new();
    activation.insert("node".into(), "20.0.0".into());
    activation.insert("python".into(), "3.12.0".into());
    save_registry(
        &registry_path,
        &RuntimeRegistry {
            installed: vec![
                InstalledRuntime {
                    runtime: "node".into(),
                    version: "20.0.0".into(),
                    install_dir: outside.clone(),
                    executable: sentinel.clone(),
                    source_sha256: "0".repeat(64),
                },
                python.clone(),
            ],
            project_activation: BTreeMap::from([("project".into(), activation)]),
        },
    )
    .unwrap();

    let report = repair_registry(&registry_path).unwrap();
    assert!(report.removed_missing.contains(&"node@20.0.0".to_string()));
    assert_eq!(report.remaining_installed, 1);
    assert_eq!(fs::read(&sentinel).unwrap(), b"must-survive");
    let registry = load_registry(&registry_path).unwrap();
    assert_eq!(registry.installed, vec![python]);
    assert_eq!(registry.project_activation["project"].len(), 1);
    assert_eq!(registry.project_activation["project"]["python"], "3.12.0");
    assert!(audit_registry(&registry_path).unwrap().healthy);
    let _ = fs::remove_dir_all(top);
}

#[test]
fn uninstall_removes_target_shim_and_activation_but_preserves_sibling_runtime() {
    let top = fresh_root("sibling");
    let root = top.join("runtimes");
    fs::create_dir_all(&root).unwrap();
    let node = installed(&root, "node", "20.0.0");
    let python = installed(&root, "python", "3.12.0");
    let registry_path = root.join("registry.json");
    let mut activation = BTreeMap::new();
    activation.insert("node".into(), "20.0.0".into());
    activation.insert("python".into(), "3.12.0".into());
    save_registry(
        &registry_path,
        &RuntimeRegistry {
            installed: vec![node.clone(), python.clone()],
            project_activation: BTreeMap::from([("project".into(), activation)]),
        },
    )
    .unwrap();
    let shim_dir = root.join("shims");
    let node_shim = write_shim(&shim_dir, "node", &node.executable).unwrap();
    let python_shim = write_shim(&shim_dir, "python", &python.executable).unwrap();

    let registry = uninstall_runtime(&registry_path, "node", "20.0.0").unwrap();
    assert_eq!(registry.installed, vec![python.clone()]);
    assert!(!node.install_dir.exists());
    assert!(python.install_dir.exists());
    assert!(!node_shim.exists());
    assert!(python_shim.exists());
    assert_eq!(registry.project_activation["project"].len(), 1);
    assert_eq!(registry.project_activation["project"]["python"], "3.12.0");
    let _ = fs::remove_dir_all(top);
}

#[test]
fn duplicate_target_uninstall_fails_before_destructive_mutation() {
    let top = fresh_root("duplicate");
    let root = top.join("runtimes");
    fs::create_dir_all(&root).unwrap();
    let node = installed(&root, "node", "20.0.0");
    let outside = top.join("outside");
    fs::create_dir_all(&outside).unwrap();
    let sentinel = outside.join("keep.txt");
    fs::write(&sentinel, b"must-survive").unwrap();
    let registry_path = root.join("registry.json");
    save_registry(
        &registry_path,
        &RuntimeRegistry {
            installed: vec![
                node.clone(),
                InstalledRuntime {
                    runtime: "node".into(),
                    version: "20.0.0".into(),
                    install_dir: outside.clone(),
                    executable: sentinel.clone(),
                    source_sha256: "0".repeat(64),
                },
            ],
            project_activation: BTreeMap::new(),
        },
    )
    .unwrap();

    assert!(uninstall_runtime(&registry_path, "node", "20.0.0").is_err());
    assert!(node.install_dir.exists());
    assert_eq!(fs::read(&sentinel).unwrap(), b"must-survive");
    let _ = fs::remove_dir_all(top);
}
