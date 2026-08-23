use vsn_policy::{Permission, Principal};

#[test]
fn container_reads_and_lifecycle_keep_existing_runtime_permissions() {
    let view = Principal::remote_delegated("pkg02-0215-view", Permission::RuntimeView)
        .expect("construct runtime viewer");
    assert!(vsn_core::container_detect(&view).is_ok());

    let denied_action = vsn_core::container_action(&view, "docker", "start", "vsn-demo")
        .expect_err("RuntimeView must not authorize lifecycle mutation");
    assert!(denied_action
        .to_string()
        .contains("permission denied: runtime.manage"));

    let manage = Principal::remote_delegated("pkg02-0215-manage", Permission::RuntimeManage)
        .expect("construct runtime manager");
    let denied_read = vsn_core::container_list(&manage, "docker", true)
        .expect_err("RuntimeManage alone must not authorize container reads");
    assert!(denied_read
        .to_string()
        .contains("permission denied: runtime.view"));

    let backend_error = vsn_core::container_action(&manage, "invalid", "start", "vsn-demo")
        .expect_err("invalid backend must fail after RuntimeManage authorization");
    assert!(backend_error
        .to_string()
        .contains("unsupported container backend: invalid"));
}

#[test]
fn local_authenticated_principal_retains_both_container_permissions() {
    let local = Principal::local_authenticated();
    assert!(vsn_policy::require(&local, Permission::RuntimeView).is_ok());
    assert!(vsn_policy::require(&local, Permission::RuntimeManage).is_ok());
}
