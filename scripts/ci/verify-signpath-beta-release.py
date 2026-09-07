#!/usr/bin/env python3
"""Verify a public GitHub unsigned beta/pre-release against frozen candidate evidence.

Read-only verification only. This script cannot create, edit, publish, sign, or delete a release.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import tempfile
import urllib.request


def die(message: str) -> None:
    raise SystemExit(f"VERIFY_FAIL: {message}")


def load_json(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8-sig"))
    except Exception as exc:
        die(f"cannot parse JSON {path}: {exc}")


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def require(condition: bool, message: str) -> None:
    if not condition:
        die(message)


def verify_policy_shape(policy: dict) -> None:
    require(policy.get("schema_version") == 1, "unsupported policy schema")
    require(policy.get("release_class") == "unsigned-beta-pre-release", "unexpected release class")
    source = policy.get("source_commit", "")
    require(re.fullmatch(r"[0-9a-f]{40}", source) is not None, "invalid frozen source SHA")

    assets = policy.get("required_assets")
    require(isinstance(assets, list) and assets, "required_assets must be non-empty")
    names = [a.get("name") for a in assets]
    require(len(names) == len(set(names)), "duplicate required asset names")
    for asset in assets:
        require(isinstance(asset.get("size_bytes"), int) and asset["size_bytes"] > 0, f"invalid size for {asset.get('name')}")
        require(re.fullmatch(r"[0-9a-f]{64}", asset.get("sha256", "")) is not None, f"invalid SHA-256 for {asset.get('name')}")

    manifest_req = policy.get("candidate_manifest_requirements", {})
    for flag in ("production_signed", "production_accepted", "pkg03_0322_accepted", "stable_1_0", "publish_authorized"):
        require(manifest_req.get(flag) is False, f"candidate manifest requirement {flag} must remain false")


def verify_release_metadata(policy: dict, release: dict, resolved_tag_commit: str) -> None:
    required = policy["required_release"]
    require(release.get("draft") is required["draft"], "release draft flag does not match policy")
    require(release.get("prerelease") is required["prerelease"], "release must be a GitHub pre-release")

    tag = release.get("tag_name", "")
    require(re.fullmatch(required["tag_pattern"], tag) is not None, f"tag does not match frozen beta/pre-release pattern: {tag!r}")
    require(resolved_tag_commit == policy["source_commit"], f"tag resolves to {resolved_tag_commit}, expected frozen source {policy['source_commit']}")

    body = release.get("body") or ""
    folded = body.casefold()
    for phrase in required.get("required_body_phrases_any_case", []):
        require(phrase.casefold() in folded, f"release body missing required phrase: {phrase}")
    for group in required.get("required_body_phrase_groups_any_case", []):
        require(any(option.casefold() in folded for option in group), f"release body missing one phrase from group: {group}")

    require(policy["source_commit"].casefold() in folded, "release body must identify the exact frozen source SHA")
    require("sha256sums.txt" in folded, "release body must point users to SHA256SUMS.txt")
    require("docs/code-signing-policy.md" in folded, "release body must link the Code signing policy")
    require("docs/privacy.md" in folded, "release body must link the privacy notice")


def download_asset(asset: dict, destination: Path) -> None:
    url = asset.get("url")
    require(isinstance(url, str) and url.startswith("https://api.github.com/"), f"unexpected asset API URL for {asset.get('name')}")
    headers = {
        "Accept": "application/octet-stream",
        "User-Agent": "vsn-signpath-beta-release-verifier/1",
        "X-GitHub-Api-Version": "2026-03-10",
    }
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    request = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(request, timeout=120) as response:
        destination.write_bytes(response.read())


def verify_assets(policy: dict, release: dict, work_dir: Path) -> dict:
    expected = {item["name"]: item for item in policy["required_assets"]}
    release_assets = release.get("assets") or []
    actual = {item.get("name"): item for item in release_assets}
    require(None not in actual, "release contains an unnamed asset")
    require(len(actual) == len(release_assets), "release contains duplicate asset names")
    require(set(actual) == set(expected), f"release asset set mismatch: expected={sorted(expected)} actual={sorted(actual)}")

    work_dir.mkdir(parents=True, exist_ok=True)
    evidence_assets = []
    for name in sorted(expected):
        expected_item = expected[name]
        api_item = actual[name]
        require(api_item.get("size") == expected_item["size_bytes"], f"GitHub asset size mismatch for {name}")
        path = work_dir / name
        download_asset(api_item, path)
        require(path.stat().st_size == expected_item["size_bytes"], f"downloaded size mismatch for {name}")
        digest = sha256_file(path)
        require(digest == expected_item["sha256"], f"SHA-256 mismatch for {name}: {digest}")
        evidence_assets.append({"name": name, "size_bytes": path.stat().st_size, "sha256": digest})

    return {"paths": {name: str(work_dir / name) for name in expected}, "assets": evidence_assets}


def verify_candidate_manifest(policy: dict, manifest_path: Path) -> None:
    manifest = load_json(manifest_path)
    required = policy["candidate_manifest_requirements"]
    for key, expected in required.items():
        require(manifest.get(key) == expected, f"candidate manifest {key!r} mismatch: expected={expected!r} actual={manifest.get(key)!r}")

    files = manifest.get("files") or []
    manifest_assets = {item.get("file_name"): item for item in files}
    binary_expected = {
        item["name"]: item
        for item in policy["required_assets"]
        if item["name"].lower().endswith((".exe", ".msi"))
    }
    require(set(manifest_assets) == set(binary_expected), "candidate manifest binary set mismatch")
    for name, expected in binary_expected.items():
        item = manifest_assets[name]
        require(item.get("authenticode_status") == "NotSigned", f"candidate manifest does not prove NotSigned for {name}")
        require(item.get("sha256") == expected["sha256"], f"candidate manifest SHA mismatch for {name}")
        require(item.get("size_bytes") == expected["size_bytes"], f"candidate manifest size mismatch for {name}")


def verify_sums(policy: dict, sums_path: Path) -> None:
    parsed = {}
    for raw in sums_path.read_text(encoding="ascii").splitlines():
        line = raw.strip()
        if not line:
            continue
        parts = line.split(None, 1)
        require(len(parts) == 2, f"invalid SHA256SUMS line: {raw!r}")
        parsed[parts[1].lstrip("*")] = parts[0].lower()

    binary_expected = {
        item["name"]: item["sha256"]
        for item in policy["required_assets"]
        if item["name"].lower().endswith((".exe", ".msi"))
    }
    require(parsed == binary_expected, f"SHA256SUMS content mismatch: expected={binary_expected} actual={parsed}")


def run_self_test(policy: dict) -> None:
    verify_policy_shape(policy)
    release = {
        "draft": False,
        "prerelease": True,
        "tag_name": "v0.38.1-beta.1",
        "body": (
            "UNSIGNED beta pre-release. This is not production-signed and does not satisfy 03.22. "
            f"Exact source {policy['source_commit']}. See SHA256SUMS.txt. "
            "Code signing policy: docs/CODE-SIGNING-POLICY.md. "
            "Privacy notice: docs/PRIVACY.md."
        ),
    }
    verify_release_metadata(policy, release, policy["source_commit"])
    try:
        bad = dict(release)
        bad["prerelease"] = False
        verify_release_metadata(policy, bad, policy["source_commit"])
    except SystemExit:
        pass
    else:
        die("self-test failed: stable release was not rejected")

    try:
        verify_release_metadata(policy, release, "0" * 40)
    except SystemExit:
        pass
    else:
        die("self-test failed: wrong tag commit was not rejected")

    print("SELF_TEST=PASS")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--policy", type=Path, required=True)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--release-json", type=Path)
    parser.add_argument("--resolved-tag-commit")
    parser.add_argument("--work-dir", type=Path)
    parser.add_argument("--evidence-out", type=Path)
    args = parser.parse_args()

    policy = load_json(args.policy)
    verify_policy_shape(policy)
    if args.self_test:
        run_self_test(policy)
        return

    require(args.release_json is not None, "--release-json is required")
    require(args.resolved_tag_commit is not None, "--resolved-tag-commit is required")
    require(re.fullmatch(r"[0-9a-f]{40}", args.resolved_tag_commit) is not None, "resolved tag commit must be a lowercase 40-char SHA")
    release = load_json(args.release_json)
    verify_release_metadata(policy, release, args.resolved_tag_commit)

    work_dir = args.work_dir or Path(tempfile.mkdtemp(prefix="vsn-beta-release-"))
    asset_evidence = verify_assets(policy, release, work_dir)
    verify_candidate_manifest(policy, Path(asset_evidence["paths"]["candidate-manifest.json"]))
    verify_sums(policy, Path(asset_evidence["paths"]["SHA256SUMS.txt"]))

    evidence = {
        "schema_version": 1,
        "verification": "PASS",
        "release_id": release.get("id"),
        "release_url": release.get("html_url"),
        "tag_name": release.get("tag_name"),
        "resolved_tag_commit": args.resolved_tag_commit,
        "source_commit": policy["source_commit"],
        "prerelease": release.get("prerelease"),
        "draft": release.get("draft"),
        "assets": asset_evidence["assets"],
        "production_signing_accepted": False,
        "pkg03_0322_accepted": False,
    }
    if args.evidence_out:
        args.evidence_out.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(evidence, indent=2))
    print("RELEASE_VERIFICATION=PASS")


if __name__ == "__main__":
    main()
