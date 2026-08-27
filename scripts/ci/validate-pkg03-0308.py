#!/usr/bin/env python3
from __future__ import annotations

import hashlib, json, re, subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
M = ROOT / '.ai/manifests/pkg03-0308-msi-enterprise.v1.json'
TRACKER = ROOT / 'certification/pkg03-windows-installer-v1.json'
STATUS = ROOT / 'docs/MASTER-EXECUTION-STATUS.json'
TAURI = ROOT / 'apps/desktop/src-tauri/tauri.conf.json'
MACHINE = ROOT / 'apps/desktop/src-tauri/tauri.per-machine.conf.json'
OWNERSHIP = ROOT / 'installer/windows/owned-payload.v1.json'
HARNESS = ROOT / 'scripts/ci/pkg03-0308-interactive-msi.ps1'
WORKFLOW = ROOT / '.github/workflows/pkg03-0308-msi-enterprise.yml'

BASE='0ac71c6392c19ad070a9ec442323c46f3c0e08b9'
PLAN_SHA='9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e'
TAURI_SHA='172cf6110e58a15442bcf97e9db6a8bdbeb6cbfd2f631d91a3031603ed474180'
MACHINE_SHA='48fd4eb22ffe99a884ce5f4770de83e29ad919650d7c254b5d180fca3add7429'
OWNERSHIP_SHA='5292a1ec1ae0a48d76f80258c9e00ba17b1466dab5c5e0cdda60caf4658dabd1'
UPGRADE='157f304f-1d1b-55e0-b89c-0610ea27c645'
PRE_READY=['03.08','03.09','03.10']
POST_READY=['03.09','03.10','03.13','03.15']
ALLOWED={
 '.ai/features/pkg03-0308/research.md','.ai/features/pkg03-0308/lifecycle-review.md',
 '.ai/features/pkg03-0308/development-preflight.md','.ai/plans/pkg03-0308-msi-enterprise-v1.md',
 '.ai/manifests/pkg03-0308-msi-enterprise.v1.json','docs/PKG03-MSI-WIX-ENTERPRISE-LIFECYCLE-V1.md',
 'scripts/ci/validate-pkg03-0308.py','scripts/ci/pkg03-0308-interactive-msi.ps1',
 '.github/workflows/pkg03-0308-msi-enterprise.yml','certification/pkg03-windows-installer-v1.json',
 'docs/MASTER-EXECUTION-STATUS.json'}

def fail(s): raise SystemExit('PKG-03 03.08 validation failed: '+s)
def gb(path, ref='HEAD'):
 p=subprocess.run(['git','show',f'{ref}:{path}'],cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 if p.returncode: fail(f'unable to read {ref}:{path}')
 return p.stdout
def h(b): return hashlib.sha256(b).hexdigest()
def changed():
 p=subprocess.run(['git','diff','--name-only',f'{BASE}..HEAD'],cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 if p.returncode: fail('unable to compare canonical base')
 return {x.strip() for x in p.stdout.splitlines() if x.strip()}

def main():
 m=json.loads(M.read_text(encoding='utf-8')); t=json.loads(TRACKER.read_text(encoding='utf-8'))
 s=json.loads(STATUS.read_text(encoding='utf-8')); tc=json.loads(TAURI.read_text(encoding='utf-8'))
 mc=json.loads(MACHINE.read_text(encoding='utf-8')); own=json.loads(OWNERSHIP.read_text(encoding='utf-8'))
 if (m.get('feature_id'),m.get('task_id'),m.get('linear_issue'),m.get('version'),m.get('status')) != ('pkg03-0308-msi-enterprise','03.08','ABD-83','1.0.0','frozen'): fail('task identity/version/status mismatch')
 if m.get('canonical_base_sha')!=BASE or m.get('parent_plan',{}).get('sha256')!=PLAN_SHA or h(gb('.ai/plans/pkg03-windows-installer-v1.md'))!=PLAN_SHA: fail('canonical base/parent plan drifted')
 for label,obj in [('research',m['research']),('lifecycle',m['lifecycle']),('preflight',m['development_preflight']),('task plan',m['task_plan']),('contract',m['lifecycle_contract'])]:
  p=obj.get('path') or obj.get('artifact')
  if not p or h(gb(p))!=obj.get('sha256'): fail(label+' digest mismatch')
 if m['research'].get('change_required') is not False: fail('material research delta unresolved')
 extra=changed()-ALLOWED
 if extra: fail('out-of-scope changed paths: '+str(sorted(extra)))
 for p,d in [('apps/desktop/src-tauri/tauri.conf.json',TAURI_SHA),('apps/desktop/src-tauri/tauri.per-machine.conf.json',MACHINE_SHA),('installer/windows/owned-payload.v1.json',OWNERSHIP_SHA)]:
  b=gb(p)
  if b!=gb(p,BASE) or h(b)!=d: fail('accepted source drifted: '+p)
 if tc.get('productName')!='VSN Dev Platform' or tc.get('mainBinaryName')!='VSN Dev Platform' or tc.get('version')!='0.38.1' or tc.get('identifier')!='dev.vsn.platform': fail('product identity drifted')
 bun=tc.get('bundle',{}); win=bun.get('windows',{}); wix=win.get('wix',{})
 if bun.get('publisher')!='Vertex Systems Network' or win.get('allowDowngrades') is not False or wix.get('upgradeCode','').lower()!=UPGRADE: fail('WiX identity contract drifted')
 if win.get('nsis',{}).get('installMode')!='currentUser' or mc.get('bundle',{}).get('windows',{}).get('nsis',{}).get('installMode')!='perMachine': fail('accepted NSIS scope drifted')
 if 'externalBin' in bun or 'resources' in bun: fail('03.10 placement authority widened')
 paths=[x.get('relative_path') for x in own.get('owned_files',[])]
 if paths!=['VSN Dev Platform.exe','bin/vsn.exe','bin/vsn-agent.exe']: fail('owned payload set drifted')
 a=m['authority']
 if a.get('msi_execution_allowed_after_planning_gates') is not True: fail('MSI execution authority missing')
 for k in ['planning_product_mutation_allowed','custom_wix_template_allowed','tauri_config_mutation_allowed','shortcut_semantics_claim_allowed','cli_agent_real_placement_allowed','service_registration_allowed','acl_mutation_allowed','silent_or_passive_deployment_allowed','signing_secret_access_allowed','updater_mutation_allowed','delegated_scope_may_expand']:
  if a.get(k) is not False: fail('authority widened: '+k)
 ac=m['acceptance']; ms=ac['msi_contract']
 if ac.get('runner')!='windows-2025' or ac.get('evidence_artifact')!='pkg03-0308-msi-enterprise': fail('runner/artifact drifted')
 if ms.get('install_scope')!='perMachine' or ms.get('install_command_shape')!='msiexec /i <exact-msi>' or ms.get('uninstall_identity')!='exact ProductCode/package': fail('MSI command/scope contract drifted')
 if ms.get('visible_ui_required') is not True or ms.get('product_code_runtime_extraction_required') is not True or ms.get('blanket_hkcu_nonmutation_claim_allowed') is not False: fail('MSI evidence/nonclaim contract drifted')
 if ms.get('forbidden_ui_suppression')!=['/quiet','/passive','/qn','/qb','/qr','/qf'] or ms.get('arp_registry_root')!='HKLM' or '{ProductCode}' not in ms.get('arp_key_shape',''): fail('UI/ARP contract drifted')
 if not HARNESS.is_file() or not WORKFLOW.is_file(): fail('certification surface incomplete')
 harness=HARNESS.read_text(encoding='utf-8'); hn=harness.replace('\\','/')
 for token in ['WindowsInstaller.Installer','ProductCode','UpgradeCode','msiexec.exe',"'/i'", "'/x'",'UIAutomationClient','ProgramFiles','HKLM','VSN Dev Platform.exe','bin/vsn.exe','bin/vsn-agent.exe','visible_install_ui_observed','visible_uninstall_ui_observed','arp_product_code_key_observed']:
  if token not in hn: fail('interactive MSI harness missing token: '+token)
 for line in re.findall(r'Start-Process[^\r\n]+',harness,flags=re.I):
  for token in ['/quiet','/passive','/qn','/qb','/qr','/qf']:
   if token.lower() in line.lower(): fail('forbidden MSI UI suppression in launch: '+token)
 wf=WORKFLOW.read_text(encoding='utf-8')
 for token in ['windows-2025','22.12.0','1.97.1','build --bundles msi','pkg03-0308-msi-enterprise',TAURI_SHA,MACHINE_SHA,OWNERSHIP_SHA]:
  if token not in wf: fail('workflow missing frozen token: '+token)
 tasks={x['id']:x for x in t.get('tasks',[])}
 if list(tasks)!=[f'03.{i:02d}' for i in range(1,26)] or t.get('required')!=25: fail('denominator/order drifted')
 for i in range(1,8):
  if tasks[f'03.{i:02d}'].get('status')!='DONE': fail(f'completed task regressed: 03.{i:02d}')
 if tasks['03.08'].get('depends_on')!=['03.02','03.03','03.04','03.05']: fail('03.08 dependencies drifted')
 state=tasks['03.08'].get('status')
 if state=='READY': done,ready,cursor=7,PRE_READY,'03.08'
 elif state=='DONE':
  done,ready,cursor=8,POST_READY,'03.09'; ev=tasks['03.08'].get('evidence')
  if not isinstance(ev,dict): fail('accepted task missing evidence')
  for k in ['source_commit','workflow_run','job','artifact','artifact_digest','evidence_sha256','msi_sha256','product_code']:
   if not ev.get(k): fail('accepted evidence missing '+k)
 else: fail('unexpected 03.08 state: '+str(state))
 if t.get('done')!=done or float(t.get('percent',-1))!=done*4.0 or t.get('active_task')!=cursor or t.get('ready_tasks')!=ready: fail('tracker progress/cursor/READY mismatch')
 for x in ready:
  if tasks[x].get('status')!='READY': fail('READY task missing: '+x)
 pkg={x['id']:x for x in s.get('packages',[])}.get('PKG-03',{})
 if s.get('active_package')!='PKG-03' or s.get('active_task')!=cursor or pkg.get('done')!=done or pkg.get('required')!=25 or float(pkg.get('percent',-1))!=done*4.0: fail('master state mismatch')
 print(json.dumps({'valid':True,'task':'03.08','state':state,'done':done,'cursor':cursor,'ready':ready,'changed_paths':sorted(changed())},indent=2))

if __name__=='__main__': main()
