use std::{
    fs,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};
use vsn_files::{delete_path, move_path, write_text, FileError};

fn unique_path(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("vsn-0216-{label}-{}-{nonce}", process::id()))
}

#[cfg(windows)]
fn create_dir_link(link: &Path, target: &Path) {
    let status = std::process::Command::new("cmd")
        .arg("/C")
        .arg("mklink")
        .arg("/J")
        .arg(link)
        .arg(target)
        .status()
        .expect("start mklink /J");
    assert!(status.success(), "mklink /J failed with {status}");
}

#[cfg(unix)]
fn create_dir_link(link: &Path, target: &Path) {
    std::os::unix::fs::symlink(target, link).expect("create directory symlink");
}

#[cfg(any(windows, unix))]
#[test]
fn recursive_delete_removes_link_entry_not_target() {
    let root = unique_path("delete");
    let target = root.join("target");
    let link = root.join("link");
    fs::create_dir_all(&target).expect("create target");
    fs::write(target.join("keep.txt"), b"keep").expect("write target sentinel");
    create_dir_link(&link, &target);

    let result = delete_path(std::slice::from_ref(&root), &link, true).expect("delete link");
    assert_eq!(result.operation, "delete");
    assert!(fs::symlink_metadata(&link).is_err(), "link entry survived delete");
    assert!(target.join("keep.txt").is_file(), "delete followed link target");

    fs::remove_dir_all(root).expect("cleanup root");
}

#[cfg(any(windows, unix))]
#[test]
fn move_renames_link_entry_not_target() {
    let root = unique_path("move");
    let target = root.join("target");
    let link = root.join("link");
    let moved = root.join("moved-link");
    fs::create_dir_all(&target).expect("create target");
    fs::write(target.join("keep.txt"), b"keep").expect("write target sentinel");
    create_dir_link(&link, &target);

    let result = move_path(std::slice::from_ref(&root), &link, &moved).expect("move link");
    assert_eq!(result.operation, "move");
    assert!(fs::symlink_metadata(&link).is_err(), "old link entry survived move");
    assert!(fs::symlink_metadata(&moved).is_ok(), "moved link entry is missing");
    assert!(target.join("keep.txt").is_file(), "move renamed link target");

    delete_path(std::slice::from_ref(&root), &moved, true).expect("delete moved link");
    fs::remove_dir_all(root).expect("cleanup root");
}

#[cfg(any(windows, unix))]
#[test]
fn write_through_parent_link_outside_workspace_is_rejected() {
    let root = unique_path("inside");
    let outside = unique_path("outside");
    let link = root.join("outside-link");
    fs::create_dir_all(&root).expect("create root");
    fs::create_dir_all(&outside).expect("create outside");
    fs::write(outside.join("keep.txt"), b"outside").expect("write outside sentinel");
    create_dir_link(&link, &outside);

    let error = write_text(
        std::slice::from_ref(&root),
        &link.join("new.txt"),
        "must not escape",
    )
    .expect_err("outside parent link must be rejected");
    assert!(matches!(error, FileError::OutsideWorkspace));
    assert!(!outside.join("new.txt").exists());
    assert_eq!(fs::read(outside.join("keep.txt")).expect("read sentinel"), b"outside");

    delete_path(std::slice::from_ref(&root), &link, true).expect("remove outside link entry");
    fs::remove_dir_all(root).expect("cleanup root");
    fs::remove_dir_all(outside).expect("cleanup outside");
}