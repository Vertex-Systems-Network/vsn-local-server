use std::collections::BTreeSet;
use vsn_policy::{Permission, Principal};

fn principal(id: &str, permissions: impl IntoIterator<Item = Permission>) -> Principal {
    Principal {
        id: id.into(),
        kind: "test".into(),
        permissions: permissions.into_iter().collect::<BTreeSet<_>>(),
    }
}

#[test]
fn dns_start_requires_network_view_and_service_manage() {
    let network_only = principal("network-only", [Permission::NetworkView]);
    let error = vsn_core::dns_start(&network_only, "127.0.0.1:53535").unwrap_err();
    assert!(error.to_string().contains("service.manage"));

    let service_only = principal("service-only", [Permission::ServiceManage]);
    let error = vsn_core::dns_start(&service_only, "127.0.0.1:53535").unwrap_err();
    assert!(error.to_string().contains("network.view"));

    let elevated_network_only = principal("network-manage-only", [Permission::NetworkManage]);
    let error = vsn_core::dns_start(&elevated_network_only, "127.0.0.1:53535").unwrap_err();
    assert!(error.to_string().contains("network.view"));
}

#[test]
fn dns_stop_requires_service_manage_not_network_manage() {
    let elevated_network_only = principal("network-manage-only", [Permission::NetworkManage]);
    let error = vsn_core::dns_stop(&elevated_network_only).unwrap_err();
    assert!(error.to_string().contains("service.manage"));
}
