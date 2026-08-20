# P30 certification handoff

Build a portable, candidate-bound pack bundle after source freeze:

```bash
python scripts/p30-handoff.py --pack linux-core --output-dir dist-p30-handoff
```

The bundle is built only from `SOURCE_SHA256SUMS.txt` entries, verifies every source hash before packaging, embeds `P30_HANDOFF.json`, and emits a SHA-256 sidecar.

For GitHub-hosted execution:

```bash
python scripts/p30-gh-dispatch.py --pack linux-core --repo OWNER/REPO
```

The helper dispatches `p30-run-pack.yml`, watches the run, downloads that run's pack artifact, and merges the candidate-bound evidence. A failed overall workflow can still yield valid partial evidence.

For recovery reruns, pass a prior same-candidate ledger:

```bash
python scripts/p30-run-pack.py --pack linux-core --resume-ledger previous/evidence.json
```

Valid prior PASS controls are protected from downgrade; failed, blocked, expired, or missing controls remain eligible for rerun.
