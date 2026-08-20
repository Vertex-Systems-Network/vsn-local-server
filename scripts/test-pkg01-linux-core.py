#!/usr/bin/env python3
from __future__ import annotations
import json,subprocess,sys
from pathlib import Path
R=Path(__file__).resolve().parents[1]
p=subprocess.run([sys.executable,str(R/'scripts/pkg01-linux-core.py'),'status'],cwd=R,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
assert p.returncode==0,p.stdout
d=json.loads(p.stdout);assert d['package_id']=='PKG-01' and d['required_passes']==6 and len(d['controls'])==6
assert set(d['controls'])=={'rust-linux','desktop-build','dashboard-build','deb-install-uninstall','updater-linux','rustsec-audit'}
assert d['complete']==(d['valid_passes']==6)
# Execute must fail closed when status is not ready; never manufacture PASS.
if not d['ready']:
 q=subprocess.run([sys.executable,str(R/'scripts/pkg01-linux-core.py'),'execute','--no-import'],cwd=R,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
 assert q.returncode==4,q.stdout
 f=subprocess.run([sys.executable,str(R/'scripts/pkg01-finalize.py')],cwd=R,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
 assert f.returncode!=0 and 'cannot finalize' in f.stdout.lower(),f.stdout
print('PKG-01 Linux Core regression: PASS')
