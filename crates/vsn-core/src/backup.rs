//! DVF-15 backups: archive a whole space (files plus fresh database dumps) and deliver it
//! to a local folder or any rclone remote (Google Drive, OneDrive, Dropbox, S3 and
//! S3-compatible storage, MEGA, Box, pCloud, WebDAV, SFTP …).
//!
//! rclone keeps its own config in VSN's private data folder; OAuth providers sign in in the
//! browser, and passwords are stored obscured by rclone. Nothing here runs through a shell.

use super::*;
use std::process::{Command, Stdio};

const RCLONE_PROVIDERS: &[&str] = &[
    "drive",
    "onedrive",
    "dropbox",
    "box",
    "pcloud",
    "mega",
    "s3",
    "b2",
    "azureblob",
    "gcs",
    "webdav",
    "sftp",
    "ftp",
    "swift",
    "yandex",
    "zoho",
    "koofr",
    "jottacloud",
    "pikpak",
    "storj",
];
/// Parameters rclone must store obscured.
const SECRET_KEYS: &[&str] = &[
    "pass",
    "password",
    "secret_access_key",
    "key",
    "account_key",
];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct BackupDestination {
    pub id: String,
    pub name: String,
    /// `local` (an absolute folder), `rclone` (`remote:path`) or `email` (recipient).
    pub kind: String,
    pub target: String,
    /// Newest archives kept per space at a local destination (0 keeps all).
    #[serde(default)]
    pub keep: u32,
    /// Email only: SMTP server, e.g. smtp.gmail.com:587 (STARTTLS) or host:465 (TLS).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smtp: Option<String>,
    /// Email only: SMTP login, also used as the sender.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smtp_user: Option<String>,
    /// Email only, write-only: the SMTP (app) password, moved into the vault on save.
    #[serde(default, skip_serializing)]
    pub smtp_password: Option<String>,
}

/// Archives larger than this are not emailed (Gmail and Outlook stop at 20–25 MB).
const EMAIL_LIMIT: u64 = 20 * 1024 * 1024;

fn email_secret(id: &str) -> String {
    format!("backup/email/{id}")
}

fn plain_email(value: &str) -> bool {
    value.len() <= 254
        && value
            .split_once('@')
            .is_some_and(|(a, b)| !a.is_empty() && b.contains('.'))
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"@._+-".contains(&b))
}

/// Send the archive as an attachment with curl's SMTP client (TLS required). The login
/// goes to curl on stdin, never on the command line.
fn email_archive(
    destination: &BackupDestination,
    archive: &Path,
    subject: &str,
) -> Result<(), CoreError> {
    let smtp = destination.smtp.as_deref().unwrap_or_default();
    let user = destination.smtp_user.as_deref().unwrap_or_default();
    let password = vsn_vault::reveal(&email_secret(&destination.id)).unwrap_or_default();
    let scheme = if smtp.ends_with(":465") {
        "smtps"
    } else {
        "smtp"
    };
    let program = vsn_system::find_executable("curl")
        .map_err(|_| CoreError::Rejected("curl is not available to send email".into()))?;
    let mut command = Command::new(program);
    command
        .args([
            "--silent",
            "--show-error",
            "--ssl-reqd",
            "--max-time",
            "600",
            "-K",
            "-",
            &format!("{scheme}://{smtp}"),
            "--mail-from",
            user,
            "--mail-rcpt",
            &destination.target,
            "-H",
            &format!("Subject: {subject}"),
            "-H",
            &format!("From: VSN Devify <{user}>"),
            "-H",
            &format!("To: <{}>", destination.target),
            "-F",
            "=(;type=multipart/mixed",
            "-F",
            &format!("={subject}. The archive is attached.;type=text/plain"),
            "-F",
            &format!(
                "file=@{};type=application/gzip;encoder=base64",
                archive.display()
            ),
            "-F",
            "=)",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().map_err(vsn_space::SpaceError::from)?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write as _;
        let escaped = format!("{user}:{password}")
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        let _ = writeln!(stdin, "user = \"{escaped}\"");
    }
    let output = child
        .wait_with_output()
        .map_err(vsn_space::SpaceError::from)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(CoreError::Rejected(format!(
            "email failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct BackupRecord {
    pub space: String,
    pub destination: String,
    pub archive: String,
    pub bytes: u64,
    pub at_unix: u64,
    pub databases: Vec<String>,
}

fn backup_dir() -> Result<PathBuf, CoreError> {
    let dir = vsn_security::data_dir()?.join("backup");
    std::fs::create_dir_all(&dir).map_err(vsn_space::SpaceError::from)?;
    Ok(dir)
}
fn destinations_path() -> Result<PathBuf, CoreError> {
    Ok(backup_dir()?.join("destinations.json"))
}
fn history_path() -> Result<PathBuf, CoreError> {
    Ok(backup_dir()?.join("history.json"))
}
fn rclone_config() -> Result<PathBuf, CoreError> {
    Ok(backup_dir()?.join("rclone.conf"))
}
fn read_json<T: serde::de::DeserializeOwned + Default>(path: &Path) -> T {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn simple_name(value: &str, what: &str) -> Result<(), CoreError> {
    let ok = !value.is_empty()
        && value.len() <= 64
        && !value.starts_with('-')
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if ok {
        Ok(())
    } else {
        Err(CoreError::Rejected(format!(
            "{what} may only use letters, digits, - and _"
        )))
    }
}

/// Run rclone with VSN's private config. Returns (success, combined output tail).
fn rclone(args: &[String], timeout: std::time::Duration) -> Result<(bool, String), CoreError> {
    let program = vsn_system::find_executable("rclone").map_err(|_| {
        CoreError::Rejected(
            "rclone is not installed. Install it (winget install Rclone.Rclone), then try again"
                .into(),
        )
    })?;
    let mut command = Command::new(program);
    command
        .args(args)
        .env("RCLONE_CONFIG", rclone_config()?)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let child = command.spawn().map_err(vsn_space::SpaceError::from)?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    let output = rx
        .recv_timeout(timeout)
        .map_err(|_| CoreError::Rejected("rclone took too long".into()))?
        .map_err(vsn_space::SpaceError::from)?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    let tail: String = text
        .chars()
        .rev()
        .take(4000)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    Ok((output.status.success(), tail))
}

/// Connect a cloud account as an rclone remote. OAuth providers (drive, onedrive, dropbox,
/// box, pcloud …) open the browser to sign in; others take keys/passwords in `params`.
pub fn backup_remote_create(
    principal: &Principal,
    name: &str,
    provider: &str,
    params: &std::collections::BTreeMap<String, String>,
) -> Result<serde_json::Value, CoreError> {
    vsn_policy::require(principal, Permission::SecretsManage)?;
    simple_name(name, "remote name")?;
    if !RCLONE_PROVIDERS.contains(&provider) {
        return Err(CoreError::Rejected(format!(
            "unsupported provider {provider}"
        )));
    }
    let mut args: Vec<String> = vec![
        "config".into(),
        "create".into(),
        name.into(),
        provider.into(),
    ];
    let mut obscure = false;
    for (key, value) in params {
        simple_name(key, "setting name")?;
        if value.len() > 4096 || value.chars().any(char::is_control) {
            return Err(CoreError::Rejected(format!("{key} has an invalid value")));
        }
        obscure |= SECRET_KEYS.contains(&key.as_str());
        args.push(format!("{key}={value}"));
    }
    if obscure {
        args.push("--obscure".into());
    }
    // OAuth sign-in waits for the browser; give the user a few minutes.
    let (ok, output) = rclone(&args, std::time::Duration::from_secs(300))?;
    if !ok {
        return Err(CoreError::Rejected(format!(
            "could not connect {provider}: {}",
            output.trim()
        )));
    }
    Ok(serde_json::json!({"name": name, "provider": provider}))
}

pub fn backup_remote_list(principal: &Principal) -> Result<Vec<String>, CoreError> {
    vsn_policy::require(principal, Permission::ServiceView)?;
    if !rclone_config()?.is_file() {
        return Ok(Vec::new());
    }
    let (ok, output) = rclone(&["listremotes".into()], std::time::Duration::from_secs(20))?;
    if !ok {
        return Err(CoreError::Rejected(output));
    }
    Ok(output
        .lines()
        .map(|l| l.trim().trim_end_matches(':').to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

pub fn backup_remote_delete(principal: &Principal, name: &str) -> Result<bool, CoreError> {
    vsn_policy::require(principal, Permission::SecretsManage)?;
    simple_name(name, "remote name")?;
    let (ok, output) = rclone(
        &["config".into(), "delete".into(), name.into()],
        std::time::Duration::from_secs(20),
    )?;
    if !ok {
        return Err(CoreError::Rejected(output));
    }
    Ok(true)
}

pub fn backup_destinations(principal: &Principal) -> Result<Vec<BackupDestination>, CoreError> {
    vsn_policy::require(principal, Permission::ServiceView)?;
    Ok(read_json(&destinations_path()?))
}

pub fn backup_destination_save(
    principal: &Principal,
    destination: &BackupDestination,
) -> Result<Vec<BackupDestination>, CoreError> {
    vsn_policy::require(principal, Permission::ServiceManage)?;
    simple_name(&destination.id, "destination id")?;
    if destination.name.trim().is_empty() || destination.name.len() > 80 {
        return Err(CoreError::Rejected("give the destination a name".into()));
    }
    match destination.kind.as_str() {
        "local" => {
            let path = Path::new(&destination.target);
            if !path.is_absolute() {
                return Err(CoreError::Rejected(
                    "a local destination must be an absolute folder (another drive, a USB disk or a synced folder)".into(),
                ));
            }
        }
        "rclone" => {
            let (remote, rest) = destination.target.split_once(':').ok_or_else(|| {
                CoreError::Rejected("use remote:folder, e.g. gdrive:Backups/VSN".into())
            })?;
            simple_name(remote, "remote name")?;
            if rest.starts_with('-') || rest.contains("..") || rest.chars().any(char::is_control) {
                return Err(CoreError::Rejected("invalid remote folder".into()));
            }
        }
        "email" => {
            let smtp = destination.smtp.as_deref().unwrap_or_default();
            let host_ok = smtp.rsplit_once(':').is_some_and(|(host, port)| {
                port.parse::<u16>().is_ok_and(|p| p > 0)
                    && !host.is_empty()
                    && !host.starts_with('-')
                    && host
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
            });
            if !plain_email(&destination.target)
                || !host_ok
                || !destination.smtp_user.as_deref().is_some_and(plain_email)
            {
                return Err(CoreError::Rejected(
                    "use a recipient email, an SMTP server like smtp.gmail.com:587 and your SMTP login email".into(),
                ));
            }
            if let Some(password) = destination
                .smtp_password
                .as_deref()
                .filter(|p| !p.is_empty())
            {
                if password.chars().any(char::is_control) || password.len() > 512 {
                    return Err(CoreError::Rejected("invalid SMTP password".into()));
                }
                vsn_policy::require(principal, Permission::SecretsManage)?;
                vsn_vault::set(&email_secret(&destination.id), password)?;
            }
        }
        other => {
            return Err(CoreError::Rejected(format!(
                "unknown destination kind {other}"
            )))
        }
    }
    let mut destination = destination.clone();
    destination.smtp_password = None;
    let destination = &destination;
    let path = destinations_path()?;
    let mut list: Vec<BackupDestination> = read_json(&path);
    list.retain(|d| d.id != destination.id);
    list.push(destination.clone());
    std::fs::write(&path, serde_json::to_vec_pretty(&list).unwrap_or_default())
        .map_err(vsn_space::SpaceError::from)?;
    Ok(list)
}

pub fn backup_destination_delete(principal: &Principal, id: &str) -> Result<bool, CoreError> {
    vsn_policy::require(principal, Permission::ServiceManage)?;
    let path = destinations_path()?;
    let mut list: Vec<BackupDestination> = read_json(&path);
    let before = list.len();
    list.retain(|d| d.id != id);
    std::fs::write(&path, serde_json::to_vec_pretty(&list).unwrap_or_default())
        .map_err(vsn_space::SpaceError::from)?;
    Ok(list.len() != before)
}

pub fn backup_history(principal: &Principal) -> Result<Vec<BackupRecord>, CoreError> {
    vsn_policy::require(principal, Permission::ServiceView)?;
    Ok(read_json(&history_path()?))
}

/// Folders never worth backing up: dependencies, build output and live service data
/// (databases are captured as dumps instead).
const EXCLUDES: &[&str] = &[
    "node_modules",
    "vendor",
    ".git",
    "target",
    ".next",
    "dist",
    ".devify/data",
];

/// Back up a space: dump its databases, archive the folder and deliver the archive.
pub fn space_backup_run(
    principal: &Principal,
    path: &Path,
    destination_id: &str,
    include_databases: bool,
) -> Result<BackupRecord, CoreError> {
    vsn_policy::require(principal, Permission::ProjectView)?;
    vsn_policy::require(principal, Permission::ServiceManage)?;
    let roots = workspace_roots(principal)?;
    let root = vsn_files::resolve_existing(&roots, path)?;
    ensure_space_unlocked(&root)?;
    let spec = vsn_space::load(&root)?;
    let destination = read_json::<Vec<BackupDestination>>(&destinations_path()?)
        .into_iter()
        .find(|d| d.id == destination_id)
        .ok_or_else(|| CoreError::Rejected("unknown backup destination".into()))?;

    let mut databases = Vec::new();
    if include_databases {
        for service in &spec.services {
            if ["postgres", "mysql", "mariadb", "mongodb", "redis", "valkey"]
                .contains(&service.kind.as_str())
            {
                let result = space_db_backup(principal, &root, &service.kind)?;
                if result.status_code != Some(0) {
                    return Err(CoreError::Rejected(format!(
                        "{} dump failed (is the service running?): {}",
                        service.kind,
                        result.output_tail.trim()
                    )));
                }
                databases.push(service.kind.clone());
            }
        }
    }

    let stamp = now_unix();
    let archive_name = format!("{}-{stamp}.tar.gz", spec.name);
    let staging = backup_dir()?.join("staging");
    std::fs::create_dir_all(&staging).map_err(vsn_space::SpaceError::from)?;
    let archive = staging.join(&archive_name);
    let parent = root
        .parent()
        .ok_or_else(|| CoreError::Rejected("a space cannot be a drive root".into()))?;
    let folder = root
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| CoreError::Rejected("space folder name is not valid UTF-8".into()))?
        .to_string();
    let mut args: Vec<String> = vec!["-czf".into(), archive.to_string_lossy().into_owned()];
    for exclude in EXCLUDES {
        args.push(format!("--exclude={folder}/{exclude}"));
    }
    args.extend([
        "-C".into(),
        parent.to_string_lossy().into_owned(),
        "--".into(),
        folder,
    ]);
    let tar = vsn_framework::run_tool("backup archive", &staging, "tar", &args, parent, &[])?;
    if tar.status_code != Some(0) {
        let _ = std::fs::remove_file(&archive);
        return Err(CoreError::Rejected(format!(
            "could not create the archive: {}",
            tar.output_tail.trim()
        )));
    }
    let bytes = std::fs::metadata(&archive).map(|m| m.len()).unwrap_or(0);

    let delivered = match destination.kind.as_str() {
        "local" => {
            let dir = Path::new(&destination.target).join(&spec.name);
            std::fs::create_dir_all(&dir).map_err(vsn_space::SpaceError::from)?;
            let target = dir.join(&archive_name);
            std::fs::copy(&archive, &target).map_err(vsn_space::SpaceError::from)?;
            if destination.keep > 0 {
                prune_local(&dir, &spec.name, destination.keep as usize);
            }
            target.to_string_lossy().into_owned()
        }
        "email" => {
            if bytes > EMAIL_LIMIT {
                let _ = std::fs::remove_file(&archive);
                return Err(CoreError::Rejected(format!(
                    "the archive is {} MB; email allows about 20 MB. Use a cloud or folder destination",
                    bytes / (1024 * 1024)
                )));
            }
            let sent = email_archive(
                &destination,
                &archive,
                &format!("VSN backup of {}", spec.name),
            );
            if let Err(error) = sent {
                let _ = std::fs::remove_file(&archive);
                return Err(error);
            }
            format!("email to {}", destination.target)
        }
        _ => {
            let target = format!(
                "{}/{}/{archive_name}",
                destination.target.trim_end_matches('/'),
                spec.name
            );
            let (ok, output) = rclone(
                &[
                    "copyto".into(),
                    archive.to_string_lossy().into_owned(),
                    target.clone(),
                ],
                std::time::Duration::from_secs(60 * 60),
            )?;
            if !ok {
                let _ = std::fs::remove_file(&archive);
                return Err(CoreError::Rejected(format!(
                    "upload failed: {}",
                    output.trim()
                )));
            }
            target
        }
    };
    let _ = std::fs::remove_file(&archive);
    let record = BackupRecord {
        space: spec.name.clone(),
        destination: destination.name.clone(),
        archive: delivered,
        bytes,
        at_unix: stamp,
        databases,
    };
    let path = history_path()?;
    let mut history: Vec<BackupRecord> = read_json(&path);
    history.push(record.clone());
    let excess = history.len().saturating_sub(500);
    history.drain(..excess);
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&history).unwrap_or_default(),
    )
    .map_err(vsn_space::SpaceError::from)?;
    publish(
        "backup.completed",
        serde_json::json!({"space": record.space, "destination": record.destination, "bytes": bytes}),
    );
    Ok(record)
}

// ---- schedules ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct BackupSchedule {
    pub space_root: PathBuf,
    pub destination: String,
    /// Hours between backups (1 = hourly, 24 = daily, 168 = weekly).
    pub every_hours: u32,
    #[serde(default = "default_true")]
    pub databases: bool,
    #[serde(default)]
    pub last_run_unix: u64,
    #[serde(default)]
    pub last_error: Option<String>,
}
fn default_true() -> bool {
    true
}
fn schedules_path() -> Result<PathBuf, CoreError> {
    Ok(backup_dir()?.join("schedules.json"))
}
fn write_schedules(list: &[BackupSchedule]) -> Result<(), CoreError> {
    std::fs::write(
        schedules_path()?,
        serde_json::to_vec_pretty(list).unwrap_or_default(),
    )
    .map_err(vsn_space::SpaceError::from)?;
    Ok(())
}

pub fn backup_schedules(principal: &Principal) -> Result<Vec<BackupSchedule>, CoreError> {
    vsn_policy::require(principal, Permission::ServiceView)?;
    Ok(read_json(&schedules_path()?))
}

pub fn backup_schedule_save(
    principal: &Principal,
    schedule: &BackupSchedule,
) -> Result<Vec<BackupSchedule>, CoreError> {
    vsn_policy::require(principal, Permission::ServiceManage)?;
    if !(1..=24 * 31).contains(&schedule.every_hours) {
        return Err(CoreError::Rejected(
            "choose between 1 hour and 31 days".into(),
        ));
    }
    let roots = workspace_roots(principal)?;
    let root = vsn_files::resolve_existing(&roots, &schedule.space_root)?;
    vsn_space::load(&root)?;
    if !backup_destinations(principal)?
        .iter()
        .any(|d| d.id == schedule.destination)
    {
        return Err(CoreError::Rejected("unknown backup destination".into()));
    }
    let mut list: Vec<BackupSchedule> = read_json(&schedules_path()?);
    list.retain(|s| !(s.space_root == root && s.destination == schedule.destination));
    list.push(BackupSchedule {
        space_root: root,
        last_run_unix: 0,
        last_error: None,
        ..schedule.clone()
    });
    write_schedules(&list)?;
    Ok(list)
}

pub fn backup_schedule_delete(
    principal: &Principal,
    space_root: &Path,
    destination: &str,
) -> Result<bool, CoreError> {
    vsn_policy::require(principal, Permission::ServiceManage)?;
    let mut list: Vec<BackupSchedule> = read_json(&schedules_path()?);
    let before = list.len();
    list.retain(|s| !(s.space_root == space_root && s.destination == destination));
    write_schedules(&list)?;
    Ok(list.len() != before)
}

/// Run every schedule that is due. The agent calls this every few minutes; a failure is
/// recorded on the schedule and retried at the next interval, never in a tight loop.
pub fn backup_scheduler_tick(principal: &Principal) -> Result<usize, CoreError> {
    let mut list: Vec<BackupSchedule> = read_json(&schedules_path()?);
    let now = now_unix();
    let mut ran = 0;
    for schedule in list.iter_mut() {
        if now < schedule.last_run_unix + u64::from(schedule.every_hours) * 3600 {
            continue;
        }
        schedule.last_run_unix = now;
        schedule.last_error = space_backup_run(
            principal,
            &schedule.space_root,
            &schedule.destination,
            schedule.databases,
        )
        .err()
        .map(|e| e.to_string());
        ran += 1;
    }
    write_schedules(&list)?;
    Ok(ran)
}

// ---- restore -----------------------------------------------------------------------------

/// Restore an archive (a local file or `remote:path/file.tar.gz`) into a **new** folder
/// next to the chosen parent; nothing existing is overwritten.
pub fn space_backup_restore(
    principal: &Principal,
    archive: &str,
    into: &Path,
) -> Result<PathBuf, CoreError> {
    vsn_policy::require(principal, Permission::ProjectEdit)?;
    let roots = workspace_roots(principal)?;
    let parent = vsn_files::resolve_existing(&roots, into)?;
    if !archive.ends_with(".tar.gz")
        || archive.starts_with('-')
        || archive.chars().any(char::is_control)
    {
        return Err(CoreError::Rejected(
            "choose a .tar.gz backup archive".into(),
        ));
    }
    let staging = backup_dir()?.join("staging");
    std::fs::create_dir_all(&staging).map_err(vsn_space::SpaceError::from)?;
    let local_archive = if Path::new(archive).is_absolute() {
        PathBuf::from(archive)
    } else {
        let (remote, _) = archive
            .split_once(':')
            .ok_or_else(|| CoreError::Rejected("use a full file path or remote:path".into()))?;
        simple_name(remote, "remote name")?;
        let local = staging.join(format!("restore-{}.tar.gz", now_unix()));
        let (ok, output) = rclone(
            &[
                "copyto".into(),
                archive.into(),
                local.to_string_lossy().into_owned(),
            ],
            std::time::Duration::from_secs(60 * 60),
        )?;
        if !ok {
            return Err(CoreError::Rejected(format!(
                "download failed: {}",
                output.trim()
            )));
        }
        local
    };
    let stem = Path::new(archive)
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix(".tar.gz"))
        .unwrap_or("backup")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>();
    let target = parent.join(format!("{stem}-restored-{}", now_unix()));
    std::fs::create_dir(&target).map_err(vsn_space::SpaceError::from)?;
    let args: Vec<String> = vec![
        "-xzf".into(),
        local_archive.to_string_lossy().into_owned(),
        "-C".into(),
        target.to_string_lossy().into_owned(),
        "--no-same-owner".into(),
    ];
    let result = vsn_framework::run_tool("restore", &target, "tar", &args, &target, &[])?;
    if local_archive.starts_with(&staging) {
        let _ = std::fs::remove_file(&local_archive);
    }
    if result.status_code != Some(0) {
        return Err(CoreError::Rejected(format!(
            "could not extract the archive: {}",
            result.output_tail.trim()
        )));
    }
    Ok(target)
}

/// Keep only the newest `keep` archives of a space in a local destination folder.
fn prune_local(dir: &Path, space: &str, keep: usize) {
    let prefix = format!("{space}-");
    let mut archives: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&prefix) && n.ends_with(".tar.gz"))
                && p.symlink_metadata().map(|m| m.is_file()).unwrap_or(false)
        })
        .collect();
    archives.sort();
    let excess = archives.len().saturating_sub(keep);
    for old in archives.into_iter().take(excess) {
        let _ = std::fs::remove_file(old);
    }
}

#[cfg(test)]
mod backup_tests {
    use super::*;

    #[test]
    fn email_values_are_plain() {
        assert!(plain_email("me@example.com"));
        for bad in [
            "me",
            "a@b",
            "x@y.com\r\nBcc: z@y.com",
            "<x@y.com>",
            "a b@c.com",
        ] {
            assert!(!plain_email(bad), "{bad:?}");
        }
    }

    #[test]
    fn names_cannot_become_rclone_options() {
        assert!(simple_name("gdrive", "x").is_ok());
        for bad in ["-c", "a:b", "a b", "", "a/b"] {
            assert!(simple_name(bad, "x").is_err(), "{bad}");
        }
    }

    #[test]
    fn local_retention_keeps_the_newest() {
        let dir = std::env::temp_dir().join(format!("vsn-backup-prune-{}", now_unix()));
        std::fs::create_dir_all(&dir).unwrap();
        for n in 1..=4 {
            std::fs::write(dir.join(format!("shop-17000000{n}.tar.gz")), b"x").unwrap();
        }
        std::fs::write(dir.join("other-1.tar.gz"), b"x").unwrap();
        prune_local(&dir, "shop", 2);
        let mut left: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(
            left,
            [
                "other-1.tar.gz",
                "shop-170000003.tar.gz",
                "shop-170000004.tar.gz"
            ]
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
