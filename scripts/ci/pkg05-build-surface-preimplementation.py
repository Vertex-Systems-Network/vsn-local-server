#!/usr/bin/env python3
"""Validate dormant PKG-05 Linux/macOS build-surface evidence.

This is NON_ACCEPTANCE_PREIMPLEMENTATION only. It must never activate PKG-05,
assign canonical 05.xx tasks, or project release/package support from hosted
runner compile evidence.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import tempfile
from pathlib import Path
from typing import Any

MODE = "NON_ACCEPTANCE_PREIMPLEMENTATION"
SCOPE = "PKG-05-DORMANT-BUILD-SURFACE"
BLOCKER = "PKG-04:COMPLETE"
EXPECTED_RUST_PACKAGES = {
    "vsn-agent": "apps/agent/Cargo.toml",
    "vsn": "apps/cli/Cargo.toml",
    "vsn-updater-helper": "apps/updater-helper/Cargo.toml",
}
EXPECTED_WORKSPACE_MEMBERS = {
    "apps/agent",
    "apps/cli",
    "apps/updater-helper",
    "apps/desktop/src-tauri",
}
EXPECTED_ROWS = {
    ("Linux", "X64"),
    ("macOS", "X64"),
    ("macOS", "ARM64"),
}
HEX64 = re.compile(r"^[0-9a-f]{64}$")
CANONICAL_TASK_ID = re.compile(r"^05\.\d{2}$")


class ContractError(AssertionError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ContractError(message)


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def package_name_from_cargo(path: Path) -> str:
    text = path.read_text(encoding="utf-8")
    package = re.search(r"(?ms)^\[package\]\s+.*?^name\s*=\s*\"([^\"]+)\"", text)
    require(package is not None, f"missing [package] name in {path}")
    return package.group(1)


def source_contract(repo: Path) -> dict[str, Any]:
    tracker = read_json(repo / "docs/MASTER-EXECUTION-STATUS.json")
    require(tracker["active_package"] == "PKG-03", "canonical active package changed")
    require(tracker["active_task"] == "03.22", "canonical active task changed")
    pkg05 = next((p for p in tracker["packages"] if p["id"] == "PKG-05"), None)
    require(pkg05 is not None, "PKG-05 missing from canonical tracker")
    require(pkg05["name"] == "Linux + macOS Release", "PKG-05 package name changed")
    require(pkg05["done"] == 0, "PKG-05 must remain dormant")
    require(pkg05["required"] == 23, "PKG-05 required count changed")
    require(pkg05["percent"] == 0.0, "PKG-05 progress must remain zero")
    require(pkg05["status"] == "NOT_STARTED", "PKG-05 must remain NOT_STARTED")

    root_cargo = (repo / "Cargo.toml").read_text(encoding="utf-8")
    for member in EXPECTED_WORKSPACE_MEMBERS:
        require(f'"{member}"' in root_cargo, f"workspace member missing: {member}")

    package_names = {}
    for expected, rel in EXPECTED_RUST_PACKAGES.items():
        actual = package_name_from_cargo(repo / rel)
        require(actual == expected, f"unexpected Cargo package name for {rel}: {actual}")
        package_names[rel] = actual

    desktop = read_json(repo / "apps/desktop/package.json")
    require(desktop["name"] == "vsn-desktop", "desktop npm package name changed")
    require(desktop["scripts"].get("build") == "tsc && vite build", "desktop production build command changed")
    require(desktop["scripts"].get("tauri") == "tauri", "desktop Tauri command changed")
    require((repo / "apps/desktop/package-lock.json").exists(), "desktop package-lock.json missing")
    require((repo / "Cargo.lock").exists(), "Cargo.lock missing")

    tauri = read_json(repo / "apps/desktop/src-tauri/tauri.conf.json")
    require(tauri["build"].get("beforeBuildCommand") == "npm run build", "Tauri beforeBuildCommand changed")
    require(tauri["bundle"].get("targets") == "all", "Tauri bundle targets changed")
    require(tauri.get("mainBinaryName") == "VSN Dev Platform", "Tauri mainBinaryName changed")
    require(package_name_from_cargo(repo / "apps/desktop/src-tauri/Cargo.toml") == "vsn-desktop", "desktop Cargo package name changed")

    return {
        "active_package": tracker["active_package"],
        "active_task": tracker["active_task"],
        "pkg05_done": pkg05["done"],
        "pkg05_required": pkg05["required"],
        "pkg05_status": pkg05["status"],
        "rust_packages": sorted(EXPECTED_RUST_PACKAGES),
        "desktop_npm_package": desktop["name"],
        "desktop_build_command": desktop["scripts"]["build"],
        "tauri_main_binary_name": tauri["mainBinaryName"],
        "tauri_bundle_targets": tauri["bundle"]["targets"],
        "tauri_targets_all_is_production_support_evidence": False,
        "cargo_lock_present": True,
        "npm_lock_present": True,
    }


def validate_artifact(name: str, artifact: Any) -> None:
    require(isinstance(artifact, dict), f"{name} artifact must be an object")
    require(artifact.get("exists") is True, f"{name} artifact missing")
    require(isinstance(artifact.get("path"), str) and artifact["path"], f"{name} artifact path missing")
    require(isinstance(artifact.get("size"), int) and artifact["size"] > 0, f"{name} artifact size invalid")
    require(isinstance(artifact.get("sha256"), str) and HEX64.fullmatch(artifact["sha256"]), f"{name} artifact sha256 invalid")


def validate_runtime(evidence: dict[str, Any], expected_source: str) -> None:
    require(evidence.get("schema_version") == 1, "runtime evidence schema mismatch")
    require(evidence.get("mode") == MODE, "runtime evidence mode mismatch")
    require(evidence.get("scope") == SCOPE, "runtime evidence scope mismatch")
    require(evidence.get("source_commit") == expected_source, "runtime source commit mismatch")
    require((evidence.get("runner_os"), evidence.get("runner_arch")) in EXPECTED_ROWS, "unsupported runner row")
    require(evidence.get("cargo_locked") is True, "Cargo build must use --locked")
    require(evidence.get("npm_ci") is True, "Desktop dependencies must use npm ci")
    require(evidence.get("desktop_frontend_build") is True, "Desktop frontend production build did not pass")
    require(evidence.get("tauri_no_bundle_build") is True, "Tauri --no-bundle build did not pass")
    require(evidence.get("architecture_verified") is True, "built executable architecture not verified")

    rust = evidence.get("rust_release_artifacts")
    require(isinstance(rust, dict), "rust_release_artifacts missing")
    require(set(rust) == set(EXPECTED_RUST_PACKAGES), "Rust release artifact set mismatch")
    for name in EXPECTED_RUST_PACKAGES:
        validate_artifact(name, rust[name])
    validate_artifact("tauri_native", evidence.get("tauri_native_artifact"))

    forbidden_true = (
        "package_format_selected",
        "distribution_package_built",
        "package_signed",
        "notarization_performed",
        "installer_tested",
        "update_tested",
        "uninstall_tested",
        "persistent_reboot_tested",
        "production_support_claimed",
        "production_evidence",
        "canonical_tasks_active",
        "canonical_state_changed",
        "implementation_authority",
        "production_evidence_consumed",
        "canonical_task_ids_assigned",
    )
    for field in forbidden_true:
        require(evidence.get(field) is False, f"{field} must remain false in dormant build evidence")
    require(evidence.get("future_canonical_task_reconciliation_required") is True, "future canonical reconciliation must remain required")

    task_id = evidence.get("task_id")
    if isinstance(task_id, str):
        require(not CANONICAL_TASK_ID.fullmatch(task_id), "dormant evidence must not assign canonical 05.xx task IDs")


def sample_evidence(source: str) -> dict[str, Any]:
    def art(path: str, seed: str) -> dict[str, Any]:
        return {
            "exists": True,
            "path": path,
            "size": 123,
            "sha256": hashlib.sha256(seed.encode()).hexdigest(),
        }

    return {
        "schema_version": 1,
        "mode": MODE,
        "scope": SCOPE,
        "task_id": "PKG-05-DORMANT-PREIMPLEMENTATION",
        "source_commit": source,
        "runner_os": "Linux",
        "runner_arch": "X64",
        "cargo_locked": True,
        "npm_ci": True,
        "desktop_frontend_build": True,
        "tauri_no_bundle_build": True,
        "architecture_verified": True,
        "rust_release_artifacts": {
            "vsn-agent": art("target/release/vsn-agent", "agent"),
            "vsn": art("target/release/vsn", "cli"),
            "vsn-updater-helper": art("target/release/vsn-updater-helper", "helper"),
        },
        "tauri_native_artifact": art("target/release/VSN Dev Platform", "desktop"),
        "package_format_selected": False,
        "distribution_package_built": False,
        "package_signed": False,
        "notarization_performed": False,
        "installer_tested": False,
        "update_tested": False,
        "uninstall_tested": False,
        "persistent_reboot_tested": False,
        "production_support_claimed": False,
        "production_evidence": False,
        "canonical_tasks_active": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "production_evidence_consumed": False,
        "canonical_task_ids_assigned": False,
        "future_canonical_task_reconciliation_required": True,
    }


def self_test() -> int:
    source = "a" * 40
    baseline = sample_evidence(source)
    validate_runtime(baseline, source)
    mutations = [
        ("wrong source", lambda e: e.__setitem__("source_commit", "b" * 40)),
        ("production claim", lambda e: e.__setitem__("production_support_claimed", True)),
        ("package built", lambda e: e.__setitem__("distribution_package_built", True)),
        ("package signed", lambda e: e.__setitem__("package_signed", True)),
        ("canonical id", lambda e: e.__setitem__("task_id", "05.01")),
        ("missing hash", lambda e: e["rust_release_artifacts"]["vsn"]["sha256"] == e["rust_release_artifacts"]["vsn"].pop("sha256")),
        ("bad architecture", lambda e: e.__setitem__("architecture_verified", False)),
        ("no npm ci", lambda e: e.__setitem__("npm_ci", False)),
        ("no tauri build", lambda e: e.__setitem__("tauri_no_bundle_build", False)),
        ("canonical active", lambda e: e.__setitem__("canonical_tasks_active", True)),
    ]
    rejected = 0
    for label, mutate in mutations:
        candidate = copy.deepcopy(baseline)
        mutate(candidate)
        try:
            validate_runtime(candidate, source)
        except (ContractError, KeyError):
            rejected += 1
        else:
            raise AssertionError(f"negative self-test unexpectedly accepted: {label}")
    require(rejected == len(mutations), "not all negative self-tests rejected")
    print(json.dumps({"pass": 1, "negative_rejections": rejected, "total": rejected + 1}, sort_keys=True))
    print("PKG05_BUILD_SURFACE_SELF_TEST=PASS")
    return rejected


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", default=".")
    parser.add_argument("--source-commit")
    parser.add_argument("--runtime-evidence")
    parser.add_argument("--output-dir")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        self_test()
        if not args.runtime_evidence:
            return 0

    require(args.source_commit is not None and re.fullmatch(r"[0-9a-f]{40}", args.source_commit), "--source-commit must be a 40-char lowercase SHA")
    require(args.runtime_evidence is not None, "--runtime-evidence required")
    require(args.output_dir is not None, "--output-dir required")

    repo = Path(args.repo_root).resolve()
    source = source_contract(repo)
    evidence = read_json(Path(args.runtime_evidence))
    validate_runtime(evidence, args.source_commit)

    out = Path(args.output_dir)
    out.mkdir(parents=True, exist_ok=True)
    contract = {
        "schema_version": 1,
        "mode": MODE,
        "scope": SCOPE,
        "package_id": "PKG-05",
        "canonical_blocker": BLOCKER,
        "canonical_task_ids_assigned": False,
        "future_canonical_task_reconciliation_required": True,
        "candidate_rows": [
            {"runner_os": "Linux", "runner_arch": "X64"},
            {"runner_os": "macOS", "runner_arch": "X64"},
            {"runner_os": "macOS", "runner_arch": "ARM64"},
        ],
        "build_surfaces": ["vsn-agent", "vsn", "vsn-updater-helper", "desktop-frontend", "tauri-native-no-bundle"],
        "package_format_selected": False,
        "distribution_package_built": False,
        "signing_or_notarization_acceptance": False,
        "install_uninstall_update_reboot_acceptance": False,
        "hosted_compile_evidence_is_release_acceptance": False,
        "production_support_claimed": False,
    }
    report = {
        "schema_version": 1,
        "mode": MODE,
        "scope": SCOPE,
        "package_id": "PKG-05",
        "canonical_blocker": BLOCKER,
        "source_commit": args.source_commit,
        "runner_os": evidence["runner_os"],
        "runner_arch": evidence["runner_arch"],
        "source_baseline": source,
        "cargo_locked": True,
        "npm_ci": True,
        "rust_release_artifact_count": len(evidence["rust_release_artifacts"]),
        "desktop_frontend_build": True,
        "tauri_no_bundle_build": True,
        "architecture_verified": True,
        "package_format_selected": False,
        "distribution_package_built": False,
        "package_signed": False,
        "notarization_performed": False,
        "production_support_claimed": False,
        "canonical_tasks_active": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "production_evidence_consumed": False,
        "canonical_task_ids_assigned": False,
        "future_canonical_task_reconciliation_required": True,
        "result": "PASS",
    }
    (out / "contract.json").write_text(json.dumps(contract, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    (out / "report.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, sort_keys=True))
    print("PKG05_BUILD_SURFACE_PREIMPLEMENTATION=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
