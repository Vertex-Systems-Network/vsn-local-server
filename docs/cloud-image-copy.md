# Cloud image copy / target-location — 0.16

AWS clone can target a different region when a compatible image already exists there. `cloud cli-copy-image` adds explicit AWS AMI region-to-region copy with `confirm_copy=true`, after which the returned AMI can be used for clone/create in the target region.

GCP machine-image clone supports an explicit target zone. Azure full clone/cross-region orchestration remains unsupported until disk, NIC, identity and network recreation can be made deterministic.

Cloud infrastructure mutations remain local `RemoteManage` actions and are not added to the signed remote-command mutation allowlist.
