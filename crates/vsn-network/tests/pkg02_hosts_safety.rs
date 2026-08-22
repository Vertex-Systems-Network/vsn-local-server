use std::{fs, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};
use vsn_network::apply_hosts_domain_at;

fn temp_hosts(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("vsn-pkg02-0224-{name}-{}-{stamp}", std::process::id()))
}

#[test]
fn hosts_mutation_fails_closed_when_existing_file_is_not_utf8() {
    let path = temp_hosts("invalid-utf8");
    let original = vec![0xff, 0xfe, 0xfd, b'\n', b'x'];
    fs::write(&path, &original).expect("fixture");

    let result = apply_hosts_domain_at(&path, "demo.test", "127.0.0.1");
    assert!(result.is_err(), "invalid existing hosts content must fail closed");
    assert_eq!(fs::read(&path).expect("read preserved fixture"), original);

    let tmp = path.with_extension("vsn.tmp");
    assert!(!tmp.exists(), "failed read must not stage a replacement");
    let _ = fs::remove_file(path);
}

#[test]
fn managed_hosts_update_preserves_unmanaged_content() {
    let path = temp_hosts("preserve");
    let original = "127.0.0.1 localhost\n10.0.0.8 internal.example\n# operator note\n";
    fs::write(&path, original).expect("fixture");

    let mutation = apply_hosts_domain_at(&path, "demo.test", "127.0.0.1").expect("apply");
    assert!(mutation.changed);
    let updated = fs::read_to_string(&path).expect("updated hosts");
    assert!(updated.contains("127.0.0.1 localhost"));
    assert!(updated.contains("10.0.0.8 internal.example"));
    assert!(updated.contains("# operator note"));
    assert!(updated.contains("127.0.0.1\tdemo.test"));

    let _ = fs::remove_file(path);
}
