use std::path::Path;
use vsn_policy::{Permission, Principal};

#[test]
fn diagnostics_require_existing_read_permissions() {
    let denied = Principal::remote_delegated("pkg02-0214-denied", Permission::ProjectView)
        .expect("construct narrow denied principal");

    assert!(vsn_core::processes(&denied).is_err());
    assert!(vsn_core::process_metrics(&denied, std::process::id()).is_err());
    assert!(vsn_core::ports(&denied).is_err());
    assert!(vsn_core::port_conflicts(&denied, 80).is_err());
    assert!(vsn_core::tcp_health(&denied, "127.0.0.1", 9, 100).is_err());
    assert!(vsn_core::tail_log(&denied, Path::new("does-not-matter.log"), 10).is_err());
}

#[test]
fn local_authenticated_principal_has_only_existing_diagnostics_permissions() {
    let local = Principal::local_authenticated();
    assert!(vsn_policy::require(&local, Permission::MachineView).is_ok());
    assert!(vsn_policy::require(&local, Permission::NetworkView).is_ok());
    assert!(vsn_policy::require(&local, Permission::ServiceView).is_ok());
    assert!(vsn_policy::require(&local, Permission::FilesRead).is_ok());
}
