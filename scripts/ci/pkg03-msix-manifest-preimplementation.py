#!/usr/bin/env python3
"""Render and validate a deterministic synthetic MSIX manifest/staging plan.

The renderer intentionally exposes fixture mode only. It cannot consume real Partner
Center identity or perform package/sign/store actions, so Fast Epoch evidence cannot
be mistaken for production Store acceptance.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from string import Template
import xml.etree.ElementTree as ET

APPX = "http://schemas.microsoft.com/appx/manifest/foundation/windows10"
RESCAP = "http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", default=".")
    parser.add_argument("--fixture", action="store_true", required=True)
    parser.add_argument("--output-dir", required=True)
    args = parser.parse_args()
    root = Path(args.repo_root)

    inputs_path = root / "apps/desktop/src-tauri/msix/store-packaging-inputs.json.template"
    inputs = json.loads(inputs_path.read_text(encoding="utf-8"))
    assert inputs["identity_name"] == "${PARTNER_CENTER_IDENTITY_NAME}"
    assert inputs["publisher"] == "${PARTNER_CENTER_PUBLISHER}"
    assert inputs["architecture"] == "${STORE_ARCHITECTURE}"
    assert inputs["assets_ready"] is False
    assert inputs["package_build_authority"] is False
    assert inputs["production_ready"] is False
    assert inputs["runtime_boundary"] == "external_or_preprovisioned_authenticated_agent"
    assert inputs["release_executable_source"] == "target/release/VSN Dev Platform.exe"

    template_path = root / inputs["manifest_template"]
    template = Template(template_path.read_text(encoding="utf-8"))
    rendered = template.substitute(
        IDENTITY_NAME="VSN.FastEpochFixture",
        PUBLISHER="CN=VSN Fast Epoch Fixture",
        VERSION=inputs["version"],
        ARCHITECTURE="x64",
    )
    if "PARTNER_CENTER" in rendered or "${" in rendered:
        raise AssertionError("production identity placeholder leaked into rendered fixture manifest")

    manifest_root = ET.fromstring(rendered)
    identity = manifest_root.find(f"{{{APPX}}}Identity")
    assert identity is not None
    assert identity.attrib == {
        "Name": "VSN.FastEpochFixture",
        "Publisher": "CN=VSN Fast Epoch Fixture",
        "Version": inputs["version"],
        "ProcessorArchitecture": "x64",
    }
    application = manifest_root.find(f"{{{APPX}}}Applications/{{{APPX}}}Application")
    assert application is not None
    assert application.attrib["Executable"] == inputs["executable"]
    assert application.attrib["EntryPoint"] == "Windows.FullTrustApplication"
    capability = manifest_root.find(f"{{{APPX}}}Capabilities/{{{RESCAP}}}Capability")
    assert capability is not None and capability.attrib.get("Name") == "runFullTrust"

    out = Path(args.output_dir)
    out.mkdir(parents=True, exist_ok=True)
    manifest_path = out / "AppxManifest.xml"
    manifest_path.write_text(rendered, encoding="utf-8", newline="\n")
    required_assets = list(inputs["required_assets"])
    staging = {
        "schema_version": 1,
        "lane": "msix-store-preimplementation",
        "mode": "NON_ACCEPTANCE_PREIMPLEMENTATION",
        "fixture_identity": True,
        "production_identity_present": False,
        "production_evidence_consumed": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "package_build_performed": False,
        "package_signed": False,
        "store_submission_performed": False,
        "executable_source": inputs["release_executable_source"],
        "executable_destination": inputs["executable"],
        "manifest_destination": "AppxManifest.xml",
        "required_assets": required_assets,
        "assets_ready": False,
        "runtime_boundary": inputs["runtime_boundary"],
        "packaging_tool": inputs["packaging_tool"],
    }
    (out / "staging-plan.json").write_bytes(canonical(staging))
    report = {
        "schema_version": 1,
        "lane": "msix-store-preimplementation",
        "mode": "NON_ACCEPTANCE_PREIMPLEMENTATION",
        "manifest_sha256": hashlib.sha256(manifest_path.read_bytes()).hexdigest(),
        "staging_plan_sha256": hashlib.sha256(canonical(staging)).hexdigest(),
        "fixture_identity": True,
        "production_identity_present": False,
        "assets_ready": False,
        "package_build_performed": False,
        "package_signed": False,
        "store_submission_performed": False,
        "standalone_full_feature_store_package_ready": False,
        "required_runtime_boundary": inputs["runtime_boundary"],
        "deterministic_outputs": True,
    }
    (out / "report.json").write_bytes(canonical(report))
    print(json.dumps(report, sort_keys=True))
    print("PKG03_MSIX_MANIFEST_PREIMPLEMENTATION=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
