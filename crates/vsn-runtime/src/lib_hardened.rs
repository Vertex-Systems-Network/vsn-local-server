#[path = "lib.rs"]
mod legacy;

pub use legacy::*;

use serde_json::{Map, Value};
use std::{collections::BTreeSet, fs, path::Path};

fn invalid(message: impl Into<String>) -> RuntimeError {
    RuntimeError::Invalid(message.into())
}

fn object<'a>(value: &'a Value, context: &str) -> Result<&'a Map<String, Value>, RuntimeError> {
    value
        .as_object()
        .ok_or_else(|| invalid(format!("{context} must be a JSON object")))
}

fn reject_unknown_fields(
    object: &Map<String, Value>,
    allowed: &[&str],
    context: &str,
) -> Result<(), RuntimeError> {
    for key in object.keys() {
        if !allowed.iter().any(|allowed_key| key == allowed_key) {
            return Err(invalid(format!("unknown {context} field: {key}")));
        }
    }
    Ok(())
}

fn required_string<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    context: &str,
) -> Result<&'a str, RuntimeError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid(format!("{context}.{field} must be a non-empty string")))
}

fn validate_runtime_id_value(value: &str) -> Result<(), RuntimeError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_'))
    {
        return Err(invalid("unsafe runtime id"));
    }
    Ok(())
}

fn validate_version_value(value: &str) -> Result<(), RuntimeError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+'))
    {
        return Err(invalid("unsafe runtime version"));
    }
    Ok(())
}

fn normalize_catalog_os(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "win32" | "windows" => Some("windows"),
        "darwin" | "macos" => Some("macos"),
        "linux" => Some("linux"),
        _ => None,
    }
}

fn normalize_catalog_arch(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "x86_64" | "amd64" => Some("x86_64"),
        "aarch64" | "arm64" => Some("aarch64"),
        _ => None,
    }
}

fn validate_executable_relpath(value: &str) -> Result<(), RuntimeError> {
    let normalized = value.replace('\\', "/");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.starts_with("//")
        || normalized.as_bytes().get(1) == Some(&b':')
        || normalized
            .split('/')
            .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
    {
        return Err(invalid(
            "runtime executable_relpath must be a normalized relative path inside install directory",
        ));
    }
    Ok(())
}

fn validate_artifact(
    value: &Value,
    release_label: &str,
) -> Result<(String, String), RuntimeError> {
    let artifact = object(value, "runtime artifact")?;
    reject_unknown_fields(
        artifact,
        &["os", "arch", "url", "sha256", "archive", "executable_relpath"],
        "runtime artifact",
    )?;

    let os_raw = required_string(artifact, "os", "runtime artifact")?;
    let arch_raw = required_string(artifact, "arch", "runtime artifact")?;
    let os = normalize_catalog_os(os_raw)
        .ok_or_else(|| invalid(format!("unsupported runtime artifact OS: {os_raw}")))?;
    let arch = normalize_catalog_arch(arch_raw)
        .ok_or_else(|| invalid(format!("unsupported runtime artifact architecture: {arch_raw}")))?;

    let url = required_string(artifact, "url", "runtime artifact")?;
    if !url.starts_with("https://") && !url.starts_with("file://") {
        return Err(invalid(format!(
            "{release_label} artifact must use HTTPS or file://"
        )));
    }

    let sha256 = required_string(artifact, "sha256", "runtime artifact")?;
    if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(invalid(format!(
            "{release_label} artifact sha256 must be 64 hexadecimal characters"
        )));
    }

    let archive = required_string(artifact, "archive", "runtime artifact")?;
    if !matches!(archive, "zip" | "tar.gz" | "tgz" | "tar.xz" | "tar" | "binary") {
        return Err(invalid(format!("unsupported archive type: {archive}")));
    }

    let executable_relpath = required_string(
        artifact,
        "executable_relpath",
        "runtime artifact",
    )?;
    validate_executable_relpath(executable_relpath)?;

    Ok((os.to_string(), arch.to_string()))
}

fn validate_catalog_bytes(bytes: &[u8]) -> Result<(), RuntimeError> {
    let value: Value = serde_json::from_slice(bytes)?;
    let catalog = object(&value, "runtime catalog")?;
    reject_unknown_fields(
        catalog,
        &["schema_version", "provider", "runtimes", "signature"],
        "runtime catalog",
    )?;

    if catalog.get("schema_version").and_then(Value::as_u64) != Some(1) {
        return Err(invalid("unsupported or invalid runtime catalog"));
    }
    required_string(catalog, "provider", "runtime catalog")?;

    if let Some(signature) = catalog.get("signature") {
        if !signature.is_null() && signature.as_str().is_none() {
            return Err(invalid("runtime catalog signature must be a string or null"));
        }
    }

    let runtimes = catalog
        .get("runtimes")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("runtime catalog.runtimes must be an array"))?;

    let mut releases = BTreeSet::new();
    for release_value in runtimes {
        let release = object(release_value, "runtime release")?;
        reject_unknown_fields(
            release,
            &["runtime", "version", "artifacts"],
            "runtime release",
        )?;

        let runtime = required_string(release, "runtime", "runtime release")?;
        let version = required_string(release, "version", "runtime release")?;
        validate_runtime_id_value(runtime)?;
        validate_version_value(version)?;

        if !releases.insert((runtime.to_string(), version.to_string())) {
            return Err(invalid(format!(
                "duplicate runtime release: {runtime}@{version}"
            )));
        }

        let artifacts = release
            .get("artifacts")
            .and_then(Value::as_array)
            .filter(|artifacts| !artifacts.is_empty())
            .ok_or_else(|| invalid(format!("{runtime} {version} has no artifacts")))?;

        let release_label = format!("{runtime}@{version}");
        let mut targets = BTreeSet::new();
        for artifact in artifacts {
            let target = validate_artifact(artifact, &release_label)?;
            if !targets.insert(target.clone()) {
                return Err(invalid(format!(
                    "duplicate runtime artifact target for {release_label}: {}/{}",
                    target.0, target.1
                )));
            }
        }
    }

    Ok(())
}

fn validate_trust_bytes(bytes: &[u8]) -> Result<(), RuntimeError> {
    let value: Value = serde_json::from_slice(bytes)?;
    let trust = object(&value, "runtime catalog trust")?;
    reject_unknown_fields(trust, &["public_keys"], "runtime catalog trust")?;
    let keys = trust
        .get("public_keys")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("runtime catalog trust.public_keys must be an array"))?;
    for key in keys {
        if key.as_str().is_none_or(str::is_empty) {
            return Err(invalid(
                "runtime catalog trust public keys must be non-empty strings",
            ));
        }
    }
    Ok(())
}

pub fn load_catalog(path: &Path) -> Result<RuntimeCatalog, RuntimeError> {
    let bytes = fs::read(path)?;
    validate_catalog_bytes(&bytes)?;
    legacy::load_catalog(path)
}

pub fn load_catalog_verified(
    path: &Path,
    trust_path: &Path,
) -> Result<(RuntimeCatalog, String), RuntimeError> {
    let catalog_bytes = fs::read(path)?;
    validate_catalog_bytes(&catalog_bytes)?;
    let trust_bytes = fs::read(trust_path)?;
    validate_trust_bytes(&trust_bytes)?;
    legacy::load_catalog_verified(path, trust_path)
}

pub fn install_plan(
    catalog: &RuntimeCatalog,
    runtime: &str,
    version: &str,
    root: &Path,
) -> Result<RuntimeInstallPlan, RuntimeError> {
    let bytes = serde_json::to_vec(catalog)?;
    validate_catalog_bytes(&bytes)?;
    legacy::install_plan(catalog, runtime, version, root)
}

#[cfg(test)]
mod hardening_tests {
    use super::*;

    fn fixture_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "vsn-runtime-catalog-hardening-{}-{name}.json",
            std::process::id()
        ))
    }

    fn write_fixture(name: &str, json: &str) -> std::path::PathBuf {
        let path = fixture_path(name);
        let _ = fs::remove_file(&path);
        fs::write(&path, json).unwrap();
        path
    }

    fn safe_artifact(os: &str, arch: &str) -> String {
        format!(
            r#"{{"os":"{os}","arch":"{arch}","url":"https://example.invalid/runtime.bin","sha256":"{}","archive":"binary","executable_relpath":"bin/runtime"}}"#,
            "0".repeat(64)
        )
    }

    #[test]
    fn catalog_rejects_unknown_fields() {
        let artifact = safe_artifact("linux", "x86_64");
        let path = write_fixture(
            "unknown-field",
            &format!(
                r#"{{"schema_version":1,"provider":"vsn.test","runtimes":[{{"runtime":"node","version":"20.0.0","artifacts":[{artifact}]}}],"signature":null,"future_policy":true}}"#
            ),
        );
        assert!(load_catalog(&path).is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn catalog_rejects_duplicate_releases() {
        let artifact = safe_artifact("linux", "x86_64");
        let release = format!(
            r#"{{"runtime":"node","version":"20.0.0","artifacts":[{artifact}]}}"#
        );
        let path = write_fixture(
            "duplicate-release",
            &format!(
                r#"{{"schema_version":1,"provider":"vsn.test","runtimes":[{release},{release}],"signature":null}}"#
            ),
        );
        assert!(load_catalog(&path).is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn catalog_rejects_duplicate_normalized_targets() {
        let first = safe_artifact("windows", "x86_64");
        let second = safe_artifact("win32", "amd64");
        let path = write_fixture(
            "duplicate-target",
            &format!(
                r#"{{"schema_version":1,"provider":"vsn.test","runtimes":[{{"runtime":"node","version":"20.0.0","artifacts":[{first},{second}]}}],"signature":null}}"#
            ),
        );
        assert!(load_catalog(&path).is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn catalog_rejects_unsafe_metadata_on_other_platforms() {
        let safe = safe_artifact("linux", "x86_64");
        let unsafe_other = format!(
            r#"{{"os":"windows","arch":"x86_64","url":"http://insecure.invalid/runtime.zip","sha256":"{}","archive":"zip","executable_relpath":"../escape.exe"}}"#,
            "0".repeat(64)
        );
        let path = write_fixture(
            "unsafe-other-platform",
            &format!(
                r#"{{"schema_version":1,"provider":"vsn.test","runtimes":[{{"runtime":"node","version":"20.0.0","artifacts":[{safe},{unsafe_other}]}}],"signature":null}}"#
            ),
        );
        assert!(load_catalog(&path).is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn catalog_rejects_unsupported_archive_types() {
        let path = write_fixture(
            "unsupported-archive",
            &format!(
                r#"{{"schema_version":1,"provider":"vsn.test","runtimes":[{{"runtime":"node","version":"20.0.0","artifacts":[{{"os":"linux","arch":"x86_64","url":"https://example.invalid/runtime.rar","sha256":"{}","archive":"rar","executable_relpath":"bin/runtime"}}]}}],"signature":null}}"#,
                "0".repeat(64)
            ),
        );
        assert!(load_catalog(&path).is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn trust_rejects_unknown_fields() {
        assert!(validate_trust_bytes(br#"{"public_keys":[],"future_policy":true}"#).is_err());
    }

    #[test]
    fn valid_catalog_remains_accepted() {
        let artifact = safe_artifact("linux", "x86_64");
        let path = write_fixture(
            "valid",
            &format!(
                r#"{{"schema_version":1,"provider":"vsn.test","runtimes":[{{"runtime":"node","version":"20.0.0","artifacts":[{artifact}]}}],"signature":null}}"#
            ),
        );
        let catalog = load_catalog(&path).unwrap();
        assert_eq!(catalog.provider, "vsn.test");
        let _ = fs::remove_file(path);
    }
}
