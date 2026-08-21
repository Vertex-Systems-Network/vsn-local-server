use serde::{Deserialize, Serialize};
use std::{
    io::{self, Read},
    path::{Path, PathBuf},
    process,
};

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
enum HelperRequest {
    Apply {
        request: vsn_update::ApplyFileRequest,
    },
    Rollback {
        install_root: PathBuf,
        confirm_rollback: bool,
    },
    Status {
        install_root: PathBuf,
    },
    RecoverLock {
        install_root: PathBuf,
        confirm_recover: bool,
    },
}
#[derive(Debug, Serialize)]
struct HelperResponse<T: Serialize> {
    ok: bool,
    result: Option<T>,
    error: Option<String>,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        process::exit(1)
    }
}
fn run() -> Result<(), String> {
    let mut input = String::new();
    io::stdin()
        .take(2 * 1024 * 1024)
        .read_to_string(&mut input)
        .map_err(|e| format!("stdin read failed: {e}"))?;
    if input.trim().is_empty() {
        return Err("updater helper expects a bounded JSON request on stdin".into());
    }
    let req: HelperRequest =
        serde_json::from_str(&input).map_err(|e| format!("invalid helper JSON: {e}"))?;
    match req {
        HelperRequest::Apply { request } => {
            let _root = canonical_root(&request.install_root)?;
            let result =
                vsn_update::apply_verified_file_locked(&request).map_err(|e| e.to_string())?;
            print_json(&HelperResponse {
                ok: true,
                result: Some(result),
                error: None,
            })?;
        }
        HelperRequest::Rollback {
            install_root,
            confirm_rollback,
        } => {
            let root = canonical_root(&install_root)?;
            let result = vsn_update::rollback_verified_file_locked(&root, confirm_rollback)
                .map_err(|e| e.to_string())?;
            print_json(&HelperResponse {
                ok: true,
                result: Some(result),
                error: None,
            })?;
        }
        HelperRequest::Status { install_root } => {
            let root = canonical_root(&install_root)?;
            let status = vsn_update::update_status(&root).map_err(|e| e.to_string())?;
            print_json(&HelperResponse {
                ok: true,
                result: Some(status),
                error: None,
            })?;
        }
        HelperRequest::RecoverLock {
            install_root,
            confirm_recover,
        } => {
            let root = canonical_root(&install_root)?;
            let result = vsn_update::recover_stale_update_lock(&root, confirm_recover)
                .map_err(|e| e.to_string())?;
            print_json(&HelperResponse {
                ok: true,
                result: Some(result),
                error: None,
            })?;
        }
    }
    Ok(())
}
fn canonical_root(path: &Path) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|e| format!("install root unavailable: {e}"))
}
fn print_json<T: Serialize>(value: &T) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string(value).map_err(|e| e.to_string())?
    );
    Ok(())
}
