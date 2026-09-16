#!/usr/bin/env python3
"""Synthetic-only deterministic VM harness preimplementation for PKG-03 03.24."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

FILES = {
    "nsis-current-user.exe": "nsis-current-user",
    "nsis-per-machine.exe": "nsis-per-machine",
    "vsn-platform.msi": "msi",
}
SCENARIOS = (
    ("fresh-cu", "fresh-install", "nsis-current-user.exe", "current-user", "install", False),
    ("fresh-pm", "fresh-install", "nsis-per-machine.exe", "per-machine", "install", False),
    ("repair-msi", "repair-tamper", "vsn-platform.msi", "per-machine", "repair", False),
    ("userdata-uninstall", "preserved-user-data-uninstall", "nsis-current-user.exe", "current-user", "uninstall", False),
    ("running-resource", "running-resource", "nsis-per-machine.exe", "per-machine", "repair", False),
    ("pending-reboot", "pending-reboot", "vsn-platform.msi", "per-machine", "repair", True),
)


def sha(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def make_matrix() -> dict:
    rows = []
    for row_id, scenario, package, scope, action, reboot in SCENARIOS:
        row = {
            "row_id": row_id,
            "scenario": scenario,
            "installer": FILES[package],
            "package_file": package,
            "package_sha256": sha(f"fast-epoch1:signed:{package}"),
            "scope": scope,
            "action": action,
            "windows_image": "SYNTHETIC-WINDOWS-2025-FIXTURE",
            "seed_sha256": sha(f"seed:{row_id}"),
            "signature_verified_before_execution": True,
            "command_line": f"synthetic:{action}:{package}",
            "native_exit_code": 0,
            "result": "PASS",
            "cleanup_verified": True,
            "forbidden_system_mutation_zero": True,
            "real_vm_executed": False,
            "real_reboot": reboot,
            "runner_provider": "synthetic-persistent-vm" if reboot else "synthetic-hosted-runner",
        }
        if reboot:
            row.update({
                "machine_id_before": "synthetic-vm-0324",
                "machine_id_after": "synthetic-vm-0324",
                "pre_boot_marker": "fixture-boot-A",
                "post_boot_marker": "fixture-boot-B",
            })
        rows.append(row)
    return {
        "schema_version": 1,
        "task_id": "03.24",
        "mode": "NON_ACCEPTANCE_PREIMPLEMENTATION",
        "production_evidence_consumed": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "real_vm_executed": False,
        "source_commit": "6" * 40,
        "rows": rows,
    }


def validate(matrix: dict) -> None:
    assert matrix["mode"] == "NON_ACCEPTANCE_PREIMPLEMENTATION"
    assert matrix["production_evidence_consumed"] is False
    assert matrix["canonical_state_changed"] is False
    assert matrix["implementation_authority"] is False
    assert matrix["real_vm_executed"] is False
    rows = matrix["rows"]
    assert len(rows) == len(SCENARIOS)
    assert {r["scenario"] for r in rows} == {
        "fresh-install", "repair-tamper", "preserved-user-data-uninstall",
        "running-resource", "pending-reboot",
    }
    assert {r["installer"] for r in rows} == {"nsis-current-user", "nsis-per-machine", "msi"}
    assert len({r["row_id"] for r in rows}) == len(rows)
    for row in rows:
        assert len(row["package_sha256"]) == 64
        assert len(row["seed_sha256"]) == 64
        assert row["signature_verified_before_execution"] is True
        assert row["cleanup_verified"] is True
        assert row["forbidden_system_mutation_zero"] is True
        assert row["result"] == "PASS"
        if row["real_reboot"]:
            assert row["runner_provider"] not in {"github-hosted", "synthetic-hosted-runner"}
            assert row["machine_id_before"] == row["machine_id_after"]
            assert row["pre_boot_marker"] != row["post_boot_marker"]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true", required=True)
    parser.add_argument("--output-dir", required=True)
    args = parser.parse_args()
    matrix = make_matrix()
    validate(matrix)
    payload = canonical(matrix)
    out = Path(args.output_dir)
    out.mkdir(parents=True, exist_ok=True)
    (out / "synthetic-vm-matrix.json").write_bytes(payload)
    report = {
        "schema_version": 1,
        "task_id": "03.24",
        "mode": "NON_ACCEPTANCE_PREIMPLEMENTATION",
        "production_evidence_consumed": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "real_vm_executed": False,
        "row_count": len(matrix["rows"]),
        "matrix_sha256": hashlib.sha256(payload).hexdigest(),
        "deterministic_outputs": True,
    }
    (out / "report.json").write_bytes(canonical(report))
    print(json.dumps(report, sort_keys=True))
    print("PKG03_0324_VM_HARNESS_PREIMPLEMENTATION=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
