#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

TASK = "03.17"
LINEAR = "ABD-92"
ACTIVATION_BASE = "f3afb66e588d01ff2e8cb37273ad413862a4edaf"
CANONICAL_BASE = "5a582dbfdd445fb304a1d858263bb7722a95adf4"
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
VALIDATOR_PATH = "scripts/ci/validate-pkg03-0317.py"
EXPECTED_SHA256 = {
    ".ai/features/pkg03-0317/research.md": "fbe054325caa88a9e69c8ac654e625d98a9999be0887ecf373497c7dd0d56ce2",
    ".ai/features/pkg03-0317/lifecycle-review.md": "90d00748d386a57c378d70a6df842210077693544792219e6791c32e5ae47022",
    ".ai/features/pkg03-0317/development-preflight.md": "a81dfb5491a1933cb1137c8c400187e64f150d73c36ef4d624802228ad8c7595",
    ".ai/plans/pkg03-0317-uninstall-cleanup-v1.md": "71727895275c2e91e9e5eb78ada563ca6978f923e03bc7be52e5b415f8da5d8b",
    "docs/PKG03-INSTALLER-UNINSTALL-CLEANUP-PRESERVATION-V1.md": "46a5f31f354ddbb56e3ea9a065fb83c17e5659d39a26261f7d63f1ccf4654d7a",
    ".ai/plans/pkg03-windows-installer-v1.md": "9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e",
}
ACCEPTED_EVIDENCE = {
    "source_commit": "c6d847120ff9069e323660ac5833fdc3eaaf28c8",
    "workflow_run": 33446236695,
    "job": 99665862753,
    "artifact": 9778914521,
    "artifact_digest": "sha256:ab11840577f405bb1c6cdc62f160ab986e9a2d73945395dcb4d6eea3b1510dcd",
    "evidence_sha256": "528bc8c9c3d7a53cb41bb006ef57eedbc5b44bca95e351be058a1a1a86b623e0",
}


def fail(message: str) -> None:
    raise SystemExit(f"03.17 authority validation failed: {message}")


def sha256(path: Path) -> str:
    """Hash tracked Git blob bytes so checkout EOL conversion cannot fake drift."""
    relative = path.as_posix()
    try:
        blob = subprocess.check_output(["git", "show", f"HEAD:{relative}"])
    except subprocess.CalledProcessError as exc:
        fail(f"cannot read tracked frozen artifact from HEAD: {relative} ({exc.returncode})")
    return hashlib.sha256(blob).hexdigest()


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True).strip()


def ref_json(ref: str, path: str) -> dict:
    raw = subprocess.check_output(["git", "show", f"{ref}:{path}"], text=True)
    return json.loads(raw)


def task_map(tracker: dict) -> dict[str, dict]:
    return {item.get("id"): item for item in tracker.get("tasks", [])}


def main() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    if manifest.get("task_id") != TASK or manifest.get("linear_issue") != LINEAR:
        fail("manifest task/Linear identity mismatch")
    if manifest.get("canonical_base_sha") != ACTIVATION_BASE:
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

    activation_tracker = ref_json(ACTIVATION_BASE, TRACKER_PATH)
    if activation_tracker.get("package_id") != "PKG-03" or activation_tracker.get("done") != 15 or activation_tracker.get("required") != 25:
        fail("activation package projection is not the frozen 15/25 baseline")
    activation_tasks = task_map(activation_tracker)
    activation_task = activation_tasks.get(TASK)
    if not activation_task or activation_task.get("status") != "READY":
        fail("03.17 was not READY on canonical activation base")
    expected_deps = ["03.11", "03.12", "03.13"]
    if activation_task.get("depends_on") != expected_deps:
        fail("03.17 dependency contract drifted at activation")
    for dep in expected_deps:
        if activation_tasks.get(dep, {}).get("status") != "DONE":
            fail(f"dependency {dep} was not canonically DONE at activation")

    canonical_tracker = ref_json(CANONICAL_BASE, TRACKER_PATH)
    if canonical_tracker.get("package_id") != "PKG-03" or canonical_tracker.get("done") != 16 or canonical_tracker.get("required") != 25:
        fail("current canonical package projection is not 16/25")
    canonical_tasks = task_map(canonical_tracker)
    if canonical_tasks.get("03.16", {}).get("status") != "DONE":
        fail("03.16 is not DONE on current canonical base")
    canonical_task = canonical_tasks.get(TASK)
    if not canonical_task or canonical_task.get("status") != "READY":
        fail("03.17 is not READY on current canonical base")
    if canonical_task.get("depends_on") != expected_deps:
        fail("03.17 dependency contract drifted on current canonical base")
    for dep in expected_deps:
        if canonical_tasks.get(dep, {}).get("status") != "DONE":
            fail(f"dependency {dep} is not DONE on current canonical base")

    head_tracker = ref_json("HEAD", TRACKER_PATH)
    head_tasks = task_map(head_tracker)
    head_task = head_tasks.get(TASK, {})
    projection_mode = (
        head_tracker.get("package_id") == "PKG-03"
        and head_tracker.get("done") == 17
        and head_tracker.get("required") == 25
        and head_tracker.get("percent") == 68.0
        and head_tracker.get("active_task") == "03.18"
        and head_tracker.get("ready_tasks") == ["03.18", "03.19", "03.22"]
        and head_task.get("status") == "DONE"
    )
    if projection_mode:
        evidence = head_task.get("evidence", {})
        for key, expected in ACCEPTED_EVIDENCE.items():
            if evidence.get(key) != expected:
                fail(f"accepted projection evidence mismatch: {key}")
        for task_id in ("03.18", "03.19", "03.22"):
            if head_tasks.get(task_id, {}).get("status") != "READY":
                fail(f"projection READY set drifted at {task_id}")
        if head_tasks.get("03.20", {}).get("status") != "BLOCKED" or head_tasks.get("03.21", {}).get("status") != "BLOCKED":
            fail("projection prematurely unblocked 03.20/03.21")
    else:
        if head_tracker.get("done") != 16 or head_task.get("status") != "READY":
            fail("HEAD is neither implementation state nor accepted 03.17 projection")

    changed = [line for line in git("diff", "--name-only", f"{CANONICAL_BASE}...HEAD").splitlines() if line]
    unexpected: list[str] = []
    for path in changed:
        allowed = (
            path in PLANNING_PATHS
            or path == VALIDATOR_PATH
            or path.startswith("scripts/ci/pkg03-0317-")
            or path.startswith(".github/workflows/pkg03-0317-")
            or (projection_mode and path in PROJECTION_PATHS)
        )
        if not allowed:
            unexpected.append(path)
    if unexpected:
        fail(f"branch changed unauthorized paths: {unexpected}")
    if (not projection_mode) and any(path in PROJECTION_PATHS for path in changed):
        fail("canonical projection is forbidden before genuine 03.17 acceptance")
    if projection_mode and not PROJECTION_PATHS.issubset(set(changed)):
        fail("accepted projection is missing one or more canonical state files")
    if any(path.startswith(("apps/", "crates/", "installer/")) for path in changed):
        fail("product/installer mutation appeared before change control")

    print(json.dumps({
        "valid": True,
        "task": TASK,
        "linear": LINEAR,
        "state": head_task.get("status"),
        "mode": "accepted-projection" if projection_mode else "implementation",
        "activation_base": ACTIVATION_BASE,
        "canonical_base": CANONICAL_BASE,
        "activation_progress": {"done": activation_tracker.get("done"), "required": activation_tracker.get("required"), "percent": activation_tracker.get("percent")},
        "canonical_progress": {"done": canonical_tracker.get("done"), "required": canonical_tracker.get("required"), "percent": canonical_tracker.get("percent")},
        "head_progress": {"done": head_tracker.get("done"), "required": head_tracker.get("required"), "percent": head_tracker.get("percent")},
        "dependencies": {dep: canonical_tasks[dep].get("status") for dep in expected_deps},
        "changed_paths": changed,
        "planning_product_mutation_allowed": False,
        "certification_first": True,
    }, indent=2))


if __name__ == "__main__":
    main()
