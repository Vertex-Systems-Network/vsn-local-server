//! `vsn-agent mcp`: a Model Context Protocol server over stdio (DVF-11), so AI tools
//! such as Claude Desktop can see and run this PC's spaces. Every tool is a call to the
//! running VSN Agent over its authenticated local IPC, so the same policy, space locks
//! and audit log apply; nothing here reaches outside this computer.

use serde_json::{json, Value};
use std::io::{BufRead, Write};

struct Tool {
    name: &'static str,
    description: &'static str,
    command: &'static str,
    /// Required string arguments, passed through to the agent command.
    args: &'static [(&'static str, &'static str)],
}

const TOOLS: &[Tool] = &[
    Tool {
        name: "list_spaces",
        description:
            "List the VSN Devify spaces (projects) on this PC with their services and ports.",
        command: "space.list",
        args: &[],
    },
    Tool {
        name: "space_status",
        description: "Show one space: tools, services, ports and whether they run.",
        command: "space.get",
        args: &[("path", "Full path of the space folder")],
    },
    Tool {
        name: "start_service",
        description: "Start a service (mysql, postgres, redis, mailpit …) of a space.",
        command: "space.service.start",
        args: &[
            ("path", "Full path of the space folder"),
            ("kind", "Service kind, e.g. mysql"),
        ],
    },
    Tool {
        name: "stop_service",
        description: "Stop a service of a space.",
        command: "space.service.stop",
        args: &[
            ("path", "Full path of the space folder"),
            ("kind", "Service kind, e.g. mysql"),
        ],
    },
    Tool {
        name: "diagnose",
        description: "Run VSN Devify's read-only health checklist and return what to fix.",
        command: "support.diagnose",
        args: &[],
    },
    Tool {
        name: "request_stats",
        description: "Requests, errors and slowest URLs per routed name.test domain.",
        command: "domain.traffic",
        args: &[],
    },
];

fn tool_list() -> Value {
    Value::Array(
        TOOLS
            .iter()
            .map(|t| {
                let props: serde_json::Map<String, Value> = t
                    .args
                    .iter()
                    .map(|(n, d)| {
                        (
                            (*n).to_string(),
                            json!({"type": "string", "description": d}),
                        )
                    })
                    .collect();
                let required: Vec<&str> = t.args.iter().map(|(n, _)| *n).collect();
                json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": {"type": "object", "properties": props, "required": required}
                })
            })
            .collect(),
    )
}

fn call_tool(name: &str, arguments: &Value) -> Value {
    let Some(tool) = TOOLS.iter().find(|t| t.name == name) else {
        return json!({"isError": true, "content": [{"type": "text", "text": format!("unknown tool {name}")}]});
    };
    let mut params = serde_json::Map::new();
    for (arg, _) in tool.args {
        match arguments.get(*arg).and_then(Value::as_str) {
            Some(v) if !v.is_empty() && v.len() <= 4096 => {
                params.insert((*arg).into(), Value::String(v.into()));
            }
            _ => {
                return json!({"isError": true, "content": [{"type": "text", "text": format!("missing argument {arg}")}]})
            }
        }
    }
    match vsn_ipc::call(tool.command, Value::Object(params)) {
        Ok(r) => json!({
            "isError": !r.ok,
            "content": [{"type": "text", "text": serde_json::to_string_pretty(&r.payload).unwrap_or_default()}]
        }),
        Err(e) => json!({
            "isError": true,
            "content": [{"type": "text", "text": format!("the VSN Agent is not reachable: {e}. Open VSN Devify first.")}]
        }),
    }
}

/// Answer one JSON-RPC request; notifications (no id) get no reply.
pub fn handle(request: &Value) -> Option<Value> {
    let id = request.get("id")?.clone();
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let result = match method {
        "initialize" => json!({
            "protocolVersion": request.pointer("/params/protocolVersion").cloned().unwrap_or(json!("2025-06-18")),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "vsn-devify", "version": env!("CARGO_PKG_VERSION")}
        }),
        "ping" => json!({}),
        "tools/list" => json!({"tools": tool_list()}),
        "tools/call" => {
            let name = request
                .pointer("/params/name")
                .and_then(Value::as_str)
                .unwrap_or("");
            let args = request
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or(json!({}));
            call_tool(name, &args)
        }
        _ => {
            return Some(
                json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": "method not found"}}),
            )
        }
    };
    Some(json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

pub fn serve_stdio() -> std::process::ExitCode {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(request) => handle(&request),
            Err(_) => Some(
                json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": "parse error"}}),
            ),
        };
        if let Some(reply) = reply {
            let _ = writeln!(stdout, "{reply}");
            let _ = stdout.flush();
        }
    }
    std::process::ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_tools_and_rejects_unknown_methods() {
        let init = handle(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}})).unwrap();
        assert_eq!(init["result"]["serverInfo"]["name"], "vsn-devify");
        let list = handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})).unwrap();
        assert!(list["result"]["tools"].as_array().unwrap().len() >= 5);
        assert!(handle(&json!({"jsonrpc":"2.0","method":"notifications/initialized"})).is_none());
        let bad = handle(&json!({"jsonrpc":"2.0","id":3,"method":"nope"})).unwrap();
        assert_eq!(bad["error"]["code"], -32601);
        let missing = handle(&json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"space_status","arguments":{}}})).unwrap();
        assert_eq!(missing["result"]["isError"], true);
    }
}
