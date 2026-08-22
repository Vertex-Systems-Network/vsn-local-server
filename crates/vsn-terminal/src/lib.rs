mod base {
    include!("lib_base.rs");
}

pub use base::*;

use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};

const DIRECT_MAX_OUTPUT_BYTES: usize = 512 * 1024;
const DIRECT_MAX_TIMEOUT_MS: u64 = 60_000;
const DIRECT_OUTPUT_DRAIN_GRACE: Duration = Duration::from_secs(2);

fn direct_timeout_ms(requested: u64) -> u64 {
    requested.clamp(100, DIRECT_MAX_TIMEOUT_MS)
}

fn validate_direct_request(
    program: &str,
    args: &[String],
    cwd: &Path,
    env: &BTreeMap<String, String>,
    roots: &[PathBuf],
) -> Result<(), TerminalError> {
    if program.is_empty() || program.len() > 512 {
        return Err(TerminalError::Invalid("invalid program".into()));
    }
    if args.len() > 256 || args.iter().any(|value| value.len() > 16_384) {
        return Err(TerminalError::Invalid("argument limit exceeded".into()));
    }
    if env.len() > 128 {
        return Err(TerminalError::Invalid("environment limit exceeded".into()));
    }
    for (key, value) in env {
        if key.is_empty()
            || key.len() > 128
            || !key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            || value.len() > 32_768
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

fn resolve_direct_program(
    roots: &[PathBuf],
    cwd: &Path,
    value: &str,
) -> Result<PathBuf, TerminalError> {
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
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(TerminalError::Invalid("unsafe executable name".into()));
    }
    Ok(vsn_system::find_executable(value)?)
}

fn read_direct_output<R: Read>(mut reader: R) -> std::io::Result<(Vec<u8>, bool)> {
    let mut captured = Vec::with_capacity(DIRECT_MAX_OUTPUT_BYTES.min(64 * 1024));
    let mut truncated = false;
    let mut buffer = [0u8; 16 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let remaining = DIRECT_MAX_OUTPUT_BYTES.saturating_sub(captured.len());
        let keep = remaining.min(read);
        if keep > 0 {
            captured.extend_from_slice(&buffer[..keep]);
        }
        if keep < read {
            truncated = true;
        }
    }
    Ok((captured, truncated))
}

fn spawn_direct_reader<R: Read + Send + 'static>(
    reader: R,
) -> Receiver<std::io::Result<(Vec<u8>, bool)>> {
    let (sender, receiver) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let _ = sender.send(read_direct_output(reader));
    });
    receiver
}

fn collect_direct_reader(
    receiver: &Receiver<std::io::Result<(Vec<u8>, bool)>>,
    deadline: Instant,
    label: &str,
) -> Result<(Vec<u8>, bool), TerminalError> {
    let wait = deadline.saturating_duration_since(Instant::now());
    match receiver.recv_timeout(wait) {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(error)) => Err(TerminalError::Process(format!(
            "{label} reader failed: {error}"
        ))),
        Err(RecvTimeoutError::Timeout) => Err(TerminalError::Process(format!(
            "{label} drain exceeded bounded shutdown window"
        ))),
        Err(RecvTimeoutError::Disconnected) => Err(TerminalError::Process(format!(
            "{label} reader terminated unexpectedly"
        ))),
    }
}

#[cfg(windows)]
mod direct_process_tree {
    use super::TerminalError;
    use std::{
        ffi::c_void,
        mem::{size_of, zeroed},
        os::windows::io::AsRawHandle,
        process::{Child, Command},
        ptr,
    };

    type Handle = *mut c_void;
    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS: i32 = 9;
    const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x0000_2000;

    #[repr(C)]
    struct JobObjectBasicLimitInformation {
        per_process_user_time_limit: i64,
        per_job_user_time_limit: i64,
        limit_flags: u32,
        minimum_working_set_size: usize,
        maximum_working_set_size: usize,
        active_process_limit: u32,
        affinity: usize,
        priority_class: u32,
        scheduling_class: u32,
    }

    #[repr(C)]
    struct IoCounters {
        read_operation_count: u64,
        write_operation_count: u64,
        other_operation_count: u64,
        read_transfer_count: u64,
        write_transfer_count: u64,
        other_transfer_count: u64,
    }

    #[repr(C)]
    struct JobObjectExtendedLimitInformation {
        basic_limit_information: JobObjectBasicLimitInformation,
        io_info: IoCounters,
        process_memory_limit: usize,
        job_memory_limit: usize,
        peak_process_memory_used: usize,
        peak_job_memory_used: usize,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        #[link_name = "CreateJobObjectW"]
        fn create_job_object_w(attributes: *const c_void, name: *const u16) -> Handle;
        #[link_name = "SetInformationJobObject"]
        fn set_information_job_object(
            job: Handle,
            information_class: i32,
            information: *const c_void,
            information_length: u32,
        ) -> i32;
        #[link_name = "AssignProcessToJobObject"]
        fn assign_process_to_job_object(job: Handle, process: Handle) -> i32;
        #[link_name = "TerminateJobObject"]
        fn terminate_job_object(job: Handle, exit_code: u32) -> i32;
        #[link_name = "CloseHandle"]
        fn close_handle(handle: Handle) -> i32;
    }

    pub fn configure(_command: &mut Command) {}

    pub struct Guard {
        job: Handle,
    }

    impl Guard {
        pub fn attach(child: &Child) -> Result<Self, TerminalError> {
            let job = unsafe { create_job_object_w(ptr::null(), ptr::null()) };
            if job.is_null() {
                return Err(TerminalError::Process(format!(
                    "direct terminal job creation failed: {}",
                    std::io::Error::last_os_error()
                )));
            }

            let mut information: JobObjectExtendedLimitInformation = unsafe { zeroed() };
            information.basic_limit_information.limit_flags =
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let configured = unsafe {
                set_information_job_object(
                    job,
                    JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
                    ptr::addr_of!(information).cast(),
                    size_of::<JobObjectExtendedLimitInformation>() as u32,
                )
            };
            if configured == 0 {
                let error = std::io::Error::last_os_error();
                unsafe {
                    close_handle(job);
                }
                return Err(TerminalError::Process(format!(
                    "direct terminal job configuration failed: {error}"
                )));
            }

            let assigned = unsafe {
                assign_process_to_job_object(job, child.as_raw_handle().cast::<c_void>())
            };
            if assigned == 0 {
                let error = std::io::Error::last_os_error();
                unsafe {
                    close_handle(job);
                }
                return Err(TerminalError::Process(format!(
                    "direct terminal process-tree assignment failed: {error}"
                )));
            }
            Ok(Self { job })
        }

        pub fn terminate_remaining(&self) -> Result<(), TerminalError> {
            if unsafe { terminate_job_object(self.job, 1) } == 0 {
                return Err(TerminalError::Process(format!(
                    "direct terminal process-tree termination failed: {}",
                    std::io::Error::last_os_error()
                )));
            }
            Ok(())
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            unsafe {
                close_handle(self.job);
            }
        }
    }
}

#[cfg(unix)]
mod direct_process_tree {
    use super::TerminalError;
    use std::{
        os::unix::process::CommandExt,
        process::{Child, Command},
    };

    const SIGKILL: i32 = 9;
    const ESRCH: i32 = 3;

    unsafe extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }

    pub fn configure(command: &mut Command) {
        command.process_group(0);
    }

    pub struct Guard {
        process_group: i32,
    }

    impl Guard {
        pub fn attach(child: &Child) -> Result<Self, TerminalError> {
            let process_group = i32::try_from(child.id()).map_err(|_| {
                TerminalError::Process("direct terminal process id exceeds platform range".into())
            })?;
            Ok(Self { process_group })
        }

        pub fn terminate_remaining(&self) -> Result<(), TerminalError> {
            let result = unsafe { kill(-self.process_group, SIGKILL) };
            if result == 0 {
                return Ok(());
            }
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(ESRCH) {
                Ok(())
            } else {
                Err(TerminalError::Process(format!(
                    "direct terminal process-group termination failed: {error}"
                )))
            }
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = self.terminate_remaining();
        }
    }
}

#[cfg(not(any(windows, unix)))]
mod direct_process_tree {
    use super::TerminalError;
    use std::process::{Child, Command};

    pub fn configure(_command: &mut Command) {}

    pub struct Guard;

    impl Guard {
        pub fn attach(_child: &Child) -> Result<Self, TerminalError> {
            Ok(Self)
        }

        pub fn terminate_remaining(&self) -> Result<(), TerminalError> {
            Ok(())
        }
    }
}

pub fn execute(roots: &[PathBuf], request: &ExecRequest) -> Result<ExecResult, TerminalError> {
    validate_direct_request(
        &request.program,
        &request.args,
        &request.cwd,
        &request.env,
        roots,
    )?;
    let cwd = vsn_files::resolve_existing(roots, &request.cwd)?;
    let program = resolve_direct_program(roots, &cwd, &request.program)?;
    let timeout_ms = direct_timeout_ms(request.timeout_ms);

    let mut command = Command::new(&program);
    command
        .args(&request.args)
        .current_dir(&cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in &request.env {
        command.env(key, value);
    }
    direct_process_tree::configure(&mut command);

    let mut child = command
        .spawn()
        .map_err(|error| TerminalError::Process(error.to_string()))?;
    let tree = match direct_process_tree::Guard::attach(&child) {
        Ok(tree) => tree,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| TerminalError::Process("stdout unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| TerminalError::Process("stderr unavailable".into()))?;
    let stdout_receiver = spawn_direct_reader(stdout);
    let stderr_receiver = spawn_direct_reader(stderr);

    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| TerminalError::Process(error.to_string()))?
        {
            break status;
        }
        if started.elapsed() >= Duration::from_millis(timeout_ms) {
            timed_out = true;
            tree.terminate_remaining()?;
            break child
                .wait()
                .map_err(|error| TerminalError::Process(error.to_string()))?;
        }
        thread::sleep(Duration::from_millis(25));
    };

    if !timed_out {
        tree.terminate_remaining()?;
    }
    let drain_deadline = Instant::now() + DIRECT_OUTPUT_DRAIN_GRACE;
    let (stdout, stdout_truncated) =
        collect_direct_reader(&stdout_receiver, drain_deadline, "stdout")?;
    let (stderr, stderr_truncated) =
        collect_direct_reader(&stderr_receiver, drain_deadline, "stderr")?;

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

#[cfg(test)]
mod direct_exec_facade_tests {
    use super::*;

    #[test]
    fn direct_timeout_is_strictly_bounded() {
        assert_eq!(direct_timeout_ms(0), 100);
        assert_eq!(direct_timeout_ms(30_000), 30_000);
        assert_eq!(direct_timeout_ms(u64::MAX), DIRECT_MAX_TIMEOUT_MS);
    }

    #[test]
    fn bounded_reader_keeps_draining_after_capture_limit() {
        let bytes = vec![b'x'; DIRECT_MAX_OUTPUT_BYTES + 32 * 1024];
        let (captured, truncated) = read_direct_output(std::io::Cursor::new(bytes)).unwrap();
        assert_eq!(captured.len(), DIRECT_MAX_OUTPUT_BYTES);
        assert!(truncated);
    }
}
