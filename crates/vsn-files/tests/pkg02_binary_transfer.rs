use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use sha2::Digest;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use vsn_files::{
    abort_binary_upload, binary_upload_status, file_digest, write_binary_chunk, FileError,
    MAX_BINARY_CHUNK_BYTES,
};

fn temp_root(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "vsn-pkg02-0217-{name}-{}-{stamp}",
        std::process::id()
    ))
}

fn part_path(final_path: &std::path::Path, transfer_id: &str) -> PathBuf {
    let name = final_path.file_name().unwrap().to_string_lossy();
    final_path.with_file_name(format!(".{name}.vsn-upload-{transfer_id}.part"))
}

fn backup_path(final_path: &std::path::Path, transfer_id: &str) -> PathBuf {
    let name = final_path.file_name().unwrap().to_string_lossy();
    final_path.with_file_name(format!(".{name}.vsn-backup-{transfer_id}.bak"))
}

#[test]
fn offset_mismatch_does_not_advance_committed_bytes() {
    let root = temp_root("offset");
    fs::create_dir_all(&root).unwrap();
    let target = root.join("payload.bin");
    let transfer = "transfer_offset_01";

    let first = B64.encode(b"first");
    let result = write_binary_chunk(
        std::slice::from_ref(&root),
        &target,
        transfer,
        0,
        &first,
        false,
        None,
    )
    .unwrap();
    assert_eq!(result.committed_bytes, 5);

    let second = B64.encode(b"second");
    let error = write_binary_chunk(
        std::slice::from_ref(&root),
        &target,
        transfer,
        3,
        &second,
        false,
        None,
    )
    .unwrap_err();
    assert!(matches!(error, FileError::Invalid(message) if message.contains("offset mismatch")));

    let status = binary_upload_status(std::slice::from_ref(&root), &target, transfer).unwrap();
    assert_eq!(status.committed_bytes, 5);
    assert!(status.partial_exists);
    assert!(!status.final_exists);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn oversized_chunk_is_rejected_without_creating_partial() {
    let root = temp_root("chunk-limit");
    fs::create_dir_all(&root).unwrap();
    let target = root.join("payload.bin");
    let transfer = "transfer_chunk_limit_01";
    let encoded = B64.encode(vec![7u8; MAX_BINARY_CHUNK_BYTES + 1]);

    let error = write_binary_chunk(
        std::slice::from_ref(&root),
        &target,
        transfer,
        0,
        &encoded,
        false,
        None,
    )
    .unwrap_err();
    assert!(
        matches!(error, FileError::TooLarge(bytes) if bytes == (MAX_BINARY_CHUNK_BYTES + 1) as u64)
    );
    assert!(!target.exists());
    assert!(!part_path(&target, transfer).exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn finalize_reports_and_persists_sha256() {
    let root = temp_root("finalize");
    fs::create_dir_all(&root).unwrap();
    let target = root.join("payload.bin");
    let transfer = "transfer_finalize_01";
    let bytes = b"alpha-beta-gamma";
    let expected = format!("{:x}", sha2::Sha256::digest(bytes));
    let encoded = B64.encode(bytes);

    let result = write_binary_chunk(
        std::slice::from_ref(&root),
        &target,
        transfer,
        0,
        &encoded,
        true,
        Some(&expected),
    )
    .unwrap();
    assert!(result.complete);
    assert_eq!(result.sha256.as_deref(), Some(expected.as_str()));

    let digest = file_digest(std::slice::from_ref(&root), &target).unwrap();
    assert_eq!(digest.bytes, bytes.len() as u64);
    assert_eq!(digest.sha256, expected);
    assert_eq!(fs::read(&target).unwrap(), bytes);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn checksum_mismatch_discards_partial_without_replacing_destination() {
    let root = temp_root("checksum");
    fs::create_dir_all(&root).unwrap();
    let target = root.join("payload.bin");
    fs::write(&target, b"accepted-before").unwrap();
    let transfer = "transfer_checksum_01";
    let encoded = B64.encode(b"replacement");

    let error = write_binary_chunk(
        std::slice::from_ref(&root),
        &target,
        transfer,
        0,
        &encoded,
        true,
        Some(&"0".repeat(64)),
    )
    .unwrap_err();
    assert!(matches!(error, FileError::Invalid(message) if message.contains("checksum mismatch")));
    assert_eq!(fs::read(&target).unwrap(), b"accepted-before");
    assert!(!part_path(&target, transfer).exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn status_recovers_interrupted_replace_before_reporting_progress() {
    let root = temp_root("status-recovery");
    fs::create_dir_all(&root).unwrap();
    let target = root.join("payload.bin");
    let transfer = "transfer_status_01";
    let part = part_path(&target, transfer);
    let backup = backup_path(&target, transfer);
    fs::write(&part, b"new-partial").unwrap();
    fs::write(&backup, b"original-destination").unwrap();

    let status = binary_upload_status(std::slice::from_ref(&root), &target, transfer).unwrap();
    assert!(status.partial_exists);
    assert!(status.final_exists);
    assert_eq!(status.committed_bytes, b"new-partial".len() as u64);
    assert_eq!(fs::read(&target).unwrap(), b"original-destination");
    assert!(!backup.exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn abort_recovers_interrupted_replace_before_discarding_partial() {
    let root = temp_root("abort-recovery");
    fs::create_dir_all(&root).unwrap();
    let target = root.join("payload.bin");
    let transfer = "transfer_abort_01";
    let part = part_path(&target, transfer);
    let backup = backup_path(&target, transfer);
    fs::write(&part, b"new-partial").unwrap();
    fs::write(&backup, b"original-destination").unwrap();

    let removed = abort_binary_upload(std::slice::from_ref(&root), &target, transfer).unwrap();
    assert!(removed);
    assert!(!part.exists());
    assert!(
        target.exists(),
        "abort must restore the pre-finalize destination"
    );
    assert_eq!(fs::read(&target).unwrap(), b"original-destination");
    assert!(
        !backup.exists(),
        "abort must not orphan the finalize backup"
    );
    let _ = fs::remove_dir_all(root);
}
