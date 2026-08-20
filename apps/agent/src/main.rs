use serde_json::{json,Value};
use std::{collections::{BTreeMap,HashMap},path::Path,sync::{Arc,atomic::{AtomicBool,Ordering}},thread,time::Duration,process::ExitCode};
use vsn_audit::AuditEventInput;
use vsn_ipc::RequestEnvelope;

fn main()->ExitCode{
    let args:Vec<String>=std::env::args().skip(1).collect();
    if args.first().map(String::as_str)==Some("service"){return service_command(&args[1..]);}
    if args.first().map(String::as_str)==Some("network-admin"){return network_admin_command(&args[1..]);}
    if args.first().map(String::as_str)==Some("dns-server"){return dns_server_command(&args[1..]);}
    #[cfg(windows)] if args.iter().any(|arg|arg=="--service-run"){return match windows_service_host::dispatch(){Ok(())=>ExitCode::SUCCESS,Err(error)=>{eprintln!("service_error={error}");ExitCode::FAILURE}};}
    if args.iter().any(|arg|arg=="--once"){return match initialize_once(){Ok(())=>ExitCode::SUCCESS,Err(error)=>{eprintln!("agent_init_error={error}");ExitCode::FAILURE}};}
    let stop=Arc::new(AtomicBool::new(false));let ctrlc_stop=Arc::clone(&stop);if let Err(error)=ctrlc::set_handler(move||ctrlc_stop.store(true,Ordering::SeqCst)){eprintln!("ctrlc_handler_error={error}");return ExitCode::FAILURE;}
    match run_agent(stop){Ok(())=>ExitCode::SUCCESS,Err(error)=>{eprintln!("agent_error={error}");ExitCode::FAILURE}}
}

fn initialize_once()->Result<(),Box<dyn std::error::Error>>{let identity=vsn_core::local_machine_identity()?;let security=vsn_core::security_status()?;let health=vsn_core::core_health();println!("VSN Agent 0.38.1");println!("device_id={}",identity.device_id);println!("machine={}",identity.display_name);println!("os={}",identity.os);println!("public_key={}",identity.public_key);println!("core_healthy={}",health.healthy);println!("secure_store={}",security.secure_store);println!("ipc_secret_ready={}",security.ipc_secret_ready);Ok(())}

fn run_agent(stop:Arc<AtomicBool>)->Result<(),Box<dyn std::error::Error>>{
    let identity=vsn_core::local_machine_identity()?;let security=vsn_core::security_status()?;let _=audit("agent",&identity.device_id,"agent.start","vsn-agent","success",BTreeMap::new());
    println!("VSN Agent 0.38.1");println!("device_id={}",identity.device_id);println!("agent_state=running");println!("ipc_address={}",vsn_ipc::IPC_ADDRESS);println!("secure_store={}",security.secure_store);
    let remote_stop=Arc::clone(&stop);let remote_thread=thread::spawn(move||remote_loop(remote_stop));
    let stream_stop=Arc::clone(&stop);let _stream_thread=thread::spawn(move||stream_relay_loop(stream_stop));
    let agent_device_id=identity.device_id.clone();
    let handler=move|request:RequestEnvelope|->(bool,Value){let command=request.command.clone();let params=request.params.clone();let principal=vsn_policy::Principal::local_authenticated();let response=dispatch_command(&principal,&command,&params);let mut metadata=BTreeMap::new();metadata.insert("nonce".into(),request.nonce);let _=audit("local-ipc-client","local-user",&format!("ipc.{command}"),&agent_device_id,if response.0{"success"}else{"denied_or_failed"},metadata);response};
    let serve_result=vsn_ipc::serve_until(Arc::clone(&stop),handler);stop.store(true,Ordering::SeqCst);let _=remote_thread.join();let _=audit("agent",&identity.device_id,"agent.stop","vsn-agent",if serve_result.is_ok(){"success"}else{"failed"},BTreeMap::new());serve_result?;Ok(())
}

fn dispatch_command(principal:&vsn_policy::Principal,command:&str,params:&Value)->(bool,Value){
    if principal.kind=="remote_signed_command" {
        let Some(required)=required_remote_permission(command) else { return (false,json!({"error":"command is not exposed to remote execution"})); };
        if let Err(error)=vsn_policy::require(principal,required){return (false,json!({"error":error.to_string()}));}
        if let Ok(remote)=vsn_core::config().map(|c|c.remote){
            if command.starts_with("terminal.")&&!remote.allow_remote_terminal{return (false,json!({"error":"remote terminal is disabled locally on this device"}));}
            if (command=="files.write"||command=="files.binary.write"||command=="files.binary.abort"||command=="files.mkdir"||command=="files.move"||command=="files.delete")&&!remote.allow_remote_file_write{return (false,json!({"error":"remote file writes are disabled locally on this device"}));}
            if matches!(command,"database.cli.query"|"database.cli.job.start"|"database.cli.job.cancel"|"database.cli.job.output"|"database.cli.job.output-remove"|"database.native.postgres.query"|"database.native.postgres.job.start"|"database.native.postgres.job.cancel"|"database.native.postgres.txn.start"|"database.native.postgres.txn.query"|"database.native.postgres.txn.close"|"database.native.mysql.query")&&!remote.allow_remote_database_query{return (false,json!({"error":"remote database queries are disabled locally on this device"}));}
        }
    }
    match command{
        "ping"=>(true,json!({"pong":true,"version":"0.38.1"})),
        "status"=>{let health=vsn_core::core_health();match vsn_core::security_status(){Ok(security)=>{let remote=vsn_core::remote_status(principal).ok();(true,json!({"health":health,"security":security,"remote":remote}))},Err(e)=>(false,json!({"error":e.to_string()}))}},
        "machine"=>respond(vsn_core::local_machine_identity()),"security.status"=>respond(vsn_core::security_status()),"diagnostics"=>respond(vsn_core::diagnostics(principal)),"config.show"=>respond(vsn_core::config()),
        "audit.verify"=>match vsn_core::verify_audit(){Ok(events)=>(true,json!({"valid":true,"events":events})),Err(error)=>(false,json!({"valid":false,"error":error.to_string()}))},
        "process.list"=>respond(vsn_core::processes(principal)),"process.metrics"=>match param_u32(params,"pid"){Ok(pid)=>respond(vsn_core::process_metrics(principal,pid)),Err(e)=>(false,json!({"error":e}))},
        "process.managed.list"=>respond(vsn_core::managed_process_list(principal)),
        "process.managed.status"=>match param_str(params,"id"){Ok(id)=>respond(vsn_core::managed_process_state(principal,id)),Err(e)=>(false,json!({"error":e}))},
        "process.managed.stop"=>match param_str(params,"id"){Ok(id)=>respond(vsn_core::managed_process_stop(principal,id)),Err(e)=>(false,json!({"error":e}))},
        "process.managed.remove"=>match param_str(params,"id"){Ok(id)=>respond(vsn_core::managed_process_remove(principal,id,params.get("force").and_then(Value::as_bool).unwrap_or(false))),Err(e)=>(false,json!({"error":e}))},
        "process.managed.start"=>respond(parse_managed_spec(params).and_then(|s|vsn_core::managed_process_start(principal,&s).map_err(|e|e.to_string()))),
        "port.list"=>respond(vsn_core::ports(principal)),"port.check"=>match param_u16(params,"port"){Ok(port)=>respond(vsn_core::port_conflicts(principal,port)),Err(e)=>(false,json!({"error":e}))},
        "service.status"=>match param_str(params,"name"){Ok(name)=>respond(vsn_core::service_state(principal,name)),Err(e)=>(false,json!({"error":e}))},
        "service.action"=>match (param_str(params,"name"),param_str(params,"action")){(Ok(name),Ok(action))=>respond(vsn_core::service_action(principal,name,action)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "service.conformance"=>respond(vsn_core::service_conformance(principal)),
        "health.tcp"=>match (param_str(params,"host"),param_u16(params,"port")){(Ok(host),Ok(port))=>respond(vsn_core::tcp_health(principal,host,port,params.get("timeout_ms").and_then(Value::as_u64).unwrap_or(1500))),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "log.tail"=>match param_str(params,"path"){Ok(path)=>respond(vsn_core::tail_log(principal,Path::new(path),params.get("lines").and_then(Value::as_u64).unwrap_or(100)as usize)),Err(e)=>(false,json!({"error":e}))},
        "runtime.list"=>respond(vsn_core::runtime_detect(principal)),"runtime.registry"=>respond(vsn_core::runtime_registry(principal)),
        "runtime.conformance"=>respond(vsn_core::runtime_provider_conformance(principal)),
        "runtime.repair"=>respond(vsn_core::runtime_repair(principal)),
        "runtime.audit"=>respond(vsn_core::runtime_audit(principal)),
        "runtime.catalog"=>match param_str(params,"path"){Ok(path)=>respond(vsn_core::runtime_catalog(principal,Path::new(path))),Err(e)=>(false,json!({"error":e}))},
        "runtime.catalog-verify"=>match (param_str(params,"path"),param_str(params,"trust")){(Ok(p),Ok(t))=>respond(vsn_core::runtime_catalog_verify(principal,Path::new(p),Path::new(t))),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "runtime.install"=>match (param_str(params,"catalog"),param_str(params,"runtime"),param_str(params,"version")){(Ok(c),Ok(r),Ok(v))=>respond(vsn_core::runtime_install(principal,Path::new(c),r,v)),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "runtime.install-trusted"=>match (param_str(params,"catalog"),param_str(params,"trust"),param_str(params,"runtime"),param_str(params,"version")){(Ok(c),Ok(t),Ok(r),Ok(v))=>respond(vsn_core::runtime_install_trusted(principal,Path::new(c),Path::new(t),r,v)),(Err(e),_,_,_)|(_,Err(e),_,_)|(_,_,Err(e),_)|(_,_,_,Err(e))=>(false,json!({"error":e}))},
        "runtime.activate"=>match (param_str(params,"project"),param_str(params,"runtime"),param_str(params,"version")){(Ok(p),Ok(r),Ok(v))=>respond(vsn_core::runtime_activate(principal,Path::new(p),r,v)),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "runtime.uninstall"=>match (param_str(params,"runtime"),param_str(params,"version")){(Ok(r),Ok(v))=>respond(vsn_core::runtime_uninstall(principal,r,v)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "container.backends"=>respond(vsn_core::container_detect(principal)),"container.list"=>match param_str(params,"backend"){Ok(b)=>respond(vsn_core::container_list(principal,b,params.get("all").and_then(Value::as_bool).unwrap_or(true))),Err(e)=>(false,json!({"error":e}))},
        "container.images"=>match param_str(params,"backend"){Ok(b)=>respond(vsn_core::container_images(principal,b)),Err(e)=>(false,json!({"error":e}))},
        "container.volumes"=>match param_str(params,"backend"){Ok(b)=>respond(vsn_core::container_volumes(principal,b)),Err(e)=>(false,json!({"error":e}))},
        "container.networks"=>match param_str(params,"backend"){Ok(b)=>respond(vsn_core::container_networks(principal,b)),Err(e)=>(false,json!({"error":e}))},
        "container.logs"=>match (param_str(params,"backend"),param_str(params,"target")){(Ok(b),Ok(t))=>respond(vsn_core::container_logs(principal,b,t,params.get("tail").and_then(Value::as_u64).and_then(|v|u32::try_from(v).ok()).unwrap_or(200))),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "container.inspect"=>match (param_str(params,"backend"),param_str(params,"target")){(Ok(b),Ok(t))=>respond(vsn_core::container_inspect(principal,b,t)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "container.stats"=>match (param_str(params,"backend"),param_str(params,"target")){(Ok(b),Ok(t))=>respond(vsn_core::container_stats(principal,b,t)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "container.action"=>match (param_str(params,"backend"),param_str(params,"action"),param_str(params,"target")){(Ok(b),Ok(a),Ok(t))=>respond(vsn_core::container_action(principal,b,a,t)),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "container.image-pull"=>match (param_str(params,"backend"),param_str(params,"image")){(Ok(b),Ok(i))=>respond(vsn_core::container_image_pull(principal,b,i)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "container.image-build"=>respond(serde_json::from_value::<vsn_container::ContainerBuildRequest>(params.clone()).map_err(|e|format!("invalid container build request: {e}")).and_then(|r|vsn_core::container_image_build(principal,&r).map_err(|e|e.to_string()))),
        "container.registry-publish"=>respond(serde_json::from_value::<vsn_container::RegistryPushRequest>(params.clone()).map_err(|e|format!("invalid container registry request: {e}")).and_then(|r|vsn_core::container_registry_publish(principal,&r).map_err(|e|e.to_string()))),
        "container.exec"=>respond(serde_json::from_value::<vsn_container::ContainerExecRequest>(params.clone()).map_err(|e|format!("invalid container exec request: {e}")).and_then(|r|vsn_core::container_exec(principal,&r).map_err(|e|e.to_string()))),
        "container.remove"=>match (param_str(params,"backend"),param_str(params,"kind"),param_str(params,"target")){(Ok(b),Ok(k),Ok(t))=>respond(vsn_core::container_remove(principal,b,k,t,params.get("force").and_then(Value::as_bool).unwrap_or(false))),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "container.compose"=>match (param_str(params,"backend"),param_str(params,"path"),param_str(params,"action")){(Ok(b),Ok(p),Ok(a))=>respond(vsn_core::compose_action(principal,b,Path::new(p),a)),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "project.detect"=>match param_str(params,"path"){Ok(path)=>respond(vsn_core::project_detect(principal,Path::new(path))),Err(e)=>(false,json!({"error":e}))},
        "project.dependencies"=>match param_str(params,"path"){Ok(path)=>respond(vsn_core::project_dependencies(principal,Path::new(path))),Err(e)=>(false,json!({"error":e}))},
        "project.bootstrap-plan"=>match (param_str(params,"template"),param_str(params,"path")){(Ok(t),Ok(p))=>respond(vsn_core::project_bootstrap_plan(principal,t,Path::new(p))),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "project.bootstrap"=>match (param_str(params,"template"),param_str(params,"path")){(Ok(t),Ok(p))=>respond(vsn_core::project_bootstrap(principal,t,Path::new(p))),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "project.conformance"=>respond(vsn_core::project_provider_conformance(principal)),
        "project.templates"=>respond(vsn_core::project_templates(principal)),
        "network.conformance"=>respond(vsn_core::network_conformance(principal)),
        "domain.reload"=>respond(vsn_core::caddy_reload(principal)),
        "workspace.roots"=>respond(vsn_core::workspace_roots(principal)),
        "workspace.add"=>match param_str(params,"path"){Ok(p)=>respond(vsn_core::workspace_add(principal,Path::new(p))),Err(e)=>(false,json!({"error":e}))},
        "workspace.remove"=>match param_str(params,"path"){Ok(p)=>respond(vsn_core::workspace_remove(principal,Path::new(p))),Err(e)=>(false,json!({"error":e}))},
        "files.conformance"=>respond(vsn_core::file_conformance(principal)),
        "files.list"=>match param_str(params,"path"){Ok(p)=>respond(vsn_core::file_list(principal,Path::new(p))),Err(e)=>(false,json!({"error":e}))},
        "files.read"=>match param_str(params,"path"){Ok(p)=>respond(vsn_core::file_read(principal,Path::new(p))),Err(e)=>(false,json!({"error":e}))},
        "files.write"=>match (param_str(params,"path"),param_str(params,"content")){(Ok(p),Ok(content))=>respond(vsn_core::file_write(principal,Path::new(p),content)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "files.binary.read"=>match param_str(params,"path"){Ok(p)=>respond(vsn_core::file_read_binary_chunk(principal,Path::new(p),params.get("offset").and_then(Value::as_u64).unwrap_or(0),params.get("max_bytes").and_then(Value::as_u64).and_then(|v|usize::try_from(v).ok()).unwrap_or(vsn_files::MAX_BINARY_CHUNK_BYTES))),Err(e)=>(false,json!({"error":e}))},
        "files.binary.write"=>match (param_str(params,"path"),param_str(params,"transfer_id"),param_str(params,"data_b64")){(Ok(p),Ok(t),Ok(data))=>respond(vsn_core::file_write_binary_chunk(principal,Path::new(p),t,params.get("offset").and_then(Value::as_u64).unwrap_or(0),data,params.get("finalize").and_then(Value::as_bool).unwrap_or(false),params.get("expected_sha256").and_then(Value::as_str))),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "files.binary.abort"=>match (param_str(params,"path"),param_str(params,"transfer_id")){(Ok(p),Ok(t))=>respond(vsn_core::file_abort_binary_upload(principal,Path::new(p),t)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "files.binary.status"=>match (param_str(params,"path"),param_str(params,"transfer_id")){(Ok(p),Ok(t))=>respond(vsn_core::file_binary_upload_status(principal,Path::new(p),t)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "files.digest"=>match param_str(params,"path"){Ok(p)=>respond(vsn_core::file_digest(principal,Path::new(p))),Err(e)=>(false,json!({"error":e}))},
        "files.mkdir"=>match param_str(params,"path"){Ok(p)=>respond(vsn_core::file_create_dir(principal,Path::new(p))),Err(e)=>(false,json!({"error":e}))},
        "files.move"=>match (param_str(params,"source"),param_str(params,"destination")){(Ok(a),Ok(b))=>respond(vsn_core::file_move(principal,Path::new(a),Path::new(b))),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "files.delete"=>match param_str(params,"path"){Ok(p)=>respond(vsn_core::file_delete(principal,Path::new(p),params.get("recursive").and_then(Value::as_bool).unwrap_or(false))),Err(e)=>(false,json!({"error":e}))},
        "ai.conformance"=>respond(vsn_core::ai_conformance(principal)),
        "ai.telemetry-summary"=>respond(vsn_core::ai_telemetry_summary(principal)),
        "ai.validate-model-output"=>{let adapter=params.get("adapter").cloned().ok_or_else(||"missing adapter".to_string()).and_then(|v|serde_json::from_value::<vsn_ai::ModelAdapterDescriptor>(v).map_err(|e|e.to_string()));let output=params.get("output_json").and_then(Value::as_str).ok_or_else(||"missing output_json".to_string());match(adapter,output){(Ok(a),Ok(o))=>respond(vsn_core::ai_validate_model_output(principal,&a,o.as_bytes())),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))}},
        "ai.plan"=>respond(serde_json::from_value::<vsn_ai::StructuredIntent>(params.clone()).map_err(|e|format!("invalid ai intent: {e}")).and_then(|intent|vsn_core::ai_plan(principal,&intent).map_err(|e|e.to_string()))),
        "ai.capabilities"=>respond(vsn_core::ai_capabilities(principal)),
        "ai.validate-plan"=>respond(serde_json::from_value::<vsn_ai::ToolPlan>(params.clone()).map_err(|e|format!("invalid ai tool plan: {e}")).and_then(|plan|vsn_core::ai_validate_plan(principal,&plan).map_err(|e|e.to_string()))),
        "ai.evaluate"=>match param_str(params,"path"){Ok(path)=>respond(vsn_core::ai_evaluate(principal,Path::new(path))),Err(e)=>(false,json!({"error":e}))},
        "ai.execute"=>respond(serde_json::from_value::<vsn_ai::ExecuteRequest>(params.clone()).map_err(|e|format!("invalid ai execute request: {e}")).and_then(|request|execute_ai_request(principal,&request))),
        "cloud.workspace-plan"=>respond(serde_json::from_value::<vsn_cloud::WorkspaceSpec>(params.clone()).map_err(|e|format!("invalid cloud workspace spec: {e}")).and_then(|spec|vsn_core::cloud_workspace_plan(principal,&spec).map_err(|e|e.to_string()))),
        "cloud.ssh-preflight"=>respond(serde_json::from_value::<vsn_cloud::ExistingSshTarget>(params.clone()).map_err(|e|format!("invalid ssh target: {e}")).and_then(|target|vsn_core::cloud_ssh_preflight(principal,&target).map_err(|e|e.to_string()))),
        "cloud.ssh-workspace.prepare"=>respond(serde_json::from_value::<vsn_cloud::ExistingSshWorkspaceRequest>(params.clone()).map_err(|e|format!("invalid SSH workspace request: {e}")).and_then(|r|vsn_core::cloud_ssh_workspace_prepare(principal,&r).map_err(|e|e.to_string()))),
        "cloud.ssh-workspace.status"=>respond(serde_json::from_value::<vsn_cloud::ExistingSshWorkspaceRequest>(params.clone()).map_err(|e|format!("invalid SSH workspace request: {e}")).and_then(|r|vsn_core::cloud_ssh_workspace_status(principal,&r).map_err(|e|e.to_string()))),
        "cloud.ssh-workspace.remove-empty"=>respond(serde_json::from_value::<vsn_cloud::ExistingSshWorkspaceRequest>(params.clone()).map_err(|e|format!("invalid SSH workspace request: {e}")).and_then(|r|vsn_core::cloud_ssh_workspace_remove_empty(principal,&r).map_err(|e|e.to_string()))),
        "cloud.ssh-release.upload"=>respond(serde_json::from_value::<vsn_cloud::SshReleaseUploadRequest>(params.clone()).map_err(|e|format!("invalid SSH release upload request: {e}")).and_then(|r|vsn_core::cloud_ssh_release_upload(principal,&r).map_err(|e|e.to_string()))),
        "cloud.ssh-release.activate"=>respond(serde_json::from_value::<vsn_cloud::SshReleasePointerRequest>(params.clone()).map_err(|e|format!("invalid SSH release activation request: {e}")).and_then(|r|vsn_core::cloud_ssh_release_activate(principal,&r).map_err(|e|e.to_string()))),
        "cloud.ssh-release.status"=>respond(parse_ssh_release_status(params).and_then(|(target,name)|vsn_core::cloud_ssh_release_status(principal,&target,&name).map_err(|e|e.to_string()))),
        "cloud.ssh-release.rollback"=>respond(parse_ssh_release_status(params).and_then(|(target,name)|vsn_core::cloud_ssh_release_rollback(principal,&target,&name).map_err(|e|e.to_string()))),
        "cloud.ssh-release.health"=>respond(serde_json::from_value::<vsn_cloud::SshReleaseHealthRequest>(params.clone()).map_err(|e|format!("invalid SSH release health request: {e}")).and_then(|r|vsn_core::cloud_ssh_release_healthcheck(principal,&r).map_err(|e|e.to_string()))),
        "cloud.cli.detect"=>respond(vsn_core::cloud_cli_detect(principal).map_err(|e|e.to_string())),
        "cloud.cli.create"=>respond(serde_json::from_value::<vsn_cloud::CloudCliCreateRequest>(params.clone()).map_err(|e|format!("invalid cloud CLI create request: {e}")).and_then(|r|vsn_core::cloud_cli_create(principal,&r).map_err(|e|e.to_string()))),
        "cloud.cli.status"=>respond(serde_json::from_value::<vsn_cloud::CloudCliInstanceRef>(params.clone()).map_err(|e|format!("invalid cloud CLI instance reference: {e}")).and_then(|r|vsn_core::cloud_cli_status(principal,&r).map_err(|e|e.to_string()))),
        "cloud.cli.start"=>respond(serde_json::from_value::<vsn_cloud::CloudCliInstanceRef>(params.clone()).map_err(|e|format!("invalid cloud CLI instance reference: {e}")).and_then(|r|vsn_core::cloud_cli_start(principal,&r).map_err(|e|e.to_string()))),
        "cloud.cli.stop"=>respond(serde_json::from_value::<vsn_cloud::CloudCliInstanceRef>(params.clone()).map_err(|e|format!("invalid cloud CLI instance reference: {e}")).and_then(|r|vsn_core::cloud_cli_stop(principal,&r).map_err(|e|e.to_string()))),
        "cloud.cli.snapshot"=>respond(serde_json::from_value::<vsn_cloud::CloudCliSnapshotRequest>(params.clone()).map_err(|e|format!("invalid cloud CLI snapshot request: {e}")).and_then(|r|vsn_core::cloud_cli_snapshot(principal,&r).map_err(|e|e.to_string()))),
        "cloud.cli.clone"=>respond(serde_json::from_value::<vsn_cloud::CloudCliCloneRequest>(params.clone()).map_err(|e|format!("invalid cloud CLI clone request: {e}")).and_then(|r|vsn_core::cloud_cli_clone(principal,&r).map_err(|e|e.to_string()))),
        "cloud.cli.copy-image"=>respond(serde_json::from_value::<vsn_cloud::CloudCliImageCopyRequest>(params.clone()).map_err(|e|format!("invalid cloud CLI image-copy request: {e}")).and_then(|r|vsn_core::cloud_cli_copy_image(principal,&r).map_err(|e|e.to_string()))),
        "cloud.cli.copy-status"=>respond(serde_json::from_value::<vsn_cloud::CloudCliArtifactRef>(params.clone()).map_err(|e|format!("invalid cloud CLI artifact reference: {e}")).and_then(|r|vsn_core::cloud_cli_copy_status(principal,&r).map_err(|e|e.to_string()))),
        "cloud.cli.destroy"=>respond(serde_json::from_value::<vsn_cloud::CloudCliDestroyRequest>(params.clone()).map_err(|e|format!("invalid cloud CLI destroy request: {e}")).and_then(|r|vsn_core::cloud_cli_destroy(principal,&r).map_err(|e|e.to_string()))),
        "terminal.exec"=>respond(parse_terminal_request(params).and_then(|r|vsn_core::terminal_execute(principal,&r).map_err(|e|e.to_string()))),
        "terminal.session.start"=>respond(parse_terminal_session_start(params).and_then(|r|vsn_core::terminal_session_start(principal,&r).map_err(|e|e.to_string()))),
        "terminal.session.write"=>match (param_str(params,"session_id"),param_str_allow_empty(params,"input")){(Ok(id),Ok(input))=>respond(vsn_core::terminal_session_write(principal,id,input)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "terminal.session.read"=>match param_str(params,"session_id"){Ok(id)=>respond(vsn_core::terminal_session_read(principal,id,params.get("max_bytes").and_then(Value::as_u64).and_then(|v|usize::try_from(v).ok()).unwrap_or(65536))),Err(e)=>(false,json!({"error":e}))},
        "terminal.session.read-wait"=>match param_str(params,"session_id"){Ok(id)=>respond(vsn_core::terminal_session_read_wait(principal,id,params.get("max_bytes").and_then(Value::as_u64).and_then(|v|usize::try_from(v).ok()).unwrap_or(65536),params.get("wait_ms").and_then(Value::as_u64).unwrap_or(3000))),Err(e)=>(false,json!({"error":e}))},
        "terminal.session.status"=>match param_str(params,"session_id"){Ok(id)=>respond(vsn_core::terminal_session_status(principal,id)),Err(e)=>(false,json!({"error":e}))},
        "terminal.session.stop"=>match param_str(params,"session_id"){Ok(id)=>respond(vsn_core::terminal_session_stop(principal,id)),Err(e)=>(false,json!({"error":e}))},
        "terminal.session.remove"=>match param_str(params,"session_id"){Ok(id)=>respond(vsn_core::terminal_session_remove(principal,id)),Err(e)=>(false,json!({"error":e}))},
        "terminal.session.list"=>respond(vsn_core::terminal_session_list(principal)),
        "terminal.pty.start"=>respond(parse_pty_session_start(params).and_then(|r|vsn_core::terminal_pty_start(principal,&r).map_err(|e|e.to_string()))),
        "terminal.pty.write"=>match (param_str(params,"session_id"),param_str_allow_empty(params,"input")){(Ok(id),Ok(input))=>respond(vsn_core::terminal_pty_write(principal,id,input)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "terminal.pty.read"=>match param_str(params,"session_id"){Ok(id)=>respond(vsn_core::terminal_pty_read(principal,id,params.get("max_bytes").and_then(Value::as_u64).and_then(|v|usize::try_from(v).ok()).unwrap_or(65536))),Err(e)=>(false,json!({"error":e}))},
        "terminal.pty.read-wait"=>match param_str(params,"session_id"){Ok(id)=>respond(vsn_core::terminal_pty_read_wait(principal,id,params.get("max_bytes").and_then(Value::as_u64).and_then(|v|usize::try_from(v).ok()).unwrap_or(65536),params.get("wait_ms").and_then(Value::as_u64).unwrap_or(3000))),Err(e)=>(false,json!({"error":e}))},
        "terminal.pty.resize"=>match (param_str(params,"session_id"),param_u16(params,"rows"),param_u16(params,"cols")){(Ok(id),Ok(rows),Ok(cols))=>respond(vsn_core::terminal_pty_resize(principal,id,rows,cols)),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "terminal.pty.status"=>match param_str(params,"session_id"){Ok(id)=>respond(vsn_core::terminal_pty_status(principal,id)),Err(e)=>(false,json!({"error":e}))},
        "terminal.pty.stop"=>match param_str(params,"session_id"){Ok(id)=>respond(vsn_core::terminal_pty_stop(principal,id)),Err(e)=>(false,json!({"error":e}))},
        "terminal.pty.remove"=>match param_str(params,"session_id"){Ok(id)=>respond(vsn_core::terminal_pty_remove(principal,id)),Err(e)=>(false,json!({"error":e}))},
        "terminal.pty.list"=>respond(vsn_core::terminal_pty_list(principal)),
        "terminal.pty.scrollback.list"=>respond(vsn_core::terminal_pty_scrollback_list(principal)),
        "terminal.pty.scrollback.read"=>match (param_str(params,"session_id"),param_u64(params,"offset"),param_u64(params,"max_bytes")){(Ok(id),Ok(offset),Ok(max))=>respond(vsn_core::terminal_pty_scrollback_read(principal,id,offset,usize::try_from(max).unwrap_or(256*1024))),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "terminal.pty.scrollback.remove"=>match param_str(params,"session_id"){Ok(id)=>respond(vsn_core::terminal_pty_scrollback_remove(principal,id)),Err(e)=>(false,json!({"error":e}))},
        "terminal.conformance"=>respond(vsn_core::terminal_conformance(principal)),
        "terminal.pty.recovery.list"=>respond(vsn_core::terminal_pty_recovery_list(principal)),
        "terminal.pty.recovery.remove"=>match param_str(params,"session_id"){Ok(id)=>respond(vsn_core::terminal_pty_recovery_remove(principal,id)),Err(e)=>(false,json!({"error":e}))},
        "preview.fetch"=>respond(parse_preview_request(params).and_then(|r|vsn_core::preview_fetch(principal,&r).map_err(|e|e.to_string()))),
        "preview.conformance"=>respond(vsn_core::preview_conformance(principal)),
        "preview.request"=>respond(parse_preview_http_request(params).and_then(|r|vsn_core::preview_request(principal,&r).map_err(|e|e.to_string()))),
        "domain.plan"=>match (param_str(params,"domain"),param_u16(params,"port")){(Ok(domain),Ok(port))=>respond(vsn_core::domain_plan(principal,domain,port,params.get("tls").and_then(Value::as_bool).unwrap_or(true))),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "domain.apply-hosts"=>match param_str(params,"domain"){Ok(d)=>respond(vsn_core::domain_apply_hosts(principal,d)),Err(e)=>(false,json!({"error":e}))},
        "domain.remove-hosts"=>match param_str(params,"domain"){Ok(d)=>respond(vsn_core::domain_remove_hosts(principal,d)),Err(e)=>(false,json!({"error":e}))},
        "network.ca-install"=>respond(vsn_core::local_ca_install(principal)),"network.caddy-start"=>respond(vsn_core::caddy_start(principal)),
        "network.proxy-config"=>match (param_str(params,"domain"),param_u16(params,"port")){(Ok(d),Ok(p))=>respond(vsn_core::caddy_proxy_config(principal,d,p)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "network.dns-plan"=>respond(vsn_core::dns_plan(principal,params.get("listen").and_then(Value::as_str).unwrap_or("127.0.0.1:53535"))),
        "network.dns-start"=>respond(vsn_core::dns_start(principal,params.get("listen").and_then(Value::as_str).unwrap_or("127.0.0.1:53535"))),
        "network.dns-status"=>respond(vsn_core::dns_status(principal)),"network.dns-stop"=>respond(vsn_core::dns_stop(principal)),
        "database.remote.conformance"=>respond(vsn_core::remote_database_conformance(principal)),
        "database.workspace"=>match param_str(params,"model").and_then(parse_data_model){Ok(model)=>respond(vsn_core::database_workspace(principal,model)),Err(e)=>(false,json!({"error":e}))},
        "database.studio.conformance"=>respond(vsn_core::database_studio_conformance(principal)),
        "database.ui-demo"=>{let entity=demo_entity();let mut caps=vsn_database::CapabilitySet::default();caps.query=true;caps.indexes=true;caps.relations=true;caps.statistics=true;respond(vsn_core::database_ui_schema(principal,&entity,&caps))},
        "database.model.analyze"=>respond(serde_json::from_value::<vsn_database::AdvancedModelRequest>(params.clone()).map_err(|e|format!("invalid advanced model analysis request: {e}")).and_then(|r|vsn_core::database_model_analyze(principal,&r).map_err(|e|e.to_string()))),
        "database.sqlite.inspect"=>match param_str(params,"path"){Ok(p)=>respond(vsn_core::sqlite_inspect(principal,Path::new(p))),Err(e)=>(false,json!({"error":e}))},
        "database.sqlite.query"=>match (param_str(params,"path"),param_str(params,"sql")){(Ok(p),Ok(sql))=>respond(vsn_core::sqlite_query(principal,Path::new(p),sql)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "database.sqlite.browse"=>match (param_str(params,"path"),param_str(params,"entity"),parse_browse_request(params)){(Ok(p),Ok(e),Ok(r))=>respond(vsn_core::sqlite_browse(principal,Path::new(p),e,&r)),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.sqlite.indexes"=>match (param_str(params,"path"),param_str(params,"entity")){(Ok(p),Ok(e))=>respond(vsn_core::sqlite_indexes(principal,Path::new(p),e)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "database.sqlite.relations"=>match (param_str(params,"path"),param_str(params,"entity")){(Ok(p),Ok(e))=>respond(vsn_core::sqlite_relations(principal,Path::new(p),e)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "database.sqlite.stats"=>match (param_str(params,"path"),param_str(params,"entity")){(Ok(p),Ok(e))=>respond(vsn_core::sqlite_statistics(principal,Path::new(p),e)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "database.sqlite.insert"=>match (param_str(params,"path"),param_str(params,"entity"),parse_mutation_request(params)){(Ok(p),Ok(e),Ok(r))=>respond(vsn_core::sqlite_insert(principal,Path::new(p),e,&r)),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.sqlite.update"=>match (param_str(params,"path"),param_str(params,"entity"),parse_mutation_request(params)){(Ok(p),Ok(e),Ok(r))=>respond(vsn_core::sqlite_update(principal,Path::new(p),e,&r)),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.sqlite.delete"=>match (param_str(params,"path"),param_str(params,"entity"),parse_mutation_request(params)){(Ok(p),Ok(e),Ok(r))=>respond(vsn_core::sqlite_delete(principal,Path::new(p),e,&r)),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.cli.detect"=>respond(vsn_core::database_cli_detect(principal)),
        "database.cli.inspect"=>respond(parse_db_connection(params).and_then(|c|vsn_core::database_cli_inspect(principal,&c).map_err(|e|e.to_string()))),
        "database.cli.query"=>match (parse_db_connection(params),param_str(params,"sql")){(Ok(c),Ok(sql))=>respond(vsn_core::database_cli_query(principal,&c,sql)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "database.cli.job.start"=>match (parse_db_connection(params),param_str(params,"sql")){(Ok(c),Ok(sql))=>respond(vsn_core::database_cli_job_start(principal,&c,sql)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "database.cli.job.status"=>match param_str(params,"job_id"){Ok(id)=>respond(vsn_core::database_cli_job_status(principal,id)),Err(e)=>(false,json!({"error":e}))},
        "database.cli.job.list"=>respond(vsn_core::database_cli_jobs(principal)),
        "database.cli.job.cancel"=>match param_str(params,"job_id"){Ok(id)=>respond(vsn_core::database_cli_job_cancel(principal,id)),Err(e)=>(false,json!({"error":e}))},
        "database.cli.job.output"=>match (param_str(params,"job_id"),param_u64(params,"offset"),param_u64(params,"max_bytes")){(Ok(id),Ok(offset),Ok(max))=>respond(vsn_core::database_cli_job_output(principal,id,offset,usize::try_from(max).unwrap_or(256*1024))),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.cli.job.output-remove"=>match param_str(params,"job_id"){Ok(id)=>respond(vsn_core::database_cli_job_output_remove(principal,id)),Err(e)=>(false,json!({"error":e}))},
        "database.native.postgres.inspect"=>respond(parse_postgres_native(params).and_then(|c|vsn_core::postgres_native_inspect(principal,&c).map_err(|e|e.to_string()))),
        "database.native.postgres.browse"=>match (parse_postgres_native(params),param_str(params,"schema"),param_str(params,"table")){(Ok(c),Ok(schema),Ok(table))=>respond(vsn_core::postgres_native_browse(principal,&c,schema,table,params.get("limit").and_then(Value::as_u64).and_then(|v|u32::try_from(v).ok()).unwrap_or(100),params.get("offset").and_then(Value::as_u64).unwrap_or(0))),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.postgres.query"=>match (parse_postgres_native(params),param_str(params,"sql")){(Ok(c),Ok(sql))=>respond(vsn_core::postgres_native_query(principal,&c,sql)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "database.native.postgres.job.start"=>match (parse_postgres_native(params),param_str(params,"sql")){(Ok(c),Ok(sql))=>respond(vsn_core::postgres_native_job_start(principal,&c,sql)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "database.native.postgres.job.status"=>match param_str(params,"job_id"){Ok(id)=>respond(vsn_core::postgres_native_job_status(principal,id)),Err(e)=>(false,json!({"error":e}))},
        "database.native.postgres.job.list"=>respond(vsn_core::postgres_native_job_list(principal)),
        "database.native.postgres.job.cancel"=>match param_str(params,"job_id"){Ok(id)=>respond(vsn_core::postgres_native_job_cancel(principal,id)),Err(e)=>(false,json!({"error":e}))},
        "database.native.postgres.txn.start"=>match parse_postgres_native(params){Ok(c)=>respond(vsn_core::postgres_native_txn_start(principal,&c,params.get("ttl_seconds").and_then(Value::as_u64).unwrap_or(30))),Err(e)=>(false,json!({"error":e}))},
        "database.native.postgres.txn.query"=>match (param_str(params,"transaction_id"),param_str(params,"sql")){(Ok(id),Ok(sql))=>respond(vsn_core::postgres_native_txn_query(principal,id,sql)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "database.native.postgres.txn.status"=>match param_str(params,"transaction_id"){Ok(id)=>respond(vsn_core::postgres_native_txn_status(principal,id)),Err(e)=>(false,json!({"error":e}))},
        "database.native.postgres.txn.close"=>match param_str(params,"transaction_id"){Ok(id)=>respond(vsn_core::postgres_native_txn_close(principal,id,params.get("commit").and_then(Value::as_bool).unwrap_or(false))),Err(e)=>(false,json!({"error":e}))},
        "database.tls.postgres.inspect"=>respond(parse_postgres_tls(params).and_then(|c|vsn_core::postgres_tls_inspect(principal,&c).map_err(|e|e.to_string()))),
        "database.tls.postgres.browse"=>match (parse_postgres_tls(params),param_str(params,"schema"),param_str(params,"table")){(Ok(c),Ok(schema),Ok(table))=>respond(vsn_core::postgres_tls_browse(principal,&c,schema,table,params.get("limit").and_then(Value::as_u64).and_then(|v|u32::try_from(v).ok()).unwrap_or(100),params.get("offset").and_then(Value::as_u64).unwrap_or(0))),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.tls.postgres.query"=>match (parse_postgres_tls(params),param_str(params,"sql")){(Ok(c),Ok(sql))=>respond(vsn_core::postgres_tls_query(principal,&c,sql)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "database.native.postgres.indexes"=>match (parse_postgres_native(params),param_str(params,"schema"),param_str(params,"table")){(Ok(c),Ok(schema),Ok(table))=>respond(vsn_core::postgres_native_indexes(principal,&c,schema,table)),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.postgres.relations"=>match (parse_postgres_native(params),param_str(params,"schema"),param_str(params,"table")){(Ok(c),Ok(schema),Ok(table))=>respond(vsn_core::postgres_native_relations(principal,&c,schema,table)),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.postgres.stats"=>match (parse_postgres_native(params),param_str(params,"schema"),param_str(params,"table")){(Ok(c),Ok(schema),Ok(table))=>respond(vsn_core::postgres_native_stats(principal,&c,schema,table)),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.postgres.insert"=>match (parse_postgres_native(params),param_str(params,"schema"),param_str(params,"table"),parse_mutation_request(params)){(Ok(c),Ok(schema),Ok(table),Ok(r))=>respond(vsn_core::postgres_native_insert(principal,&c,schema,table,&r)),(Err(e),_,_,_)|(_,Err(e),_,_)|(_,_,Err(e),_)|(_,_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.postgres.update"=>match (parse_postgres_native(params),param_str(params,"schema"),param_str(params,"table"),parse_mutation_request(params)){(Ok(c),Ok(schema),Ok(table),Ok(r))=>respond(vsn_core::postgres_native_update(principal,&c,schema,table,&r)),(Err(e),_,_,_)|(_,Err(e),_,_)|(_,_,Err(e),_)|(_,_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.postgres.delete"=>match (parse_postgres_native(params),param_str(params,"schema"),param_str(params,"table"),parse_mutation_request(params)){(Ok(c),Ok(schema),Ok(table),Ok(r))=>respond(vsn_core::postgres_native_delete(principal,&c,schema,table,&r)),(Err(e),_,_,_)|(_,Err(e),_,_)|(_,_,Err(e),_)|(_,_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.mysql.inspect"=>respond(parse_mysql_native(params).and_then(|c|vsn_core::mysql_native_inspect(principal,&c).map_err(|e|e.to_string()))),
        "database.native.mysql.browse"=>match (parse_mysql_native(params),param_str(params,"database"),param_str(params,"table")){(Ok(c),Ok(database),Ok(table))=>respond(vsn_core::mysql_native_browse(principal,&c,database,table,params.get("limit").and_then(Value::as_u64).and_then(|v|u32::try_from(v).ok()).unwrap_or(100),params.get("offset").and_then(Value::as_u64).unwrap_or(0))),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.mysql.query"=>match (parse_mysql_native(params),param_str(params,"sql")){(Ok(c),Ok(sql))=>respond(vsn_core::mysql_native_query(principal,&c,sql)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "database.tls.mysql.inspect"=>respond(parse_mysql_tls(params).and_then(|c|vsn_core::mysql_tls_inspect(principal,&c).map_err(|e|e.to_string()))),
        "database.tls.mysql.browse"=>match (parse_mysql_tls(params),param_str(params,"database"),param_str(params,"table")){(Ok(c),Ok(database),Ok(table))=>respond(vsn_core::mysql_tls_browse(principal,&c,database,table,params.get("limit").and_then(Value::as_u64).and_then(|v|u32::try_from(v).ok()).unwrap_or(100),params.get("offset").and_then(Value::as_u64).unwrap_or(0))),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.tls.mysql.query"=>match (parse_mysql_tls(params),param_str(params,"sql")){(Ok(c),Ok(sql))=>respond(vsn_core::mysql_tls_query(principal,&c,sql)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "database.native.mysql.indexes"=>match (parse_mysql_native(params),param_str(params,"database"),param_str(params,"table")){(Ok(c),Ok(database),Ok(table))=>respond(vsn_core::mysql_native_indexes(principal,&c,database,table)),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.mysql.relations"=>match (parse_mysql_native(params),param_str(params,"database"),param_str(params,"table")){(Ok(c),Ok(database),Ok(table))=>respond(vsn_core::mysql_native_relations(principal,&c,database,table)),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.mysql.stats"=>match (parse_mysql_native(params),param_str(params,"database"),param_str(params,"table")){(Ok(c),Ok(database),Ok(table))=>respond(vsn_core::mysql_native_stats(principal,&c,database,table)),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.mysql.insert"=>match (parse_mysql_native(params),param_str(params,"database"),param_str(params,"table"),parse_mutation_request(params)){(Ok(c),Ok(database),Ok(table),Ok(r))=>respond(vsn_core::mysql_native_insert(principal,&c,database,table,&r)),(Err(e),_,_,_)|(_,Err(e),_,_)|(_,_,Err(e),_)|(_,_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.mysql.update"=>match (parse_mysql_native(params),param_str(params,"database"),param_str(params,"table"),parse_mutation_request(params)){(Ok(c),Ok(database),Ok(table),Ok(r))=>respond(vsn_core::mysql_native_update(principal,&c,database,table,&r)),(Err(e),_,_,_)|(_,Err(e),_,_)|(_,_,Err(e),_)|(_,_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.mysql.delete"=>match (parse_mysql_native(params),param_str(params,"database"),param_str(params,"table"),parse_mutation_request(params)){(Ok(c),Ok(database),Ok(table),Ok(r))=>respond(vsn_core::mysql_native_delete(principal,&c,database,table,&r)),(Err(e),_,_,_)|(_,Err(e),_,_)|(_,_,Err(e),_)|(_,_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.mongo.inspect"=>respond(parse_mongo_native(params).and_then(|c|vsn_core::mongo_native_inspect(principal,&c,params.get("database").and_then(Value::as_str)).map_err(|e|e.to_string()))),
        "database.native.mongo.browse"=>match (parse_mongo_native(params),param_str(params,"database"),param_str(params,"collection")){(Ok(c),Ok(database),Ok(collection))=>respond(vsn_core::mongo_native_browse(principal,&c,database,collection,params.get("limit").and_then(Value::as_u64).and_then(|v|u32::try_from(v).ok()).unwrap_or(100),params.get("offset").and_then(Value::as_u64).unwrap_or(0),params.get("filter").cloned())),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.mongo.indexes"=>match (parse_mongo_native(params),param_str(params,"database"),param_str(params,"collection")){(Ok(c),Ok(database),Ok(collection))=>respond(vsn_core::mongo_native_indexes(principal,&c,database,collection)),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.mongo.stats"=>match (parse_mongo_native(params),param_str(params,"database"),param_str(params,"collection")){(Ok(c),Ok(database),Ok(collection))=>respond(vsn_core::mongo_native_stats(principal,&c,database,collection)),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.mongo.insert"=>match (parse_mongo_native(params),param_str(params,"database"),param_str(params,"collection"),parse_mutation_request(params)){(Ok(c),Ok(database),Ok(collection),Ok(r))=>respond(vsn_core::mongo_native_insert(principal,&c,database,collection,&r.values)),(Err(e),_,_,_)|(_,Err(e),_,_)|(_,_,Err(e),_)|(_,_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.mongo.update"=>match (parse_mongo_native(params),param_str(params,"database"),param_str(params,"collection"),parse_mutation_request(params)){(Ok(c),Ok(database),Ok(collection),Ok(r))=>respond(vsn_core::mongo_native_update(principal,&c,database,collection,&r)),(Err(e),_,_,_)|(_,Err(e),_,_)|(_,_,Err(e),_)|(_,_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.mongo.delete"=>match (parse_mongo_native(params),param_str(params,"database"),param_str(params,"collection"),parse_mutation_request(params)){(Ok(c),Ok(database),Ok(collection),Ok(r))=>respond(vsn_core::mongo_native_delete(principal,&c,database,collection,&r.filter)),(Err(e),_,_,_)|(_,Err(e),_,_)|(_,_,Err(e),_)|(_,_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.redis.inspect"=>respond(parse_redis_native(params).and_then(|c|vsn_core::redis_native_inspect(principal,&c).map_err(|e|e.to_string()))),
        "database.native.redis.get"=>match (parse_redis_native(params),param_str(params,"key")){(Ok(c),Ok(key))=>respond(vsn_core::redis_native_get(principal,&c,key)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "database.native.redis.set"=>match (parse_redis_native(params),param_str(params,"key"),param_str_allow_empty(params,"value")){(Ok(c),Ok(key),Ok(value))=>respond(vsn_core::redis_native_set(principal,&c,key,value,params.get("ttl_seconds").and_then(Value::as_u64))),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "database.native.redis.delete"=>match (parse_redis_native(params),param_str(params,"key")){(Ok(c),Ok(key))=>respond(vsn_core::redis_native_delete(principal,&c,key)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "stream.open"=>respond(serde_json::from_value::<vsn_stream::StreamOpenRequest>(params.clone()).map_err(|e|e.to_string()).and_then(|r|vsn_core::stream_open(principal,r).map_err(|e|e.to_string()))),
        "stream.input"=>match (param_str(params,"id"),params.get("seq").and_then(Value::as_u64),param_str_allow_empty(params,"payload_base64")){(Ok(id),Some(seq),Ok(payload))=>respond(vsn_core::stream_input(principal,id,seq,payload,params.get("eof").and_then(Value::as_bool).unwrap_or(false))),(Err(e),_,_)|(_,_,Err(e))=>(false,json!({"error":e})),(_,None,_)=>(false,json!({"error":"missing seq"}))},
        "stream.input.pull"=>match param_str(params,"id"){Ok(id)=>respond(vsn_core::stream_input_pull(principal,id,params.get("max_frames").and_then(Value::as_u64).and_then(|v|usize::try_from(v).ok()).unwrap_or(16))),Err(e)=>(false,json!({"error":e}))},
        "stream.output"=>match (param_str(params,"id"),param_str_allow_empty(params,"payload_base64")){(Ok(id),Ok(payload))=>{use base64::{engine::general_purpose::STANDARD as B64,Engine as _};match B64.decode(payload){Ok(bytes)=>respond(vsn_core::stream_output(principal,id,&bytes,params.get("eof").and_then(Value::as_bool).unwrap_or(false))),Err(_)=>(false,json!({"error":"invalid payload_base64"}))}},(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "stream.pull"=>match param_str(params,"id"){Ok(id)=>respond(vsn_core::stream_pull(principal,id,params.get("max_frames").and_then(Value::as_u64).unwrap_or(16) as usize)),Err(e)=>(false,json!({"error":e}))},
        "stream.close"=>match param_str(params,"id"){Ok(id)=>respond(vsn_core::stream_close(principal,id,params.get("reason").and_then(Value::as_str))),Err(e)=>(false,json!({"error":e}))},
        "stream.list"=>respond(vsn_core::stream_list(principal)),
        "vault.list"=>respond(vsn_core::vault_list(principal)),"vault.set"=>match (param_str(params,"name"),param_str(params,"value")){(Ok(n),Ok(v))=>respond(vsn_core::vault_set(principal,n,v)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "vault.delete"=>match param_str(params,"name"){Ok(n)=>respond(vsn_core::vault_delete(principal,n)),Err(e)=>(false,json!({"error":e}))},"vault.reveal"=>match param_str(params,"name"){Ok(n)=>respond(vsn_core::vault_reveal(principal,n)),Err(e)=>(false,json!({"error":e}))},
        "vault.status"=>respond(vsn_core::vault_status(principal)),"vault.rotate"=>respond(vsn_core::vault_rotate(principal)),
        "vault.key-history"=>respond(vsn_core::vault_key_history(principal)),
        "vault.restore"=>match (param_str(params,"key_id"),params.get("confirm").and_then(Value::as_bool)){(Ok(id),Some(confirm))=>respond(vsn_core::vault_restore(principal,id,confirm)),(Err(e),_)=>(false,json!({"error":e})),(_,None)=>(false,json!({"error":"confirm is required"}))},
        "vault.retire"=>match (param_str(params,"key_id"),params.get("confirm").and_then(Value::as_bool)){(Ok(id),Some(confirm))=>respond(vsn_core::vault_retire(principal,id,confirm)),(Err(e),_)=>(false,json!({"error":e})),(_,None)=>(false,json!({"error":"confirm is required"}))},
        "marketplace.verify"=>match (param_str(params,"index"),param_str(params,"trust")){(Ok(index),Ok(trust))=>respond(vsn_core::marketplace_verify(principal,Path::new(index),Path::new(trust))),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "marketplace.publishers"=>match (param_str(params,"index"),param_str(params,"trust")){(Ok(index),Ok(trust))=>respond(vsn_core::marketplace_publishers(principal,Path::new(index),Path::new(trust))),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "marketplace.conformance"=>respond(vsn_core::marketplace_conformance(principal)),
        "cloud.conformance"=>respond(vsn_core::cloud_conformance(principal)),
        "marketplace.search"=>match (param_str(params,"index"),param_str(params,"trust"),param_str(params,"query")){(Ok(index),Ok(trust),Ok(query))=>respond(vsn_core::marketplace_search(principal,Path::new(index),Path::new(trust),query)),(Err(e),_,_)|(_,Err(e),_)|(_,_,Err(e))=>(false,json!({"error":e}))},
        "marketplace.resolve-update"=>match (param_str(params,"index"),param_str(params,"trust"),param_str(params,"id"),param_str(params,"current_version")){(Ok(index),Ok(trust),Ok(id),Ok(version))=>respond(vsn_core::marketplace_resolve_update(principal,Path::new(index),Path::new(trust),id,version)),(Err(e),_,_,_)|(_,Err(e),_,_)|(_,_,Err(e),_)|(_,_,_,Err(e))=>(false,json!({"error":e}))},
        "marketplace.resolve-update-channel"=>match (param_str(params,"index"),param_str(params,"trust"),param_str(params,"id"),param_str(params,"current_version"),param_str(params,"channel")){(Ok(index),Ok(trust),Ok(id),Ok(version),Ok(channel))=>respond(vsn_core::marketplace_resolve_update_channel(principal,Path::new(index),Path::new(trust),id,version,channel)),(Err(e),_,_,_,_)|(_,Err(e),_,_,_)|(_,_,Err(e),_,_)|(_,_,_,Err(e),_)|(_,_,_,_,Err(e))=>(false,json!({"error":e}))},
        "extension.conformance"=>respond(vsn_core::extension_conformance(principal)),
        "extension.dependencies"=>match(param_str(params,"id"),param_str(params,"version")){(Ok(id),Ok(v))=>respond(vsn_core::extension_dependencies(principal,id,v)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "extension.list"=>respond(vsn_core::extension_list(principal)),
        "extension.verify"=>match (param_str(params,"package"),param_str(params,"trust")){(Ok(pkg),Ok(trust))=>respond(vsn_core::extension_verify(principal,Path::new(pkg),Path::new(trust))),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "extension.install"=>match (param_str(params,"package"),param_str(params,"trust")){(Ok(pkg),Ok(trust))=>respond(vsn_core::extension_install(principal,Path::new(pkg),Path::new(trust))),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "extension.uninstall"=>match (param_str(params,"id"),param_str(params,"version")){(Ok(id),Ok(v))=>respond(vsn_core::extension_uninstall(principal,id,v)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "extension.providers"=>match (param_str(params,"id"),param_str(params,"version")){(Ok(id),Ok(v))=>respond(vsn_core::extension_providers(principal,id,v,params.get("kind").and_then(Value::as_str))),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "extension.sandbox-capabilities"=>respond(vsn_core::extension_sandbox_capabilities(principal)),
        "extension.exec"=>respond(serde_json::from_value::<vsn_extension::SandboxExecRequest>(params.clone()).map_err(|e|format!("invalid extension exec request: {e}")).and_then(|r|vsn_core::extension_exec(principal,&r).map_err(|e|e.to_string()))),
        "update.verify-manifest"=>match (param_str(params,"path"),param_str(params,"public_key")){(Ok(p),Ok(k))=>respond(vsn_core::update_verify_manifest(principal,Path::new(p),k)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "update.verify-artifact"=>match (param_str(params,"path"),param_str(params,"sha256")){(Ok(p),Ok(h))=>respond(vsn_core::update_verify_artifact(principal,Path::new(p),h)),(Err(e),_)|(_,Err(e))=>(false,json!({"error":e}))},
        "update.apply-file"=>respond(serde_json::from_value::<vsn_update::ApplyFileRequest>(params.clone()).map_err(|e|format!("invalid update apply request: {e}")).and_then(|r|vsn_core::update_apply_file(principal,&r).map_err(|e|e.to_string()))),
        "update.rollback-file"=>match (param_str(params,"install_root"),params.get("confirm_rollback").and_then(Value::as_bool)){(Ok(root),Some(confirm))=>respond(vsn_core::update_rollback_file(principal,Path::new(root),confirm)),(Err(e),_)=>(false,json!({"error":e})),(_,None)=>(false,json!({"error":"confirm_rollback is required"}))},
        "update.status"=>match param_str(params,"install_root"){Ok(root)=>respond(vsn_core::update_status(principal,Path::new(root))),Err(e)=>(false,json!({"error":e}))},
        "update.recover-lock"=>match (param_str(params,"install_root"),params.get("confirm_recover").and_then(Value::as_bool)){(Ok(root),Some(confirm))=>respond(vsn_core::update_recover_lock(principal,Path::new(root),confirm)),(Err(e),_)=>(false,json!({"error":e})),(_,None)=>(false,json!({"error":"confirm_recover is required"}))},
        "remote.status"=>respond(vsn_core::remote_status(principal)),
        "remote.configure"=>respond(parse_remote_config(params).and_then(|c|vsn_core::remote_configure(principal,c).map_err(|e|e.to_string()))),
        "remote.enroll"=>match param_str(params,"pairing_nonce"){Ok(n)=>respond(vsn_core::remote_enroll(principal,n)),Err(e)=>(false,json!({"error":e}))},
        _=>(false,json!({"error":"unknown or unauthorized command"})),
    }
}

fn flush_remote_audit(client:&vsn_remote::HttpControlPlaneClient,device_id:&str,cursor_path:&Path)->Result<(),Box<dyn std::error::Error>>{
    let audit_path=vsn_audit::default_audit_path()?;
    let cursor=std::fs::read_to_string(cursor_path).ok().map(|v|v.trim().to_string()).filter(|v|!v.is_empty());
    let events=vsn_audit::read_events_after(&audit_path,cursor.as_deref(),128)?;
    if events.is_empty(){return Ok(());}
    let last=events.last().map(|e|e.event_id.clone()).unwrap_or_default();
    let batch=vsn_remote::AgentAuditBatchV1{version:vsn_remote::REMOTE_PROTOCOL_VERSION,device_id:device_id.into(),events};
    client.submit_audit(&batch)?;
    if let Some(parent)=cursor_path.parent(){std::fs::create_dir_all(parent)?;}
    let tmp=cursor_path.with_extension("tmp");std::fs::write(&tmp,format!("{last}\n"))?;if cursor_path.exists(){std::fs::remove_file(cursor_path)?;}std::fs::rename(tmp,cursor_path)?;Ok(())
}

fn flush_pending_remote_results(identity:&vsn_security::DeviceIdentity,client:&vsn_remote::HttpControlPlaneClient,path:&Path)->Result<(),Box<dyn std::error::Error>>{
    for cached in vsn_remote::list_cached_agent_results(path)?{
        let refreshed=vsn_remote::refresh_agent_result(identity,&cached)?;
        vsn_remote::store_cached_agent_result(path,&refreshed)?;
        client.submit_result(&refreshed)?;
        vsn_remote::remove_cached_agent_result(path,&refreshed.command_id)?;
    }
    Ok(())
}


fn gateway_poll(gateway:&mut Option<vsn_remote::WebSocketControlPlaneClient>,http:&vsn_remote::HttpControlPlaneClient,url:&str,poll:&vsn_remote::AgentPollV1)->Result<vsn_remote::AgentPollResponseV1,Box<dyn std::error::Error>>{
    if gateway.is_none(){if let Ok(client)=vsn_remote::WebSocketControlPlaneClient::new(url){*gateway=Some(client);}}
    if let Some(client)=gateway.as_mut(){match client.poll(poll){Ok(v)=>return Ok(v),Err(e)=>{eprintln!("remote_gateway_poll_fallback={e}");*gateway=None;}}}
    Ok(http.poll(poll)?)
}
fn gateway_submit_result(gateway:&mut Option<vsn_remote::WebSocketControlPlaneClient>,http:&vsn_remote::HttpControlPlaneClient,result:&vsn_remote::AgentCommandResultV1)->Result<(),Box<dyn std::error::Error>>{
    if let Some(client)=gateway.as_mut(){match client.submit_result(result){Ok(())=>return Ok(()),Err(e)=>{eprintln!("remote_gateway_result_fallback={e}");*gateway=None;}}}
    http.submit_result(result)?;Ok(())
}

fn remote_loop(stop:Arc<AtomicBool>){
    let config=match vsn_core::config(){Ok(c)=>c.remote,Err(e)=>{eprintln!("remote_config_error={e}");return;}};if !config.enabled{return;}
    let Some(url)=config.control_plane_url else{eprintln!("remote_disabled_reason=missing_url");return;};let Some(public_key)=config.control_plane_public_key else{eprintln!("remote_disabled_reason=missing_public_key");return;};
    let identity=match vsn_core::device_identity(){Ok(v)=>v,Err(e)=>{eprintln!("remote_identity_error={e}");return;}};let verifier=vsn_remote::RemoteCommandVerifier::new(public_key,identity.metadata().device_id.clone());let client=match vsn_remote::HttpControlPlaneClient::new(&url){Ok(v)=>v,Err(e)=>{eprintln!("remote_client_error={e}");return;}};let mut gateway=match vsn_remote::WebSocketControlPlaneClient::new(&url){Ok(v)=>{eprintln!("remote_transport=websocket");Some(v)},Err(e)=>{eprintln!("remote_transport=https_polling fallback_reason={e}");None}};
    let remote_data=match vsn_security::data_dir(){Ok(p)=>p.join("remote"),Err(e)=>{eprintln!("remote_disabled_reason=result_cache_unavailable:{e}");return;}};let result_cache=remote_data.join("result-cache.json");let audit_cursor=remote_data.join("audit-cursor.txt");
    while !stop.load(Ordering::SeqCst){
        let cycle=(||->Result<(),Box<dyn std::error::Error>>{
            flush_pending_remote_results(&identity,&client,&result_cache)?;
            flush_remote_audit(&client,&identity.metadata().device_id,&audit_cursor)?;
            let poll=vsn_remote::build_agent_poll(&identity)?;
            let response=gateway_poll(&mut gateway,&client,&url,&poll)?;
            if let Some(command)=response.command{
                let verification=verifier.verify_delivery(&command)?;
                if let Some(cached)=vsn_remote::load_cached_agent_result(&result_cache,&command.command_id)?{
                    let refreshed=vsn_remote::refresh_agent_result(&identity,&cached)?;
                    vsn_remote::store_cached_agent_result(&result_cache,&refreshed)?;
                    gateway_submit_result(&mut gateway,&client,&refreshed)?;
                    vsn_remote::remove_cached_agent_result(&result_cache,&command.command_id)?;
                    return Ok(());
                }
                if matches!(verification,vsn_remote::DeliveryVerification::Duplicate){return Err("duplicate remote command has no durable execution record".into());}
                let permission=vsn_policy::Permission::from_str(&command.permission).ok_or_else(||std::io::Error::new(std::io::ErrorKind::PermissionDenied,"unknown delegated permission"))?;
                let principal=vsn_policy::Principal::remote_delegated(command.principal_id.clone(),permission)?;
                // Persist a fail-closed execution record before invoking anything with side effects. If the
                // Agent crashes after this point, restart sends this record instead of re-running the command.
                let interrupted=vsn_remote::build_agent_result(&identity,&command,false,json!({"error":"remote execution was interrupted before a final result was durably persisted; command was not retried automatically"}))?;
                vsn_remote::store_cached_agent_result(&result_cache,&interrupted)?;
                let (mut ok,mut payload)=dispatch_command(&principal,&command.command,&command.params);
                if serde_json::to_vec(&payload).map(|v|v.len()).unwrap_or(usize::MAX)>1_500_000{ok=false;payload=json!({"error":"remote result payload exceeds 1.5 MiB safety limit; use a narrower operation"});}
                let mut remote_meta=BTreeMap::new();remote_meta.insert("command_id".into(),command.command_id.clone());remote_meta.insert("session_id".into(),command.session_id.clone());remote_meta.insert("permission".into(),command.permission.clone());
                let _=audit("remote-principal",&command.principal_id,&format!("remote.{}",command.command),&command.device_id,if ok{"success"}else{"denied_or_failed"},remote_meta);
                let result=vsn_remote::build_agent_result(&identity,&command,ok,payload)?;
                vsn_remote::store_cached_agent_result(&result_cache,&result)?;
                gateway_submit_result(&mut gateway,&client,&result)?;
                vsn_remote::remove_cached_agent_result(&result_cache,&command.command_id)?;
            }
            Ok(())
        })();
        if let Err(e)=cycle{eprintln!("remote_cycle_error={e}");}
        let delay=config.poll_interval_ms.clamp(500,60_000);let mut waited=0;while waited<delay&&!stop.load(Ordering::SeqCst){let slice=(delay-waited).min(250);thread::sleep(Duration::from_millis(slice));waited+=slice;}
    }
}


#[derive(Clone)]
struct ActiveStreamRelay{principal:vsn_policy::Principal,request:vsn_remote::RelayStreamOpenV1,stream_id:String,file_offset:u64,preview_stream_id:Option<String>,preview_websocket_id:Option<String>,preview_http_input:Vec<u8>}

fn stream_relay_loop(stop:Arc<AtomicBool>){
    let cfg=match vsn_core::config(){Ok(v)=>v.remote,Err(_)=>return};if !cfg.enabled{return;}let Some(url)=cfg.control_plane_url else{return};let Some(control_key)=cfg.control_plane_public_key else{return};let identity=match vsn_core::device_identity(){Ok(v)=>v,Err(e)=>{eprintln!("stream_relay_identity_error={e}");return;}};let verifier=vsn_remote::RemoteCommandVerifier::new(control_key,identity.metadata().device_id.clone());
    while !stop.load(Ordering::SeqCst){
        let mut client=match vsn_remote::AgentStreamRelayClient::connect(&url,&identity){Ok(v)=>{eprintln!("remote_stream_transport=websocket");v},Err(e)=>{eprintln!("remote_stream_connect_error={e}");thread::sleep(Duration::from_secs(2));continue;}};let mut relays:HashMap<String,ActiveStreamRelay>=HashMap::new();
        loop{if stop.load(Ordering::SeqCst){return;}let message=match client.read_server(){Ok(v)=>v,Err(e)=>{eprintln!("remote_stream_read_error={e}");break;}};if let Err(e)=handle_stream_relay_message(&cfg,&verifier,&mut client,&mut relays,message){eprintln!("remote_stream_message_error={e}");}}
        thread::sleep(Duration::from_millis(500));
    }
}

fn handle_stream_relay_message(cfg:&vsn_config::RemoteConfig,verifier:&vsn_remote::RemoteCommandVerifier,client:&mut vsn_remote::AgentStreamRelayClient,relays:&mut HashMap<String,ActiveStreamRelay>,message:vsn_remote::AgentStreamServerMessageV1)->Result<(),Box<dyn std::error::Error>>{
    match message{
        vsn_remote::AgentStreamServerMessageV1::Open{relay_id,authorization,request}=>{
            let result=(||->Result<ActiveStreamRelay,String>{
                verifier.verify(&authorization).map_err(|e|e.to_string())?;if authorization.command!="stream.relay.open"||authorization.session_id!=relay_id{return Err("invalid stream authorization envelope".into());}if authorization.params!=serde_json::to_value(&request).map_err(|e|e.to_string())?{return Err("stream authorization/request mismatch".into());}
                let expected=agent_stream_permission(&request.kind,&request.direction).ok_or_else(||"unsupported remote stream kind/direction".to_string())?;if authorization.permission!=expected{return Err("stream permission mismatch".into());}
                vsn_stream::validate_open_request(&vsn_stream::StreamOpenRequest{kind:request.kind.clone(),direction:request.direction.clone(),resource_id:request.resource_id.clone(),metadata:request.metadata.clone()}).map_err(|e|e.to_string())?;
                if matches!(request.kind,vsn_stream::StreamKind::Terminal)&&!cfg.allow_remote_terminal{return Err("remote terminal streaming is disabled locally".into());}
                if matches!(request.kind,vsn_stream::StreamKind::FileUpload)&&!cfg.allow_remote_file_write{return Err("remote file upload is disabled locally".into());}
                if matches!(request.kind,vsn_stream::StreamKind::Database)&&!cfg.allow_remote_database_query{return Err("remote database streaming is disabled locally".into());}if matches!(request.kind,vsn_stream::StreamKind::Preview)&&matches!(request.direction,vsn_stream::StreamDirection::Bidirectional)&&!cfg.allow_remote_preview_interactive{return Err("interactive remote preview WebSocket is disabled locally".into());}
                let permissions=agent_stream_permissions(&request.kind,&request.direction);let principal=vsn_policy::Principal::remote_stream(authorization.principal_id.clone(),permissions).map_err(|e|e.to_string())?;
                let mut effective=request.clone();
                if matches!(effective.kind,vsn_stream::StreamKind::Terminal){
                    if effective.resource_id=="new"{
                        let program=effective.metadata.get("program").cloned().ok_or_else(||"new terminal stream requires program metadata".to_string())?;
                        let cwd=effective.metadata.get("cwd").cloned().ok_or_else(||"new terminal stream requires cwd metadata".to_string())?;
                        let args:Vec<String>=serde_json::from_str(effective.metadata.get("args_json").map(String::as_str).unwrap_or("[]")).map_err(|e|format!("invalid terminal args_json: {e}"))?;
                        let env:std::collections::BTreeMap<String,String>=serde_json::from_str(effective.metadata.get("env_json").map(String::as_str).unwrap_or("{}" )).map_err(|e|format!("invalid terminal env_json: {e}"))?;
                        let rows=effective.metadata.get("rows").and_then(|v|v.parse::<u16>().ok()).unwrap_or(24);let cols=effective.metadata.get("cols").and_then(|v|v.parse::<u16>().ok()).unwrap_or(80);
                        let created=vsn_core::terminal_pty_start(&principal,&vsn_terminal::PtySessionStartRequest{program,args,cwd:Path::new(&cwd).to_path_buf(),env,rows,cols}).map_err(|e|format!("PTY create failed: {e}"))?;
                        effective.resource_id=created.session_id;
                    }else{vsn_core::terminal_pty_status(&principal,&effective.resource_id).map_err(|e|format!("PTY session unavailable: {e}"))?;}
                }
                let resume_in=effective.metadata.get("vsn_resume_input_seq").and_then(|v|v.parse::<u64>().ok()).unwrap_or(0);
                let resume_out=effective.metadata.get("vsn_resume_output_seq").and_then(|v|v.parse::<u64>().ok()).unwrap_or(0);
                if matches!(effective.kind,vsn_stream::StreamKind::Terminal)&&(resume_in>0||resume_out>0){return Err("terminal stream cannot be auto-reconstructed after Agent reconnect".into());}
                let state=vsn_core::stream_open_resumed(&principal,vsn_stream::StreamOpenRequest{kind:effective.kind.clone(),direction:effective.direction.clone(),resource_id:effective.resource_id.clone(),metadata:effective.metadata.clone()},resume_in,resume_out).map_err(|e|e.to_string())?;
                let file_offset=effective.metadata.get("offset").and_then(|v|v.parse::<u64>().ok()).unwrap_or(0);
                let preview_stream_id=if matches!(effective.kind,vsn_stream::StreamKind::Preview)&&effective.metadata.get("mode").map(String::as_str)==Some("sse"){let port=effective.metadata.get("port").ok_or_else(||"preview SSE stream requires port metadata".to_string())?.parse::<u16>().map_err(|_|"preview SSE port is invalid".to_string())?;let max_duration_seconds=effective.metadata.get("max_duration_seconds").and_then(|v|v.parse::<u64>().ok()).unwrap_or(60);let last_event_id=effective.metadata.get("last_event_id").cloned();let opened=vsn_core::preview_event_stream_start(&principal,&vsn_preview::PreviewEventStreamRequest{port,path:effective.resource_id.clone(),last_event_id,max_duration_seconds}).map_err(|e|e.to_string())?;Some(opened.stream_id)}else{None};
                let preview_websocket_id=if matches!(effective.kind,vsn_stream::StreamKind::Preview)&&effective.metadata.get("mode").map(String::as_str)==Some("websocket"){if !matches!(effective.direction,vsn_stream::StreamDirection::Bidirectional){return Err("preview WebSocket relay must be bidirectional".into());}let port=effective.metadata.get("port").ok_or_else(||"preview WebSocket requires port metadata".to_string())?.parse::<u16>().map_err(|_|"preview WebSocket port is invalid".to_string())?;let max_duration_seconds=effective.metadata.get("max_duration_seconds").and_then(|v|v.parse::<u64>().ok()).unwrap_or(60);let opened=vsn_core::preview_websocket_start(&principal,&vsn_preview::PreviewWebSocketRequest{port,path:effective.resource_id.clone(),max_duration_seconds}).map_err(|e|e.to_string())?;Some(opened.session_id)}else{None};
                Ok(ActiveStreamRelay{principal,request:effective,stream_id:state.stream_id,file_offset,preview_stream_id,preview_websocket_id,preview_http_input:Vec::new()})
            })();
            match result{Ok(relay)=>{let stream_id=relay.stream_id.clone();let resource_id=relay.request.resource_id.clone();relays.insert(relay_id.clone(),relay);client.send_client(&vsn_remote::AgentStreamClientMessageV1::Opened{relay_id:relay_id.clone(),ok:true,stream_id:Some(stream_id),resource_id:Some(resource_id),error:None})?;pump_stream_relay(client,relays,&relay_id)?;},Err(error)=>client.send_client(&vsn_remote::AgentStreamClientMessageV1::Opened{relay_id,ok:false,stream_id:None,resource_id:None,error:Some(error)})?;}
        },
        vsn_remote::AgentStreamServerMessageV1::Input{relay_id,frame}=>{
            frame.decoded_len()?;let mut close_after_ack=false;let mut preview_http_response:Option<Vec<u8>>=None;if let Some(relay)=relays.get_mut(&relay_id){let expected=vsn_core::stream_expected_input_seq(&relay.principal,&relay.stream_id)?;if frame.seq!=expected{return Err(format!("agent stream input sequence mismatch: expected {expected}, got {}",frame.seq).into());}
                let(committed_bytes,digest_sha256)=match relay.request.kind{
                    vsn_stream::StreamKind::Terminal=>{use base64::{engine::general_purpose::STANDARD as B64,Engine as _};let bytes=B64.decode(&frame.payload_base64)?;let text=String::from_utf8(bytes).map_err(|_|"terminal stream input must be UTF-8")?;vsn_core::terminal_pty_write(&relay.principal,&relay.request.resource_id,&text)?;(None,None)},
                    vsn_stream::StreamKind::FileUpload=>{let transfer=relay.request.metadata.get("transfer_id").ok_or("file upload stream requires transfer_id metadata")?;let expected=relay.request.metadata.get("expected_sha256").map(String::as_str);let result=vsn_core::file_write_binary_chunk(&relay.principal,Path::new(&relay.request.resource_id),transfer,relay.file_offset,&frame.payload_base64,frame.eof,expected)?;relay.file_offset=result.committed_bytes;close_after_ack=frame.eof;(Some(result.committed_bytes),result.sha256)},
                    vsn_stream::StreamKind::Preview if relay.request.metadata.get("mode").map(String::as_str)==Some("http")=>{use base64::{engine::general_purpose::STANDARD as B64,Engine as _};let bytes=B64.decode(&frame.payload_base64)?;if relay.preview_http_input.len().saturating_add(bytes.len())>4*1024*1024{return Err("preview HTTP relay request envelope exceeds 4 MiB".into());}relay.preview_http_input.extend_from_slice(&bytes);if frame.eof{let request:vsn_preview::PreviewHttpRequest=serde_json::from_slice(&relay.preview_http_input).map_err(|e|format!("invalid preview HTTP relay request: {e}"))?;let port=relay.request.metadata.get("port").ok_or("preview HTTP relay requires port metadata")?.parse::<u16>()?;if request.port!=port||request.path!=relay.request.resource_id{return Err("preview HTTP relay target does not match authorized stream target".into());}let response=vsn_core::preview_request(&relay.principal,&request)?;preview_http_response=Some(serde_json::to_vec(&response)?);close_after_ack=true;}(None,None)},
                    vsn_stream::StreamKind::Preview if relay.request.metadata.get("mode").map(String::as_str)==Some("websocket")=>{use base64::{engine::general_purpose::STANDARD as B64,Engine as _};let bytes=B64.decode(&frame.payload_base64)?;let request:vsn_preview::PreviewWebSocketSend=serde_json::from_slice(&bytes).map_err(|e|format!("invalid preview WebSocket relay frame: {e}"))?;let id=relay.preview_websocket_id.as_deref().ok_or("preview WebSocket relay missing local session")?;vsn_core::preview_websocket_send(&relay.principal,id,&request)?;(None,None)},
                    _=>return Err("stream kind does not accept relay input".into()),
                };
                let stream_eof=if matches!(relay.request.kind,vsn_stream::StreamKind::Preview)&&relay.request.metadata.get("mode").map(String::as_str)==Some("http"){false}else{frame.eof};vsn_core::stream_input(&relay.principal,&relay.stream_id,frame.seq,&frame.payload_base64,stream_eof)?;let _=vsn_core::stream_input_pull(&relay.principal,&relay.stream_id,16);if let Some(bytes)=preview_http_response.take(){send_relay_output_chunked(client,&relay_id,relay,&bytes,true)?;}client.send_client(&vsn_remote::AgentStreamClientMessageV1::InputAck{relay_id:relay_id.clone(),next_input_seq:frame.seq.saturating_add(1),committed_bytes,digest_sha256})?;
                if !close_after_ack{pump_stream_relay(client,relays,&relay_id)?;}
            }
            if close_after_ack{if let Some(relay)=relays.remove(&relay_id){let reason=if matches!(relay.request.kind,vsn_stream::StreamKind::Preview){"preview_http_complete"}else{"upload_complete"};let _=vsn_core::stream_close(&relay.principal,&relay.stream_id,Some(reason));client.send_client(&vsn_remote::AgentStreamClientMessageV1::Closed{relay_id,reason:Some(reason.into())})?;}}
        },
        vsn_remote::AgentStreamServerMessageV1::Close{relay_id,reason}=>{if let Some(relay)=relays.remove(&relay_id){if let Some(id)=relay.preview_stream_id.as_deref(){let _=vsn_core::preview_event_stream_close(&relay.principal,id);}if let Some(id)=relay.preview_websocket_id.as_deref(){let _=vsn_core::preview_websocket_close(&relay.principal,id);}let _=vsn_core::stream_close(&relay.principal,&relay.stream_id,reason.as_deref());client.send_client(&vsn_remote::AgentStreamClientMessageV1::Closed{relay_id,reason})?;}},
        vsn_remote::AgentStreamServerMessageV1::Ping{timestamp_unix_ms}=>{let ids=relays.keys().cloned().collect::<Vec<_>>();for id in ids{let _=pump_stream_relay(client,relays,&id);}client.send_client(&vsn_remote::AgentStreamClientMessageV1::Pong{timestamp_unix_ms})?;},
    }Ok(())
}

fn pump_stream_relay(client:&mut vsn_remote::AgentStreamRelayClient,relays:&mut HashMap<String,ActiveStreamRelay>,relay_id:&str)->Result<(),Box<dyn std::error::Error>>{
    let mut close=false;if let Some(relay)=relays.get_mut(relay_id){match relay.request.kind{
        vsn_stream::StreamKind::Terminal=>{let chunk=vsn_core::terminal_pty_read_wait(&relay.principal,&relay.request.resource_id,64*1024,1)?;if !chunk.output.is_empty(){send_relay_output(client,relay_id,relay,chunk.output.as_bytes(),false)?;}if !chunk.running{send_relay_output(client,relay_id,relay,&[],true)?;close=true;}},
        vsn_stream::StreamKind::FileDownload=>{let chunk=vsn_core::file_read_binary_chunk(&relay.principal,Path::new(&relay.request.resource_id),relay.file_offset,vsn_files::MAX_BINARY_CHUNK_BYTES)?;use base64::{engine::general_purpose::STANDARD as B64,Engine as _};let bytes=B64.decode(&chunk.data_b64)?;relay.file_offset=relay.file_offset.saturating_add(chunk.bytes as u64);send_relay_output(client,relay_id,relay,&bytes,chunk.eof)?;if chunk.eof{close=true;}},
        vsn_stream::StreamKind::Preview=>{let mode=relay.request.metadata.get("mode").map(String::as_str).unwrap_or("snapshot");if mode=="http"{}else if mode=="sse"{let id=relay.preview_stream_id.as_deref().ok_or("preview SSE relay is missing its local stream id")?;let chunk=vsn_core::preview_event_stream_read(&relay.principal,id)?;if !chunk.payload_base64.is_empty(){use base64::{engine::general_purpose::STANDARD as B64,Engine as _};let bytes=B64.decode(&chunk.payload_base64)?;send_relay_output(client,relay_id,relay,&bytes,false)?;}if chunk.eof{if let Some(error)=chunk.error.as_deref(){send_relay_output(client,relay_id,relay,error.as_bytes(),false)?;}send_relay_output(client,relay_id,relay,&[],true)?;close=true;}}else if mode=="websocket"{let id=relay.preview_websocket_id.as_deref().ok_or("preview WebSocket relay is missing its local session id")?;let frame=vsn_core::preview_websocket_read(&relay.principal,id)?;if !frame.payload_base64.is_empty()||frame.eof||frame.error.is_some(){let bytes=serde_json::to_vec(&frame)?;send_relay_output(client,relay_id,relay,&bytes,frame.eof)?;}if frame.eof{close=true;}}else{let port=relay.request.metadata.get("port").ok_or("preview stream requires port metadata")?.parse::<u16>()?;let response=vsn_core::preview_fetch(&relay.principal,&vsn_preview::PreviewRequest{port,path:relay.request.resource_id.clone(),method:relay.request.metadata.get("method").cloned().unwrap_or_else(||"GET".into())})?;let bytes=serde_json::to_vec(&response)?;send_relay_output_chunked(client,relay_id,relay,&bytes,true)?;close=true;}},
        vsn_stream::StreamKind::Database=>{let roots=vsn_core::workspace_roots(&relay.principal)?;let path=vsn_files::resolve_existing(&roots,Path::new(&relay.request.resource_id))?;let operation=relay.request.metadata.get("operation").map(String::as_str).unwrap_or("browse");let result=match operation{
            "query"=>{let sql=relay.request.metadata.get("sql").ok_or("database query stream requires sql metadata")?;vsn_core::sqlite_query(&relay.principal,&path,sql)?},
            "browse"=>{let entity=relay.request.metadata.get("entity").ok_or("database browse stream requires entity metadata")?;let limit=relay.request.metadata.get("limit").and_then(|v|v.parse::<u32>().ok()).unwrap_or(100).clamp(1,1000);let offset=relay.request.metadata.get("offset").and_then(|v|v.parse::<u64>().ok()).unwrap_or(0);serde_json::to_value(vsn_core::sqlite_browse(&relay.principal,&path,entity,&vsn_database::BrowseRequest{limit,offset,order_by:None,descending:false})?)?},
            _=>return Err("database stream operation must be query or browse".into()),
        };let bytes=serde_json::to_vec(&result)?;send_relay_output_chunked(client,relay_id,relay,&bytes,true)?;close=true;},
        _=>{}
    }}if close{if let Some(relay)=relays.remove(relay_id){if let Some(id)=relay.preview_stream_id.as_deref(){let _=vsn_core::preview_event_stream_close(&relay.principal,id);}if let Some(id)=relay.preview_websocket_id.as_deref(){let _=vsn_core::preview_websocket_close(&relay.principal,id);}let _=vsn_core::stream_close(&relay.principal,&relay.stream_id,Some("resource_eof"));client.send_client(&vsn_remote::AgentStreamClientMessageV1::Closed{relay_id:relay_id.into(),reason:Some("resource_eof".into())})?;}}Ok(())
}
fn send_relay_output(client:&mut vsn_remote::AgentStreamRelayClient,relay_id:&str,relay:&ActiveStreamRelay,payload:&[u8],eof:bool)->Result<(),Box<dyn std::error::Error>>{let frame=vsn_core::stream_output(&relay.principal,&relay.stream_id,payload,eof)?;let _=vsn_core::stream_pull(&relay.principal,&relay.stream_id,1)?;client.send_client(&vsn_remote::AgentStreamClientMessageV1::Output{relay_id:relay_id.into(),frame:vsn_remote::RelayStreamFrameV1{seq:frame.seq,eof:frame.eof,payload_base64:frame.payload_base64}})?;Ok(())}
fn send_relay_output_chunked(client:&mut vsn_remote::AgentStreamRelayClient,relay_id:&str,relay:&ActiveStreamRelay,payload:&[u8],eof:bool)->Result<(),Box<dyn std::error::Error>>{
    if payload.is_empty(){return send_relay_output(client,relay_id,relay,&[],eof)}
    let chunks=payload.chunks(vsn_stream::MAX_FRAME_BYTES).collect::<Vec<_>>();let last=chunks.len().saturating_sub(1);for(i,chunk)in chunks.into_iter().enumerate(){send_relay_output(client,relay_id,relay,chunk,eof&&i==last)?;}Ok(())
}
fn agent_stream_permission(kind:&vsn_stream::StreamKind,direction:&vsn_stream::StreamDirection)->Option<&'static str>{use vsn_stream::{StreamDirection::*,StreamKind::*};Some(match(kind,direction){(Terminal,Bidirectional)=>"terminal.execute",(FileUpload,ClientToAgent)=>"files.write",(FileDownload,AgentToClient)=>"files.read",(Preview,AgentToClient)=>"project.view",(Preview,Bidirectional)=>"project.edit",(Database,AgentToClient)=>"database.query",_=>return None})}
fn agent_stream_permissions(kind:&vsn_stream::StreamKind,direction:&vsn_stream::StreamDirection)->Vec<vsn_policy::Permission>{use vsn_policy::Permission::*;match(kind,direction){(vsn_stream::StreamKind::Terminal,_)=>vec![TerminalExecute,TerminalView],(vsn_stream::StreamKind::FileUpload,_)=>vec![FilesWrite],(vsn_stream::StreamKind::FileDownload,_)=>vec![FilesRead],(vsn_stream::StreamKind::Preview,vsn_stream::StreamDirection::Bidirectional)=>vec![ProjectView,ProjectEdit],(vsn_stream::StreamKind::Preview,_)=>vec![ProjectView],(vsn_stream::StreamKind::Database,_)=>vec![DatabaseQuery,DatabaseView,ProjectView],_=>vec![MachineView]}}

fn execute_ai_request(principal:&vsn_policy::Principal,request:&vsn_ai::ExecuteRequest)->Result<vsn_ai::ExecutionReport,String>{
    let started=std::time::Instant::now();
    let plan=vsn_core::ai_plan(principal,&request.intent).map_err(|e|e.to_string())?;
    if plan.unrestricted_shell_allowed{return Err("AI plan attempted to enable unrestricted shell".into());}
    if plan.calls.is_empty()||plan.calls.len()>16{return Err("AI plan call count is outside 1..16".into());}
    if plan.calls.iter().any(|c|c.mutating&&c.requires_confirmation)&&!request.confirm_mutations{return Err("AI plan contains mutating calls that require confirm_mutations=true; no calls were executed".into());}
    for call in &plan.calls{let expected=ai_command_permission(&call.command).ok_or_else(||format!("AI plan contains an unapproved command: {}",call.command))?;if expected!=call.permission{return Err(format!("AI plan permission mismatch for {}: declared {}, expected {}",call.command,call.permission,expected));}if call.command.starts_with("ai."){return Err("AI plans may not recursively invoke AI commands".into());}}
    let mut results=Vec::new();let mut total_bytes=0usize;let mut completed=true;
    for call in &plan.calls{let(ok,value)=dispatch_command(principal,&call.command,&call.params);let bytes=serde_json::to_vec(&value).map(|v|v.len()).unwrap_or(0);total_bytes=total_bytes.saturating_add(bytes);if bytes>2*1024*1024||total_bytes>4*1024*1024{return Err("AI tool results exceeded bounded response budget".into());}results.push(vsn_ai::ToolExecution{command:call.command.clone(),ok,result:value});if !ok{completed=false;break;}}
    let report=vsn_ai::ExecutionReport{plan,results,completed};
    let record=vsn_ai::AiTelemetryRecord{timestamp_unix_ms:vsn_remote::now_ms(),adapter_id:"builtin.deterministic".into(),intent:report.plan.intent.clone(),accepted:true,calls:report.plan.calls.len(),mutating_calls:report.plan.calls.iter().filter(|c|c.mutating).count(),completed:report.completed,duration_ms:started.elapsed().as_millis()};
    vsn_core::ai_record_telemetry(principal,&record).map_err(|e|e.to_string())?;
    Ok(report)
}
fn ai_command_permission(command:&str)->Option<&'static str>{Some(match command{"project.detect"|"project.dependencies"=>"project.view","runtime.list"=>"runtime.view","port.list"|"port.check"=>"network.view","database.cli.inspect"=>"database.view","project.bootstrap-plan"|"project.bootstrap"=>"project.edit","status"|"process.list"=>"machine.view",_=>return None})}

fn required_remote_permission(command:&str)->Option<vsn_policy::Permission>{
    use vsn_policy::Permission::*;
    Some(match command{
        "ping"|"status"|"machine"|"diagnostics"|"process.list"|"process.metrics"=>MachineView,
        "security.status"|"audit.verify"=>SecurityAuditView,
        "config.show"=>MachineView,
        "port.list"|"port.check"|"domain.plan"|"network.conformance"=>NetworkView,
        "domain.apply-hosts"|"domain.remove-hosts"|"domain.reload"=>NetworkManage,
        "service.status"|"health.tcp"|"log.tail"=>ServiceView,
        "service.action"|"process.managed.stop"|"process.managed.start"|"process.managed.remove"=>ServiceManage,
        "process.managed.status"|"process.managed.list"=>ServiceView,
        "runtime.list"|"runtime.registry"|"runtime.catalog"|"runtime.audit"|"runtime.conformance"|"marketplace.conformance"|"container.backends"|"container.list"|"container.images"|"container.volumes"|"container.networks"|"container.logs"|"container.inspect"|"container.stats"=>RuntimeView,
        "runtime.install"|"runtime.activate"|"runtime.uninstall"|"runtime.repair"|"container.action"|"container.compose"|"container.image-pull"|"container.image-build"|"container.remove"|"container.registry-publish"|"container.exec"=>RuntimeManage,
        "project.detect"|"project.dependencies"|"project.bootstrap-plan"|"workspace.roots"=>ProjectView,
        "files.conformance"|"files.list"|"files.read"|"files.binary.read"|"files.binary.status"|"files.digest"=>FilesRead,
        "files.write"|"files.binary.write"|"files.binary.abort"|"files.mkdir"|"files.move"|"files.delete"=>FilesWrite,
        "terminal.exec"|"terminal.session.start"|"terminal.session.write"|"terminal.session.stop"|"terminal.session.remove"|"terminal.pty.start"|"terminal.pty.write"|"terminal.pty.resize"|"terminal.pty.stop"|"terminal.pty.remove"|"terminal.pty.scrollback.remove"=>TerminalExecute,
        "terminal.conformance"|"terminal.session.read"|"terminal.session.read-wait"|"terminal.session.status"|"terminal.session.list"|"terminal.pty.read"|"terminal.pty.read-wait"|"terminal.pty.status"|"terminal.pty.list"|"terminal.pty.scrollback.list"|"terminal.pty.scrollback.read"=>TerminalView,
        "preview.fetch"|"preview.conformance"=>ProjectView,
        "database.remote.conformance"|"database.studio.conformance"|"database.workspace"|"database.ui-demo"|"database.model.analyze"|"database.sqlite.inspect"|"database.sqlite.browse"|"database.sqlite.indexes"|"database.sqlite.relations"|"database.sqlite.stats"|"database.cli.detect"|"database.cli.inspect"|"database.cli.job.status"|"database.cli.job.list"|"database.cli.job.output"|"database.native.postgres.inspect"|"database.native.postgres.browse"|"database.native.postgres.indexes"|"database.native.postgres.relations"|"database.native.postgres.stats"|"database.native.postgres.job.status"|"database.native.postgres.job.list"|"database.native.postgres.txn.status"|"database.native.mysql.inspect"|"database.native.mysql.browse"|"database.native.mysql.indexes"|"database.native.mysql.relations"|"database.native.mysql.stats"|"database.native.mongo.inspect"|"database.native.mongo.browse"|"database.native.mongo.indexes"|"database.native.mongo.stats"|"database.native.redis.inspect"|"database.native.redis.get"=>DatabaseView,
        "database.sqlite.query"|"database.cli.query"|"database.cli.job.start"|"database.cli.job.cancel"|"database.cli.job.output-remove"|"database.native.postgres.query"|"database.native.postgres.job.start"|"database.native.postgres.job.cancel"|"database.native.postgres.txn.start"|"database.native.postgres.txn.query"|"database.native.postgres.txn.close"|"database.native.mysql.query"=>DatabaseQuery,
        "vault.list"=>SecretsUse,
        "vault.set"|"vault.delete"=>SecretsManage,
        "remote.status"|"cloud.conformance"=>RemoteView,
        _=>return None,
    })
}

fn respond<T:serde::Serialize,E:std::fmt::Display>(result:Result<T,E>)->(bool,Value){match result{Ok(value)=>(true,json!(value)),Err(error)=>(false,json!({"error":error.to_string()}))}}
fn param_str<'a>(params:&'a Value,key:&str)->Result<&'a str,String>{params.get(key).and_then(Value::as_str).filter(|v|!v.is_empty()).ok_or_else(||format!("missing or invalid parameter: {key}"))}
fn param_str_allow_empty<'a>(params:&'a Value,key:&str)->Result<&'a str,String>{params.get(key).and_then(Value::as_str).ok_or_else(||format!("missing or invalid parameter: {key}"))}
fn param_u16(params:&Value,key:&str)->Result<u16,String>{params.get(key).and_then(Value::as_u64).and_then(|v|u16::try_from(v).ok()).ok_or_else(||format!("missing or invalid u16 parameter: {key}"))}
fn param_u32(params:&Value,key:&str)->Result<u32,String>{params.get(key).and_then(Value::as_u64).and_then(|v|u32::try_from(v).ok()).ok_or_else(||format!("missing or invalid u32 parameter: {key}"))}
fn parse_data_model(value:&str)->Result<vsn_database::DataModel,String>{use vsn_database::DataModel::*;match value{"relational"=>Ok(Relational),"document"=>Ok(Document),"key_value"=>Ok(KeyValue),"graph"=>Ok(Graph),"search"=>Ok(Search),"time_series"=>Ok(TimeSeries),"column"=>Ok(Column),"vector"=>Ok(Vector),"custom"=>Ok(Custom),_=>Err(format!("unknown database model: {value}"))}}
fn parse_managed_spec(params:&Value)->Result<vsn_system::ManagedProcessSpec,String>{let id=param_str(params,"id")?.into();let program=Path::new(param_str(params,"program")?).to_path_buf();let cwd=Path::new(param_str(params,"cwd")?).to_path_buf();let log_path=Path::new(param_str(params,"log_path")?).to_path_buf();let args=params.get("args").and_then(Value::as_array).map(|a|a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();Ok(vsn_system::ManagedProcessSpec{id,program,args,cwd,env:vec![],log_path})}
fn parse_terminal_request(params:&Value)->Result<vsn_terminal::ExecRequest,String>{serde_json::from_value(params.clone()).map_err(|e|format!("invalid terminal request: {e}"))}
fn parse_pty_session_start(params:&Value)->Result<vsn_terminal::PtySessionStartRequest,String>{serde_json::from_value(params.clone()).map_err(|e|format!("invalid PTY session request: {e}"))}
fn parse_terminal_session_start(params:&Value)->Result<vsn_terminal::SessionStartRequest,String>{serde_json::from_value(params.clone()).map_err(|e|format!("invalid terminal session request: {e}"))}
fn parse_browse_request(params:&Value)->Result<vsn_database::BrowseRequest,String>{serde_json::from_value(params.get("request").cloned().unwrap_or_else(||json!({"limit":params.get("limit").and_then(Value::as_u64).unwrap_or(100),"offset":params.get("offset").and_then(Value::as_u64).unwrap_or(0),"order_by":params.get("order_by").cloned().unwrap_or(Value::Null),"descending":params.get("descending").and_then(Value::as_bool).unwrap_or(false)}))).map_err(|e|format!("invalid browse request: {e}"))}
fn parse_mutation_request(params:&Value)->Result<vsn_database::MutationRequest,String>{serde_json::from_value(params.get("request").cloned().unwrap_or_else(||json!({"values":params.get("values").cloned().unwrap_or_else(||json!({})),"filter":params.get("filter").cloned().unwrap_or_else(||json!({}))}))).map_err(|e|format!("invalid mutation request: {e}"))}
fn parse_preview_request(params:&Value)->Result<vsn_preview::PreviewRequest,String>{serde_json::from_value(params.clone()).map_err(|e|format!("invalid preview request: {e}"))}
fn parse_preview_http_request(params:&Value)->Result<vsn_preview::PreviewHttpRequest,String>{serde_json::from_value(params.clone()).map_err(|e|format!("invalid preview HTTP request: {e}"))}
fn parse_db_connection(params:&Value)->Result<vsn_database_cli::ConnectionSpec,String>{let value=params.get("connection").cloned().unwrap_or_else(||params.clone());serde_json::from_value(value).map_err(|e|format!("invalid database connection: {e}"))}
fn parse_postgres_native(params:&Value)->Result<vsn_database_native::PostgresConnection,String>{let value=params.get("connection").cloned().unwrap_or_else(||json!({"connection_string":params.get("connection_string").and_then(Value::as_str).unwrap_or_default()}));serde_json::from_value(value).map_err(|e|format!("invalid native PostgreSQL connection: {e}"))}
fn parse_mysql_native(params:&Value)->Result<vsn_database_native::MySqlConnection,String>{let value=params.get("connection").cloned().unwrap_or_else(||json!({"url":params.get("url").and_then(Value::as_str).unwrap_or_default()}));serde_json::from_value(value).map_err(|e|format!("invalid native MySQL connection: {e}"))}
fn parse_postgres_tls(params:&Value)->Result<vsn_database_native::PostgresTlsConnection,String>{let value=params.get("connection").cloned().unwrap_or_else(||json!({"connection_string":params.get("connection_string").and_then(Value::as_str).unwrap_or_default(),"root_ca_pem_path":params.get("root_ca_pem_path").and_then(Value::as_str).unwrap_or_default()}));serde_json::from_value(value).map_err(|e|format!("invalid PostgreSQL TLS connection: {e}"))}
fn parse_mysql_tls(params:&Value)->Result<vsn_database_native::MySqlTlsConnection,String>{let value=params.get("connection").cloned().unwrap_or_else(||json!({"url":params.get("url").and_then(Value::as_str).unwrap_or_default(),"root_ca_path":params.get("root_ca_path").and_then(Value::as_str).unwrap_or_default()}));serde_json::from_value(value).map_err(|e|format!("invalid MySQL TLS connection: {e}"))}
fn parse_mongo_native(params:&Value)->Result<vsn_database_native::MongoConnection,String>{let value=params.get("connection").cloned().unwrap_or_else(||json!({"url":params.get("url").and_then(Value::as_str).unwrap_or_default()}));serde_json::from_value(value).map_err(|e|format!("invalid native MongoDB connection: {e}"))}
fn parse_redis_native(params:&Value)->Result<vsn_database_native::RedisConnection,String>{let value=params.get("connection").cloned().unwrap_or_else(||json!({"url":params.get("url").and_then(Value::as_str).unwrap_or_default()}));serde_json::from_value(value).map_err(|e|format!("invalid native Redis connection: {e}"))}
fn parse_remote_config(params:&Value)->Result<vsn_config::RemoteConfig,String>{Ok(vsn_config::RemoteConfig{enabled:params.get("enabled").and_then(Value::as_bool).unwrap_or(false),control_plane_url:params.get("url").and_then(Value::as_str).map(str::to_string),control_plane_public_key:params.get("public_key").and_then(Value::as_str).map(str::to_string),poll_interval_ms:params.get("poll_interval_ms").and_then(Value::as_u64).unwrap_or(2500),allow_remote_terminal:params.get("allow_remote_terminal").and_then(Value::as_bool).unwrap_or(false),allow_remote_file_write:params.get("allow_remote_file_write").and_then(Value::as_bool).unwrap_or(false),allow_remote_database_query:params.get("allow_remote_database_query").and_then(Value::as_bool).unwrap_or(false),allow_remote_preview_interactive:params.get("allow_remote_preview_interactive").and_then(Value::as_bool).unwrap_or(false)})}
fn parse_ssh_release_status(params:&Value)->Result<(vsn_cloud::ExistingSshTarget,String),String>{let target:vsn_cloud::ExistingSshTarget=serde_json::from_value(params.get("target").cloned().ok_or_else(||"missing target".to_string())?).map_err(|e|format!("invalid SSH target: {e}"))?;let name=param_str(params,"workspace_name")?.to_string();Ok((target,name))}
fn demo_entity()->vsn_database::EntityMeta{use vsn_database::{EntityMeta,FieldMeta,FieldType};EntityMeta{name:"users".into(),display_name:"Users".into(),fields:vec![FieldMeta{name:"id".into(),field_type:FieldType::Integer,nullable:false,primary:true,generated:true,enum_values:vec![],relation_target:None,metadata:Value::Null},FieldMeta{name:"email".into(),field_type:FieldType::Text,nullable:false,primary:false,generated:false,enum_values:vec![],relation_target:None,metadata:Value::Null},FieldMeta{name:"active".into(),field_type:FieldType::Boolean,nullable:false,primary:false,generated:false,enum_values:vec![],relation_target:None,metadata:Value::Null},FieldMeta{name:"profile".into(),field_type:FieldType::Json,nullable:true,primary:false,generated:false,enum_values:vec![],relation_target:None,metadata:Value::Null},FieldMeta{name:"created_at".into(),field_type:FieldType::DateTime,nullable:false,primary:false,generated:true,enum_values:vec![],relation_target:None,metadata:Value::Null}],metadata:Value::Null}}
fn audit(actor_type:&str,actor_id:&str,action:&str,target:&str,result:&str,metadata:BTreeMap<String,String>)->Result<(),Box<dyn std::error::Error>>{let _=vsn_core::write_audit(AuditEventInput{actor_type:actor_type.into(),actor_id:actor_id.into(),action:action.into(),target:target.into(),result:result.into(),metadata})?;Ok(())}

fn dns_server_command(args:&[String])->ExitCode{let mut listen="127.0.0.1:53535".to_string();let mut i=0usize;while i<args.len(){match args[i].as_str(){"--listen" if i+1<args.len()=>{listen=args[i+1].clone();i+=2},_=>{eprintln!("error=usage: vsn-agent dns-server [--listen 127.0.0.1:53535]");return ExitCode::FAILURE}}}let stop=AtomicBool::new(false);match vsn_network::run_dns_server(&vsn_network::DnsServerConfig{listen,suffix:".test".into(),ttl_seconds:30},&stop){Ok(())=>ExitCode::SUCCESS,Err(e)=>{eprintln!("dns_server_error={e}");ExitCode::FAILURE}}}

fn network_admin_command(args:&[String])->ExitCode{
    if !is_os_elevated(){eprintln!("error=network-admin commands require OS elevation (Run as Administrator/root)");return ExitCode::FAILURE;}
    let principal=vsn_policy::Principal::local_network_admin();
    let result:Result<Value,String>=match args{
        [action,domain] if action=="apply-hosts"=>vsn_core::domain_apply_hosts(&principal,domain).map(|v|json!(v)).map_err(|e|e.to_string()),
        [action,domain] if action=="remove-hosts"=>vsn_core::domain_remove_hosts(&principal,domain).map(|v|json!(v)).map_err(|e|e.to_string()),
        [action] if action=="install-ca"=>vsn_core::local_ca_install(&principal).map(|v|json!(v)).map_err(|e|e.to_string()),
        [action,domain,port] if action=="proxy-config"=>port.parse::<u16>().map_err(|_|"invalid port".to_string()).and_then(|p|vsn_core::caddy_proxy_config(&principal,domain,p).map(|v|json!(v)).map_err(|e|e.to_string())),
        [action] if action=="caddy-start"=>vsn_core::caddy_start(&principal).map(|v|json!(v)).map_err(|e|e.to_string()),
        [action] if action=="caddy-status"=>vsn_core::caddy_status(&principal).map(|v|json!(v)).map_err(|e|e.to_string()),
        [action] if action=="caddy-stop"=>vsn_core::caddy_stop(&principal).map(|v|json!(v)).map_err(|e|e.to_string()),
        [action] if action=="caddy-restart"=>vsn_core::caddy_restart(&principal).map(|v|json!(v)).map_err(|e|e.to_string()),
        [action,listen] if action=="resolver-apply"=>vsn_core::dns_os_apply(&principal,listen).map(|v|json!(v)).map_err(|e|e.to_string()),
        [action] if action=="resolver-remove"=>vsn_core::dns_os_remove(&principal).map(|v|json!(v)).map_err(|e|e.to_string()),
        [action] if action=="resolver-status"=>vsn_core::dns_os_status(&principal).map(|v|json!(v)).map_err(|e|e.to_string()),
        _=>Err("usage: vsn-agent network-admin apply-hosts <name.test> | remove-hosts <name.test> | install-ca | proxy-config <name.test> <port> | caddy-start | caddy-status | caddy-stop | caddy-restart | resolver-apply <127.0.0.1:53> | resolver-remove | resolver-status".into()),
    };
    match result{Ok(value)=>{println!("{}",serde_json::to_string_pretty(&value).unwrap_or_default());ExitCode::SUCCESS},Err(e)=>{eprintln!("network_admin_error={e}");ExitCode::FAILURE}}
}
fn is_os_elevated()->bool{
    #[cfg(windows)]{std::process::Command::new("net.exe").arg("session").output().map(|o|o.status.success()).unwrap_or(false)}
    #[cfg(not(windows))]{std::process::Command::new("id").arg("-u").output().ok().and_then(|o|String::from_utf8(o.stdout).ok()).map(|v|v.trim()=="0").unwrap_or(false)}
}

fn service_command(args:&[String])->ExitCode{#[cfg(windows)]{match windows_service_host::manage(args){Ok(())=>ExitCode::SUCCESS,Err(error)=>{eprintln!("service_error={error}");ExitCode::FAILURE}}}#[cfg(not(windows))]{let _=args;eprintln!("use the supplied systemd/LaunchAgent scripts for non-Windows service installation");ExitCode::FAILURE}}

#[cfg(windows)]mod windows_service_host{use super::run_agent;use std::{ffi::OsString,process::Command,sync::{Arc,atomic::{AtomicBool,Ordering}},time::Duration};use windows_service::{define_windows_service,service::{ServiceControl,ServiceControlAccept,ServiceExitCode,ServiceState,ServiceStatus,ServiceType},service_control_handler::{self,ServiceControlHandlerResult},service_dispatcher};const SERVICE_NAME:&str="VSNAgent";const SERVICE_DISPLAY_NAME:&str="VSN Agent";define_windows_service!(ffi_service_main,service_main);pub fn dispatch()->Result<(),Box<dyn std::error::Error>>{service_dispatcher::start(SERVICE_NAME,ffi_service_main)?;Ok(())}fn service_main(_arguments:Vec<OsString>){if let Err(error)=run_service(){eprintln!("windows_service_runtime_error={error}");}}fn run_service()->Result<(),Box<dyn std::error::Error>>{let stop=Arc::new(AtomicBool::new(false));let handler_stop=Arc::clone(&stop);let event_handler=move|control_event|->ServiceControlHandlerResult{match control_event{ServiceControl::Stop=>{handler_stop.store(true,Ordering::SeqCst);ServiceControlHandlerResult::NoError},ServiceControl::Interrogate=>ServiceControlHandlerResult::NoError,_=>ServiceControlHandlerResult::NotImplemented}};let status_handle=service_control_handler::register(SERVICE_NAME,event_handler)?;status_handle.set_service_status(ServiceStatus{service_type:ServiceType::OWN_PROCESS,current_state:ServiceState::Running,controls_accepted:ServiceControlAccept::STOP,exit_code:ServiceExitCode::Win32(0),checkpoint:0,wait_hint:Duration::default(),process_id:None})?;let result=run_agent(Arc::clone(&stop));status_handle.set_service_status(ServiceStatus{service_type:ServiceType::OWN_PROCESS,current_state:ServiceState::Stopped,controls_accepted:ServiceControlAccept::empty(),exit_code:if result.is_ok(){ServiceExitCode::Win32(0)}else{ServiceExitCode::Win32(1)},checkpoint:0,wait_hint:Duration::default(),process_id:None})?;result}pub fn manage(args:&[String])->Result<(),Box<dyn std::error::Error>>{match args.first().map(String::as_str){Some("install")=>install(),Some("start")=>sc(&["start",SERVICE_NAME]),Some("stop")=>sc(&["stop",SERVICE_NAME]),Some("status")=>sc(&["query",SERVICE_NAME]),Some("uninstall")|Some("remove")=>sc(&["delete",SERVICE_NAME]),_=>{println!("VSN Agent Windows Service commands:\n  vsn-agent service install\n  vsn-agent service start\n  vsn-agent service stop\n  vsn-agent service status\n  vsn-agent service uninstall");Ok(())}}}fn install()->Result<(),Box<dyn std::error::Error>>{vsn_core::provision_local_ipc()?;let exe=std::env::current_exe()?;let bin_path=format!("\"{}\" --service-run",exe.display());sc(&["create",SERVICE_NAME,"binPath=",&bin_path,"start=","auto","obj=","NT AUTHORITY\\LocalService","DisplayName=",SERVICE_DISPLAY_NAME])?;let _=sc(&["description",SERVICE_NAME,"VSN machine-local execution and secure control agent"]);println!("service_installed={SERVICE_NAME}");Ok(())}fn sc(args:&[&str])->Result<(),Box<dyn std::error::Error>>{let output=Command::new("sc.exe").args(args).output()?;print!("{}",String::from_utf8_lossy(&output.stdout));eprint!("{}",String::from_utf8_lossy(&output.stderr));if !output.status.success(){return Err(format!("sc.exe failed with status {}",output.status).into());}Ok(())}}
