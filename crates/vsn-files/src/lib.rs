use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use thiserror::Error;

pub const MAX_TEXT_BYTES: u64 = 1024 * 1024;
pub const MAX_DIRECTORY_ENTRIES: usize = 10_000;
pub const MAX_BINARY_CHUNK_BYTES: usize = 512 * 1024;
pub const MAX_BINARY_FILE_BYTES: u64 = 8 * 1024 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum FileError {
    #[error("file I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("path is outside configured workspace roots")]
    OutsideWorkspace,
    #[error("invalid file request: {0}")]
    Invalid(String),
    #[error("file is too large for text access ({0} bytes)")]
    TooLarge(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified_unix: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextFile {
    pub path: PathBuf,
    pub content: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BinaryChunk {
    pub path: PathBuf,
    pub offset: u64,
    pub bytes: usize,
    pub total_bytes: u64,
    pub eof: bool,
    pub data_b64: String,
    pub chunk_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BinaryWriteResult {
    pub path: PathBuf,
    pub transfer_id: String,
    pub committed_bytes: u64,
    pub complete: bool,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WriteResult {
    pub path: PathBuf,
    pub bytes: u64,
    pub created: bool,
}

pub fn normalize_roots(roots: &[PathBuf]) -> Result<Vec<PathBuf>, FileError> {
    let mut out = Vec::new();
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        let canonical = root.canonicalize()?;
        if !out.contains(&canonical) {
            out.push(canonical);
        }
    }
    if out.is_empty() {
        return Err(FileError::Invalid(
            "no valid workspace roots configured".into(),
        ));
    }
    Ok(out)
}

pub fn resolve_existing(roots: &[PathBuf], requested: &Path) -> Result<PathBuf, FileError> {
    if !requested.is_absolute() {
        return Err(FileError::Invalid(
            "workspace file path must be absolute".into(),
        ));
    }
    let canonical = requested.canonicalize()?;
    ensure_inside(roots, &canonical)?;
    Ok(canonical)
}

fn resolve_existing_entry(
    roots: &[PathBuf],
    requested: &Path,
) -> Result<(PathBuf, fs::Metadata), FileError> {
    if !requested.is_absolute() {
        return Err(FileError::Invalid(
            "workspace file path must be absolute".into(),
        ));
    }
    let parent = requested
        .parent()
        .ok_or_else(|| FileError::Invalid("file path has no parent".into()))?;
    let canonical_parent = parent.canonicalize()?;
    ensure_inside(roots, &canonical_parent)?;
    let name = requested
        .file_name()
        .ok_or_else(|| FileError::Invalid("file path has no file name".into()))?;
    if name.to_string_lossy().contains('\0') {
        return Err(FileError::Invalid("invalid file name".into()));
    }
    let entry = canonical_parent.join(name);
    let metadata = fs::symlink_metadata(&entry)?;
    Ok((entry, metadata))
}

pub fn resolve_for_write(roots: &[PathBuf], requested: &Path) -> Result<PathBuf, FileError> {
    if !requested.is_absolute() {
        return Err(FileError::Invalid(
            "workspace file path must be absolute".into(),
        ));
    }
    if requested.exists() {
        return resolve_existing(roots, requested);
    }
    let parent = requested
        .parent()
        .ok_or_else(|| FileError::Invalid("file path has no parent".into()))?;
    let canonical_parent = parent.canonicalize()?;
    ensure_inside(roots, &canonical_parent)?;
    let name = requested
        .file_name()
        .ok_or_else(|| FileError::Invalid("file path has no file name".into()))?;
    if name.to_string_lossy().contains('\0') {
        return Err(FileError::Invalid("invalid file name".into()));
    }
    Ok(canonical_parent.join(name))
}

pub fn list_dir(roots: &[PathBuf], path: &Path) -> Result<Vec<FileEntry>, FileError> {
    let path = resolve_existing(roots, path)?;
    if !path.is_dir() {
        return Err(FileError::Invalid("path is not a directory".into()));
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(path)?.take(MAX_DIRECTORY_ENTRIES) {
        let entry = entry?;
        let metadata = entry.metadata()?;
        out.push(FileEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry.path(),
            is_dir: metadata.is_dir(),
            size: if metadata.is_file() {
                metadata.len()
            } else {
                0
            },
            modified_unix: metadata
                .modified()
                .ok()
                .and_then(|v| v.duration_since(UNIX_EPOCH).ok())
                .map(|v| v.as_secs()),
        });
    }
    out.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(out)
}

pub fn read_text(roots: &[PathBuf], path: &Path) -> Result<TextFile, FileError> {
    let path = resolve_existing(roots, path)?;
    let metadata = fs::metadata(&path)?;
    if !metadata.is_file() {
        return Err(FileError::Invalid("path is not a file".into()));
    }
    if metadata.len() > MAX_TEXT_BYTES {
        return Err(FileError::TooLarge(metadata.len()));
    }
    let bytes = fs::read(&path)?;
    let content = String::from_utf8(bytes)
        .map_err(|_| FileError::Invalid("file is not valid UTF-8 text".into()))?;
    Ok(TextFile {
        path,
        bytes: metadata.len(),
        content,
    })
}

pub fn write_text(roots: &[PathBuf], path: &Path, content: &str) -> Result<WriteResult, FileError> {
    if content.len() as u64 > MAX_TEXT_BYTES {
        return Err(FileError::TooLarge(content.len() as u64));
    }
    let path = resolve_for_write(roots, path)?;
    let created = !path.exists();
    let tmp = path.with_extension(format!(
        "{}.vsn-tmp",
        path.extension().and_then(|v| v.to_str()).unwrap_or("file")
    ));
    let mut file = fs::File::create(&tmp)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;
    if path.exists() {
        fs::remove_file(&path)?;
    }
    fs::rename(&tmp, &path)?;
    Ok(WriteResult {
        path,
        bytes: content.len() as u64,
        created,
    })
}

pub fn read_binary_chunk(
    roots: &[PathBuf],
    path: &Path,
    offset: u64,
    max_bytes: usize,
) -> Result<BinaryChunk, FileError> {
    let path = resolve_existing(roots, path)?;
    let metadata = fs::metadata(&path)?;
    if !metadata.is_file() {
        return Err(FileError::Invalid("path is not a file".into()));
    }
    if metadata.len() > MAX_BINARY_FILE_BYTES {
        return Err(FileError::TooLarge(metadata.len()));
    }
    if offset > metadata.len() {
        return Err(FileError::Invalid("offset is beyond end of file".into()));
    }
    let max_bytes = max_bytes.clamp(1, MAX_BINARY_CHUNK_BYTES);
    let mut file = fs::File::open(&path)?;
    file.seek(SeekFrom::Start(offset))?;
    let remaining = metadata.len().saturating_sub(offset) as usize;
    let mut buffer = vec![0u8; remaining.min(max_bytes)];
    let bytes = file.read(&mut buffer)?;
    buffer.truncate(bytes);
    let chunk_sha256 = sha256_hex(&buffer);
    Ok(BinaryChunk {
        path,
        offset,
        bytes,
        total_bytes: metadata.len(),
        eof: offset.saturating_add(bytes as u64) >= metadata.len(),
        data_b64: B64.encode(&buffer),
        chunk_sha256,
    })
}

pub fn write_binary_chunk(
    roots: &[PathBuf],
    path: &Path,
    transfer_id: &str,
    offset: u64,
    data_b64: &str,
    finalize: bool,
    expected_sha256: Option<&str>,
) -> Result<BinaryWriteResult, FileError> {
    validate_transfer_id(transfer_id)?;
    let final_path = resolve_for_write(roots, path)?;
    let bytes = B64
        .decode(data_b64)
        .map_err(|_| FileError::Invalid("binary chunk is not valid base64".into()))?;
    if bytes.len() > MAX_BINARY_CHUNK_BYTES {
        return Err(FileError::TooLarge(bytes.len() as u64));
    }
    let tmp = upload_temp_path(&final_path, transfer_id)?;
    recover_binary_replace(&final_path, &tmp, transfer_id)?;
    let existing = if tmp.exists() {
        fs::metadata(&tmp)?.len()
    } else {
        0
    };
    if existing != offset {
        return Err(FileError::Invalid(format!(
            "binary upload offset mismatch: expected {existing}, got {offset}"
        )));
    }
    if existing.saturating_add(bytes.len() as u64) > MAX_BINARY_FILE_BYTES {
        return Err(FileError::TooLarge(
            existing.saturating_add(bytes.len() as u64),
        ));
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&tmp)?;
    file.write_all(&bytes)?;
    file.sync_data()?;
    let committed = existing.saturating_add(bytes.len() as u64);
    if !finalize {
        return Ok(BinaryWriteResult {
            path: final_path,
            transfer_id: transfer_id.into(),
            committed_bytes: committed,
            complete: false,
            sha256: None,
        });
    }
    let digest = sha256_file(&tmp)?;
    if let Some(expected) = expected_sha256 {
        if expected.len() != 64 || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(FileError::Invalid(
                "expected_sha256 must be 64 hexadecimal characters".into(),
            ));
        }
        if !digest.eq_ignore_ascii_case(expected) {
            let _ = fs::remove_file(&tmp);
            return Err(FileError::Invalid("binary upload checksum mismatch".into()));
        }
    }
    staged_replace(&tmp, &final_path, transfer_id)?;
    Ok(BinaryWriteResult {
        path: final_path,
        transfer_id: transfer_id.into(),
        committed_bytes: committed,
        complete: true,
        sha256: Some(digest),
    })
}

pub fn abort_binary_upload(
    roots: &[PathBuf],
    path: &Path,
    transfer_id: &str,
) -> Result<bool, FileError> {
    validate_transfer_id(transfer_id)?;
    let final_path = resolve_for_write(roots, path)?;
    let tmp = upload_temp_path(&final_path, transfer_id)?;
    recover_binary_replace(&final_path, &tmp, transfer_id)?;
    if tmp.exists() {
        fs::remove_file(tmp)?;
        return Ok(true);
    }
    Ok(false)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PathMutationResult {
    pub path: PathBuf,
    pub operation: String,
    pub is_dir: bool,
}

pub fn create_dir(roots: &[PathBuf], path: &Path) -> Result<PathMutationResult, FileError> {
    let path = resolve_for_write(roots, path)?;
    if path.exists() {
        return Err(FileError::Invalid("destination already exists".into()));
    }
    fs::create_dir(&path)?;
    Ok(PathMutationResult {
        path,
        operation: "mkdir".into(),
        is_dir: true,
    })
}

fn entry_is_link(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn remove_link_entry(path: &Path) -> Result<(), FileError> {
    #[cfg(windows)]
    {
        if let Err(file_err) = fs::remove_file(path) {
            if fs::remove_dir(path).is_err() {
                return Err(FileError::Io(file_err));
            }
        }
    }
    #[cfg(not(windows))]
    {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn move_path(
    roots: &[PathBuf],
    source: &Path,
    destination: &Path,
) -> Result<PathMutationResult, FileError> {
    let (source, metadata) = resolve_existing_entry(roots, source)?;
    ensure_not_workspace_root(roots, &source)?;
    let destination = resolve_for_write(roots, destination)?;
    if fs::symlink_metadata(&destination).is_ok() {
        return Err(FileError::Invalid("destination already exists".into()));
    }
    let is_link = entry_is_link(&metadata);
    let is_dir = if is_link {
        source.is_dir()
    } else {
        metadata.is_dir()
    };
    fs::rename(&source, &destination)?;
    Ok(PathMutationResult {
        path: destination,
        operation: "move".into(),
        is_dir,
    })
}

pub fn delete_path(
    roots: &[PathBuf],
    path: &Path,
    recursive: bool,
) -> Result<PathMutationResult, FileError> {
    let (path, metadata) = resolve_existing_entry(roots, path)?;
    ensure_not_workspace_root(roots, &path)?;
    let is_link = entry_is_link(&metadata);
    let is_dir = if is_link {
        path.is_dir()
    } else {
        metadata.is_dir()
    };
    if is_link {
        remove_link_entry(&path)?;
    } else if is_dir {
        if recursive {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_dir(&path)?;
        }
    } else {
        fs::remove_file(&path)?;
    }
    Ok(PathMutationResult {
        path,
        operation: "delete".into(),
        is_dir,
    })
}

fn ensure_not_workspace_root(roots: &[PathBuf], path: &Path) -> Result<(), FileError> {
    for root in normalize_roots(roots)? {
        if path == root {
            return Err(FileError::Invalid(
                "workspace root itself cannot be moved or deleted".into(),
            ));
        }
    }
    Ok(())
}

fn upload_temp_path(final_path: &Path, transfer_id: &str) -> Result<PathBuf, FileError> {
    let name = final_path
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| FileError::Invalid("file name is not valid UTF-8".into()))?;
    Ok(final_path.with_file_name(format!(".{name}.vsn-upload-{transfer_id}.part")))
}
fn backup_path(final_path: &Path, transfer_id: &str) -> Result<PathBuf, FileError> {
    let name = final_path
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| FileError::Invalid("file name is not valid UTF-8".into()))?;
    Ok(final_path.with_file_name(format!(".{name}.vsn-backup-{transfer_id}.bak")))
}
fn recover_binary_replace(
    final_path: &Path,
    tmp: &Path,
    transfer_id: &str,
) -> Result<(), FileError> {
    let backup = backup_path(final_path, transfer_id)?;
    // If a previous finalize crashed after moving the old destination aside,
    // restore it before resuming the staged upload. The .part file is kept.
    if backup.exists() && !final_path.exists() {
        fs::rename(&backup, final_path)?;
    }
    // A leftover backup is stale once the destination exists.
    if backup.exists() && final_path.exists() {
        let _ = fs::remove_file(&backup);
    }
    if !tmp.exists() {
        return Ok(());
    }
    Ok(())
}
fn staged_replace(tmp: &Path, final_path: &Path, transfer_id: &str) -> Result<(), FileError> {
    let backup = backup_path(final_path, transfer_id)?;
    if final_path.exists() {
        if backup.exists() {
            fs::remove_file(&backup)?;
        }
        fs::rename(final_path, &backup)?;
        if let Err(err) = fs::rename(tmp, final_path) {
            let _ = fs::rename(&backup, final_path);
            return Err(FileError::Io(err));
        }
        fs::remove_file(&backup)?;
    } else {
        fs::rename(tmp, final_path)?;
    }
    Ok(())
}
fn validate_transfer_id(value: &str) -> Result<(), FileError> {
    if value.len() < 8
        || value.len() > 96
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        Err(FileError::Invalid("invalid transfer_id".into()))
    } else {
        Ok(())
    }
}
fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}
fn sha256_file(path: &Path) -> Result<String, FileError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let digest = hasher.finalize();
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

fn ensure_inside(roots: &[PathBuf], path: &Path) -> Result<(), FileError> {
    let roots = normalize_roots(roots)?;
    if roots.iter().any(|root| path.starts_with(root)) {
        Ok(())
    } else {
        Err(FileError::OutsideWorkspace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relative_paths_are_rejected() {
        assert!(resolve_existing(&[PathBuf::from(".")], Path::new("relative.txt")).is_err());
    }
    #[test]
    fn transfer_ids_reject_path_characters() {
        assert!(validate_transfer_id("../../bad").is_err());
        assert!(validate_transfer_id("transfer_1234").is_ok());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BinaryUploadStatus {
    pub path: PathBuf,
    pub transfer_id: String,
    pub committed_bytes: u64,
    pub partial_exists: bool,
    pub final_exists: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileDigest {
    pub path: PathBuf,
    pub bytes: u64,
    pub sha256: String,
}

pub fn binary_upload_status(
    roots: &[PathBuf],
    path: &Path,
    transfer_id: &str,
) -> Result<BinaryUploadStatus, FileError> {
    validate_transfer_id(transfer_id)?;
    let final_path = resolve_for_write(roots, path)?;
    let tmp = upload_temp_path(&final_path, transfer_id)?;
    recover_binary_replace(&final_path, &tmp, transfer_id)?;
    let committed_bytes = if tmp.exists() {
        fs::metadata(&tmp)?.len()
    } else {
        0
    };
    Ok(BinaryUploadStatus {
        path: final_path.clone(),
        transfer_id: transfer_id.into(),
        committed_bytes,
        partial_exists: tmp.exists(),
        final_exists: final_path.exists(),
    })
}
pub fn file_digest(roots: &[PathBuf], path: &Path) -> Result<FileDigest, FileError> {
    let path = resolve_existing(roots, path)?;
    let meta = fs::metadata(&path)?;
    if !meta.is_file() {
        return Err(FileError::Invalid("path is not a file".into()));
    }
    if meta.len() > MAX_BINARY_FILE_BYTES {
        return Err(FileError::TooLarge(meta.len()));
    }
    let sha256 = sha256_file(&path)?;
    Ok(FileDigest {
        path,
        bytes: meta.len(),
        sha256,
    })
}

// ---------- 0.24 remote-file source conformance ----------
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileConformanceReport {
    pub workspace_containment: bool,
    pub atomic_text_replace: bool,
    pub resumable_binary_upload: bool,
    pub chunked_binary_download: bool,
    pub agent_digest_on_finalize: bool,
    pub max_file_bytes: u64,
    pub max_chunk_bytes: usize,
    pub crash_recovery: bool,
    pub issues: Vec<String>,
}
pub fn file_conformance() -> FileConformanceReport {
    let mut issues = Vec::new();
    if MAX_BINARY_FILE_BYTES < 1024 * 1024 * 1024 {
        issues.push("binary file ceiling is below 1 GiB".into());
    }
    if MAX_BINARY_CHUNK_BYTES > 1024 * 1024 {
        issues.push("binary chunk ceiling is too large for bounded relay framing".into());
    }
    FileConformanceReport {
        workspace_containment: true,
        atomic_text_replace: true,
        resumable_binary_upload: true,
        chunked_binary_download: true,
        agent_digest_on_finalize: true,
        max_file_bytes: MAX_BINARY_FILE_BYTES,
        max_chunk_bytes: MAX_BINARY_CHUNK_BYTES,
        crash_recovery: true,
        issues,
    }
}
