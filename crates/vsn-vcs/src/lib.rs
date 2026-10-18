//! Git for VSN Devify's Source control screen (DVF-08).
//!
//! Only a fixed set of git operations is exposed. User values (paths, branch names,
//! messages, remote URLs) are always separate argv entries, paths come after `--`, and
//! nothing can start with `-`, so no value can turn into a git option. Prompts are
//! disabled so a missing credential fails instead of hanging the agent.

use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VcsError {
    #[error("git is not installed or not on PATH")]
    GitMissing,
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("git failed: {0}")]
    Failed(String),
    #[error("git timed out after {0} s")]
    Timeout(u64),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

const OUTPUT_LIMIT: usize = 512 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileChange {
    /// Two-letter porcelain code, e.g. " M", "M ", "??", "A ".
    pub code: String,
    pub path: String,
    pub staged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RepoStatus {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<FileChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Commit {
    pub hash: String,
    pub author: String,
    pub when: String,
    pub subject: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Branch {
    pub name: String,
    pub current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitOutput {
    pub ok: bool,
    pub output: String,
}

fn git_program() -> Result<PathBuf, VcsError> {
    let names: &[&str] = if cfg!(windows) {
        &["git.exe", "git.cmd"]
    } else {
        &["git"]
    };
    let path = std::env::var_os("PATH").ok_or(VcsError::GitMissing)?;
    for dir in std::env::split_paths(&path) {
        for name in names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(VcsError::GitMissing)
}

/// Run git in `repo` with a deadline; returns (success, combined output).
fn git(repo: &Path, args: &[&str], timeout: Duration) -> Result<(bool, String), VcsError> {
    let mut command = Command::new(git_program()?);
    command
        .args(args)
        .current_dir(repo)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("LC_ALL", "C");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn()?;
    let mut stdout = child.stdout.take().expect("piped");
    let mut stderr = child.stderr.take().expect("piped");
    let out = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = (&mut stdout)
            .take(OUTPUT_LIMIT as u64)
            .read_to_end(&mut buf);
        std::io::copy(&mut stdout, &mut std::io::sink()).ok();
        buf
    });
    let err = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = (&mut stderr).take(64 * 1024).read_to_end(&mut buf);
        std::io::copy(&mut stderr, &mut std::io::sink()).ok();
        buf
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(VcsError::Timeout(timeout.as_secs()));
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    let mut text = String::from_utf8_lossy(&out.join().unwrap_or_default()).into_owned();
    let err_text = String::from_utf8_lossy(&err.join().unwrap_or_default()).into_owned();
    if !err_text.trim().is_empty() {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&err_text);
    }
    Ok((status.success(), text))
}

const QUICK: Duration = Duration::from_secs(20);
const NETWORK: Duration = Duration::from_secs(180);

fn checked(repo: &Path, args: &[&str], timeout: Duration) -> Result<String, VcsError> {
    let (ok, text) = git(repo, args, timeout)?;
    if ok {
        Ok(text)
    } else {
        Err(VcsError::Failed(text.trim().chars().take(4000).collect()))
    }
}

fn plain(field: &str, value: &str, max: usize) -> Result<(), VcsError> {
    if value.is_empty() || value.len() > max || value.starts_with('-') || value.contains('\0') {
        return Err(VcsError::Invalid(format!(
            "{field} is empty, too long or starts with '-'"
        )));
    }
    Ok(())
}

/// Branch names: git's own rules (`git check-ref-format --branch`) plus no leading '-'.
fn branch_name(repo: &Path, name: &str) -> Result<(), VcsError> {
    plain("branch name", name, 200)?;
    if name.chars().any(|c| c.is_control() || c == ' ') {
        return Err(VcsError::Invalid(
            "branch names cannot contain spaces".into(),
        ));
    }
    checked(repo, &["check-ref-format", "--branch", name], QUICK)
        .map(|_| ())
        .map_err(|_| VcsError::Invalid(format!("'{name}' is not a valid branch name")))
}

/// Parse `git status --porcelain=v1 -b -z`.
pub fn parse_status(raw: &str) -> RepoStatus {
    let mut status = RepoStatus {
        is_repo: true,
        ..Default::default()
    };
    let mut entries = raw.split('\0').peekable();
    while let Some(entry) = entries.next() {
        if entry.is_empty() {
            continue;
        }
        if let Some(head) = entry.strip_prefix("## ") {
            let (names, counts) = match head.split_once(" [") {
                Some((n, c)) => (n, c.trim_end_matches(']')),
                None => (head, ""),
            };
            let (branch, upstream) = match names.split_once("...") {
                Some((b, u)) => (b, Some(u.to_string())),
                None => (names, None),
            };
            let branch = branch.trim_start_matches("No commits yet on ");
            status.branch = Some(branch.to_string());
            status.upstream = upstream;
            for part in counts.split(", ") {
                if let Some(n) = part.strip_prefix("ahead ") {
                    status.ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix("behind ") {
                    status.behind = n.parse().unwrap_or(0);
                }
            }
            continue;
        }
        if entry.len() < 4 {
            continue;
        }
        let code = entry[..2].to_string();
        let path = entry[3..].to_string();
        // Renames/copies carry the original path as the next NUL-separated field.
        if code.starts_with('R') || code.starts_with('C') {
            entries.next();
        }
        let first = code.as_bytes()[0];
        status.files.push(FileChange {
            staged: first != b' ' && first != b'?',
            code,
            path,
        });
    }
    status
}

pub fn status(repo: &Path) -> Result<RepoStatus, VcsError> {
    let (ok, text) = git(repo, &["status", "--porcelain=v1", "-b", "-z"], QUICK)?;
    if !ok {
        if text.contains("not a git repository") {
            return Ok(RepoStatus::default());
        }
        return Err(VcsError::Failed(text.trim().to_string()));
    }
    Ok(parse_status(&text))
}

pub fn log(repo: &Path, limit: u32) -> Result<Vec<Commit>, VcsError> {
    let n = format!("-n{}", limit.clamp(1, 200));
    let (ok, text) = git(
        repo,
        &["log", &n, "--pretty=format:%h%x1f%an%x1f%ar%x1f%s%x1e"],
        QUICK,
    )?;
    if !ok {
        // A new repository has no commits yet.
        return Ok(Vec::new());
    }
    Ok(text
        .split('\u{1e}')
        .filter_map(|line| {
            let mut f = line.trim_matches('\n').split('\u{1f}');
            Some(Commit {
                hash: f.next().filter(|h| !h.is_empty())?.to_string(),
                author: f.next()?.to_string(),
                when: f.next()?.to_string(),
                subject: f.next().unwrap_or_default().to_string(),
            })
        })
        .collect())
}

pub fn branches(repo: &Path) -> Result<Vec<Branch>, VcsError> {
    let text = checked(repo, &["branch", "--format=%(HEAD)%(refname:short)"], QUICK)?;
    Ok(text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| Branch {
            current: l.starts_with('*'),
            name: l[1..].trim().to_string(),
        })
        .collect())
}

/// Diff of one file (working tree, or staged when `staged`).
pub fn diff(repo: &Path, path: &str, staged: bool) -> Result<String, VcsError> {
    plain("path", path, 4096)?;
    let mut args = vec!["diff", "--no-color", "--no-ext-diff"];
    if staged {
        args.push("--cached");
    }
    args.extend(["--", path]);
    checked(repo, &args, QUICK)
}

pub fn stage(repo: &Path, paths: &[String]) -> Result<GitOutput, VcsError> {
    path_command(repo, &["add", "--"], paths)
}
pub fn unstage(repo: &Path, paths: &[String]) -> Result<GitOutput, VcsError> {
    path_command(repo, &["restore", "--staged", "--"], paths)
}
/// Throw away working-tree changes to tracked files (not untracked ones).
pub fn discard(repo: &Path, paths: &[String]) -> Result<GitOutput, VcsError> {
    path_command(repo, &["restore", "--"], paths)
}
fn path_command(repo: &Path, prefix: &[&str], paths: &[String]) -> Result<GitOutput, VcsError> {
    if paths.is_empty() || paths.len() > 1000 {
        return Err(VcsError::Invalid("choose between 1 and 1000 files".into()));
    }
    for p in paths {
        plain("path", p, 4096)?;
    }
    let mut args: Vec<&str> = prefix.to_vec();
    args.extend(paths.iter().map(String::as_str));
    Ok(GitOutput {
        ok: true,
        output: checked(repo, &args, QUICK)?,
    })
}

pub fn commit(repo: &Path, message: &str) -> Result<GitOutput, VcsError> {
    if message.trim().is_empty() || message.len() > 20_000 || message.contains('\0') {
        return Err(VcsError::Invalid("write a commit message".into()));
    }
    Ok(GitOutput {
        ok: true,
        output: checked(repo, &["commit", "-m", message], QUICK)?,
    })
}

pub fn switch(repo: &Path, name: &str, create: bool) -> Result<GitOutput, VcsError> {
    branch_name(repo, name)?;
    let args: Vec<&str> = if create {
        vec!["switch", "-c", name]
    } else {
        vec!["switch", name]
    };
    Ok(GitOutput {
        ok: true,
        output: checked(repo, &args, QUICK)?,
    })
}

pub fn fetch(repo: &Path) -> Result<GitOutput, VcsError> {
    Ok(GitOutput {
        ok: true,
        output: checked(repo, &["fetch", "--prune"], NETWORK)?,
    })
}
pub fn pull(repo: &Path) -> Result<GitOutput, VcsError> {
    Ok(GitOutput {
        ok: true,
        output: checked(repo, &["pull", "--ff-only"], NETWORK)?,
    })
}
/// Push the current branch, setting its upstream on first push.
pub fn push(repo: &Path) -> Result<GitOutput, VcsError> {
    let st = status(repo)?;
    let output = if st.upstream.is_some() {
        checked(repo, &["push"], NETWORK)?
    } else {
        let branch = st
            .branch
            .ok_or_else(|| VcsError::Invalid("no current branch".into()))?;
        branch_name(repo, &branch)?;
        checked(repo, &["push", "-u", "origin", &branch], NETWORK)?
    };
    Ok(GitOutput { ok: true, output })
}

pub fn init(repo: &Path) -> Result<GitOutput, VcsError> {
    Ok(GitOutput {
        ok: true,
        output: checked(repo, &["init"], QUICK)?,
    })
}

/// Clone `url` into `dest` (a new folder name inside `parent`).
pub fn clone(parent: &Path, url: &str, dest: &str) -> Result<GitOutput, VcsError> {
    plain("repository URL", url, 2048)?;
    let allowed =
        url.starts_with("https://") || url.starts_with("ssh://") || url.starts_with("git@");
    if !allowed || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(VcsError::Invalid(
            "use an https://, ssh:// or git@ repository URL".into(),
        ));
    }
    plain("folder name", dest, 200)?;
    if dest.contains(['/', '\\']) || dest == "." || dest == ".." {
        return Err(VcsError::Invalid(
            "folder name must be a single folder".into(),
        ));
    }
    Ok(GitOutput {
        ok: true,
        output: checked(parent, &["clone", "--", url, dest], NETWORK)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_branch_and_files() {
        let raw = "## main...origin/main [ahead 2, behind 1]\0 M src/a.rs\0A  new.txt\0?? notes.md\0R  b.rs\0old.rs\0";
        let s = parse_status(raw);
        assert_eq!(s.branch.as_deref(), Some("main"));
        assert_eq!(s.upstream.as_deref(), Some("origin/main"));
        assert_eq!((s.ahead, s.behind), (2, 1));
        let codes: Vec<_> = s
            .files
            .iter()
            .map(|f| (f.code.as_str(), f.path.as_str(), f.staged))
            .collect();
        assert_eq!(
            codes,
            vec![
                (" M", "src/a.rs", false),
                ("A ", "new.txt", true),
                ("??", "notes.md", false),
                ("R ", "b.rs", true)
            ]
        );
    }

    #[test]
    fn new_repo_branch_header() {
        let s = parse_status("## No commits yet on main\0");
        assert_eq!(s.branch.as_deref(), Some("main"));
        assert!(s.files.is_empty());
    }

    #[test]
    fn rejects_option_like_values() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            diff(dir.path(), "--output=/tmp/x", false),
            Err(VcsError::Invalid(_))
        ));
        assert!(matches!(
            stage(dir.path(), &["-A".into()]),
            Err(VcsError::Invalid(_))
        ));
        assert!(matches!(
            clone(dir.path(), "--upload-pack=touch /tmp/x", "x"),
            Err(VcsError::Invalid(_))
        ));
        assert!(matches!(
            clone(dir.path(), "file:///etc", "x"),
            Err(VcsError::Invalid(_))
        ));
        assert!(matches!(
            clone(dir.path(), "https://example.com/r.git", "../x"),
            Err(VcsError::Invalid(_))
        ));
    }

    #[test]
    fn full_local_flow() {
        if git_program().is_err() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        init(repo).unwrap();
        git(repo, &["config", "user.email", "t@example.com"], QUICK).unwrap();
        git(repo, &["config", "user.name", "Tester"], QUICK).unwrap();
        std::fs::write(repo.join("a.txt"), "one\n").unwrap();
        let st = status(repo).unwrap();
        assert!(st.is_repo);
        assert_eq!(st.files[0].code, "??");
        stage(repo, &["a.txt".into()]).unwrap();
        assert!(status(repo).unwrap().files[0].staged);
        commit(repo, "first").unwrap();
        assert_eq!(log(repo, 10).unwrap()[0].subject, "first");
        std::fs::write(repo.join("a.txt"), "two\n").unwrap();
        assert!(diff(repo, "a.txt", false).unwrap().contains("+two"));
        discard(repo, &["a.txt".into()]).unwrap();
        assert!(status(repo).unwrap().files.is_empty());
        switch(repo, "feature/x", true).unwrap();
        assert!(branches(repo)
            .unwrap()
            .iter()
            .any(|b| b.current && b.name == "feature/x"));
        assert!(switch(repo, "bad name", true).is_err());
        let outside = tempfile::tempdir().unwrap();
        assert!(!status(outside.path()).unwrap().is_repo);
    }
}
