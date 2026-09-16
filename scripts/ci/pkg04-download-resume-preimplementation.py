#!/usr/bin/env python3
"""Fixture-only PKG-04 04.04 bounded discovery/download/resume/cache contract.

NON_ACCEPTANCE_PREIMPLEMENTATION. No network request is issued and no product updater
code is modified. The harness models the fail-closed boundary between verified update
metadata and the pre-downloaded artifact consumed by vsn-update.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from urllib.parse import urlparse

MODE = "NON_ACCEPTANCE_PREIMPLEMENTATION"
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


class DownloadError(ValueError):
    pass


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def artifact_cache_key(artifact: dict) -> str:
    digest = artifact.get("sha256")
    size = artifact.get("bytes")
    if not isinstance(digest, str) or not SHA256_RE.fullmatch(digest):
        raise DownloadError("artifact digest is not canonical SHA-256")
    if not isinstance(size, int) or size <= 0:
        raise DownloadError("artifact byte count is invalid")
    return f"sha256-{digest}-{size}"


def validate_source(artifact: dict, policy: dict) -> str:
    if artifact.get("metadata_signature_verified") is not True:
        raise DownloadError("artifact descriptor is not from verified metadata")
    url = artifact.get("url")
    if not isinstance(url, str):
        raise DownloadError("artifact URL is missing")
    parsed = urlparse(url)
    if parsed.scheme.lower() != "https":
        raise DownloadError("artifact download requires HTTPS")
    if parsed.username is not None or parsed.password is not None:
        raise DownloadError("artifact URL must not contain userinfo")
    host = (parsed.hostname or "").lower()
    if host not in policy["allowed_hosts"]:
        raise DownloadError("artifact host is not allowlisted")
    if parsed.port not in (None, 443):
        raise DownloadError("artifact download requires default TLS port")
    size = artifact.get("bytes")
    if not isinstance(size, int) or size <= 0 or size > policy["max_artifact_bytes"]:
        raise DownloadError("artifact byte count exceeds bounded download policy")
    artifact_cache_key(artifact)
    return host


def validate_redirect(original_url: str, redirect_url: str, allowed_hosts: set[str]) -> None:
    original = urlparse(original_url)
    redirect = urlparse(redirect_url)
    if redirect.scheme.lower() != "https":
        raise DownloadError("redirect must preserve HTTPS")
    if redirect.username is not None or redirect.password is not None:
        raise DownloadError("redirect must not contain userinfo")
    if (redirect.hostname or "").lower() not in allowed_hosts:
        raise DownloadError("redirect host is not allowlisted")
    if redirect.port not in (None, 443):
        raise DownloadError("redirect requires default TLS port")
    if (original.hostname or "").lower() != (redirect.hostname or "").lower():
        raise DownloadError("cross-host redirect requires separately frozen policy")


def validate_resume_response(state: dict, response: dict, artifact: dict) -> None:
    offset = state.get("downloaded_bytes")
    expected_total = artifact.get("bytes")
    if not isinstance(offset, int) or offset < 0 or offset >= expected_total:
        raise DownloadError("resume offset is invalid")
    if state.get("artifact_cache_key") != artifact_cache_key(artifact):
        raise DownloadError("resume state artifact identity mismatch")
    if response.get("status") != 206:
        raise DownloadError("resume requires HTTP 206; never append a full-body response")
    if response.get("content_range_start") != offset:
        raise DownloadError("resume Content-Range start does not match local offset")
    if response.get("content_range_total") != expected_total:
        raise DownloadError("resume Content-Range total does not match signed byte count")
    state_etag = state.get("strong_etag")
    response_etag = response.get("strong_etag")
    if state_etag is not None:
        if response_etag != state_etag:
            raise DownloadError("resume entity identity changed")
    elif response_etag is not None and str(response_etag).startswith("W/"):
        raise DownloadError("weak ETag is not a resume identity authority")


def stream_verify(chunks: list[bytes], expected_sha256: str, expected_bytes: int, max_chunk_bytes: int) -> bool:
    if max_chunk_bytes <= 0:
        raise DownloadError("stream chunk bound is invalid")
    hasher = hashlib.sha256()
    total = 0
    for chunk in chunks:
        if len(chunk) > max_chunk_bytes:
            raise DownloadError("stream chunk exceeds bounded memory policy")
        total += len(chunk)
        if total > expected_bytes:
            raise DownloadError("download exceeded signed artifact byte count")
        hasher.update(chunk)
    if total != expected_bytes:
        raise DownloadError("download byte count does not match signed artifact byte count")
    if hasher.hexdigest() != expected_sha256:
        raise DownloadError("download SHA-256 does not match verified artifact identity")
    return True


def validate_cache_reuse(cache: dict, artifact: dict) -> None:
    if cache.get("artifact_cache_key") != artifact_cache_key(artifact):
        raise DownloadError("cache identity does not match artifact")
    if cache.get("verified_complete") is not True:
        raise DownloadError("cache entry is not verified complete")
    if cache.get("size") != artifact["bytes"]:
        raise DownloadError("cache byte count mismatch")
    if cache.get("sha256") != artifact["sha256"]:
        raise DownloadError("cache digest mismatch")


def assert_rejected(fn, expected: str) -> str:
    try:
        fn()
    except DownloadError as exc:
        text = str(exc)
        if expected not in text:
            raise AssertionError((expected, text))
        return text
    raise AssertionError(f"expected rejection containing: {expected}")


def run(source_commit: str) -> tuple[dict, dict, list[dict]]:
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise DownloadError("source commit must be lowercase 40-character SHA-1")

    payload = b"0123456789abcdef" * 4096
    artifact = {
        "url": "https://updates.fixture.invalid/vsn/0.39.0/windows-x86_64.bin",
        "sha256": sha(payload),
        "bytes": len(payload),
        "metadata_signature_verified": True,
    }
    policy = {
        "allowed_hosts": {"updates.fixture.invalid"},
        "max_artifact_bytes": 1024 * 1024 * 1024,
        "max_chunk_bytes": 4 * 1024 * 1024,
        "max_redirects": 3,
        "max_retries": 4,
    }
    validate_source(artifact, policy)
    key = artifact_cache_key(artifact)

    serializable_policy = {
        "schema_version": 1,
        "mode": MODE,
        "fixture_only": True,
        "production_endpoint_frozen": False,
        "network_access_performed": False,
        "verified_metadata_required_before_download": True,
        "https_and_host_allowlist_required": True,
        "cross_host_redirect_default": "REJECT",
        "resume_requires_206_and_exact_content_range": True,
        "full_body_200_must_never_append_to_partial": True,
        "strong_entity_identity_required_when_resume_identity_present": True,
        "cache_key_binds_sha256_and_bytes": True,
        "completed_cache_reverified_before_reuse": True,
        "streaming_sha256_required": True,
        "bounded_chunk_memory_required": True,
        "signed_byte_count_required": True,
        "mutation_lock_held_during_download": False,
        "download_never_executes_artifact": True,
        "fixture_max_artifact_bytes": policy["max_artifact_bytes"],
        "fixture_max_chunk_bytes": policy["max_chunk_bytes"],
        "fixture_max_redirects": policy["max_redirects"],
        "fixture_max_retries": policy["max_retries"],
    }

    matrix: list[dict] = [{"case": "verified-metadata-download-source", "result": "PASS"}]

    unverified = dict(artifact)
    unverified["metadata_signature_verified"] = False
    reason = assert_rejected(lambda: validate_source(unverified, policy), "not from verified metadata")
    matrix.append({"case": "unverified-metadata-source", "result": "REJECTED", "reason": reason})

    for label, url, expected in (
        ("http-source", "http://updates.fixture.invalid/a", "requires HTTPS"),
        ("foreign-host", "https://example.com/a", "not allowlisted"),
        ("userinfo-source", "https://user:pass@updates.fixture.invalid/a", "must not contain userinfo"),
        ("nondefault-port", "https://updates.fixture.invalid:444/a", "default TLS port"),
    ):
        candidate = dict(artifact)
        candidate["url"] = url
        reason = assert_rejected(lambda c=candidate: validate_source(c, policy), expected)
        matrix.append({"case": label, "result": "REJECTED", "reason": reason})

    oversized = dict(artifact)
    oversized["bytes"] = policy["max_artifact_bytes"] + 1
    reason = assert_rejected(lambda: validate_source(oversized, policy), "exceeds bounded")
    matrix.append({"case": "oversized-artifact", "result": "REJECTED", "reason": reason})

    validate_redirect(artifact["url"], "https://updates.fixture.invalid/cdn/a", policy["allowed_hosts"])
    matrix.append({"case": "same-host-https-redirect", "result": "PASS"})
    reason = assert_rejected(
        lambda: validate_redirect(artifact["url"], "https://cdn.example.com/a", policy["allowed_hosts"]),
        "not allowlisted",
    )
    matrix.append({"case": "cross-host-redirect", "result": "REJECTED", "reason": reason})

    offset = len(payload) // 2
    resume_state = {
        "artifact_cache_key": key,
        "downloaded_bytes": offset,
        "strong_etag": '"fixture-v1"',
    }
    resume_response = {
        "status": 206,
        "content_range_start": offset,
        "content_range_total": len(payload),
        "strong_etag": '"fixture-v1"',
    }
    validate_resume_response(resume_state, resume_response, artifact)
    matrix.append({"case": "exact-range-resume", "result": "PASS"})

    for label, mutate, expected in (
        ("resume-server-returned-200", lambda r: r.__setitem__("status", 200), "requires HTTP 206"),
        ("resume-range-offset-mismatch", lambda r: r.__setitem__("content_range_start", offset + 1), "start does not match"),
        ("resume-total-mismatch", lambda r: r.__setitem__("content_range_total", len(payload) + 1), "total does not match"),
        ("resume-entity-changed", lambda r: r.__setitem__("strong_etag", '"fixture-v2"'), "entity identity changed"),
    ):
        response = dict(resume_response)
        mutate(response)
        reason = assert_rejected(lambda r=response: validate_resume_response(resume_state, r, artifact), expected)
        matrix.append({"case": label, "result": "REJECTED", "reason": reason})

    chunk_size = 8192
    chunks = [payload[i:i + chunk_size] for i in range(0, len(payload), chunk_size)]
    assert stream_verify(chunks, artifact["sha256"], artifact["bytes"], policy["max_chunk_bytes"])
    matrix.append({"case": "streaming-sha256-byte-count-verification", "result": "PASS"})

    truncated = chunks[:-1]
    reason = assert_rejected(
        lambda: stream_verify(truncated, artifact["sha256"], artifact["bytes"], policy["max_chunk_bytes"]),
        "byte count does not match",
    )
    matrix.append({"case": "truncated-download", "result": "REJECTED", "reason": reason})

    tampered_payload = payload[:-1] + bytes([payload[-1] ^ 1])
    tampered_chunks = [tampered_payload[i:i + chunk_size] for i in range(0, len(tampered_payload), chunk_size)]
    reason = assert_rejected(
        lambda: stream_verify(tampered_chunks, artifact["sha256"], artifact["bytes"], policy["max_chunk_bytes"]),
        "SHA-256 does not match",
    )
    matrix.append({"case": "same-size-tampered-download", "result": "REJECTED", "reason": reason})

    cache = {
        "artifact_cache_key": key,
        "verified_complete": True,
        "size": artifact["bytes"],
        "sha256": artifact["sha256"],
    }
    validate_cache_reuse(cache, artifact)
    matrix.append({"case": "verified-cache-reuse", "result": "PASS"})
    bad_cache = dict(cache)
    bad_cache["sha256"] = "0" * 64
    reason = assert_rejected(lambda: validate_cache_reuse(bad_cache, artifact), "cache digest mismatch")
    matrix.append({"case": "tampered-cache-reuse", "result": "REJECTED", "reason": reason})

    report = {
        "schema_version": 1,
        "package_id": "PKG-04",
        "task_scope": ["04.04"],
        "mode": MODE,
        "source_commit": source_commit,
        "canonical_prerequisite": "04.02:DONE,04.03:DONE",
        "canonical_tasks_active": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "production_evidence_consumed": False,
        "product_updater_mutated": False,
        "network_access_performed": False,
        "fixture_identity": True,
        "verified_metadata_required": True,
        "bounded_download_required": True,
        "resume_identity_bound": True,
        "range_append_safety_required": True,
        "streaming_hash_and_byte_count_required": True,
        "verified_cache_reuse_required": True,
        "cross_host_redirect_fail_closed": True,
        "mutation_lock_during_download_forbidden": True,
        "artifact_execution_during_download_forbidden": True,
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
    (out / "report.json").write_bytes(canonical(report))
    (out / "policy.json").write_bytes(canonical(policy))
    (out / "matrix.json").write_bytes(canonical(matrix))
    print(json.dumps(report, sort_keys=True))
    print("PKG04_DOWNLOAD_RESUME_PREIMPLEMENTATION=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
