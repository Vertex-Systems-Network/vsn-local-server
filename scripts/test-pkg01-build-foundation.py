#!/usr/bin/env python3
import json,subprocess,sys
from pathlib import Path
R=Path(__file__).resolve().parents[1];p=subprocess.run([sys.executable,str(R/'scripts/pkg01-build-foundation.py'),'status'],cwd=R,text=True,stdout=subprocess.PIPE,check=False);d=json.loads(p.stdout);assert p.returncode==0 and len(d['tasks'])==22 and d['required']==22 and next(x for x in d['tasks'] if x['id']=='01.01')['status']=='DONE';print('PKG-01 build foundation regression: PASS')
