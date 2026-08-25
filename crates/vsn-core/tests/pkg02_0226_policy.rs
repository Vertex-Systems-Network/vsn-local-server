use vsn_policy::{require, Permission, Principal};

#[test]
fn database_permission_boundary_is_preserved() {
    let local = Principal::local_authenticated();
    assert!(require(&local, Permission::DatabaseView).is_ok());
    assert!(require(&local, Permission::DatabaseQuery).is_ok());
    assert!(require(&local, Permission::DatabaseWrite).is_ok());
    assert!(require(&local, Permission::DatabaseDestructive).is_err());
    assert!(Principal::remote_delegated("remote", Permission::DatabaseDestructive).is_err());
}
