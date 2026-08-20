use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};
use thiserror::Error;

const MAX_OUTPUT_BYTES: usize = 512 * 1024;
const MAX_ROWS: usize = 5_000;

#[derive(Debug, Error)]
pub enum CliDatabaseError {
    #[error("database CLI request rejected: {0}")]
    Invalid(String),
    #[error("database client unavailable: {0}")]
    ClientUnavailable(String),
    #[error("database client failed: {0}")]
    Client(String),
    #[error("database output is not UTF-8")]
    Encoding,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Engine {
    Postgresql,
    Mysql,
    Mariadb,
    Mongo,
    Redis,
}

impl Engine {
    pub fn parse(value: &str) -> Result<Self, CliDatabaseError> {
        match value {
            "postgresql" | "postgres" | "pgsql" => Ok(Self::Postgresql),
            "mysql" => Ok(Self::Mysql),
            "mariadb" => Ok(Self::Mariadb),
            "mongo" | "mongodb" => Ok(Self::Mongo),
            "redis" => Ok(Self::Redis),
            _ => Err(CliDatabaseError::Invalid(format!(
                "unsupported CLI database engine: {value}"
            ))),
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::Postgresql => "postgresql",
            Self::Mysql => "mysql",
            Self::Mariadb => "mariadb",
            Self::Mongo => "mongodb",
            Self::Redis => "redis",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectionSpec {
    pub engine: Engine,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub database: Option<String>,
    pub credential_file: Option<PathBuf>,
    pub service: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientDetection {
    pub engine: String,
    pub executable: Option<PathBuf>,
    pub version: Option<String>,
    pub available: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueryGrid {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub row_count: usize,
    pub truncated: bool,
    pub raw: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Inspection {
    pub engine: String,
    pub database: Option<String>,
    pub entities: Vec<Value>,
    pub metadata: Value,
}

pub fn detect_clients() -> Vec<ClientDetection> {
    [
        Engine::Postgresql,
        Engine::Mysql,
        Engine::Mariadb,
        Engine::Mongo,
        Engine::Redis,
    ]
    .into_iter()
    .map(detect_client)
    .collect()
}

pub fn inspect(spec: &ConnectionSpec) -> Result<Inspection, CliDatabaseError> {
    validate_spec(spec)?;
    match spec.engine {
        Engine::Postgresql => {
            let sql="SELECT table_schema, table_name, table_type FROM information_schema.tables WHERE table_schema NOT IN ('pg_catalog','information_schema') ORDER BY table_schema, table_name";
            let grid = relational_query(spec, sql)?;
            Ok(Inspection {
                engine: spec.engine.id().into(),
                database: spec.database.clone(),
                entities: grid
                    .rows
                    .iter()
                    .map(|r| json!({"schema":r.first(),"name":r.get(1),"type":r.get(2)}))
                    .collect(),
                metadata: json!({"client":"psql","read_only":true}),
            })
        }
        Engine::Mysql | Engine::Mariadb => {
            let sql="SELECT TABLE_SCHEMA, TABLE_NAME, TABLE_TYPE FROM information_schema.TABLES WHERE TABLE_SCHEMA NOT IN ('information_schema','mysql','performance_schema','sys') ORDER BY TABLE_SCHEMA, TABLE_NAME";
            let grid = relational_query(spec, sql)?;
            Ok(Inspection {
                engine: spec.engine.id().into(),
                database: spec.database.clone(),
                entities: grid
                    .rows
                    .iter()
                    .map(|r| json!({"schema":r.first(),"name":r.get(1),"type":r.get(2)}))
                    .collect(),
                metadata: json!({"client":client_name(spec.engine),"read_only":true}),
            })
        }
        Engine::Mongo => mongo_inspect(spec),
        Engine::Redis => redis_inspect(spec),
    }
}

pub fn query_read_only(
    spec: &ConnectionSpec,
    statement: &str,
) -> Result<QueryGrid, CliDatabaseError> {
    validate_spec(spec)?;
    match spec.engine {
        Engine::Postgresql|Engine::Mysql|Engine::Mariadb => {
            enforce_read_only_sql(statement)?;
            relational_query(spec,statement)
        }
        Engine::Mongo|Engine::Redis => Err(CliDatabaseError::Invalid("arbitrary script/query execution is disabled for MongoDB/Redis baseline; use introspection operations".into())),
    }
}

fn relational_query(spec: &ConnectionSpec, statement: &str) -> Result<QueryGrid, CliDatabaseError> {
    match spec.engine {
        Engine::Postgresql => psql_query(spec, statement),
        Engine::Mysql | Engine::Mariadb => mysql_query(spec, statement),
        _ => Err(CliDatabaseError::Invalid(
            "not a relational CLI provider".into(),
        )),
    }
}

fn psql_query(spec: &ConnectionSpec, statement: &str) -> Result<QueryGrid, CliDatabaseError> {
    let exe = client_path(Engine::Postgresql)?;
    let mut command = Command::new(exe);
    command.args([
        "-X",
        "--no-psqlrc",
        "--no-align",
        "--field-separator=\t",
        "--pset",
        "footer=off",
        "--command",
        statement,
    ]);
    command.env(
        "PGOPTIONS",
        "-c default_transaction_read_only=on -c statement_timeout=15000",
    );
    if let Some(host) = &spec.host {
        command.env("PGHOST", host);
    }
    if let Some(port) = spec.port {
        command.env("PGPORT", port.to_string());
    }
    if let Some(user) = &spec.user {
        command.env("PGUSER", user);
    }
    if let Some(db) = &spec.database {
        command.env("PGDATABASE", db);
    }
    if let Some(service) = &spec.service {
        command.env("PGSERVICE", service);
    }
    if let Some(file) = &spec.credential_file {
        command.env("PGPASSFILE", file);
    }
    parse_tabular(run(&mut command)?)
}

fn mysql_query(spec: &ConnectionSpec, statement: &str) -> Result<QueryGrid, CliDatabaseError> {
    let exe = client_path(spec.engine)?;
    let mut command = Command::new(exe);
    if let Some(file) = &spec.credential_file {
        command.arg(format!("--defaults-extra-file={}", file.display()));
    }
    command.args(["--batch", "--raw"]);
    if spec.engine == Engine::Mysql {
        command.arg("--init-command=SET SESSION TRANSACTION READ ONLY");
    }
    if let Some(host) = &spec.host {
        command.arg(format!("--host={host}"));
    }
    if let Some(port) = spec.port {
        command.arg(format!("--port={port}"));
    }
    if let Some(user) = &spec.user {
        command.arg(format!("--user={user}"));
    }
    if let Some(db) = &spec.database {
        command.arg(format!("--database={db}"));
    }
    command.args(["--execute", statement]);
    parse_tabular(run(&mut command)?)
}

fn mongo_inspect(spec: &ConnectionSpec) -> Result<Inspection, CliDatabaseError> {
    let exe = client_path(Engine::Mongo)?;
    let mut command = Command::new(exe);
    command.arg("--quiet");
    if let Some(host) = &spec.host {
        command.args(["--host", host]);
    }
    if let Some(port) = spec.port {
        command.args(["--port", &port.to_string()]);
    }
    if let Some(db) = &spec.database {
        command.arg(db);
    }
    if spec.user.is_some() {
        return Err(CliDatabaseError::Invalid("Mongo authenticated CLI baseline requires external mongosh configuration; username/password argv exposure is intentionally disabled".into()));
    }
    command.args(["--eval","JSON.stringify(db.getCollectionInfos({}, {nameOnly:true}).map(x => ({name:x.name,type:x.type})))"]);
    let raw = run(&mut command)?;
    let entities: Vec<Value> = serde_json::from_str(raw.trim())
        .map_err(|e| CliDatabaseError::Client(format!("mongosh JSON parse failed: {e}")))?;
    Ok(Inspection {
        engine: "mongodb".into(),
        database: spec.database.clone(),
        entities,
        metadata: json!({"client":"mongosh","read_only":true}),
    })
}

fn redis_inspect(spec: &ConnectionSpec) -> Result<Inspection, CliDatabaseError> {
    let exe = client_path(Engine::Redis)?;
    let mut command = Command::new(exe);
    if let Some(host) = &spec.host {
        command.args(["-h", host]);
    }
    if let Some(port) = spec.port {
        command.args(["-p", &port.to_string()]);
    }
    if let Some(db) = &spec.database {
        let index = db
            .parse::<u16>()
            .map_err(|_| CliDatabaseError::Invalid("Redis database must be numeric".into()))?;
        command.args(["-n", &index.to_string()]);
    }
    if spec.user.is_some() || spec.credential_file.is_some() {
        return Err(CliDatabaseError::Invalid("Redis credentials must be supplied through the local REDISCLI_AUTH/environment configuration; secret argv exposure is disabled".into()));
    }
    command.args(["--scan", "--count", "500"]);
    let raw = run(&mut command)?;
    let mut entities = Vec::new();
    for key in raw.lines().filter(|v| !v.is_empty()).take(MAX_ROWS) {
        entities.push(json!({"key":key}));
    }
    Ok(Inspection {
        engine: "redis".into(),
        database: spec.database.clone(),
        entities,
        metadata: json!({"client":"redis-cli","read_only":true,"truncated":raw.lines().count()>MAX_ROWS}),
    })
}

fn detect_client(engine: Engine) -> ClientDetection {
    let name = client_name(engine);
    match vsn_system::find_executable(name) {
        Ok(path) => {
            let version = Command::new(&path)
                .arg("--version")
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|v| !v.is_empty());
            ClientDetection {
                engine: engine.id().into(),
                executable: Some(path),
                version,
                available: true,
            }
        }
        Err(_) => ClientDetection {
            engine: engine.id().into(),
            executable: None,
            version: None,
            available: false,
        },
    }
}
fn client_name(engine: Engine) -> &'static str {
    match engine {
        Engine::Postgresql => "psql",
        Engine::Mysql => "mysql",
        Engine::Mariadb => "mariadb",
        Engine::Mongo => "mongosh",
        Engine::Redis => "redis-cli",
    }
}
fn client_path(engine: Engine) -> Result<PathBuf, CliDatabaseError> {
    vsn_system::find_executable(client_name(engine))
        .map_err(|_| CliDatabaseError::ClientUnavailable(client_name(engine).into()))
}

fn validate_spec(spec: &ConnectionSpec) -> Result<(), CliDatabaseError> {
    for value in [
        spec.host.as_deref(),
        spec.user.as_deref(),
        spec.database.as_deref(),
        spec.service.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if value.len() > 512 || value.contains('\0') || value.contains('\n') || value.contains('\r')
        {
            return Err(CliDatabaseError::Invalid("unsafe connection field".into()));
        }
    }
    if let Some(path) = &spec.credential_file {
        if !path.is_file() {
            return Err(CliDatabaseError::Invalid(
                "credential file not found".into(),
            ));
        }
    }
    Ok(())
}
fn enforce_read_only_sql(statement: &str) -> Result<(), CliDatabaseError> {
    if statement.len() > 1024 * 1024 {
        return Err(CliDatabaseError::Invalid("query too large".into()));
    }
    let trimmed = statement.trim();
    if trimmed.is_empty() {
        return Err(CliDatabaseError::Invalid("query is empty".into()));
    }
    let without_tail = trimmed.strip_suffix(';').unwrap_or(trimmed);
    if without_tail.contains(';') {
        return Err(CliDatabaseError::Invalid(
            "multiple SQL statements are not allowed".into(),
        ));
    }
    let upper = without_tail.to_ascii_uppercase();
    let keyword = upper
        .split(|c: char| c.is_whitespace() || c == '(')
        .next()
        .unwrap_or("");
    if !matches!(keyword, "SELECT" | "SHOW" | "DESCRIBE" | "DESC") {
        return Err(CliDatabaseError::Invalid("only single-statement SELECT/SHOW/DESCRIBE queries are accepted by the CLI provider baseline".into()));
    }
    if keyword == "SELECT" {
        for banned in [
            " INTO ",
            " FOR UPDATE",
            " FOR SHARE",
            " LOCK IN SHARE MODE",
            " GET_LOCK(",
            " RELEASE_LOCK(",
            " PG_TERMINATE_BACKEND(",
            " PG_CANCEL_BACKEND(",
            " PG_ADVISORY_LOCK(",
        ] {
            if format!(" {upper} ").contains(banned) {
                return Err(CliDatabaseError::Invalid("query contains a side-effect-capable clause/function blocked by the read-only baseline".into()));
            }
        }
    }
    Ok(())
}

fn run(command: &mut Command) -> Result<String, CliDatabaseError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = command
        .output()
        .map_err(|e| CliDatabaseError::Client(e.to_string()))?;
    if !output.status.success() {
        return Err(CliDatabaseError::Client(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    if output.stdout.len() > MAX_OUTPUT_BYTES {
        return Err(CliDatabaseError::Client(
            "database output exceeded 512 KiB limit".into(),
        ));
    }
    String::from_utf8(output.stdout).map_err(|_| CliDatabaseError::Encoding)
}
fn parse_tabular(raw: String) -> Result<QueryGrid, CliDatabaseError> {
    let mut lines = raw.lines();
    let columns = lines
        .next()
        .map(|l| l.split('\t').map(str::to_string).collect())
        .unwrap_or_default();
    let mut rows = Vec::new();
    let mut truncated = false;
    for line in lines {
        if rows.len() >= MAX_ROWS {
            truncated = true;
            break;
        }
        rows.push(
            line.split('\t')
                .map(|v| Value::String(v.to_string()))
                .collect(),
        );
    }
    let row_count = rows.len();
    Ok(QueryGrid {
        columns,
        rows,
        row_count,
        truncated,
        raw,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn write_query_is_rejected() {
        assert!(enforce_read_only_sql("DELETE FROM users").is_err());
        assert!(enforce_read_only_sql("SELECT * FROM users").is_ok());
    }
    #[test]
    fn stacked_statement_is_rejected() {
        assert!(enforce_read_only_sql("SELECT 1; DROP TABLE users").is_err());
    }
    #[test]
    fn side_effect_clauses_are_rejected() {
        assert!(enforce_read_only_sql("SELECT * INTO backup FROM users").is_err());
        assert!(enforce_read_only_sql("SELECT * FROM users FOR UPDATE").is_err());
        assert!(enforce_read_only_sql("EXPLAIN ANALYZE DELETE FROM users").is_err());
    }
}

// 0.14 durable cancellable read-query jobs. These intentionally use the native
// database CLI processes so cancellation can terminate the exact child process.
use std::{
    collections::HashMap,
    fs,
    io::{Read as _, Seek, SeekFrom},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
const MAX_JOB_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_JOB_PREVIEW_BYTES: usize = 512 * 1024;
const MAX_JOB_OUTPUT_CHUNK: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryJobState {
    Running,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueryJobStatus {
    pub job_id: String,
    pub engine: String,
    pub state: QueryJobState,
    pub created_at_unix_ms: u128,
    pub updated_at_unix_ms: u128,
    pub result: Option<QueryGrid>,
    pub error: Option<String>,
    pub pid: Option<u32>,
    #[serde(default)]
    pub artifact: Option<QueryJobArtifact>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueryJobArtifact {
    pub bytes: u64,
    pub sha256: String,
    pub preview_truncated: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueryJobOutputChunk {
    pub job_id: String,
    pub offset: u64,
    pub next_offset: u64,
    pub eof: bool,
    pub payload_base64: String,
    pub total_bytes: u64,
    pub sha256: String,
}
struct QueryJobEntry {
    status: QueryJobStatus,
    cancel: Arc<AtomicBool>,
}
static QUERY_JOBS: OnceLock<Mutex<HashMap<String, QueryJobEntry>>> = OnceLock::new();
fn query_jobs() -> &'static Mutex<HashMap<String, QueryJobEntry>> {
    QUERY_JOBS.get_or_init(|| Mutex::new(HashMap::new()))
}
fn qnow() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_millis())
        .unwrap_or(0)
}
fn qid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering as O};
    static N: AtomicU64 = AtomicU64::new(1);
    format!("dbjob_{:x}_{:x}", qnow(), N.fetch_add(1, O::Relaxed))
}
fn validate_job_id(id: &str) -> Result<(), CliDatabaseError> {
    if id.len() < 8
        || id.len() > 160
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        Err(CliDatabaseError::Invalid("invalid database job id".into()))
    } else {
        Ok(())
    }
}
fn job_path(state_dir: &Path, id: &str) -> Result<PathBuf, CliDatabaseError> {
    validate_job_id(id)?;
    Ok(state_dir.join(format!("{id}.json")))
}
fn write_job_status(state_dir: &Path, status: &QueryJobStatus) -> Result<(), CliDatabaseError> {
    fs::create_dir_all(state_dir).map_err(|e| {
        CliDatabaseError::Client(format!("database job state directory failed: {e}"))
    })?;
    let path = job_path(state_dir, &status.job_id)?;
    let tmp = path.with_extension("tmp");
    let backup = path.with_extension("bak");
    let mut bytes = serde_json::to_vec_pretty(status).map_err(|e| {
        CliDatabaseError::Client(format!("database job state serialization failed: {e}"))
    })?;
    bytes.push(b'\n');
    {
        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)
            .map_err(|e| {
                CliDatabaseError::Client(format!("database job state open failed: {e}"))
            })?;
        f.write_all(&bytes).map_err(|e| {
            CliDatabaseError::Client(format!("database job state write failed: {e}"))
        })?;
        f.sync_all().map_err(|e| {
            CliDatabaseError::Client(format!("database job state sync failed: {e}"))
        })?;
    }
    let had_previous = path.exists();
    if had_previous {
        let _ = fs::remove_file(&backup);
        fs::rename(&path, &backup).map_err(|e| {
            CliDatabaseError::Client(format!("database job previous state staging failed: {e}"))
        })?;
    }
    match fs::rename(&tmp, &path) {
        Ok(()) => {
            if had_previous {
                let _ = fs::remove_file(&backup);
            }
            Ok(())
        }
        Err(e) => {
            if had_previous && backup.exists() {
                let _ = fs::rename(&backup, &path);
            }
            let _ = fs::remove_file(&tmp);
            Err(CliDatabaseError::Client(format!(
                "database job state commit failed: {e}"
            )))
        }
    }
}

pub fn recover_interrupted_query_jobs(state_dir: &Path) -> Result<u32, CliDatabaseError> {
    fs::create_dir_all(state_dir).map_err(|e| CliDatabaseError::Client(e.to_string()))?;
    let mut recovered = 0u32;
    for entry in fs::read_dir(state_dir).map_err(|e| CliDatabaseError::Client(e.to_string()))? {
        let path = entry
            .map_err(|e| CliDatabaseError::Client(e.to_string()))?
            .path();
        if path.extension().and_then(|v| v.to_str()) != Some("json") {
            continue;
        }
        let bytes = match fs::read(&path) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let Ok(mut status) = serde_json::from_slice::<QueryJobStatus>(&bytes) else {
            continue;
        };
        let active_in_process = {
            let jobs = query_jobs()
                .lock()
                .map_err(|_| CliDatabaseError::Client("database job registry poisoned".into()))?;
            jobs.get(&status.job_id)
                .is_some_and(|entry| entry.status.state == QueryJobState::Running)
        };
        if status.state == QueryJobState::Running && !active_in_process {
            status.state = QueryJobState::Interrupted;
            status.updated_at_unix_ms = qnow();
            status.pid = None;
            status.error=Some("Agent/process restarted while database query was running; job was not automatically re-executed".into());
            write_job_status(state_dir, &status)?;
            recovered = recovered.saturating_add(1);
        }
        if !active_in_process {
            query_jobs()
                .lock()
                .map_err(|_| CliDatabaseError::Client("database job registry poisoned".into()))?
                .entry(status.job_id.clone())
                .or_insert(QueryJobEntry {
                    status,
                    cancel: Arc::new(AtomicBool::new(false)),
                });
        }
    }
    Ok(recovered)
}

pub fn start_read_query_job(
    spec: ConnectionSpec,
    statement: String,
    state_dir: PathBuf,
) -> Result<QueryJobStatus, CliDatabaseError> {
    validate_spec(&spec)?;
    enforce_read_only_sql(&statement)?;
    if !matches!(
        spec.engine,
        Engine::Postgresql | Engine::Mysql | Engine::Mariadb
    ) {
        return Err(CliDatabaseError::Invalid(
            "cancellable query jobs currently support PostgreSQL/MySQL/MariaDB read queries".into(),
        ));
    }
    fs::create_dir_all(&state_dir).map_err(|e| CliDatabaseError::Client(e.to_string()))?;
    let _ = recover_interrupted_query_jobs(&state_dir)?;
    {
        let mut jobs = query_jobs()
            .lock()
            .map_err(|_| CliDatabaseError::Client("database job registry poisoned".into()))?;
        jobs.retain(|_, e| {
            e.status.state == QueryJobState::Running
                || qnow().saturating_sub(e.status.updated_at_unix_ms) < 60 * 60 * 1000
        });
        if jobs.len() >= 256 {
            return Err(CliDatabaseError::Invalid(
                "database job limit reached".into(),
            ));
        }
    }
    let id = qid();
    let now = qnow();
    let status = QueryJobStatus {
        job_id: id.clone(),
        engine: spec.engine.id().into(),
        state: QueryJobState::Running,
        created_at_unix_ms: now,
        updated_at_unix_ms: now,
        result: None,
        error: None,
        pid: None,
        artifact: None,
    };
    write_job_status(&state_dir, &status)?;
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut jobs = query_jobs()
            .lock()
            .map_err(|_| CliDatabaseError::Client("database job registry poisoned".into()))?;
        jobs.insert(
            id.clone(),
            QueryJobEntry {
                status: status.clone(),
                cancel: cancel.clone(),
            },
        );
    }
    std::thread::spawn(move || {
        let outcome = run_cancellable_query_job(&spec, &statement, &state_dir, &id, &cancel);
        let mut final_status = match query_jobs().lock() {
            Ok(jobs) => jobs
                .get(&id)
                .map(|e| e.status.clone())
                .unwrap_or(QueryJobStatus {
                    job_id: id.clone(),
                    engine: spec.engine.id().into(),
                    state: QueryJobState::Interrupted,
                    created_at_unix_ms: now,
                    updated_at_unix_ms: qnow(),
                    result: None,
                    error: Some("database job registry entry disappeared".into()),
                    pid: None,
                    artifact: None,
                }),
            Err(_) => QueryJobStatus {
                job_id: id.clone(),
                engine: spec.engine.id().into(),
                state: QueryJobState::Interrupted,
                created_at_unix_ms: now,
                updated_at_unix_ms: qnow(),
                result: None,
                error: Some("database job registry poisoned".into()),
                pid: None,
                artifact: None,
            },
        };
        match outcome {
            Ok(success) => {
                final_status.state = QueryJobState::Completed;
                final_status.result = success.preview;
                final_status.artifact = Some(success.artifact);
                final_status.error = None;
            }
            Err(JobRunError::Cancelled) => {
                final_status.state = QueryJobState::Cancelled;
                final_status.error = Some("database query cancelled by operator".into());
            }
            Err(JobRunError::Failed(e)) => {
                final_status.state = QueryJobState::Failed;
                final_status.error = Some(e);
            }
        }
        final_status.pid = None;
        final_status.updated_at_unix_ms = qnow();
        if let Ok(mut jobs) = query_jobs().lock() {
            if let Some(entry) = jobs.get_mut(&id) {
                entry.status = final_status.clone();
            }
        }
        let _ = write_job_status(&state_dir, &final_status);
    });
    Ok(status)
}

pub fn query_job_status(
    job_id: &str,
    state_dir: &Path,
) -> Result<QueryJobStatus, CliDatabaseError> {
    validate_job_id(job_id)?;
    let _ = recover_interrupted_query_jobs(state_dir)?;
    if let Some(status) = query_jobs()
        .lock()
        .map_err(|_| CliDatabaseError::Client("database job registry poisoned".into()))?
        .get(job_id)
        .map(|e| e.status.clone())
    {
        return Ok(status);
    }
    let bytes = fs::read(job_path(state_dir, job_id)?)
        .map_err(|_| CliDatabaseError::Invalid("database job not found".into()))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| CliDatabaseError::Client(format!("database job state invalid: {e}")))
}
pub fn list_query_jobs(state_dir: &Path) -> Result<Vec<QueryJobStatus>, CliDatabaseError> {
    let _ = recover_interrupted_query_jobs(state_dir)?;
    let mut out = query_jobs()
        .lock()
        .map_err(|_| CliDatabaseError::Client("database job registry poisoned".into()))?
        .values()
        .map(|e| e.status.clone())
        .collect::<Vec<_>>();
    out.sort_by_key(|b| std::cmp::Reverse(b.created_at_unix_ms));
    out.truncate(256);
    Ok(out)
}
pub fn cancel_query_job(
    job_id: &str,
    state_dir: &Path,
) -> Result<QueryJobStatus, CliDatabaseError> {
    validate_job_id(job_id)?;
    let _ = recover_interrupted_query_jobs(state_dir)?;
    let mut jobs = query_jobs()
        .lock()
        .map_err(|_| CliDatabaseError::Client("database job registry poisoned".into()))?;
    let entry = jobs
        .get_mut(job_id)
        .ok_or_else(|| CliDatabaseError::Invalid("database job not found".into()))?;
    if entry.status.state == QueryJobState::Running {
        entry.cancel.store(true, Ordering::SeqCst);
    }
    Ok(entry.status.clone())
}

enum JobRunError {
    Cancelled,
    Failed(String),
}
struct JobRunSuccess {
    preview: Option<QueryGrid>,
    artifact: QueryJobArtifact,
}
fn job_result_path(state_dir: &Path, job_id: &str) -> Result<PathBuf, CliDatabaseError> {
    validate_job_id(job_id)?;
    Ok(state_dir.join(format!("{job_id}.result.bin")))
}
fn run_cancellable_query_job(
    spec: &ConnectionSpec,
    statement: &str,
    state_dir: &Path,
    job_id: &str,
    cancel: &Arc<AtomicBool>,
) -> Result<JobRunSuccess, JobRunError> {
    let mut command = build_read_query_command(spec, statement)
        .map_err(|e| JobRunError::Failed(e.to_string()))?;
    let stdout_path = state_dir.join(format!("{job_id}.stdout.tmp"));
    let stderr_path = state_dir.join(format!("{job_id}.stderr.tmp"));
    let final_path =
        job_result_path(state_dir, job_id).map_err(|e| JobRunError::Failed(e.to_string()))?;
    let _ = fs::remove_file(&final_path);
    let stdout = fs::File::create(&stdout_path).map_err(|e| JobRunError::Failed(e.to_string()))?;
    let stderr = fs::File::create(&stderr_path).map_err(|e| JobRunError::Failed(e.to_string()))?;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    let mut child = command
        .spawn()
        .map_err(|e| JobRunError::Failed(e.to_string()))?;
    if let Ok(mut jobs) = query_jobs().lock() {
        if let Some(entry) = jobs.get_mut(job_id) {
            entry.status.pid = Some(child.id());
            entry.status.updated_at_unix_ms = qnow();
            let _ = write_job_status(state_dir, &entry.status);
        }
    }
    let started = std::time::Instant::now();
    let timeout = Duration::from_secs(30);
    let status = loop {
        if cancel.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = fs::remove_file(&stdout_path);
            let _ = fs::remove_file(&stderr_path);
            return Err(JobRunError::Cancelled);
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = fs::remove_file(&stdout_path);
            let _ = fs::remove_file(&stderr_path);
            return Err(JobRunError::Failed(
                "database query exceeded 30 second job timeout".into(),
            ));
        }
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = fs::remove_file(&stdout_path);
                let _ = fs::remove_file(&stderr_path);
                return Err(JobRunError::Failed(e.to_string()));
            }
        }
    };
    let stderr = read_bounded_file(&stderr_path, 256 * 1024).unwrap_or_default();
    let _ = fs::remove_file(&stderr_path);
    if !status.success() {
        let _ = fs::remove_file(&stdout_path);
        return Err(JobRunError::Failed(
            String::from_utf8_lossy(&stderr)
                .chars()
                .take(4096)
                .collect(),
        ));
    }
    let meta = fs::metadata(&stdout_path).map_err(|e| JobRunError::Failed(e.to_string()))?;
    if meta.len() > MAX_JOB_ARTIFACT_BYTES {
        let _ = fs::remove_file(&stdout_path);
        return Err(JobRunError::Failed(format!(
            "database job output exceeded {} MiB artifact limit",
            MAX_JOB_ARTIFACT_BYTES / (1024 * 1024)
        )));
    }
    let sha = sha256_file(&stdout_path).map_err(JobRunError::Failed)?;
    fs::rename(&stdout_path, &final_path)
        .map_err(|e| JobRunError::Failed(format!("database result artifact commit failed: {e}")))?;
    let prefix = read_prefix(&final_path, MAX_JOB_PREVIEW_BYTES).map_err(JobRunError::Failed)?;
    let preview_truncated = meta.len() > prefix.len() as u64;
    let preview = if prefix.is_empty() {
        Some(QueryGrid {
            columns: Vec::new(),
            rows: Vec::new(),
            row_count: 0,
            truncated: preview_truncated,
            raw: String::new(),
        })
    } else {
        let raw = String::from_utf8_lossy(&prefix).into_owned();
        match parse_tabular(raw) {
            Ok(mut g) => {
                g.truncated |= preview_truncated;
                Some(g)
            }
            Err(_) => None,
        }
    };
    Ok(JobRunSuccess {
        preview,
        artifact: QueryJobArtifact {
            bytes: meta.len(),
            sha256: sha,
            preview_truncated,
        },
    })
}
fn read_bounded_file(path: &Path, max: usize) -> Result<Vec<u8>, String> {
    let f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    f.take(max as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|e| e.to_string())?;
    if out.len() > max {
        return Err("database job output exceeded safety limit".into());
    }
    Ok(out)
}
fn read_prefix(path: &Path, max: usize) -> Result<Vec<u8>, String> {
    let mut f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    f.by_ref()
        .take(max as u64)
        .read_to_end(&mut out)
        .map_err(|e| e.to_string())?;
    Ok(out)
}
fn sha256_file(path: &Path) -> Result<String, String> {
    let mut f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}
pub fn read_query_job_output(
    job_id: &str,
    state_dir: &Path,
    offset: u64,
    max_bytes: usize,
) -> Result<QueryJobOutputChunk, CliDatabaseError> {
    let status = query_job_status(job_id, state_dir)?;
    let artifact = status
        .artifact
        .ok_or_else(|| CliDatabaseError::Invalid("database job has no result artifact".into()))?;
    if status.state != QueryJobState::Completed {
        return Err(CliDatabaseError::Invalid(
            "database job is not completed".into(),
        ));
    }
    if offset > artifact.bytes {
        return Err(CliDatabaseError::Invalid(
            "database job output offset exceeds artifact size".into(),
        ));
    }
    let cap = max_bytes.clamp(1, MAX_JOB_OUTPUT_CHUNK);
    let path = job_result_path(state_dir, job_id)?;
    let mut f = fs::File::open(path).map_err(|e| CliDatabaseError::Client(e.to_string()))?;
    f.seek(SeekFrom::Start(offset))
        .map_err(|e| CliDatabaseError::Client(e.to_string()))?;
    let remaining = artifact.bytes.saturating_sub(offset);
    let take = remaining.min(cap as u64) as usize;
    let mut bytes = vec![0u8; take];
    if take > 0 {
        f.read_exact(&mut bytes)
            .map_err(|e| CliDatabaseError::Client(e.to_string()))?;
    }
    let next = offset + bytes.len() as u64;
    Ok(QueryJobOutputChunk {
        job_id: job_id.into(),
        offset,
        next_offset: next,
        eof: next >= artifact.bytes,
        payload_base64: B64.encode(bytes),
        total_bytes: artifact.bytes,
        sha256: artifact.sha256,
    })
}
pub fn remove_query_job_artifact(job_id: &str, state_dir: &Path) -> Result<bool, CliDatabaseError> {
    let status = query_job_status(job_id, state_dir)?;
    if status.state == QueryJobState::Running {
        return Err(CliDatabaseError::Invalid(
            "cannot remove output from a running job".into(),
        ));
    }
    let path = job_result_path(state_dir, job_id)?;
    if path.exists() {
        fs::remove_file(path).map_err(|e| CliDatabaseError::Client(e.to_string()))?;
        let mut updated = status;
        updated.artifact = None;
        updated.result = None;
        updated.updated_at_unix_ms = qnow();
        write_job_status(state_dir, &updated)?;
        if let Ok(mut jobs) = query_jobs().lock() {
            if let Some(entry) = jobs.get_mut(job_id) {
                entry.status = updated;
            }
        }
        Ok(true)
    } else {
        Ok(false)
    }
}
fn build_read_query_command(
    spec: &ConnectionSpec,
    statement: &str,
) -> Result<Command, CliDatabaseError> {
    match spec.engine {
        Engine::Postgresql => {
            let exe = client_path(Engine::Postgresql)?;
            let mut command = Command::new(exe);
            command.args([
                "-X",
                "--no-psqlrc",
                "--no-align",
                "--field-separator=\t",
                "--pset",
                "footer=off",
                "--command",
                statement,
            ]);
            command.env(
                "PGOPTIONS",
                "-c default_transaction_read_only=on -c statement_timeout=15000",
            );
            if let Some(host) = &spec.host {
                command.env("PGHOST", host);
            }
            if let Some(port) = spec.port {
                command.env("PGPORT", port.to_string());
            }
            if let Some(user) = &spec.user {
                command.env("PGUSER", user);
            }
            if let Some(db) = &spec.database {
                command.env("PGDATABASE", db);
            }
            if let Some(service) = &spec.service {
                command.env("PGSERVICE", service);
            }
            if let Some(file) = &spec.credential_file {
                command.env("PGPASSFILE", file);
            }
            Ok(command)
        }
        Engine::Mysql | Engine::Mariadb => {
            let exe = client_path(spec.engine)?;
            let mut command = Command::new(exe);
            if let Some(file) = &spec.credential_file {
                command.arg(format!("--defaults-extra-file={}", file.display()));
            }
            command.args(["--batch", "--raw"]);
            if spec.engine == Engine::Mysql {
                command.arg("--init-command=SET SESSION TRANSACTION READ ONLY");
            }
            if let Some(host) = &spec.host {
                command.arg(format!("--host={host}"));
            }
            if let Some(port) = spec.port {
                command.arg(format!("--port={port}"));
            }
            if let Some(user) = &spec.user {
                command.arg(format!("--user={user}"));
            }
            if let Some(db) = &spec.database {
                command.arg(format!("--database={db}"));
            }
            command.args(["--execute", statement]);
            Ok(command)
        }
        _ => Err(CliDatabaseError::Invalid(
            "database query job engine is unsupported".into(),
        )),
    }
}
#[cfg(test)]
mod query_job_tests {
    use super::*;
    #[test]
    fn job_id_validation_is_strict() {
        assert!(validate_job_id("dbjob_12345678").is_ok());
        assert!(validate_job_id("../bad").is_err());
    }
}
