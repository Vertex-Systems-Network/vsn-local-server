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
    "pkg04-updater-recovery-preimplementation": ("PKG-04-PREIMPLEMENTATION", "PKG-03:COMPLETE"),
}
SECRET_KEY_FRAGMENTS = (
    "password",
    "private_key",
    "private-key",
    "pfx",
    "api_token",
    "access_token",
    "oidc_token",
    "client_secret",
)


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


def require_surfaces(lane: dict, label: str, surfaces: tuple[str, ...]) -> None:
    for required_surface in surfaces:
        assert required_surface in lane["mutable_surfaces"], f"{label} lane surface missing: {required_surface}"


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

    provenance = by_id["pkg03-0323-provenance"]
    require_surfaces(provenance, "03.23", (
        "scripts/ci/pkg03-0323-provenance-preimplementation.py",
        "scripts/ci/pkg03-0323-activation-preflight.py",
        ".github/workflows/fast-epoch-0323-activation.yml",
    ))
    assert "production-handoff activation validator" in provenance["fast_gate"]

    vm_lane = by_id["pkg03-0324-vm"]
    require_surfaces(vm_lane, "03.24", (
        "scripts/ci/pkg03-0324-vm-harness-preimplementation.py",
        "scripts/ci/pkg03-0324-activation-preflight.py",
        ".github/workflows/fast-epoch-0324-activation.yml",
    ))
    assert "provenance-handoff activation validator" in vm_lane["fast_gate"]

    final_lane = by_id["pkg03-0325-final"]
    require_surfaces(final_lane, "03.25", (
        "scripts/ci/pkg03-0325-final-index-preimplementation.py",
        "scripts/ci/pkg03-0325-activation-preflight.py",
        ".github/workflows/fast-epoch-0325-activation.yml",
    ))
    assert "VM-handoff activation validator" in final_lane["fast_gate"]

    msix = by_id["pkg03-msix-store"]
    require_surfaces(msix, "MSIX", (
        "scripts/ci/pkg03-msix-fixture-package.ps1",
        "scripts/ci/pkg03-msix-wack-preflight.ps1",
        "scripts/ci/pkg03-msix-test-install.ps1",
        ".github/workflows/fast-epoch-msix-fixture.yml",
        "apps/desktop/src-tauri/msix/**",
    ))
    assert "test-sign/register/query/remove lifecycle" in msix["fast_gate"]

    pkg04 = by_id["pkg04-updater-recovery-preimplementation"]
    require_surfaces(pkg04, "PKG-04 preimplementation", (
        "scripts/ci/pkg04-updater-recovery-preimplementation.py",
        "scripts/ci/pkg04-updater-adversarial-preimplementation.py",
        "scripts/ci/pkg04-activation-preflight.py",
        "scripts/ci/pkg04-metadata-trust-preimplementation.py",
        "scripts/ci/pkg04-helper-authority-preimplementation.py",
        "scripts/ci/pkg04-download-resume-preimplementation.py",
        "scripts/ci/pkg04-package-transaction-preimplementation.py",
        "scripts/ci/pkg04-negative-matrix-preimplementation.py",
        "scripts/ci/pkg04-update-ux-preimplementation.py",
        "scripts/ci/pkg04-final-handoff-preimplementation.py",
        ".github/workflows/fast-epoch-pkg04-preimplementation.yml",
        ".github/workflows/fast-epoch-pkg04-adversarial.yml",
        ".github/workflows/fast-epoch-pkg04-activation.yml",
        ".github/workflows/fast-epoch-pkg04-metadata-trust.yml",
        ".github/workflows/fast-epoch-pkg04-helper-authority.yml",
        ".github/workflows/fast-epoch-pkg04-download-resume.yml",
        ".github/workflows/fast-epoch-pkg04-package-transaction.yml",
        ".github/workflows/fast-epoch-pkg04-negative-matrix.yml",
        ".github/workflows/fast-epoch-pkg04-update-ux.yml",
        ".github/workflows/fast-epoch-pkg04-final-handoff.yml",
    ))
    assert pkg04["canonical_blocker"] == "PKG-03:COMPLETE"
    assert "six-phase transaction journal model" in pkg04["fast_gate"]
    assert "owner-token lock release" in pkg04["fast_gate"]
    assert "rollback hash/size identity binding" in pkg04["fast_gate"]
    assert "Windows durability/ACL proof requirement" in pkg04["fast_gate"]
    assert "mutation-time parent identity binding" in pkg04["fast_gate"]
    assert "non-reparse control-root contract" in pkg04["fast_gate"]
    assert "target-digest backup/failed-current namespacing" in pkg04["fast_gate"]
    assert "exclusive state-temp creation" in pkg04["fast_gate"]
    assert "PKG-03 final-acceptance handoff eligibility validator" in pkg04["fast_gate"]
    assert "without activating PKG-04" in pkg04["fast_gate"]
    assert "fixture-only 04.02/04.03 metadata/trust policy" in pkg04["fast_gate"]
    assert "fixture-only 04.04 bounded discovery/download/resume/cache contract" in pkg04["fast_gate"]
    assert "exact HTTP 206 Content-Range resume identity" in pkg04["fast_gate"]
    assert "streaming SHA-256 with bounded memory" in pkg04["fast_gate"]
    assert "no mutation lock during download" in pkg04["fast_gate"]
    assert "fixture-only 04.05/04.11 helper bootstrap/invocation authority" in pkg04["fast_gate"]
    assert "verified caller process and signature identity" in pkg04["fast_gate"]
    assert "secure launch-channel binding" in pkg04["fast_gate"]
    assert "stable structured failure codes" in pkg04["fast_gate"]
    assert "fixture-only 04.06-04.10 package transaction coordinator" in pkg04["fast_gate"]
    assert "Agent/CLI/Desktop/updater-helper" in pkg04["fast_gate"]
    assert "single-release committed-state coherence" in pkg04["fast_gate"]
    assert "component-boundary fault restoration" in pkg04["fast_gate"]
    assert "restart only after coherent terminal state" in pkg04["fast_gate"]
    assert "fixture-only 04.12/04.13 Desktop/CLI operator UX contract" in pkg04["fast_gate"]
    assert "existing low-level CLI/Agent update primitives" in pkg04["fast_gate"]
    assert "substrate rather than competing trust authority" in pkg04["fast_gate"]
    assert "stable machine-readable CLI results" in pkg04["fast_gate"]
    assert "activation-time Desktop bridge reconciliation" in pkg04["fast_gate"]
    assert "fixture-only 04.14/04.15 integrated eligibility and negative matrix" in pkg04["fast_gate"]
    assert "offline/partial/corrupt/tampered download" in pkg04["fast_gate"]
    assert "live-owner lock" in pkg04["fast_gate"]
    assert "recover to one coherent accepted release" in pkg04["fast_gate"]
    assert "fixture-only 04.16-04.18 genuine-evidence handoff schema" in pkg04["fast_gate"]
    assert "production acceptance mode with fixture evidence forbidden" in pkg04["fast_gate"]
    assert "real persistent Windows execution" in pkg04["fast_gate"]
    assert "release checksums/SBOM/provenance plus PKG-05 handoff without activation" in pkg04["fast_gate"]
    assert "fresh-state exact-head 04.18 gate" in pkg04["fast_gate"]
    assert "all 04.02-04.17 DONE" in pkg04["fast_gate"]
    assert "forbidding self-completion projection" in pkg04["fast_gate"]

    shared = data["shared_single_writer_surfaces"]
    assert ".github/workflows/fast-epoch-gate.yml" in shared
    assert ".ai/changes/FAST-EPOCH-1-EXECUTION-MAP.json" in shared
    assert len(shared) == len(set(shared))

    assert data["fast_gate_workflow"] == ".github/workflows/fast-epoch-gate.yml"
    assert data["targeted_windows_gate_workflow"] == ".github/workflows/fast-epoch-msix-fixture.yml"
    assert data["targeted_0323_activation_gate_workflow"] == ".github/workflows/fast-epoch-0323-activation.yml"
    assert data["targeted_0324_activation_gate_workflow"] == ".github/workflows/fast-epoch-0324-activation.yml"
    assert data["targeted_0325_activation_gate_workflow"] == ".github/workflows/fast-epoch-0325-activation.yml"
    assert data["targeted_pkg04_preimplementation_gate_workflow"] == ".github/workflows/fast-epoch-pkg04-preimplementation.yml"
    assert data["targeted_pkg04_adversarial_gate_workflow"] == ".github/workflows/fast-epoch-pkg04-adversarial.yml"
    assert data["targeted_pkg04_activation_gate_workflow"] == ".github/workflows/fast-epoch-pkg04-activation.yml"
    assert data["targeted_pkg04_metadata_trust_gate_workflow"] == ".github/workflows/fast-epoch-pkg04-metadata-trust.yml"
    assert data["targeted_pkg04_helper_authority_gate_workflow"] == ".github/workflows/fast-epoch-pkg04-helper-authority.yml"
    assert data["targeted_pkg04_download_resume_gate_workflow"] == ".github/workflows/fast-epoch-pkg04-download-resume.yml"
    assert data["targeted_pkg04_package_transaction_gate_workflow"] == ".github/workflows/fast-epoch-pkg04-package-transaction.yml"
    assert data["targeted_pkg04_negative_matrix_gate_workflow"] == ".github/workflows/fast-epoch-pkg04-negative-matrix.yml"
    assert data["targeted_pkg04_update_ux_gate_workflow"] == ".github/workflows/fast-epoch-pkg04-update-ux.yml"
    assert data["targeted_pkg04_final_handoff_gate_workflow"] == ".github/workflows/fast-epoch-pkg04-final-handoff.yml"
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
        "targeted_windows_gate_bound": True,
        "targeted_0323_activation_gate_bound": True,
        "targeted_0324_activation_gate_bound": True,
        "targeted_0325_activation_gate_bound": True,
        "targeted_pkg04_preimplementation_gate_bound": True,
        "targeted_pkg04_adversarial_gate_bound": True,
        "targeted_pkg04_activation_gate_bound": True,
        "targeted_pkg04_metadata_trust_gate_bound": True,
        "targeted_pkg04_helper_authority_gate_bound": True,
        "targeted_pkg04_download_resume_gate_bound": True,
        "targeted_pkg04_package_transaction_gate_bound": True,
        "targeted_pkg04_negative_matrix_gate_bound": True,
        "targeted_pkg04_update_ux_gate_bound": True,
        "targeted_pkg04_final_handoff_gate_bound": True,
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
