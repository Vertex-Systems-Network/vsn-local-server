# PKG-03 Fast Epoch-1 preimplementation

Status: **NON_ACCEPTANCE_PREIMPLEMENTATION**

This epoch prepares downstream implementation before the external PKG-03 `03.22` production-trust gate clears. It does not change the canonical DAG, task status, package progress, product runtime, installer semantics, or target-system behavior.

## Executable lanes

### 03.23 — provenance/SBOM

`scripts/ci/pkg03-0323-provenance-preimplementation.py` builds deterministic synthetic SBOM, provenance and handoff documents. Every document binds the same four synthetic signed package subjects and exact SHA-256 values. The outputs are explicitly synthetic and cannot consume production signing evidence or authorize `03.23 DONE`.

### 03.24 — VM harness

`scripts/ci/pkg03-0324-vm-harness-preimplementation.py` builds a deterministic synthetic Windows verification matrix covering current-user NSIS, per-machine NSIS and MSI. The fixture covers fresh install, repair/tamper, preserved-user-data uninstall, running-resource and pending-reboot cases. It does not execute a real VM or reboot. Any future real-reboot claim must use persistent same-machine evidence.

### 03.25 — final evidence index

`scripts/ci/pkg03-0325-final-index-preimplementation.py` builds a deterministic synthetic final index with dependency, subject-byte, source-lineage and PKG-04 non-activation invariants. It cannot project `03.25 DONE`, PKG-03 COMPLETE, or PKG-04 activation.

## MSIX / Microsoft Store boundary

The desktop entry point is a current-user Tauri shell that delegates operations through authenticated IPC. The agent owns privileged capabilities including Windows service management and elevated network administration (hosts, local CA and resolver operations). Therefore Epoch-1 treats Store/MSIX as a **current-user UI shell** that requires an external or pre-provisioned authenticated agent. It is not a standalone replacement for the direct NSIS/MSI lane.

`apps/desktop/src-tauri/msix/store-packaging-inputs.json.template` intentionally keeps Partner Center identity and publisher values as placeholders. No production identity, signing key, token, package, Store submission, certification or acceptance is represented by this epoch.

The direct installer lane remains required until a separately reviewed architecture can preserve the privileged agent/service capabilities under the chosen Store distribution model.

## Promotion rule

Fast Epoch evidence may reduce implementation latency only. Final task promotion must rebind to genuine accepted predecessor evidence and pass the original task-specific full gates. Synthetic PASS never becomes production acceptance.
