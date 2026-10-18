//! FTP / FTPS / SFTP for VSN Devify (DVF-07): list, upload and download against a
//! space's deploy server.
//!
//! FTP and FTPS run through `curl` (shipped with Windows 10+), SFTP through the OpenSSH
//! `sftp` client (also shipped with Windows) in batch mode with key authentication.
//! Passwords never appear on a command line: curl reads them from a private config file
//! that is deleted right after the call. Remote paths are validated and quoted.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransferError {
    #[error("{0} is not installed or not on PATH")]
    ToolMissing(&'static str),
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("transfer failed: {0}")]
    Failed(String),
    #[error("transfer timed out after {0} s")]
    Timeout(u64),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Ftp,
    /// Explicit TLS (AUTH TLS) on the FTP port.
    Ftps,
    Sftp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Profile {
    pub protocol: Protocol,
    pub host: String,
    #[serde(default)]
    pub port: Option<u16>,
    pub user: String,
    /// Base folder on the server, e.g. `/public_html`.
    #[serde(default)]
    pub remote_dir: String,
    /// SFTP private key; empty uses the user's default SSH keys/agent.
    #[serde(default)]
    pub key_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: Option<u64>,
}

const TIMEOUT: Duration = Duration::from_secs(600);

fn find_tool(name: &'static str) -> Result<PathBuf, TransferError> {
    let candidates: Vec<String> = if cfg!(windows) {
        vec![format!("{name}.exe")]
    } else {
        vec![name.to_string()]
    };
    let path = std::env::var_os("PATH").ok_or(TransferError::ToolMissing(name))?;
    for dir in std::env::split_paths(&path) {
        for c in &candidates {
            let p = dir.join(c);
            if p.is_file() {
                return Ok(p);
            }
        }
    }
    Err(TransferError::ToolMissing(name))
}

fn run(mut command: Command, input: Option<&str>) -> Result<(bool, String), TransferError> {
    command
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn()?;
    if let (Some(text), Some(mut stdin)) = (input, child.stdin.take()) {
        stdin.write_all(text.as_bytes())?;
    }
    let mut out = child.stdout.take().expect("piped");
    let mut err = child.stderr.take().expect("piped");
    let out_t = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = (&mut out).take(1 << 20).read_to_end(&mut b);
        std::io::copy(&mut out, &mut std::io::sink()).ok();
        b
    });
    let err_t = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = (&mut err).take(64 << 10).read_to_end(&mut b);
        std::io::copy(&mut err, &mut std::io::sink()).ok();
        b
    });
    let started = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break s;
        }
        if started.elapsed() > TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err(TransferError::Timeout(TIMEOUT.as_secs()));
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    let mut text = String::from_utf8_lossy(&out_t.join().unwrap_or_default()).into_owned();
    let e = String::from_utf8_lossy(&err_t.join().unwrap_or_default()).into_owned();
    if !status.success() {
        text = e;
    }
    Ok((status.success(), text))
}

fn checked(command: Command, input: Option<&str>) -> Result<String, TransferError> {
    let (ok, text) = run(command, input)?;
    if ok {
        Ok(text)
    } else {
        Err(TransferError::Failed(
            text.trim().chars().take(4000).collect(),
        ))
    }
}

pub fn validate(p: &Profile) -> Result<(), TransferError> {
    let host_ok = !p.host.is_empty()
        && p.host.len() <= 253
        && !p.host.starts_with('-')
        && p.host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    if !host_ok {
        return Err(TransferError::Invalid("server host".into()));
    }
    if p.user.is_empty()
        || p.user.len() > 128
        || p.user.starts_with('-')
        || p.user
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '@' | ':' | '"'))
    {
        return Err(TransferError::Invalid("user name".into()));
    }
    if p.port == Some(0) {
        return Err(TransferError::Invalid("port".into()));
    }
    remote_path(&p.remote_dir)?;
    Ok(())
}

/// Remote paths: '/'-separated, no `..`, no quotes or control characters.
fn remote_path(path: &str) -> Result<String, TransferError> {
    if path.len() > 1024
        || path
            .chars()
            .any(|c| c.is_control() || c == '"' || c == '\\')
    {
        return Err(TransferError::Invalid("remote path".into()));
    }
    let parts: Vec<&str> = path
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();
    if parts.contains(&"..") {
        return Err(TransferError::Invalid(
            "remote path cannot contain '..'".into(),
        ));
    }
    Ok(parts.join("/"))
}

fn join_remote(base: &str, sub: &str) -> Result<String, TransferError> {
    let (b, s) = (remote_path(base)?, remote_path(sub)?);
    Ok(match (b.is_empty(), s.is_empty()) {
        (true, _) => s,
        (_, true) => b,
        _ => format!("{b}/{s}"),
    })
}

fn encode_segment(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn ftp_url(p: &Profile, path: &str, dir: bool) -> String {
    let port = p.port.unwrap_or(21);
    let encoded: Vec<String> = path
        .split('/')
        .filter(|s| !s.is_empty())
        .map(encode_segment)
        .collect();
    // A leading %2F makes the path absolute instead of relative to the login folder.
    let mut url = format!("ftp://{}:{port}/%2F{}", p.host, encoded.join("/"));
    if dir && !url.ends_with('/') {
        url.push('/');
    }
    url
}

/// curl call with the password in a private, short-lived config file.
fn curl(
    p: &Profile,
    password: &str,
    scratch: &Path,
    args: &[&str],
) -> Result<String, TransferError> {
    if password.contains(['\n', '\r', '\0']) {
        return Err(TransferError::Invalid("password".into()));
    }
    std::fs::create_dir_all(scratch)?;
    let cfg = scratch.join(format!("curl-{}-{}.cfg", std::process::id(), nanos()));
    let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    write_private(
        &cfg,
        format!("user = \"{}:{}\"\n", esc(&p.user), esc(password)).as_bytes(),
    )?;
    let mut cmd = Command::new(find_tool("curl")?);
    cmd.args([
        "--silent",
        "--show-error",
        "--fail",
        "--connect-timeout",
        "20",
        "--max-time",
        "590",
    ])
    .arg("--config")
    .arg(&cfg);
    if p.protocol == Protocol::Ftps {
        cmd.arg("--ssl-reqd");
    }
    cmd.args(args);
    let result = checked(cmd, None);
    let _ = std::fs::remove_file(&cfg);
    result
}

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default()
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<(), TransferError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(bytes)?;
    }
    #[cfg(not(unix))]
    std::fs::write(path, bytes)?;
    Ok(())
}

fn sftp(p: &Profile, batch: &str) -> Result<String, TransferError> {
    let mut cmd = Command::new(find_tool("sftp")?);
    cmd.args([
        "-b",
        "-",
        "-o",
        "BatchMode=yes",
        "-o",
        "StrictHostKeyChecking=accept-new",
        "-o",
        "ConnectTimeout=20",
    ]);
    cmd.arg("-P").arg(p.port.unwrap_or(22).to_string());
    if let Some(key) = p.key_path.as_ref().filter(|k| !k.as_os_str().is_empty()) {
        cmd.arg("-i").arg(key);
    }
    cmd.arg(format!("{}@{}", p.user, p.host));
    checked(cmd, Some(batch))
}

/// Parse a Unix `ls -l` style listing (FTP LIST and sftp `ls -l`).
pub fn parse_listing(text: &str) -> Vec<RemoteEntry> {
    text.lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 9 || !matches!(fields[0].chars().next(), Some('d' | '-' | 'l')) {
                return None;
            }
            // Name is everything after the 8th field (keeps spaces in names).
            let mut rest = line;
            for _ in 0..8 {
                rest = rest.trim_start();
                rest = &rest[rest.find(char::is_whitespace).unwrap_or(rest.len())..];
            }
            let name = rest
                .trim_start()
                .split(" -> ")
                .next()
                .unwrap_or("")
                .to_string();
            if name.is_empty() || name == "." || name == ".." {
                return None;
            }
            Some(RemoteEntry {
                is_dir: fields[0].starts_with('d'),
                size: fields[4].parse().ok(),
                name,
            })
        })
        .collect()
}

/// List a folder (relative to the profile's remote_dir).
pub fn list(
    p: &Profile,
    password: &str,
    scratch: &Path,
    sub: &str,
) -> Result<Vec<RemoteEntry>, TransferError> {
    validate(p)?;
    let path = join_remote(&p.remote_dir, sub)?;
    let text = match p.protocol {
        Protocol::Sftp => sftp(p, &format!("ls -l \"/{path}\"\n"))?,
        _ => curl(p, password, scratch, &[&ftp_url(p, &path, true)])?,
    };
    Ok(parse_listing(&text))
}

/// Upload one local file into a remote folder (created when missing on FTP).
pub fn upload(
    p: &Profile,
    password: &str,
    scratch: &Path,
    local: &Path,
    remote_sub: &str,
) -> Result<String, TransferError> {
    validate(p)?;
    if !local.is_file() {
        return Err(TransferError::Invalid("choose a local file".into()));
    }
    let name = local
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| TransferError::Invalid("local file name".into()))?;
    let dir = join_remote(&p.remote_dir, remote_sub)?;
    match p.protocol {
        Protocol::Sftp => {
            let l = local.to_string_lossy();
            if l.contains(['"', '\n', '\r']) {
                return Err(TransferError::Invalid("local path".into()));
            }
            sftp(
                p,
                &format!("put \"{l}\" \"/{dir}/{}\"\n", remote_path(name)?),
            )
        }
        _ => {
            let url = format!("{}{}", ftp_url(p, &dir, true), encode_segment(name));
            let local_s = local.to_string_lossy().into_owned();
            curl(
                p,
                password,
                scratch,
                &["--ftp-create-dirs", "--upload-file", &local_s, &url],
            )
        }
    }
}

/// Download one remote file into a local folder.
pub fn download(
    p: &Profile,
    password: &str,
    scratch: &Path,
    remote_file: &str,
    local_dir: &Path,
) -> Result<PathBuf, TransferError> {
    validate(p)?;
    let path = join_remote(&p.remote_dir, remote_file)?;
    let name = path
        .rsplit('/')
        .next()
        .filter(|n| !n.is_empty())
        .ok_or_else(|| TransferError::Invalid("remote file".into()))?;
    if !local_dir.is_dir() {
        return Err(TransferError::Invalid("local folder does not exist".into()));
    }
    let dest = local_dir.join(name);
    let dest_s = dest.to_string_lossy().into_owned();
    match p.protocol {
        Protocol::Sftp => {
            if dest_s.contains(['"', '\n', '\r']) {
                return Err(TransferError::Invalid("local path".into()));
            }
            sftp(p, &format!("get \"/{path}\" \"{dest_s}\"\n"))?;
        }
        _ => {
            curl(
                p,
                password,
                scratch,
                &["--output", &dest_s, &ftp_url(p, &path, false)],
            )?;
        }
    }
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Profile {
        Profile {
            protocol: Protocol::Ftp,
            host: "127.0.0.1".into(),
            port: Some(2121),
            user: "dev".into(),
            remote_dir: "/site".into(),
            key_path: None,
        }
    }

    #[test]
    fn rejects_unsafe_values() {
        let mut p = profile();
        p.host = "-oProxyCommand=x".into();
        assert!(validate(&p).is_err());
        let mut p = profile();
        p.user = "a@b".into();
        assert!(validate(&p).is_err());
        assert!(join_remote("/site", "../etc").is_err());
        assert!(join_remote("/site", "a\"b").is_err());
        assert_eq!(join_remote("/site/", "/img//x/").unwrap(), "site/img/x");
    }

    #[test]
    fn builds_ftp_urls() {
        assert_eq!(
            ftp_url(&profile(), "site/my file", true),
            "ftp://127.0.0.1:2121/%2Fsite/my%20file/"
        );
    }

    #[test]
    fn parses_ls_l() {
        let text = "drwxr-xr-x   2 dev dev     4096 Oct 18 09:00 img\n-rw-r--r--   1 dev dev      120 Oct 18 09:01 my page.html\nlrwxrwxrwx 1 a a 4 Oct 1 1:00 cur -> img\n";
        let e = parse_listing(text);
        assert_eq!(e.len(), 3);
        assert!(e[0].is_dir && e[0].name == "img");
        assert_eq!((e[1].name.as_str(), e[1].size), ("my page.html", Some(120)));
        assert_eq!(e[2].name, "cur");
    }
}
