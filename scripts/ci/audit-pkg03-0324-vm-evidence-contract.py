#!/usr/bin/env python3
"""Synthetic audit for the future PKG-03 03.23 -> 03.24 VM evidence contract.

No real package, signing, SBOM/provenance or VM evidence is accepted as input.
This is deliberately non-authoritative while 03.24 is dependency-blocked.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
from pathlib import Path

HEX40 = re.compile(r"^[0-9a-f]{40}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")
PACKAGE_NAMES = (
    "nsis-current-user.exe",
    "nsis-per-machine.exe",
    "vsn-platform.msi",
    "VSN Dev Platform.exe",
)
REQUIRED_SCENARIOS = {
    "fresh-install",
    "repair-tamper",
    "preserved-user-data-uninstall",
    "running-resource",
    "pending-reboot",
}
ALLOWED_INSTALLERS = {"nsis-current-user", "nsis-per-machine", "msi"}
ALLOWED_ACTIONS = {"install", "repair", "uninstall"}
ALLOWED_SCOPES = {"current-user", "per-machine"}


class ContractError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ContractError(message)


def sha(label: str) -> str:
    return hashlib.sha256(label.encode("utf-8")).hexdigest()


def validate(upstream: dict, matrix: dict) -> None:
    require(upstream.get("schema_version") == 1, "03.23 schema mismatch")
    require(upstream.get("task_id") == "03.23", "upstream task must be 03.23")
    require(upstream.get("accepted") is True, "03.23 acceptance is required")
    source = upstream.get("source_commit")
    require(isinstance(source, str) and HEX40.fullmatch(source) is not None, "invalid source SHA")
    handoff_sha = upstream.get("handoff_sha256")
    require(isinstance(handoff_sha, str) and HEX64.fullmatch(handoff_sha) is not None, "invalid 03.23 handoff digest")

    packages = upstream.get("packages")
    require(isinstance(packages, list) and len(packages) == len(PACKAGE_NAMES), "03.23 package set mismatch")
    by_name: dict[str, dict] = {}
    for package in packages:
        require(isinstance(package, dict), "03.23 package row must be an object")
        name = package.get("file_name")
        require(name in PACKAGE_NAMES, f"unexpected 03.23 package: {name}")
        require(name not in by_name, f"duplicate 03.23 package: {name}")
        digest = package.get("sha256")
        require(isinstance(digest, str) and HEX64.fullmatch(digest) is not None, f"invalid package digest for {name}")
        require(package.get("signature_verified") is True, f"03.23 package signature not verified: {name}")
        require(package.get("provenance_verified") is True, f"03.23 provenance not verified: {name}")
        by_name[name] = package
    require(set(by_name) == set(PACKAGE_NAMES), "03.23 package names incomplete")

    require(matrix.get("schema_version") == 1, "03.24 matrix schema mismatch")
    require(matrix.get("task_id") == "03.24", "matrix task must be 03.24")
    require(matrix.get("upstream_task_id") == "03.23", "matrix upstream task mismatch")
    require(matrix.get("synthetic_audit_only") is True, "synthetic audit marker is required")
    require(matrix.get("source_commit") == source, "03.24 source differs from accepted 03.23 source")
    require(matrix.get("upstream_handoff_sha256") == handoff_sha, "03.24 handoff digest differs from accepted 03.23 handoff")

    rows = matrix.get("rows")
    require(isinstance(rows, list) and rows, "03.24 matrix rows are required")
    ids: set[str] = set()
    scenarios: set[str] = set()
    installers: set[str] = set()

    for row in rows:
        require(isinstance(row, dict), "matrix row must be an object")
        row_id = row.get("row_id")
        require(isinstance(row_id, str) and row_id.strip(), "matrix row_id missing")
        require(row_id not in ids, f"duplicate matrix row_id: {row_id}")
        ids.add(row_id)

        scenario = row.get("scenario")
        require(scenario in REQUIRED_SCENARIOS, f"unsupported or unfrozen scenario: {scenario}")
        scenarios.add(scenario)

        installer = row.get("installer")
        require(installer in ALLOWED_INSTALLERS, f"invalid installer family: {installer}")
        installers.add(installer)
        require(row.get("action") in ALLOWED_ACTIONS, f"invalid action in {row_id}")
        require(row.get("scope") in ALLOWED_SCOPES, f"invalid scope in {row_id}")

        package_name = row.get("package_file")
        require(package_name in by_name, f"row {row_id} references unknown package")
        require(row.get("package_sha256") == by_name[package_name]["sha256"], f"row {row_id} package digest mismatch")
        require(row.get("signature_verified_before_execution") is True, f"row {row_id} lacks signature verification")

        image = row.get("windows_image")
        require(isinstance(image, str) and image.strip(), f"row {row_id} lacks Windows image identity")
        seed = row.get("seed_sha256")
        require(isinstance(seed, str) and HEX64.fullmatch(seed) is not None, f"row {row_id} lacks deterministic seed digest")
        require(isinstance(row.get("command_line"), str) and row["command_line"].strip(), f"row {row_id} command line missing")
        require(isinstance(row.get("native_exit_code"), int), f"row {row_id} native exit code missing")
        require(row.get("result") == "PASS", f"row {row_id} is not PASS")
        require(row.get("cleanup_verified") is True, f"row {row_id} cleanup not verified")
        require(row.get("forbidden_system_mutation_zero") is True, f"row {row_id} forbidden system mutation not zero")

        real_reboot = row.get("real_reboot") is True
        provider = row.get("runner_provider")
        require(isinstance(provider, str) and provider.strip(), f"row {row_id} runner provider missing")
        if real_reboot:
            require(provider != "github-hosted", f"row {row_id} falsely claims reboot persistence on ordinary GitHub-hosted runner")
            before = row.get("machine_id_before")
            after = row.get("machine_id_after")
            require(isinstance(before, str) and before.strip(), f"row {row_id} pre-reboot machine identity missing")
            require(before == after, f"row {row_id} same-machine continuity failed")
            pre = row.get("pre_boot_marker")
            post = row.get("post_boot_marker")
            require(isinstance(pre, str) and pre.strip(), f"row {row_id} pre-boot marker missing")
            require(isinstance(post, str) and post.strip(), f"row {row_id} post-boot marker missing")
            require(pre != post, f"row {row_id} reboot markers do not demonstrate a boot boundary")

    require(REQUIRED_SCENARIOS.issubset(scenarios), f"required scenarios missing: {sorted(REQUIRED_SCENARIOS - scenarios)}")
    require(ALLOWED_INSTALLERS.issubset(installers), f"installer family coverage missing: {sorted(ALLOWED_INSTALLERS - installers)}")


def make_fixture() -> tuple[dict, dict]:
    source = "3" * 40
    packages = [
        {"file_name": name, "sha256": sha(f"signed:{name}"), "signature_verified": True, "provenance_verified": True}
        for name in PACKAGE_NAMES
    ]
    upstream = {
        "schema_version": 1,
        "task_id": "03.23",
        "accepted": True,
        "source_commit": source,
        "handoff_sha256": sha("03.23-handoff"),
        "packages": packages,
    }
    digest = {p["file_name"]: p["sha256"] for p in packages}
    common = {
        "source_commit": source,
        "signature_verified_before_execution": True,
        "windows_image": "Windows-Server-2025-audit-image",
        "command_line": "synthetic-command",
        "native_exit_code": 0,
        "result": "PASS",
        "cleanup_verified": True,
        "forbidden_system_mutation_zero": True,
        "runner_provider": "github-hosted",
        "real_reboot": False,
    }
    rows = [
        {**common, "row_id": "fresh-cu", "scenario": "fresh-install", "installer": "nsis-current-user", "scope": "current-user", "action": "install", "package_file": "nsis-current-user.exe", "package_sha256": digest["nsis-current-user.exe"], "seed_sha256": sha("fresh-cu")},
        {**common, "row_id": "fresh-pm", "scenario": "fresh-install", "installer": "nsis-per-machine", "scope": "per-machine", "action": "install", "package_file": "nsis-per-machine.exe", "package_sha256": digest["nsis-per-machine.exe"], "seed_sha256": sha("fresh-pm")},
        {**common, "row_id": "repair-msi", "scenario": "repair-tamper", "installer": "msi", "scope": "per-machine", "action": "repair", "package_file": "vsn-platform.msi", "package_sha256": digest["vsn-platform.msi"], "seed_sha256": sha("repair-msi")},
        {**common, "row_id": "userdata-uninstall", "scenario": "preserved-user-data-uninstall", "installer": "nsis-current-user", "scope": "current-user", "action": "uninstall", "package_file": "nsis-current-user.exe", "package_sha256": digest["nsis-current-user.exe"], "seed_sha256": sha("userdata-uninstall")},
        {**common, "row_id": "running-resource", "scenario": "running-resource", "installer": "nsis-per-machine", "scope": "per-machine", "action": "repair", "package_file": "nsis-per-machine.exe", "package_sha256": digest["nsis-per-machine.exe"], "seed_sha256": sha("running-resource")},
        {**common, "row_id": "pending-reboot", "scenario": "pending-reboot", "installer": "msi", "scope": "per-machine", "action": "repair", "package_file": "vsn-platform.msi", "package_sha256": digest["vsn-platform.msi"], "seed_sha256": sha("pending-reboot"), "runner_provider": "persistent-vm-audit", "real_reboot": True, "machine_id_before": "vm-0324-001", "machine_id_after": "vm-0324-001", "pre_boot_marker": "boot-A", "post_boot_marker": "boot-B"},
    ]
    matrix = {
        "schema_version": 1,
        "task_id": "03.24",
        "upstream_task_id": "03.23",
        "synthetic_audit_only": True,
        "source_commit": source,
        "upstream_handoff_sha256": upstream["handoff_sha256"],
        "rows": rows,
    }
    return upstream, matrix


def self_test() -> dict:
    upstream, matrix = make_fixture()
    cases: list[dict] = []

    def accept(name: str) -> None:
        validate(copy.deepcopy(upstream), copy.deepcopy(matrix))
        cases.append({"name": name, "expected": "PASS", "result": "PASS"})

    def reject(name: str, mutate) -> None:
        u = copy.deepcopy(upstream)
        m = copy.deepcopy(matrix)
        mutate(u, m)
        try:
            validate(u, m)
        except ContractError as exc:
            cases.append({"name": name, "expected": "REJECT", "result": "REJECT", "reason": str(exc)})
            return
        raise AssertionError(f"negative fixture unexpectedly passed: {name}")

    accept("candidate-bound-complete-matrix")
    reject("reject-unaccepted-0323", lambda u, m: u.__setitem__("accepted", False))
    reject("reject-handoff-digest-mismatch", lambda u, m: m.__setitem__("upstream_handoff_sha256", sha("other-handoff")))
    reject("reject-package-digest-mismatch", lambda u, m: m["rows"][0].__setitem__("package_sha256", sha("rebuilt")))
    reject("reject-signature-not-verified", lambda u, m: m["rows"][0].__setitem__("signature_verified_before_execution", False))
    reject("reject-missing-seed-digest", lambda u, m: m["rows"][1].__setitem__("seed_sha256", ""))
    reject("reject-missing-required-scenario", lambda u, m: m.__setitem__("rows", [r for r in m["rows"] if r["scenario"] != "running-resource"]))
    reject("reject-github-hosted-real-reboot", lambda u, m: m["rows"][-1].__setitem__("runner_provider", "github-hosted"))
    reject("reject-cross-machine-reboot", lambda u, m: m["rows"][-1].__setitem__("machine_id_after", "vm-0324-002"))
    reject("reject-no-boot-boundary", lambda u, m: m["rows"][-1].__setitem__("post_boot_marker", "boot-A"))
    reject("reject-cleanup-failure", lambda u, m: m["rows"][2].__setitem__("cleanup_verified", False))
    reject("reject-forbidden-system-mutation", lambda u, m: m["rows"][3].__setitem__("forbidden_system_mutation_zero", False))
    reject("reject-missing-installer-family", lambda u, m: m.__setitem__("rows", [r for r in m["rows"] if r["installer"] != "msi"]))

    require(len(cases) == 13, "self-test case count drift")
    require(sum(c["result"] == "PASS" for c in cases) == 1, "positive case count mismatch")
    require(sum(c["result"] == "REJECT" for c in cases) == 12, "negative case count mismatch")
    return {
        "schema_version": 1,
        "audit": "PKG-03 03.23 -> 03.24 VM evidence contract",
        "mode": "synthetic-only",
        "real_vm_executed": False,
        "production_evidence_consumed": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "summary": {"pass": 1, "negative_rejections": 12, "total": 13},
        "cases": cases,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    report = self_test()
    out = Path(args.output)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report["summary"], sort_keys=True))
    print("AUDIT_ONLY_0324_VM_EVIDENCE_CONTRACT=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
