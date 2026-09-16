#!/usr/bin/env python3
"""Freeze dormant PKG-05 Linux/macOS platform + layout preimplementation.

NON_ACCEPTANCE_PREIMPLEMENTATION only. This harness audits current source and freezes
candidate platform/layout invariants without assigning canonical PKG-05 task numbers,
claiming Linux/macOS production support, producing release packages, or activating PKG-05.
Canonical PKG-05 task definitions must be reconciled after PKG-04 is genuinely complete.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

MODE = "NON_ACCEPTANCE_PREIMPLEMENTATION"
PACKAGE_ID = "PKG-05"
CANONICAL_BLOCKER = "PKG-04:COMPLETE"
CANDIDATE_PLATFORMS = (
    ("linux", "x86_64"),
    ("macos", "x86_64"),
    ("macos", "aarch64"),
)


class PlatformLayoutError(ValueError):
    pass


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_text(root: Path, relative: str) -> str:
    return (root / relative).read_text(encoding="utf-8")


def require_patterns(text: str, patterns: tuple[str, ...], label: str) -> None:
    missing = [pattern for pattern in patterns if pattern not in text]
    if missing:
        raise PlatformLayoutError(f"{label} source baseline drifted; missing patterns: {missing}")


def source_baseline(repo_root: Path) -> dict:
    tauri_path = repo_root / "apps/desktop/src-tauri/tauri.conf.json"
    config_path = repo_root / "crates/vsn-config/src/lib.rs"
    security_path = repo_root / "crates/vsn-security/src/lib.rs"
    audit_path = repo_root / "crates/vsn-audit/src/lib.rs"
    core_path = repo_root / "crates/vsn-core/src/lib.rs"
    update_path = repo_root / "crates/vsn-update/src/lib.rs"
    tracker_path = repo_root / "docs/MASTER-EXECUTION-STATUS.json"
    icons_dir = repo_root / "apps/desktop/src-tauri/icons"

    tauri = json.loads(tauri_path.read_text(encoding="utf-8"))
    if tauri.get("productName") != "VSN Dev Platform" or tauri.get("identifier") != "dev.vsn.platform":
        raise PlatformLayoutError("Tauri product identity baseline drifted")
    if tauri.get("version") != "0.38.1":
        raise PlatformLayoutError("Tauri product version baseline drifted")
    bundle = tauri.get("bundle")
    if not isinstance(bundle, dict) or bundle.get("active") is not True or bundle.get("targets") != "all":
        raise PlatformLayoutError("Tauri bundle baseline must remain active with targets=all for this audit")
    configured_icons = bundle.get("icon")
    if configured_icons != ["icons/icon.ico"]:
        raise PlatformLayoutError("configured bundle icon baseline changed; reconcile platform assets before continuing")

    icon_names = sorted(path.name for path in icons_dir.iterdir() if path.is_file())
    if "icon.ico" not in icon_names or "icon.png" not in icon_names:
        raise PlatformLayoutError("current icon inventory must include both icon.ico and icon.png")

    config = config_path.read_text(encoding="utf-8")
    require_patterns(
        config,
        (
            'ProjectDirs::from("dev", "VSN", "VSN Platform")',
            'dirs.config_dir().join("config.json")',
            "#[cfg(unix)]",
            "fs::File::open(path)?.sync_all()?",
        ),
        "configuration",
    )

    security = security_path.read_text(encoding="utf-8")
    require_patterns(
        security,
        (
            'ProjectDirs::from("dev", "VSN", "VSN Platform")',
            "dirs.data_local_dir().to_path_buf()",
            'data_dir()?.join("security").join("device.json")',
            'const KEYRING_SERVICE: &str = "vsn-agent"',
        ),
        "security data",
    )

    audit = audit_path.read_text(encoding="utf-8")
    require_patterns(
        audit,
        ('vsn_security::data_dir()?.join("audit").join("agent.jsonl")',),
        "audit data",
    )

    core = core_path.read_text(encoding="utf-8")
    require_patterns(
        core,
        (
            'vsn_security::data_dir()?.join("processes")',
            'vsn_security::data_dir()?.join("runtimes")',
        ),
        "core mutable data",
    )

    update = update_path.read_text(encoding="utf-8")
    require_patterns(
        update,
        (
            'root.join(".vsn-update").join("state.json")',
            'root.join(".vsn-update")',
            'join("previous")',
            'join("failed-current")',
        ),
        "updater control state",
    )

    tracker = json.loads(tracker_path.read_text(encoding="utf-8"))
    package = next((row for row in tracker.get("packages", []) if row.get("id") == PACKAGE_ID), None)
    if package != {
        "id": "PKG-05",
        "name": "Linux + macOS Release",
        "done": 0,
        "required": 23,
        "percent": 0.0,
        "status": "NOT_STARTED",
    }:
        raise PlatformLayoutError("canonical PKG-05 dormant tracker baseline changed; activation-time reconciliation required")
    if tracker.get("active_package") != "PKG-03" or tracker.get("active_task") != "03.22":
        raise PlatformLayoutError("canonical active package/task changed; reconcile this dormant preimplementation lane")

    source_hashes = {}
    for relative, path in (
        ("tauri", tauri_path),
        ("config", config_path),
        ("security", security_path),
        ("audit", audit_path),
        ("core", core_path),
        ("update", update_path),
        ("tracker", tracker_path),
    ):
        source_hashes[relative] = sha256_bytes(path.read_bytes())

    return {
        "product_name": tauri["productName"],
        "product_version": tauri["version"],
        "identifier": tauri["identifier"],
        "bundle_targets": bundle["targets"],
        "bundle_targets_all_is_production_support_evidence": False,
        "configured_bundle_icons": configured_icons,
        "icon_inventory": icon_names,
        "icon_inventory_has_png_and_ico": True,
        "config_root_source_backed": "ProjectDirs.config_dir",
        "config_file_source_backed": "config.json",
        "data_root_source_backed": "ProjectDirs.data_local_dir",
        "device_metadata_under_data_root": "security/device.json",
        "audit_log_under_data_root": "audit/agent.jsonl",
        "managed_process_state_under_data_root": "processes",
        "runtime_state_under_data_root": "runtimes",
        "update_control_under_install_root": ".vsn-update",
        "unix_parent_directory_durability_sync_present": True,
        "pkg05_required_task_count": 23,
        "pkg05_status": "NOT_STARTED",
        "source_sha256": source_hashes,
    }


def validate_platform_row(row: dict) -> None:
    identity = (row.get("os"), row.get("arch"))
    if identity not in CANDIDATE_PLATFORMS:
        raise PlatformLayoutError(f"unexpected candidate platform: {identity}")
    if row.get("status") != "CANDIDATE_ONLY":
        raise PlatformLayoutError("platform status must remain CANDIDATE_ONLY")
    if row.get("production_support_claimed") is not False:
        raise PlatformLayoutError("dormant preimplementation cannot claim platform production support")
    if row.get("release_package_format_frozen") is not False:
        raise PlatformLayoutError("release package format cannot be frozen before canonical reconciliation")
    if row.get("production_signing_or_notarization_evidence") is not False:
        raise PlatformLayoutError("production signing/notarization evidence cannot be projected from fixture work")


def validate_layout(layout: dict) -> None:
    if layout.get("immutable_application_bytes") != "PLATFORM_PACKAGE_OWNED_CANDIDATE":
        raise PlatformLayoutError("immutable application bytes must remain a platform-package candidate boundary")
    if layout.get("mutable_config") != "ProjectDirs.config_dir/config.json":
        raise PlatformLayoutError("mutable config must retain the source-backed ProjectDirs config boundary")
    if layout.get("mutable_data") != "ProjectDirs.data_local_dir":
        raise PlatformLayoutError("mutable data must retain the source-backed ProjectDirs local-data boundary")
    if layout.get("update_transaction_control") != "install_root/.vsn-update":
        raise PlatformLayoutError("update transaction control baseline drifted")
    if layout.get("mutable_config_inside_immutable_package") is not False:
        raise PlatformLayoutError("mutable config must not be projected inside immutable package bytes")
    if layout.get("mutable_data_inside_immutable_package") is not False:
        raise PlatformLayoutError("mutable data must not be projected inside immutable package bytes")
    if layout.get("exact_linux_install_root_frozen") is not False or layout.get("exact_macos_bundle_root_frozen") is not False:
        raise PlatformLayoutError("platform install roots cannot be frozen by dormant source audit")


def validate_governance_projection(projection: dict) -> None:
    if projection.get("canonical_task_definitions_frozen") is not False:
        raise PlatformLayoutError("canonical PKG-05 task definitions are not frozen in current repository governance")
    if projection.get("canonical_task_ids_assigned") is not False:
        raise PlatformLayoutError("dormant preimplementation must not assign canonical PKG-05 task IDs")
    if projection.get("pkg05_activated") is not False:
        raise PlatformLayoutError("dormant preimplementation must not activate PKG-05")
    if projection.get("pkg05_completion_projected") is not False:
        raise PlatformLayoutError("dormant preimplementation must not project PKG-05 completion")
    if projection.get("future_canonical_task_reconciliation_required") is not True:
        raise PlatformLayoutError("future canonical task reconciliation must remain required")


def assert_rejected(fn, expected: str) -> str:
    try:
        fn()
    except PlatformLayoutError as exc:
        text = str(exc)
        if expected not in text:
            raise AssertionError((expected, text))
        return text
    raise AssertionError(f"expected rejection containing: {expected}")


def run(repo_root: Path, source_commit: str) -> tuple[dict, dict, list[dict]]:
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise PlatformLayoutError("source commit must be lowercase 40-character git SHA")
    baseline = source_baseline(repo_root)

    platforms = [
        {
            "os": os_name,
            "arch": arch,
            "status": "CANDIDATE_ONLY",
            "production_support_claimed": False,
            "release_package_format_frozen": False,
            "production_signing_or_notarization_evidence": False,
        }
        for os_name, arch in CANDIDATE_PLATFORMS
    ]
    for row in platforms:
        validate_platform_row(row)

    layout = {
        "immutable_application_bytes": "PLATFORM_PACKAGE_OWNED_CANDIDATE",
        "mutable_config": "ProjectDirs.config_dir/config.json",
        "mutable_data": "ProjectDirs.data_local_dir",
        "mutable_config_inside_immutable_package": False,
        "mutable_data_inside_immutable_package": False,
        "device_metadata": "ProjectDirs.data_local_dir/security/device.json",
        "audit_log": "ProjectDirs.data_local_dir/audit/agent.jsonl",
        "managed_process_state": "ProjectDirs.data_local_dir/processes",
        "runtime_state": "ProjectDirs.data_local_dir/runtimes",
        "update_transaction_control": "install_root/.vsn-update",
        "exact_linux_install_root_frozen": False,
        "exact_macos_bundle_root_frozen": False,
    }
    validate_layout(layout)

    governance = {
        "canonical_blocker": CANONICAL_BLOCKER,
        "canonical_task_definitions_frozen": False,
        "canonical_task_ids_assigned": False,
        "pkg05_activated": False,
        "pkg05_completion_projected": False,
        "future_canonical_task_reconciliation_required": True,
    }
    validate_governance_projection(governance)

    matrix: list[dict] = [
        {"case": "current-source-platform-layout-baseline", "result": "PASS"},
        {"case": "candidate-platform-matrix", "result": "PASS"},
        {"case": "source-backed-mutable-layout", "result": "PASS"},
        {"case": "dormant-governance-boundary", "result": "PASS"},
    ]

    bad = dict(platforms[0])
    bad["status"] = "SUPPORTED"
    matrix.append({"case": "reject-supported-platform-claim", "result": "REJECTED", "reason": assert_rejected(lambda: validate_platform_row(bad), "CANDIDATE_ONLY")})

    bad = dict(platforms[1])
    bad["production_support_claimed"] = True
    matrix.append({"case": "reject-production-support-claim", "result": "REJECTED", "reason": assert_rejected(lambda: validate_platform_row(bad), "production support")})

    bad = dict(platforms[1])
    bad["release_package_format_frozen"] = True
    matrix.append({"case": "reject-package-format-freeze", "result": "REJECTED", "reason": assert_rejected(lambda: validate_platform_row(bad), "package format")})

    bad = dict(platforms[2])
    bad["production_signing_or_notarization_evidence"] = True
    matrix.append({"case": "reject-signing-notarization-projection", "result": "REJECTED", "reason": assert_rejected(lambda: validate_platform_row(bad), "signing/notarization")})

    bad_layout = dict(layout)
    bad_layout["mutable_config_inside_immutable_package"] = True
    matrix.append({"case": "reject-config-inside-package", "result": "REJECTED", "reason": assert_rejected(lambda: validate_layout(bad_layout), "mutable config")})

    bad_layout = dict(layout)
    bad_layout["mutable_data_inside_immutable_package"] = True
    matrix.append({"case": "reject-data-inside-package", "result": "REJECTED", "reason": assert_rejected(lambda: validate_layout(bad_layout), "mutable data")})

    bad_layout = dict(layout)
    bad_layout["exact_linux_install_root_frozen"] = True
    matrix.append({"case": "reject-premature-install-root-freeze", "result": "REJECTED", "reason": assert_rejected(lambda: validate_layout(bad_layout), "install roots")})

    bad_governance = dict(governance)
    bad_governance["canonical_task_definitions_frozen"] = True
    matrix.append({"case": "reject-fabricated-task-schema", "result": "REJECTED", "reason": assert_rejected(lambda: validate_governance_projection(bad_governance), "task definitions")})

    bad_governance = dict(governance)
    bad_governance["canonical_task_ids_assigned"] = True
    matrix.append({"case": "reject-fabricated-task-ids", "result": "REJECTED", "reason": assert_rejected(lambda: validate_governance_projection(bad_governance), "task IDs")})

    bad_governance = dict(governance)
    bad_governance["pkg05_activated"] = True
    matrix.append({"case": "reject-pkg05-activation", "result": "REJECTED", "reason": assert_rejected(lambda: validate_governance_projection(bad_governance), "activate PKG-05")})

    bad_governance = dict(governance)
    bad_governance["pkg05_completion_projected"] = True
    matrix.append({"case": "reject-pkg05-completion", "result": "REJECTED", "reason": assert_rejected(lambda: validate_governance_projection(bad_governance), "completion")})

    contract = {
        "schema_version": 1,
        "mode": MODE,
        "package_id": PACKAGE_ID,
        "scope": "PKG-05-DORMANT-PLATFORM-LAYOUT",
        "canonical_blocker": CANONICAL_BLOCKER,
        "canonical_task_definitions_frozen": False,
        "canonical_task_ids_assigned": False,
        "future_canonical_task_reconciliation_required": True,
        "bundle_targets_all_is_capability_intent_not_support_evidence": True,
        "candidate_platforms": platforms,
        "layout": layout,
        "package_formats": "UNRESOLVED_UNTIL_CANONICAL_RECONCILIATION",
        "linux_distribution_policy": "UNRESOLVED_UNTIL_CANONICAL_RECONCILIATION",
        "macos_signing_notarization_policy": "UNRESOLVED_UNTIL_GENUINE_PRODUCTION_EVIDENCE",
        "production_release_workflow_claimed": False,
        "production_support_claimed": False,
    }

    report = {
        "schema_version": 1,
        "package_id": PACKAGE_ID,
        "mode": MODE,
        "scope": "PKG-05-DORMANT-PLATFORM-LAYOUT",
        "source_commit": source_commit,
        "canonical_blocker": CANONICAL_BLOCKER,
        "canonical_tasks_active": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "production_evidence_consumed": False,
        "production_support_claimed": False,
        "release_packages_built": False,
        "network_access_performed": False,
        "product_code_mutated": False,
        "fixture_identity": True,
        "canonical_task_definitions_frozen": False,
        "canonical_task_ids_assigned": False,
        "future_canonical_task_reconciliation_required": True,
        "source_baseline": baseline,
        "candidate_platform_count": len(platforms),
        "source_backed_mutable_boundaries_frozen": True,
        "platform_install_roots_frozen": False,
        "package_formats_frozen": False,
        "test_case_count": len(matrix),
        "negative_rejections": sum(row["result"] == "REJECTED" for row in matrix),
        "result": "PASS",
    }
    return report, contract, matrix


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true", required=True)
    parser.add_argument("--repo-root", default=".")
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--output-dir", required=True)
    args = parser.parse_args()

    report, contract, matrix = run(Path(args.repo_root), args.source_commit)
    out = Path(args.output_dir)
    out.mkdir(parents=True, exist_ok=True)
    (out / "report.json").write_bytes(canonical(report))
    (out / "contract.json").write_bytes(canonical(contract))
    (out / "matrix.json").write_bytes(canonical(matrix))
    print(json.dumps(report, sort_keys=True))
    print("PKG05_PLATFORM_LAYOUT_PREIMPLEMENTATION=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
