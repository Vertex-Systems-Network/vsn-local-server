#!/usr/bin/env python3
"""Repository-semantic Store/MSIX compatibility probe.

This does not package, submit, or sign anything. It proves the current VSN
privilege boundary and verifies that Store identity inputs remain placeholders.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path


def require_text(path: Path, needles: tuple[str, ...]) -> str:
    text = path.read_text(encoding="utf-8")
    for needle in needles:
        if needle not in text:
            raise AssertionError(f"{path}: required architecture marker missing: {needle}")
    return text


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", default=".")
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    root = Path(args.repo_root)

    desktop = require_text(
        root / "apps/desktop/src-tauri/src/main.rs",
        ("vsn_ipc::call", "agent_call", "generate_handler![agent_call]"),
    )
    tauri = require_text(
        root / "apps/desktop/src-tauri/tauri.conf.json",
        ('"installMode": "currentUser"', '"publisher": "Vertex Systems Network"'),
    )
    agent = require_text(
        root / "apps/agent/src/main.rs",
        (
            'Some("install") => install()',
            '"create"',
            "LocalService",
            'Some("network-admin")',
            "network-admin commands require OS elevation",
            "domain_apply_hosts",
            "local_ca_install",
            "dns_os_apply",
        ),
    )
    template_path = root / "apps/desktop/src-tauri/msix/store-packaging-inputs.json.template"
    template = json.loads(template_path.read_text(encoding="utf-8"))
    assert template["identity_name"] == "${PARTNER_CENTER_IDENTITY_NAME}"
    assert template["publisher"] == "${PARTNER_CENTER_PUBLISHER}"
    assert template["runtime_boundary"] == "external_or_preprovisioned_authenticated_agent"
    assert template["production_ready"] is False

    forbidden_desktop = (
        "network-admin commands require OS elevation",
        'Some("install") => install()',
        "domain_apply_hosts",
        "local_ca_install",
        "dns_os_apply",
    )
    leaked = [marker for marker in forbidden_desktop if marker in desktop]
    if leaked:
        raise AssertionError(f"privileged markers moved into desktop shell: {leaked}")

    report = {
        "schema_version": 1,
        "lane": "msix-store-preimplementation",
        "mode": "NON_ACCEPTANCE_PREIMPLEMENTATION",
        "production_evidence_consumed": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "store_submission_performed": False,
        "package_built": False,
        "package_signed": False,
        "partner_center_identity_present": False,
        "identity_placeholders_verified": True,
        "desktop_current_user": '"installMode": "currentUser"' in tauri,
        "desktop_ipc_delegation": "vsn_ipc::call" in desktop,
        "agent_windows_service_management": 'Some("install") => install()' in agent,
        "agent_elevated_network_admin": "network-admin commands require OS elevation" in agent,
        "machine_mutation_capabilities": {
            "hosts": "domain_apply_hosts" in agent,
            "local_ca": "local_ca_install" in agent,
            "resolver": "dns_os_apply" in agent,
            "windows_service": "LocalService" in agent,
        },
        "store_shell_source_preparation_viable": True,
        "standalone_full_feature_store_package_ready": False,
        "required_runtime_boundary": "external_or_preprovisioned_authenticated_agent",
        "direct_installer_lane_must_remain": True,
        "reason": "privileged service/network capabilities are intentionally outside the non-elevated desktop shell",
    }

    out = Path(args.output)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, sort_keys=True))
    print("PKG03_MSIX_STORE_COMPATIBILITY=PASS_WITH_EXTERNAL_AGENT_BOUNDARY")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
