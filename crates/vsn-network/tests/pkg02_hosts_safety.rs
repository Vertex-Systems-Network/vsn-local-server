use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
#[cfg(windows)]
use vsn_network::reload_caddyfile_with_executable;
use vsn_network::{
    apply_hosts_domain_at, caddy_site, remove_hosts_domain_at, render_caddyfile, LocalCertificate,
};

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "vsn-pkg02-0224-{name}-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("sandbox");
        Self { dir }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn count(text: &str, needle: &str) -> usize {
    text.match_indices(needle).count()
}

#[test]
fn hosts_apply_and_remove_preserve_unmanaged_and_unrelated_managed_entries() {
    let sandbox = Sandbox::new("hosts-preserve");
    let path = sandbox.path("hosts");
    let original = concat!(
        "127.0.0.1 localhost\n",
        "10.0.0.8 internal.example\n",
        "# operator note\n",
        "\n",
        "# BEGIN VSN MANAGED\n",
        "::1\tother.test\n",
        "# END VSN MANAGED\n",
    );
    fs::write(&path, original).expect("fixture");

    let first = apply_hosts_domain_at(&path, "demo.test", "127.0.0.1").expect("apply");
    assert!(first.changed);
    assert_eq!(first.domain, "demo.test");
    let applied = fs::read_to_string(&path).expect("applied hosts");
    assert!(applied.contains("127.0.0.1 localhost"));
    assert!(applied.contains("10.0.0.8 internal.example"));
    assert!(applied.contains("# operator note"));
    assert!(applied.contains("::1\tother.test"));
    assert!(applied.contains("127.0.0.1\tdemo.test"));
    assert_eq!(count(&applied, "# BEGIN VSN MANAGED"), 1);
    assert_eq!(count(&applied, "# END VSN MANAGED"), 1);
    assert_eq!(count(&applied, "demo.test"), 1);

    let second = apply_hosts_domain_at(&path, "demo.test", "127.0.0.1").expect("reapply");
    assert!(!second.changed);
    assert_eq!(
        fs::read_to_string(&path).expect("idempotent hosts"),
        applied
    );

    let removed = remove_hosts_domain_at(&path, "demo.test").expect("remove");
    assert!(removed.changed);
    let after_remove = fs::read_to_string(&path).expect("removed hosts");
    assert!(after_remove.contains("127.0.0.1 localhost"));
    assert!(after_remove.contains("10.0.0.8 internal.example"));
    assert!(after_remove.contains("# operator note"));
    assert!(after_remove.contains("::1\tother.test"));
    assert!(!after_remove.contains("demo.test"));
    assert_eq!(count(&after_remove, "# BEGIN VSN MANAGED"), 1);
    assert_eq!(count(&after_remove, "# END VSN MANAGED"), 1);

    let second_remove = remove_hosts_domain_at(&path, "demo.test").expect("second remove");
    assert!(!second_remove.changed);
    assert_eq!(
        fs::read_to_string(&path).expect("idempotent remove"),
        after_remove
    );
}

#[test]
fn path_scoped_hosts_mutation_rejects_non_loopback_without_changing_file() {
    let sandbox = Sandbox::new("hosts-loopback");
    let path = sandbox.path("hosts");
    let original = b"127.0.0.1 localhost\n";
    fs::write(&path, original).expect("fixture");

    let result = apply_hosts_domain_at(&path, "demo.test", "10.10.10.10");
    assert!(result.is_err());
    assert_eq!(fs::read(&path).expect("preserved fixture"), original);
}

#[test]
fn hosts_read_failures_are_fail_closed() {
    let sandbox = Sandbox::new("hosts-read-fail");
    let invalid = sandbox.path("hosts-invalid");
    let original = vec![0xff, 0xfe, 0xfd, b'\n', b'x'];
    fs::write(&invalid, &original).expect("fixture");

    assert!(apply_hosts_domain_at(&invalid, "demo.test", "127.0.0.1").is_err());
    assert_eq!(
        fs::read(&invalid).expect("invalid fixture preserved"),
        original
    );
    assert!(remove_hosts_domain_at(&invalid, "demo.test").is_err());
    assert_eq!(
        fs::read(&invalid).expect("remove fixture preserved"),
        original
    );

    let missing = sandbox.path("missing-hosts");
    assert!(apply_hosts_domain_at(&missing, "demo.test", "127.0.0.1").is_err());
    assert!(!missing.exists());
    assert!(remove_hosts_domain_at(&missing, "demo.test").is_err());
    assert!(!missing.exists());
}

#[test]
fn caddy_render_is_loopback_only_and_never_auto_installs_trust() {
    let internal = caddy_site("internal.test", 8123, None).expect("internal site");
    let explicit = caddy_site(
        "explicit.test",
        8124,
        Some(LocalCertificate {
            domain: "explicit.test".into(),
            cert_path: PathBuf::from(r"C:\sandbox\explicit.test.pem"),
            key_path: PathBuf::from(r"C:\sandbox\explicit.test-key.pem"),
        }),
    )
    .expect("explicit site");
    let rendered = render_caddyfile(&[internal, explicit]).expect("render");

    assert!(rendered.contains("skip_install_trust"));
    assert!(rendered.contains("reverse_proxy 127.0.0.1:8123"));
    assert!(rendered.contains("reverse_proxy 127.0.0.1:8124"));
    assert!(rendered.contains("tls internal"));
    assert!(rendered.contains("explicit.test.pem"));
    assert!(!rendered.contains("tls_insecure_skip_verify"));
    assert!(caddy_site("zero.test", 0, None).is_err());
}

#[cfg(windows)]
fn write_fake_caddy(path: &std::path::Path, log: &std::path::Path, validation_exit: i32) {
    let body = format!(
        "@echo off\r\necho %1>>\"{}\"\r\nif /I \"%1\"==\"validate\" exit /b {}\r\nif /I \"%1\"==\"reload\" exit /b 0\r\nexit /b 9\r\n",
        log.display(),
        validation_exit
    );
    fs::write(path, body).expect("fake caddy");
}

#[cfg(windows)]
#[test]
fn caddy_reload_validates_before_reload_and_rejects_bad_config_paths() {
    let sandbox = Sandbox::new("caddy-reload");
    let config = sandbox.path("Caddyfile");
    let helper = sandbox.path("caddy.cmd");
    let calls = sandbox.path("calls.log");
    fs::write(&config, "{\n\tauto_https off\n\tskip_install_trust\n}\n").expect("config");

    write_fake_caddy(&helper, &calls, 17);
    let failed = reload_caddyfile_with_executable(&config, &helper);
    assert!(failed.is_err());
    assert_eq!(
        fs::read_to_string(&calls).expect("failure calls").trim(),
        "validate"
    );

    fs::write(&calls, "").expect("clear calls");
    write_fake_caddy(&helper, &calls, 0);
    let success = reload_caddyfile_with_executable(&config, &helper).expect("reload success");
    assert!(success.validated);
    assert!(success.reloaded);
    let successful_calls: Vec<_> = fs::read_to_string(&calls)
        .expect("success calls")
        .lines()
        .map(str::trim)
        .collect();
    assert_eq!(successful_calls, vec!["validate", "reload"]);

    assert!(
        reload_caddyfile_with_executable(std::path::Path::new("relative-Caddyfile"), &helper)
            .is_err()
    );
    assert!(reload_caddyfile_with_executable(&sandbox.path("missing-Caddyfile"), &helper).is_err());
}
