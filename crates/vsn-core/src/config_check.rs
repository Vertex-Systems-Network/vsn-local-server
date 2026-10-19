//! DVF-03: check a space config file before the editor saves it, and say which of the
//! space's services read it (so the UI can offer a restart). Files Devify has no
//! checker for are reported as unchecked, never as valid.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::CoreError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ConfigCheck {
    /// A checker ran (parser or the server's own `-t`/`-C` mode).
    pub checked: bool,
    pub ok: bool,
    pub message: String,
    /// Service kinds that read this file and need a restart to apply it.
    pub restart: Vec<String>,
}

fn verdict(ok: bool, message: impl Into<String>) -> ConfigCheck {
    ConfigCheck {
        checked: true,
        ok,
        message: message.into(),
        restart: Vec::new(),
    }
}

/// Service kinds that load `file` (a path relative to the space root).
pub fn services_reading(file: &str) -> &'static [&'static str] {
    match file {
        ".devify/nginx.conf" => &["nginx"],
        ".devify/httpd.conf" => &["apache"],
        ".devify/varnish.vcl" => &["varnish"],
        _ => &[],
    }
}

fn tail(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim();
    let start = text.len().saturating_sub(800);
    let start = (start..=text.len())
        .find(|i| text.is_char_boundary(*i))
        .unwrap_or(0);
    text[start..].to_string()
}

/// Run a server's own syntax check against `content` written to a scratch folder.
fn tool_check(
    program: &str,
    file_name: &str,
    content: &str,
    args: impl Fn(&Path, &Path) -> Vec<String>,
) -> Result<ConfigCheck, CoreError> {
    let Ok(exe) = vsn_system::find_executable(program) else {
        return Ok(ConfigCheck {
            checked: false,
            ok: true,
            message: format!("{program} is not installed, so this file was not checked"),
            restart: Vec::new(),
        });
    };
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let dir = std::env::temp_dir().join(format!("vsn-config-check-{nanos}"));
    let io = |e: std::io::Error| CoreError::Rejected(format!("config check: {e}"));
    std::fs::create_dir_all(&dir).map_err(io)?;
    let file = dir.join(file_name);
    std::fs::write(&file, content).map_err(io)?;
    let mut cmd = Command::new(exe);
    cmd.args(args(&dir, &file))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output();
    let _ = std::fs::remove_dir_all(&dir);
    let out = out.map_err(io)?;
    let mut text = tail(&out.stderr);
    if text.is_empty() {
        text = tail(&out.stdout);
    }
    Ok(verdict(out.status.success(), text))
}

/// Check `content` as the space file `file` (relative path, forward slashes).
pub fn check(file: &str, content: &str) -> Result<ConfigCheck, CoreError> {
    let name = file.rsplit('/').next().unwrap_or(file);
    let mut result = if name.ends_with(".json") {
        match serde_json::from_str::<serde_json::Value>(content) {
            Ok(_) => verdict(true, "valid JSON"),
            Err(e) => verdict(false, format!("invalid JSON: {e}")),
        }
    } else if file == ".devify/space.toml" {
        match vsn_space::parse(content) {
            Ok(_) => verdict(true, "valid space.toml"),
            Err(e) => verdict(false, e.to_string()),
        }
    } else if file == ".devify/nginx.conf" {
        tool_check("nginx", "nginx.conf", content, |dir, f| {
            let d = dir.to_string_lossy().into_owned();
            vec![
                "-t".into(),
                "-q".into(),
                "-p".into(),
                d.clone(),
                "-e".into(),
                format!("{d}/error.log"),
                "-c".into(),
                f.to_string_lossy().into_owned(),
            ]
        })?
    } else if file == ".devify/httpd.conf" {
        let program = if cfg!(windows) { "httpd" } else { "apache2" };
        tool_check(program, "httpd.conf", content, |_, f| {
            vec!["-t".into(), "-f".into(), f.to_string_lossy().into_owned()]
        })?
    } else if file == ".devify/varnish.vcl" {
        tool_check("varnishd", "varnish.vcl", content, |dir, f| {
            vec![
                "-C".into(),
                "-n".into(),
                dir.join("n").to_string_lossy().into_owned(),
                "-f".into(),
                f.to_string_lossy().into_owned(),
            ]
        })?
    } else {
        ConfigCheck {
            checked: false,
            ok: true,
            message: "Devify has no checker for this file".into(),
            restart: Vec::new(),
        }
    };
    if result.ok && result.message.is_empty() {
        result.message = "syntax is ok".into();
    }
    result.restart = services_reading(file)
        .iter()
        .map(|s| s.to_string())
        .collect();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_and_space_toml_are_parsed() {
        assert!(check("package.json", r#"{"name":"x"}"#).unwrap().ok);
        let bad = check("composer.json", "{ nope").unwrap();
        assert!(bad.checked && !bad.ok);
        assert!(!check(".devify/space.toml", "not = [toml").unwrap().ok);
    }

    #[test]
    fn unknown_files_are_unchecked_and_server_files_name_their_service() {
        let r = check("php.ini", "memory_limit = 1G").unwrap();
        assert!(!r.checked && r.ok);
        assert_eq!(services_reading(".devify/nginx.conf"), ["nginx"]);
        assert!(services_reading("nginx.conf").is_empty());
    }
}

#[cfg(test)]
mod nginx_probe {
    #[test]
    #[ignore = "needs nginx on PATH"]
    fn nginx_t_accepts_good_and_rejects_bad() {
        let good = "events {}\nhttp { server { listen 127.0.0.1:18099; } }\n";
        let r = super::check(".devify/nginx.conf", good).unwrap();
        assert!(r.checked && r.ok, "{r:?}");
        let r = super::check(
            ".devify/nginx.conf",
            "events {}\nhttp { server { listen; }\n",
        )
        .unwrap();
        assert!(r.checked && !r.ok, "{r:?}");
        println!("bad: {}", r.message);
    }
}
