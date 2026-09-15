#!/usr/bin/env python3
"""Synthetic, audit-only probe for the future PKG-03 03.22 -> 03.23 handoff.

This tool intentionally accepts no production evidence inputs. It only exercises
in-memory fixtures so it cannot become 03.23 acceptance authority while 03.22 is
not canonically DONE.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
from pathlib import Path

EXPECTED_FILES = (
    "nsis-current-user.exe",
    "nsis-per-machine.exe",
    "vsn-platform.msi",
    "VSN Dev Platform.exe",
)
HEX40 = re.compile(r"^[0-9a-f]{40}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")
FORBIDDEN_KEY_FRAGMENTS = ("token", "password", "pfx", "private_key", "private-key")


class ContractError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ContractError(message)


def synthetic_hash(label: str) -> str:
    return hashlib.sha256(label.encode("utf-8")).hexdigest()


def scan_non_secret(value: object, path: str = "$") -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            lowered = str(key).lower()
            require(
                not any(fragment in lowered for fragment in FORBIDDEN_KEY_FRAGMENTS),
                f"secret-bearing field is forbidden in handoff evidence: {path}.{key}",
            )
            scan_non_secret(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            scan_non_secret(child, f"{path}[{index}]")


def validate_contract(upstream: dict, handoff: dict) -> None:
    """Validate only provider-neutral continuity invariants planned for 03.23."""

    require(upstream.get("schema_version") == 1, "upstream schema mismatch")
    require(upstream.get("task_id") == "03.22", "upstream task must be 03.22")
    require(upstream.get("production_accepted") is True, "03.22 production acceptance is required")

    source = upstream.get("source_commit")
    require(isinstance(source, str) and HEX40.fullmatch(source) is not None, "invalid 03.22 source SHA")

    evidence_sha = upstream.get("evidence_sha256")
    require(isinstance(evidence_sha, str) and HEX64.fullmatch(evidence_sha) is not None, "invalid 03.22 evidence digest")

    candidates = upstream.get("candidates")
    require(isinstance(candidates, list), "03.22 candidates must be a list")
    require(len(candidates) == len(EXPECTED_FILES), "03.22 candidate count mismatch")

    by_name: dict[str, dict] = {}
    for row in candidates:
        require(isinstance(row, dict), "03.22 candidate row must be an object")
        name = row.get("file_name")
        require(name in EXPECTED_FILES, f"unexpected 03.22 candidate: {name}")
        require(name not in by_name, f"duplicate 03.22 candidate: {name}")
        signed_sha = row.get("signed_sha256")
        unsigned_sha = row.get("unsigned_sha256")
        require(isinstance(signed_sha, str) and HEX64.fullmatch(signed_sha) is not None, f"invalid signed SHA-256 for {name}")
        require(isinstance(unsigned_sha, str) and HEX64.fullmatch(unsigned_sha) is not None, f"invalid unsigned SHA-256 for {name}")
        require(signed_sha != unsigned_sha, f"signed and unsigned hashes unexpectedly match for {name}")
        require(row.get("windows_status") == "Valid", f"Windows Authenticode status is not Valid for {name}")
        require(row.get("timestamp_present") is True, f"timestamp missing for {name}")
        require(row.get("identity_equal") is True, f"package identity changed for {name}")
        require(isinstance(row.get("signer_subject"), str) and row["signer_subject"].strip(), f"signer subject missing for {name}")
        by_name[name] = row

    require(set(by_name) == set(EXPECTED_FILES), "03.22 candidate set mismatch")

    require(handoff.get("schema_version") == 1, "handoff schema mismatch")
    require(handoff.get("task_id") == "03.23", "handoff task must be 03.23")
    require(handoff.get("upstream_task_id") == "03.22", "handoff upstream task mismatch")
    require(handoff.get("source_commit") == source, "handoff source SHA differs from accepted 03.22 source")
    require(handoff.get("upstream_evidence_sha256") == evidence_sha, "handoff references different 03.22 evidence")
    require(handoff.get("synthetic_audit_only") is True, "audit fixture marker is required")

    subjects = handoff.get("subjects")
    require(isinstance(subjects, list), "handoff subjects must be a list")
    require(len(subjects) == len(EXPECTED_FILES), "handoff subject count mismatch")

    seen: set[str] = set()
    for subject in subjects:
        require(isinstance(subject, dict), "handoff subject row must be an object")
        name = subject.get("file_name")
        require(name in by_name, f"handoff contains unknown subject: {name}")
        require(name not in seen, f"handoff contains duplicate subject: {name}")
        seen.add(name)
        require(subject.get("sha256") == by_name[name]["signed_sha256"], f"handoff digest is not exact accepted signed bytes for {name}")
        require(subject.get("signer_subject") == by_name[name]["signer_subject"], f"handoff signer subject mismatch for {name}")
        require(subject.get("timestamp_verified") is True, f"handoff timestamp verification missing for {name}")

    require(seen == set(EXPECTED_FILES), "handoff subject set is incomplete")
    scan_non_secret(handoff)


def make_good_fixture() -> tuple[dict, dict]:
    source = "1" * 40
    publisher = "CN=SignPath Foundation"
    candidates = []
    subjects = []
    for name in EXPECTED_FILES:
        unsigned_sha = synthetic_hash(f"unsigned:{name}")
        signed_sha = synthetic_hash(f"signed:{name}")
        candidates.append(
            {
                "file_name": name,
                "unsigned_sha256": unsigned_sha,
                "signed_sha256": signed_sha,
                "windows_status": "Valid",
                "timestamp_present": True,
                "identity_equal": True,
                "signer_subject": publisher,
            }
        )
        subjects.append(
            {
                "file_name": name,
                "sha256": signed_sha,
                "signer_subject": publisher,
                "timestamp_verified": True,
            }
        )

    upstream = {
        "schema_version": 1,
        "task_id": "03.22",
        "source_commit": source,
        "production_accepted": True,
        "evidence_sha256": synthetic_hash("accepted-03.22-evidence"),
        "candidates": candidates,
    }
    handoff = {
        "schema_version": 1,
        "task_id": "03.23",
        "upstream_task_id": "03.22",
        "source_commit": source,
        "upstream_evidence_sha256": upstream["evidence_sha256"],
        "synthetic_audit_only": True,
        "subjects": subjects,
    }
    return upstream, handoff


def run_self_test() -> dict:
    base_upstream, base_handoff = make_good_fixture()
    cases: list[dict] = []

    def expect_pass(name: str, upstream: dict, handoff: dict) -> None:
        validate_contract(upstream, handoff)
        cases.append({"name": name, "expected": "PASS", "result": "PASS"})

    def expect_reject(name: str, mutate) -> None:
        upstream = copy.deepcopy(base_upstream)
        handoff = copy.deepcopy(base_handoff)
        mutate(upstream, handoff)
        try:
            validate_contract(upstream, handoff)
        except ContractError as exc:
            cases.append({"name": name, "expected": "REJECT", "result": "REJECT", "reason": str(exc)})
            return
        raise AssertionError(f"negative fixture unexpectedly passed: {name}")

    expect_pass("exact-production-signed-handoff", copy.deepcopy(base_upstream), copy.deepcopy(base_handoff))
    expect_reject("reject-non-production-0322", lambda u, h: u.__setitem__("production_accepted", False))
    expect_reject("reject-rebuild-or-different-signed-bytes", lambda u, h: h["subjects"][0].__setitem__("sha256", synthetic_hash("different-build")))
    expect_reject("reject-unsigned-substitution", lambda u, h: h["subjects"][1].__setitem__("sha256", u["candidates"][1]["unsigned_sha256"]))
    expect_reject("reject-source-lineage-mismatch", lambda u, h: h.__setitem__("source_commit", "2" * 40))
    expect_reject("reject-upstream-evidence-mismatch", lambda u, h: h.__setitem__("upstream_evidence_sha256", synthetic_hash("other-evidence")))
    expect_reject("reject-duplicate-subject", lambda u, h: h["subjects"].__setitem__(3, copy.deepcopy(h["subjects"][0])))
    expect_reject("reject-missing-subject", lambda u, h: h["subjects"].pop())
    expect_reject("reject-invalid-authenticode", lambda u, h: u["candidates"][0].__setitem__("windows_status", "HashMismatch"))
    expect_reject("reject-missing-timestamp", lambda u, h: u["candidates"][0].__setitem__("timestamp_present", False))
    expect_reject("reject-package-identity-drift", lambda u, h: u["candidates"][2].__setitem__("identity_equal", False))
    expect_reject("reject-secret-bearing-handoff-field", lambda u, h: h.__setitem__("api_token", "synthetic-do-not-use"))

    require(len(cases) == 12, "self-test case count drift")
    require(sum(case["result"] == "PASS" for case in cases) == 1, "positive case count mismatch")
    require(sum(case["result"] == "REJECT" for case in cases) == 11, "negative rejection count mismatch")

    return {
        "schema_version": 1,
        "audit": "PKG-03 03.22 -> 03.23 handoff contract",
        "mode": "synthetic-only",
        "production_evidence_consumed": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "cases": cases,
        "summary": {"pass": 1, "negative_rejections": 11, "total": 12},
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    report = run_self_test()
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report["summary"], sort_keys=True))
    print("AUDIT_ONLY_0323_HANDOFF_CONTRACT=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
