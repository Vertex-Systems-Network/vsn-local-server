# P30 Governance Operations

```bash
python scripts/p30-evidence-governance.py list
python scripts/p30-evidence-governance.py verify
python scripts/p30-evidence-governance.py rebuild --dry-run
python scripts/p30-evidence-governance.py quarantine --bundle-sha <sha> --reason "incident review"
python scripts/p30-evidence-governance.py unquarantine --bundle-sha <sha>
python scripts/p30-evidence-governance.py revoke --bundle-sha <sha> --reason "invalid evidence"
python scripts/p30-evidence-governance.py restore --bundle-sha <sha>
python scripts/p30-evidence-governance.py supersede --bundle-sha <old> --replacement-sha <active-new> --reason "new certification"
python scripts/p30-evidence-governance.py checkpoint --path dist-p30/p30-checkpoint.zip
python scripts/p30-evidence-governance.py restore-checkpoint --path dist-p30/p30-checkpoint.zip --dry-run
python scripts/p30-evidence-policy.py --warning-days 2
```
