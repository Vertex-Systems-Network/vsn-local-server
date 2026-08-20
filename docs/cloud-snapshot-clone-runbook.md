# Cloud snapshot / clone baseline — VSN 0.13

The local authenticated Agent exposes provider-CLI-backed snapshot and selected clone operations. Cloud credentials are not accepted in command JSON; the installed provider CLI uses its existing authenticated context.

## Safety controls

- Snapshot requires `acknowledge_crash_consistency: true` because application-level quiescing is not yet orchestrated.
- Clone requires `confirm_new_instance: true`.
- All provider values use structured argv execution, bounded runtime/output and validated identifiers; no user shell interpolation is used.
- Cloud mutation commands remain outside the signed remote Agent allowlist and require local `RemoteManage`.

## Provider baseline

- AWS: create an EBS-backed AMI from an existing instance; clone launches a new private-by-default instance from an image ID.
- Azure: create a managed OS-disk snapshot. Full VM clone is intentionally unsupported in this batch because deterministic disk/NIC/network recreation is not yet implemented.
- GCP: create a machine image; clone creates a no-external-address VM from the machine image.

These are infrastructure snapshots/images, not guaranteed application-consistent backups. Database quiescing, multi-disk coordination, cross-region migration and provider-native SDK orchestration remain later work.
