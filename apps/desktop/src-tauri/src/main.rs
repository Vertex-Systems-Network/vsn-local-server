// Release builds are GUI apps: no console window behind the desktop UI.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::Manager;

#[tauri::command]
fn agent_call(command: String, params: Value) -> Result<Value, String> {
    let response = vsn_ipc::call(&command, params).map_err(|e| e.to_string())?;
    if response.ok {
        Ok(response.payload)
    } else {
        Err(response.payload.to_string())
    }
}

/// Bundled agent binary (`bin/vsn-agent.exe` next to the app's resources).
struct AgentBinary(Option<PathBuf>);
/// Last spawn attempt, so repeated retries do not start several agents.
struct LastSpawn(Mutex<Option<Instant>>);

fn agent_reachable() -> bool {
    vsn_ipc::call("status", json!({})).is_ok()
}

/// Per-user installs have no VSN-Agent Windows service, so the desktop starts the
/// bundled agent as a background user process when nothing answers on the IPC port.
fn spawn_agent(binary: &AgentBinary, last: &LastSpawn) -> Result<bool, String> {
    if agent_reachable() {
        return Ok(false);
    }
    let path = binary
        .0
        .as_ref()
        .filter(|p| p.is_file())
        .ok_or("The bundled vsn-agent was not found next to the app")?;
    {
        let mut guard = last.0.lock().map_err(|e| e.to_string())?;
        if guard.is_some_and(|t| t.elapsed() < Duration::from_secs(10)) {
            return Ok(true);
        }
        *guard = Some(Instant::now());
    }
    let mut command = Command::new(path);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // A hidden console, not DETACHED_PROCESS: with no console at all, every
        // tasklist/where/docker the agent runs would pop up a console window of its own.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
        .spawn()
        .map_err(|e| format!("Could not start vsn-agent: {e}"))?;
    Ok(true)
}

/// True for `http://localhost…`, `http://127.0.0.1…`, `http://<name>.localhost…` and the
/// reserved local TLD `http://<name>.test…` (Settings → Project URL pattern).
fn is_local_http(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("http://") else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = host.rsplit_once(':').map_or(host, |(h, _)| h);
    host == "localhost"
        || host == "127.0.0.1"
        || host.ends_with(".localhost")
        || host.ends_with(".test")
}

/// Open a link in the default browser: any https link (About: website, releases,
/// donation) or a local http address of a space.
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    let ok = (url.starts_with("https://") || is_local_http(&url))
        && url.len() < 512
        && !url
            .chars()
            .any(|c| c.is_whitespace() || c == '"' || c == '^' || c == '&');
    if !ok {
        return Err("Only https links and local http addresses can be opened".into());
    }
    #[cfg(windows)]
    let mut command = {
        use std::os::windows::process::CommandExt;
        let mut c = Command::new("rundll32.exe");
        c.args(["url.dll,FileProtocolHandler", &url]);
        c.creation_flags(0x0800_0000);
        c
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = Command::new("open");
        c.arg(&url);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut c = Command::new("xdg-open");
        c.arg(&url);
        c
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not open the browser: {e}"))
}

/// `name.test` with a plain DNS label: the only domains the elevated route action accepts.
fn is_route_domain(domain: &str) -> bool {
    domain.strip_suffix(".test").is_some_and(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    })
}

/// Route a space domain (shop.test) to a local port: runs the bundled agent's
/// `network-admin route-add` elevated, so Windows shows one UAC prompt. Returns the exit code.
#[tauri::command]
fn route_domain(
    binary: tauri::State<'_, AgentBinary>,
    domain: String,
    port: u16,
) -> Result<i32, String> {
    if !is_route_domain(&domain) || port == 0 {
        return Err("use a name.test domain and a port between 1 and 65535".into());
    }
    let agent = binary
        .0
        .as_ref()
        .filter(|p| p.is_file())
        .ok_or_else(|| "bundled agent not found".to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Values are validated above; the path is quoted for PowerShell (single quotes doubled).
        let path = agent.display().to_string().replace('\'', "''");
        let script = format!(
            "$p = Start-Process -FilePath '{path}' -ArgumentList 'network-admin','route-add','{domain}','{port}' -Verb RunAs -WindowStyle Hidden -Wait -PassThru; exit $p.ExitCode"
        );
        let status = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .creation_flags(0x0800_0000)
            .status()
            .map_err(|e| format!("could not ask for administrator rights: {e}"))?;
        Ok(status.code().unwrap_or(-1))
    }
    #[cfg(not(windows))]
    {
        let _ = agent;
        Err(format!(
            "run as root: vsn-agent network-admin route-add {domain} {port}"
        ))
    }
}

/// Start the bundled agent if it is not running, then wait up to ~8 s for it to answer.
#[tauri::command]
fn agent_start(
    binary: tauri::State<'_, AgentBinary>,
    last: tauri::State<'_, LastSpawn>,
) -> Result<bool, String> {
    spawn_agent(&binary, &last)?;
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        if agent_reachable() {
            return Ok(true);
        }
        std::thread::sleep(Duration::from_millis(400));
    }
    Ok(agent_reachable())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let binary = app.path().resource_dir().ok().map(|dir| {
                dir.join("bin").join(if cfg!(windows) {
                    "vsn-agent.exe"
                } else {
                    "vsn-agent"
                })
            });
            app.manage(AgentBinary(binary));
            // Product name shown to the user. The installer identity ("VSN Dev Platform")
            // stays frozen in tauri.conf.json until the governed public-release rename.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title("VSN Devify");
            }
            app.manage(LastSpawn(Mutex::new(None)));
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let _ = spawn_agent(&handle.state::<AgentBinary>(), &handle.state::<LastSpawn>());
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            agent_call,
            agent_start,
            open_url,
            route_domain
        ])
        .run(tauri::generate_context!())
        .expect("error while running VSN desktop")
}

#[cfg(test)]
mod tests {
    use super::agent_call;
    use serde_json::{json, Value};

    #[test]
    #[ignore = "02.04 acceptance: run only while vsn-agent is stopped"]
    fn desktop_bridge_reports_agent_unavailable() {
        let error = agent_call("status".to_string(), json!({})).expect_err(
            "Desktop bridge must fail when the authenticated Agent channel is unavailable",
        );
        assert!(!error.trim().is_empty());
    }

    #[test]
    #[ignore = "02.04 acceptance: requires a live vsn-agent in the same secure-store session"]
    fn desktop_bridge_uses_authenticated_agent() {
        let status = agent_call("status".to_string(), json!({}))
            .expect("Desktop status bridge must reach the authenticated Agent");
        assert_eq!(
            status.pointer("/health/healthy"),
            Some(&Value::Bool(true)),
            "Agent health must be healthy"
        );
        assert_eq!(
            status.pointer("/security/ipc_secret_ready"),
            Some(&Value::Bool(true)),
            "Desktop bridge must report the authenticated IPC secret as ready"
        );

        let machine = agent_call("machine".to_string(), json!({}))
            .expect("Desktop machine bridge must reach the authenticated Agent");
        let device_id = machine
            .get("device_id")
            .and_then(Value::as_str)
            .expect("machine response must contain a device_id");
        assert!(device_id.len() >= 8, "device_id must be non-trivial");
    }
}
