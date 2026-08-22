mod base {
    include!("lib_base.rs");
}

pub use base::*;

fn audit_runtime_id_is_safe(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_'))
}

fn audit_version_is_safe(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+'))
}

/// Fail closed before the legacy filesystem-derived runtime audit when registry
/// metadata cannot safely participate in managed path or shim derivation.
///
/// Valid registries continue through the canonical implementation unchanged.
/// Corrupt registries are reported at the metadata boundary without touching
/// paths derived from attacker-controlled runtime/version strings.
pub fn audit_registry(path: &std::path::Path) -> Result<RuntimeAuditReport, RuntimeError> {
    let registry = base::load_registry(path)?;
    let provider_runtime_ids = base::builtins()
        .into_iter()
        .map(|runtime| runtime.id)
        .collect::<std::collections::HashSet<_>>();
    let mut issues = Vec::new();
    let mut registrations = std::collections::HashSet::new();
    let mut known_safe = std::collections::HashSet::new();
    let mut unsafe_metadata = false;

    for item in &registry.installed {
        let key = (item.runtime.clone(), item.version.clone());
        if !registrations.insert(key.clone()) {
            issues.push(RuntimeAuditIssue {
                severity: RuntimeAuditSeverity::Error,
                code: "duplicate_registration".into(),
                runtime: Some(item.runtime.clone()),
                version: Some(item.version.clone()),
                message: "runtime registry contains a duplicate runtime/version registration"
                    .into(),
            });
        }

        if !audit_runtime_id_is_safe(&item.runtime) || !audit_version_is_safe(&item.version) {
            unsafe_metadata = true;
            issues.push(RuntimeAuditIssue {
                severity: RuntimeAuditSeverity::Error,
                code: "invalid_registration".into(),
                runtime: Some(item.runtime.clone()),
                version: Some(item.version.clone()),
                message: "runtime registry contains unsafe runtime/version metadata".into(),
            });
            continue;
        }

        known_safe.insert(key);
        if !provider_runtime_ids.contains(&item.runtime) {
            issues.push(RuntimeAuditIssue {
                severity: RuntimeAuditSeverity::Error,
                code: "unknown_runtime".into(),
                runtime: Some(item.runtime.clone()),
                version: Some(item.version.clone()),
                message: "runtime registry references an ID not reported by the active provider"
                    .into(),
            });
        }
    }

    if !unsafe_metadata {
        return base::audit_registry(path);
    }

    let mut activations = 0usize;
    for (project, map) in &registry.project_activation {
        for (runtime, version) in map {
            activations += 1;
            if !known_safe.contains(&(runtime.clone(), version.clone())) {
                issues.push(RuntimeAuditIssue {
                    severity: RuntimeAuditSeverity::Error,
                    code: "dangling_activation".into(),
                    runtime: Some(runtime.clone()),
                    version: Some(version.clone()),
                    message: format!("project activation references missing runtime: {project}"),
                });
            }
        }
    }

    issues.push(RuntimeAuditIssue {
        severity: RuntimeAuditSeverity::Warning,
        code: "filesystem_audit_skipped".into(),
        runtime: None,
        version: None,
        message: "filesystem-derived runtime and shim checks were skipped because unsafe registry metadata was present"
            .into(),
    });

    Ok(RuntimeAuditReport {
        installed: registry.installed.len(),
        activations,
        healthy: false,
        issues,
    })
}

#[cfg(test)]
mod facade_tests {
    use super::*;
    use std::{collections::BTreeMap, fs};

    #[test]
    fn audit_invalid_metadata_never_derives_filesystem_paths() {
        let root = std::env::temp_dir().join(format!(
            "vsn-runtime-invalid-metadata-audit-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let runtime_root = root.join("runtimes");
        let outside_install = runtime_root.join("..").join("1.0");
        fs::create_dir_all(&runtime_root).unwrap();
        fs::create_dir_all(&outside_install).unwrap();
        let sentinel = outside_install.join("keep.txt");
        let executable = outside_install.join("runtime.exe");
        fs::write(&sentinel, b"keep").unwrap();
        fs::write(&executable, b"outside-runtime").unwrap();
        let registry_path = runtime_root.join("registry.json");
        save_registry(
            &registry_path,
            &RuntimeRegistry {
                installed: vec![InstalledRuntime {
                    runtime: "..".into(),
                    version: "1.0".into(),
                    install_dir: outside_install,
                    executable,
                    source_sha256: "0".repeat(64),
                }],
                project_activation: BTreeMap::new(),
            },
        )
        .unwrap();

        let audit = audit_registry(&registry_path).unwrap();
        assert!(!audit.healthy);
        assert!(audit
            .issues
            .iter()
            .any(|issue| issue.code == "invalid_registration"));
        assert!(audit
            .issues
            .iter()
            .any(|issue| issue.code == "filesystem_audit_skipped"));
        for forbidden in [
            "install_dir_escape",
            "unexpected_install_dir",
            "missing_install_dir",
            "unsafe_install_object",
            "executable_path_escape",
            "missing_executable",
            "unsafe_executable_object",
            "missing_shim",
            "unsafe_shim",
            "stale_shim",
        ] {
            assert!(!audit.issues.iter().any(|issue| issue.code == forbidden));
        }
        assert_eq!(fs::read(&sentinel).unwrap(), b"keep");
        let _ = fs::remove_dir_all(root);
    }
}
