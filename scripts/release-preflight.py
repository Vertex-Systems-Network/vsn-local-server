#!/usr/bin/env python3
"""VSN release-runner preflight. Reports capability only; never records certification evidence."""
from __future__ import annotations
import argparse, json, os, platform, shutil, subprocess, sys
from pathlib import Path

VERSION=(Path(__file__).resolve().parents[1]/'VERSION').read_text().strip()
TOOLS={
  'rust': ['cargo','rustc'],
  'frontend': ['node','npm'],
  'windows': ['pwsh','wix','signtool'],
  'linux': ['dpkg-deb','systemctl','bwrap'],
  'macos': ['pkgbuild','productbuild','productsign','codesign','xcrun'],
  'containers': ['docker','podman'],
  'security': ['cargo-fuzz','cargo-audit'],
}

def command_version(path:str)->str|None:
    probes=[[path,'--version'],[path,'-version'],[path,'version']]
    for cmd in probes:
        try:
            p=subprocess.run(cmd,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=3,check=False)
            line=(p.stdout or '').strip().splitlines()
            if line:return line[0][:300]
        except Exception: pass
    return None

def main()->int:
    ap=argparse.ArgumentParser();ap.add_argument('--json',action='store_true');ap.add_argument('--strict',action='store_true',help='fail if host-required core tools are missing');args=ap.parse_args()
    host=platform.system().lower();found={}
    for group,names in TOOLS.items():
        for name in names:
            path=shutil.which(name);found[name]={'available':bool(path),'path':path,'version':command_version(path) if path else None,'group':group}
    host_required=['cargo','rustc','node','npm']
    if host=='windows':host_required+=['pwsh','wix','signtool']
    elif host=='linux':host_required+=['dpkg-deb']
    elif host=='darwin':host_required+=['pkgbuild','productbuild','codesign','xcrun']
    missing=[x for x in host_required if not found.get(x,{}).get('available')]
    optional_backends={
      'extension_sandbox_linux':found['bwrap']['available'] if host=='linux' else False,
      'container_docker':found['docker']['available'],
      'container_podman':found['podman']['available'],
      'rust_fuzz':found['cargo-fuzz']['available'],
      'rust_audit':found['cargo-audit']['available'],
    }
    payload={'version':VERSION,'host':{'system':platform.system(),'release':platform.release(),'machine':platform.machine()},'required_for_host':host_required,'missing_required':missing,'ready_for_host_build':not missing,'tools':found,'optional_backends':optional_backends,'note':'Preflight reports runner capability only. It does not mark any P30 release-certification control as passed.'}
    if args.json: print(json.dumps(payload,indent=2))
    else:
        print(f"VSN release preflight {VERSION} · {platform.system()} {platform.machine()}")
        for name in host_required:print(f"{'PASS' if found[name]['available'] else 'MISS':4} {name:14} {found[name]['path'] or ''}")
        print('ready_for_host_build=',str(not missing).lower())
        if missing:print('missing=',','.join(missing))
    return 1 if args.strict and missing else 0
if __name__=='__main__':raise SystemExit(main())
