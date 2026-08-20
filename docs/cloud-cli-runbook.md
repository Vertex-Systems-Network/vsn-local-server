# Cloud CLI lifecycle runbook — 0.13

VSN can operate an already-authenticated local AWS CLI, Azure CLI or Google Cloud CLI through structured, bounded process execution. The lifecycle remains local/operator-controlled.

## Supported operations

- detect provider CLI
- create one VM
- status
- start
- graceful stop/deallocate
- destroy with explicit `confirm_destroy=true`
- create a snapshot/image with explicit `acknowledge_crash_consistency=true`
- clone from a known snapshot/image with explicit `confirm_new_instance=true` where the provider path is deterministic

Snapshot/image baseline:

- AWS: creates an AMI from an EBS-backed instance.
- Azure: creates a managed snapshot of the VM OS managed disk. Full VM clone orchestration remains unsupported in this baseline.
- Google Cloud: creates a machine image and can create a private-by-default VM from a known machine image.

Creation/clone requests do not contain provider secrets. Authentication comes from the installed provider CLI's existing credential/session configuration.

## Default network posture

Create/clone paths request no public IP where the provider CLI exposes the corresponding option. An operator must still supply valid provider-side private networking, routing, security-group/firewall and access configuration. VSN does not claim that a VM is reachable merely because creation succeeded.

## Safety boundary

Cloud lifecycle commands require local `RemoteManage`. They are not in the signed remote-command allowlist. Arguments are passed as process arguments rather than a shell command string; execution has bounded timeout/stdout/stderr and null stdin. Destroy, snapshot/image creation and clone additionally require their explicit acknowledgement/confirmation flags.

## Still pending

Provider-native SDK/API backends, Azure full clone orchestration, application-consistent snapshots, generalized provider migration/copy workflows, image-building policy, zero-downtime release orchestration, health-driven provider rollback and organization cloud-governance policies remain later work. AWS AMI region-copy and AWS/GCP clone target-location are implemented in 0.16.
