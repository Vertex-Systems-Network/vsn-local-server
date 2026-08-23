use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

const MAX_OUTPUT_BYTES: u64 = 64 * 1024;
const MAX_TIMEOUT_MS: u64 = 60_000;
const MAX_SESSION_BUFFER: usize = 1024 * 1024;
const MAX_SESSION_READ: usize = 256 * 1024;
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);
#[derive(Debug, Error)]
pub enum TerminalError {
    #[error("terminal request rejected: {0}")]
    Invalid(String),
    #[error("terminal process failed: {0}")]
    Process(String),
    #[error("workspace check failed: {0}")]
    Workspace(#[from] vsn_files::FileError),
    #[error("system lookup failed: {0}")]
    System(#[from] vsn_system::SystemError),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecRequest {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: PathBuf,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}
fn default_timeout() -> u64 {
    30_000
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecResult {
    pub program: PathBuf,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub stdout: String,
    pub stderr: String,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub duration_ms: u128,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionStartRequest {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: PathBuf,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionState {
    pub session_id: String,
    pub program: PathBuf,
    pub cwd: PathBuf,
    pub running: bool,
    pub exit_code: Option<i32>,
    pub started_at_unix_ms: u128,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionChunk {
    pub session_id: String,
    pub stdout: String,
    pub stderr: String,
    pub stdout_dropped_bytes: u64,
    pub stderr_dropped_bytes: u64,
    pub running: bool,
    pub exit_code: Option<i32>,
}
#[derive(Default)]
struct OutputBuffer {
    bytes: VecDeque<u8>,
    dropped: u64,
}
impl OutputBuffer {
    fn push(&mut self, data: &[u8]) {
        for b in data {
            self.bytes.push_back(*b);
        }
        while self.bytes.len() > MAX_SESSION_BUFFER {
            self.bytes.pop_front();
            self.dropped = self.dropped.saturating_add(1);
        }
    }
    fn drain(&mut self, max: usize) -> (Vec<u8>, u64) {
        let n = max.min(self.bytes.len());
        let out = self.bytes.drain(..n).collect::<Vec<_>>();
        let dropped = self.dropped;
        self.dropped = 0;
        (out, dropped)
    }
}
struct TerminalSession {
    child: Child,
    stdin: ChildStdin,
    stdout: Arc<Mutex<OutputBuffer>>,
    stderr: Arc<Mutex<OutputBuffer>>,
    program: PathBuf,
    cwd: PathBuf,
    started_at_unix_ms: u128,
    exit_code: Option<i32>,
}
static SESSIONS: OnceLock<Mutex<HashMap<String, TerminalSession>>> = OnceLock::new();
fn sessions() -> &'static Mutex<HashMap<String, TerminalSession>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn execute(roots: &[PathBuf], request: &ExecRequest) -> Result<ExecResult, TerminalError> {
    validate_request(
        &request.program,
        &request.args,
        &request.cwd,
        &request.env,
        roots,
    )?;
    let cwd = vsn_files::resolve_existing(roots, &request.cwd)?;
    let program = resolve_program(roots, &cwd, &request.program)?;
    let timeout_ms = request.timeout_ms.clamp(100, MAX_TIMEOUT_MS);
    let mut command = Command::new(&program);
    command
        .args(&request.args)
        .current_dir(&cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in &request.env {
        command.env(k, v);
    }
    let mut child = command
        .spawn()
        .map_err(|e| TerminalError::Process(e.to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| TerminalError::Process("stdout unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| TerminalError::Process("stderr unavailable".into()))?;
    let out_thread = thread::spawn(move || read_bounded(stdout));
    let err_thread = thread::spawn(move || read_bounded(stderr));
    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| TerminalError::Process(e.to_string()))?
        {
            break status;
        }
        if started.elapsed() >= Duration::from_millis(timeout_ms) {
            timed_out = true;
            let _ = child.kill();
            break child
                .wait()
                .map_err(|e| TerminalError::Process(e.to_string()))?;
        }
        thread::sleep(Duration::from_millis(25));
    };
    let (stdout, stdout_truncated) = out_thread
        .join()
        .unwrap_or_else(|_| (b"output reader failed".to_vec(), false));
    let (stderr, stderr_truncated) = err_thread
        .join()
        .unwrap_or_else(|_| (b"error reader failed".to_vec(), false));
    Ok(ExecResult {
        program,
        exit_code: status.code(),
        timed_out,
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        stdout_truncated,
        stderr_truncated,
        duration_ms: started.elapsed().as_millis(),
    })
}

pub fn start_session(
    roots: &[PathBuf],
    request: &SessionStartRequest,
) -> Result<SessionState, TerminalError> {
    validate_request(
        &request.program,
        &request.args,
        &request.cwd,
        &request.env,
        roots,
    )?;
    let cwd = vsn_files::resolve_existing(roots, &request.cwd)?;
    let program = resolve_program(roots, &cwd, &request.program)?;
    let mut map = sessions()
        .lock()
        .map_err(|_| TerminalError::Process("terminal session lock poisoned".into()))?;
    if map.len() >= 64 {
        return Err(TerminalError::Invalid(
            "maximum 64 terminal sessions per agent".into(),
        ));
    }
    let mut cmd = Command::new(&program);
    cmd.args(&request.args)
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in &request.env {
        cmd.env(k, v);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| TerminalError::Process(e.to_string()))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| TerminalError::Process("stdin unavailable".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| TerminalError::Process("stdout unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| TerminalError::Process("stderr unavailable".into()))?;
    let out_buf = Arc::new(Mutex::new(OutputBuffer::default()));
    let err_buf = Arc::new(Mutex::new(OutputBuffer::default()));
    spawn_reader(stdout, Arc::clone(&out_buf));
    spawn_reader(stderr, Arc::clone(&err_buf));
    let id = format!(
        "term_{:x}_{:x}",
        now_ms(),
        SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let started = now_ms();
    let state = SessionState {
        session_id: id.clone(),
        program: program.clone(),
        cwd: cwd.clone(),
        running: true,
        exit_code: None,
        started_at_unix_ms: started,
    };
    map.insert(
        id,
        TerminalSession {
            child,
            stdin,
            stdout: out_buf,
            stderr: err_buf,
            program,
            cwd,
            started_at_unix_ms: started,
            exit_code: None,
        },
    );
    Ok(state)
}
pub fn write_session(session_id: &str, input: &str) -> Result<SessionState, TerminalError> {
    if input.len() > 256 * 1024 {
        return Err(TerminalError::Invalid(
            "terminal input chunk exceeds 256 KiB".into(),
        ));
    }
    let mut map = sessions()
        .lock()
        .map_err(|_| TerminalError::Process("terminal session lock poisoned".into()))?;
    let s = map
        .get_mut(session_id)
        .ok_or_else(|| TerminalError::Invalid("terminal session not found".into()))?;
    refresh_session(s)?;
    if s.exit_code.is_some() {
        return Err(TerminalError::Invalid(
            "terminal session is not running".into(),
        ));
    }
    s.stdin
        .write_all(input.as_bytes())
        .map_err(|e| TerminalError::Process(e.to_string()))?;
    s.stdin
        .flush()
        .map_err(|e| TerminalError::Process(e.to_string()))?;
    Ok(state_for(session_id, s))
}
pub fn read_session(session_id: &str, max_bytes: usize) -> Result<SessionChunk, TerminalError> {
    let mut map = sessions()
        .lock()
        .map_err(|_| TerminalError::Process("terminal session lock poisoned".into()))?;
    let s = map
        .get_mut(session_id)
        .ok_or_else(|| TerminalError::Invalid("terminal session not found".into()))?;
    refresh_session(s)?;
    let cap = max_bytes.clamp(1, MAX_SESSION_READ);
    let (out, out_drop) = s
        .stdout
        .lock()
        .map_err(|_| TerminalError::Process("stdout buffer lock poisoned".into()))?
        .drain(cap);
    let (err, err_drop) = s
        .stderr
        .lock()
        .map_err(|_| TerminalError::Process("stderr buffer lock poisoned".into()))?
        .drain(cap);
    Ok(SessionChunk {
        session_id: session_id.into(),
        stdout: String::from_utf8_lossy(&out).into_owned(),
        stderr: String::from_utf8_lossy(&err).into_owned(),
        stdout_dropped_bytes: out_drop,
        stderr_dropped_bytes: err_drop,
        running: s.exit_code.is_none(),
        exit_code: s.exit_code,
    })
}
pub fn session_state(session_id: &str) -> Result<SessionState, TerminalError> {
    let mut map = sessions()
        .lock()
        .map_err(|_| TerminalError::Process("terminal session lock poisoned".into()))?;
    let s = map
        .get_mut(session_id)
        .ok_or_else(|| TerminalError::Invalid("terminal session not found".into()))?;
    refresh_session(s)?;
    Ok(state_for(session_id, s))
}
pub fn stop_session(session_id: &str) -> Result<SessionState, TerminalError> {
    let mut map = sessions()
        .lock()
        .map_err(|_| TerminalError::Process("terminal session lock poisoned".into()))?;
    let s = map
        .get_mut(session_id)
        .ok_or_else(|| TerminalError::Invalid("terminal session not found".into()))?;
    refresh_session(s)?;
    if s.exit_code.is_none() {
        let _ = s.child.kill();
        let status = s
            .child
            .wait()
            .map_err(|e| TerminalError::Process(e.to_string()))?;
        s.exit_code = status.code().or(Some(-1));
    }
    Ok(state_for(session_id, s))
}
pub fn remove_session(session_id: &str) -> Result<bool, TerminalError> {
    let mut map = sessions()
        .lock()
        .map_err(|_| TerminalError::Process("terminal session lock poisoned".into()))?;
    if let Some(mut s) = map.remove(session_id) {
        if s.exit_code.is_none() {
            let _ = s.child.kill();
            let _ = s.child.wait();
        }
        Ok(true)
    } else {
        Ok(false)
    }
}
pub fn list_sessions() -> Result<Vec<SessionState>, TerminalError> {
    let mut map = sessions()
        .lock()
        .map_err(|_| TerminalError::Process("terminal session lock poisoned".into()))?;
    let ids = map.keys().cloned().collect::<Vec<_>>();
    let mut out = Vec::new();
    for id in ids {
        if let Some(s) = map.get_mut(&id) {
            refresh_session(s)?;
            out.push(state_for(&id, s));
        }
    }
    Ok(out)
}

fn refresh_session(s: &mut TerminalSession) -> Result<(), TerminalError> {
    if s.exit_code.is_none() {
        if let Some(status) = s
            .child
            .try_wait()
            .map_err(|e| TerminalError::Process(e.to_string()))?
        {
            s.exit_code = status.code().or(Some(-1));
        }
    }
    Ok(())
}
fn state_for(id: &str, s: &TerminalSession) -> SessionState {
    SessionState {
        session_id: id.into(),
        program: s.program.clone(),
        cwd: s.cwd.clone(),
        running: s.exit_code.is_none(),
        exit_code: s.exit_code,
        started_at_unix_ms: s.started_at_unix_ms,
    }
}
fn spawn_reader<R: Read + Send + 'static>(reader: R, buffer: Arc<Mutex<OutputBuffer>>) {
    spawn_reader_with_journal(reader, buffer, None)
}
fn spawn_reader_with_journal<R: Read + Send + 'static>(
    mut reader: R,
    buffer: Arc<Mutex<OutputBuffer>>,
    journal: Option<PathBuf>,
) {
    thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    if let Ok(mut b) = buffer.lock() {
                        b.push(&chunk[..n]);
                    } else {
                        break;
                    }
                    if let Some(path) = journal.as_ref() {
                        let _ = append_scrollback(path, &chunk[..n]);
                    }
                }
                Err(_) => break,
            }
        }
    });
}
fn append_scrollback(path: &Path, data: &[u8]) -> Result<(), TerminalError> {
    if data.is_empty() {
        return Ok(());
    }
    if data.len() > 256 * 1024 {
        return Err(TerminalError::Invalid(
            "PTY scrollback append exceeds 256 KiB".into(),
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| TerminalError::Invalid("PTY scrollback path has no parent".into()))?;
    std::fs::create_dir_all(parent).map_err(|e| TerminalError::Process(e.to_string()))?;
    harden_scrollback_dir(parent)?;
    let current = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if current >= MAX_PTY_SCROLLBACK_BYTES {
        return Ok(());
    }
    let remain = (MAX_PTY_SCROLLBACK_BYTES - current) as usize;
    let slice = &data[..data.len().min(remain)];
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| TerminalError::Process(e.to_string()))?;
    file.write_all(slice)
        .map_err(|e| TerminalError::Process(e.to_string()))?;
    file.flush()
        .map_err(|e| TerminalError::Process(e.to_string()))?;
    Ok(())
}
#[cfg(unix)]
fn harden_scrollback_dir(path: &Path) -> Result<(), TerminalError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| TerminalError::Process(e.to_string()))?;
    Ok(())
}
#[cfg(not(unix))]
fn harden_scrollback_dir(_path: &Path) -> Result<(), TerminalError> {
    Ok(())
}
fn validate_request(
    program: &str,
    args: &[String],
    cwd: &Path,
    env: &BTreeMap<String, String>,
    roots: &[PathBuf],
) -> Result<(), TerminalError> {
    if program.is_empty() || program.len() > 512 {
        return Err(TerminalError::Invalid("invalid program".into()));
    }
    if args.len() > 256 || args.iter().any(|v| v.len() > 16_384) {
        return Err(TerminalError::Invalid("argument limit exceeded".into()));
    }
    if env.len() > 128 {
        return Err(TerminalError::Invalid("environment limit exceeded".into()));
    }
    for (k, v) in env {
        if k.is_empty()
            || k.len() > 128
            || !k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || v.len() > 32_768
        {
            return Err(TerminalError::Invalid("unsafe environment entry".into()));
        }
    }
    let resolved = vsn_files::resolve_existing(roots, cwd)?;
    if !resolved.is_dir() {
        return Err(TerminalError::Invalid(
            "cwd must be a workspace directory".into(),
        ));
    }
    Ok(())
}
fn resolve_program(roots: &[PathBuf], cwd: &Path, value: &str) -> Result<PathBuf, TerminalError> {
    let requested = Path::new(value);
    if requested.is_absolute() {
        let program = vsn_files::resolve_existing(roots, requested)?;
        if !program.is_file() {
            return Err(TerminalError::Invalid("program path is not a file".into()));
        }
        return Ok(program);
    }
    if requested.components().count() > 1 {
        let program = vsn_files::resolve_existing(roots, &cwd.join(requested))?;
        if !program.is_file() {
            return Err(TerminalError::Invalid("program path is not a file".into()));
        }
        return Ok(program);
    }
    if !value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(TerminalError::Invalid("unsafe executable name".into()));
    }
    Ok(vsn_system::find_executable(value)?)
}
fn read_bounded<R: Read>(mut reader: R) -> (Vec<u8>, bool) {
    let mut bytes = Vec::with_capacity(MAX_OUTPUT_BYTES as usize);
    let mut truncated = false;
    let mut chunk = [0u8; 8192];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                let remaining = (MAX_OUTPUT_BYTES as usize).saturating_sub(bytes.len());
                let keep = remaining.min(n);
                if keep > 0 {
                    bytes.extend_from_slice(&chunk[..keep]);
                }
                if keep < n {
                    truncated = true;
                }
            }
            Err(_) => break,
        }
    }
    (bytes, truncated)
}
fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timeout_is_bounded() {
        assert_eq!(default_timeout(), 30_000);
    }
    #[test]
    fn terminal_session_read_cap_is_bounded() {
        assert_eq!(MAX_SESSION_READ, 256 * 1024);
    }
    #[test]
    fn read_bounded_drains_source_after_retention_cap() {
        let source_len = MAX_OUTPUT_BYTES as usize + 8192;
        let mut cursor = std::io::Cursor::new(vec![b'x'; source_len]);
        let (bytes, truncated) = read_bounded(&mut cursor);
        assert_eq!(cursor.position(), source_len as u64);
        assert!(truncated);
        assert_eq!(bytes.len(), MAX_OUTPUT_BYTES as usize);
    }
    #[test]
    fn direct_exec_json_budget_is_frame_safe_for_escaped_output() {
        let result = ExecResult {
            program: PathBuf::from("fixture"),
            exit_code: Some(0),
            timed_out: false,
            stdout: "\0".repeat(MAX_OUTPUT_BYTES as usize),
            stderr: "\u{1}".repeat(MAX_OUTPUT_BYTES as usize),
            stdout_truncated: true,
            stderr_truncated: true,
            duration_ms: 1,
        };
        let encoded = serde_json::to_vec(&result).expect("serialize direct exec result");
        assert!(encoded.len() < 900 * 1024);
    }
}

// True PTY/ConPTY sessions. These are kept separate from the pipe-backed sessions above so
// callers can explicitly choose terminal emulation when ANSI/interactive behavior is required.
use portable_pty::{native_pty_system, CommandBuilder as PtyCommandBuilder, MasterPty, PtySize};

const MAX_PTY_SESSIONS: usize = 32;
const MAX_PTY_SCROLLBACK_BYTES: u64 = 64 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PtySessionStartRequest {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: PathBuf,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default = "default_pty_rows")]
    pub rows: u16,
    #[serde(default = "default_pty_cols")]
    pub cols: u16,
}
fn default_pty_rows() -> u16 {
    24
}
fn default_pty_cols() -> u16 {
    80
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PtySessionState {
    pub session_id: String,
    pub program: PathBuf,
    pub cwd: PathBuf,
    pub running: bool,
    pub exit_code: Option<u32>,
    pub pid: Option<u32>,
    pub rows: u16,
    pub cols: u16,
    pub started_at_unix_ms: u128,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PtySessionChunk {
    pub session_id: String,
    pub output: String,
    pub dropped_bytes: u64,
    pub running: bool,
    pub exit_code: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PtyRecoveryInfo {
    pub session_id: String,
    pub program: PathBuf,
    pub cwd: PathBuf,
    pub pid: Option<u32>,
    pub rows: u16,
    pub cols: u16,
    pub started_at_unix_ms: u128,
    pub state: String,
    pub exit_code: Option<u32>,
    pub scrollback_file: Option<PathBuf>,
    #[serde(default)]
    pub orphaned: bool,
}
struct PtyTerminalSession {
    child: Box<dyn portable_pty::Child + Send>,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    output: Arc<Mutex<OutputBuffer>>,
    session_id: String,
    program: PathBuf,
    cwd: PathBuf,
    started_at_unix_ms: u128,
    exit_code: Option<u32>,
    rows: u16,
    cols: u16,
    pid: Option<u32>,
    recovery_path: Option<PathBuf>,
    scrollback_file: Option<PathBuf>,
}
static PTY_SESSIONS: OnceLock<Mutex<HashMap<String, PtyTerminalSession>>> = OnceLock::new();
fn pty_sessions() -> &'static Mutex<HashMap<String, PtyTerminalSession>> {
    PTY_SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn start_pty_session(
    roots: &[PathBuf],
    request: &PtySessionStartRequest,
) -> Result<PtySessionState, TerminalError> {
    start_pty_session_inner(roots, request, None)
}
pub fn start_pty_session_with_scrollback(
    roots: &[PathBuf],
    request: &PtySessionStartRequest,
    journal_dir: &Path,
) -> Result<PtySessionState, TerminalError> {
    start_pty_session_inner(roots, request, Some(journal_dir))
}
fn start_pty_session_inner(
    roots: &[PathBuf],
    request: &PtySessionStartRequest,
    journal_dir: Option<&Path>,
) -> Result<PtySessionState, TerminalError> {
    validate_request(
        &request.program,
        &request.args,
        &request.cwd,
        &request.env,
        roots,
    )?;
    if request.rows == 0 || request.cols == 0 || request.rows > 500 || request.cols > 1000 {
        return Err(TerminalError::Invalid(
            "PTY size is outside allowed bounds".into(),
        ));
    }
    let cwd = vsn_files::resolve_existing(roots, &request.cwd)?;
    let program = resolve_program(roots, &cwd, &request.program)?;
    let mut map = pty_sessions()
        .lock()
        .map_err(|_| TerminalError::Process("PTY session lock poisoned".into()))?;
    if map.len() >= MAX_PTY_SESSIONS {
        return Err(TerminalError::Invalid(format!(
            "maximum {MAX_PTY_SESSIONS} PTY sessions per agent"
        )));
    }
    let id = format!(
        "pty_{:x}_{:x}",
        now_ms(),
        SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let journal_path = if let Some(dir) = journal_dir {
        std::fs::create_dir_all(dir)
            .map_err(|e| TerminalError::Process(format!("PTY scrollback directory failed: {e}")))?;
        harden_scrollback_dir(dir).map_err(|e| TerminalError::Process(e.to_string()))?;
        let path = dir.join(format!("{id}.log"));
        let _ = std::fs::remove_file(&path);
        Some(path)
    } else {
        None
    };
    let recovery_path = journal_dir.map(|dir| dir.join(format!("{id}.json")));
    let system = native_pty_system();
    let pair = system
        .openpty(PtySize {
            rows: request.rows,
            cols: request.cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| TerminalError::Process(format!("open PTY failed: {e}")))?;
    let mut cmd = PtyCommandBuilder::new(&program);
    cmd.args(&request.args);
    cmd.cwd(&cwd);
    for (k, v) in &request.env {
        cmd.env(k, v);
    }
    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| TerminalError::Process(format!("PTY spawn failed: {e}")))?;
    drop(pair.slave);
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| TerminalError::Process(format!("PTY reader failed: {e}")))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| TerminalError::Process(format!("PTY writer failed: {e}")))?;
    let output = Arc::new(Mutex::new(OutputBuffer::default()));
    spawn_reader_with_journal(reader, Arc::clone(&output), journal_path.clone());
    let started = now_ms();
    let pid = child.process_id();
    let state = PtySessionState {
        session_id: id.clone(),
        program: program.clone(),
        cwd: cwd.clone(),
        running: true,
        exit_code: None,
        pid,
        rows: request.rows,
        cols: request.cols,
        started_at_unix_ms: started,
    };
    let session = PtyTerminalSession {
        session_id: id.clone(),
        child,
        master: pair.master,
        writer,
        output,
        program,
        cwd,
        started_at_unix_ms: started,
        exit_code: None,
        rows: request.rows,
        cols: request.cols,
        pid,
        recovery_path,
        scrollback_file: journal_path.clone(),
    };
    write_pty_recovery(&session, "running_at_last_checkpoint")?;
    map.insert(id, session);
    Ok(state)
}
pub fn write_pty_session(session_id: &str, input: &str) -> Result<PtySessionState, TerminalError> {
    if input.len() > 256 * 1024 {
        return Err(TerminalError::Invalid(
            "PTY input chunk exceeds 256 KiB".into(),
        ));
    }
    let mut map = pty_sessions()
        .lock()
        .map_err(|_| TerminalError::Process("PTY session lock poisoned".into()))?;
    let s = map
        .get_mut(session_id)
        .ok_or_else(|| TerminalError::Invalid("PTY session not found".into()))?;
    refresh_pty(s)?;
    if s.exit_code.is_some() {
        return Err(TerminalError::Invalid("PTY session is not running".into()));
    }
    s.writer
        .write_all(input.as_bytes())
        .map_err(|e| TerminalError::Process(e.to_string()))?;
    s.writer
        .flush()
        .map_err(|e| TerminalError::Process(e.to_string()))?;
    Ok(pty_state_for(session_id, s))
}
pub fn read_pty_session(
    session_id: &str,
    max_bytes: usize,
) -> Result<PtySessionChunk, TerminalError> {
    let mut map = pty_sessions()
        .lock()
        .map_err(|_| TerminalError::Process("PTY session lock poisoned".into()))?;
    let s = map
        .get_mut(session_id)
        .ok_or_else(|| TerminalError::Invalid("PTY session not found".into()))?;
    refresh_pty(s)?;
    let cap = max_bytes.clamp(1, MAX_SESSION_READ);
    let (out, dropped) = s
        .output
        .lock()
        .map_err(|_| TerminalError::Process("PTY output lock poisoned".into()))?
        .drain(cap);
    Ok(PtySessionChunk {
        session_id: session_id.into(),
        output: String::from_utf8_lossy(&out).into_owned(),
        dropped_bytes: dropped,
        running: s.exit_code.is_none(),
        exit_code: s.exit_code,
    })
}
pub fn resize_pty_session(
    session_id: &str,
    rows: u16,
    cols: u16,
) -> Result<PtySessionState, TerminalError> {
    if rows == 0 || cols == 0 || rows > 500 || cols > 1000 {
        return Err(TerminalError::Invalid(
            "PTY size is outside allowed bounds".into(),
        ));
    }
    let mut map = pty_sessions()
        .lock()
        .map_err(|_| TerminalError::Process("PTY session lock poisoned".into()))?;
    let s = map
        .get_mut(session_id)
        .ok_or_else(|| TerminalError::Invalid("PTY session not found".into()))?;
    refresh_pty(s)?;
    s.master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| TerminalError::Process(format!("PTY resize failed: {e}")))?;
    s.rows = rows;
    s.cols = cols;
    Ok(pty_state_for(session_id, s))
}
pub fn pty_session_state(session_id: &str) -> Result<PtySessionState, TerminalError> {
    let mut map = pty_sessions()
        .lock()
        .map_err(|_| TerminalError::Process("PTY session lock poisoned".into()))?;
    let s = map
        .get_mut(session_id)
        .ok_or_else(|| TerminalError::Invalid("PTY session not found".into()))?;
    refresh_pty(s)?;
    Ok(pty_state_for(session_id, s))
}
pub fn stop_pty_session(session_id: &str) -> Result<PtySessionState, TerminalError> {
    let mut map = pty_sessions()
        .lock()
        .map_err(|_| TerminalError::Process("PTY session lock poisoned".into()))?;
    let s = map
        .get_mut(session_id)
        .ok_or_else(|| TerminalError::Invalid("PTY session not found".into()))?;
    refresh_pty(s)?;
    if s.exit_code.is_none() {
        s.child
            .kill()
            .map_err(|e| TerminalError::Process(format!("PTY kill failed: {e}")))?;
        let status = s
            .child
            .wait()
            .map_err(|e| TerminalError::Process(format!("PTY wait failed: {e}")))?;
        s.exit_code = Some(status.exit_code());
        write_pty_recovery(s, "stopped")?;
    }
    Ok(pty_state_for(session_id, s))
}
pub fn remove_pty_session(session_id: &str) -> Result<bool, TerminalError> {
    let mut map = pty_sessions()
        .lock()
        .map_err(|_| TerminalError::Process("PTY session lock poisoned".into()))?;
    if let Some(mut s) = map.remove(session_id) {
        if s.exit_code.is_none() {
            let _ = s.child.kill();
            if let Ok(status) = s.child.wait() {
                s.exit_code = Some(status.exit_code());
            }
        }
        let _ = write_pty_recovery(&s, "removed");
        Ok(true)
    } else {
        Ok(false)
    }
}
pub fn list_pty_sessions() -> Result<Vec<PtySessionState>, TerminalError> {
    let mut map = pty_sessions()
        .lock()
        .map_err(|_| TerminalError::Process("PTY session lock poisoned".into()))?;
    let ids = map.keys().cloned().collect::<Vec<_>>();
    let mut out = Vec::new();
    for id in ids {
        if let Some(s) = map.get_mut(&id) {
            refresh_pty(s)?;
            out.push(pty_state_for(&id, s));
        }
    }
    Ok(out)
}
fn refresh_pty(s: &mut PtyTerminalSession) -> Result<(), TerminalError> {
    if s.exit_code.is_none() {
        if let Some(status) = s
            .child
            .try_wait()
            .map_err(|e| TerminalError::Process(format!("PTY wait failed: {e}")))?
        {
            s.exit_code = Some(status.exit_code());
            write_pty_recovery(s, "exited")?;
        }
    }
    Ok(())
}
fn write_pty_recovery(s: &PtyTerminalSession, state: &str) -> Result<(), TerminalError> {
    let Some(path) = s.recovery_path.as_ref() else {
        return Ok(());
    };
    let info = PtyRecoveryInfo {
        session_id: s.session_id.clone(),
        program: s.program.clone(),
        cwd: s.cwd.clone(),
        pid: s.pid,
        rows: s.rows,
        cols: s.cols,
        started_at_unix_ms: s.started_at_unix_ms,
        state: state.into(),
        exit_code: s.exit_code,
        scrollback_file: s.scrollback_file.clone(),
        orphaned: false,
    };
    let tmp = path.with_extension("tmp");
    let mut bytes =
        serde_json::to_vec_pretty(&info).map_err(|e| TerminalError::Process(e.to_string()))?;
    bytes.push(b'\n');
    std::fs::write(&tmp, bytes).map_err(|e| TerminalError::Process(e.to_string()))?;
    std::fs::rename(tmp, path).map_err(|e| TerminalError::Process(e.to_string()))?;
    Ok(())
}
pub fn list_pty_recovery(journal_dir: &Path) -> Result<Vec<PtyRecoveryInfo>, TerminalError> {
    std::fs::create_dir_all(journal_dir).map_err(|e| TerminalError::Process(e.to_string()))?;
    let active = pty_sessions()
        .lock()
        .map_err(|_| TerminalError::Process("PTY session lock poisoned".into()))?
        .keys()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    let mut out = Vec::new();
    for entry in
        std::fs::read_dir(journal_dir).map_err(|e| TerminalError::Process(e.to_string()))?
    {
        let entry = entry.map_err(|e| TerminalError::Process(e.to_string()))?;
        let path = entry.path();
        if path.extension().and_then(|v| v.to_str()) != Some("json") {
            continue;
        }
        let Ok(mut info) = serde_json::from_slice::<PtyRecoveryInfo>(
            &std::fs::read(&path).map_err(|e| TerminalError::Process(e.to_string()))?,
        ) else {
            continue;
        };
        info.orphaned =
            !active.contains(&info.session_id) && info.state == "running_at_last_checkpoint";
        out.push(info);
    }
    out.sort_by_key(|b| std::cmp::Reverse(b.started_at_unix_ms));
    out.truncate(256);
    Ok(out)
}
pub fn remove_pty_recovery(journal_dir: &Path, session_id: &str) -> Result<bool, TerminalError> {
    validate_pty_journal_id(session_id)?;
    if pty_sessions()
        .lock()
        .map_err(|_| TerminalError::Process("PTY session lock poisoned".into()))?
        .contains_key(session_id)
    {
        return Err(TerminalError::Invalid(
            "cannot remove recovery metadata for an active PTY session".into(),
        ));
    }
    let path = journal_dir.join(format!("{session_id}.json"));
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| TerminalError::Process(e.to_string()))?;
        Ok(true)
    } else {
        Ok(false)
    }
}
fn pty_state_for(id: &str, s: &PtyTerminalSession) -> PtySessionState {
    PtySessionState {
        session_id: id.into(),
        program: s.program.clone(),
        cwd: s.cwd.clone(),
        running: s.exit_code.is_none(),
        exit_code: s.exit_code,
        pid: s.pid,
        rows: s.rows,
        cols: s.cols,
        started_at_unix_ms: s.started_at_unix_ms,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PtyScrollbackInfo {
    pub session_id: String,
    pub bytes: u64,
    pub active: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PtyScrollbackChunk {
    pub session_id: String,
    pub offset: u64,
    pub next_offset: u64,
    pub total_bytes: u64,
    pub eof: bool,
    pub payload_base64: String,
}
fn validate_pty_journal_id(id: &str) -> Result<(), TerminalError> {
    if id.len() < 8
        || id.len() > 160
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        Err(TerminalError::Invalid(
            "invalid PTY scrollback session id".into(),
        ))
    } else {
        Ok(())
    }
}
pub fn read_pty_scrollback(
    journal_dir: &Path,
    session_id: &str,
    offset: u64,
    max_bytes: usize,
) -> Result<PtyScrollbackChunk, TerminalError> {
    validate_pty_journal_id(session_id)?;
    let path = journal_dir.join(format!("{session_id}.log"));
    let meta = std::fs::metadata(&path)
        .map_err(|_| TerminalError::Invalid("PTY scrollback not found".into()))?;
    if offset > meta.len() {
        return Err(TerminalError::Invalid(
            "PTY scrollback offset exceeds size".into(),
        ));
    }
    let cap = max_bytes.clamp(1, 256 * 1024);
    let mut file = std::fs::File::open(path).map_err(|e| TerminalError::Process(e.to_string()))?;
    use std::io::{Seek, SeekFrom};
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| TerminalError::Process(e.to_string()))?;
    let take = (meta.len() - offset).min(cap as u64) as usize;
    let mut bytes = vec![0u8; take];
    if take > 0 {
        file.read_exact(&mut bytes)
            .map_err(|e| TerminalError::Process(e.to_string()))?;
    }
    let next = offset + take as u64;
    Ok(PtyScrollbackChunk {
        session_id: session_id.into(),
        offset,
        next_offset: next,
        total_bytes: meta.len(),
        eof: next >= meta.len(),
        payload_base64: B64.encode(bytes),
    })
}
pub fn list_pty_scrollback(journal_dir: &Path) -> Result<Vec<PtyScrollbackInfo>, TerminalError> {
    std::fs::create_dir_all(journal_dir).map_err(|e| TerminalError::Process(e.to_string()))?;
    let active = pty_sessions()
        .lock()
        .map_err(|_| TerminalError::Process("PTY session lock poisoned".into()))?
        .keys()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    let mut out = Vec::new();
    for entry in
        std::fs::read_dir(journal_dir).map_err(|e| TerminalError::Process(e.to_string()))?
    {
        let entry = entry.map_err(|e| TerminalError::Process(e.to_string()))?;
        let path = entry.path();
        if path.extension().and_then(|v| v.to_str()) != Some("log") {
            continue;
        }
        let Some(id) = path.file_stem().and_then(|v| v.to_str()) else {
            continue;
        };
        if validate_pty_journal_id(id).is_err() {
            continue;
        }
        let bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
        out.push(PtyScrollbackInfo {
            session_id: id.into(),
            bytes,
            active: active.contains(id),
        });
    }
    out.sort_by(|a, b| b.session_id.cmp(&a.session_id));
    out.truncate(256);
    Ok(out)
}
pub fn remove_pty_scrollback(journal_dir: &Path, session_id: &str) -> Result<bool, TerminalError> {
    validate_pty_journal_id(session_id)?;
    if pty_sessions()
        .lock()
        .map_err(|_| TerminalError::Process("PTY session lock poisoned".into()))?
        .contains_key(session_id)
    {
        return Err(TerminalError::Invalid(
            "cannot remove scrollback for an active PTY session".into(),
        ));
    }
    let path = journal_dir.join(format!("{session_id}.log"));
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| TerminalError::Process(e.to_string()))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Bounded long-poll read for remote/browser terminal consumers. The function never
/// holds the global PTY session lock while sleeping, so stdin/resize/stop requests can
/// continue concurrently. A zero `wait_ms` behaves like an immediate read.
pub fn read_pty_session_wait(
    session_id: &str,
    max_bytes: usize,
    wait_ms: u64,
) -> Result<PtySessionChunk, TerminalError> {
    let deadline = Instant::now() + Duration::from_millis(wait_ms.min(5_000));
    loop {
        let chunk = read_pty_session(session_id, max_bytes)?;
        if !chunk.output.is_empty()
            || chunk.dropped_bytes > 0
            || !chunk.running
            || Instant::now() >= deadline
        {
            return Ok(chunk);
        }
        thread::sleep(Duration::from_millis(25));
    }
}

pub fn read_session_wait(
    session_id: &str,
    max_bytes: usize,
    wait_ms: u64,
) -> Result<SessionChunk, TerminalError> {
    let deadline = Instant::now() + Duration::from_millis(wait_ms.min(5_000));
    loop {
        let chunk = read_session(session_id, max_bytes)?;
        if !chunk.stdout.is_empty()
            || !chunk.stderr.is_empty()
            || chunk.stdout_dropped_bytes > 0
            || chunk.stderr_dropped_bytes > 0
            || !chunk.running
            || Instant::now() >= deadline
        {
            return Ok(chunk);
        }
        thread::sleep(Duration::from_millis(25));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalConformanceReport {
    pub ok: bool,
    pub pipe_sessions: bool,
    pub pty_conpty: bool,
    pub resize: bool,
    pub bounded_wait_read: bool,
    pub durable_scrollback: bool,
    pub recovery_metadata: bool,
    pub orphan_detection: bool,
    pub auto_recreate_after_agent_restart: bool,
    pub safety_invariant: String,
}
pub fn terminal_conformance() -> TerminalConformanceReport {
    TerminalConformanceReport{ok:true,pipe_sessions:true,pty_conpty:true,resize:true,bounded_wait_read:true,durable_scrollback:true,recovery_metadata:true,orphan_detection:true,auto_recreate_after_agent_restart:false,safety_invariant:"PTY processes are never automatically re-created after Agent loss because command side effects cannot be proven idempotent; durable scrollback and orphan metadata are retained instead".into()}
}
