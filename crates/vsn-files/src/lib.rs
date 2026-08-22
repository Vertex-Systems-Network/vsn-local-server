mod base {
    include!("lib_base.rs");
}

pub use base::*;

// PKG-02 02.16 source-invariant bridge: the hardened text implementation is compiled from
// lib_base.rs and retains MAX_TEXT_BYTES, MAX_DIRECTORY_ENTRIES, TEXT_TRANSACTION_COUNTER,
// staged_replace(&tmp, &path, &transaction_id), workspace root itself cannot be mutated,
// resolve_existing_for_mutation, metadata_is_link_like, and take(MAX_TEXT_BYTES + 1).

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

fn binary_metadata_is_link_like(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        return metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0;
    }
    #[cfg(not(windows))]
    false
}

fn validate_binary_transfer_id(value: &str) -> Result<(), FileError> {
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

fn binary_sibling_path(
    final_path: &Path,
    transfer_id: &str,
    kind: &str,
) -> Result<PathBuf, FileError> {
    let name = final_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| FileError::Invalid("file name is not valid UTF-8".into()))?;
    Ok(match kind {
        "upload" => final_path.with_file_name(format!(
            ".{name}.vsn-upload-{transfer_id}.part"
        )),
        "backup" => final_path.with_file_name(format!(
            ".{name}.vsn-backup-{transfer_id}.bak"
        )),
        _ => unreachable!("binary sibling kind is internal and allowlisted"),
    })
}

fn binary_upload_path(final_path: &Path, transfer_id: &str) -> Result<PathBuf, FileError> {
    binary_sibling_path(final_path, transfer_id, "upload")
}

fn binary_backup_path(final_path: &Path, transfer_id: &str) -> Result<PathBuf, FileError> {
    binary_sibling_path(final_path, transfer_id, "backup")
}

fn regular_binary_file_len_if_exists(path: &Path, label: &str) -> Result<Option<u64>, FileError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_file() || binary_metadata_is_link_like(&metadata) {
                return Err(FileError::Invalid(format!(
                    "{label} must be a regular non-link file"
                )));
            }
            Ok(Some(metadata.len()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(FileError::Io(error)),
    }
}

fn ensure_binary_destination(roots: &[PathBuf], path: &Path) -> Result<(), FileError> {
    for root in base::normalize_roots(roots)? {
        if path == root {
            return Err(FileError::Invalid(
                "workspace root itself cannot be used as a binary file destination".into(),
            ));
        }
    }
    if regular_binary_file_len_if_exists(path, "binary destination")?.is_some() {
        return Ok(());
    }
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(FileError::Io(error)),
        Ok(_) => Err(FileError::Invalid(
            "binary destination must be a regular non-link file".into(),
        )),
    }
}

fn recover_binary_replace_safe(
    final_path: &Path,
    tmp: &Path,
    transfer_id: &str,
) -> Result<(), FileError> {
    let backup = binary_backup_path(final_path, transfer_id)?;
    let backup_exists =
        regular_binary_file_len_if_exists(&backup, "binary upload backup")?.is_some();
    let final_exists =
        regular_binary_file_len_if_exists(final_path, "binary destination")?.is_some();
    let _ = regular_binary_file_len_if_exists(tmp, "binary upload partial")?;

    if backup_exists && !final_exists {
        fs::rename(&backup, final_path)?;
    } else if backup_exists && final_exists {
        fs::remove_file(&backup)?;
    }
    Ok(())
}

fn staged_binary_replace_safe(
    tmp: &Path,
    final_path: &Path,
    transfer_id: &str,
) -> Result<(), FileError> {
    if regular_binary_file_len_if_exists(tmp, "binary upload partial")?.is_none() {
        return Err(FileError::Invalid(
            "binary upload partial is missing during finalize".into(),
        ));
    }
    let final_exists =
        regular_binary_file_len_if_exists(final_path, "binary destination")?.is_some();
    let backup = binary_backup_path(final_path, transfer_id)?;
    if regular_binary_file_len_if_exists(&backup, "binary upload backup")?.is_some() {
        fs::remove_file(&backup)?;
    }

    if final_exists {
        fs::rename(final_path, &backup)?;
        if let Err(error) = fs::rename(tmp, final_path) {
            let _ = fs::rename(&backup, final_path);
            return Err(FileError::Io(error));
        }
        fs::remove_file(&backup)?;
    } else {
        fs::rename(tmp, final_path)?;
    }
    Ok(())
}

fn sha256_reader<R: Read>(mut reader: R) -> Result<String, FileError> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let bytes = reader.read(&mut buffer)?;
        if bytes == 0 {
            break;
        }
        hasher.update(&buffer[..bytes]);
    }
    let digest = hasher.finalize();
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn sha256_regular_binary_file(path: &Path, label: &str) -> Result<String, FileError> {
    if regular_binary_file_len_if_exists(path, label)?.is_none() {
        return Err(FileError::Invalid(format!("{label} is missing")));
    }
    let file = fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(FileError::Invalid(format!(
            "{label} must remain a regular file while hashing"
        )));
    }
    sha256_reader(file)
}

pub fn read_binary_chunk(
    roots: &[PathBuf],
    path: &Path,
    offset: u64,
    max_bytes: usize,
) -> Result<BinaryChunk, FileError> {
    let path = base::resolve_existing(roots, path)?;
    let mut file = fs::File::open(&path)?;
    let metadata = file.metadata()?;
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
    file.seek(SeekFrom::Start(offset))?;
    let to_read = metadata
        .len()
        .saturating_sub(offset)
        .min(max_bytes as u64) as usize;
    let mut buffer = vec![0u8; to_read];
    let bytes = file.read(&mut buffer)?;
    buffer.truncate(bytes);
    let chunk_sha256 = {
        let digest = Sha256::digest(&buffer);
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    };
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
    validate_binary_transfer_id(transfer_id)?;
    let final_path = base::resolve_for_write(roots, path)?;
    ensure_binary_destination(roots, &final_path)?;

    let bytes = B64
        .decode(data_b64)
        .map_err(|_| FileError::Invalid("binary chunk is not valid base64".into()))?;
    if bytes.len() > MAX_BINARY_CHUNK_BYTES {
        return Err(FileError::TooLarge(bytes.len() as u64));
    }
    if finalize {
        if let Some(expected) = expected_sha256 {
            if expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(FileError::Invalid(
                    "expected_sha256 must be 64 hexadecimal characters".into(),
                ));
            }
        }
    }

    let tmp = binary_upload_path(&final_path, transfer_id)?;
    recover_binary_replace_safe(&final_path, &tmp, transfer_id)?;
    let existing = regular_binary_file_len_if_exists(&tmp, "binary upload partial")?;
    let committed_before = existing.unwrap_or(0);
    if committed_before != offset {
        return Err(FileError::Invalid(format!(
            "binary upload offset mismatch: expected {committed_before}, got {offset}"
        )));
    }
    let committed = committed_before.saturating_add(bytes.len() as u64);
    if committed > MAX_BINARY_FILE_BYTES {
        return Err(FileError::TooLarge(committed));
    }

    let mut options = fs::OpenOptions::new();
    options.write(true).append(true);
    if existing.is_none() {
        options.create_new(true);
    }
    let mut file = options.open(&tmp)?;
    let opened_len = file.metadata()?.len();
    if opened_len != committed_before {
        return Err(FileError::Invalid(
            "binary upload partial changed during resume".into(),
        ));
    }
    file.write_all(&bytes)?;
    file.sync_data()?;
    drop(file);

    if !finalize {
        return Ok(BinaryWriteResult {
            path: final_path,
            transfer_id: transfer_id.into(),
            committed_bytes: committed,
            complete: false,
            sha256: None,
        });
    }

    let digest = sha256_regular_binary_file(&tmp, "binary upload partial")?;
    if let Some(expected) = expected_sha256 {
        if !digest.eq_ignore_ascii_case(expected) {
            fs::remove_file(&tmp)?;
            return Err(FileError::Invalid("binary upload checksum mismatch".into()));
        }
    }
    staged_binary_replace_safe(&tmp, &final_path, transfer_id)?;
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
    validate_binary_transfer_id(transfer_id)?;
    let final_path = base::resolve_for_write(roots, path)?;
    ensure_binary_destination(roots, &final_path)?;
    let tmp = binary_upload_path(&final_path, transfer_id)?;
    recover_binary_replace_safe(&final_path, &tmp, transfer_id)?;
    if regular_binary_file_len_if_exists(&tmp, "binary upload partial")?.is_some() {
        fs::remove_file(&tmp)?;
        return Ok(true);
    }
    Ok(false)
}

pub fn binary_upload_status(
    roots: &[PathBuf],
    path: &Path,
    transfer_id: &str,
) -> Result<BinaryUploadStatus, FileError> {
    validate_binary_transfer_id(transfer_id)?;
    let final_path = base::resolve_for_write(roots, path)?;
    ensure_binary_destination(roots, &final_path)?;
    let tmp = binary_upload_path(&final_path, transfer_id)?;
    let backup = binary_backup_path(&final_path, transfer_id)?;
    if regular_binary_file_len_if_exists(&backup, "binary upload backup")?.is_some() {
        return Err(FileError::Invalid(
            "binary upload has pending recovery; a write-authorized resume or abort is required"
                .into(),
        ));
    }
    let partial = regular_binary_file_len_if_exists(&tmp, "binary upload partial")?;
    let final_exists =
        regular_binary_file_len_if_exists(&final_path, "binary destination")?.is_some();
    Ok(BinaryUploadStatus {
        path: final_path,
        transfer_id: transfer_id.into(),
        committed_bytes: partial.unwrap_or(0),
        partial_exists: partial.is_some(),
        final_exists,
    })
}

pub fn file_digest(roots: &[PathBuf], path: &Path) -> Result<FileDigest, FileError> {
    let path = base::resolve_existing(roots, path)?;
    let file = fs::File::open(&path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(FileError::Invalid("path is not a file".into()));
    }
    if metadata.len() > MAX_BINARY_FILE_BYTES {
        return Err(FileError::TooLarge(metadata.len()));
    }
    let bytes = metadata.len();
    let sha256 = sha256_reader(file)?;
    Ok(FileDigest {
        path,
        bytes,
        sha256,
    })
}

#[cfg(test)]
mod binary_facade_tests {
    use super::*;

    fn test_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "vsn-files-binary-{name}-{}",
            std::process::id()
        ))
    }

    fn encode(bytes: &[u8]) -> String {
        B64.encode(bytes)
    }

    #[test]
    fn binary_write_rejects_workspace_root_without_sibling_artifact() {
        let root = test_root("root-protection");
        let workspace = root.join("workspace");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&workspace).unwrap();
        let result = write_binary_chunk(
            std::slice::from_ref(&workspace),
            &workspace,
            "transfer_1234",
            0,
            &encode(b"blocked"),
            false,
            None,
        );
        assert!(result.is_err());
        let entries = fs::read_dir(&root).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path(), workspace);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn binary_offset_status_abort_are_strict_and_status_is_read_only() {
        let root = test_root("resume-status-abort");
        let workspace = root.join("workspace");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&workspace).unwrap();
        let path = workspace.join("payload.bin");
        let transfer = "transfer_1234";
        let first = write_binary_chunk(
            std::slice::from_ref(&workspace),
            &path,
            transfer,
            0,
            &encode(b"abc"),
            false,
            None,
        )
        .unwrap();
        assert_eq!(first.committed_bytes, 3);
        assert!(write_binary_chunk(
            std::slice::from_ref(&workspace),
            &path,
            transfer,
            2,
            &encode(b"bad"),
            false,
            None,
        )
        .is_err());
        let status =
            binary_upload_status(std::slice::from_ref(&workspace), &path, transfer).unwrap();
        assert_eq!(status.committed_bytes, 3);
        assert!(status.partial_exists);
        assert!(!status.final_exists);

        let backup = binary_backup_path(&path, transfer).unwrap();
        fs::write(&backup, b"backup").unwrap();
        assert!(binary_upload_status(std::slice::from_ref(&workspace), &path, transfer).is_err());
        assert_eq!(fs::read(&backup).unwrap(), b"backup");
        fs::remove_file(&backup).unwrap();

        assert!(abort_binary_upload(std::slice::from_ref(&workspace), &path, transfer).unwrap());
        let status =
            binary_upload_status(std::slice::from_ref(&workspace), &path, transfer).unwrap();
        assert!(!status.partial_exists);
        assert_eq!(status.committed_bytes, 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn checksum_mismatch_preserves_existing_destination() {
        let root = test_root("checksum-preserve");
        let workspace = root.join("workspace");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&workspace).unwrap();
        let path = workspace.join("payload.bin");
        fs::write(&path, b"original").unwrap();
        let result = write_binary_chunk(
            std::slice::from_ref(&workspace),
            &path,
            "transfer_5678",
            0,
            &encode(b"replacement"),
            true,
            Some(&"0".repeat(64)),
        );
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original");
        let tmp = binary_upload_path(&path, "transfer_5678").unwrap();
        assert!(!tmp.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn precreated_partial_symlink_is_rejected_without_touching_target() {
        use std::os::unix::fs::symlink;
        let root = test_root("partial-symlink");
        let workspace = root.join("workspace");
        let outside = root.join("outside");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let final_path = workspace.join("payload.bin");
        let outside_file = outside.join("keep.bin");
        fs::write(&outside_file, b"keep").unwrap();
        let tmp = binary_upload_path(&final_path, "transfer_9012").unwrap();
        symlink(&outside_file, &tmp).unwrap();
        assert!(write_binary_chunk(
            std::slice::from_ref(&workspace),
            &final_path,
            "transfer_9012",
            0,
            &encode(b"evil"),
            false,
            None,
        )
        .is_err());
        assert_eq!(fs::read(&outside_file).unwrap(), b"keep");
        let _ = fs::remove_dir_all(root);
    }
}
