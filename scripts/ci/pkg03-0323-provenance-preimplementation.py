#!/usr/bin/env python3
"""Synthetic-only provenance/SBOM preimplementation for PKG-03 03.23."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

FILES = (
    "nsis-current-user.exe",
    "nsis-per-machine.exe",
    "vsn-platform.msi",
    "VSN Dev Platform.exe",
)


def sha256_text(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def canonical_bytes(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def digest(value: object) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def signed_subjects() -> list[dict]:
    return [
        {
            "file_name": name,
            "sha256": sha256_text(f"fast-epoch1:signed:{name}"),
            "signer_subject": "CN=SYNTHETIC-PREIMPLEMENTATION",
            "timestamp_verified": True,
        }
        for name in FILES
    ]


def build_bundle() -> tuple[dict, dict, dict]:
    source = "6" * 40
    subjects = signed_subjects()
    sbom = {
        "schema_version": 1,
        "document_type": "synthetic-sbom",
        "task_id": "03.23",
        "source_commit": source,
        "synthetic_preimplementation": True,
        "components": [
            {
                "name": row["file_name"],
                "type": "file",
                "hashes": [{"algorithm": "SHA-256", "value": row["sha256"]}],
            }
            for row in subjects
        ],
    }
    provenance = {
        "schema_version": 1,
        "document_type": "synthetic-provenance",
        "task_id": "03.23",
        "source_commit": source,
        "synthetic_preimplementation": True,
        "builder": {"id": "vsn-fast-epoch-1-fixture"},
        "subjects": [{"name": row["file_name"], "sha256": row["sha256"]} for row in subjects],
        "sbom_sha256": digest(sbom),
    }
    handoff = {
        "schema_version": 1,
        "task_id": "03.23",
        "mode": "NON_ACCEPTANCE_PREIMPLEMENTATION",
        "source_commit": source,
        "production_evidence_consumed": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "subjects": subjects,
        "sbom_sha256": digest(sbom),
        "provenance_sha256": digest(provenance),
    }
    return sbom, provenance, handoff


def validate(sbom: dict, provenance: dict, handoff: dict) -> None:
    assert handoff["mode"] == "NON_ACCEPTANCE_PREIMPLEMENTATION"
    assert handoff["production_evidence_consumed"] is False
    assert handoff["canonical_state_changed"] is False
    assert handoff["implementation_authority"] is False
    assert sbom["synthetic_preimplementation"] is True
    assert provenance["synthetic_preimplementation"] is True
    assert sbom["source_commit"] == provenance["source_commit"] == handoff["source_commit"]
    assert handoff["sbom_sha256"] == digest(sbom)
    assert handoff["provenance_sha256"] == digest(provenance)

    expected = {row["file_name"]: row["sha256"] for row in handoff["subjects"]}
    assert set(expected) == set(FILES)
    sbom_map = {row["name"]: row["hashes"][0]["value"] for row in sbom["components"]}
    provenance_map = {row["name"]: row["sha256"] for row in provenance["subjects"]}
    assert expected == sbom_map == provenance_map
    assert all(row["timestamp_verified"] is True for row in handoff["subjects"])
    assert all(row["signer_subject"] == "CN=SYNTHETIC-PREIMPLEMENTATION" for row in handoff["subjects"])


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true", required=True)
    parser.add_argument("--output-dir", required=True)
    args = parser.parse_args()

    sbom, provenance, handoff = build_bundle()
    validate(sbom, provenance, handoff)
    out = Path(args.output_dir)
    out.mkdir(parents=True, exist_ok=True)
    (out / "synthetic-sbom.json").write_bytes(canonical_bytes(sbom))
    (out / "synthetic-provenance.json").write_bytes(canonical_bytes(provenance))
    (out / "synthetic-handoff.json").write_bytes(canonical_bytes(handoff))
    report = {
        "schema_version": 1,
        "task_id": "03.23",
        "mode": "NON_ACCEPTANCE_PREIMPLEMENTATION",
        "production_evidence_consumed": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "subject_count": len(FILES),
        "sbom_sha256": handoff["sbom_sha256"],
        "provenance_sha256": handoff["provenance_sha256"],
        "handoff_sha256": digest(handoff),
        "deterministic_outputs": True,
    }
    (out / "report.json").write_bytes(canonical_bytes(report))
    print(json.dumps(report, sort_keys=True))
    print("PKG03_0323_PREIMPLEMENTATION=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
