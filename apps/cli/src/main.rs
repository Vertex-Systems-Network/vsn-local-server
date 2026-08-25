use serde_json::{json, Value};
use std::{
    io::{self, Read},
    process::ExitCode,
};
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match dispatch(&args) {
        Ok(Some(value)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&value).unwrap_or_else(|_| "null".into())
            );
            ExitCode::SUCCESS
        }
        Ok(None) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error={error}");
            eprintln!("hint=ensure vsn-agent is running and the authenticated local IPC channel is available");
            ExitCode::FAILURE
        }
    }
}
fn dispatch(args: &[String]) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    let response = match args {
        [cmd] if cmd == "status" => call("status", json!({}))?,
        [cmd] if cmd == "machine" => call("machine", json!({}))?,
        [cmd] if cmd == "security" => call("security.status", json!({}))?,
        [cmd] if cmd == "diagnostics" => call("diagnostics", json!({}))?,
        [cmd, sub] if cmd == "config" && sub == "show" => call("config.show", json!({}))?,
        [cmd, sub] if cmd == "audit" && sub == "verify" => call("audit.verify", json!({}))?,
        [cmd] if cmd == "ping" => call("ping", json!({}))?,
        [cmd, sub] if cmd == "process" && sub == "list" => call("process.list", json!({}))?,
        [cmd, sub, pid] if cmd == "process" && sub == "metrics" => {
            call("process.metrics", json!({"pid":parse_u32(pid,"pid")?}))?
        }
        [cmd, sub] if cmd == "managed" && sub == "list" => call("process.managed.list", json!({}))?,
        [cmd, sub, id] if cmd == "managed" && sub == "status" => {
            call("process.managed.status", json!({"id":id}))?
        }
        [cmd, sub, id] if cmd == "managed" && sub == "stop" => {
            call("process.managed.stop", json!({"id":id}))?
        }
        [cmd, sub, id] if cmd == "managed" && sub == "remove" => {
            call("process.managed.remove", json!({"id":id,"force":false}))?
        }
        [cmd, sub] if cmd == "port" && sub == "list" => call("port.list", json!({}))?,
        [cmd, sub, port] if cmd == "port" && sub == "check" => {
            call("port.check", json!({"port":parse_u16(port)?}))?
        }
        [cmd, sub, name] if cmd == "service" && sub == "status" => {
            call("service.status", json!({"name":name}))?
        }
        [cmd, action, name]
            if cmd == "service" && matches!(action.as_str(), "start" | "stop" | "restart") =>
        {
            call("service.action", json!({"name":name,"action":action}))?
        }
        [cmd, sub] if cmd == "service" && sub == "conformance" => {
            call("service.conformance", json!({}))?
        }
        [cmd, sub, host, port] if cmd == "health" && sub == "tcp" => call(
            "health.tcp",
            json!({"host":host,"port":parse_u16(port)?,"timeout_ms":1500}),
        )?,
        [cmd, sub, path] if cmd == "log" && sub == "tail" => {
            call("log.tail", json!({"path":path,"lines":100}))?
        }
        [cmd, sub, path, lines] if cmd == "log" && sub == "tail" => call(
            "log.tail",
            json!({"path":path,"lines":parse_u64(lines,"line count")?}),
        )?,
        [cmd, sub] if cmd == "runtime" && sub == "list" => call("runtime.list", json!({}))?,
        [cmd, sub] if cmd == "runtime" && sub == "registry" => call("runtime.registry", json!({}))?,
        [cmd, sub] if cmd == "runtime" && sub == "audit" => call("runtime.audit", json!({}))?,
        [cmd, sub] if cmd == "runtime" && sub == "conformance" => {
            call("runtime.conformance", json!({}))?
        }
        [cmd, sub] if cmd == "runtime" && sub == "repair" => call("runtime.repair", json!({}))?,
        [cmd, sub, path] if cmd == "runtime" && sub == "catalog" => {
            call("runtime.catalog", json!({"path":path}))?
        }
        [cmd, sub, catalog, runtime, version] if cmd == "runtime" && sub == "install" => call(
            "runtime.install",
            json!({"catalog":catalog,"runtime":runtime,"version":version}),
        )?,
        [cmd, sub, project, runtime, version] if cmd == "runtime" && sub == "activate" => call(
            "runtime.activate",
            json!({"project":project,"runtime":runtime,"version":version}),
        )?,
        [cmd, sub, runtime, version] if cmd == "runtime" && sub == "uninstall" => call(
            "runtime.uninstall",
            json!({"runtime":runtime,"version":version}),
        )?,
        [cmd, sub, path, trust] if cmd == "runtime" && sub == "catalog-verify" => {
            call("runtime.catalog-verify", json!({"path":path,"trust":trust}))?
        }
        [cmd, sub, catalog, trust, runtime, version]
            if cmd == "runtime" && sub == "install-trusted" =>
        {
            call(
                "runtime.install-trusted",
                json!({"catalog":catalog,"trust":trust,"runtime":runtime,"version":version}),
            )?
        }
        [cmd, sub] if cmd == "container" && sub == "backends" => {
            call("container.backends", json!({}))?
        }
        [cmd, sub] if cmd == "container" && sub == "registry-publish" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("container.registry-publish", value)?
        }
        [cmd, sub, backend] if cmd == "container" && sub == "list" => {
            call("container.list", json!({"backend":backend,"all":true}))?
        }
        [cmd, sub, backend] if cmd == "container" && sub == "images" => {
            call("container.images", json!({"backend":backend}))?
        }
        [cmd, sub, backend] if cmd == "container" && sub == "volumes" => {
            call("container.volumes", json!({"backend":backend}))?
        }
        [cmd, sub, backend] if cmd == "container" && sub == "networks" => {
            call("container.networks", json!({"backend":backend}))?
        }
        [cmd, sub, backend, image] if cmd == "container" && sub == "pull" => call(
            "container.image-pull",
            json!({"backend":backend,"image":image}),
        )?,
        [cmd, sub] if cmd == "container" && sub == "build" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("container.image-build", value)?
        }
        [cmd, sub, backend, kind, target] if cmd == "container" && sub == "remove" => call(
            "container.remove",
            json!({"backend":backend,"kind":kind,"target":target,"force":false}),
        )?,
        [cmd, sub, backend, target] if cmd == "container" && sub == "logs" => call(
            "container.logs",
            json!({"backend":backend,"target":target,"tail":200}),
        )?,
        [cmd, sub, backend, target] if cmd == "container" && sub == "inspect" => call(
            "container.inspect",
            json!({"backend":backend,"target":target}),
        )?,
        [cmd, sub, backend, target] if cmd == "container" && sub == "stats" => call(
            "container.stats",
            json!({"backend":backend,"target":target}),
        )?,
        [cmd, sub] if cmd == "container" && sub == "exec" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("container.exec", value)?
        }
        [cmd, action, backend, target]
            if cmd == "container"
                && matches!(
                    action.as_str(),
                    "start" | "stop" | "restart" | "pause" | "unpause"
                ) =>
        {
            call(
                "container.action",
                json!({"backend":backend,"action":action,"target":target}),
            )?
        }
        [cmd, sub, backend, action, path] if cmd == "container" && sub == "compose" => call(
            "container.compose",
            json!({"backend":backend,"action":action,"path":path}),
        )?,
        [cmd, sub, path] if cmd == "project" && sub == "detect" => {
            call("project.detect", json!({"path":path}))?
        }
        [cmd, sub, path] if cmd == "project" && sub == "dependencies" => {
            call("project.dependencies", json!({"path":path}))?
        }
        [cmd, sub, template, path] if cmd == "project" && sub == "bootstrap-plan" => call(
            "project.bootstrap-plan",
            json!({"template":template,"path":path}),
        )?,
        [cmd, sub, template, path] if cmd == "project" && sub == "bootstrap" => call(
            "project.bootstrap",
            json!({"template":template,"path":path}),
        )?,
        [cmd, sub] if cmd == "project" && sub == "conformance" => {
            call("project.conformance", json!({}))?
        }
        [cmd, sub] if cmd == "project" && sub == "templates" => {
            call("project.templates", json!({}))?
        }
        [cmd, sub] if cmd == "workspace" && sub == "list" => call("workspace.roots", json!({}))?,
        [cmd, sub, path] if cmd == "workspace" && sub == "add" => {
            call("workspace.add", json!({"path":path}))?
        }
        [cmd, sub, path] if cmd == "workspace" && sub == "remove" => {
            call("workspace.remove", json!({"path":path}))?
        }
        [cmd, sub] if cmd == "files" && sub == "conformance" => {
            call("files.conformance", json!({}))?
        }
        [cmd, sub, path] if cmd == "files" && sub == "list" => {
            call("files.list", json!({"path":path}))?
        }
        [cmd, sub, path] if cmd == "files" && sub == "read" => {
            call("files.read", json!({"path":path}))?
        }
        [cmd, sub, path] if cmd == "files" && sub == "write" => {
            let content = read_stdin_text()?;
            call("files.write", json!({"path":path,"content":content}))?
        }
        [cmd, sub, path] if cmd == "files" && sub == "binary-read" => call(
            "files.binary.read",
            json!({"path":path,"offset":0,"max_bytes":524288}),
        )?,
        [cmd, sub, path, offset, max_bytes] if cmd == "files" && sub == "binary-read" => call(
            "files.binary.read",
            json!({"path":path,"offset":parse_u64(offset,"offset")?,"max_bytes":parse_u64(max_bytes,"max bytes")?}),
        )?,
        [cmd, sub, path, transfer_id, offset, finalize]
            if cmd == "files" && sub == "binary-write" =>
        {
            let data_b64 = read_stdin_text()?.trim().to_string();
            call(
                "files.binary.write",
                json!({"path":path,"transfer_id":transfer_id,"offset":parse_u64(offset,"offset")?,"data_b64":data_b64,"finalize":parse_bool(finalize)?}),
            )?
        }
        [cmd, sub, path, transfer_id] if cmd == "files" && sub == "binary-abort" => call(
            "files.binary.abort",
            json!({"path":path,"transfer_id":transfer_id}),
        )?,
        [cmd, sub, path, transfer_id] if cmd == "files" && sub == "binary-status" => call(
            "files.binary.status",
            json!({"path":path,"transfer_id":transfer_id}),
        )?,
        [cmd, sub, path] if cmd == "files" && sub == "digest" => {
            call("files.digest", json!({"path":path}))?
        }
        [cmd, sub, path] if cmd == "files" && sub == "mkdir" => {
            call("files.mkdir", json!({"path":path}))?
        }
        [cmd, sub, source, destination] if cmd == "files" && sub == "move" => call(
            "files.move",
            json!({"source":source,"destination":destination}),
        )?,
        [cmd, sub, path] if cmd == "files" && sub == "delete" => {
            call("files.delete", json!({"path":path,"recursive":false}))?
        }
        [cmd, sub, path, recursive] if cmd == "files" && sub == "delete" => call(
            "files.delete",
            json!({"path":path,"recursive":parse_bool(recursive)?}),
        )?,
        [cmd, sub, cwd, program, rest @ ..] if cmd == "terminal" && sub == "exec" => call(
            "terminal.exec",
            json!({"cwd":cwd,"program":program,"args":rest,"env":{},"timeout_ms":30000}),
        )?,
        [cmd, sub, cwd, program, rest @ ..] if cmd == "terminal" && sub == "start" => call(
            "terminal.session.start",
            json!({"cwd":cwd,"program":program,"args":rest,"env":{}}),
        )?,
        [cmd, sub, id] if cmd == "terminal" && sub == "write" => {
            let input = read_stdin_text()?;
            call(
                "terminal.session.write",
                json!({"session_id":id,"input":input}),
            )?
        }
        [cmd, sub, id] if cmd == "terminal" && sub == "read" => call(
            "terminal.session.read",
            json!({"session_id":id,"max_bytes":65536}),
        )?,
        [cmd, sub, id] if cmd == "terminal" && sub == "read-wait" => call(
            "terminal.session.read-wait",
            json!({"session_id":id,"max_bytes":65536,"wait_ms":3000}),
        )?,
        [cmd, sub, id] if cmd == "terminal" && sub == "status" => {
            call("terminal.session.status", json!({"session_id":id}))?
        }
        [cmd, sub, id] if cmd == "terminal" && sub == "stop" => {
            call("terminal.session.stop", json!({"session_id":id}))?
        }
        [cmd, sub, id] if cmd == "terminal" && sub == "remove" => {
            call("terminal.session.remove", json!({"session_id":id}))?
        }
        [cmd, sub] if cmd == "terminal" && sub == "list" => {
            call("terminal.session.list", json!({}))?
        }
        [cmd, sub, cwd, program, rest @ ..] if cmd == "terminal" && sub == "pty-start" => call(
            "terminal.pty.start",
            json!({"cwd":cwd,"program":program,"args":rest,"env":{},"rows":30,"cols":120}),
        )?,
        [cmd, sub, id] if cmd == "terminal" && sub == "pty-write" => {
            let input = read_stdin_text()?;
            call("terminal.pty.write", json!({"session_id":id,"input":input}))?
        }
        [cmd, sub, id] if cmd == "terminal" && sub == "pty-read" => call(
            "terminal.pty.read",
            json!({"session_id":id,"max_bytes":65536}),
        )?,
        [cmd, sub, id] if cmd == "terminal" && sub == "pty-read-wait" => call(
            "terminal.pty.read-wait",
            json!({"session_id":id,"max_bytes":65536,"wait_ms":3000}),
        )?,
        [cmd, sub, id, wait_ms] if cmd == "terminal" && sub == "pty-read-wait" => call(
            "terminal.pty.read-wait",
            json!({"session_id":id,"max_bytes":65536,"wait_ms":parse_u64(wait_ms,"wait ms")?}),
        )?,
        [cmd, sub, id, rows, cols] if cmd == "terminal" && sub == "pty-resize" => call(
            "terminal.pty.resize",
            json!({"session_id":id,"rows":parse_u16(rows)?,"cols":parse_u16(cols)?}),
        )?,
        [cmd, sub, id] if cmd == "terminal" && sub == "pty-status" => {
            call("terminal.pty.status", json!({"session_id":id}))?
        }
        [cmd, sub, id] if cmd == "terminal" && sub == "pty-stop" => {
            call("terminal.pty.stop", json!({"session_id":id}))?
        }
        [cmd, sub, id] if cmd == "terminal" && sub == "pty-remove" => {
            call("terminal.pty.remove", json!({"session_id":id}))?
        }
        [cmd, sub] if cmd == "terminal" && sub == "pty-list" => {
            call("terminal.pty.list", json!({}))?
        }
        [cmd, sub] if cmd == "terminal" && sub == "pty-scrollback-list" => {
            call("terminal.pty.scrollback.list", json!({}))?
        }
        [cmd, sub, id, offset, max] if cmd == "terminal" && sub == "pty-scrollback-read" => call(
            "terminal.pty.scrollback.read",
            json!({"session_id":id,"offset":parse_u64(offset,"offset")?,"max_bytes":parse_u64(max,"max bytes")?}),
        )?,
        [cmd, sub, id] if cmd == "terminal" && sub == "pty-scrollback-remove" => {
            call("terminal.pty.scrollback.remove", json!({"session_id":id}))?
        }
        [cmd, sub] if cmd == "terminal" && sub == "conformance" => {
            call("terminal.conformance", json!({}))?
        }
        [cmd, sub] if cmd == "terminal" && sub == "pty-recovery-list" => {
            call("terminal.pty.recovery.list", json!({}))?
        }
        [cmd, sub, id] if cmd == "terminal" && sub == "pty-recovery-remove" => {
            call("terminal.pty.recovery.remove", json!({"session_id":id}))?
        }
        [cmd, sub, port, path] if cmd == "preview" && sub == "fetch" => call(
            "preview.fetch",
            json!({"port":parse_u16(port)?,"path":path,"method":"GET"}),
        )?,
        [cmd, sub] if cmd == "preview" && sub == "request" => {
            let request: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("preview.request", request)?
        }
        [cmd, sub] if cmd == "preview" && sub == "conformance" => {
            call("preview.conformance", json!({}))?
        }
        [cmd, sub, domain, port] if cmd == "domain" && sub == "plan" => call(
            "domain.plan",
            json!({"domain":domain,"port":parse_u16(port)?,"tls":true}),
        )?,
        [cmd, sub, domain] if cmd == "domain" && sub == "apply" => {
            call("domain.apply-hosts", json!({"domain":domain}))?
        }
        [cmd, sub, domain] if cmd == "domain" && sub == "remove" => {
            call("domain.remove-hosts", json!({"domain":domain}))?
        }
        [cmd, sub] if cmd == "domain" && sub == "reload" => call("domain.reload", json!({}))?,
        [cmd, sub] if cmd == "domain" && sub == "conformance" => {
            call("network.conformance", json!({}))?
        }
        [cmd, sub] if cmd == "dns" && sub == "plan" => {
            call("network.dns-plan", json!({"listen":"127.0.0.1:53535"}))?
        }
        [cmd, sub, listen] if cmd == "dns" && sub == "plan" => {
            call("network.dns-plan", json!({"listen":listen}))?
        }
        [cmd, sub] if cmd == "dns" && sub == "start" => {
            call("network.dns-start", json!({"listen":"127.0.0.1:53535"}))?
        }
        [cmd, sub, listen] if cmd == "dns" && sub == "start" => {
            call("network.dns-start", json!({"listen":listen}))?
        }
        [cmd, sub] if cmd == "dns" && sub == "status" => call("network.dns-status", json!({}))?,
        [cmd, sub] if cmd == "dns" && sub == "stop" => call("network.dns-stop", json!({}))?,
        [cmd, sub] if cmd == "db" && sub == "remote-conformance" => {
            call("database.remote.conformance", json!({}))?
        }
        [cmd, sub] if cmd == "db" && sub == "studio-conformance" => {
            call("database.studio.conformance", json!({}))?
        }
        [cmd, sub, model] if cmd == "db" && sub == "workspace" => {
            call("database.workspace", json!({"model":model}))?
        }
        [cmd, sub] if cmd == "db" && sub == "model-analyze" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("database.model.analyze", value)?
        }
        [cmd, sub] if cmd == "db" && sub == "ui-demo" => call("database.ui-demo", json!({}))?,
        [cmd, sub, path] if cmd == "db" && sub == "sqlite-inspect" => {
            call("database.sqlite.inspect", json!({"path":path}))?
        }
        [cmd, sub, path, sql] if cmd == "db" && sub == "sqlite-query" => {
            call("database.sqlite.query", json!({"path":path,"sql":sql}))?
        }
        [cmd, sub, path, entity] if cmd == "db" && sub == "sqlite-browse" => call(
            "database.sqlite.browse",
            json!({"path":path,"entity":entity,"limit":100,"offset":0}),
        )?,
        [cmd, sub, path, entity, limit, offset] if cmd == "db" && sub == "sqlite-browse" => call(
            "database.sqlite.browse",
            json!({"path":path,"entity":entity,"limit":parse_u32(limit,"limit")?,"offset":parse_u64(offset,"offset")?}),
        )?,
        [cmd, sub, path, entity, limit, offset, order_by, descending]
            if cmd == "db" && sub == "sqlite-browse" =>
        {
            call(
                "database.sqlite.browse",
                json!({"path":path,"entity":entity,"limit":parse_u32(limit,"limit")?,"offset":parse_u64(offset,"offset")?,"order_by":opt_arg(order_by),"descending":parse_bool(descending)?}),
            )?
        }
        [cmd, sub, path, entity] if cmd == "db" && sub == "sqlite-indexes" => call(
            "database.sqlite.indexes",
            json!({"path":path,"entity":entity}),
        )?,
        [cmd, sub, path, entity] if cmd == "db" && sub == "sqlite-relations" => call(
            "database.sqlite.relations",
            json!({"path":path,"entity":entity}),
        )?,
        [cmd, sub, path, entity] if cmd == "db" && sub == "sqlite-stats" => call(
            "database.sqlite.stats",
            json!({"path":path,"entity":entity}),
        )?,
        [cmd, sub, path, entity]
            if cmd == "db"
                && matches!(
                    sub.as_str(),
                    "sqlite-insert" | "sqlite-update" | "sqlite-delete"
                ) =>
        {
            let request: Value = serde_json::from_str(&read_stdin_text()?)?;
            let command = match sub.as_str() {
                "sqlite-insert" => "database.sqlite.insert",
                "sqlite-update" => "database.sqlite.update",
                _ => "database.sqlite.delete",
            };
            call(
                command,
                json!({"path":path,"entity":entity,"request":request}),
            )?
        }
        [cmd, sub] if cmd == "db" && sub == "clients" => call("database.cli.detect", json!({}))?,
        [cmd, sub, engine, host, port, user, database, root_ca]
            if cmd == "db" && sub == "inspect-tls" =>
        {
            call(
                "database.cli.inspect",
                json!({"connection":db_connection_tls_json(engine,host,port,user,database,root_ca,None)?}),
            )?
        }
        [cmd, sub, engine, host, port, user, database, root_ca, credential]
            if cmd == "db" && sub == "inspect-tls" =>
        {
            call(
                "database.cli.inspect",
                json!({"connection":db_connection_tls_json(engine,host,port,user,database,root_ca,Some(credential))?}),
            )?
        }
        [cmd, sub, engine, host, port, user, database, root_ca, sql]
            if cmd == "db" && sub == "query-tls" =>
        {
            call(
                "database.cli.query",
                json!({"connection":db_connection_tls_json(engine,host,port,user,database,root_ca,None)?,"sql":sql}),
            )?
        }
        [cmd, sub, connection, root_ca] if cmd == "db" && sub == "pg-tls-inspect" => call(
            "database.tls.postgres.inspect",
            json!({"connection_string":connection,"root_ca_pem_path":root_ca}),
        )?,
        [cmd, sub, connection, root_ca, schema, table] if cmd == "db" && sub == "pg-tls-browse" => {
            call(
                "database.tls.postgres.browse",
                json!({"connection_string":connection,"root_ca_pem_path":root_ca,"schema":schema,"table":table,"limit":100,"offset":0}),
            )?
        }
        [cmd, sub, connection, root_ca, sql] if cmd == "db" && sub == "pg-tls-query" => call(
            "database.tls.postgres.query",
            json!({"connection_string":connection,"root_ca_pem_path":root_ca,"sql":sql}),
        )?,
        [cmd, sub, url, root_ca] if cmd == "db" && sub == "mysql-tls-inspect" => call(
            "database.tls.mysql.inspect",
            json!({"url":url,"root_ca_path":root_ca}),
        )?,
        [cmd, sub, url, root_ca, database, table] if cmd == "db" && sub == "mysql-tls-browse" => {
            call(
                "database.tls.mysql.browse",
                json!({"url":url,"root_ca_path":root_ca,"database":database,"table":table,"limit":100,"offset":0}),
            )?
        }
        [cmd, sub, url, root_ca, sql] if cmd == "db" && sub == "mysql-tls-query" => call(
            "database.tls.mysql.query",
            json!({"url":url,"root_ca_path":root_ca,"sql":sql}),
        )?,
        [cmd, sub, engine, host, port, user, database] if cmd == "db" && sub == "inspect" => call(
            "database.cli.inspect",
            json!({"connection":db_connection_json(engine,host,port,user,database,None)?}),
        )?,
        [cmd, sub, engine, host, port, user, database, credential]
            if cmd == "db" && sub == "inspect" =>
        {
            call(
                "database.cli.inspect",
                json!({"connection":db_connection_json(engine,host,port,user,database,Some(credential))?}),
            )?
        }
        [cmd, sub, engine, host, port, user, database, sql] if cmd == "db" && sub == "query" => {
            call(
                "database.cli.query",
                json!({"connection":db_connection_json(engine,host,port,user,database,None)?,"sql":sql}),
            )?
        }
        [cmd, sub] if cmd == "db" && sub == "job-start" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("database.cli.job.start", value)?
        }
        [cmd, sub, id, offset, max] if cmd == "db" && sub == "job-output" => call(
            "database.cli.job.output",
            json!({"job_id":id,"offset":offset.parse::<u64>()?,"max_bytes":max.parse::<u64>()?}),
        )?,
        [cmd, sub, id] if cmd == "db" && sub == "job-output-remove" => {
            call("database.cli.job.output-remove", json!({"job_id":id}))?
        }
        [cmd, sub, id] if cmd == "db" && sub == "job-status" => {
            call("database.cli.job.status", json!({"job_id":id}))?
        }
        [cmd, sub] if cmd == "db" && sub == "job-list" => call("database.cli.job.list", json!({}))?,
        [cmd, sub, id] if cmd == "db" && sub == "job-cancel" => {
            call("database.cli.job.cancel", json!({"job_id":id}))?
        }
        [cmd, sub, connection] if cmd == "db" && sub == "pg-native-inspect" => call(
            "database.native.postgres.inspect",
            json!({"connection_string":connection}),
        )?,
        [cmd, sub, connection, schema, table] if cmd == "db" && sub == "pg-native-browse" => call(
            "database.native.postgres.browse",
            json!({"connection_string":connection,"schema":schema,"table":table,"limit":100,"offset":0}),
        )?,
        [cmd, sub, connection, sql] if cmd == "db" && sub == "pg-native-query" => call(
            "database.native.postgres.query",
            json!({"connection_string":connection,"sql":sql}),
        )?,
        [cmd, sub, connection, sql] if cmd == "db" && sub == "pg-native-job-start" => call(
            "database.native.postgres.job.start",
            json!({"connection_string":connection,"sql":sql}),
        )?,
        [cmd, sub, job] if cmd == "db" && sub == "pg-native-job-status" => {
            call("database.native.postgres.job.status", json!({"job_id":job}))?
        }
        [cmd, sub] if cmd == "db" && sub == "pg-native-job-list" => {
            call("database.native.postgres.job.list", json!({}))?
        }
        [cmd, sub, job] if cmd == "db" && sub == "pg-native-job-cancel" => {
            call("database.native.postgres.job.cancel", json!({"job_id":job}))?
        }
        [cmd, sub, connection] if cmd == "db" && sub == "pg-native-txn-start" => call(
            "database.native.postgres.txn.start",
            json!({"connection_string":connection,"ttl_seconds":30}),
        )?,
        [cmd, sub, txn, sql] if cmd == "db" && sub == "pg-native-txn-query" => call(
            "database.native.postgres.txn.query",
            json!({"transaction_id":txn,"sql":sql}),
        )?,
        [cmd, sub, txn] if cmd == "db" && sub == "pg-native-txn-status" => call(
            "database.native.postgres.txn.status",
            json!({"transaction_id":txn}),
        )?,
        [cmd, sub, txn, action] if cmd == "db" && sub == "pg-native-txn-close" => call(
            "database.native.postgres.txn.close",
            json!({"transaction_id":txn,"commit":action=="commit"}),
        )?,
        [cmd, sub, connection, schema, table]
            if cmd == "db"
                && matches!(
                    sub.as_str(),
                    "pg-native-indexes" | "pg-native-relations" | "pg-native-stats"
                ) =>
        {
            let command = match sub.as_str() {
                "pg-native-indexes" => "database.native.postgres.indexes",
                "pg-native-relations" => "database.native.postgres.relations",
                _ => "database.native.postgres.stats",
            };
            call(
                command,
                json!({"connection_string":connection,"schema":schema,"table":table}),
            )?
        }
        [cmd, sub, connection, schema, table]
            if cmd == "db"
                && matches!(
                    sub.as_str(),
                    "pg-native-insert" | "pg-native-update" | "pg-native-delete"
                ) =>
        {
            let request: Value = serde_json::from_str(&read_stdin_text()?)?;
            let command = match sub.as_str() {
                "pg-native-insert" => "database.native.postgres.insert",
                "pg-native-update" => "database.native.postgres.update",
                _ => "database.native.postgres.delete",
            };
            call(
                command,
                json!({"connection_string":connection,"schema":schema,"table":table,"request":request}),
            )?
        }
        [cmd, sub, url] if cmd == "db" && sub == "mysql-native-inspect" => {
            call("database.native.mysql.inspect", json!({"url":url}))?
        }
        [cmd, sub, url, database, table] if cmd == "db" && sub == "mysql-native-browse" => call(
            "database.native.mysql.browse",
            json!({"url":url,"database":database,"table":table,"limit":100,"offset":0}),
        )?,
        [cmd, sub, url, sql] if cmd == "db" && sub == "mysql-native-query" => {
            call("database.native.mysql.query", json!({"url":url,"sql":sql}))?
        }
        [cmd, sub, url, database, table]
            if cmd == "db"
                && matches!(
                    sub.as_str(),
                    "mysql-native-indexes" | "mysql-native-relations" | "mysql-native-stats"
                ) =>
        {
            let command = match sub.as_str() {
                "mysql-native-indexes" => "database.native.mysql.indexes",
                "mysql-native-relations" => "database.native.mysql.relations",
                _ => "database.native.mysql.stats",
            };
            call(
                command,
                json!({"url":url,"database":database,"table":table}),
            )?
        }
        [cmd, sub, url, database, table]
            if cmd == "db"
                && matches!(
                    sub.as_str(),
                    "mysql-native-insert" | "mysql-native-update" | "mysql-native-delete"
                ) =>
        {
            let request: Value = serde_json::from_str(&read_stdin_text()?)?;
            let command = match sub.as_str() {
                "mysql-native-insert" => "database.native.mysql.insert",
                "mysql-native-update" => "database.native.mysql.update",
                _ => "database.native.mysql.delete",
            };
            call(
                command,
                json!({"url":url,"database":database,"table":table,"request":request}),
            )?
        }
        [cmd, sub, url] if cmd == "db" && sub == "redis-native-inspect" => {
            call("database.native.redis.inspect", json!({"url":url}))?
        }
        [cmd, sub, url, key] if cmd == "db" && sub == "redis-native-get" => {
            call("database.native.redis.get", json!({"url":url,"key":key}))?
        }
        [cmd, sub, url, key] if cmd == "db" && sub == "redis-native-set" => {
            let value = read_stdin_text()?;
            call(
                "database.native.redis.set",
                json!({"url":url,"key":key,"value":value,"ttl_seconds":Value::Null}),
            )?
        }
        [cmd, sub, url, key, ttl] if cmd == "db" && sub == "redis-native-set" => {
            let value = read_stdin_text()?;
            call(
                "database.native.redis.set",
                json!({"url":url,"key":key,"value":value,"ttl_seconds":parse_u64(ttl,"ttl seconds")?}),
            )?
        }
        [cmd, sub, url, key] if cmd == "db" && sub == "redis-native-delete" => {
            call("database.native.redis.delete", json!({"url":url,"key":key}))?
        }
        [cmd, sub] if cmd == "vault" && sub == "list" => call("vault.list", json!({}))?,
        [cmd, sub, name] if cmd == "vault" && sub == "set" => {
            let value = read_secret_stdin()?;
            call("vault.set", json!({"name":name,"value":value}))?
        }
        [cmd, sub, name] if cmd == "vault" && sub == "delete" => {
            call("vault.delete", json!({"name":name}))?
        }
        [cmd, sub, name] if cmd == "vault" && sub == "reveal" => {
            call("vault.reveal", json!({"name":name}))?
        }
        [cmd, sub] if cmd == "vault" && sub == "status" => call("vault.status", json!({}))?,
        [cmd, sub] if cmd == "vault" && sub == "rotate" => call("vault.rotate", json!({}))?,
        [cmd, sub] if cmd == "vault" && sub == "key-history" => {
            call("vault.key-history", json!({}))?
        }
        [cmd, sub, key] if cmd == "vault" && sub == "restore" => {
            call("vault.restore", json!({"key_id":key,"confirm":true}))?
        }
        [cmd, sub, key] if cmd == "vault" && sub == "retire" => {
            call("vault.retire", json!({"key_id":key,"confirm":true}))?
        }
        [cmd, sub, index, trust] if cmd == "marketplace" && sub == "verify" => {
            call("marketplace.verify", json!({"index":index,"trust":trust}))?
        }
        [cmd, sub, index, trust] if cmd == "marketplace" && sub == "publishers" => call(
            "marketplace.publishers",
            json!({"index":index,"trust":trust}),
        )?,
        [cmd, sub] if cmd == "marketplace" && sub == "conformance" => {
            call("marketplace.conformance", json!({}))?
        }
        [cmd, sub, index, trust, query] if cmd == "marketplace" && sub == "search" => call(
            "marketplace.search",
            json!({"index":index,"trust":trust,"query":query}),
        )?,
        [cmd, sub, index, trust, id, current]
            if cmd == "marketplace" && sub == "resolve-update" =>
        {
            call(
                "marketplace.resolve-update",
                json!({"index":index,"trust":trust,"id":id,"current_version":current}),
            )?
        }
        [cmd, sub, index, trust, id, current, channel]
            if cmd == "marketplace" && sub == "resolve-update-channel" =>
        {
            call(
                "marketplace.resolve-update-channel",
                json!({"index":index,"trust":trust,"id":id,"current_version":current,"channel":channel}),
            )?
        }
        [cmd, sub] if cmd == "extension" && sub == "conformance" => {
            call("extension.conformance", json!({}))?
        }
        [cmd, sub, id, version] if cmd == "extension" && sub == "dependencies" => {
            call("extension.dependencies", json!({"id":id,"version":version}))?
        }
        [cmd, sub] if cmd == "extension" && sub == "list" => call("extension.list", json!({}))?,
        [cmd, sub, package, trust] if cmd == "extension" && sub == "verify" => {
            call("extension.verify", json!({"package":package,"trust":trust}))?
        }
        [cmd, sub, package, trust] if cmd == "extension" && sub == "install" => call(
            "extension.install",
            json!({"package":package,"trust":trust}),
        )?,
        [cmd, sub, id, version] if cmd == "extension" && sub == "uninstall" => {
            call("extension.uninstall", json!({"id":id,"version":version}))?
        }
        [cmd, sub, id, version] if cmd == "extension" && sub == "providers" => {
            call("extension.providers", json!({"id":id,"version":version}))?
        }
        [cmd, sub, id, version, kind] if cmd == "extension" && sub == "providers" => call(
            "extension.providers",
            json!({"id":id,"version":version,"kind":kind}),
        )?,
        [cmd, sub] if cmd == "extension" && sub == "sandbox-capabilities" => {
            call("extension.sandbox-capabilities", json!({}))?
        }
        [cmd, sub] if cmd == "extension" && sub == "exec" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("extension.exec", value)?
        }
        [cmd, sub, path, key] if cmd == "update" && sub == "verify-manifest" => call(
            "update.verify-manifest",
            json!({"path":path,"public_key":key}),
        )?,
        [cmd, sub, path, sha] if cmd == "update" && sub == "verify-artifact" => {
            call("update.verify-artifact", json!({"path":path,"sha256":sha}))?
        }
        [cmd, sub] if cmd == "update" && sub == "apply-file" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)
                .map_err(|e| format!("invalid update apply JSON: {e}"))?;
            call("update.apply-file", value)?
        }
        [cmd, sub, root] if cmd == "update" && sub == "rollback-file" => call(
            "update.rollback-file",
            json!({"install_root":root,"confirm_rollback":true}),
        )?,
        [cmd, sub, root] if cmd == "update" && sub == "status" => {
            call("update.status", json!({"install_root":root}))?
        }
        [cmd, sub, root] if cmd == "update" && sub == "recover-lock" => call(
            "update.recover-lock",
            json!({"install_root":root,"confirm_recover":true}),
        )?,
        [cmd, sub] if cmd == "ai" && sub == "conformance" => call("ai.conformance", json!({}))?,
        [cmd, sub] if cmd == "ai" && sub == "telemetry-summary" => {
            call("ai.telemetry-summary", json!({}))?
        }
        [cmd, sub] if cmd == "ai" && sub == "validate-model-output" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("ai.validate-model-output", value)?
        }
        [cmd, sub] if cmd == "ai" && sub == "capabilities" => call("ai.capabilities", json!({}))?,
        [cmd, sub] if cmd == "ai" && sub == "validate-plan" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("ai.validate-plan", value)?
        }
        [cmd, sub] if cmd == "ai" && sub == "plan" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("ai.plan", value)?
        }
        [cmd, sub] if cmd == "ai" && sub == "execute" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("ai.execute", value)?
        }
        [cmd, sub, path] if cmd == "ai" && sub == "evaluate" => {
            call("ai.evaluate", json!({"path":path}))?
        }
        [cmd, sub] if cmd == "cloud" && sub == "workspace-plan" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.workspace-plan", value)?
        }
        [cmd, sub] if cmd == "cloud" && sub == "ssh-preflight" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.ssh-preflight", value)?
        }
        [cmd, sub] if cmd == "cloud" && sub == "ssh-workspace-prepare" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.ssh-workspace.prepare", value)?
        }
        [cmd, sub] if cmd == "cloud" && sub == "ssh-workspace-status" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.ssh-workspace.status", value)?
        }
        [cmd, sub] if cmd == "cloud" && sub == "ssh-workspace-remove-empty" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.ssh-workspace.remove-empty", value)?
        }
        [cmd, sub] if cmd == "cloud" && sub == "ssh-release-upload" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.ssh-release.upload", value)?
        }
        [cmd, sub] if cmd == "cloud" && sub == "ssh-release-activate" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.ssh-release.activate", value)?
        }
        [cmd, sub] if cmd == "cloud" && sub == "ssh-release-status" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.ssh-release.status", value)?
        }
        [cmd, sub] if cmd == "cloud" && sub == "ssh-release-rollback" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.ssh-release.rollback", value)?
        }
        [cmd, sub] if cmd == "cloud" && sub == "ssh-release-health" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.ssh-release.health", value)?
        }
        [cmd, sub] if cmd == "cloud" && sub == "conformance" => {
            call("cloud.conformance", json!({}))?
        }
        [cmd, sub] if cmd == "cloud" && sub == "cli-detect" => call("cloud.cli.detect", json!({}))?,
        [cmd, sub] if cmd == "cloud" && sub == "cli-create" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.cli.create", value)?
        }
        [cmd, sub] if cmd == "cloud" && sub == "cli-status" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.cli.status", value)?
        }
        [cmd, sub] if cmd == "cloud" && sub == "cli-start" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.cli.start", value)?
        }
        [cmd, sub] if cmd == "cloud" && sub == "cli-stop" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.cli.stop", value)?
        }
        [cmd, sub] if cmd == "cloud" && sub == "cli-snapshot" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.cli.snapshot", value)?
        }
        [cmd, sub] if cmd == "cloud" && sub == "cli-clone" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.cli.clone", value)?
        }
        [cmd, sub] if cmd == "cloud" && sub == "cli-copy-image" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.cli.copy-image", value)?
        }
        [cmd, sub] if cmd == "cloud" && sub == "cli-copy-status" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.cli.copy-status", value)?
        }
        [cmd, sub] if cmd == "cloud" && sub == "cli-destroy" => {
            let value: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("cloud.cli.destroy", value)?
        }
        [cmd, sub] if cmd == "stream" && sub == "open" => {
            let request: Value = serde_json::from_str(&read_stdin_text()?)?;
            call("stream.open", request)?
        }
        [cmd, sub, id, seq, eof] if cmd == "stream" && sub == "input" => {
            let payload_base64 = read_stdin_text()?.trim().to_string();
            call(
                "stream.input",
                json!({"id":id,"seq":parse_u64(seq,"sequence")?,"payload_base64":payload_base64,"eof":parse_bool(eof)?}),
            )?
        }
        [cmd, sub, id] if cmd == "stream" && sub == "input-pull" => {
            call("stream.input.pull", json!({"id":id,"max_frames":16}))?
        }
        [cmd, sub, id, eof] if cmd == "stream" && sub == "output" => {
            let payload_base64 = read_stdin_text()?.trim().to_string();
            call(
                "stream.output",
                json!({"id":id,"payload_base64":payload_base64,"eof":parse_bool(eof)?}),
            )?
        }
        [cmd, sub, id] if cmd == "stream" && sub == "pull" => {
            call("stream.pull", json!({"id":id,"max_frames":16}))?
        }
        [cmd, sub, id] if cmd == "stream" && sub == "close" => {
            call("stream.close", json!({"id":id,"reason":"cli_close"}))?
        }
        [cmd, sub] if cmd == "stream" && sub == "list" => call("stream.list", json!({}))?,
        [cmd, sub] if cmd == "remote" && sub == "status" => call("remote.status", json!({}))?,
        [cmd, sub, url, key] if cmd == "remote" && sub == "configure" => call(
            "remote.configure",
            json!({"url":url,"public_key":key,"enabled":false,"poll_interval_ms":2500,"allow_remote_terminal":false,"allow_remote_file_write":false,"allow_remote_database_query":false}),
        )?,
        [cmd, sub, url, key, enabled] if cmd == "remote" && sub == "configure" => call(
            "remote.configure",
            json!({"url":url,"public_key":key,"enabled":parse_bool(enabled)?,"poll_interval_ms":2500,"allow_remote_terminal":false,"allow_remote_file_write":false,"allow_remote_database_query":false}),
        )?,
        [cmd, sub, url, key, enabled, terminal, file_write, db_query]
            if cmd == "remote" && sub == "configure" =>
        {
            call(
                "remote.configure",
                json!({"url":url,"public_key":key,"enabled":parse_bool(enabled)?,"poll_interval_ms":2500,"allow_remote_terminal":parse_bool(terminal)?,"allow_remote_file_write":parse_bool(file_write)?,"allow_remote_database_query":parse_bool(db_query)?}),
            )?
        }
        [cmd, sub, nonce] if cmd == "remote" && sub == "enroll" => {
            call("remote.enroll", json!({"pairing_nonce":nonce}))?
        }
        [cmd] if cmd == "commands" => return Ok(Some(command_catalog())),
        [cmd, shell] if cmd == "completion" => {
            print!("{}", completion_script(shell)?);
            return Ok(None);
        }
        [cmd] if cmd == "version" || cmd == "--version" || cmd == "-V" => {
            println!("vsn 0.38.1");
            return Ok(None);
        }
        [] => {
            help();
            return Ok(None);
        }
        [cmd] if cmd == "help" || cmd == "--help" || cmd == "-h" => {
            help();
            return Ok(None);
        }
        _ => {
            help();
            return Err("unknown command".into());
        }
    };
    Ok(Some(response))
}
fn call(command: &str, params: Value) -> Result<Value, Box<dyn std::error::Error>> {
    let response = vsn_ipc::call(command, params)?;
    if response.ok {
        Ok(response.payload)
    } else {
        Err(format!("agent rejected request: {}", response.payload).into())
    }
}
fn parse_u16(value: &str) -> Result<u16, Box<dyn std::error::Error>> {
    value.parse::<u16>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid port: {value}"),
        )
        .into()
    })
}
fn parse_u32(value: &str, label: &str) -> Result<u32, Box<dyn std::error::Error>> {
    value.parse::<u32>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid {label}: {value}"),
        )
        .into()
    })
}
fn parse_u64(value: &str, label: &str) -> Result<u64, Box<dyn std::error::Error>> {
    value.parse::<u64>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid {label}: {value}"),
        )
        .into()
    })
}
fn parse_bool(v: &str) -> Result<bool, Box<dyn std::error::Error>> {
    match v {
        "true" | "1" | "on" => Ok(true),
        "false" | "0" | "off" => Ok(false),
        _ => Err("enabled must be true or false".into()),
    }
}
fn read_secret_stdin() -> Result<String, Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let value = input.trim_end_matches(['\r', '\n']).to_string();
    if value.is_empty() {
        return Err("secret value must be provided on stdin".into());
    }
    Ok(value)
}
fn read_stdin_text() -> Result<String, Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    Ok(input)
}
fn opt_arg(value: &str) -> Option<String> {
    if value == "-" || value.eq_ignore_ascii_case("none") {
        None
    } else {
        Some(value.to_string())
    }
}
fn db_connection_tls_json(
    engine: &str,
    host: &str,
    port: &str,
    user: &str,
    database: &str,
    root_ca: &str,
    credential: Option<&String>,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut value = db_connection_json(engine, host, port, user, database, credential)?;
    let object = value
        .as_object_mut()
        .ok_or("database connection must serialize as an object")?;
    object.insert("root_ca_file".into(), Value::String(root_ca.to_string()));
    Ok(value)
}
fn db_connection_json(
    engine: &str,
    host: &str,
    port: &str,
    user: &str,
    database: &str,
    credential: Option<&String>,
) -> Result<Value, Box<dyn std::error::Error>> {
    let port = if port == "-" {
        None
    } else {
        Some(parse_u16(port)?)
    };
    Ok(
        json!({"engine":engine,"host":opt_arg(host),"port":port,"user":opt_arg(user),"database":opt_arg(database),"credential_file":credential.and_then(|v|opt_arg(v)),"root_ca_file":null,"service":null}),
    )
}
const CLI_TOP_LEVEL: &[&str] = &[
    "status",
    "machine",
    "security",
    "diagnostics",
    "ping",
    "config",
    "audit",
    "process",
    "managed",
    "port",
    "service",
    "health",
    "log",
    "runtime",
    "container",
    "project",
    "workspace",
    "files",
    "terminal",
    "preview",
    "db",
    "domain",
    "dns",
    "vault",
    "extension",
    "marketplace",
    "update",
    "ai",
    "cloud",
    "stream",
    "remote",
    "commands",
    "completion",
    "version",
    "help",
];
fn command_catalog() -> Value {
    json!({"version":"0.38.1","top_level":CLI_TOP_LEVEL,"groups":{"ai":["capabilities","conformance","evaluate","execute","plan","telemetry-summary","validate-model-output","validate-plan"],"audit":["verify"],"cloud":["cli-clone","cli-copy-image","cli-copy-status","cli-create","cli-destroy","cli-detect","cli-snapshot","cli-start","cli-status","cli-stop","conformance","ssh-preflight","ssh-release-activate","ssh-release-health","ssh-release-rollback","ssh-release-status","ssh-release-upload","ssh-workspace-prepare","ssh-workspace-remove-empty","ssh-workspace-status","workspace-plan"],"config":["show"],"container":["backends","build","compose","exec","images","inspect","list","logs","networks","pause","pull","registry-publish","remove","restart","start","stats","stop","unpause","volumes"],"db":["clients","inspect","job-cancel","job-list","job-output","job-output-remove","job-start","job-status","model-analyze","mysql-native-browse","mysql-native-delete","mysql-native-indexes","mysql-native-insert","mysql-native-inspect","mysql-native-query","mysql-native-relations","mysql-native-stats","mysql-native-update","pg-native-browse","pg-native-delete","pg-native-indexes","pg-native-insert","pg-native-inspect","pg-native-job-cancel","pg-native-job-list","pg-native-job-start","pg-native-job-status","pg-native-query","pg-native-relations","pg-native-stats","pg-native-txn-close","pg-native-txn-query","pg-native-txn-start","pg-native-txn-status","pg-native-update","query","redis-native-delete","redis-native-get","redis-native-inspect","redis-native-set","remote-conformance","sqlite-browse","sqlite-delete","sqlite-indexes","sqlite-insert","sqlite-inspect","sqlite-query","sqlite-relations","sqlite-stats","sqlite-update","studio-conformance","ui-demo","workspace"],"dns":["plan","start","status","stop"],"domain":["apply","conformance","plan","reload","remove"],"extension":["conformance","dependencies","exec","install","list","providers","sandbox-capabilities","uninstall","verify"],"files":["binary-abort","binary-read","binary-status","binary-write","conformance","delete","digest","list","mkdir","move","read","write"],"health":["tcp"],"log":["tail"],"managed":["list","remove","status","stop"],"marketplace":["conformance","publishers","resolve-update","resolve-update-channel","search","verify"],"port":["check","list"],"preview":["conformance","fetch","request"],"process":["list","metrics"],"project":["bootstrap","bootstrap-plan","conformance","dependencies","detect","templates"],"remote":["configure","enroll","status"],"runtime":["activate","audit","catalog","catalog-verify","conformance","install","install-trusted","list","registry","repair","uninstall"],"service":["conformance","restart","start","status","stop"],"stream":["close","input","input-pull","list","open","output","pull"],"terminal":["conformance","exec","list","pty-list","pty-read","pty-read-wait","pty-recovery-list","pty-recovery-remove","pty-remove","pty-resize","pty-scrollback-list","pty-scrollback-read","pty-scrollback-remove","pty-start","pty-status","pty-stop","pty-write","read","read-wait","remove","start","status","stop","write"],"update":["apply-file","recover-lock","rollback-file","status","verify-artifact","verify-manifest"],"vault":["delete","key-history","list","restore","retire","reveal","rotate","set","status"],"workspace":["add","list","remove"]}})
}
fn completion_script(shell: &str) -> Result<String, Box<dyn std::error::Error>> {
    let words = CLI_TOP_LEVEL.join(" ");
    match shell{
"bash"=>Ok(format!("_vsn_complete() {{ local cur=\"${{COMP_WORDS[COMP_CWORD]}}\"; COMPREPLY=( $(compgen -W '{words}' -- \"$cur\") ); }}\ncomplete -F _vsn_complete vsn\n")),
"zsh"=>Ok(format!("#compdef vsn\n_arguments '1:command:({words})' '*::arg:->args'\n")),
"powershell"|"pwsh"=>Ok(format!("Register-ArgumentCompleter -Native -CommandName vsn -ScriptBlock {{ param($wordToComplete) '{words}'.Split(' ') | Where-Object {{ $_ -like \"$wordToComplete*\" }} | ForEach-Object {{ [System.Management.Automation.CompletionResult]::new($_,$_,\"ParameterValue\",$_) }} }}\n")),
_=>Err("completion shell must be bash, zsh, powershell, or pwsh".into())}
}
fn help() {
    println!("VSN CLI 0.38.1\n\nCore: vsn ping | status | machine | security | diagnostics | audit verify | config show\nSystem: process/port/service/health commands\nRuntime: list/registry/repair/catalog/catalog-verify/install/install-trusted/activate/uninstall\nContainers: backends/list/images/volumes/networks/logs/inspect/stats/start/stop/restart/compose/pull/build/remove/registry-publish/exec\nProjects: detect/dependencies/bootstrap-plan/bootstrap/templates/conformance; workspace list/add/remove\nFiles: list/read/write/mkdir/move/delete; binary-read/binary-write/binary-abort/binary-status; digest\nTerminal: exec; pipe sessions; PTY start/read/write/resize/status/stop/remove/list; scrollback and recovery metadata list/read/remove\nPreview: bounded localhost HTTP/SSE/WebSocket relay surfaces\nStreams: open/input/input-pull/output/pull/close/list\nDatabase: SQLite/native/external reads and CRUD; durable CLI query jobs; native PostgreSQL server cancellation + read-only transactions\nNetworking: domain plan; dns plan/start/status/stop; privileged OS .test resolver apply/status/remove via `vsn-agent network-admin`\nCloud: workspace/release lifecycle; AWS/Azure/GCP CLI create/status/start/stop/snapshot/clone/destroy\nMarketplace: signed verify/search/update, revocations and channel-aware update resolution\nExtensions: verify/install/list/providers/uninstall; Linux Bubblewrap executable sandbox capability/exec, fail-closed elsewhere\nAI: plan/execute/evaluate; mutations require pre-confirmation and unrestricted shell remains disabled\nVault: list/set/reveal/delete/status/rotate/key-history/restore/retire with OS-secure-store key IDs\nRemote: status/configure/enroll; remote mutation capabilities remain local opt-in/fail-closed\nDiscovery: vsn commands | vsn completion bash|zsh|powershell");
}
