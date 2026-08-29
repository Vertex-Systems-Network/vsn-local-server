#!/usr/bin/env python3
from __future__ import annotations

import hashlib, json, subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / '.ai/manifests/pkg03-0309-desktop-registration.v1.json'
TRACKER = ROOT / 'certification/pkg03-windows-installer-v1.json'
STATUS = ROOT / 'docs/MASTER-EXECUTION-STATUS.json'
TAURI = ROOT / 'apps/desktop/src-tauri/tauri.conf.json'
OWNERSHIP = ROOT / 'installer/windows/owned-payload.v1.json'
HARNESS = ROOT / 'scripts/ci/pkg03-0309-desktop-registration.ps1'
WORKFLOW = ROOT / '.github/workflows/pkg03-0309-desktop-registration.yml'

BASE = '4f5e8ab30f030e758c52c4ca4ac08f73f896247a'
PLAN_SHA = '9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e'
TAURI_SHA = '172cf6110e58a15442bcf97e9db6a8bdbeb6cbfd2f631d91a3031603ed474180'
OWNERSHIP_SHA = '5292a1ec1ae0a48d76f80258c9e00ba17b1466dab5c5e0cdda60caf4658dabd1'
PRE_READY = ['03.09','03.10','03.13','03.15']
POST_READY = ['03.10','03.13','03.15']
ALLOWED = {
 '.ai/features/pkg03-0309/research.md', '.ai/features/pkg03-0309/lifecycle-review.md',
 '.ai/features/pkg03-0309/development-preflight.md', '.ai/plans/pkg03-0309-desktop-registration-v1.md',
 '.ai/manifests/pkg03-0309-desktop-registration.v1.json', 'docs/PKG03-DESKTOP-REGISTRATION-LIFECYCLE-V1.md',
 'scripts/ci/validate-pkg03-0309.py', 'scripts/ci/pkg03-0309-desktop-registration.ps1',
 '.github/workflows/pkg03-0309-desktop-registration.yml', 'certification/pkg03-windows-installer-v1.json',
 'docs/MASTER-EXECUTION-STATUS.json'
}

def fail(msg): raise SystemExit('PKG-03 03.09 validation failed: ' + msg)
def git_bytes(path, ref='HEAD'):
 p = subprocess.run(['git','show',f'{ref}:{path}'], cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
 if p.returncode: fail(f'unable to read {ref}:{path}')
 return p.stdout
def sha(b): return hashlib.sha256(b).hexdigest()
def changed():
 p = subprocess.run(['git','diff','--name-only',f'{BASE}..HEAD'], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
 if p.returncode: fail('unable to compare canonical base')
 return {x.strip() for x in p.stdout.splitlines() if x.strip()}

def main():
 m = json.loads(MANIFEST.read_text(encoding='utf-8'))
 t = json.loads(TRACKER.read_text(encoding='utf-8'))
 s = json.loads(STATUS.read_text(encoding='utf-8'))
 tc = json.loads(TAURI.read_text(encoding='utf-8'))
 own = json.loads(OWNERSHIP.read_text(encoding='utf-8'))
 if (m.get('feature_id'),m.get('task_id'),m.get('linear_issue'),m.get('version'),m.get('status')) != ('pkg03-0309-desktop-registration','03.09','ABD-84','1.0.0','frozen'): fail('task identity/version/status mismatch')
 if m.get('canonical_base_sha') != BASE or m.get('parent_plan',{}).get('sha256') != PLAN_SHA or sha(git_bytes('.ai/plans/pkg03-windows-installer-v1.md')) != PLAN_SHA: fail('canonical base/parent plan drifted')
 for label,obj in [('research',m['research']),('lifecycle',m['lifecycle']),('preflight',m['development_preflight']),('task plan',m['task_plan']),('contract',m['lifecycle_contract'])]:
  p = obj.get('path') or obj.get('artifact')
  if not p or sha(git_bytes(p)) != obj.get('sha256'): fail(label + ' digest mismatch')
 if m['research'].get('change_required') is not False: fail('material research delta unresolved')
 extra = changed() - ALLOWED
 if extra: fail('out-of-scope changed paths: ' + str(sorted(extra)))
 for p,d in [('apps/desktop/src-tauri/tauri.conf.json',TAURI_SHA),('installer/windows/owned-payload.v1.json',OWNERSHIP_SHA)]:
  b = git_bytes(p)
  if b != git_bytes(p, BASE) or sha(b) != d: fail('accepted source drifted: ' + p)
 if tc.get('productName') != 'VSN Dev Platform' or tc.get('version') != '0.38.1' or tc.get('identifier') != 'dev.vsn.platform': fail('product identity drifted')
 bun = tc.get('bundle',{})
 if bun.get('publisher') != 'Vertex Systems Network' or 'externalBin' in bun or 'resources' in bun: fail('publisher/payload contract drifted')
 if [x.get('relative_path') for x in own.get('owned_files',[])] != ['VSN Dev Platform.exe','bin/vsn.exe','bin/vsn-agent.exe']: fail('owned payload set drifted')
 a = m['authority']
 for k in ['nsis_interactive_execution_allowed_after_planning_gates','msi_interactive_execution_allowed_after_planning_gates']:
  if a.get(k) is not True: fail('execution authority missing: ' + k)
 for k in ['planning_product_mutation_allowed','custom_nsis_template_allowed','custom_wix_template_allowed','tauri_config_mutation_allowed','cli_agent_real_placement_allowed','service_registration_allowed','acl_mutation_allowed','silent_or_passive_deployment_allowed','signing_secret_access_allowed','updater_mutation_allowed','delegated_scope_may_expand']:
  if a.get(k) is not False: fail('authority widened: ' + k)
 ac=m['acceptance']; ns=ac['nsis_contract']; wx=ac['wix_contract']
 if ac.get('runner') != 'windows-2025' or ac.get('evidence_artifact') != 'pkg03-0309-desktop-registration': fail('runner/artifact drifted')
 if ns.get('start_menu_shortcut_required') is not True or ns.get('desktop_shortcut_positive_path_requires_gui_selection') is not True or ns.get('shortcut_target') != 'VSN Dev Platform.exe' or ns.get('app_user_model_id') != 'dev.vsn.platform' or ns.get('uninstall_cleanup_required') is not True: fail('NSIS shortcut contract drifted')
 if wx.get('start_menu_shortcut_required') is not True or wx.get('desktop_shortcut_required') is not True or wx.get('shortcut_target') != 'VSN Dev Platform.exe' or wx.get('start_menu_app_user_model_id') != 'dev.vsn.platform' or wx.get('uninstall_cleanup_required') is not True: fail('WiX shortcut contract drifted')
 if ac.get('nonclaims',{}).get('file_or_deep_link_registration_configured') is not False: fail('undeclared registration claim widened')
 if not HARNESS.is_file() or not WORKFLOW.is_file(): fail('certification surface incomplete')
 harness = HARNESS.read_text(encoding='utf-8')
 for token in ['UIAutomationClient','WScript.Shell','System.AppUserModel.ID','Desktop shortcut','Start Menu','VSN Dev Platform.exe','dev.vsn.platform','msiexec.exe','uninstall.exe','nsis_start_menu_removed','wix_start_menu_removed']:
  if token not in harness: fail('desktop registration harness missing token: ' + token)
 wf = WORKFLOW.read_text(encoding='utf-8')
 for token in ['windows-2025','22.12.0','1.97.1','build --bundles nsis,msi','pkg03-0309-desktop-registration',TAURI_SHA,OWNERSHIP_SHA]:
  if token not in wf: fail('workflow missing frozen token: ' + token)
 tasks = {x['id']:x for x in t.get('tasks',[])}
 if list(tasks) != [f'03.{i:02d}' for i in range(1,26)] or t.get('required') != 25: fail('denominator/order drifted')
 for i in range(1,9):
  if tasks[f'03.{i:02d}'].get('status') != 'DONE': fail(f'completed task regressed: 03.{i:02d}')
 if tasks['03.09'].get('depends_on') != ['03.03','03.05']: fail('03.09 dependencies drifted')
 state = tasks['03.09'].get('status')
 if state == 'READY': done,ready,cursor = 8,PRE_READY,'03.09'
 elif state == 'DONE':
  done,ready,cursor = 9,POST_READY,'03.10'
  ev=tasks['03.09'].get('evidence')
  if not isinstance(ev,dict): fail('accepted task missing evidence')
  for k in ['source_commit','workflow_run','job','artifact','artifact_digest','evidence_sha256']:
   if not ev.get(k): fail('accepted evidence missing ' + k)
 else: fail('unexpected 03.09 state: ' + str(state))
 if t.get('done') != done or float(t.get('percent',-1)) != done*4.0 or t.get('active_task') != cursor or t.get('ready_tasks') != ready: fail('tracker progress/cursor/READY mismatch')
 for x in ready:
  if tasks[x].get('status') != 'READY': fail('READY task missing: ' + x)
 pkg = {x['id']:x for x in s.get('packages',[])}.get('PKG-03',{})
 if s.get('active_package') != 'PKG-03' or s.get('active_task') != cursor or pkg.get('done') != done or pkg.get('required') != 25 or float(pkg.get('percent',-1)) != done*4.0: fail('master state mismatch')
 print(json.dumps({'valid':True,'task':'03.09','state':state,'done':done,'cursor':cursor,'ready':ready,'changed_paths':sorted(changed())}, indent=2))

if __name__ == '__main__': main()
