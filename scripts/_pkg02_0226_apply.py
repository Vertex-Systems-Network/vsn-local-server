from pathlib import Path
import re

def read(path):
    return Path(path).read_text(encoding="utf-8")

def write(path, text):
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text, encoding="utf-8", newline="\n")

def replace_once(path, old, new):
    text = read(path)
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one exact match, got {count}")
    write(path, text.replace(old, new, 1))

def regex_once(path, pattern, replacement):
    text = read(path)
    text2, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{path}: expected one regex match, got {count}: {pattern}")
    write(path, text2)

# vsn-database truthful five-engine matrix
replace_once(
    "crates/vsn-database/src/lib.rs",
    '''    pub transactions: bool,
    pub live_stream_read: bool,
    pub notes: Vec<String>,
''',
    '''    pub transactions: bool,
    pub live_stream_read: bool,
    pub plaintext_loopback: bool,
    pub verified_tls_remote: bool,
    pub notes: Vec<String>,
'''
)
regex_once(
    "crates/vsn-database/src/lib.rs",
    r'pub fn remote_database_capabilities\(\) -> Vec<RemoteDatabaseCapability> \{.*?\n\}\npub fn validate_remote_database_capabilities',
    '''pub fn remote_database_capabilities() -> Vec<RemoteDatabaseCapability> {
    vec![
        RemoteDatabaseCapability {
            engine: "postgresql".into(),
            inspect: true,
            browse: true,
            query: true,
            write: true,
            indexes: true,
            relations: true,
            statistics: true,
            durable_jobs: true,
            cancellable_jobs: true,
            transactions: true,
            live_stream_read: true,
            plaintext_loopback: true,
            verified_tls_remote: true,
            notes: vec![
                "native plaintext is exact-loopback only; verified TLS is available for remote reads".into(),
                "structured writes remain DatabaseWrite-gated and native loopback-only".into(),
            ],
        },
        RemoteDatabaseCapability {
            engine: "mysql".into(),
            inspect: true,
            browse: true,
            query: true,
            write: true,
            indexes: true,
            relations: true,
            statistics: true,
            durable_jobs: true,
            cancellable_jobs: true,
            transactions: false,
            live_stream_read: true,
            plaintext_loopback: true,
            verified_tls_remote: true,
            notes: vec![
                "native plaintext is exact-loopback only; verified TLS is available for remote reads".into(),
                "structured writes remain DatabaseWrite-gated and native loopback-only".into(),
            ],
        },
        RemoteDatabaseCapability {
            engine: "mariadb".into(),
            inspect: true,
            browse: false,
            query: true,
            write: false,
            indexes: false,
            relations: false,
            statistics: false,
            durable_jobs: true,
            cancellable_jobs: true,
            transactions: false,
            live_stream_read: true,
            plaintext_loopback: true,
            verified_tls_remote: true,
            notes: vec![
                "external client read/query beta only; remote use forces CA and server-certificate verification".into(),
            ],
        },
        RemoteDatabaseCapability {
            engine: "mongodb".into(),
            inspect: true,
            browse: true,
            query: false,
            write: true,
            indexes: true,
            relations: false,
            statistics: true,
            durable_jobs: false,
            cancellable_jobs: false,
            transactions: false,
            live_stream_read: false,
            plaintext_loopback: true,
            verified_tls_remote: true,
            notes: vec![
                "structured document browse/filter and CRUD; arbitrary JavaScript/query execution is unavailable".into(),
                "remote native SRV and external client paths reject insecure TLS overrides".into(),
            ],
        },
        RemoteDatabaseCapability {
            engine: "redis".into(),
            inspect: true,
            browse: false,
            query: false,
            write: true,
            indexes: false,
            relations: false,
            statistics: false,
            durable_jobs: false,
            cancellable_jobs: false,
            transactions: false,
            live_stream_read: false,
            plaintext_loopback: true,
            verified_tls_remote: true,
            notes: vec![
                "typed key inspection/get/set/delete baseline; arbitrary Redis command execution is unavailable".into(),
                "remote TLS uses trusted certificate verification; insecure mode is rejected".into(),
            ],
        },
    ]
}
pub fn validate_remote_database_capabilities'''
)
replace_once(
    "crates/vsn-database/src/lib.rs",
    '''        if c.write && !c.inspect {
            issues.push(format!("{} exposes writes without inspection", c.engine));
        }
''',
    '''        if !c.plaintext_loopback {
            issues.push(format!("{} does not declare exact-loopback plaintext policy", c.engine));
        }
        if !c.verified_tls_remote {
            issues.push(format!("{} does not declare verified remote TLS policy", c.engine));
        }
        if c.write && !c.inspect {
            issues.push(format!("{} exposes writes without inspection", c.engine));
        }
'''
)

# vsn-database-cli transport and process bounds
replace_once(
    "crates/vsn-database-cli/src/lib.rs",
    '''    pub credential_file: Option<PathBuf>,
    pub service: Option<String>,
}
''',
    '''    pub credential_file: Option<PathBuf>,
    #[serde(default)]
    pub root_ca_file: Option<PathBuf>,
    pub service: Option<String>,
}
'''
)
marker = '''pub fn inspect(spec: &ConnectionSpec) -> Result<Inspection, CliDatabaseError> {
'''
helpers = r'''fn exact_loopback_host(host: &str) -> bool {
    matches!(host.to_ascii_lowercase().as_str(), "localhost" | "127.0.0.1" | "::1")
}

fn remote_host(spec: &ConnectionSpec) -> bool {
    spec.host
        .as_deref()
        .is_some_and(|host| !exact_loopback_host(host))
}

fn use_verified_tls(spec: &ConnectionSpec) -> bool {
    spec.root_ca_file.is_some()
}

fn apply_postgres_transport(command: &mut Command, spec: &ConnectionSpec) {
    if let Some(ca) = &spec.root_ca_file {
        command.env("PGSSLMODE", "verify-full");
        command.env("PGSSLROOTCERT", ca);
    } else {
        command.env("PGSSLMODE", "disable");
    }
}

fn apply_mysql_transport(command: &mut Command, spec: &ConnectionSpec) {
    let Some(ca) = &spec.root_ca_file else {
        return;
    };
    if spec.engine == Engine::Mysql {
        command.arg("--ssl-mode=VERIFY_IDENTITY");
        command.arg(format!("--ssl-ca={}", ca.display()));
    } else {
        command.arg("--ssl");
        command.arg(format!("--ssl-ca={}", ca.display()));
        command.arg("--ssl-verify-server-cert");
    }
}

fn apply_mongo_transport(command: &mut Command, spec: &ConnectionSpec) {
    if let Some(ca) = &spec.root_ca_file {
        command.arg("--tls");
        command.args(["--tlsCAFile", &ca.display().to_string()]);
    }
}

fn apply_redis_transport(command: &mut Command, spec: &ConnectionSpec) {
    if let Some(ca) = &spec.root_ca_file {
        command.arg("--tls");
        command.args(["--cacert", &ca.display().to_string()]);
    }
}

'''
replace_once("crates/vsn-database-cli/src/lib.rs", marker, helpers + marker)
replace_once(
    "crates/vsn-database-cli/src/lib.rs",
    '''    if let Some(file) = &spec.credential_file {
        command.env("PGPASSFILE", file);
    }
    parse_tabular(run(&mut command)?)
''',
    '''    if let Some(file) = &spec.credential_file {
        command.env("PGPASSFILE", file);
    }
    apply_postgres_transport(&mut command, spec);
    parse_tabular(run(&mut command)?)
'''
)
replace_once(
    "crates/vsn-database-cli/src/lib.rs",
    '''    if let Some(db) = &spec.database {
        command.arg(format!("--database={db}"));
    }
    command.args(["--execute", statement]);
''',
    '''    if let Some(db) = &spec.database {
        command.arg(format!("--database={db}"));
    }
    apply_mysql_transport(&mut command, spec);
    command.args(["--execute", statement]);
'''
)
replace_once(
    "crates/vsn-database-cli/src/lib.rs",
    '''    if spec.user.is_some() {
        return Err(CliDatabaseError::Invalid("Mongo authenticated CLI baseline requires external mongosh configuration; username/password argv exposure is intentionally disabled".into()));
    }
    command.args(["--eval","JSON.stringify(db.getCollectionInfos({}, {nameOnly:true}).map(x => ({name:x.name,type:x.type})))"]);
''',
    '''    if spec.user.is_some() {
        return Err(CliDatabaseError::Invalid("Mongo authenticated CLI baseline requires external mongosh configuration; username/password argv exposure is intentionally disabled".into()));
    }
    apply_mongo_transport(&mut command, spec);
    command.args(["--eval","JSON.stringify(db.getCollectionInfos({}, {nameOnly:true}).map(x => ({name:x.name,type:x.type})))"]);
'''
)
replace_once(
    "crates/vsn-database-cli/src/lib.rs",
    '''    if spec.user.is_some() || spec.credential_file.is_some() {
        return Err(CliDatabaseError::Invalid("Redis credentials must be supplied through the local REDISCLI_AUTH/environment configuration; secret argv exposure is disabled".into()));
    }
    command.args(["--scan", "--count", "500"]);
''',
    '''    if spec.user.is_some() || spec.credential_file.is_some() {
        return Err(CliDatabaseError::Invalid("Redis credentials must be supplied through the local REDISCLI_AUTH/environment configuration; secret argv exposure is disabled".into()));
    }
    apply_redis_transport(&mut command, spec);
    command.args(["--scan", "--count", "500"]);
'''
)
regex_once(
    "crates/vsn-database-cli/src/lib.rs",
    r'fn detect_client\(engine: Engine\) -> ClientDetection \{.*?\n\}\nfn client_name',
    r'''fn detect_client(engine: Engine) -> ClientDetection {
    let name = client_name(engine);
    match vsn_system::find_executable(name) {
        Ok(path) => detect_client_at(engine, path),
        Err(_) => ClientDetection {
            engine: engine.id().into(),
            executable: None,
            version: None,
            available: false,
        },
    }
}

fn detect_client_at(engine: Engine, path: PathBuf) -> ClientDetection {
    let mut command = Command::new(&path);
    command.arg("--version");
    let version = run_bounded(
        &mut command,
        Duration::from_secs(5),
        64 * 1024,
        64 * 1024,
    )
    .ok()
    .map(|v| v.trim().to_string())
    .filter(|v| !v.is_empty());
    ClientDetection {
        engine: engine.id().into(),
        executable: Some(path),
        version,
        available: true,
    }
}
fn client_name'''
)
regex_once(
    "crates/vsn-database-cli/src/lib.rs",
    r'fn validate_spec\(spec: &ConnectionSpec\) -> Result<\(\), CliDatabaseError> \{.*?\n\}\nfn enforce_read_only_sql',
    r'''fn validate_spec(spec: &ConnectionSpec) -> Result<(), CliDatabaseError> {
    for value in [
        spec.host.as_deref(),
        spec.user.as_deref(),
        spec.database.as_deref(),
        spec.service.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if value.len() > 512 || value.contains('\0') || value.contains('\n') || value.contains('\r') {
            return Err(CliDatabaseError::Invalid("unsafe connection field".into()));
        }
    }
    if spec.port == Some(0) {
        return Err(CliDatabaseError::Invalid("database port 0 is invalid".into()));
    }
    if let Some(host) = spec.host.as_deref() {
        if host.is_empty()
            || host.chars().any(char::is_whitespace)
            || host.contains('@')
            || host.contains('/')
            || host.contains('?')
            || host.contains('#')
            || host.contains(',')
            || (host.contains(':') && host != "::1")
        {
            return Err(CliDatabaseError::Invalid(
                "database host must be one unambiguous hostname/address".into(),
            ));
        }
    }
    if spec.service.is_some() && spec.host.is_none() {
        return Err(CliDatabaseError::Invalid(
            "database service profiles require an explicit host for transport verification".into(),
        ));
    }
    for (label, path) in [
        ("credential file", spec.credential_file.as_ref()),
        ("root CA file", spec.root_ca_file.as_ref()),
    ] {
        if let Some(path) = path {
            if !path.is_file() {
                return Err(CliDatabaseError::Invalid(format!("{label} not found")));
            }
        }
    }
    if remote_host(spec) && !use_verified_tls(spec) {
        return Err(CliDatabaseError::Invalid(
            "remote database profiles require a trusted root CA and verified TLS".into(),
        ));
    }
    Ok(())
}
fn enforce_read_only_sql'''
)
regex_once(
    "crates/vsn-database-cli/src/lib.rs",
    r'fn run\(command: &mut Command\) -> Result<String, CliDatabaseError> \{.*?\n\}\nfn parse_tabular',
    r'''fn drain_pipe<R: std::io::Read + Send + 'static>(
    mut reader: R,
    max: usize,
) -> std::thread::JoinHandle<Result<(Vec<u8>, bool), std::io::Error>> {
    std::thread::spawn(move || {
        let mut kept = Vec::with_capacity(max.min(64 * 1024));
        let mut buffer = [0u8; 16 * 1024];
        let mut overflow = false;
        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            let remaining = max.saturating_add(1).saturating_sub(kept.len());
            if remaining > 0 {
                kept.extend_from_slice(&buffer[..n.min(remaining)]);
            }
            if kept.len() > max {
                overflow = true;
            }
        }
        Ok((kept, overflow))
    })
}

fn run_bounded(
    command: &mut Command,
    timeout: Duration,
    stdout_max: usize,
    stderr_max: usize,
) -> Result<String, CliDatabaseError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| CliDatabaseError::Client(e.to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| CliDatabaseError::Client("database client stdout unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| CliDatabaseError::Client("database client stderr unavailable".into()))?;
    let stdout_reader = drain_pipe(stdout, stdout_max);
    let stderr_reader = drain_pipe(stderr, stderr_max);
    let started = std::time::Instant::now();
    let status = loop {
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(CliDatabaseError::Client(format!(
                "database client exceeded {} second timeout",
                timeout.as_secs()
            )));
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(CliDatabaseError::Client(error.to_string()));
            }
        }
    };
    let (stdout, stdout_overflow) = stdout_reader
        .join()
        .map_err(|_| CliDatabaseError::Client("database client stdout reader panicked".into()))?
        .map_err(|e| CliDatabaseError::Client(e.to_string()))?;
    let (stderr, stderr_overflow) = stderr_reader
        .join()
        .map_err(|_| CliDatabaseError::Client("database client stderr reader panicked".into()))?
        .map_err(|e| CliDatabaseError::Client(e.to_string()))?;
    if stdout_overflow {
        return Err(CliDatabaseError::Client(format!(
            "database output exceeded {} KiB limit",
            stdout_max / 1024
        )));
    }
    if stderr_overflow {
        return Err(CliDatabaseError::Client(format!(
            "database stderr exceeded {} KiB limit",
            stderr_max / 1024
        )));
    }
    if !status.success() {
        return Err(CliDatabaseError::Client(
            String::from_utf8_lossy(&stderr).trim().to_string(),
        ));
    }
    String::from_utf8(stdout).map_err(|_| CliDatabaseError::Encoding)
}

fn run(command: &mut Command) -> Result<String, CliDatabaseError> {
    run_bounded(
        command,
        Duration::from_secs(30),
        MAX_OUTPUT_BYTES,
        256 * 1024,
    )
}
fn parse_tabular'''
)
replace_once(
    "crates/vsn-database-cli/src/lib.rs",
    '''            if let Some(file) = &spec.credential_file {
                command.env("PGPASSFILE", file);
            }
            Ok(command)
''',
    '''            if let Some(file) = &spec.credential_file {
                command.env("PGPASSFILE", file);
            }
            apply_postgres_transport(&mut command, spec);
            Ok(command)
'''
)
replace_once(
    "crates/vsn-database-cli/src/lib.rs",
    '''            if let Some(db) = &spec.database {
                command.arg(format!("--database={db}"));
            }
            command.args(["--execute", statement]);
''',
    '''            if let Some(db) = &spec.database {
                command.arg(format!("--database={db}"));
            }
            apply_mysql_transport(&mut command, spec);
            command.args(["--execute", statement]);
'''
)
replace_once(
    "crates/vsn-database-cli/src/lib.rs",
    '''    fn side_effect_clauses_are_rejected() {
        assert!(enforce_read_only_sql("SELECT * INTO backup FROM users").is_err());
        assert!(enforce_read_only_sql("SELECT * FROM users FOR UPDATE").is_err());
        assert!(enforce_read_only_sql("EXPLAIN ANALYZE DELETE FROM users").is_err());
    }
}
''',
    r'''    fn side_effect_clauses_are_rejected() {
        assert!(enforce_read_only_sql("SELECT * INTO backup FROM users").is_err());
        assert!(enforce_read_only_sql("SELECT * FROM users FOR UPDATE").is_err());
        assert!(enforce_read_only_sql("EXPLAIN ANALYZE DELETE FROM users").is_err());
    }

    #[test]
    fn exact_loopback_and_remote_tls_policy_is_fail_closed() {
        let base = ConnectionSpec {
            engine: Engine::Postgresql,
            host: Some("localhost".into()),
            port: Some(5432),
            user: None,
            database: None,
            credential_file: None,
            root_ca_file: None,
            service: None,
        };
        assert!(validate_spec(&base).is_ok());
        for host in [
            "localhost.evil.invalid",
            "127.0.0.1.evil.invalid",
            "user@localhost",
            "db1.example.com,db2.example.com",
        ] {
            let mut spec = base.clone();
            spec.host = Some(host.into());
            assert!(validate_spec(&spec).is_err(), "{host}");
        }
        let mut zero = base.clone();
        zero.port = Some(0);
        assert!(validate_spec(&zero).is_err());

        let ca = std::env::temp_dir().join(format!("vsn-0226-ca-{}.pem", std::process::id()));
        std::fs::write(&ca, b"fixture").unwrap();
        let mut remote = base;
        remote.host = Some("db.example.com".into());
        assert!(validate_spec(&remote).is_err());
        remote.root_ca_file = Some(ca.clone());
        assert!(validate_spec(&remote).is_ok());

        let mut pg = Command::new("psql");
        apply_postgres_transport(&mut pg, &remote);
        let pg_debug = format!("{pg:?}");
        assert!(pg_debug.contains("PGSSLMODE"));
        assert!(pg_debug.contains("verify-full"));
        assert!(pg_debug.contains("PGSSLROOTCERT"));

        let mut mysql_spec = remote.clone();
        mysql_spec.engine = Engine::Mysql;
        let mut mysql = Command::new("mysql");
        apply_mysql_transport(&mut mysql, &mysql_spec);
        let mysql_debug = format!("{mysql:?}");
        assert!(mysql_debug.contains("--ssl-mode=VERIFY_IDENTITY"));
        assert!(mysql_debug.contains("--ssl-ca="));

        let mut maria_spec = remote.clone();
        maria_spec.engine = Engine::Mariadb;
        let mut maria = Command::new("mariadb");
        apply_mysql_transport(&mut maria, &maria_spec);
        let maria_debug = format!("{maria:?}");
        assert!(maria_debug.contains("--ssl-verify-server-cert"));

        let mut mongo = Command::new("mongosh");
        apply_mongo_transport(&mut mongo, &remote);
        let mongo_debug = format!("{mongo:?}");
        assert!(mongo_debug.contains("--tls"));
        assert!(mongo_debug.contains("--tlsCAFile"));

        let mut redis = Command::new("redis-cli");
        apply_redis_transport(&mut redis, &remote);
        let redis_debug = format!("{redis:?}");
        assert!(redis_debug.contains("--tls"));
        assert!(redis_debug.contains("--cacert"));
        let _ = std::fs::remove_file(ca);
    }

    #[cfg(windows)]
    #[test]
    fn bounded_child_times_out_and_drains_high_output() {
        let mut slow = Command::new("cmd.exe");
        slow.args(["/C", "ping -n 8 127.0.0.1 >nul"]);
        let started = std::time::Instant::now();
        let error = run_bounded(
            &mut slow,
            std::time::Duration::from_millis(300),
            64 * 1024,
            64 * 1024,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("timeout"));
        assert!(started.elapsed() < std::time::Duration::from_secs(4));

        let mut noisy = Command::new("cmd.exe");
        noisy.args(["/C", "for /L %i in (1,1,70000) do @echo 01234567890123456789"]);
        let error = run_bounded(
            &mut noisy,
            std::time::Duration::from_secs(10),
            64 * 1024,
            64 * 1024,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("exceeded"));
    }
}
'''
)

# vsn-database-native structural loopback/TLS/budget
replace_once(
    "crates/vsn-database-native/src/lib.rs",
    '''const MAX_ROWS: u32 = 1000;
const MAX_MUTATION_FIELDS: usize = 128;
''',
    '''const MAX_ROWS: u32 = 1000;
const MAX_MUTATION_FIELDS: usize = 128;
const MAX_TEXT_CELL_BYTES: usize = 256 * 1024;
const MAX_SERIALIZED_READ_BYTES: usize = 512 * 1024;
'''
)
replace_once(
    "crates/vsn-database-native/src/lib.rs",
    '''use postgres::{
    config::SslMode, types::ToSql, Client as PgClient, Config as PgConfig, NoTls,
    SimpleQueryMessage,
};
''',
    '''use postgres::{
    config::{Host as PgHost, SslMode}, types::ToSql, Client as PgClient, Config as PgConfig, NoTls,
    SimpleQueryMessage,
};
'''
)
replace_once(
    "crates/vsn-database-native/src/lib.rs",
    '''    let ssl = mysql::SslOpts::default().with_root_cert_path(Some(ca));
''',
    '''    let ssl = mysql::SslOpts::default()
        .with_root_cert_path(Some(ca))
        .with_danger_skip_domain_validation(false)
        .with_danger_accept_invalid_certs(false);
'''
)
regex_once(
    "crates/vsn-database-native/src/lib.rs",
    r'fn mysql_loopback\(url: &str\) -> bool \{.*?\n\}\nfn mysql_quote_ident',
    r'''fn mysql_loopback(url: &str) -> bool {
    let Ok(opts) = mysql::Opts::from_url(url) else {
        return false;
    };
    exact_loopback_host(opts.get_ip_or_hostname().as_ref())
        && opts.get_tcp_port() != 0
        && opts.get_ssl_opts().is_none()
}

fn exact_loopback_host(host: &str) -> bool {
    matches!(host.to_ascii_lowercase().as_str(), "localhost" | "127.0.0.1" | "::1")
}
fn mysql_quote_ident'''
)
regex_once(
    "crates/vsn-database-native/src/lib.rs",
    r'fn postgres_loopback_no_tls\(s: &str\) -> bool \{.*?\n\}\nfn simple_grid',
    r'''fn postgres_loopback_no_tls(s: &str) -> bool {
    let Ok(config) = PgConfig::from_str(s) else {
        return false;
    };
    if config.get_hosts().len() != 1
        || !config.get_hostaddrs().is_empty()
        || config.get_ports().iter().any(|port| *port == 0)
    {
        return false;
    }
    matches!(
        config.get_hosts().first(),
        Some(PgHost::Tcp(host)) if exact_loopback_host(host)
    )
}
fn simple_grid'''
)
replace_once(
    "crates/vsn-database-native/src/lib.rs",
    '''fn simple_grid(c: &mut PgClient, sql: &str) -> Result<NativeGrid, NativeDbError> {
''',
    r'''fn ensure_json_value_budget(value: &Value) -> Result<(), NativeDbError> {
    fn walk(value: &Value) -> bool {
        match value {
            Value::String(text) => text.len() <= MAX_TEXT_CELL_BYTES,
            Value::Array(values) => values.iter().all(walk),
            Value::Object(values) => values.values().all(walk),
            _ => true,
        }
    }
    if !walk(value) {
        return Err(NativeDbError::Invalid(
            "native database text cell exceeded 256 KiB limit".into(),
        ));
    }
    let size = serde_json::to_vec(value)
        .map_err(|e| NativeDbError::Invalid(format!("native result encode failed: {e}")))?
        .len();
    if size > MAX_SERIALIZED_READ_BYTES {
        return Err(NativeDbError::Invalid(
            "native database serialized read result exceeded 512 KiB limit".into(),
        ));
    }
    Ok(())
}

fn ensure_serialized_budget<T: Serialize>(value: &T) -> Result<(), NativeDbError> {
    let size = serde_json::to_vec(value)
        .map_err(|e| NativeDbError::Invalid(format!("native result encode failed: {e}")))?
        .len();
    if size > MAX_SERIALIZED_READ_BYTES {
        return Err(NativeDbError::Invalid(
            "native database serialized read result exceeded 512 KiB limit".into(),
        ));
    }
    Ok(())
}

fn bounded_read_result<T: Serialize>(value: T) -> Result<T, NativeDbError> {
    let json = serde_json::to_value(&value)
        .map_err(|e| NativeDbError::Invalid(format!("native result encode failed: {e}")))?;
    ensure_json_value_budget(&json)?;
    Ok(value)
}

fn simple_grid(c: &mut PgClient, sql: &str) -> Result<NativeGrid, NativeDbError> {
'''
)
replace_once(
    "crates/vsn-database-native/src/lib.rs",
    '''                    row.get(i)
                        .map(|v| Value::String(v.to_string()))
                        .unwrap_or(Value::Null),
''',
    '''                    row.get(i)
                        .map(|v| {
                            if v.len() > MAX_TEXT_CELL_BYTES {
                                Err(NativeDbError::Invalid(
                                    "native database text cell exceeded 256 KiB limit".into(),
                                ))
                            } else {
                                Ok(Value::String(v.to_string()))
                            }
                        })
                        .transpose()?
                        .unwrap_or(Value::Null),
'''
)
replace_once(
    "crates/vsn-database-native/src/lib.rs",
    '''    Ok(NativeGrid {
        columns,
        row_count: rows.len() as u64,
        rows,
    })
}
fn scalar''',
    '''    let grid = NativeGrid {
        columns,
        row_count: rows.len() as u64,
        rows,
    };
    ensure_serialized_budget(&grid)?;
    Ok(grid)
}
fn scalar'''
)
replace_once(
    "crates/vsn-database-native/src/lib.rs",
    '''            obj.insert(name.clone(), mysql_value_json(row[i].clone()));
''',
    '''            let value = mysql_value_json(row[i].clone());
            ensure_json_value_budget(&value)?;
            obj.insert(name.clone(), value);
'''
)
replace_once(
    "crates/vsn-database-native/src/lib.rs",
    '''    Ok(NativeGrid {
        columns,
        row_count: out.len() as u64,
        rows: out,
    })
}
fn mysql_value_json''',
    '''    let grid = NativeGrid {
        columns,
        row_count: out.len() as u64,
        rows: out,
    };
    ensure_serialized_budget(&grid)?;
    Ok(grid)
}
fn mysql_value_json'''
)
replace_once(
    "crates/vsn-database-native/src/lib.rs",
    '''fn document_to_json(value: Document) -> Result<Value, NativeDbError> {
    serde_json::to_value(value).map_err(|e| NativeDbError::Mongo(e.to_string()))
}
''',
    '''fn document_to_json(value: Document) -> Result<Value, NativeDbError> {
    let value = serde_json::to_value(value).map_err(|e| NativeDbError::Mongo(e.to_string()))?;
    ensure_json_value_budget(&value)?;
    Ok(value)
}
'''
)
replace_once(
    "crates/vsn-database-native/src/lib.rs",
    '''    Ok(NativeGrid {
        columns: columns.into_iter().collect(),
        row_count: rows.len() as u64,
        rows,
    })
}
pub fn mongo_insert''',
    '''    let grid = NativeGrid {
        columns: columns.into_iter().collect(),
        row_count: rows.len() as u64,
        rows,
    };
    ensure_serialized_budget(&grid)?;
    Ok(grid)
}
pub fn mongo_insert'''
)
replace_once(
    "crates/vsn-database-native/src/lib.rs",
    '''    Ok(json!({"key":key,"type":kind,"value":redis_value_json(value)}))
}
''',
    '''    let result = json!({"key":key,"type":kind,"value":redis_value_json(value)});
    ensure_json_value_budget(&result)?;
    Ok(result)
}
'''
)
replace_once(
    "crates/vsn-database-native/src/lib.rs",
    '''    if url.starts_with("mongodb+srv://") {
        return Ok(());
    }
''',
    '''    let lower = url.to_ascii_lowercase();
    for forbidden in [
        "tls=false",
        "ssl=false",
        "tlsinsecure=true",
        "tlsallowinvalidhostnames=true",
        "tlsallowinvalidcertificates=true",
    ] {
        if lower.contains(forbidden) {
            return Err(NativeDbError::Invalid(format!(
                "MongoDB insecure TLS option is forbidden: {forbidden}"
            )));
        }
    }
    if lower.starts_with("mongodb+srv://") {
        return Ok(());
    }
'''
)
regex_once(
    "crates/vsn-database-native/src/lib.rs",
    r'fn validate_redis_url\(url: &str\) -> Result<\(\), NativeDbError> \{.*?\n\}\nfn safe_key',
    r'''fn validate_redis_url(url: &str) -> Result<(), NativeDbError> {
    if url.len() > 4096 || url.chars().any(char::is_control) {
        return Err(NativeDbError::Invalid("Redis URL is invalid".into()));
    }
    let lower = url.to_ascii_lowercase();
    if lower.contains("#insecure") || lower.contains("insecure=true") {
        return Err(NativeDbError::Invalid(
            "Redis insecure TLS mode is forbidden".into(),
        ));
    }
    if lower.starts_with("rediss://") {
        return Ok(());
    }
    if !lower.starts_with("redis://") {
        return Err(NativeDbError::Invalid(
            "Redis URL must use redis:// or rediss://".into(),
        ));
    }
    let authority = lower
        .trim_start_matches("redis://")
        .rsplit('@')
        .next()
        .unwrap_or("")
        .split('/')
        .next()
        .unwrap_or("");
    let host = if let Some(rest) = authority.strip_prefix('[') {
        rest.split(']').next().unwrap_or("")
    } else {
        authority.split_once(':').map(|(host, _)| host).unwrap_or(authority)
    };
    if exact_loopback_host(host) {
        let port = if let Some(rest) = authority.strip_prefix('[') {
            rest.split_once("]: ").and_then(|(_, port)| port.parse::<u16>().ok())
        } else {
            authority.split_once(':').and_then(|(_, port)| port.parse::<u16>().ok())
        };
        if port == Some(0) {
            return Err(NativeDbError::Invalid("Redis port 0 is invalid".into()));
        }
        Ok(())
    } else {
        Err(NativeDbError::Invalid(
            "remote Redis must use rediss://; plaintext redis:// is restricted to exact loopback".into(),
        ))
    }
}
fn safe_key'''
)
replace_once(
    "crates/vsn-database-native/src/lib.rs",
    '''        assert!(!postgres_loopback_no_tls(
            "host=db.example.com user=postgres"
        ));
        assert!(mysql_loopback("mysql://root@localhost/test"));
        assert!(!mysql_loopback("mysql://root@db.example.com/test"));
''',
    '''        assert!(!postgres_loopback_no_tls(
            "host=db.example.com user=postgres"
        ));
        assert!(!postgres_loopback_no_tls(
            "host=localhost.evil.invalid user=postgres"
        ));
        assert!(!postgres_loopback_no_tls(
            "host=localhost,db.example.com user=postgres"
        ));
        assert!(!postgres_loopback_no_tls("host=localhost port=0 user=postgres"));
        assert!(mysql_loopback("mysql://root@localhost/test"));
        assert!(!mysql_loopback("mysql://root@db.example.com/test"));
        assert!(!mysql_loopback("mysql://root@localhost.evil.invalid/test"));
        assert!(!mysql_loopback("mysql://root@127.0.0.1.evil.invalid/test"));
        assert!(!mysql_loopback("mysql://root@localhost:0/test"));
'''
)
replace_once(
    "crates/vsn-database-native/src/lib.rs",
    '''        assert!(validate_redis_url("rediss://db.example.com:6380/0").is_ok());
    }

    #[test]
    fn update_delete_require_filter() {
''',
    '''        assert!(validate_redis_url("rediss://db.example.com:6380/0").is_ok());
        assert!(validate_redis_url("rediss://db.example.com:6380/0#insecure").is_err());
        assert!(validate_redis_url("redis://localhost.evil.invalid:6379/0").is_err());
        assert!(validate_redis_url("redis://localhost:0/0").is_err());
    }

    #[test]
    fn mongo_remote_tls_cannot_be_disabled() {
        assert!(validate_mongo_url("mongodb+srv://db.example.com/app").is_ok());
        assert!(validate_mongo_url("mongodb+srv://db.example.com/app?tls=false").is_err());
        assert!(validate_mongo_url(
            "mongodb+srv://db.example.com/app?tlsAllowInvalidCertificates=true"
        )
        .is_err());
        assert!(validate_mongo_url("mongodb://localhost:27017/app").is_ok());
        assert!(validate_mongo_url("mongodb://localhost.evil.invalid:27017/app").is_err());
    }

    #[test]
    fn native_result_budgets_reject_large_cells_and_results() {
        assert!(ensure_json_value_budget(&Value::String("x".repeat(MAX_TEXT_CELL_BYTES))).is_ok());
        assert!(ensure_json_value_budget(&Value::String("x".repeat(MAX_TEXT_CELL_BYTES + 1))).is_err());
        let value = json!({"rows": vec!["x".repeat(1024); 600]});
        assert!(ensure_json_value_budget(&value).is_err());
    }

    #[test]
    fn update_delete_require_filter() {
'''
)

# Bound inspection/read aggregate objects that are not returned through NativeGrid helpers.
text = read("crates/vsn-database-native/src/lib.rs")
for ctor, expected in [
    ("Ok(PostgresInspection {", 2),
    ("Ok(MySqlInspection {", 2),
    ("Ok(MongoInspection {", 1),
    ("Ok(RedisInspection {", 1),
]:
    count = text.count(ctor)
    if count != expected:
        raise SystemExit(f"native: expected {expected} occurrences of {ctor}, got {count}")
    text = text.replace(ctor, ctor.replace("Ok(", "bounded_read_result("))
write("crates/vsn-database-native/src/lib.rs", text)

# Core file containment
regex_once(
    "crates/vsn-core/src/lib.rs",
    r'fn validate_database_credential_file\(\n    spec: &vsn_database_cli::ConnectionSpec,\n\) -> Result<\(\), CoreError> \{.*?\n\}\npub fn database_cli_inspect',
    r'''fn validate_database_file_path(path: &Path, label: &str) -> Result<(), CoreError> {
    let requested = path
        .canonicalize()
        .map_err(|e| CoreError::Rejected(format!("{label} unavailable: {e}")))?;
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
    Err(CoreError::Rejected(format!(
        "{label} must be inside a configured workspace or VSN-owned data directory"
    )))
}

fn validate_database_connection_files(
    spec: &vsn_database_cli::ConnectionSpec,
) -> Result<(), CoreError> {
    if let Some(path) = spec.credential_file.as_ref() {
        validate_database_file_path(path, "database credential file")?;
    }
    if let Some(path) = spec.root_ca_file.as_ref() {
        validate_database_file_path(path, "database TLS root CA file")?;
    }
    Ok(())
}
pub fn database_cli_inspect'''
)
text = read("crates/vsn-core/src/lib.rs")
count = text.count("validate_database_credential_file(spec)?;")
if count != 3:
    raise SystemExit(f"core: expected 3 credential validation calls, got {count}")
write("crates/vsn-core/src/lib.rs", text.replace(
    "validate_database_credential_file(spec)?;",
    "validate_database_connection_files(spec)?;"
))
for fn_name, field in [
    ("postgres_tls_inspect", "root_ca_pem_path"),
    ("postgres_tls_browse", "root_ca_pem_path"),
    ("postgres_tls_query", "root_ca_pem_path"),
    ("mysql_tls_inspect", "root_ca_path"),
    ("mysql_tls_browse", "root_ca_path"),
    ("mysql_tls_query", "root_ca_path"),
]:
    text = read("crates/vsn-core/src/lib.rs")
    pattern = rf'(pub fn {fn_name}\(.*?\{{\n    vsn_policy::require\(principal, Permission::Database(?:View|Query)\)\?;\n)'
    replacement = rf'''\1    validate_database_file_path(Path::new(&spec.{field}), "database TLS root CA file")?;
'''
    text2, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"core: failed to add CA containment to {fn_name}")
    write("crates/vsn-core/src/lib.rs", text2)

# CLI public TLS routes
insert_after = '''        [cmd, sub] if cmd == "db" && sub == "clients" => call("database.cli.detect", json!({}))?,
'''
tls_routes = r'''        [cmd, sub, engine, host, port, user, database, root_ca]
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
        [cmd, sub, connection, root_ca, schema, table]
            if cmd == "db" && sub == "pg-tls-browse" => call(
                "database.tls.postgres.browse",
                json!({"connection_string":connection,"root_ca_pem_path":root_ca,"schema":schema,"table":table,"limit":100,"offset":0}),
            )?,
        [cmd, sub, connection, root_ca, sql] if cmd == "db" && sub == "pg-tls-query" => call(
            "database.tls.postgres.query",
            json!({"connection_string":connection,"root_ca_pem_path":root_ca,"sql":sql}),
        )?,
        [cmd, sub, url, root_ca] if cmd == "db" && sub == "mysql-tls-inspect" => call(
            "database.tls.mysql.inspect",
            json!({"url":url,"root_ca_path":root_ca}),
        )?,
        [cmd, sub, url, root_ca, database, table]
            if cmd == "db" && sub == "mysql-tls-browse" => call(
                "database.tls.mysql.browse",
                json!({"url":url,"root_ca_path":root_ca,"database":database,"table":table,"limit":100,"offset":0}),
            )?,
        [cmd, sub, url, root_ca, sql] if cmd == "db" && sub == "mysql-tls-query" => call(
            "database.tls.mysql.query",
            json!({"url":url,"root_ca_path":root_ca,"sql":sql}),
        )?,
'''
replace_once("apps/cli/src/main.rs", insert_after, insert_after + tls_routes)
marker = '''fn db_connection_json(
'''
tls_helper = r'''fn db_connection_tls_json(
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
'''
replace_once("apps/cli/src/main.rs", marker, tls_helper + marker)
replace_once(
    "apps/cli/src/main.rs",
    '''        json!({"engine":engine,"host":opt_arg(host),"port":port,"user":opt_arg(user),"database":opt_arg(database),"credential_file":credential.and_then(|v|opt_arg(v)),"service":null}),
''',
    '''        json!({"engine":engine,"host":opt_arg(host),"port":port,"user":opt_arg(user),"database":opt_arg(database),"credential_file":credential.and_then(|v|opt_arg(v)),"root_ca_file":null,"service":null}),
'''
)

# focused integration tests
write(
    "crates/vsn-database/tests/pkg02_0226_capabilities.rs",
    r'''use vsn_database::{remote_database_capabilities, validate_remote_database_capabilities};

#[test]
fn external_native_beta_matrix_is_exact_and_truthful() {
    let report = validate_remote_database_capabilities();
    assert!(report.valid, "{:?}", report.issues);
    let engines = report
        .engines
        .iter()
        .map(|engine| engine.engine.as_str())
        .collect::<Vec<_>>();
    assert_eq!(engines, ["postgresql", "mysql", "mariadb", "mongodb", "redis"]);
    assert!(report
        .engines
        .iter()
        .all(|engine| engine.plaintext_loopback && engine.verified_tls_remote));
    assert!(!report.engines.iter().any(|engine| engine.engine == "sqlite"));
    let maria = report.engines.iter().find(|engine| engine.engine == "mariadb").unwrap();
    assert!(maria.query);
    assert!(!maria.write);
    let mongo = report.engines.iter().find(|engine| engine.engine == "mongodb").unwrap();
    assert!(mongo.browse && mongo.write && !mongo.query);
    let redis = report.engines.iter().find(|engine| engine.engine == "redis").unwrap();
    assert!(redis.write && !redis.query && !redis.browse);
    assert_eq!(remote_database_capabilities().len(), 5);
}
'''
)
write(
    "crates/vsn-core/tests/pkg02_0226_policy.rs",
    r'''use vsn_policy::{require, Permission, Principal};

#[test]
fn database_permission_boundary_is_preserved() {
    let local = Principal::local_authenticated();
    assert!(require(&local, Permission::DatabaseView).is_ok());
    assert!(require(&local, Permission::DatabaseQuery).is_ok());
    assert!(require(&local, Permission::DatabaseWrite).is_ok());
    assert!(require(&local, Permission::DatabaseDestructive).is_err());
    assert!(Principal::remote_delegated("remote", Permission::DatabaseDestructive).is_err());
}
'''
)
