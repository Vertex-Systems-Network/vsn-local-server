#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

TASK = "03.17"
LINEAR = "ABD-92"
BASE = "f3afb66e588d01ff2e8cb37273ad413862a4edaf"
MANIFEST_PATH = Path(".ai/manifests/pkg03-0317-uninstall-cleanup.v1.json")
TRACKER_PATH = "certification/pkg03-windows-installer-v1.json"
PROJECTION_PATHS = {
    "certification/pkg03-windows-installer-v1.json",
    "docs/MASTER-EXECUTION-STATUS.json",
    "README.md",
    ".ai/README.md",
    "docs/MASTER-EXECUTION-PLAN.md",
}
PLANNING_PATHS = {
    ".ai/features/pkg03-0317/research.md",
    ".ai/features/pkg03-0317/lifecycle-review.md",
    ".ai/features/pkg03-0317/development-preflight.md",
    ".ai/plans/pkg03-0317-uninstall-cleanup-v1.md",
    ".ai/manifests/pkg03-0317-uninstall-cleanup.v1.json",
    "docs/PKG03-INSTALLER-UNINSTALL-CLEANUP-PRESERVATION-V1.md",
}
EXPECTED_SHA256 = {
    ".ai/features/pkg03-0317/research.md": "fbe054325caa88a9e69c8ac654e625d98a9999be0887ecf373497c7dd0d56ce2",
    ".ai/features/pkg03-0317/lifecycle-review.md": "90d00748d386a57c378d70a6df842210077693544792219e6791c32e5ae47022",
    ".ai/features/pkg03-0317/development-preflight.md": "a81dfb5491a1933cb1137c8c400187e64f150d73c36ef4d624802228ad8c7595",
    ".ai/plans/pkg03-0317-uninstall-cleanup-v1.md": "71727895275c2e91e9e5eb78ada563ca6978f923e03bc7be52e5b415f8da5d8b",
    "docs/PKG03-INSTALLER-UNINSTALL-CLEANUP-PRESERVATION-V1.md": "46a5f31f354ddbb56e3ea9a065fb83c17e5659d39a26261f7d63f1ccf4654d7a",
    ".ai/plans/pkg03-windows-installer-v1.md": "9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e",
}


def fail(message: str) -> None:
    raise SystemExit(f"03.17 authority validation failed: {message}")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True).strip()


def base_json(path: str) -> dict:
    raw = subprocess.check_output(["git", "show", f"{BASE}:{path}"], text=True)
    return json.loads(raw)


def main() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    if manifest.get("task_id") != TASK or manifest.get("linear_issue") != LINEAR:
        fail("manifest task/Linear identity mismatch")
    if manifest.get("canonical_base_sha") != BASE:
        fail("canonical activation base changed")
    if manifest.get("status") != "frozen":
        fail("task plan is not frozen")

    research = manifest.get("research", {})
    if research.get("change_required") is not False:
        fail("planning did not conclude certification-first")
    authority = manifest.get("authority", {})
    required_false = (
        "planning_product_mutation_allowed",
        "tauri_config_mutation_allowed",
        "installer_template_or_hook_mutation_allowed",
        "recursive_data_tree_deletion_allowed",
        "package_identity_mutation_allowed",
        "product_payload_source_mutation_allowed",
        "service_identity_mutation_allowed",
        "acl_mutation_allowed",
        "firewall_hosts_dns_trust_mutation_allowed",
        "running_process_coordination_allowed",
        "rollback_or_recovery_claim_allowed",
        "reboot_semantics_claim_allowed",
        "silent_or_passive_deployment_allowed",
        "signing_secret_access_allowed",
        "updater_mutation_allowed",
        "delegated_scope_may_expand",
    )
    for key in required_false:
        if authority.get(key) is not False:
            fail(f"authority unexpectedly widened: {key}")

    for path, expected in EXPECTED_SHA256.items():
        actual = sha256(Path(path))
        if actual != expected:
            fail(f"frozen digest mismatch for {path}: {actual}")

    if manifest.get("parent_plan", {}).get("sha256") != EXPECTED_SHA256[".ai/plans/pkg03-windows-installer-v1.md"]:
        fail("manifest parent plan digest mismatch")
    for key, path in (
        ("research", ".ai/features/pkg03-0317/research.md"),
        ("lifecycle", ".ai/features/pkg03-0317/lifecycle-review.md"),
        ("development_preflight", ".ai/features/pkg03-0317/development-preflight.md"),
        ("task_plan", ".ai/plans/pkg03-0317-uninstall-cleanup-v1.md"),
        ("lifecycle_contract", "docs/PKG03-INSTALLER-UNINSTALL-CLEANUP-PRESERVATION-V1.md"),
    ):
        if manifest.get(key, {}).get("sha256") != EXPECTED_SHA256[path]:
            fail(f"manifest digest binding mismatch: {key}")

    tracker = base_json(TRACKER_PATH)
    if tracker.get("package_id") != "PKG-03" or tracker.get("done") != 15 or tracker.get("required") != 25:
        fail("canonical package projection is not the 15/25 activation baseline")
    task_by_id = {item.get("id"): item for item in tracker.get("tasks", [])}
    task = task_by_id.get(TASK)
    if not task or task.get("status") != "READY":
        fail("03.17 is not READY on canonical activation base")
    expected_deps = ["03.11", "03.12", "03.13"]
    if task.get("depends_on") != expected_deps:
        fail("03.17 dependency contract drifted")
    for dep in expected_deps:
        if task_by_id.get(dep, {}).get("status") != "DONE":
            fail(f"dependency {dep} is not canonically DONE")

    changed = [line for line in git("diff", "--name-only", f"{BASE}...HEAD").splitlines() if line]
    unexpected: list[str] = []
    for path in changed:
        allowed = (
            path in PLANNING_PATHS
            or path.startswith("scripts/ci/pkg03-0317-")
            or path.startswith(".github/workflows/pkg03-0317-")
        )
        if not allowed:
            unexpected.append(path)
    if unexpected:
        fail(f"branch changed unauthorized paths: {unexpected}")
    if any(path in PROJECTION_PATHS for path in changed):
        fail("canonical projection is forbidden before genuine 03.17 acceptance")
    if any(path.startswith(("apps/", "crates/", "installer/")) for path in changed):
        fail("product/installer mutation appeared before change control")

    print(json.dumps({
        "valid": True,
        "task": TASK,
        "linear": LINEAR,
        "state": task.get("status"),
        "canonical_base": BASE,
        "canonical_progress": {"done": tracker.get("done"), "required": tracker.get("required"), "percent": tracker.get("percent")},
        "dependencies": {dep: task_by_id[dep].get("status") for dep in expected_deps},
        "changed_paths": changed,
        "planning_product_mutation_allowed": False,
        "certification_first": True,
    }, indent=2))


if __name__ == "__main__":
    main()
