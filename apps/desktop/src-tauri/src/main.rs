use serde_json::Value;

#[tauri::command]
fn agent_call(command: String, params: Value) -> Result<Value, String> {
    let response = vsn_ipc::call(&command, params).map_err(|e| e.to_string())?;
    if response.ok {
        Ok(response.payload)
    } else {
        Err(response.payload.to_string())
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![agent_call])
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
        let error = agent_call("status".to_string(), json!({}))
            .expect_err("Desktop bridge must fail when the authenticated Agent channel is unavailable");
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
