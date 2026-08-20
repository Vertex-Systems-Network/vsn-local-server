# VSN Dev Platform 0.32 — P30 Runner Attestation & Portable Certification

P0–P29 remain 100% source-closed. P30 remains evidence-driven.

## 0.32 changes
- Local/self-hosted PASS evidence now requires a hashed runner attestation in addition to a result artifact.
- Runner attestation records candidate ID, OS/arch, source-manifest digest and relevant tool versions without secrets.
- External operations/reviewer packs use candidate-bound direct argv manifests; shell-string execution was removed.
- `certification/` is now part of the release-candidate fingerprint, so runner-pack manifests and certification Dockerfiles cannot drift without changing candidate ID.
- Added a Rust 1.97.0 + Node 22 Linux certification container and manual GitHub workflow with signed GitHub artifact provenance (`actions/attest@v4`).
- Release Evidence schema advanced to v4 with runner-attestation fields.

Current numerical status remains evidence-driven: 0/21 valid PASS controls => P30 66.00%, overall exact 98.9032%. No certification percentage is awarded for these source-side trust improvements.

## Compatibility anchors
P30 Runner-Pack Execution remains the operational model. Every pack emits `candidate_id`-bound evidence, and evidence from different source candidates is rejected rather than merged.
