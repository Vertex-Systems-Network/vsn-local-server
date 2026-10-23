#!/usr/bin/env python3
"""Write a CycloneDX 1.5 SBOM and a build provenance record for a personal build.

Pre-implementation for 03.23 (NON_ACCEPTANCE_PREIMPLEMENTATION; canonical blocker 03.22):
it lists every locked Rust crate (Cargo.lock) and Desktop npm package
(apps/desktop/package-lock.json), and records the source commit, workflow run, toolchain
and SHA-256 of each produced file. It reads only lock files and the output directory.

Usage: sbom.py <output-dir>
"""

from __future__ import annotations

import hashlib
import json
import os
import platform
import subprocess
import sys
import tomllib
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def cargo_components() -> list[dict]:
    lock = tomllib.loads((ROOT / "Cargo.lock").read_text(encoding="utf-8"))
    out = []
    for pkg in lock.get("package", []):
        name, version = pkg["name"], pkg["version"]
        local = "source" not in pkg
        out.append({
            "type": "application" if local else "library",
            "name": name,
            "version": version,
            "purl": f"pkg:cargo/{name}@{version}",
            **({"hashes": [{"alg": "SHA-256", "content": pkg["checksum"]}]} if pkg.get("checksum") else {}),
        })
    return out


def npm_components() -> list[dict]:
    lock = json.loads((ROOT / "apps/desktop/package-lock.json").read_text(encoding="utf-8"))
    out = []
    for path, pkg in sorted(lock.get("packages", {}).items()):
        if not path or "version" not in pkg:
            continue
        name = pkg.get("name") or path.split("node_modules/")[-1]
        out.append({
            "type": "library",
            "name": name,
            "version": pkg["version"],
            "purl": f"pkg:npm/{name.replace('@', '%40', 1) if name.startswith('@') else name}@{pkg['version']}",
            "scope": "optional" if pkg.get("dev") else "required",
        })
    return out


def tool_version(*cmd: str) -> str:
    try:
        return subprocess.run(cmd, capture_output=True, text=True, check=False).stdout.strip()
    except OSError:
        return "unavailable"


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    sha = os.environ.get("GITHUB_SHA") or tool_version("git", "-C", str(ROOT), "rev-parse", "HEAD")
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    sbom = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "metadata": {
            "timestamp": now,
            "component": {"type": "application", "name": "vsn-devify", "version": sha},
            "properties": [{"name": "vsn:status", "value": "NON_ACCEPTANCE_PREIMPLEMENTATION (03.23, blocked by 03.22)"}],
        },
        "components": cargo_components() + npm_components(),
    }
    (out / "sbom.cdx.json").write_text(json.dumps(sbom, indent=2) + "\n", encoding="utf-8")

    artifacts = {
        p.name: hashlib.sha256(p.read_bytes()).hexdigest()
        for p in sorted(out.iterdir())
        if p.is_file() and p.name not in {"provenance.json", "SHA256SUMS.txt"}
    }
    provenance = {
        "status": "NON_ACCEPTANCE_PREIMPLEMENTATION",
        "canonical_blocker": "03.22",
        "signed": False,
        "source_commit": sha,
        "repository": os.environ.get("GITHUB_REPOSITORY", "Vertex-Systems-Network/vsn-local-server"),
        "workflow": os.environ.get("GITHUB_WORKFLOW"),
        "run_id": os.environ.get("GITHUB_RUN_ID"),
        "runner": f"{platform.system()} {platform.machine()}",
        "toolchain": {"rustc": tool_version("rustc", "--version"), "node": tool_version("node", "--version")},
        "built_at": now,
        "artifacts_sha256": artifacts,
    }
    (out / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n", encoding="utf-8")
    print(f"SBOM: {len(sbom['components'])} components; provenance for {len(artifacts)} file(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
