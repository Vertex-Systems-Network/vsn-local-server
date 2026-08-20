#!/usr/bin/env python3
"""Offline/source integration gate for VSN 0.32.0. Native compilation remains a separate P30 evidence gate."""
from __future__ import annotations
import json,re,sys,tomllib,xml.etree.ElementTree as ET
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]; errors=[]; counts={"json":0,"cargo":0,"rust":0,"yaml":0,"plist":0,"local_paths":0,"packages":0}
# JSON/contracts/providers
for p in sorted(list((ROOT/'contracts').rglob('*.json'))+list((ROOT/'providers').rglob('*.json'))):
 try: json.loads(p.read_text()); counts['json']+=1
 except Exception as e: errors.append(f'JSON {p.relative_to(ROOT)}: {e}')
# roadmap
try:
 d=json.loads((ROOT/'docs/roadmap-status.json').read_text()); phases=d['phases']; ids={x['id'] for x in phases}; vals=[int(x['completion_percent']) for x in phases]
 if d.get('product_version')!='0.32.0': errors.append('roadmap product_version != 0.32.0')
 if d.get('version')!=1: errors.append('roadmap schema version must be integer 1')
 if abs(float(d.get('overall_completion_exact_percent',-1))-98.9032)>0.0001: errors.append('roadmap exact overall completion must be 98.9032 with 0/21 evidence')
 if float(d.get('source_completion_percent',-1))!=100.0: errors.append('roadmap source completion must be 100.0')
 if float(d.get('certification_completion_percent',-1))!=0.0: errors.append('roadmap certification completion must be 0.0 for bundled evidence')
 if float(d.get('p30_completion_exact_percent',-1))!=66.0: errors.append('roadmap P30 exact completion must be 66.0 for bundled evidence')
 if d.get('stable_release_certified') is not False: errors.append('bundled roadmap must not claim Stable 1.0 certification')
 if len(phases)!=31 or ids!={f'P{i}' for i in range(31)}: errors.append('roadmap must contain exactly P0..P30')
 if any(v<0 or v>100 for v in vals): errors.append('roadmap completion percent outside 0..100')
 if d.get('overall_completion_percent')!=round(sum(vals)/31): errors.append('roadmap overall percentage is not rounded mean')
 for closed in tuple(f'P{i}' for i in range(30)):
  row=next(x for x in phases if x['id']==closed)
  if row['completion_percent']!=100 or row['status']!='done': errors.append(f'{closed} must be source-closed at 100%')
except Exception as e: errors.append(f'roadmap: {e}')
# Cargo + versions + local paths
for p in sorted(ROOT.rglob('Cargo.toml')):
 try:
  d=tomllib.loads(p.read_text()); counts['cargo']+=1
  if 'package' in d:
   counts['packages']+=1; v=d['package'].get('version')
   if v not in ('0.32.0','0.0.0'): errors.append(f'package version drift {p.relative_to(ROOT)}={v}')
  tables=[]
  for k in ('dependencies','dev-dependencies','build-dependencies'):
   if isinstance(d.get(k),dict): tables.append(d[k])
  for target in (d.get('target') or {}).values():
   if isinstance(target,dict):
    for k in ('dependencies','dev-dependencies','build-dependencies'):
     if isinstance(target.get(k),dict): tables.append(target[k])
  for t in tables:
   for name,spec in t.items():
    if isinstance(spec,dict) and 'path' in spec:
     counts['local_paths']+=1; dest=(p.parent/spec['path']).resolve()
     if not dest.exists(): errors.append(f'missing Cargo path {p.relative_to(ROOT)}->{name}')
 except Exception as e: errors.append(f'Cargo {p.relative_to(ROOT)}: {e}')
# Rust delimiter sanity
def strip(src:str)->str:
 out=[];i=0
 while i<len(src):
  if src.startswith('//',i):
   j=src.find('\n',i);i=len(src) if j<0 else j+1;out.append('\n');continue
  if src.startswith('/*',i):
   depth=1;i+=2
   while i<len(src) and depth:
    if src.startswith('/*',i):depth+=1;i+=2
    elif src.startswith('*/',i):depth-=1;i+=2
    else:i+=1
   continue
  m=re.match(r'r(#+)?"',src[i:])
  if m:
   h=m.group(1) or '';i+=len(m.group(0));end='"'+h;j=src.find(end,i);i=len(src) if j<0 else j+len(end);out.append('""');continue
  if src[i]=='"':
   i+=1
   while i<len(src):
    if src[i]=='\\':i+=2;continue
    if src[i]=='"':i+=1;break
    i+=1
   out.append('""');continue
  if src[i]=="'":
   j=i+1
   if j<len(src) and src[j]=='\\': j+=2
   else: j+=1
   if j<len(src) and src[j]=="'": i=j+1;out.append("''");continue
  out.append(src[i]);i+=1
 return ''.join(out)
for p in sorted(ROOT.rglob('*.rs')):
 counts['rust']+=1; clean=strip(p.read_text(errors='ignore')); stack=[]; pairs={')':'(',']':'[','}':'{'}
 for n,ch in enumerate(clean):
  if ch in '([{':stack.append(ch)
  elif ch in pairs:
   if not stack or stack[-1]!=pairs[ch]:errors.append(f'Rust delimiter {p.relative_to(ROOT)} near {n}');break
   stack.pop()
 else:
  if stack: errors.append(f'Rust unclosed delimiter {p.relative_to(ROOT)}')
# YAML/plist
try:
 import yaml
 for p in sorted(list(ROOT.rglob('*.yml'))+list(ROOT.rglob('*.yaml'))): yaml.safe_load(p.read_text());counts['yaml']+=1
except Exception as e: errors.append(f'YAML: {e}')
for p in ROOT.rglob('*.plist'):
 try: ET.parse(p);counts['plist']+=1
 except Exception as e: errors.append(f'plist {p.relative_to(ROOT)}: {e}')
# Product closure anchors
required={
 'crates/vsn-config/src/lib.rs':['CURRENT_CONFIG_VERSION','recover_atomic_config','json.bak','sync_dir'],
 'crates/vsn-ipc/src/lib.rs':['MAX_CONNECTIONS','CONNECTION_TIMEOUT','AtomicUsize','set_read_timeout','set_write_timeout'],
 'crates/vsn-core/src/lib.rs':['pub fn diagnostics','managed_process_remove','container_exec'],
 'crates/vsn-project/src/lib.rs':['BootstrapResult','15 minute timeout','bootstrap destination must be empty','PROJECT_PROVIDER_SDK_VERSION','ProjectProviderConformanceReport','builtin_project_conformance'],
 'crates/vsn-database/src/lib.rs':['DATABASE_SDK_VERSION','ProviderDescriptor','ProviderConformanceReport','list_functions','list_users','list_permissions','import_data','export_data','backup','restore'],
 'apps/cli/src/main.rs':['0.32.0','command_catalog','completion_script','diagnostics','commands'],
 'crates/vsn-control-store/src/lib.rs':['key_id','vsn_control_team_vault_meta','rotate_team_secrets','delete_fleet_group','delete_environment'],
 'cloud/control-plane/src/main.rs':['0.32.0','/v1/admin/vault/rotate','loaded_key_ids','active_key_id','delete_fleet_group','delete_environment','validate_fleet'],
 'cloud/control-plane/src/main.rs':['0.32.0','/v1/admin/control/validate','/v1/admin/iam/validate','/v1/admin/security/validate','validate_control_plane','validate_iam','validate_security'],
 'cloud/dashboard/src/main.tsx':['key_id:string','Rotate all secrets atomically','active key'],
 'crates/vsn-container/src/lib.rs':['ContainerExecRequest','ContainerStats','container_inspect','container_stats','container_exec','restart','pull','build','logs'],
 'crates/vsn-system/src/lib.rs':['list_managed','remove_managed','Command::new("kill")','-TERM','-KILL','ServiceProviderConformanceReport','launchctl','kickstart','SIGTERM'],
 'apps/agent/src/main.rs':['process.managed.remove','container.exec','diagnostics','service.conformance','project.conformance'],
 'contracts/database-sdk-conformance-v1.schema.json':['Database SDK Provider Conformance'],
 'contracts/team-vault-keyring-v1.schema.json':['Team Vault Keyring Rotation'],
 'contracts/container-operations-v1.schema.json':['Container Operations'],
 'contracts/fleet-consistency-v1.schema.json':['Fleet Consistency'],
 'contracts/cli-command-catalog-v1.schema.json':['CLI Command Catalog'],
 'contracts/core-diagnostics-v1.schema.json':['Core Diagnostics'],
 'contracts/service-provider-conformance-v1.schema.json':['Service Provider Conformance'],
 'contracts/project-provider-conformance-v1.schema.json':['Project Provider Conformance'],
 'contracts/control-plane-consistency-v1.schema.json':['Control Plane Consistency'],
 'contracts/control-plane-dr-manifest-v1.schema.json':['Control Plane DR Manifest'],
 'crates/vsn-runtime/src/lib.rs':['RUNTIME_PROVIDER_SDK_VERSION','RuntimeProviderConformanceReport','builtin_provider_conformance'],
 'crates/vsn-database/src/lib.rs':['DatabaseStudioConformanceReport','database_studio_conformance','AdvancedModelRequest','Search','TimeSeries','Column','RemoteDatabaseConformanceReport'],
 'crates/vsn-terminal/src/lib.rs':['TerminalConformanceReport','auto_recreate_after_agent_restart','durable_scrollback'],
 'apps/desktop/src/App.tsx':['DESKTOP_CAPABILITIES','Search / Time-series / Column analyzer','Desktop source coverage'],
 'cloud/dashboard/src/main.tsx':['DASHBOARD_CAPABILITIES','DASHBOARD COVERAGE'],
 'cloud/control-plane/src/main.rs':['/v1/admin/gateway/validate','/v1/admin/auth/federation/validate','federated_logout','unlink_oidc_identity','unlink_saml_identity'],
 'crates/vsn-auth/src/lib.rs':['end_session_endpoint','post_logout_redirect_url','slo_url'],
 'crates/vsn-saml/src/lib.rs':['SamlLogoutStart','create_logout_start'],
 'contracts/runtime-provider-conformance-v1.schema.json':['Runtime'],
 'contracts/database-studio-conformance-v1.schema.json':['Database Studio Conformance'],
 'contracts/remote-database-conformance-v1.schema.json':['Remote Database'],
 'contracts/terminal-conformance-v1.schema.json':['Terminal Conformance'],
 'contracts/desktop-coverage-v1.schema.json':['Desktop Coverage'],
 'contracts/gateway-conformance-v1.schema.json':['Gateway'],
 'contracts/federation-conformance-v1.schema.json':['Federation'],
 'docs/BATCH-0.32.md':['P30 Runner-Pack Execution','98.9032%'],
 'docs/source-closure-0.32.md':['P0–P29','P30'],
 'scripts/control-plane-dr.py':['pg_dump','pg_restore','RESTORE_CONTROL_PLANE'],
 'scripts/source-readiness.py':['SOURCE_CLOSED','P30'],
 'packaging/linux/build-deb.sh':['0.32.0'],
 'docs/STATUS-0.32.md':['98.9032%','0/21','runner packs'],
 '.github/workflows/ci.yml':['validate-batch-0.32.py'],
 '.github/workflows/release-gate.yml':['validate-batch-0.32.py'],
}
for rel,needles in required.items():
 p=ROOT/rel
 if not p.is_file(): errors.append(f'missing integration file {rel}');continue
 text=p.read_text(errors='ignore')
 for needle in needles:
  if needle not in text: errors.append(f'missing integration anchor {needle!r} in {rel}')
# 0.27 source-closure anchors for P23/P25
extra_required={
 'crates/vsn-cloud/src/lib.rs':['CloudArtifactKind','cloud_cli_copy_image','azure_copy_artifact','azure_copy_incremental_snapshot','azure_copy_with_azcopy','cloud_cli_copy_status','cross_location_artifact_copy'],
 'crates/vsn-core/src/lib.rs':['cloud_cli_copy_status'],
 'apps/agent/src/main.rs':['cloud.cli.copy-status'],
 'apps/cli/src/main.rs':['cli-copy-status'],
 'crates/vsn-extension/src/lib.rs':['run_linux_bubblewrap','run_windows_appcontainer','run_macos_app_sandbox','windows_appcontainer','macos_app_sandbox','unsupported_capabilities_fail_closed'],
 'native/windows/vsn-extension-appcontainer.cpp':['CreateAppContainerProfile','PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES','CreateProcessW','--probe'],
 'packaging/windows/build-extension-appcontainer.ps1':['vsn-extension-appcontainer.exe','cl.exe'],
 'packaging/windows/build-msi.ps1':['build-extension-appcontainer.ps1','vsn-extension-appcontainer.exe'],
 'packaging/windows/VSN.wxs':['vsn-extension-appcontainer.exe'],
 'contracts/cloud-artifact-status-v1.schema.json':['Cloud'],
 'contracts/extension-sandbox-capabilities-v1.schema.json':['Sandbox'],
 'contracts/extension-conformance-v1.schema.json':['unsupported_capabilities_fail_closed','host_backend_available'],
 'docs/BATCH-0.32.md':['P30','evidence'],
 'docs/source-closure-0.32.md':['P0–P29','P30'],
 'docs/STATUS-0.32.md':['99%','30/30','P30'],
}
for rel,needles in extra_required.items():
 p=ROOT/rel
 if not p.is_file(): errors.append(f'missing 0.25 closure file {rel}');continue
 text=p.read_text(errors='ignore')
 for needle in needles:
  if needle not in text: errors.append(f'missing 0.25 closure anchor {needle!r} in {rel}')
# CLI dispatch/catalog parity for simple and matches! subcommands
try:
 cli=(ROOT/'apps/cli/src/main.rs').read_text(errors='ignore'); pre=cli.split('fn command_catalog()',1)[0]; cat=cli.split('fn command_catalog()',1)[1].split('fn completion_script',1)[0]
 dispatch={}
 for group,sub in re.findall(r'cmd=="([^"]+)"&&sub=="([^"]+)"',pre): dispatch.setdefault(group,set()).add(sub)
 for group,var,alts in re.findall(r'cmd=="([^"]+)"&&matches!\((\w+)\.as_str\(\),([^\)]*)\)',pre):
  for sub in re.findall(r'"([^"]+)"',alts): dispatch.setdefault(group,set()).add(sub)
 catalog={group:set(re.findall(r'"([^"]+)"',body)) for group,body in re.findall(r'"([a-z0-9_-]+)":\[([^\]]*)\]',cat)}
 for group,subs in dispatch.items():
  missing=subs-catalog.get(group,set())
  if missing: errors.append(f'CLI catalog missing {group} subcommands: {sorted(missing)}')
except Exception as e: errors.append(f'CLI catalog parity: {e}')
# P30 evidence-driven certification anchors
for rel,needles in {
 'scripts/release-evidence.py':['schema_version":4','blocked','max_age_days','allow_waivers'],
 'scripts/certify-local.py':['blocked','rust-linux','deb-install-uninstall','control-load-slo'],
 'scripts/p30-progress.py':['p30_exact','overall_completion_exact_percent','certification_completion_percent'],
 'contracts/release-evidence-v4.schema.json':['candidate_id','blocked','max_age_days','runner_attestation_sha256'],
 'contracts/p30-progress-v1.schema.json':['p30_completion_percent'],
 'docs/release-evidence-current.json':['"schema_version": 4','"product_version": "0.32.0"','"candidate_id"'],
 '.github/workflows/release-gate.yml':['release-evidence-current.json','scripts/p30-fragment.py'],
 '.github/workflows/release-signing.yml':['signing-evidence','0.32.0'],
}.items():
 p=ROOT/rel
 if not p.is_file(): errors.append(f'missing P30 certification file {rel}');continue
 text=p.read_text(errors='ignore')
 for needle in needles:
  if needle not in text: errors.append(f'missing P30 certification anchor {needle!r} in {rel}')
# 0.27 P30 evidence aggregation / provenance anchors
extra_027={
 'scripts/release-evidence.py':['pass_has_provenance','invalid_provenance','Newest evidence wins'],
 'scripts/p30-collect.py':['discover','version mismatch','release-evidence-current.json','p30-progress.py'],
 'scripts/test-release-evidence.py':['invalid_provenance','version mismatch','release-evidence regression tests: PASS'],
 '.github/workflows/p30-aggregate.yml':['gh run download','vsn-release-evidence-ci','vsn-nightly-security-evidence','vsn-signing-evidence','vsn-p30-reviewed-evidence','production-certification'],
 'docs/BATCH-0.32.md':['candidate_id','different source candidates'],
 'docs/STATUS-0.32.md':['30/30','0/21'],
}
for rel,needles in extra_027.items():
 p=ROOT/rel
 if not p.is_file(): errors.append(f'missing 0.27 integration file {rel}'); continue
 text=p.read_text(errors='ignore')
 for needle in needles:
  if needle not in text: errors.append(f'missing 0.27 integration anchor {needle!r} in {rel}')

# 0.28 exact P30 scoreboard anchors
for rel,needles in {
 'scripts/p30-scoreboard.py':['overall_completion_exact_percent','p30_points_per_valid_control','current_runner_status','milestones'],
 'scripts/p30-fastest-path.py':['linux-core','windows-core','macos-core','independent-review'],
 'contracts/p30-scoreboard-v2.schema.json':['release_candidate_id','overall_completion_exact_percent','controls','milestones'],
 'docs/p30-certification-status.json':['"overall_completion_exact_percent": 98.9032','"certification_satisfied": 0'],
 'docs/p30-certification-status.md':['Overall exact','Each valid certification control contributes'],
 'docs/p30-fastest-path.md':['`linux-core`','100.00%'],
 'docs/roadmap-status.json':['"overall_completion_exact_percent": 98.9032','"p30_completion_exact_percent": 66.0'],
}.items():
 p=ROOT/rel
 if not p.is_file(): errors.append(f'missing 0.28 scoreboard file {rel}'); continue
 text=p.read_text(errors='ignore')
 for needle in needles:
  if needle not in text: errors.append(f'missing 0.28 scoreboard anchor {needle!r} in {rel}')

# 0.29 candidate-bound certification anchors
for rel,needles in {
 'scripts/release-candidate.py':['release-inputs-v1','candidate_id','source_fingerprint_sha256'],
 'scripts/release-evidence.py':['schema_version\":4','candidate_bound','candidate mismatch','runner_attestation_sha256'],
 'contracts/release-candidate-v1.schema.json':['candidate_id','source_fingerprint_sha256'],
 'contracts/release-evidence-v3.schema.json':['candidate_id','const\": 3'],
 'contracts/p30-scoreboard-v2.schema.json':['release_candidate_id','const\": 2'],
 'docs/release-candidate-current.json':['candidate_id','source_fingerprint_sha256'],
 'scripts/test-release-candidate.py':['candidate mismatch','release candidate regression PASS'],
 'rust-toolchain.toml':['1.97.0','rustfmt','clippy'],
 '.github/workflows/release-gate.yml':['scripts/p30-fragment.py','toolchain: 1.97.0'],
 '.github/workflows/security-nightly.yml':['scripts/p30-fragment.py'],
 '.github/workflows/release-signing.yml':['scripts/p30-fragment.py'],
 '.github/workflows/p30-evidence-intake.yml':['release-candidate.py id'],
}.items():
 p=ROOT/rel
 if not p.is_file(): errors.append(f'missing 0.29 candidate file {rel}'); continue
 text=p.read_text(errors='ignore')
 for needle in needles:
  if needle not in text: errors.append(f'missing 0.29 candidate anchor {needle!r} in {rel}')
try:
 cand=json.loads((ROOT/'docs/release-candidate-current.json').read_text())
 evid=json.loads((ROOT/'docs/release-evidence-current.json').read_text())
 score=json.loads((ROOT/'docs/p30-certification-status.json').read_text())
 if evid.get('candidate_id')!=cand.get('candidate_id'): errors.append('current evidence candidate does not match release candidate')
 if score.get('release_candidate_id')!=cand.get('candidate_id'): errors.append('P30 scoreboard candidate does not match release candidate')
except Exception as e: errors.append(f'candidate instance validation: {e}')
# 0.31 incremental runner evidence / runner-pack anchors
for rel,needles in {
 'certification/p30-runner-packs.json':['linux-core','windows-core','macos-core','security-nightly','operations','independent-review'],
 'contracts/p30-runner-packs-v1.schema.json':['VSN P30 Runner Packs v1','required_tools','optional_tools','execution_mode'],
 'scripts/p30-fragment.py':['candidate-bound PASS evidence fragment','--controls','release-evidence.py'],
 'scripts/p30-runner-plan.py':['ready_on_current_host','remaining_controls','missing_required_tools'],
 'scripts/test-p30-fragments.py':['p30 evidence fragments PASS','rust-linux','desktop-build'],
 '.github/workflows/release-gate.yml':['vsn-p30-fragment-rust-${{ runner.os }}','vsn-p30-fragment-frontends','vsn-p30-fragment-msi','vsn-p30-fragment-deb','vsn-p30-fragment-pkg','if: ${{ always() }}','pattern: vsn-p30-fragment-*'],
 '.github/workflows/security-nightly.yml':['vsn-p30-fragment-nightly-fuzz','vsn-p30-fragment-nightly-rustsec','pattern: vsn-p30-fragment-nightly-*'],
 '.github/workflows/release-signing.yml':['vsn-p30-fragment-signing-windows','vsn-p30-fragment-signing-macos','pattern: vsn-p30-fragment-signing-*'],
}.items():
 p=ROOT/rel
 if not p.is_file(): errors.append(f'missing 0.31 runner evidence file {rel}'); continue
 text=p.read_text(errors='ignore')
 for needle in needles:
  if needle not in text: errors.append(f'missing 0.31 runner evidence anchor {needle!r} in {rel}')
try:
 packs=json.loads((ROOT/'certification/p30-runner-packs.json').read_text())
 if packs.get('product_version')!='0.32.0': errors.append('runner pack product_version != 0.32.0')
 covered=[]
 for pack in packs.get('packs',[]): covered.extend(pack.get('controls',[]))
 required={
  'rust-windows','rust-linux','rust-macos','desktop-build','dashboard-build','msi-install-uninstall','deb-install-uninstall','pkg-install-uninstall',
  'updater-windows','updater-linux','updater-macos','windows-authenticode','macos-notarization','rustsec-audit','fuzz-remote-protocol','fuzz-stream-open',
  'control-load-slo','ha-failover','dr-restore','vault-key-rotation','penetration-test'
 }
 if set(covered)!=required: errors.append('runner packs must cover exactly all 21 P30 controls once or more')
except Exception as e: errors.append(f'runner pack validation: {e}')
# 0.31 one-command runner-pack execution anchors
for rel,needles in {
 'scripts/p30-pack-preflight.py':['missing_required_tools','missing_environment','ready'],
 'scripts/p30-run-pack.py':['linux_core','windows_core','macos_core','security_pack','external_pack','candidate_id'],
 'scripts/p30-bootstrap-plan.py':['rust_toolchain','linux-core','windows-core','macos-core','security-nightly'],
 'contracts/p30-pack-run-v1.schema.json':['P30 Pack Run Summary v1','candidate_id','results'],
 'contracts/p30-bootstrap-plan-v1.schema.json':['P30 Bootstrap Plan v1','rust_toolchain','commands'],
 '.github/workflows/p30-run-pack.yml':['linux-core','windows-core','macos-core','security-nightly','dist-p30'],
 'packaging/windows/build-msi.ps1':['Get-Content','VERSION'],
 'packaging/macos/build-pkg.sh':['ROOT_REPO','VERSION'],
}.items():
 p=ROOT/rel
 if not p.is_file(): errors.append(f'missing 0.31 pack-execution file {rel}'); continue
 text=p.read_text(errors='ignore')
 for needle in needles:
  if needle not in text: errors.append(f'missing 0.31 pack-execution anchor {needle!r} in {rel}')
# 0.32 runner attestation / portable certification anchors
for rel,needles in {
 'scripts/p30-runner-attest.py':['runner attestation','source_manifest_sha256','tools'],
 'contracts/p30-runner-attestation-v1.schema.json':['P30 Runner Attestation v1','source_manifest_sha256'],
 'contracts/p30-external-command-manifest-v1.schema.json':['External Command Manifest v1','commands'],
 'scripts/p30-run-pack.py':['runner-attestation.json','--external-command-file','load_external_commands','runner_attestation_sha256'],
 'contracts/release-evidence-v4.schema.json':['runner_attestation_sha256','runner_attestation_ref','const": 4'],
 'certification/linux-core.Dockerfile':['rust:1.97.0-bookworm','node:22-bookworm','cargo-audit','cargo-fuzz'],
 'scripts/p30-linux-container.sh':['p30-run-pack.py','linux-core'],
 '.github/workflows/p30-linux-container.yml':['actions/attest@v4','dist-p30-container','linux-core'],
}.items():
 p=ROOT/rel
 if not p.is_file(): errors.append(f'missing 0.32 certification file {rel}'); continue
 text=p.read_text(errors='ignore')
 for needle in needles:
  if needle not in text: errors.append(f'missing 0.32 certification anchor {needle!r} in {rel}')
# no generated dirs
for name in ('target','node_modules','dist','__pycache__'):
 for p in ROOT.rglob(name):
  if p.is_dir(): errors.append(f'generated directory included {p.relative_to(ROOT)}')
print(json.dumps({'ok':not errors,'counts':counts,'errors':errors},indent=2));sys.exit(1 if errors else 0)
