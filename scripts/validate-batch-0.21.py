#!/usr/bin/env python3
"""Offline/source integration gate for VSN 0.21.0. Native compilation remains a separate P30 evidence gate."""
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
 if d.get('product_version')!='0.21.0': errors.append('roadmap product_version != 0.21.0')
 if len(phases)!=31 or ids!={f'P{i}' for i in range(31)}: errors.append('roadmap must contain exactly P0..P30')
 if any(v<0 or v>100 for v in vals): errors.append('roadmap completion percent outside 0..100')
 if d.get('overall_completion_percent')!=round(sum(vals)/31): errors.append('roadmap overall percentage is not rounded mean')
 for closed in ('P0','P1','P2','P7','P11','P20','P22','P24'):
  row=next(x for x in phases if x['id']==closed)
  if row['completion_percent']!=100 or row['status']!='done': errors.append(f'{closed} must be source-closed at 100%')
except Exception as e: errors.append(f'roadmap: {e}')
# Cargo + versions + local paths
for p in sorted(ROOT.rglob('Cargo.toml')):
 try:
  d=tomllib.loads(p.read_text()); counts['cargo']+=1
  if 'package' in d:
   counts['packages']+=1; v=d['package'].get('version')
   if v not in ('0.21.0','0.0.0'): errors.append(f'package version drift {p.relative_to(ROOT)}={v}')
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
 'crates/vsn-project/src/lib.rs':['BootstrapResult','15 minute timeout','bootstrap destination must be empty'],
 'crates/vsn-database/src/lib.rs':['DATABASE_SDK_VERSION','ProviderDescriptor','ProviderConformanceReport','list_functions','list_users','list_permissions','import_data','export_data','backup','restore'],
 'apps/cli/src/main.rs':['0.21.0','command_catalog','completion_script','diagnostics','commands'],
 'crates/vsn-control-store/src/lib.rs':['key_id','vsn_control_team_vault_meta','rotate_team_secrets','delete_fleet_group','delete_environment'],
 'cloud/control-plane/src/main.rs':['0.21.0','/v1/admin/vault/rotate','loaded_key_ids','active_key_id','delete_fleet_group','delete_environment','validate_fleet'],
 'cloud/dashboard/src/main.tsx':['key_id:string','Rotate all secrets atomically','active key'],
 'crates/vsn-container/src/lib.rs':['ContainerExecRequest','ContainerStats','container_inspect','container_stats','container_exec','restart','pull','build','logs'],
 'crates/vsn-system/src/lib.rs':['list_managed','remove_managed','Command::new("kill")','-TERM','-KILL'],
 'apps/agent/src/main.rs':['process.managed.remove','container.exec','diagnostics'],
 'contracts/database-sdk-conformance-v1.schema.json':['Database SDK Provider Conformance'],
 'contracts/team-vault-keyring-v1.schema.json':['Team Vault Keyring Rotation'],
 'contracts/container-operations-v1.schema.json':['Container Operations'],
 'contracts/fleet-consistency-v1.schema.json':['Fleet Consistency'],
 'contracts/cli-command-catalog-v1.schema.json':['CLI Command Catalog'],
 'contracts/core-diagnostics-v1.schema.json':['Core Diagnostics'],
 'docs/BATCH-0.21.md':['close-first','P24 Containers'],
 'docs/STATUS-0.21.md':['89%'],
 '.github/workflows/ci.yml':['validate-batch-0.21.py'],
 '.github/workflows/release-gate.yml':['validate-batch-0.21.py'],
}
for rel,needles in required.items():
 p=ROOT/rel
 if not p.is_file(): errors.append(f'missing integration file {rel}');continue
 text=p.read_text(errors='ignore')
 for needle in needles:
  if needle not in text: errors.append(f'missing integration anchor {needle!r} in {rel}')
# no generated dirs
for name in ('target','node_modules','dist','__pycache__'):
 for p in ROOT.rglob(name):
  if p.is_dir(): errors.append(f'generated directory included {p.relative_to(ROOT)}')
print(json.dumps({'ok':not errors,'counts':counts,'errors':errors},indent=2));sys.exit(1 if errors else 0)
