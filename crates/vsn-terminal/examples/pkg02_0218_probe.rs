use serde_json::json;
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};
use vsn_terminal::{execute, ExecRequest};

fn helper_parent(sentinel: &Path) -> Result<(), Box<dyn std::error::Error>> {
    thread::sleep(Duration::from_millis(200));
    let child = std::env::current_exe()?;
    let _spawned = Command::new(child)
        .arg("helper-child")
        .arg(sentinel)
        .spawn()?;
    println!("parent-spawned");
    thread::sleep(Duration::from_secs(5));
    Ok(())
}

fn helper_child(sentinel: &Path) -> Result<(), Box<dyn std::error::Error>> {
    thread::sleep(Duration::from_millis(1_200));
    fs::write(sentinel, b"descendant-survived")?;
    Ok(())
}

fn helper_large_output() -> Result<(), Box<dyn std::error::Error>> {
    let payload = vec![b'x'; 768 * 1024];
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&payload)?;
    stdout.flush()?;
    eprintln!("stderr-marker-0218");
    Ok(())
}

fn copied_probe_path(workspace: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let current = std::env::current_exe()?;
    let extension = current.extension().and_then(|value| value.to_str());
    let name = match extension {
        Some(extension) => format!("pkg02-0218-probe.{extension}"),
        None => "pkg02-0218-probe".into(),
    };
    let copied = workspace.join(name);
    fs::copy(current, &copied)?;
    Ok(copied)
}

fn request(program: &Path, cwd: &Path, args: &[&str], timeout_ms: u64) -> ExecRequest {
    ExecRequest {
        program: program.display().to_string(),
        args: args.iter().map(|value| (*value).to_string()).collect(),
        cwd: cwd.to_path_buf(),
        env: BTreeMap::new(),
        timeout_ms,
    }
}

fn run_probe(workspace: &Path, outside: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(workspace)?;
    fs::create_dir_all(outside)?;
    let workspace = workspace.canonicalize()?;
    let outside = outside.canonicalize()?;
    let roots = vec![workspace.clone()];
    let program = copied_probe_path(&workspace)?;

    let large = execute(
        &roots,
        &request(&program, &workspace, &["helper-large-output"], 5_000),
    )?;
    if large.timed_out
        || !large.stdout_truncated
        || large.stdout.len() != 512 * 1024
        || !large.stderr.contains("stderr-marker-0218")
    {
        return Err("large-output bounded drain invariant failed".into());
    }

    let sentinel = outside.join("descendant-sentinel.txt");
    let _ = fs::remove_file(&sentinel);
    let timeout = execute(
        &roots,
        &request(
            &program,
            &workspace,
            &["helper-parent", &sentinel.display().to_string()],
            250,
        ),
    )?;
    if !timeout.timed_out {
        return Err("direct execution timeout did not trigger".into());
    }
    thread::sleep(Duration::from_millis(1_600));
    if sentinel.exists() {
        return Err("timed-out descendant escaped process-tree termination".into());
    }
    if timeout.duration_ms > 4_000 {
        return Err("timeout plus output drain exceeded bounded shutdown budget".into());
    }

    let outside_cwd = execute(
        &roots,
        &request(&program, &outside, &["helper-large-output"], 1_000),
    );
    if outside_cwd.is_ok() {
        return Err("outside-workspace cwd was accepted".into());
    }

    let original_program = std::env::current_exe()?;
    let outside_program = execute(
        &roots,
        &request(&original_program, &workspace, &["helper-large-output"], 1_000),
    );
    if outside_program.is_ok() {
        return Err("absolute program outside workspace was accepted".into());
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version": 1,
            "task_id": "02.18",
            "large_output_capture_bytes": large.stdout.len(),
            "large_output_truncated": large.stdout_truncated,
            "large_output_completed_without_timeout": !large.timed_out,
            "timeout_triggered": timeout.timed_out,
            "timeout_duration_ms": timeout.duration_ms,
            "descendant_sentinel_absent": !sentinel.exists(),
            "outside_cwd_rejected": outside_cwd.is_err(),
            "outside_absolute_program_rejected": outside_program.is_err()
        }))?
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [mode, sentinel] if mode == "helper-parent" => helper_parent(Path::new(sentinel)),
        [mode, sentinel] if mode == "helper-child" => helper_child(Path::new(sentinel)),
        [mode] if mode == "helper-large-output" => helper_large_output(),
        [workspace, outside] => run_probe(Path::new(workspace), Path::new(outside)),
        _ => Err("usage: pkg02_0218_probe <workspace> <outside>".into()),
    }
}
