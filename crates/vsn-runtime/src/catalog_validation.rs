use crate::RuntimeError;
use serde_json::{Map, Value};
use std::{collections::BTreeSet, path::Path};

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

fn normalize_os(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "win32" | "windows" => Some("windows"),
        "darwin" | "macos" => Some("macos"),
        "linux" => Some("linux"),
        _ => None,
    }
}

fn normalize_arch(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "x86_64" | "amd64" => Some("x86_64"),
        "aarch64" | "arm64" => Some("aarch64"),
        _ => None,
    }
}

fn validate_executable_relpath(value: &str) -> Result<(), RuntimeError> {
    let normalized = value.replace('\\', "/");
    if normalized.is_empty()
        || Path::new(value).is_absolute()
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
    let os = normalize_os(os_raw)
        .ok_or_else(|| invalid(format!("unsupported runtime artifact OS: {os_raw}")))?;
    let arch = normalize_arch(arch_raw)
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

    validate_executable_relpath(required_string(
        artifact,
        "executable_relpath",
        "runtime artifact",
    )?)?;

    Ok((os.to_string(), arch.to_string()))
}

pub(crate) fn validate_catalog_bytes(bytes: &[u8]) -> Result<(), RuntimeError> {
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
        super::validate_runtime_id(runtime)?;
        super::validate_version(version)?;

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

pub(crate) fn validate_trust_bytes(bytes: &[u8]) -> Result<(), RuntimeError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn safe_artifact(os: &str, arch: &str) -> String {
        format!(
            r#"{{"os":"{os}","arch":"{arch}","url":"https://example.invalid/runtime.bin","sha256":"{}","archive":"binary","executable_relpath":"bin/runtime"}}"#,
            "0".repeat(64)
        )
    }

    fn catalog_with(artifacts: &str) -> String {
        format!(
            r#"{{"schema_version":1,"provider":"vsn.test","runtimes":[{{"runtime":"node","version":"20.0.0","artifacts":[{artifacts}]}}],"signature":null}}"#
        )
    }

    #[test]
    fn rejects_unknown_fields() {
        let json = r#"{"schema_version":1,"provider":"vsn.test","runtimes":[],"signature":null,"future_policy":true}"#;
        assert!(validate_catalog_bytes(json.as_bytes()).is_err());
    }

    #[test]
    fn rejects_duplicate_releases() {
        let artifact = safe_artifact("linux", "x86_64");
        let release = format!(
            r#"{{"runtime":"node","version":"20.0.0","artifacts":[{artifact}]}}"#
        );
        let json = format!(
            r#"{{"schema_version":1,"provider":"vsn.test","runtimes":[{release},{release}],"signature":null}}"#
        );
        assert!(validate_catalog_bytes(json.as_bytes()).is_err());
    }

    #[test]
    fn rejects_duplicate_normalized_targets() {
        let json = catalog_with(&format!(
            "{},{}",
            safe_artifact("windows", "x86_64"),
            safe_artifact("win32", "amd64")
        ));
        assert!(validate_catalog_bytes(json.as_bytes()).is_err());
    }

    #[test]
    fn rejects_unsafe_other_platform_metadata() {
        let unsafe_other = format!(
            r#"{{"os":"windows","arch":"x86_64","url":"http://insecure.invalid/runtime.zip","sha256":"{}","archive":"zip","executable_relpath":"../escape.exe"}}"#,
            "0".repeat(64)
        );
        let json = catalog_with(&format!(
            "{},{}",
            safe_artifact("linux", "x86_64"),
            unsafe_other
        ));
        assert!(validate_catalog_bytes(json.as_bytes()).is_err());
    }

    #[test]
    fn rejects_unsupported_archive() {
        let artifact = format!(
            r#"{{"os":"linux","arch":"x86_64","url":"https://example.invalid/runtime.rar","sha256":"{}","archive":"rar","executable_relpath":"bin/runtime"}}"#,
            "0".repeat(64)
        );
        assert!(validate_catalog_bytes(catalog_with(&artifact).as_bytes()).is_err());
    }

    #[test]
    fn accepts_valid_catalog() {
        let json = catalog_with(&safe_artifact("linux", "x86_64"));
        assert!(validate_catalog_bytes(json.as_bytes()).is_ok());
    }

    #[test]
    fn rejects_unknown_trust_fields() {
        assert!(validate_trust_bytes(br#"{"public_keys":[],"future_policy":true}"#).is_err());
    }
}
