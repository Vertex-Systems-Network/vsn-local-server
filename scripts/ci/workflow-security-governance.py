#!/usr/bin/env python3
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"

# Current Node 24 action revisions used by newly hardened workflows in this PR.
CURRENT_CHECKOUT = "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1"
CURRENT_UPLOAD = "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a"

# Existing trusted-main production-signing pins are deliberately preserved in this
# independent hardening PR. Changing them would require a fresh 03.22 trust-boundary
# reconciliation/readiness cycle.
TRUSTED_SIGNING_CHECKOUT = "actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683"
TRUSTED_SIGNING_UPLOAD = "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02"
TRUSTED_SIGNING_DOWNLOAD = "actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093"

# This historical write workflow targeted a completed PKG-01 branch/merged PR and is
# intentionally retired. A future auto-fix workflow must be designed under a new,
# explicit least-privilege contract rather than silently restoring this file.
RETIRED_WRITE_WORKFLOWS = ("pkg01-autoformat.yml",)


def read(rel: str) -> str:
    path = ROOT / rel
    if not path.is_file():
        raise RuntimeError(f"missing required workflow-security file: {rel}")
    return path.read_text(encoding="utf-8")


def workflow_paths() -> list[Path]:
    return sorted((*WORKFLOWS.glob("*.yml"), *WORKFLOWS.glob("*.yaml")))


def job_section(text: str, name: str, next_names: tuple[str, ...]) -> str:
    marker = f"  {name}:\n"
    start = text.find(marker)
    if start < 0:
        return ""
    end = len(text)
    for other in next_names:
        pos = text.find(f"  {other}:\n", start + len(marker))
        if pos >= 0:
            end = min(end, pos)
    return text[start:end]


def global_workflow_errors() -> list[str]:
    errors: list[str] = []
    for path in workflow_paths():
        text = path.read_text(encoding="utf-8")
        rel = path.relative_to(ROOT).as_posix()
        if re.search(r"(?m)^\s*pull_request_target\s*:", text):
            errors.append(f"{rel}: pull_request_target is forbidden")
        if re.search(r"(?m)^\s*permissions\s*:\s*write-all\s*(?:#.*)?$", text):
            errors.append(f"{rel}: permissions: write-all is forbidden")
        if re.search(
            r"(?m)^\s*uses\s*:\s*[^\s#]+@(main|master|latest)\s*(?:#.*)?$",
            text,
            re.IGNORECASE,
        ):
            errors.append(f"{rel}: action refs may not use floating main/master/latest branches")
    return errors


def privileged_action_pin_errors() -> list[str]:
    errors: list[str] = []
    write_permission = re.compile(
        r"(?m)^\s+[A-Za-z0-9_-]+\s*:\s*write\s*(?:#.*)?$"
    )
    uses_line = re.compile(r"(?m)^\s*uses\s*:\s*([^\s#]+)\s*(?:#.*)?$")
    immutable_external = re.compile(r"^[^@\s]+@[0-9a-fA-F]{40}$")

    for path in workflow_paths():
        text = path.read_text(encoding="utf-8")
        if not write_permission.search(text):
            continue
        rel = path.relative_to(ROOT).as_posix()
        for action in uses_line.findall(text):
            if action.startswith("./"):
                # Local actions are governed as repository code and cannot be SHA-pinned.
                continue
            if not immutable_external.fullmatch(action):
                errors.append(
                    f"{rel}: privileged workflow action must use immutable 40-char SHA: {action}"
                )
    return errors


def retired_workflow_errors() -> list[str]:
    errors: list[str] = []
    for name in RETIRED_WRITE_WORKFLOWS:
        if (WORKFLOWS / name).exists():
            errors.append(
                f".github/workflows/{name}: retired write-capable workflow must not be restored"
            )
    return errors


def repository_governance_errors() -> list[str]:
    errors: list[str] = []
    text = read(".github/workflows/repository-governance.yml")
    required = (
        "permissions:\n  contents: read",
        CURRENT_CHECKOUT,
        "persist-credentials: false",
        "run: python3 scripts/repository-governance.py",
        "run: python3 scripts/ci/workflow-security-governance.py",
    )
    for token in required:
        if token not in text:
            errors.append(f"repository-governance.yml missing security invariant: {token}")
    if re.search(r"actions/checkout@v[0-9]", text):
        errors.append("repository-governance.yml must not use a floating checkout major tag")
    return errors


def certification_router_errors() -> list[str]:
    errors: list[str] = []
    rel = ".github/workflows/self-hosted-certification-router.yml"
    text = read(rel)

    route = job_section(text, "route-certification", ("certify", "report-result"))
    certify = job_section(text, "certify", ("report-result",))
    report = job_section(text, "report-result", ())
    if not route or not certify or not report:
        errors.append(f"{rel}: expected route-certification -> certify -> report-result privilege separation")
        return errors

    top = text[: text.find("jobs:")]
    if "issues: write" in top:
        errors.append(f"{rel}: issues: write must not be workflow-global")
    if "pull-requests: read" not in top or "contents: read" not in top:
        errors.append(f"{rel}: global permissions must be read-only routing permissions")

    for token in (
        "VSN_COMMENT_BODY: ${{ github.event.comment.body }}",
        "$command = [string]$env:VSN_COMMENT_BODY",
    ):
        if token not in route:
            errors.append(f"{rel}: route job missing safe comment transport invariant: {token}")
    if "$command = '${{ github.event.comment.body }}'" in route:
        errors.append(f"{rel}: comment body must never be directly interpolated into PowerShell source")
    if "issues: write" in route:
        errors.append(f"{rel}: route job must not have issues: write")

    required_certify = (
        "permissions:\n      contents: read",
        CURRENT_CHECKOUT,
        "persist-credentials: false",
        CURRENT_UPLOAD,
        "Prove PR-controlled execution has no write token surface",
    )
    for token in required_certify:
        if token not in certify:
            errors.append(f"{rel}: certify job missing invariant: {token}")
    for forbidden in ("issues: write", "GH_TOKEN:", "VSN_REPORT_TOKEN:", "${{ github.token }}"):
        if forbidden in certify:
            errors.append(f"{rel}: certify job exposes forbidden token/permission surface: {forbidden}")

    if "permissions:\n      issues: write" not in report:
        errors.append(f"{rel}: report-result must hold the isolated issues: write permission")
    for forbidden in ("actions/checkout@", "scripts/self-hosted/", "Run requested certification script"):
        if forbidden in report:
            errors.append(f"{rel}: report-result must never checkout or execute PR-controlled code: {forbidden}")
    if "VSN_REPORT_TOKEN: ${{ github.token }}" not in report:
        errors.append(f"{rel}: report-result token must be scoped to the reporting job only")

    return errors


def production_signing_errors() -> list[str]:
    errors: list[str] = []
    rel = ".github/workflows/pkg03-0322-production-signing-trusted.yml"
    text = read(rel)
    required = (
        "permissions:\n  contents: read",
        "environment: production-signing",
        "persist-credentials: false",
        TRUSTED_SIGNING_CHECKOUT,
        TRUSTED_SIGNING_UPLOAD,
        TRUSTED_SIGNING_DOWNLOAD,
        "github.event_name == 'push' && github.ref == 'refs/heads/main'",
    )
    for token in required:
        if token not in text:
            errors.append(f"{rel}: missing trusted-signing invariant: {token}")
    if re.search(r"(?m)^\s*pull_request_target\s*:", text):
        errors.append(f"{rel}: trusted signing must never use pull_request_target")
    return errors


def main() -> int:
    errors: list[str] = []
    errors.extend(global_workflow_errors())
    errors.extend(privileged_action_pin_errors())
    errors.extend(retired_workflow_errors())
    errors.extend(repository_governance_errors())
    errors.extend(certification_router_errors())
    errors.extend(production_signing_errors())

    if errors:
        print("WORKFLOW SECURITY GOVERNANCE: FAIL")
        for error in errors:
            print(f"- {error}")
        return 1

    print("WORKFLOW SECURITY GOVERNANCE: PASS")
    print("- pull_request_target forbidden repository-wide")
    print("- write-all and floating branch action refs forbidden repository-wide")
    print("- every external action in a write-capable workflow is immutable-SHA pinned")
    print("- completed PKG-01 write auto-fix workflow remains retired")
    print("- required governance checkout pinned to current Node 24 release and credential-free")
    print("- certification PR code isolated from issues:write token")
    print("- certification checkout/upload actions pinned to current immutable Node 24 revisions")
    print("- trusted production-signing boundary invariants preserved without provider-lane mutation")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
