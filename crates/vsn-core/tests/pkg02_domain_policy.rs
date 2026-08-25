use vsn_policy::{require, Permission, Principal};

fn assert_network_manage_denied(error: impl ToString) {
    let message = error.to_string();
    assert!(
        message.contains("permission denied: network.manage"),
        "unexpected denial: {message}"
    );
}

#[test]
fn ordinary_authenticated_principal_cannot_reach_privileged_network_mutations() {
    let principal = Principal::local_authenticated();
    assert!(require(&principal, Permission::NetworkView).is_ok());
    assert!(require(&principal, Permission::NetworkManage).is_err());

    assert_network_manage_denied(
        vsn_core::domain_apply_hosts(&principal, "demo.test").expect_err("apply must deny"),
    );
    assert_network_manage_denied(
        vsn_core::domain_remove_hosts(&principal, "demo.test").expect_err("remove must deny"),
    );
    assert_network_manage_denied(vsn_core::caddy_reload(&principal).expect_err("reload must deny"));
}

#[test]
fn elevated_network_principal_is_distinct_and_remote_delegation_fails_closed() {
    let ordinary = Principal::local_authenticated();
    let elevated = Principal::local_network_admin();

    assert_ne!(ordinary.kind, elevated.kind);
    assert!(require(&elevated, Permission::NetworkManage).is_ok());
    assert!(Principal::remote_delegated("remote", Permission::NetworkManage).is_err());
}
