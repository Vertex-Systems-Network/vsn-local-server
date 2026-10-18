//! Subversion basics for the Source control screen: status, info, update, add, revert
//! and commit through the `svn` CLI. Same rules as git: user values are separate argv
//! entries after `--`, nothing may start with `-`, and `--non-interactive` makes a
//! missing credential fail instead of waiting for a prompt.

use super::{find_program, plain, run, GitOutput, VcsError, NETWORK, QUICK};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SvnChange {
    /// `svn status` item letter: M, A, D, ?, !, C, ~, R or I.
    pub code: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SvnStatus {
    pub is_working_copy: bool,
    pub url: Option<String>,
    pub revision: Option<String>,
    pub files: Vec<SvnChange>,
}

fn svn_program() -> Result<PathBuf, VcsError> {
    let names: &[&str] = if cfg!(windows) {
        &["svn.exe"]
    } else {
        &["svn"]
    };
    find_program(names, || VcsError::SvnMissing)
}

fn svn(repo: &Path, args: &[&str], timeout: Duration) -> Result<String, VcsError> {
    let mut full = vec!["--non-interactive"];
    full.extend_from_slice(args);
    let (ok, text) = run(svn_program()?, repo, &full, timeout)?;
    if ok {
        Ok(text)
    } else {
        Err(VcsError::Failed(text.trim().chars().take(4000).collect()))
    }
}

/// Parse plain `svn status` output (the item letter is column 1, the path starts at 9).
pub fn parse_status(raw: &str) -> Vec<SvnChange> {
    raw.lines()
        .filter(|l| l.len() > 8 && !l.starts_with(' '))
        .filter_map(|l| {
            let code = l.get(..1)?.to_string();
            let path = l.get(8..)?.trim().to_string();
            (!path.is_empty() && "MADR?!C~I".contains(code.as_str()))
                .then_some(SvnChange { code, path })
        })
        .collect()
}

fn info_field(info: &str, key: &str) -> Option<String> {
    info.lines()
        .find_map(|l| l.strip_prefix(key))
        .map(|v| v.trim().to_string())
}

pub fn status(repo: &Path) -> Result<SvnStatus, VcsError> {
    let info = match svn(repo, &["info"], QUICK) {
        Ok(text) => text,
        Err(VcsError::Failed(_)) => return Ok(SvnStatus::default()),
        Err(e) => return Err(e),
    };
    Ok(SvnStatus {
        is_working_copy: true,
        url: info_field(&info, "URL:"),
        revision: info_field(&info, "Revision:"),
        files: parse_status(&svn(repo, &["status"], QUICK)?),
    })
}

fn with_paths(
    repo: &Path,
    prefix: &[&str],
    paths: &[String],
    timeout: Duration,
) -> Result<GitOutput, VcsError> {
    if paths.len() > 1000 {
        return Err(VcsError::Invalid("choose at most 1000 files".into()));
    }
    for p in paths {
        plain("path", p, 4096)?;
    }
    let mut args: Vec<&str> = prefix.to_vec();
    args.push("--");
    args.extend(paths.iter().map(String::as_str));
    Ok(GitOutput {
        ok: true,
        output: svn(repo, &args, timeout)?,
    })
}

pub fn update(repo: &Path) -> Result<GitOutput, VcsError> {
    Ok(GitOutput {
        ok: true,
        output: svn(repo, &["update"], NETWORK)?,
    })
}

pub fn add(repo: &Path, paths: &[String]) -> Result<GitOutput, VcsError> {
    if paths.is_empty() {
        return Err(VcsError::Invalid("choose files to add".into()));
    }
    with_paths(repo, &["add", "--parents"], paths, QUICK)
}

/// Throw away local changes to the chosen files.
pub fn revert(repo: &Path, paths: &[String]) -> Result<GitOutput, VcsError> {
    if paths.is_empty() {
        return Err(VcsError::Invalid("choose files to revert".into()));
    }
    with_paths(repo, &["revert"], paths, QUICK)
}

/// Commit the chosen files, or every change when `paths` is empty.
pub fn commit(repo: &Path, message: &str, paths: &[String]) -> Result<GitOutput, VcsError> {
    if message.trim().is_empty() || message.len() > 20_000 || message.contains('\0') {
        return Err(VcsError::Invalid("write a commit message".into()));
    }
    with_paths(repo, &["commit", "-m", message], paths, NETWORK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status_lines() {
        let raw =
            "M       src/a.php\n?       new file.txt\nA  +    copied.rb\n        > moved away\n";
        let files = parse_status(raw);
        assert_eq!(
            files,
            vec![
                SvnChange {
                    code: "M".into(),
                    path: "src/a.php".into()
                },
                SvnChange {
                    code: "?".into(),
                    path: "new file.txt".into()
                },
                SvnChange {
                    code: "A".into(),
                    path: "copied.rb".into()
                },
            ]
        );
    }

    #[test]
    fn rejects_option_like_paths() {
        let dir = std::env::temp_dir();
        assert!(matches!(
            revert(&dir, &["-R".into()]),
            Err(VcsError::Invalid(_))
        ));
        assert!(matches!(commit(&dir, " ", &[]), Err(VcsError::Invalid(_))));
    }

    #[test]
    fn round_trip_when_svn_is_installed() {
        if svn_program().is_err() || which_svnadmin().is_none() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let server = dir.path().join("server");
        let ok = std::process::Command::new("svnadmin")
            .arg("create")
            .arg(&server)
            .status()
            .unwrap()
            .success();
        assert!(ok);
        let url = format!("file://{}", server.display());
        svn(dir.path(), &["checkout", &url, "wc"], QUICK).unwrap();
        let wc = dir.path().join("wc");
        std::fs::write(wc.join("index.php"), "<?php echo 1;").unwrap();
        let st = status(&wc).unwrap();
        assert!(st.is_working_copy);
        assert_eq!(st.files[0].code, "?");
        add(&wc, &["index.php".into()]).unwrap();
        commit(&wc, "first", &[]).unwrap();
        update(&wc).unwrap();
        let st = status(&wc).unwrap();
        assert_eq!(st.revision.as_deref(), Some("1"));
        assert!(st.files.is_empty());
        std::fs::write(wc.join("index.php"), "changed").unwrap();
        assert_eq!(status(&wc).unwrap().files[0].code, "M");
        revert(&wc, &["index.php".into()]).unwrap();
        assert!(status(&wc).unwrap().files.is_empty());
        assert!(!status(dir.path()).unwrap().is_working_copy);
    }

    fn which_svnadmin() -> Option<PathBuf> {
        find_program(&["svnadmin"], || VcsError::SvnMissing).ok()
    }
}
