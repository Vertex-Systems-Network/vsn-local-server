#!/usr/bin/env python3
"""Fixture-only PKG-04 04.02/04.03 metadata and trust policy model.

NON_ACCEPTANCE_PREIMPLEMENTATION. Production endpoint, trust-root and channel policy
remain activation-time inputs. This harness validates the shape and fail-closed
semantics without changing crates/vsn-update or contacting a network endpoint.
"""
from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import re
from pathlib import Path
from urllib.parse import urlparse

MODE = "NON_ACCEPTANCE_PREIMPLEMENTATION"
SEMVER_RE = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z.-]+))?$", re.ASCII)
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
SAFE_ID_RE = re.compile(r"^[A-Za-z0-9._-]{1,128}$")


class PolicyError(ValueError):
    pass


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha(value: str | bytes) -> str:
    if isinstance(value, str):
        value = value.encode("utf-8")
    return hashlib.sha256(value).hexdigest()


def semver(value: str) -> tuple[int, int, int, tuple[str, ...] | None]:
    match = SEMVER_RE.fullmatch(value)
    if not match:
        raise PolicyError("release must be a strict three-component semantic version")
    pre = tuple(match.group(4).split(".")) if match.group(4) else None
    return int(match.group(1)), int(match.group(2)), int(match.group(3)), pre


def compare_release(left: str, right: str) -> int:
    a = semver(left)
    b = semver(right)
    if a[:3] != b[:3]:
        return (a[:3] > b[:3]) - (a[:3] < b[:3])
    if a[3] is None and b[3] is None:
        return 0
    if a[3] is None:
        return 1
    if b[3] is None:
        return -1
    return (a[3] > b[3]) - (a[3] < b[3])


def validate_https_endpoint(url: str, allowed_hosts: set[str]) -> str:
    parsed = urlparse(url)
    if parsed.scheme.lower() != "https":
        raise PolicyError("update endpoint must use HTTPS")
    if parsed.username is not None or parsed.password is not None:
        raise PolicyError("update endpoint must not contain userinfo")
    host = (parsed.hostname or "").lower()
    if not host or host not in allowed_hosts:
        raise PolicyError("update endpoint host is not allowlisted")
    try:
        ipaddress.ip_address(host)
    except ValueError:
        pass
    else:
        raise PolicyError("literal IP update endpoints are forbidden")
    if parsed.port not in (None, 443):
        raise PolicyError("update endpoint requires default TLS port")
    if parsed.fragment:
        raise PolicyError("update endpoint must not contain a fragment")
    return host


def validate_artifacts(artifacts: list[dict], supported_platforms: set[tuple[str, str]], allowed_hosts: set[str]) -> None:
    if not artifacts:
        raise PolicyError("manifest requires artifacts")
    seen: set[tuple[str, str]] = set()
    for artifact in artifacts:
        platform = (artifact.get("os"), artifact.get("arch"))
        if platform not in supported_platforms:
            raise PolicyError("artifact platform is not in the frozen platform matrix")
        if platform in seen:
            raise PolicyError("duplicate artifact for platform identity")
        seen.add(platform)
        digest = artifact.get("sha256")
        if not isinstance(digest, str) or not SHA256_RE.fullmatch(digest):
            raise PolicyError("artifact SHA-256 must be lowercase canonical hex")
        size = artifact.get("bytes")
        if not isinstance(size, int) or size <= 0:
            raise PolicyError("artifact byte count must be positive")
        validate_https_endpoint(str(artifact.get("url", "")), allowed_hosts)


def validate_manifest_identity(manifest: dict, policy: dict) -> None:
    if manifest.get("schema_version") != 1 or manifest.get("product") != "vsn-platform":
        raise PolicyError("manifest identity mismatch")
    release = manifest.get("release")
    if not isinstance(release, str):
        raise PolicyError("manifest release missing")
    semver(release)
    channel = manifest.get("channel")
    if channel not in policy["allowed_channels"]:
        raise PolicyError("manifest channel is not allowed")
    published = manifest.get("published_at_unix_ms")
    if not isinstance(published, int) or published <= 0:
        raise PolicyError("manifest publication time invalid")
    key_id = manifest.get("signing_key_id")
    if not isinstance(key_id, str) or not SAFE_ID_RE.fullmatch(key_id):
        raise PolicyError("manifest signing key id invalid")
    if key_id not in policy["trusted_key_ids"]:
        raise PolicyError("manifest signing key is not trusted")
    validate_artifacts(manifest.get("artifacts", []), policy["supported_platforms"], policy["allowed_hosts"])


def validate_eligibility(manifest: dict, state: dict, policy: dict) -> None:
    validate_manifest_identity(manifest, policy)
    current_release = state.get("current_release")
    current_channel = state.get("current_channel")
    last_published = state.get("last_accepted_published_at_unix_ms")
    if not isinstance(current_release, str) or current_channel not in policy["allowed_channels"]:
        raise PolicyError("accepted update state is incomplete")
    semver(current_release)
    if not isinstance(last_published, int) or last_published <= 0:
        raise PolicyError("accepted update replay state is incomplete")
    if manifest["published_at_unix_ms"] <= last_published:
        raise PolicyError("stale or replayed update metadata")
    if compare_release(manifest["release"], current_release) <= 0:
        raise PolicyError("network update must be strictly newer than current release")
    transition = (current_channel, manifest["channel"])
    if transition not in policy["allowed_channel_transitions"]:
        raise PolicyError("channel transition is not authorized")


def validate_key_rotation(rotation: dict, policy: dict) -> None:
    old_key = rotation.get("old_key_id")
    new_key = rotation.get("new_key_id")
    if old_key not in policy["trusted_key_ids"]:
        raise PolicyError("key rotation is not anchored in an existing trusted key")
    if not isinstance(new_key, str) or not SAFE_ID_RE.fullmatch(new_key) or new_key == old_key:
        raise PolicyError("new key identity is invalid")
    if rotation.get("authorized_by_key_id") != old_key:
        raise PolicyError("key rotation authorization is not chained to old trust root")
    not_before = rotation.get("not_before_unix_ms")
    not_after = rotation.get("old_key_not_after_unix_ms")
    if not isinstance(not_before, int) or not isinstance(not_after, int) or not_before <= 0 or not_after <= not_before:
        raise PolicyError("key rotation overlap window invalid")
    if rotation.get("authorization_signature_verified") is not True:
        raise PolicyError("key rotation authorization signature is not verified")


def assert_rejected(fn, expected: str) -> str:
    try:
        fn()
    except PolicyError as exc:
        text = str(exc)
        if expected not in text:
            raise AssertionError((expected, text))
        return text
    raise AssertionError(f"expected rejection containing: {expected}")


def run(source_commit: str) -> tuple[dict, dict, list[dict]]:
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise PolicyError("source commit must be lowercase 40-character SHA-1")

    policy = {
        "schema_version": 1,
        "fixture_only": True,
        "production_policy_frozen": False,
        "allowed_channels": {"stable", "beta"},
        "allowed_channel_transitions": {("stable", "stable"), ("beta", "beta"), ("beta", "stable")},
        "supported_platforms": {("windows", "x86_64"), ("windows", "aarch64")},
        "allowed_hosts": {"updates.fixture.invalid"},
        "trusted_key_ids": {"fixture-ed25519-root-2026"},
    }
    serializable_policy = {
        "schema_version": 1,
        "mode": MODE,
        "fixture_only": True,
        "production_policy_frozen": False,
        "production_endpoint_present": False,
        "production_trust_root_present": False,
        "manifest_signature_algorithm": "Ed25519",
        "artifact_digest_algorithm": "SHA-256",
        "allowed_channels_fixture": sorted(policy["allowed_channels"]),
        "allowed_channel_transitions_fixture": [list(pair) for pair in sorted(policy["allowed_channel_transitions"])],
        "supported_platforms_fixture": [list(pair) for pair in sorted(policy["supported_platforms"])],
        "endpoint_hosts_fixture": sorted(policy["allowed_hosts"]),
        "trusted_key_ids_fixture": sorted(policy["trusted_key_ids"]),
        "network_update_requires_strict_version_increase": True,
        "replay_state_requires_monotonic_published_at": True,
        "explicit_operator_rollback_is_separate_authority": True,
        "artifact_identity_binds_platform_url_sha256_and_bytes": True,
        "key_rotation_requires_old-root_authorization_and_overlap": True,
    }

    artifact = {
        "os": "windows",
        "arch": "x86_64",
        "url": "https://updates.fixture.invalid/vsn/0.39.0/windows-x86_64.bin",
        "sha256": sha(b"fixture-artifact"),
        "bytes": 123456,
    }
    manifest = {
        "schema_version": 1,
        "product": "vsn-platform",
        "release": "0.39.0",
        "channel": "stable",
        "published_at_unix_ms": 1_800_000_000_000,
        "signing_key_id": "fixture-ed25519-root-2026",
        "artifacts": [artifact],
    }
    state = {
        "current_release": "0.38.1",
        "current_channel": "stable",
        "last_accepted_published_at_unix_ms": 1_700_000_000_000,
    }
    validate_eligibility(manifest, state, policy)

    matrix: list[dict] = [{"case": "newer-same-channel-metadata", "result": "PASS"}]
    cases = (
        ("replay", lambda m, s: m.__setitem__("published_at_unix_ms", s["last_accepted_published_at_unix_ms"]), "stale or replayed"),
        ("downgrade", lambda m, s: m.__setitem__("release", "0.37.9"), "strictly newer"),
        ("same-version", lambda m, s: m.__setitem__("release", s["current_release"]), "strictly newer"),
        ("http-endpoint", lambda m, s: m["artifacts"][0].__setitem__("url", "http://updates.fixture.invalid/a"), "must use HTTPS"),
        ("foreign-host", lambda m, s: m["artifacts"][0].__setitem__("url", "https://example.com/a"), "not allowlisted"),
        ("userinfo-endpoint", lambda m, s: m["artifacts"][0].__setitem__("url", "https://user:pass@updates.fixture.invalid/a"), "must not contain userinfo"),
        ("bad-hash", lambda m, s: m["artifacts"][0].__setitem__("sha256", "ABC"), "lowercase canonical hex"),
        ("zero-bytes", lambda m, s: m["artifacts"][0].__setitem__("bytes", 0), "byte count must be positive"),
        ("unsupported-platform", lambda m, s: m["artifacts"][0].__setitem__("arch", "mips64"), "not in the frozen platform matrix"),
        ("untrusted-key", lambda m, s: m.__setitem__("signing_key_id", "fixture-unknown-key"), "not trusted"),
        ("unauthorized-channel", lambda m, s: m.__setitem__("channel", "nightly"), "channel is not allowed"),
    )
    for label, mutate, expected in cases:
        candidate = json.loads(json.dumps(manifest))
        candidate_state = json.loads(json.dumps(state))
        mutate(candidate, candidate_state)
        reason = assert_rejected(lambda c=candidate, s=candidate_state: validate_eligibility(c, s, policy), expected)
        matrix.append({"case": label, "result": "REJECTED", "reason": reason})

    beta_state = dict(state)
    beta_state["current_channel"] = "stable"
    beta_manifest = json.loads(json.dumps(manifest))
    beta_manifest["channel"] = "beta"
    reason = assert_rejected(lambda: validate_eligibility(beta_manifest, beta_state, policy), "transition is not authorized")
    matrix.append({"case": "stable-to-beta-requires-explicit-policy", "result": "REJECTED", "reason": reason})

    duplicate = json.loads(json.dumps(manifest))
    duplicate["artifacts"].append(dict(duplicate["artifacts"][0]))
    reason = assert_rejected(lambda: validate_eligibility(duplicate, state, policy), "duplicate artifact")
    matrix.append({"case": "duplicate-platform-artifact", "result": "REJECTED", "reason": reason})

    rotation = {
        "old_key_id": "fixture-ed25519-root-2026",
        "new_key_id": "fixture-ed25519-root-2027",
        "authorized_by_key_id": "fixture-ed25519-root-2026",
        "not_before_unix_ms": 1_850_000_000_000,
        "old_key_not_after_unix_ms": 1_860_000_000_000,
        "authorization_signature_verified": True,
    }
    validate_key_rotation(rotation, policy)
    matrix.append({"case": "old-root-authorized-key-rotation", "result": "PASS"})
    bad_rotation = dict(rotation)
    bad_rotation["authorization_signature_verified"] = False
    reason = assert_rejected(lambda: validate_key_rotation(bad_rotation, policy), "signature is not verified")
    matrix.append({"case": "unsigned-key-rotation", "result": "REJECTED", "reason": reason})

    report = {
        "schema_version": 1,
        "package_id": "PKG-04",
        "task_scope": ["04.02", "04.03"],
        "mode": MODE,
        "source_commit": source_commit,
        "canonical_prerequisite": "04.01:DONE",
        "canonical_tasks_active": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "production_evidence_consumed": False,
        "product_updater_mutated": False,
        "network_access_performed": False,
        "fixture_identity": True,
        "production_endpoint_present": False,
        "production_trust_root_present": False,
        "manifest_authority_single_source_required": True,
        "ed25519_signature_authority_preserved": True,
        "sha256_artifact_identity_preserved": True,
        "artifact_byte_count_enforcement_required": True,
        "strict_https_endpoint_policy_required": True,
        "replay_rejection_required": True,
        "anti_downgrade_required": True,
        "explicit_channel_transition_policy_required": True,
        "trust_root_rotation_chain_required": True,
        "test_case_count": len(matrix),
        "negative_rejections": sum(row["result"] == "REJECTED" for row in matrix),
        "result": "PASS",
    }
    return report, serializable_policy, matrix


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--output-dir", required=True)
    args = parser.parse_args()

    report, policy, matrix = run(args.source_commit)
    out = Path(args.output_dir)
    out.mkdir(parents=True, exist_ok=True)
    (out / "policy.json").write_bytes(canonical(policy))
    (out / "matrix.json").write_bytes(canonical(matrix))
    (out / "report.json").write_bytes(canonical(report))
    print(json.dumps(report, sort_keys=True))
    print("PKG04_METADATA_TRUST_PREIMPLEMENTATION=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
