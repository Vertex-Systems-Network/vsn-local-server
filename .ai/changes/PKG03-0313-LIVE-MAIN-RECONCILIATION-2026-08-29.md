# PKG-03 03.13 Live-Main Reconciliation — 2026-08-29

Task: `03.13 — Firewall, hosts, resolver and trust-store non-mutation boundary`
Linear: `ABD-88`
PR: `#122`

## Classification

This is a **live-canonical reconciliation record**, not a product-scope amendment.

The frozen 03.13 planning bundle was created from historical canonical base `4f5e8ab30f030e758c52c4ca4ac08f73f896247a`. Before resuming acceptance, live canonical `main` was re-read as `0eaa4abb7c5e817334f13672952a5901fbbc8fa9`.

The existing 03.13 branch head `b5dae7b41942f1bd3c04a627a882621c00976216` was 188 commits behind live main. Its historical exact-head 03.13 workflow was green, but that evidence is stale for current acceptance because accepted 03.09, 03.10 and 03.11 installer/runtime behavior has since entered canonical main.

## Contract-preservation check

The protected 03.13 contract remains unchanged:
- observe only Windows Firewall persistent policy, hosts, resolver/NRPT and certificate trust stores;
- compare baseline, post-install and post-uninstall snapshots;
- fail closed on any semantic difference;
- do not repair or mutate protected state;
- keep application launch disabled during installer attribution;
- do not mutate product/Tauri/installer templates, services, ACLs, signing, updater or recovery behavior.

Locked installer inputs were compared between the historical planning base and live main and are byte-identical by Git blob identity:
- `apps/desktop/src-tauri/tauri.conf.json`: `62215d58a5fbf3a0c3098b4cf5c39bea497d1d7a`
- `apps/desktop/src-tauri/tauri.per-machine.conf.json`: `35e184a689e697d0d1e144176f34be8ceb0c3529`
- `installer/windows/owned-payload.v1.json`: `641bbdc4c106fcb36cc232c8a74549e42df1749c`

Therefore no planning goal, protected surface, privilege, data flow, negative requirement or product mutation authority changes.

## Reconciliation mechanics

- Preserve `.ai/manifests/pkg03-0313-installer-nonmutation.v1.json` byte-for-byte as the historical frozen plan binding.
- Preserve all planning artifacts and task-owned harness/workflow files unless a current-source certification defect is demonstrated.
- Reconcile branch ancestry non-destructively by retaining the historical 03.13 head and live-main head as parents of the reconciliation commit.
- Update only `scripts/ci/validate-pkg03-0313.py` so the historical manifest base and the refreshed live execution base are modeled separately.
- Changed-path authority for resumed execution is measured from live main `0eaa4abb7c5e817334f13672952a5901fbbc8fa9`.
- Fresh exact-head governance and genuine Windows installer non-mutation evidence are mandatory; historical run `33100113476` is context only and cannot close 03.13.

## 03.12 canonical-integration reconciliation

Canonical `03.12` was subsequently integrated through PR `#139`, advancing live `main` to `b4fe7d07503b13ba0f3d2fcd1741a40163086de7` and canonical PKG-03 state to `12/25 = 48%`, cursor `03.13`.

The previously accepted 03.13 exact-head evidence at `6329a4b55d82a3f0cb9c12469b629b0b21778c8b` is therefore historical acceptance context only for integration. It cannot be reused as exact-head evidence after the canonical tree changed.

The 03.13 branch is reconciled again by:
- composing the exact 03.13 task-owned files onto canonical tree `b4fe7d07503b13ba0f3d2fcd1741a40163086de7`;
- preserving both the previous accepted 03.13 head and new canonical main as reconciliation ancestry;
- updating only this reconciliation record and the validator live-execution-base binding for the new canonical SHA;
- leaving the frozen planning bundle, protected-state collector, lifecycle harness and workflow contract unchanged;
- prohibiting product/Tauri/installer/service/ACL/firewall/hosts/DNS/trust/signing/updater/recovery mutation.

Fresh exact-head governance and a fresh genuine Windows 03.13 lifecycle are mandatory on the resulting branch head before branch evidence can be re-accepted or integrated.

## Acceptance firewall

03.13 remains `In Progress` until a fresh current-main-composed exact head passes required governance, all three genuine installer lifecycles, protected-state equality, exact evidence binding and zero tracked repository drift. Canonical accepted state must not be projected from historical evidence.
