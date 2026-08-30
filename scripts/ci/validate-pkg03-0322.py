#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import re
import subprocess
from pathlib import Path

TASK = "03.22"
LINEAR = "ABD-97"
BASE = "f3afb66e588d01ff2e8cb37273ad413862a4edaf"
MANIFEST_PATH = Path(".ai/manifests/pkg03-0322-authenticode-signing.v1.json")
TRACKER_PATH = "certification/pkg03-windows-installer-v1.json"
VALIDATOR_PATH = "scripts/ci/validate-pkg03-0322.py"
PLANNING_PATHS = {
    ".ai/features/pkg03-0322/research.md",
    ".ai/features/pkg03-0322/lifecycle-review.md",
    ".ai/features/pkg03-0322/development-preflight.md",
    ".ai/plans/pkg03-0322-authenticode-signing-v1.md",
    ".ai/manifests/pkg03-0322-authenticode-signing.v1.json",
    "docs/PKG03-AUTHENTICODE-SIGNING-VERIFICATION-V1.md",
}
PROJECTION_PATHS = {
    "certification/pkg03-windows-installer-v1.json",
    "docs/MASTER-EXECUTION-STATUS.json",
    "README.md",
    ".ai/README.md",
    "docs/MASTER-EXECUTION-PLAN.md",
}
FORBIDDEN_SECRET_SUFFIXES = {".pfx", ".p12", ".key", ".pem"}
# Match actual PEM private-key material, not source-code literals that implement
# secret scanning. A genuine PEM block has the header followed by a base64 body.
PRIVATE_KEY_PEM_RE = re.compile(
    r"-----BEGIN (?:RSA |EC |ENCRYPTED )?PRIVATE KEY-----[\r\n]+[A-Za-z0-9+/=]{32,}",
    re.MULTILINE,
)


def fail(message: str) -> None:
    raise SystemExit(f"03.22 authority validation failed: {message}")


def git_text(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True).strip()


def tracked_bytes(path: str, ref: str = "HEAD") -> bytes:
    try:
        return subprocess.check_output(["git", "show", f"{ref}:{path}"])
    except subprocess.CalledProcessError as exc:
        fail(f"cannot read tracked artifact {ref}:{path} ({exc.returncode})")


def tracked_sha256(path: str, ref: str = "HEAD") -> str:
    return hashlib.sha256(tracked_bytes(path, ref)).hexdigest()


def base_json(path: str) -> dict:
    return json.loads(tracked_bytes(path, BASE).decode("utf-8"))


def main() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    if manifest.get("task_id") != TASK or manifest.get("linear_issue") != LINEAR:
        fail("manifest task/Linear identity mismatch")
    if manifest.get("canonical_base_sha") != BASE or manifest.get("status") != "frozen":
        fail("canonical base/frozen status mismatch")
    if manifest.get("research", {}).get("change_required") is not False:
        fail("planning is not integration/certification-first")

    bindings = [
        (manifest["research"]["artifact"], manifest["research"]["sha256"]),
        (manifest["lifecycle"]["artifact"], manifest["lifecycle"]["sha256"]),
        (manifest["development_preflight"]["artifact"], manifest["development_preflight"]["sha256"]),
        (manifest["task_plan"]["path"], manifest["task_plan"]["sha256"]),
        (manifest["lifecycle_contract"]["artifact"], manifest["lifecycle_contract"]["sha256"]),
        (manifest["parent_plan"]["path"], manifest["parent_plan"]["sha256"]),
    ]
    mismatches: list[dict[str, str]] = []
    for path, expected in bindings:
        actual = tracked_sha256(path)
        if actual != expected:
            mismatches.append({"path": path, "expected": expected, "actual": actual})
    if mismatches:
        fail(f"frozen Git-blob digest mismatches: {json.dumps(mismatches, sort_keys=True)}")

    locked = manifest.get("locked_inputs", {})
    if locked.get("dependency_tasks") != ["03.02", "03.03", "03.14"]:
        fail("dependency binding drifted")
    if locked.get("authenticode_digest") != "SHA256":
        fail("Authenticode digest is not SHA256")
    if locked.get("timestamp_protocol") != "RFC3161" or locked.get("timestamp_digest") != "SHA256":
        fail("timestamp contract is not RFC3161/SHA256")
    if locked.get("production_credentials_external") is not True:
        fail("production credentials are not marked external")

    authority = manifest.get("authority", {})
    required_false = (
        "production_secret_material_in_repo_allowed",
        "package_identity_mutation_allowed",
        "product_payload_source_mutation_allowed",
        "service_identity_mutation_allowed",
        "acl_mutation_allowed",
        "network_mutation_allowed",
        "updater_mutation_allowed",
        "pkg05_release_mutation_allowed",
        "delegated_scope_may_expand",
    )
    for key in required_false:
        if authority.get(key) is not False:
            fail(f"authority widened: {key}")
    if authority.get("tauri_signing_config_mutation_requires_change_control") is not True:
        fail("shared Tauri signing configuration change-control requirement disappeared")

    acceptance = manifest.get("acceptance", {})
    for key in (
        "sha256_authenticode_required",
        "rfc3161_timestamp_required",
        "expected_publisher_binding_required",
        "windows_native_verification_required",
        "tamper_negative_required",
        "secret_leak_scan_required",
    ):
        if acceptance.get(key) is not True:
            fail(f"required acceptance flag missing: {key}")
    if acceptance.get("production_acceptance_from_test_certificate_allowed") is not False:
        fail("test certificate was allowed to satisfy production acceptance")

    tracker = base_json(TRACKER_PATH)
    if tracker.get("package_id") != "PKG-03" or tracker.get("done") != 15 or tracker.get("required") != 25:
        fail("canonical package baseline is not 15/25")
    tasks = {row.get("id"): row for row in tracker.get("tasks", [])}
    deps = ["03.02", "03.03", "03.14"]
    task = tasks.get(TASK)
    if not task or task.get("status") != "READY" or task.get("depends_on") != deps:
        fail("03.22 READY/dependency contract drifted")
    for dep in deps:
        if tasks.get(dep, {}).get("status") != "DONE":
            fail(f"dependency {dep} is not canonically DONE")

    changed = [p for p in git_text("diff", "--name-only", f"{BASE}...HEAD").splitlines() if p]
    unexpected: list[str] = []
    for path in changed:
        allowed = (
            path in PLANNING_PATHS
            or path == VALIDATOR_PATH
            or path.startswith("scripts/ci/pkg03-0322-")
            or path.startswith(".github/workflows/pkg03-0322-")
        )
        if not allowed:
            unexpected.append(path)
    if unexpected:
        fail(f"unauthorized changed paths: {unexpected}")
    if any(path in PROJECTION_PATHS for path in changed):
        fail("canonical projection appeared before accepted production-signing evidence")
    if any(path.startswith(("apps/", "crates/", "installer/")) for path in changed):
        fail("product/runtime/installer mutation appeared without change control")

    for path in changed:
        suffix = Path(path).suffix.lower()
        if suffix in FORBIDDEN_SECRET_SUFFIXES:
            fail(f"forbidden secret-bearing file type tracked: {path}")
        try:
            text = tracked_bytes(path).decode("utf-8")
        except UnicodeDecodeError:
            continue
        if PRIVATE_KEY_PEM_RE.search(text):
            fail(f"actual PEM private-key material tracked in {path}")

    print(json.dumps({
        "valid": True,
        "task": TASK,
        "linear": LINEAR,
        "canonical_base": BASE,
        "canonical_progress": {"done": tracker.get("done"), "required": tracker.get("required"), "percent": tracker.get("percent")},
        "dependencies": {dep: tasks[dep].get("status") for dep in deps},
        "changed_paths": changed,
        "production_credentials_external": True,
        "shared_tauri_config_mutated": False,
        "production_acceptance_from_test_certificate": False,
    }, indent=2))


if __name__ == "__main__":
    main()
