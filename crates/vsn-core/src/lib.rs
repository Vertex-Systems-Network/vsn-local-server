use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use vsn_audit::{AuditError, AuditEvent, AuditEventInput};
use vsn_policy::{Permission, PolicyError, Principal};
use vsn_security::{DeviceIdentity, SecurityError};
use vsn_types::{HealthStatus, MachineIdentity, SecurityStatus};

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("security initialization failed: {0}")]
    Security(#[from] SecurityError),
    #[error("audit operation failed: {0}")]
    Audit(#[from] AuditError),
    #[error("policy denied operation: {0}")]
    Policy(#[from] PolicyError),
    #[error("system operation failed: {0}")]
    System(#[from] vsn_system::SystemError),
    #[error("runtime operation failed: {0}")]
    Runtime(#[from] vsn_runtime::RuntimeError),
    #[error("container operation failed: {0}")]
    Container(#[from] vsn_container::ContainerError),
    #[error("network operation failed: {0}")]
    Network(#[from] vsn_network::NetworkError),
    #[error("project operation failed: {0}")]
    Project(#[from] vsn_project::ProjectError),
    #[error("database operation failed: {0}")]
    Database(#[from] vsn_database::DatabaseError),
    #[error("configuration operation failed: {0}")]
    Config(#[from] vsn_config::ConfigError),
    #[error("remote operation failed: {0}")]
    Remote(#[from] vsn_remote::RemoteError),
    #[error("vault operation failed: {0}")]
    Vault(#[from] vsn_vault::VaultError),
    #[error("workspace file operation failed: {0}")]
    Files(#[from] vsn_files::FileError),
    #[error("terminal operation failed: {0}")]
    Terminal(#[from] vsn_terminal::TerminalError),
    #[error("database CLI operation failed: {0}")]
    DatabaseCli(#[from] vsn_database_cli::CliDatabaseError),
    #[error("native database operation failed: {0}")]
    DatabaseNative(#[from] vsn_database_native::NativeDbError),
    #[error("preview operation failed: {0}")]
    Preview(#[from] vsn_preview::PreviewError),
    #[error("stream operation failed: {0}")]
    Stream(#[from] vsn_stream::StreamError),
    #[error("operation rejected: {0}")]
    Rejected(String),
}

pub fn device_identity() -> Result<DeviceIdentity, CoreError> {
    Ok(DeviceIdentity::load_or_create()?)
}
pub fn local_machine_identity() -> Result<MachineIdentity, CoreError> {
    let identity = device_identity()?;
    let meta = identity.metadata();
    Ok(MachineIdentity {
        device_id: meta.device_id.clone(),
        display_name: meta.display_name.clone(),
        os: meta.os.clone(),
        public_key: meta.public_key.clone(),
        created_at_unix: meta.created_at_unix,
    })
}
pub fn provision_local_ipc() -> Result<(), CoreError> {
    let _ = vsn_security::IpcAuthenticator::load_or_create()?;
    Ok(())
}
pub fn security_status() -> Result<SecurityStatus, CoreError> {
    let _ = DeviceIdentity::load_or_create()?;
    let _ = vsn_security::IpcAuthenticator::load_or_create()?;
    Ok(SecurityStatus {
        device_identity_ready: true,
        ipc_secret_ready: true,
        secure_store: vsn_security::secure_store_name().into(),
    })
}
pub fn core_health() -> HealthStatus {
    HealthStatus {
        service: "vsn-core".into(),
        healthy: true,
        detail: "core initialized".into(),
    }
}
pub fn diagnostics(principal: &Principal) -> Result<serde_json::Value, CoreError> {
    vsn_policy::require(principal, Permission::MachineView)?;
    let cfg = config()?;
    let security = security_status()?;
    let data_dir = vsn_security::data_dir()?;
    let audit_path = vsn_audit::default_audit_path()?;
    let audit = if audit_path.exists() {
        match vsn_audit::verify(&audit_path) {
            Ok(count) => serde_json::json!({"ok":true,"events":count,"path":audit_path}),
            Err(e) => serde_json::json!({"ok":false,"error":e.to_string(),"path":audit_path}),
        }
    } else {
        serde_json::json!({"ok":true,"events":0,"path":audit_path,"note":"audit file not created yet"})
    };
    let identity = local_machine_identity()?;
    Ok(
        serde_json::json!({"healthy":audit.get("ok").and_then(|v|v.as_bool()).unwrap_or(false),"config":{"version":cfg.version,"workspace_roots":cfg.workspace_roots.len(),"execution_backend":cfg.default_execution_backend,"remote_enabled":cfg.remote.enabled},"security":security,"machine":identity,"data_dir":data_dir,"audit":audit,"event_bus_ready":true}),
    )
}
static EVENT_BUS: OnceLock<vsn_events::EventBus> = OnceLock::new();
pub fn event_bus() -> &'static vsn_events::EventBus {
    EVENT_BUS.get_or_init(vsn_events::EventBus::new)
}
pub fn config() -> Result<vsn_config::AppConfig, CoreError> {
    Ok(vsn_config::load_or_default()?)
}
pub fn write_audit(input: AuditEventInput) -> Result<AuditEvent, CoreError> {
    let identity = device_identity()?;
    let path = vsn_audit::default_audit_path()?;
    Ok(vsn_audit::append(&path, &identity, input)?)
}
pub fn verify_audit() -> Result<usize, CoreError> {
    let path = vsn_audit::default_audit_path()?;
    Ok(vsn_audit::verify(&path)?)
}
pub fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn processes(principal: &Principal) -> Result<Vec<vsn_system::ProcessInfo>, CoreError> {
    vsn_policy::require(principal, Permission::MachineView)?;
    Ok(vsn_system::list_processes()?)
}
pub fn process_metrics(
    principal: &Principal,
    pid: u32,
) -> Result<vsn_system::ProcessMetrics, CoreError> {
    vsn_policy::require(principal, Permission::MachineView)?;
    Ok(vsn_system::process_metrics(pid)?)
}
pub fn ports(principal: &Principal) -> Result<Vec<vsn_system::PortInfo>, CoreError> {
    vsn_policy::require(principal, Permission::NetworkView)?;
    Ok(vsn_system::list_ports()?)
}
pub fn port_conflicts(
    principal: &Principal,
    port: u16,
) -> Result<Vec<vsn_system::PortInfo>, CoreError> {
    vsn_policy::require(principal, Permission::NetworkView)?;
    Ok(vsn_system::port_conflicts(port)?)
}
pub fn service_state(
    principal: &Principal,
    name: &str,
) -> Result<vsn_system::ServiceState, CoreError> {
    vsn_policy::require(principal, Permission::ServiceView)?;
    Ok(vsn_system::service_state(name)?)
}
pub fn service_action(
    principal: &Principal,
    name: &str,
    action: &str,
) -> Result<vsn_system::ServiceState, CoreError> {
    vsn_policy::require(principal, Permission::ServiceManage)?;
    if !name.starts_with("VSN-") {
        return Err(CoreError::Rejected(
            "only VSN-managed OS services (VSN-* names) may be mutated through the baseline Agent"
                .into(),
        ));
    }
    let state = vsn_system::service_action(name, action)?;
    publish(
        "service.changed",
        serde_json::json!({"service":name,"action":action,"state":state.state.clone()}),
    );
    Ok(state)
}
pub fn service_conformance(
    principal: &Principal,
) -> Result<vsn_system::ServiceProviderConformanceReport, CoreError> {
    vsn_policy::require(principal, Permission::ServiceView)?;
    Ok(vsn_system::service_provider_conformance())
}
pub fn tcp_health(
    principal: &Principal,
    host: &str,
    port: u16,
    timeout_ms: u64,
) -> Result<vsn_system::HealthCheck, CoreError> {
    vsn_policy::require(principal, Permission::ServiceView)?;
    Ok(vsn_system::tcp_health(host, port, timeout_ms))
}
pub fn tail_log(
    principal: &Principal,
    path: &Path,
    lines: usize,
) -> Result<Vec<String>, CoreError> {
    vsn_policy::require(principal, Permission::FilesRead)?;
    let base = vsn_security::data_dir()?
        .canonicalize()
        .map_err(|e| CoreError::Rejected(format!("VSN data directory unavailable: {e}")))?;
    let requested = path
        .canonicalize()
        .map_err(|e| CoreError::Rejected(format!("log path unavailable: {e}")))?;
    if !requested.starts_with(&base) {
        return Err(CoreError::Rejected(
            "baseline log access is restricted to VSN-owned data".into(),
        ));
    }
    Ok(vsn_system::tail_log(&requested, lines)?)
}

fn managed_state_dir() -> Result<PathBuf, CoreError> {
    Ok(vsn_security::data_dir()?.join("processes"))
}
pub fn managed_process_start(
    principal: &Principal,
    spec: &vsn_system::ManagedProcessSpec,
) -> Result<vsn_system::ManagedProcessState, CoreError> {
    vsn_policy::require(principal, Permission::ServiceManage)?;
    let base = vsn_security::data_dir()?;
    if !spec.log_path.starts_with(&base) {
        return Err(CoreError::Rejected(
            "managed process log must be stored under the VSN data directory".into(),
        ));
    }
    let state = vsn_system::spawn_managed(spec, &managed_state_dir()?)?;
    publish(
        "process.started",
        serde_json::json!({"id":state.id,"pid":state.pid}),
    );
    Ok(state)
}
pub fn managed_process_state(
    principal: &Principal,
    id: &str,
) -> Result<vsn_system::ManagedProcessState, CoreError> {
    vsn_policy::require(principal, Permission::ServiceView)?;
    Ok(vsn_system::managed_process_state(
        id,
        &managed_state_dir()?,
    )?)
}
pub fn managed_process_stop(
    principal: &Principal,
    id: &str,
) -> Result<vsn_system::ManagedProcessState, CoreError> {
    vsn_policy::require(principal, Permission::ServiceManage)?;
    let state = vsn_system::stop_managed(id, &managed_state_dir()?)?;
    publish(
        "process.stopped",
        serde_json::json!({"id":state.id,"pid":state.pid}),
    );
    Ok(state)
}
pub fn managed_process_list(
    principal: &Principal,
) -> Result<Vec<vsn_system::ManagedProcessState>, CoreError> {
    vsn_policy::require(principal, Permission::ServiceView)?;
    Ok(vsn_system::list_managed(&managed_state_dir()?)?)
}
pub fn managed_process_remove(
    principal: &Principal,
    id: &str,
    force: bool,
) -> Result<bool, CoreError> {
    vsn_policy::require(principal, Permission::ServiceManage)?;
    let removed = vsn_system::remove_managed(id, &managed_state_dir()?, force)?;
    if removed {
        publish("process.removed", serde_json::json!({"id":id}));
    }
    Ok(removed)
}

pub fn runtime_detect(
    principal: &Principal,
) -> Result<Vec<vsn_runtime::RuntimeDetection>, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    Ok(vsn_runtime::detect_all())
}
pub fn runtime_provider_conformance(
    principal: &Principal,
) -> Result<vsn_runtime::RuntimeProviderConformanceReport, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    Ok(vsn_runtime::builtin_provider_conformance())
}
fn runtime_root() -> Result<PathBuf, CoreError> {
    Ok(vsn_security::data_dir()?.join("runtimes"))
}
fn runtime_registry_path() -> Result<PathBuf, CoreError> {
    Ok(runtime_root()?.join("registry.json"))
}
pub fn runtime_registry(principal: &Principal) -> Result<vsn_runtime::RuntimeRegistry, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    Ok(vsn_runtime::load_registry(&runtime_registry_path()?)?)
}
pub fn runtime_repair(
    principal: &Principal,
) -> Result<vsn_runtime::RuntimeRepairReport, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeManage)?;
    Ok(vsn_runtime::repair_registry(&runtime_registry_path()?)?)
}
pub fn runtime_audit(principal: &Principal) -> Result<vsn_runtime::RuntimeAuditReport, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    Ok(vsn_runtime::audit_registry(&runtime_registry_path()?)?)
}
pub fn runtime_catalog(
    principal: &Principal,
    path: &Path,
) -> Result<vsn_runtime::RuntimeCatalog, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    Ok(vsn_runtime::load_catalog(path)?)
}
pub fn runtime_catalog_verify(
    principal: &Principal,
    path: &Path,
    trust: &Path,
) -> Result<serde_json::Value, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    let (catalog, signer) = vsn_runtime::load_catalog_verified(path, trust)?;
    Ok(
        serde_json::json!({"provider":catalog.provider,"releases":catalog.runtimes.len(),"signer_public_key":signer}),
    )
}
pub fn runtime_install(
    principal: &Principal,
    catalog_path: &Path,
    runtime: &str,
    version: &str,
) -> Result<vsn_runtime::InstalledRuntime, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeManage)?;
    let catalog = vsn_runtime::load_catalog(catalog_path)?;
    runtime_install_from_catalog(&catalog, runtime, version)
}
pub fn runtime_install_trusted(
    principal: &Principal,
    catalog_path: &Path,
    trust_path: &Path,
    runtime: &str,
    version: &str,
) -> Result<vsn_runtime::InstalledRuntime, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeManage)?;
    let (catalog, _signer) = vsn_runtime::load_catalog_verified(catalog_path, trust_path)?;
    runtime_install_from_catalog(&catalog, runtime, version)
}
fn runtime_install_from_catalog(
    catalog: &vsn_runtime::RuntimeCatalog,
    runtime: &str,
    version: &str,
) -> Result<vsn_runtime::InstalledRuntime, CoreError> {
    let root = runtime_root()?;
    let plan = vsn_runtime::install_plan(catalog, runtime, version, &root)?;
    let artifact = vsn_runtime::download_artifact(&plan, &root.join("cache"))?;
    let installed = vsn_runtime::install_from_artifact(&plan, &artifact)?;
    let _ = vsn_runtime::register_runtime(&runtime_registry_path()?, installed.clone())?;
    let shim = vsn_runtime::write_shim(&root.join("shims"), runtime, &installed.executable)?;
    publish(
        "runtime.installed",
        serde_json::json!({"runtime":runtime,"version":version,"shim":shim}),
    );
    Ok(installed)
}
pub fn runtime_activate(
    principal: &Principal,
    project: &Path,
    runtime: &str,
    version: &str,
) -> Result<vsn_runtime::RuntimeRegistry, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeManage)?;
    let roots = config()?.workspace_roots;
    Ok(vsn_runtime::activate_for_project_in_workspaces(
        &runtime_registry_path()?,
        project,
        runtime,
        version,
        &roots,
    )?)
}
pub fn runtime_uninstall(
    principal: &Principal,
    runtime: &str,
    version: &str,
) -> Result<vsn_runtime::RuntimeRegistry, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeManage)?;
    let registry = vsn_runtime::uninstall_runtime(&runtime_registry_path()?, runtime, version)?;
    publish(
        "runtime.uninstalled",
        serde_json::json!({"runtime":runtime,"version":version}),
    );
    Ok(registry)
}

pub fn container_detect(
    principal: &Principal,
) -> Result<Vec<vsn_container::ContainerBackend>, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    Ok(vsn_container::detect_all())
}
pub fn container_list(
    principal: &Principal,
    backend: &str,
    all: bool,
) -> Result<Vec<vsn_container::ContainerInfo>, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    Ok(vsn_container::list_containers(backend, all)?)
}
pub fn container_images(
    principal: &Principal,
    backend: &str,
) -> Result<Vec<vsn_container::ContainerResource>, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    Ok(vsn_container::list_images(backend)?)
}
pub fn container_volumes(
    principal: &Principal,
    backend: &str,
) -> Result<Vec<vsn_container::ContainerResource>, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    Ok(vsn_container::list_volumes(backend)?)
}
pub fn container_networks(
    principal: &Principal,
    backend: &str,
) -> Result<Vec<vsn_container::ContainerResource>, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    Ok(vsn_container::list_networks(backend)?)
}
pub fn container_logs(
    principal: &Principal,
    backend: &str,
    target: &str,
    tail: u32,
) -> Result<String, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    Ok(vsn_container::container_logs(backend, target, tail)?)
}
pub fn container_inspect(
    principal: &Principal,
    backend: &str,
    target: &str,
) -> Result<String, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    Ok(vsn_container::container_inspect(backend, target)?)
}
pub fn container_stats(
    principal: &Principal,
    backend: &str,
    target: &str,
) -> Result<vsn_container::ContainerStats, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    Ok(vsn_container::container_stats(backend, target)?)
}
pub fn container_action(
    principal: &Principal,
    backend: &str,
    action: &str,
    target: &str,
) -> Result<vsn_container::ContainerActionResult, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeManage)?;
    Ok(vsn_container::container_action(backend, action, target)?)
}
pub fn container_image_pull(
    principal: &Principal,
    backend: &str,
    image: &str,
) -> Result<vsn_container::ContainerActionResult, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeManage)?;
    Ok(vsn_container::image_pull(backend, image)?)
}
pub fn container_image_build(
    principal: &Principal,
    request: &vsn_container::ContainerBuildRequest,
) -> Result<vsn_container::ContainerActionResult, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeManage)?;
    let roots = workspace_roots(principal)?;
    let context = vsn_files::resolve_existing(&roots, &request.context)?;
    let mut safe = request.clone();
    safe.context = context;
    if let Some(file) = &request.dockerfile {
        safe.dockerfile = Some(vsn_files::resolve_existing(&roots, file)?);
    }
    Ok(vsn_container::image_build(&safe)?)
}
pub fn container_remove(
    principal: &Principal,
    backend: &str,
    kind: &str,
    target: &str,
    force: bool,
) -> Result<vsn_container::ContainerActionResult, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeManage)?;
    Ok(vsn_container::remove_resource(
        backend, kind, target, force,
    )?)
}
pub fn container_registry_publish(
    principal: &Principal,
    request: &vsn_container::RegistryPushRequest,
) -> Result<vsn_container::RegistryPushResult, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeManage)?;
    Ok(vsn_container::tag_and_push(request)?)
}
pub fn container_exec(
    principal: &Principal,
    request: &vsn_container::ContainerExecRequest,
) -> Result<vsn_container::ContainerActionResult, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeManage)?;
    Ok(vsn_container::container_exec(request)?)
}
pub fn compose_action(
    principal: &Principal,
    backend: &str,
    path: &Path,
    action: &str,
) -> Result<vsn_container::ContainerActionResult, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeManage)?;
    let roots = workspace_roots(principal)?;
    let path = vsn_files::resolve_existing(&roots, path)?;
    Ok(vsn_container::compose_action(backend, &path, action)?)
}

pub fn workspace_roots(principal: &Principal) -> Result<Vec<PathBuf>, CoreError> {
    vsn_policy::require(principal, Permission::ProjectView)?;
    Ok(config()?.workspace_roots)
}
pub fn workspace_add(
    principal: &Principal,
    path: &Path,
) -> Result<vsn_config::AppConfig, CoreError> {
    vsn_policy::require(principal, Permission::ProjectEdit)?;
    Ok(vsn_config::add_workspace_root(path)?)
}
pub fn workspace_remove(
    principal: &Principal,
    path: &Path,
) -> Result<vsn_config::AppConfig, CoreError> {
    vsn_policy::require(principal, Permission::ProjectEdit)?;
    Ok(vsn_config::remove_workspace_root(path)?)
}

pub fn file_list(
    principal: &Principal,
    path: &Path,
) -> Result<Vec<vsn_files::FileEntry>, CoreError> {
    vsn_policy::require(principal, Permission::FilesRead)?;
    let roots = config()?.workspace_roots;
    Ok(vsn_files::list_dir(&roots, path)?)
}
pub fn file_read(principal: &Principal, path: &Path) -> Result<vsn_files::TextFile, CoreError> {
    vsn_policy::require(principal, Permission::FilesRead)?;
    let roots = config()?.workspace_roots;
    Ok(vsn_files::read_text(&roots, path)?)
}
pub fn file_write(
    principal: &Principal,
    path: &Path,
    content: &str,
) -> Result<vsn_files::WriteResult, CoreError> {
    vsn_policy::require(principal, Permission::FilesWrite)?;
    let roots = config()?.workspace_roots;
    let result = vsn_files::write_text(&roots, path, content)?;
    publish(
        "file.written",
        serde_json::json!({"path":result.path,"bytes":result.bytes,"created":result.created}),
    );
    Ok(result)
}
pub fn file_read_binary_chunk(
    principal: &Principal,
    path: &Path,
    offset: u64,
    max_bytes: usize,
) -> Result<vsn_files::BinaryChunk, CoreError> {
    vsn_policy::require(principal, Permission::FilesRead)?;
    let roots = config()?.workspace_roots;
    Ok(vsn_files::read_binary_chunk(
        &roots, path, offset, max_bytes,
    )?)
}
pub fn file_write_binary_chunk(
    principal: &Principal,
    path: &Path,
    transfer_id: &str,
    offset: u64,
    data_b64: &str,
    finalize: bool,
    expected_sha256: Option<&str>,
) -> Result<vsn_files::BinaryWriteResult, CoreError> {
    vsn_policy::require(principal, Permission::FilesWrite)?;
    let roots = config()?.workspace_roots;
    let result = vsn_files::write_binary_chunk(
        &roots,
        path,
        transfer_id,
        offset,
        data_b64,
        finalize,
        expected_sha256,
    )?;
    publish(
        "file.binary_chunk_written",
        serde_json::json!({"path":result.path,"transfer_id":result.transfer_id,"bytes":result.committed_bytes,"complete":result.complete}),
    );
    Ok(result)
}
pub fn file_abort_binary_upload(
    principal: &Principal,
    path: &Path,
    transfer_id: &str,
) -> Result<bool, CoreError> {
    vsn_policy::require(principal, Permission::FilesWrite)?;
    let roots = config()?.workspace_roots;
    Ok(vsn_files::abort_binary_upload(&roots, path, transfer_id)?)
}
pub fn file_binary_upload_status(
    principal: &Principal,
    path: &Path,
    transfer_id: &str,
) -> Result<vsn_files::BinaryUploadStatus, CoreError> {
    vsn_policy::require(principal, Permission::FilesRead)?;
    let roots = config()?.workspace_roots;
    Ok(vsn_files::binary_upload_status(&roots, path, transfer_id)?)
}
pub fn file_conformance(
    principal: &Principal,
) -> Result<vsn_files::FileConformanceReport, CoreError> {
    vsn_policy::require(principal, Permission::FilesRead)?;
    Ok(vsn_files::file_conformance())
}
pub fn file_digest(principal: &Principal, path: &Path) -> Result<vsn_files::FileDigest, CoreError> {
    vsn_policy::require(principal, Permission::FilesRead)?;
    let roots = config()?.workspace_roots;
    Ok(vsn_files::file_digest(&roots, path)?)
}
pub fn file_create_dir(
    principal: &Principal,
    path: &Path,
) -> Result<vsn_files::PathMutationResult, CoreError> {
    vsn_policy::require(principal, Permission::FilesWrite)?;
    let roots = config()?.workspace_roots;
    Ok(vsn_files::create_dir(&roots, path)?)
}
pub fn file_move(
    principal: &Principal,
    source: &Path,
    destination: &Path,
) -> Result<vsn_files::PathMutationResult, CoreError> {
    vsn_policy::require(principal, Permission::FilesWrite)?;
    let roots = config()?.workspace_roots;
    Ok(vsn_files::move_path(&roots, source, destination)?)
}
pub fn file_delete(
    principal: &Principal,
    path: &Path,
    recursive: bool,
) -> Result<vsn_files::PathMutationResult, CoreError> {
    vsn_policy::require(principal, Permission::FilesWrite)?;
    let roots = config()?.workspace_roots;
    Ok(vsn_files::delete_path(&roots, path, recursive)?)
}
pub fn terminal_execute(
    principal: &Principal,
    request: &vsn_terminal::ExecRequest,
) -> Result<vsn_terminal::ExecResult, CoreError> {
    vsn_policy::require(principal, Permission::TerminalExecute)?;
    let roots = config()?.workspace_roots;
    Ok(vsn_terminal::execute(&roots, request)?)
}
pub fn terminal_session_start(
    principal: &Principal,
    request: &vsn_terminal::SessionStartRequest,
) -> Result<vsn_terminal::SessionState, CoreError> {
    vsn_policy::require(principal, Permission::TerminalExecute)?;
    let roots = config()?.workspace_roots;
    Ok(vsn_terminal::start_session(&roots, request)?)
}
pub fn terminal_session_write(
    principal: &Principal,
    id: &str,
    input: &str,
) -> Result<vsn_terminal::SessionState, CoreError> {
    vsn_policy::require(principal, Permission::TerminalExecute)?;
    Ok(vsn_terminal::write_session(id, input)?)
}
pub fn terminal_session_read(
    principal: &Principal,
    id: &str,
    max_bytes: usize,
) -> Result<vsn_terminal::SessionChunk, CoreError> {
    vsn_policy::require(principal, Permission::TerminalView)?;
    Ok(vsn_terminal::read_session(id, max_bytes)?)
}
pub fn terminal_session_read_wait(
    principal: &Principal,
    id: &str,
    max_bytes: usize,
    wait_ms: u64,
) -> Result<vsn_terminal::SessionChunk, CoreError> {
    vsn_policy::require(principal, Permission::TerminalView)?;
    Ok(vsn_terminal::read_session_wait(id, max_bytes, wait_ms)?)
}
pub fn terminal_session_status(
    principal: &Principal,
    id: &str,
) -> Result<vsn_terminal::SessionState, CoreError> {
    vsn_policy::require(principal, Permission::TerminalView)?;
    Ok(vsn_terminal::session_state(id)?)
}
pub fn terminal_session_stop(
    principal: &Principal,
    id: &str,
) -> Result<vsn_terminal::SessionState, CoreError> {
    vsn_policy::require(principal, Permission::TerminalExecute)?;
    Ok(vsn_terminal::stop_session(id)?)
}
pub fn terminal_session_remove(principal: &Principal, id: &str) -> Result<bool, CoreError> {
    vsn_policy::require(principal, Permission::TerminalExecute)?;
    Ok(vsn_terminal::remove_session(id)?)
}
pub fn terminal_session_list(
    principal: &Principal,
) -> Result<Vec<vsn_terminal::SessionState>, CoreError> {
    vsn_policy::require(principal, Permission::TerminalView)?;
    Ok(vsn_terminal::list_sessions()?)
}

pub fn project_detect(
    principal: &Principal,
    path: &Path,
) -> Result<vsn_project::ProjectDetection, CoreError> {
    vsn_policy::require(principal, Permission::ProjectView)?;
    let roots = workspace_roots(principal)?;
    let path = vsn_files::resolve_existing(&roots, path)?;
    Ok(vsn_project::detect(&path)?)
}
pub fn project_dependencies(
    principal: &Principal,
    path: &Path,
) -> Result<vsn_project::ProjectDependencyReport, CoreError> {
    vsn_policy::require(principal, Permission::ProjectView)?;
    let roots = workspace_roots(principal)?;
    let path = vsn_files::resolve_existing(&roots, path)?;
    Ok(vsn_project::dependency_report(&path)?)
}
pub fn project_bootstrap_plan(
    principal: &Principal,
    template: &str,
    path: &Path,
) -> Result<vsn_project::BootstrapPlan, CoreError> {
    vsn_policy::require(principal, Permission::ProjectEdit)?;
    let roots = workspace_roots(principal)?;
    let path = vsn_files::resolve_for_write(&roots, path)?;
    Ok(vsn_project::bootstrap_plan(template, &path)?)
}
pub fn project_bootstrap(
    principal: &Principal,
    template: &str,
    path: &Path,
) -> Result<vsn_project::BootstrapResult, CoreError> {
    vsn_policy::require(principal, Permission::ProjectEdit)?;
    let roots = workspace_roots(principal)?;
    let path = vsn_files::resolve_for_write(&roots, path)?;
    let plan = vsn_project::bootstrap_plan(template, &path)?;
    Ok(vsn_project::execute_bootstrap(&plan)?)
}
pub fn project_provider_conformance(
    principal: &Principal,
) -> Result<vsn_project::ProjectProviderConformanceReport, CoreError> {
    vsn_policy::require(principal, Permission::ProjectView)?;
    Ok(vsn_project::builtin_project_conformance())
}
pub fn project_templates(principal: &Principal) -> Result<Vec<String>, CoreError> {
    vsn_policy::require(principal, Permission::ProjectView)?;
    Ok(vsn_project::builtin_project_templates())
}

pub fn domain_plan(
    principal: &Principal,
    domain: &str,
    port: u16,
    tls: bool,
) -> Result<vsn_network::DomainPlan, CoreError> {
    vsn_policy::require(principal, Permission::NetworkView)?;
    Ok(vsn_network::plan_local_domain(domain, port, tls)?)
}
pub fn domain_apply_hosts(
    principal: &Principal,
    domain: &str,
) -> Result<vsn_network::HostsMutation, CoreError> {
    vsn_policy::require(principal, Permission::NetworkManage)?;
    Ok(vsn_network::apply_hosts_domain(domain, "127.0.0.1")?)
}
pub fn domain_remove_hosts(
    principal: &Principal,
    domain: &str,
) -> Result<vsn_network::HostsMutation, CoreError> {
    vsn_policy::require(principal, Permission::NetworkManage)?;
    Ok(vsn_network::remove_hosts_domain(domain)?)
}
pub fn local_ca_install(principal: &Principal) -> Result<String, CoreError> {
    vsn_policy::require(principal, Permission::NetworkManage)?;
    Ok(vsn_network::mkcert_install_ca()?)
}
pub fn local_certificate(
    principal: &Principal,
    domain: &str,
) -> Result<vsn_network::LocalCertificate, CoreError> {
    vsn_policy::require(principal, Permission::NetworkManage)?;
    let dir = vsn_security::data_dir()?.join("network").join("certs");
    Ok(vsn_network::ensure_mkcert_certificate(domain, &dir)?)
}
pub fn caddy_proxy_config(
    principal: &Principal,
    domain: &str,
    port: u16,
) -> Result<PathBuf, CoreError> {
    vsn_policy::require(principal, Permission::NetworkManage)?;
    let base = vsn_security::data_dir()?.join("network");
    let cert = vsn_network::ensure_mkcert_certificate(domain, &base.join("certs"))?;
    let site = vsn_network::caddy_site(domain, port, Some(cert))?;
    Ok(vsn_network::write_caddyfile(
        &base.join("Caddyfile"),
        &[site],
    )?)
}
pub fn caddy_start(principal: &Principal) -> Result<vsn_system::ManagedProcessState, CoreError> {
    vsn_policy::require(principal, Permission::NetworkManage)?;
    let base = vsn_security::data_dir()?.join("network");
    let caddy = vsn_system::find_executable("caddy")?;
    let spec = vsn_system::ManagedProcessSpec {
        id: "vsn-caddy".into(),
        program: caddy,
        args: vec![
            "run".into(),
            "--config".into(),
            base.join("Caddyfile").display().to_string(),
            "--adapter".into(),
            "caddyfile".into(),
        ],
        cwd: base.clone(),
        env: vec![],
        log_path: base.join("caddy.log"),
    };
    Ok(vsn_system::spawn_managed(&spec, &managed_state_dir()?)?)
}
pub fn caddy_status(principal: &Principal) -> Result<vsn_system::ManagedProcessState, CoreError> {
    vsn_policy::require(principal, Permission::NetworkView)?;
    Ok(vsn_system::managed_process_state(
        "vsn-caddy",
        &managed_state_dir()?,
    )?)
}
pub fn caddy_stop(principal: &Principal) -> Result<vsn_system::ManagedProcessState, CoreError> {
    vsn_policy::require(principal, Permission::NetworkManage)?;
    Ok(vsn_system::stop_managed(
        "vsn-caddy",
        &managed_state_dir()?,
    )?)
}
pub fn caddy_restart(principal: &Principal) -> Result<vsn_system::ManagedProcessState, CoreError> {
    vsn_policy::require(principal, Permission::NetworkManage)?;
    let _ = vsn_system::stop_managed("vsn-caddy", &managed_state_dir()?);
    caddy_start(principal)
}
pub fn dns_plan(
    principal: &Principal,
    listen: &str,
) -> Result<vsn_network::DnsResolverPlan, CoreError> {
    vsn_policy::require(principal, Permission::NetworkView)?;
    Ok(vsn_network::dns_resolver_plan(listen)?)
}
pub fn dns_os_status(principal: &Principal) -> Result<vsn_network::OsResolverStatus, CoreError> {
    vsn_policy::require(principal, Permission::NetworkView)?;
    Ok(vsn_network::os_resolver_status()?)
}
pub fn dns_os_apply(
    principal: &Principal,
    listen: &str,
) -> Result<vsn_network::OsResolverStatus, CoreError> {
    vsn_policy::require(principal, Permission::NetworkManage)?;
    Ok(vsn_network::apply_os_test_resolver(listen)?)
}
pub fn dns_os_remove(principal: &Principal) -> Result<vsn_network::OsResolverStatus, CoreError> {
    vsn_policy::require(principal, Permission::NetworkManage)?;
    Ok(vsn_network::remove_os_test_resolver()?)
}
pub fn dns_start(
    principal: &Principal,
    listen: &str,
) -> Result<vsn_system::ManagedProcessState, CoreError> {
    vsn_policy::require(principal, Permission::NetworkManage)?;
    let plan = vsn_network::dns_resolver_plan(listen)?;
    let exe = std::env::current_exe()
        .map_err(|e| CoreError::Rejected(format!("Agent executable unavailable: {e}")))?;
    let base = vsn_security::data_dir()?.join("network");
    std::fs::create_dir_all(&base).map_err(|e| CoreError::Rejected(e.to_string()))?;
    let spec = vsn_system::ManagedProcessSpec {
        id: "vsn-dns".into(),
        program: exe,
        args: vec!["dns-server".into(), "--listen".into(), plan.listen],
        cwd: base.clone(),
        env: vec![],
        log_path: base.join("dns.log"),
    };
    Ok(vsn_system::spawn_managed(&spec, &managed_state_dir()?)?)
}
pub fn dns_status(principal: &Principal) -> Result<vsn_system::ManagedProcessState, CoreError> {
    vsn_policy::require(principal, Permission::NetworkView)?;
    Ok(vsn_system::managed_process_state(
        "vsn-dns",
        &managed_state_dir()?,
    )?)
}
pub fn dns_stop(principal: &Principal) -> Result<vsn_system::ManagedProcessState, CoreError> {
    vsn_policy::require(principal, Permission::NetworkManage)?;
    Ok(vsn_system::stop_managed("vsn-dns", &managed_state_dir()?)?)
}
pub fn network_conformance(
    principal: &Principal,
) -> Result<vsn_network::NetworkConformanceReport, CoreError> {
    vsn_policy::require(principal, Permission::NetworkView)?;
    Ok(vsn_network::network_conformance())
}
pub fn caddy_reload(principal: &Principal) -> Result<vsn_network::ProxyReloadResult, CoreError> {
    vsn_policy::require(principal, Permission::NetworkManage)?;
    let path = vsn_security::data_dir()?.join("network").join("Caddyfile");
    Ok(vsn_network::reload_caddyfile(&path)?)
}

pub fn database_workspace(
    principal: &Principal,
    model: vsn_database::DataModel,
) -> Result<Vec<&'static str>, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database::workspace_for_model(model))
}
pub fn database_studio_conformance(
    principal: &Principal,
) -> Result<vsn_database::DatabaseStudioConformanceReport, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database::database_studio_conformance())
}
pub fn database_ui_schema(
    principal: &Principal,
    entity: &vsn_database::EntityMeta,
    caps: &vsn_database::CapabilitySet,
) -> Result<vsn_database::EntityUiSchema, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database::generate_ui_schema(entity, caps))
}
pub fn database_model_analyze(
    principal: &Principal,
    request: &vsn_database::AdvancedModelRequest,
) -> Result<vsn_database::AdvancedModelAnalysis, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database::analyze_advanced_model(request)?)
}
pub fn remote_database_conformance(
    principal: &Principal,
) -> Result<vsn_database::RemoteDatabaseConformanceReport, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database::validate_remote_database_capabilities())
}
pub fn sqlite_inspect(principal: &Principal, path: &Path) -> Result<serde_json::Value, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_sqlite::inspect(path)?)
}
pub fn sqlite_query(
    principal: &Principal,
    path: &Path,
    statement: &str,
) -> Result<serde_json::Value, CoreError> {
    use vsn_database::DatabaseProvider;
    vsn_policy::require(principal, Permission::DatabaseQuery)?;
    let provider = vsn_database_sqlite::SqliteProvider::open(path, true)?;
    Ok(provider.query(statement, &serde_json::Value::Null)?)
}
pub fn sqlite_browse(
    principal: &Principal,
    path: &Path,
    entity: &str,
    request: &vsn_database::BrowseRequest,
) -> Result<vsn_database::BrowsePage, CoreError> {
    use vsn_database::DatabaseProvider;
    vsn_policy::require(principal, Permission::DatabaseView)?;
    let provider = vsn_database_sqlite::SqliteProvider::open(path, true)?;
    Ok(provider.browse(None, entity, request)?)
}
pub fn sqlite_indexes(
    principal: &Principal,
    path: &Path,
    entity: &str,
) -> Result<Vec<vsn_database::IndexMeta>, CoreError> {
    use vsn_database::DatabaseProvider;
    vsn_policy::require(principal, Permission::DatabaseView)?;
    let provider = vsn_database_sqlite::SqliteProvider::open(path, true)?;
    Ok(provider.list_indexes(None, entity)?)
}
pub fn sqlite_relations(
    principal: &Principal,
    path: &Path,
    entity: &str,
) -> Result<Vec<vsn_database::RelationMeta>, CoreError> {
    use vsn_database::DatabaseProvider;
    vsn_policy::require(principal, Permission::DatabaseView)?;
    let provider = vsn_database_sqlite::SqliteProvider::open(path, true)?;
    Ok(provider.list_relations(None, entity)?)
}
pub fn sqlite_statistics(
    principal: &Principal,
    path: &Path,
    entity: &str,
) -> Result<vsn_database::EntityStatistics, CoreError> {
    use vsn_database::DatabaseProvider;
    vsn_policy::require(principal, Permission::DatabaseView)?;
    let provider = vsn_database_sqlite::SqliteProvider::open(path, true)?;
    Ok(provider.statistics(None, entity)?)
}
pub fn sqlite_insert(
    principal: &Principal,
    path: &Path,
    entity: &str,
    request: &vsn_database::MutationRequest,
) -> Result<vsn_database::MutationResult, CoreError> {
    use vsn_database::DatabaseProvider;
    vsn_policy::require(principal, Permission::DatabaseWrite)?;
    let provider = vsn_database_sqlite::SqliteProvider::open(path, false)?;
    Ok(provider.insert(None, entity, request)?)
}
pub fn sqlite_update(
    principal: &Principal,
    path: &Path,
    entity: &str,
    request: &vsn_database::MutationRequest,
) -> Result<vsn_database::MutationResult, CoreError> {
    use vsn_database::DatabaseProvider;
    vsn_policy::require(principal, Permission::DatabaseWrite)?;
    let provider = vsn_database_sqlite::SqliteProvider::open(path, false)?;
    Ok(provider.update(None, entity, request)?)
}
pub fn sqlite_delete(
    principal: &Principal,
    path: &Path,
    entity: &str,
    request: &vsn_database::MutationRequest,
) -> Result<vsn_database::MutationResult, CoreError> {
    use vsn_database::DatabaseProvider;
    vsn_policy::require(principal, Permission::DatabaseWrite)?;
    let provider = vsn_database_sqlite::SqliteProvider::open(path, false)?;
    Ok(provider.delete(None, entity, request)?)
}

pub fn database_cli_detect(
    principal: &Principal,
) -> Result<Vec<vsn_database_cli::ClientDetection>, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_cli::detect_clients())
}
fn validate_database_credential_file(
    spec: &vsn_database_cli::ConnectionSpec,
) -> Result<(), CoreError> {
    let Some(path) = spec.credential_file.as_ref() else {
        return Ok(());
    };
    let requested = path
        .canonicalize()
        .map_err(|e| CoreError::Rejected(format!("database credential file unavailable: {e}")))?;
    let data = vsn_security::data_dir()?;
    if let Ok(base) = data.canonicalize() {
        if requested.starts_with(&base) {
            return Ok(());
        }
    }
    for root in config()?.workspace_roots {
        if let Ok(base) = root.canonicalize() {
            if requested.starts_with(base) {
                return Ok(());
            }
        }
    }
    Err(CoreError::Rejected("database credential files must be inside a configured workspace or VSN-owned data directory".into()))
}
pub fn database_cli_inspect(
    principal: &Principal,
    spec: &vsn_database_cli::ConnectionSpec,
) -> Result<vsn_database_cli::Inspection, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    validate_database_credential_file(spec)?;
    Ok(vsn_database_cli::inspect(spec)?)
}
pub fn database_cli_query(
    principal: &Principal,
    spec: &vsn_database_cli::ConnectionSpec,
    statement: &str,
) -> Result<vsn_database_cli::QueryGrid, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseQuery)?;
    validate_database_credential_file(spec)?;
    Ok(vsn_database_cli::query_read_only(spec, statement)?)
}
fn database_job_state_dir() -> Result<PathBuf, CoreError> {
    Ok(vsn_security::data_dir()?.join("database-query-jobs"))
}
pub fn database_cli_job_start(
    principal: &Principal,
    spec: &vsn_database_cli::ConnectionSpec,
    statement: &str,
) -> Result<vsn_database_cli::QueryJobStatus, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseQuery)?;
    validate_database_credential_file(spec)?;
    Ok(vsn_database_cli::start_read_query_job(
        spec.clone(),
        statement.to_string(),
        database_job_state_dir()?,
    )?)
}
pub fn database_cli_job_status(
    principal: &Principal,
    job_id: &str,
) -> Result<vsn_database_cli::QueryJobStatus, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_cli::query_job_status(
        job_id,
        &database_job_state_dir()?,
    )?)
}
pub fn database_cli_jobs(
    principal: &Principal,
) -> Result<Vec<vsn_database_cli::QueryJobStatus>, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_cli::list_query_jobs(
        &database_job_state_dir()?
    )?)
}
pub fn database_cli_job_cancel(
    principal: &Principal,
    job_id: &str,
) -> Result<vsn_database_cli::QueryJobStatus, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseQuery)?;
    Ok(vsn_database_cli::cancel_query_job(
        job_id,
        &database_job_state_dir()?,
    )?)
}
pub fn database_cli_job_output(
    principal: &Principal,
    job_id: &str,
    offset: u64,
    max_bytes: usize,
) -> Result<vsn_database_cli::QueryJobOutputChunk, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_cli::read_query_job_output(
        job_id,
        &database_job_state_dir()?,
        offset,
        max_bytes,
    )?)
}
pub fn database_cli_job_output_remove(
    principal: &Principal,
    job_id: &str,
) -> Result<bool, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseQuery)?;
    Ok(vsn_database_cli::remove_query_job_artifact(
        job_id,
        &database_job_state_dir()?,
    )?)
}

fn terminal_scrollback_dir() -> Result<PathBuf, CoreError> {
    Ok(vsn_security::data_dir()?.join("terminal-scrollback"))
}
pub fn terminal_pty_start(
    principal: &Principal,
    request: &vsn_terminal::PtySessionStartRequest,
) -> Result<vsn_terminal::PtySessionState, CoreError> {
    vsn_policy::require(principal, Permission::TerminalExecute)?;
    Ok(vsn_terminal::start_pty_session_with_scrollback(
        &workspace_roots(principal)?,
        request,
        &terminal_scrollback_dir()?,
    )?)
}
pub fn terminal_pty_write(
    principal: &Principal,
    id: &str,
    input: &str,
) -> Result<vsn_terminal::PtySessionState, CoreError> {
    vsn_policy::require(principal, Permission::TerminalExecute)?;
    Ok(vsn_terminal::write_pty_session(id, input)?)
}
pub fn terminal_pty_read(
    principal: &Principal,
    id: &str,
    max: usize,
) -> Result<vsn_terminal::PtySessionChunk, CoreError> {
    vsn_policy::require(principal, Permission::TerminalView)?;
    Ok(vsn_terminal::read_pty_session(id, max)?)
}
pub fn terminal_pty_read_wait(
    principal: &Principal,
    id: &str,
    max: usize,
    wait_ms: u64,
) -> Result<vsn_terminal::PtySessionChunk, CoreError> {
    vsn_policy::require(principal, Permission::TerminalView)?;
    Ok(vsn_terminal::read_pty_session_wait(id, max, wait_ms)?)
}
pub fn terminal_pty_resize(
    principal: &Principal,
    id: &str,
    rows: u16,
    cols: u16,
) -> Result<vsn_terminal::PtySessionState, CoreError> {
    vsn_policy::require(principal, Permission::TerminalExecute)?;
    Ok(vsn_terminal::resize_pty_session(id, rows, cols)?)
}
pub fn terminal_pty_status(
    principal: &Principal,
    id: &str,
) -> Result<vsn_terminal::PtySessionState, CoreError> {
    vsn_policy::require(principal, Permission::TerminalView)?;
    Ok(vsn_terminal::pty_session_state(id)?)
}
pub fn terminal_pty_stop(
    principal: &Principal,
    id: &str,
) -> Result<vsn_terminal::PtySessionState, CoreError> {
    vsn_policy::require(principal, Permission::TerminalExecute)?;
    Ok(vsn_terminal::stop_pty_session(id)?)
}
pub fn terminal_pty_remove(principal: &Principal, id: &str) -> Result<bool, CoreError> {
    vsn_policy::require(principal, Permission::TerminalExecute)?;
    Ok(vsn_terminal::remove_pty_session(id)?)
}
pub fn terminal_pty_list(
    principal: &Principal,
) -> Result<Vec<vsn_terminal::PtySessionState>, CoreError> {
    vsn_policy::require(principal, Permission::TerminalView)?;
    Ok(vsn_terminal::list_pty_sessions()?)
}
pub fn terminal_pty_scrollback_list(
    principal: &Principal,
) -> Result<Vec<vsn_terminal::PtyScrollbackInfo>, CoreError> {
    vsn_policy::require(principal, Permission::TerminalView)?;
    Ok(vsn_terminal::list_pty_scrollback(
        &terminal_scrollback_dir()?,
    )?)
}
pub fn terminal_pty_scrollback_read(
    principal: &Principal,
    id: &str,
    offset: u64,
    max: usize,
) -> Result<vsn_terminal::PtyScrollbackChunk, CoreError> {
    vsn_policy::require(principal, Permission::TerminalView)?;
    Ok(vsn_terminal::read_pty_scrollback(
        &terminal_scrollback_dir()?,
        id,
        offset,
        max,
    )?)
}
pub fn terminal_pty_scrollback_remove(principal: &Principal, id: &str) -> Result<bool, CoreError> {
    vsn_policy::require(principal, Permission::TerminalExecute)?;
    Ok(vsn_terminal::remove_pty_scrollback(
        &terminal_scrollback_dir()?,
        id,
    )?)
}
pub fn terminal_conformance(
    principal: &Principal,
) -> Result<vsn_terminal::TerminalConformanceReport, CoreError> {
    vsn_policy::require(principal, Permission::TerminalView)?;
    Ok(vsn_terminal::terminal_conformance())
}
pub fn terminal_pty_recovery_list(
    principal: &Principal,
) -> Result<Vec<vsn_terminal::PtyRecoveryInfo>, CoreError> {
    vsn_policy::require(principal, Permission::TerminalView)?;
    Ok(vsn_terminal::list_pty_recovery(&terminal_scrollback_dir()?)?)
}
pub fn terminal_pty_recovery_remove(principal: &Principal, id: &str) -> Result<bool, CoreError> {
    vsn_policy::require(principal, Permission::TerminalExecute)?;
    Ok(vsn_terminal::remove_pty_recovery(
        &terminal_scrollback_dir()?,
        id,
    )?)
}

pub fn postgres_native_inspect(
    principal: &Principal,
    spec: &vsn_database_native::PostgresConnection,
) -> Result<vsn_database_native::PostgresInspection, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::postgres_inspect(spec)?)
}
pub fn postgres_native_browse(
    principal: &Principal,
    spec: &vsn_database_native::PostgresConnection,
    schema: &str,
    table: &str,
    limit: u32,
    offset: u64,
) -> Result<vsn_database_native::NativeGrid, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::postgres_browse(
        spec, schema, table, limit, offset,
    )?)
}
pub fn postgres_native_query(
    principal: &Principal,
    spec: &vsn_database_native::PostgresConnection,
    sql: &str,
) -> Result<vsn_database_native::NativeGrid, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseQuery)?;
    Ok(vsn_database_native::postgres_read_query(spec, sql)?)
}
pub fn postgres_native_job_start(
    principal: &Principal,
    spec: &vsn_database_native::PostgresConnection,
    sql: &str,
) -> Result<vsn_database_native::NativePostgresJobStatus, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseQuery)?;
    Ok(vsn_database_native::postgres_job_start(spec, sql)?)
}
pub fn postgres_native_job_status(
    principal: &Principal,
    job_id: &str,
) -> Result<vsn_database_native::NativePostgresJobStatus, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::postgres_job_status(job_id)?)
}
pub fn postgres_native_job_list(
    principal: &Principal,
) -> Result<Vec<vsn_database_native::NativePostgresJobStatus>, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::postgres_job_list()?)
}
pub fn postgres_native_job_cancel(
    principal: &Principal,
    job_id: &str,
) -> Result<vsn_database_native::NativePostgresJobStatus, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseQuery)?;
    Ok(vsn_database_native::postgres_job_cancel(job_id)?)
}
pub fn postgres_native_txn_start(
    principal: &Principal,
    spec: &vsn_database_native::PostgresConnection,
    ttl_seconds: u64,
) -> Result<vsn_database_native::NativePostgresTransactionState, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseQuery)?;
    Ok(vsn_database_native::postgres_read_transaction_start(
        spec,
        ttl_seconds,
    )?)
}
pub fn postgres_native_txn_query(
    principal: &Principal,
    transaction_id: &str,
    sql: &str,
) -> Result<vsn_database_native::NativeGrid, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseQuery)?;
    Ok(vsn_database_native::postgres_read_transaction_query(
        transaction_id,
        sql,
    )?)
}
pub fn postgres_native_txn_status(
    principal: &Principal,
    transaction_id: &str,
) -> Result<vsn_database_native::NativePostgresTransactionState, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::postgres_read_transaction_status(
        transaction_id,
    )?)
}
pub fn postgres_native_txn_close(
    principal: &Principal,
    transaction_id: &str,
    commit: bool,
) -> Result<vsn_database_native::NativePostgresTransactionState, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseQuery)?;
    Ok(vsn_database_native::postgres_read_transaction_close(
        transaction_id,
        commit,
    )?)
}
pub fn postgres_native_insert(
    principal: &Principal,
    spec: &vsn_database_native::PostgresConnection,
    schema: &str,
    table: &str,
    request: &vsn_database::MutationRequest,
) -> Result<vsn_database::MutationResult, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseWrite)?;
    Ok(vsn_database_native::postgres_insert(
        spec, schema, table, request,
    )?)
}
pub fn postgres_native_update(
    principal: &Principal,
    spec: &vsn_database_native::PostgresConnection,
    schema: &str,
    table: &str,
    request: &vsn_database::MutationRequest,
) -> Result<vsn_database::MutationResult, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseWrite)?;
    Ok(vsn_database_native::postgres_update(
        spec, schema, table, request,
    )?)
}
pub fn postgres_native_delete(
    principal: &Principal,
    spec: &vsn_database_native::PostgresConnection,
    schema: &str,
    table: &str,
    request: &vsn_database::MutationRequest,
) -> Result<vsn_database::MutationResult, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseWrite)?;
    Ok(vsn_database_native::postgres_delete(
        spec, schema, table, request,
    )?)
}
pub fn postgres_native_indexes(
    principal: &Principal,
    spec: &vsn_database_native::PostgresConnection,
    schema: &str,
    table: &str,
) -> Result<vsn_database_native::NativeGrid, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::postgres_indexes(spec, schema, table)?)
}
pub fn postgres_native_relations(
    principal: &Principal,
    spec: &vsn_database_native::PostgresConnection,
    schema: &str,
    table: &str,
) -> Result<vsn_database_native::NativeGrid, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::postgres_relations(
        spec, schema, table,
    )?)
}
pub fn postgres_native_stats(
    principal: &Principal,
    spec: &vsn_database_native::PostgresConnection,
    schema: &str,
    table: &str,
) -> Result<vsn_database_native::NativeTableStats, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::postgres_stats(spec, schema, table)?)
}
pub fn postgres_tls_inspect(
    principal: &Principal,
    spec: &vsn_database_native::PostgresTlsConnection,
) -> Result<vsn_database_native::PostgresInspection, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::postgres_tls_inspect(spec)?)
}
pub fn postgres_tls_browse(
    principal: &Principal,
    spec: &vsn_database_native::PostgresTlsConnection,
    schema: &str,
    table: &str,
    limit: u32,
    offset: u64,
) -> Result<vsn_database_native::NativeGrid, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::postgres_tls_browse(
        spec, schema, table, limit, offset,
    )?)
}
pub fn postgres_tls_query(
    principal: &Principal,
    spec: &vsn_database_native::PostgresTlsConnection,
    sql: &str,
) -> Result<vsn_database_native::NativeGrid, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseQuery)?;
    Ok(vsn_database_native::postgres_tls_read_query(spec, sql)?)
}
pub fn mysql_native_inspect(
    principal: &Principal,
    spec: &vsn_database_native::MySqlConnection,
) -> Result<vsn_database_native::MySqlInspection, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::mysql_inspect(spec)?)
}
pub fn mysql_native_browse(
    principal: &Principal,
    spec: &vsn_database_native::MySqlConnection,
    database: &str,
    table: &str,
    limit: u32,
    offset: u64,
) -> Result<vsn_database_native::NativeGrid, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::mysql_browse(
        spec, database, table, limit, offset,
    )?)
}
pub fn mysql_native_query(
    principal: &Principal,
    spec: &vsn_database_native::MySqlConnection,
    sql: &str,
) -> Result<vsn_database_native::NativeGrid, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseQuery)?;
    Ok(vsn_database_native::mysql_read_query(spec, sql)?)
}
pub fn mysql_native_insert(
    principal: &Principal,
    spec: &vsn_database_native::MySqlConnection,
    database: &str,
    table: &str,
    request: &vsn_database::MutationRequest,
) -> Result<vsn_database::MutationResult, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseWrite)?;
    Ok(vsn_database_native::mysql_insert(
        spec, database, table, request,
    )?)
}
pub fn mysql_native_update(
    principal: &Principal,
    spec: &vsn_database_native::MySqlConnection,
    database: &str,
    table: &str,
    request: &vsn_database::MutationRequest,
) -> Result<vsn_database::MutationResult, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseWrite)?;
    Ok(vsn_database_native::mysql_update(
        spec, database, table, request,
    )?)
}
pub fn mysql_native_delete(
    principal: &Principal,
    spec: &vsn_database_native::MySqlConnection,
    database: &str,
    table: &str,
    request: &vsn_database::MutationRequest,
) -> Result<vsn_database::MutationResult, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseWrite)?;
    Ok(vsn_database_native::mysql_delete(
        spec, database, table, request,
    )?)
}
pub fn mysql_native_indexes(
    principal: &Principal,
    spec: &vsn_database_native::MySqlConnection,
    database: &str,
    table: &str,
) -> Result<vsn_database_native::NativeGrid, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::mysql_indexes(spec, database, table)?)
}
pub fn mysql_native_relations(
    principal: &Principal,
    spec: &vsn_database_native::MySqlConnection,
    database: &str,
    table: &str,
) -> Result<vsn_database_native::NativeGrid, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::mysql_relations(spec, database, table)?)
}
pub fn mysql_native_stats(
    principal: &Principal,
    spec: &vsn_database_native::MySqlConnection,
    database: &str,
    table: &str,
) -> Result<vsn_database_native::NativeTableStats, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::mysql_stats(spec, database, table)?)
}
pub fn mysql_tls_inspect(
    principal: &Principal,
    spec: &vsn_database_native::MySqlTlsConnection,
) -> Result<vsn_database_native::MySqlInspection, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::mysql_tls_inspect(spec)?)
}
pub fn mysql_tls_browse(
    principal: &Principal,
    spec: &vsn_database_native::MySqlTlsConnection,
    database: &str,
    table: &str,
    limit: u32,
    offset: u64,
) -> Result<vsn_database_native::NativeGrid, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::mysql_tls_browse(
        spec, database, table, limit, offset,
    )?)
}
pub fn mysql_tls_query(
    principal: &Principal,
    spec: &vsn_database_native::MySqlTlsConnection,
    sql: &str,
) -> Result<vsn_database_native::NativeGrid, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseQuery)?;
    Ok(vsn_database_native::mysql_tls_read_query(spec, sql)?)
}
pub fn mongo_native_inspect(
    principal: &Principal,
    spec: &vsn_database_native::MongoConnection,
    database: Option<&str>,
) -> Result<vsn_database_native::MongoInspection, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::mongo_inspect(spec, database)?)
}
pub fn mongo_native_browse(
    principal: &Principal,
    spec: &vsn_database_native::MongoConnection,
    database: &str,
    collection: &str,
    limit: u32,
    offset: u64,
    filter: Option<serde_json::Value>,
) -> Result<vsn_database_native::NativeGrid, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::mongo_browse(
        spec, database, collection, limit, offset, filter,
    )?)
}
pub fn mongo_native_insert(
    principal: &Principal,
    spec: &vsn_database_native::MongoConnection,
    database: &str,
    collection: &str,
    values: &std::collections::BTreeMap<String, serde_json::Value>,
) -> Result<vsn_database_native::MongoMutationResult, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseWrite)?;
    Ok(vsn_database_native::mongo_insert(
        spec, database, collection, values,
    )?)
}
pub fn mongo_native_update(
    principal: &Principal,
    spec: &vsn_database_native::MongoConnection,
    database: &str,
    collection: &str,
    request: &vsn_database::MutationRequest,
) -> Result<vsn_database_native::MongoMutationResult, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseWrite)?;
    Ok(vsn_database_native::mongo_update(
        spec, database, collection, request,
    )?)
}
pub fn mongo_native_delete(
    principal: &Principal,
    spec: &vsn_database_native::MongoConnection,
    database: &str,
    collection: &str,
    filter: &std::collections::BTreeMap<String, serde_json::Value>,
) -> Result<vsn_database_native::MongoMutationResult, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseWrite)?;
    Ok(vsn_database_native::mongo_delete(
        spec, database, collection, filter,
    )?)
}
pub fn mongo_native_indexes(
    principal: &Principal,
    spec: &vsn_database_native::MongoConnection,
    database: &str,
    collection: &str,
) -> Result<vsn_database_native::NativeGrid, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::mongo_indexes(
        spec, database, collection,
    )?)
}
pub fn mongo_native_stats(
    principal: &Principal,
    spec: &vsn_database_native::MongoConnection,
    database: &str,
    collection: &str,
) -> Result<vsn_database_native::NativeTableStats, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::mongo_stats(
        spec, database, collection,
    )?)
}
pub fn redis_native_inspect(
    principal: &Principal,
    spec: &vsn_database_native::RedisConnection,
) -> Result<vsn_database_native::RedisInspection, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::redis_inspect(spec)?)
}
pub fn redis_native_get(
    principal: &Principal,
    spec: &vsn_database_native::RedisConnection,
    key: &str,
) -> Result<serde_json::Value, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseView)?;
    Ok(vsn_database_native::redis_get(spec, key)?)
}
pub fn redis_native_set(
    principal: &Principal,
    spec: &vsn_database_native::RedisConnection,
    key: &str,
    value: &str,
    ttl_seconds: Option<u64>,
) -> Result<serde_json::Value, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseWrite)?;
    Ok(vsn_database_native::redis_set_string(
        spec,
        key,
        value,
        ttl_seconds,
    )?)
}
pub fn redis_native_delete(
    principal: &Principal,
    spec: &vsn_database_native::RedisConnection,
    key: &str,
) -> Result<serde_json::Value, CoreError> {
    vsn_policy::require(principal, Permission::DatabaseWrite)?;
    Ok(vsn_database_native::redis_delete(spec, key)?)
}

pub fn ai_plan(
    principal: &Principal,
    intent: &vsn_ai::StructuredIntent,
) -> Result<vsn_ai::ToolPlan, CoreError> {
    // Planning itself is read-only. Execution still goes through normal command/permission boundaries.
    vsn_policy::require(principal, Permission::ProjectView)?;
    vsn_ai::plan(intent).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn ai_conformance(principal: &Principal) -> Result<vsn_ai::AiConformanceReport, CoreError> {
    vsn_policy::require(principal, Permission::MachineView)?;
    Ok(vsn_ai::conformance())
}
pub fn ai_validate_model_output(
    principal: &Principal,
    adapter: &vsn_ai::ModelAdapterDescriptor,
    bytes: &[u8],
) -> Result<vsn_ai::ModelOutputValidation, CoreError> {
    vsn_policy::require(principal, Permission::MachineView)?;
    Ok(vsn_ai::validate_model_output(adapter, bytes))
}
pub fn ai_telemetry_summary(
    principal: &Principal,
) -> Result<vsn_ai::AiTelemetrySummary, CoreError> {
    vsn_policy::require(principal, Permission::MachineView)?;
    let path = vsn_security::data_dir()?.join("ai").join("telemetry.jsonl");
    vsn_ai::telemetry_summary(&path).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn ai_record_telemetry(
    principal: &Principal,
    record: &vsn_ai::AiTelemetryRecord,
) -> Result<(), CoreError> {
    vsn_policy::require(principal, Permission::ProjectView)?;
    let path = vsn_security::data_dir()?.join("ai").join("telemetry.jsonl");
    vsn_ai::append_telemetry(&path, record).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn ai_capabilities(principal: &Principal) -> Result<vsn_ai::AiCapabilityReport, CoreError> {
    vsn_policy::require(principal, Permission::MachineView)?;
    Ok(vsn_ai::capabilities())
}
pub fn ai_validate_plan(
    principal: &Principal,
    plan: &vsn_ai::ToolPlan,
) -> Result<vsn_ai::CandidatePlanValidation, CoreError> {
    vsn_policy::require(principal, Permission::MachineView)?;
    Ok(vsn_ai::validate_candidate_plan(plan))
}
pub fn ai_evaluate(
    principal: &Principal,
    path: &Path,
) -> Result<vsn_ai::EvaluationReport, CoreError> {
    vsn_policy::require(principal, Permission::ProjectView)?;
    let bytes = std::fs::read(path)
        .map_err(|e| CoreError::Rejected(format!("AI evaluation file read failed: {e}")))?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(CoreError::Rejected(
            "AI evaluation file exceeds 4 MiB".into(),
        ));
    }
    vsn_ai::evaluate_json(&bytes).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn cloud_workspace_plan(
    principal: &Principal,
    spec: &vsn_cloud::WorkspaceSpec,
) -> Result<vsn_cloud::ProvisionPlan, CoreError> {
    vsn_policy::require(principal, Permission::ProjectView)?;
    vsn_cloud::generic_ssh_plan(spec).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn cloud_ssh_preflight(
    principal: &Principal,
    target: &vsn_cloud::ExistingSshTarget,
) -> Result<vsn_cloud::SshPreflightResult, CoreError> {
    vsn_policy::require(principal, Permission::RemoteManage)?;
    vsn_cloud::ssh_preflight(target).map_err(|e| CoreError::Rejected(e.to_string()))
}

pub fn cloud_ssh_workspace_prepare(
    principal: &Principal,
    request: &vsn_cloud::ExistingSshWorkspaceRequest,
) -> Result<vsn_cloud::ExistingSshWorkspaceResult, CoreError> {
    vsn_policy::require(principal, Permission::RemoteManage)?;
    vsn_cloud::ssh_workspace_prepare(request).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn cloud_ssh_workspace_status(
    principal: &Principal,
    request: &vsn_cloud::ExistingSshWorkspaceRequest,
) -> Result<vsn_cloud::ExistingSshWorkspaceResult, CoreError> {
    vsn_policy::require(principal, Permission::RemoteView)?;
    vsn_cloud::ssh_workspace_status(request).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn cloud_ssh_workspace_remove_empty(
    principal: &Principal,
    request: &vsn_cloud::ExistingSshWorkspaceRequest,
) -> Result<vsn_cloud::ExistingSshWorkspaceResult, CoreError> {
    vsn_policy::require(principal, Permission::RemoteManage)?;
    vsn_cloud::ssh_workspace_remove_empty(request).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn cloud_ssh_release_upload(
    principal: &Principal,
    request: &vsn_cloud::SshReleaseUploadRequest,
) -> Result<vsn_cloud::SshReleaseResult, CoreError> {
    vsn_policy::require(principal, Permission::RemoteManage)?;
    vsn_cloud::ssh_release_upload(request).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn cloud_ssh_release_activate(
    principal: &Principal,
    request: &vsn_cloud::SshReleasePointerRequest,
) -> Result<vsn_cloud::SshReleaseStatus, CoreError> {
    vsn_policy::require(principal, Permission::RemoteManage)?;
    vsn_cloud::ssh_release_activate(request).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn cloud_ssh_release_status(
    principal: &Principal,
    target: &vsn_cloud::ExistingSshTarget,
    workspace_name: &str,
) -> Result<vsn_cloud::SshReleaseStatus, CoreError> {
    vsn_policy::require(principal, Permission::RemoteView)?;
    vsn_cloud::ssh_release_status(target, workspace_name)
        .map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn cloud_ssh_release_rollback(
    principal: &Principal,
    target: &vsn_cloud::ExistingSshTarget,
    workspace_name: &str,
) -> Result<vsn_cloud::SshReleaseStatus, CoreError> {
    vsn_policy::require(principal, Permission::RemoteManage)?;
    vsn_cloud::ssh_release_rollback(target, workspace_name)
        .map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn cloud_ssh_release_healthcheck(
    principal: &Principal,
    request: &vsn_cloud::SshReleaseHealthRequest,
) -> Result<vsn_cloud::SshReleaseHealthResult, CoreError> {
    vsn_policy::require(principal, Permission::RemoteManage)?;
    vsn_cloud::ssh_release_healthcheck(request).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn cloud_cli_detect(
    principal: &Principal,
) -> Result<Vec<(vsn_cloud::CloudCliProvider, bool)>, CoreError> {
    vsn_policy::require(principal, Permission::RemoteView)?;
    Ok(vsn_cloud::cloud_cli_detect())
}
pub fn cloud_cli_create(
    principal: &Principal,
    request: &vsn_cloud::CloudCliCreateRequest,
) -> Result<vsn_cloud::CloudCliResult, CoreError> {
    vsn_policy::require(principal, Permission::RemoteManage)?;
    vsn_cloud::cloud_cli_create(request).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn cloud_cli_status(
    principal: &Principal,
    request: &vsn_cloud::CloudCliInstanceRef,
) -> Result<vsn_cloud::CloudCliResult, CoreError> {
    vsn_policy::require(principal, Permission::RemoteView)?;
    vsn_cloud::cloud_cli_status(request).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn cloud_cli_start(
    principal: &Principal,
    request: &vsn_cloud::CloudCliInstanceRef,
) -> Result<vsn_cloud::CloudCliResult, CoreError> {
    vsn_policy::require(principal, Permission::RemoteManage)?;
    vsn_cloud::cloud_cli_start(request).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn cloud_cli_stop(
    principal: &Principal,
    request: &vsn_cloud::CloudCliInstanceRef,
) -> Result<vsn_cloud::CloudCliResult, CoreError> {
    vsn_policy::require(principal, Permission::RemoteManage)?;
    vsn_cloud::cloud_cli_stop(request).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn cloud_cli_snapshot(
    principal: &Principal,
    request: &vsn_cloud::CloudCliSnapshotRequest,
) -> Result<vsn_cloud::CloudCliResult, CoreError> {
    vsn_policy::require(principal, Permission::RemoteManage)?;
    vsn_cloud::cloud_cli_snapshot(request).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn cloud_cli_clone(
    principal: &Principal,
    request: &vsn_cloud::CloudCliCloneRequest,
) -> Result<vsn_cloud::CloudCliResult, CoreError> {
    vsn_policy::require(principal, Permission::RemoteManage)?;
    vsn_cloud::cloud_cli_clone(request).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn cloud_cli_copy_image(
    principal: &Principal,
    request: &vsn_cloud::CloudCliImageCopyRequest,
) -> Result<vsn_cloud::CloudCliResult, CoreError> {
    vsn_policy::require(principal, Permission::RemoteManage)?;
    vsn_cloud::cloud_cli_copy_image(request).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn cloud_cli_copy_status(
    principal: &Principal,
    request: &vsn_cloud::CloudCliArtifactRef,
) -> Result<vsn_cloud::CloudCliResult, CoreError> {
    vsn_policy::require(principal, Permission::RemoteView)?;
    vsn_cloud::cloud_cli_copy_status(request).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn cloud_cli_destroy(
    principal: &Principal,
    request: &vsn_cloud::CloudCliDestroyRequest,
) -> Result<vsn_cloud::CloudCliResult, CoreError> {
    vsn_policy::require(principal, Permission::RemoteManage)?;
    vsn_cloud::cloud_cli_destroy(request).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn stream_open(
    principal: &Principal,
    request: vsn_stream::StreamOpenRequest,
) -> Result<vsn_stream::StreamState, CoreError> {
    stream_open_resumed(principal, request, 0, 0)
}
pub fn stream_open_resumed(
    principal: &Principal,
    mut request: vsn_stream::StreamOpenRequest,
    next_in_seq: u64,
    next_out_seq: u64,
) -> Result<vsn_stream::StreamState, CoreError> {
    match request.kind {
        vsn_stream::StreamKind::Terminal => {
            vsn_policy::require(principal, Permission::TerminalExecute)?
        }
        vsn_stream::StreamKind::FileUpload => {
            vsn_policy::require(principal, Permission::FilesWrite)?
        }
        vsn_stream::StreamKind::FileDownload => {
            vsn_policy::require(principal, Permission::FilesRead)?
        }
        vsn_stream::StreamKind::Database => {
            vsn_policy::require(principal, Permission::DatabaseQuery)?
        }
        vsn_stream::StreamKind::Preview => vsn_policy::require(principal, Permission::ProjectView)?,
        vsn_stream::StreamKind::Logs => vsn_policy::require(principal, Permission::ServiceView)?,
        vsn_stream::StreamKind::Custom => {
            return Err(CoreError::Rejected(
                "custom streams require a provider-specific policy binding".into(),
            ))
        }
    };
    request
        .metadata
        .insert("vsn_owner_principal".into(), principal.id.clone());
    Ok(vsn_stream::open_stream_at(
        request,
        next_in_seq,
        next_out_seq,
    )?)
}
fn require_stream_owner(
    principal: &Principal,
    state: &vsn_stream::StreamState,
) -> Result<(), CoreError> {
    if state
        .metadata
        .get("vsn_owner_principal")
        .map(String::as_str)
        == Some(principal.id.as_str())
    {
        Ok(())
    } else {
        Err(CoreError::Rejected(
            "stream belongs to another principal".into(),
        ))
    }
}
pub fn stream_expected_input_seq(principal: &Principal, id: &str) -> Result<u64, CoreError> {
    let state = vsn_stream::stream_state(id)?;
    require_stream_owner(principal, &state)?;
    Ok(state.next_in_seq)
}
pub fn stream_input(
    principal: &Principal,
    id: &str,
    seq: u64,
    payload_base64: &str,
    eof: bool,
) -> Result<vsn_stream::StreamState, CoreError> {
    let state = vsn_stream::stream_state(id)?;
    require_stream_owner(principal, &state)?;
    match state.kind {
        vsn_stream::StreamKind::Terminal => {
            vsn_policy::require(principal, Permission::TerminalExecute)?
        }
        vsn_stream::StreamKind::FileUpload => {
            vsn_policy::require(principal, Permission::FilesWrite)?
        }
        vsn_stream::StreamKind::Database => {
            vsn_policy::require(principal, Permission::DatabaseQuery)?
        }
        vsn_stream::StreamKind::Preview => vsn_policy::require(principal, Permission::ProjectEdit)?,
        _ => {
            return Err(CoreError::Rejected(
                "this stream kind does not accept client input".into(),
            ))
        }
    };
    Ok(vsn_stream::accept_input_frame(
        id,
        seq,
        payload_base64,
        eof,
    )?)
}
pub fn stream_input_pull(
    principal: &Principal,
    id: &str,
    max_frames: usize,
) -> Result<vsn_stream::StreamPull, CoreError> {
    let state = vsn_stream::stream_state(id)?;
    require_stream_owner(principal, &state)?;
    match state.kind {
        vsn_stream::StreamKind::Terminal => {
            vsn_policy::require(principal, Permission::TerminalExecute)?
        }
        vsn_stream::StreamKind::FileUpload => {
            vsn_policy::require(principal, Permission::FilesWrite)?
        }
        vsn_stream::StreamKind::Database => {
            vsn_policy::require(principal, Permission::DatabaseQuery)?
        }
        vsn_stream::StreamKind::Preview => vsn_policy::require(principal, Permission::ProjectEdit)?,
        _ => {
            return Err(CoreError::Rejected(
                "this stream kind does not accept client input".into(),
            ))
        }
    };
    Ok(vsn_stream::pull_input(id, max_frames)?)
}
pub fn stream_output(
    principal: &Principal,
    id: &str,
    payload: &[u8],
    eof: bool,
) -> Result<vsn_stream::StreamFrame, CoreError> {
    let state = vsn_stream::stream_state(id)?;
    require_stream_owner(principal, &state)?;
    match state.kind {
        vsn_stream::StreamKind::Terminal => {
            vsn_policy::require(principal, Permission::TerminalView)?
        }
        vsn_stream::StreamKind::FileDownload => {
            vsn_policy::require(principal, Permission::FilesRead)?
        }
        vsn_stream::StreamKind::Database => {
            vsn_policy::require(principal, Permission::DatabaseView)?
        }
        vsn_stream::StreamKind::Preview => vsn_policy::require(principal, Permission::ProjectView)?,
        vsn_stream::StreamKind::Logs => vsn_policy::require(principal, Permission::ServiceView)?,
        _ => {
            return Err(CoreError::Rejected(
                "this stream kind does not produce agent output".into(),
            ))
        }
    };
    Ok(vsn_stream::queue_output(id, payload, eof)?)
}
pub fn stream_pull(
    principal: &Principal,
    id: &str,
    max_frames: usize,
) -> Result<vsn_stream::StreamPull, CoreError> {
    let state = vsn_stream::stream_state(id)?;
    require_stream_owner(principal, &state)?;
    match state.kind {
        vsn_stream::StreamKind::Terminal => {
            vsn_policy::require(principal, Permission::TerminalView)?
        }
        vsn_stream::StreamKind::FileDownload => {
            vsn_policy::require(principal, Permission::FilesRead)?
        }
        vsn_stream::StreamKind::Database => {
            vsn_policy::require(principal, Permission::DatabaseView)?
        }
        vsn_stream::StreamKind::Preview => vsn_policy::require(principal, Permission::ProjectView)?,
        vsn_stream::StreamKind::Logs => vsn_policy::require(principal, Permission::ServiceView)?,
        _ => vsn_policy::require(principal, Permission::MachineView)?,
    }
    Ok(vsn_stream::pull_output(id, max_frames)?)
}
pub fn stream_close(
    principal: &Principal,
    id: &str,
    reason: Option<&str>,
) -> Result<vsn_stream::StreamState, CoreError> {
    let state = vsn_stream::stream_state(id)?;
    require_stream_owner(principal, &state)?;
    match state.kind {
        vsn_stream::StreamKind::Terminal => {
            vsn_policy::require(principal, Permission::TerminalExecute)?
        }
        vsn_stream::StreamKind::FileUpload => {
            vsn_policy::require(principal, Permission::FilesWrite)?
        }
        vsn_stream::StreamKind::FileDownload => {
            vsn_policy::require(principal, Permission::FilesRead)?
        }
        vsn_stream::StreamKind::Database => {
            vsn_policy::require(principal, Permission::DatabaseQuery)?
        }
        vsn_stream::StreamKind::Preview => vsn_policy::require(principal, Permission::ProjectView)?,
        vsn_stream::StreamKind::Logs => vsn_policy::require(principal, Permission::ServiceView)?,
        vsn_stream::StreamKind::Custom => {
            return Err(CoreError::Rejected(
                "custom streams require provider policy".into(),
            ))
        }
    };
    Ok(vsn_stream::close_stream(id, reason)?)
}
pub fn stream_list(principal: &Principal) -> Result<Vec<vsn_stream::StreamState>, CoreError> {
    vsn_policy::require(principal, Permission::MachineView)?;
    Ok(vsn_stream::list_streams()?
        .into_iter()
        .filter(|s| {
            s.metadata.get("vsn_owner_principal").map(String::as_str) == Some(principal.id.as_str())
        })
        .collect())
}

pub fn preview_conformance(
    principal: &Principal,
) -> Result<vsn_preview::PreviewConformanceReport, CoreError> {
    vsn_policy::require(principal, Permission::ProjectView)?;
    Ok(vsn_preview::conformance())
}
pub fn preview_fetch(
    principal: &Principal,
    request: &vsn_preview::PreviewRequest,
) -> Result<vsn_preview::PreviewResponse, CoreError> {
    vsn_policy::require(principal, Permission::ProjectView)?;
    Ok(vsn_preview::fetch(request)?)
}
pub fn preview_request(
    principal: &Principal,
    request: &vsn_preview::PreviewHttpRequest,
) -> Result<vsn_preview::PreviewHttpResponse, CoreError> {
    vsn_policy::require(principal, Permission::ProjectEdit)?;
    Ok(vsn_preview::request(request)?)
}
pub fn preview_event_stream_start(
    principal: &Principal,
    request: &vsn_preview::PreviewEventStreamRequest,
) -> Result<vsn_preview::PreviewEventStreamState, CoreError> {
    vsn_policy::require(principal, Permission::ProjectView)?;
    Ok(vsn_preview::start_event_stream(request)?)
}
pub fn preview_event_stream_read(
    principal: &Principal,
    id: &str,
) -> Result<vsn_preview::PreviewEventStreamChunk, CoreError> {
    vsn_policy::require(principal, Permission::ProjectView)?;
    Ok(vsn_preview::read_event_stream(id)?)
}
pub fn preview_event_stream_close(principal: &Principal, id: &str) -> Result<bool, CoreError> {
    vsn_policy::require(principal, Permission::ProjectView)?;
    Ok(vsn_preview::close_event_stream(id)?)
}
pub fn preview_websocket_start(
    principal: &Principal,
    request: &vsn_preview::PreviewWebSocketRequest,
) -> Result<vsn_preview::PreviewWebSocketState, CoreError> {
    vsn_policy::require(principal, Permission::ProjectEdit)?;
    Ok(vsn_preview::start_websocket(request)?)
}
pub fn preview_websocket_send(
    principal: &Principal,
    id: &str,
    request: &vsn_preview::PreviewWebSocketSend,
) -> Result<vsn_preview::PreviewWebSocketState, CoreError> {
    vsn_policy::require(principal, Permission::ProjectEdit)?;
    Ok(vsn_preview::send_websocket(id, request)?)
}
pub fn preview_websocket_read(
    principal: &Principal,
    id: &str,
) -> Result<vsn_preview::PreviewWebSocketFrame, CoreError> {
    vsn_policy::require(principal, Permission::ProjectView)?;
    Ok(vsn_preview::read_websocket(id)?)
}
pub fn preview_websocket_close(principal: &Principal, id: &str) -> Result<bool, CoreError> {
    vsn_policy::require(principal, Permission::ProjectEdit)?;
    Ok(vsn_preview::close_websocket(id)?)
}

pub fn vault_list(principal: &Principal) -> Result<Vec<vsn_vault::SecretMetadata>, CoreError> {
    vsn_policy::require(principal, Permission::SecretsUse)?;
    Ok(vsn_vault::list()?)
}
pub fn vault_set(
    principal: &Principal,
    name: &str,
    value: &str,
) -> Result<vsn_vault::SecretMetadata, CoreError> {
    vsn_policy::require(principal, Permission::SecretsManage)?;
    Ok(vsn_vault::set(name, value)?)
}
pub fn vault_delete(principal: &Principal, name: &str) -> Result<bool, CoreError> {
    vsn_policy::require(principal, Permission::SecretsManage)?;
    Ok(vsn_vault::delete(name)?)
}
pub fn vault_reveal(principal: &Principal, name: &str) -> Result<String, CoreError> {
    vsn_policy::require(principal, Permission::SecretsReveal)?;
    Ok(vsn_vault::reveal(name)?)
}
pub fn vault_status(principal: &Principal) -> Result<vsn_vault::VaultStatus, CoreError> {
    vsn_policy::require(principal, Permission::SecretsUse)?;
    Ok(vsn_vault::status()?)
}
pub fn vault_rotate(principal: &Principal) -> Result<vsn_vault::VaultRotationResult, CoreError> {
    vsn_policy::require(principal, Permission::SecretsManage)?;
    Ok(vsn_vault::rotate_master_key()?)
}
pub fn vault_key_history(
    principal: &Principal,
) -> Result<Vec<vsn_vault::VaultKeyRecord>, CoreError> {
    vsn_policy::require(principal, Permission::SecretsManage)?;
    Ok(vsn_vault::key_history()?)
}
pub fn vault_restore(
    principal: &Principal,
    key_id: &str,
    confirm: bool,
) -> Result<vsn_vault::VaultRecoveryResult, CoreError> {
    vsn_policy::require(principal, Permission::SecretsManage)?;
    Ok(vsn_vault::restore_recovery_key(key_id, confirm)?)
}
pub fn vault_retire(
    principal: &Principal,
    key_id: &str,
    confirm: bool,
) -> Result<vsn_vault::VaultRetirementResult, CoreError> {
    vsn_policy::require(principal, Permission::SecretsManage)?;
    Ok(vsn_vault::retire_recovery_key(key_id, confirm)?)
}

pub fn extension_conformance(
    principal: &Principal,
) -> Result<vsn_extension::ExtensionConformanceReport, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    Ok(vsn_extension::extension_conformance())
}
pub fn extension_dependencies(
    principal: &Principal,
    id: &str,
    version: &str,
) -> Result<vsn_extension::ExtensionDependencyReport, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    let root = vsn_security::data_dir()?.join("extensions");
    vsn_extension::dependency_report(&root, id, version)
        .map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn extension_verify(
    principal: &Principal,
    package_dir: &Path,
    trust_path: &Path,
) -> Result<String, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeManage)?;
    let trust: vsn_extension::TrustStore = serde_json::from_slice(
        &std::fs::read(trust_path)
            .map_err(|e| CoreError::Rejected(format!("trust store read failed: {e}")))?,
    )
    .map_err(|e| CoreError::Rejected(format!("trust store parse failed: {e}")))?;
    let manifest = vsn_extension::load_manifest(&package_dir.join("extension.json"))
        .map_err(|e| CoreError::Rejected(e.to_string()))?;
    vsn_extension::verify_manifest(&manifest, &trust)
        .map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn extension_install(
    principal: &Principal,
    package_dir: &Path,
    trust_path: &Path,
) -> Result<vsn_extension::InstalledExtension, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeManage)?;
    let trust: vsn_extension::TrustStore = serde_json::from_slice(
        &std::fs::read(trust_path)
            .map_err(|e| CoreError::Rejected(format!("trust store read failed: {e}")))?,
    )
    .map_err(|e| CoreError::Rejected(format!("trust store parse failed: {e}")))?;
    let root = vsn_security::data_dir()?.join("extensions");
    vsn_extension::install_package(package_dir, &root, &trust)
        .map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn extension_list(
    principal: &Principal,
) -> Result<Vec<vsn_extension::InstalledExtension>, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    let root = vsn_security::data_dir()?.join("extensions");
    vsn_extension::list_installed(&root).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn extension_uninstall(
    principal: &Principal,
    id: &str,
    version: &str,
) -> Result<bool, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeManage)?;
    let root = vsn_security::data_dir()?.join("extensions");
    vsn_extension::uninstall(&root, id, version).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn extension_providers(
    principal: &Principal,
    id: &str,
    version: &str,
    kind: Option<&str>,
) -> Result<Vec<vsn_extension::ResolvedProvider>, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    let root = vsn_security::data_dir()?.join("extensions");
    vsn_extension::resolve_providers(&root, id, version, kind)
        .map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn extension_sandbox_capabilities(
    principal: &Principal,
) -> Result<vsn_extension::SandboxCapabilities, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    Ok(vsn_extension::sandbox_capabilities())
}
pub fn extension_exec(
    principal: &Principal,
    request: &vsn_extension::SandboxExecRequest,
) -> Result<vsn_extension::SandboxExecResult, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeManage)?;
    let root = vsn_security::data_dir()?.join("extensions");
    vsn_extension::run_sandboxed(&root, request).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn marketplace_verify(
    principal: &Principal,
    index: &Path,
    trust: &Path,
) -> Result<serde_json::Value, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    let (catalog, signer) = vsn_marketplace::load_and_verify(index, trust)
        .map_err(|e| CoreError::Rejected(e.to_string()))?;
    Ok(
        serde_json::json!({"signer_public_key":signer,"entries":catalog.entries.len(),"generated_at_unix_ms":catalog.generated_at_unix_ms}),
    )
}
pub fn marketplace_publishers(
    principal: &Principal,
    index: &Path,
    trust: &Path,
) -> Result<Vec<vsn_marketplace::PublisherSummary>, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    let (catalog, _) = vsn_marketplace::load_and_verify(index, trust)
        .map_err(|e| CoreError::Rejected(e.to_string()))?;
    Ok(vsn_marketplace::publishers(&catalog))
}
pub fn marketplace_search(
    principal: &Principal,
    index: &Path,
    trust: &Path,
    query: &str,
) -> Result<Vec<vsn_marketplace::MarketplaceEntry>, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    let (catalog, _) = vsn_marketplace::load_and_verify(index, trust)
        .map_err(|e| CoreError::Rejected(e.to_string()))?;
    Ok(vsn_marketplace::search(&catalog, query))
}
pub fn marketplace_resolve_update(
    principal: &Principal,
    index: &Path,
    trust: &Path,
    id: &str,
    current_version: &str,
) -> Result<vsn_marketplace::UpdateResolution, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    let (catalog, _) = vsn_marketplace::load_and_verify(index, trust)
        .map_err(|e| CoreError::Rejected(e.to_string()))?;
    vsn_marketplace::resolve_update(&catalog, id, current_version)
        .map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn marketplace_resolve_update_channel(
    principal: &Principal,
    index: &Path,
    trust: &Path,
    id: &str,
    current_version: &str,
    channel: &str,
) -> Result<vsn_marketplace::ChannelUpdateResolution, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    let (catalog, _) = vsn_marketplace::load_and_verify(index, trust)
        .map_err(|e| CoreError::Rejected(e.to_string()))?;
    vsn_marketplace::resolve_update_channel(&catalog, id, current_version, channel)
        .map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn marketplace_conformance(
    principal: &Principal,
) -> Result<vsn_marketplace::MarketplaceConformanceReport, CoreError> {
    vsn_policy::require(principal, Permission::RuntimeView)?;
    Ok(vsn_marketplace::conformance())
}
pub fn cloud_conformance(
    principal: &Principal,
) -> Result<vsn_cloud::CloudConformanceReport, CoreError> {
    vsn_policy::require(principal, Permission::RemoteView)?;
    Ok(vsn_cloud::cloud_conformance())
}
pub fn update_verify_manifest(
    principal: &Principal,
    path: &Path,
    public_key_b64: &str,
) -> Result<vsn_update::UpdateManifest, CoreError> {
    vsn_policy::require(principal, Permission::SecurityAuditView)?;
    let bytes = std::fs::read(path)
        .map_err(|e| CoreError::Rejected(format!("update manifest read failed: {e}")))?;
    let manifest: vsn_update::UpdateManifest = serde_json::from_slice(&bytes)
        .map_err(|e| CoreError::Rejected(format!("update manifest parse failed: {e}")))?;
    vsn_update::verify_manifest(&manifest, public_key_b64)
        .map_err(|e| CoreError::Rejected(e.to_string()))?;
    Ok(manifest)
}
pub fn update_verify_artifact(
    principal: &Principal,
    path: &Path,
    sha256: &str,
) -> Result<bool, CoreError> {
    vsn_policy::require(principal, Permission::SecurityAuditView)?;
    vsn_update::verify_artifact(path, sha256).map_err(|e| CoreError::Rejected(e.to_string()))?;
    Ok(true)
}
pub fn update_apply_file(
    principal: &Principal,
    request: &vsn_update::ApplyFileRequest,
) -> Result<vsn_update::ApplyFileResult, CoreError> {
    vsn_policy::require(principal, Permission::MachineManage)?;
    vsn_update::apply_verified_file_locked(request).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn update_rollback_file(
    principal: &Principal,
    install_root: &Path,
    confirm: bool,
) -> Result<vsn_update::ApplyFileResult, CoreError> {
    vsn_policy::require(principal, Permission::MachineManage)?;
    vsn_update::rollback_verified_file_locked(install_root, confirm)
        .map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn update_status(
    principal: &Principal,
    install_root: &Path,
) -> Result<vsn_update::UpdateStatus, CoreError> {
    vsn_policy::require(principal, Permission::MachineView)?;
    vsn_update::update_status(install_root).map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn update_recover_lock(
    principal: &Principal,
    install_root: &Path,
    confirm: bool,
) -> Result<bool, CoreError> {
    vsn_policy::require(principal, Permission::MachineManage)?;
    vsn_update::recover_stale_update_lock(install_root, confirm)
        .map_err(|e| CoreError::Rejected(e.to_string()))
}
pub fn remote_status(principal: &Principal) -> Result<vsn_config::RemoteConfig, CoreError> {
    vsn_policy::require(principal, Permission::RemoteView)?;
    Ok(config()?.remote)
}
pub fn remote_configure(
    principal: &Principal,
    remote: vsn_config::RemoteConfig,
) -> Result<vsn_config::AppConfig, CoreError> {
    vsn_policy::require(principal, Permission::RemoteManage)?;
    Ok(vsn_config::update_remote(remote)?)
}
pub fn remote_enroll(principal: &Principal, pairing_nonce: &str) -> Result<(), CoreError> {
    vsn_policy::require(principal, Permission::RemoteManage)?;
    let cfg = config()?.remote;
    let url = cfg
        .control_plane_url
        .ok_or_else(|| CoreError::Rejected("configure control plane URL first".into()))?;
    let identity = device_identity()?;
    let enrollment = vsn_remote::build_device_enrollment(&identity, pairing_nonce)?;
    vsn_remote::HttpControlPlaneClient::new(&url)?.enroll(&enrollment)?;
    Ok(())
}

fn publish(topic: &str, payload: serde_json::Value) {
    let _ = event_bus().publish(vsn_events::Event {
        topic: topic.into(),
        timestamp_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0),
        payload,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn health_is_up() {
        assert!(core_health().healthy);
    }
    #[test]
    fn timestamp_is_nonzero() {
        assert!(unix_timestamp() > 0);
    }
}
