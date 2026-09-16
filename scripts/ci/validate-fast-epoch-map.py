#!/usr/bin/env python3
"""Validate the machine-readable Fast Development execution map."""
from __future__ import annotations

import argparse
import json
from pathlib import Path

REQUIRED_PREIMPLEMENTATION = {
    "pkg03-0323-provenance": ("03.23", "03.22:DONE"),
    "pkg03-0324-vm": ("03.24", "03.23:DONE"),
    "pkg03-0325-final": ("03.25", "03.24:DONE"),
    "pkg03-msix-store": ("STORE-EXTENSION", None),
}
SECRET_KEY_FRAGMENTS = ("password", "private_key", "private-key", "pfx", "api_token", "access_token")


def scan_no_secret_fields(value: object, path: str = "$") -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            lowered = str(key).lower()
            if any(fragment in lowered for fragment in SECRET_KEY_FRAGMENTS):
                raise AssertionError(f"secret-bearing execution-map field forbidden: {path}.{key}")
            scan_no_secret_fields(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            scan_no_secret_fields(child, f"{path}[{index}]")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--map", required=True)
    parser.add_argument("--expected-main", required=True)
    parser.add_argument("--expected-branch", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    data = json.loads(Path(args.map).read_text(encoding="utf-8"))
    assert data["schema_version"] == 1
    assert data["profile"] == "FAST-DEVELOPMENT-V1"
    assert data["epoch_id"] == "fast-epoch-1"
    assert data["marker"] == "NON_ACCEPTANCE_PREIMPLEMENTATION"
    assert data["canonical_base_main"] == args.expected_main
    assert data["integration_branch"] == args.expected_branch
    assert data["integration_owner"] == "orchestrator"
    assert data["canonical_state"] == {
        "active_package": "PKG-03",
        "active_task": "03.22",
        "ready_tasks": ["03.22"],
        "progress_done": 21,
        "progress_required": 25,
        "progress_percent": 84.0,
    }

    acceptance = data["acceptance_lane"]
    assert acceptance["task_id"] == "03.22"
    assert acceptance["pull_request"] == 231
    assert acceptance["branch"] == "pkg03/0322-authenticode-signing-reconciled-v6"
    assert len(acceptance["head"]) == 40
    assert acceptance["state"] == "DRAFT_EXTERNAL_GATE"
    assert acceptance["mutable_surfaces"]
    assert acceptance["collision_keys"]
    assert acceptance["final_prerequisite"]
    assert acceptance["full_gate"]

    lanes = data["preimplementation_lanes"]
    assert len(lanes) == len(REQUIRED_PREIMPLEMENTATION)
    by_id = {lane["lane_id"]: lane for lane in lanes}
    assert set(by_id) == set(REQUIRED_PREIMPLEMENTATION)

    all_collision_keys: set[str] = set(acceptance["collision_keys"])
    for lane_id, (task_id, blocker) in REQUIRED_PREIMPLEMENTATION.items():
        lane = by_id[lane_id]
        assert lane["task_id"] == task_id
        assert lane["mode"] in {"PARALLEL_SAFE", "COORDINATED_PARALLEL"}
        if blocker is not None:
            assert lane["canonical_blocker"] == blocker
        else:
            assert lane["canonical_blocker"]
        assert lane["mutable_surfaces"]
        assert lane["fast_gate"]
        assert lane["promotion_full_gate"]
        for key in lane["collision_keys"]:
            assert key not in all_collision_keys, f"collision key reused: {key}"
            all_collision_keys.add(key)

    shared = data["shared_single_writer_surfaces"]
    assert ".github/workflows/fast-epoch-gate.yml" in shared
    assert ".ai/changes/FAST-EPOCH-1-EXECUTION-MAP.json" in shared
    assert len(shared) == len(set(shared))

    forbidden = data["forbidden_projections"]
    assert forbidden and all(value is True for value in forbidden.values())
    assert data["integration_full_gate_required_before_main_merge"] is True
    assert data["release_certification_gate_still_required"] is True
    assert data["canonical_state_mutation_authority"] is False
    scan_no_secret_fields(data)

    report = {
        "schema_version": 1,
        "epoch_id": data["epoch_id"],
        "mode": data["marker"],
        "canonical_base_main": data["canonical_base_main"],
        "acceptance_task": acceptance["task_id"],
        "preimplementation_lane_count": len(lanes),
        "collision_key_count": len(all_collision_keys),
        "shared_single_writer_surface_count": len(shared),
        "canonical_state_changed": False,
        "implementation_authority": False,
        "production_evidence_consumed": False,
        "result": "PASS",
    }
    out = Path(args.output)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, sort_keys=True))
    print("FAST_EPOCH_EXECUTION_MAP=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
