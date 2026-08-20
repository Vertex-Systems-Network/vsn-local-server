#!/usr/bin/env python3
import argparse,json,sys
from pathlib import Path
from p30_result_lib import ResultError,build,verify
ap=argparse.ArgumentParser();sp=ap.add_subparsers(dest='cmd',required=True)
p=sp.add_parser('build');p.add_argument('--run-dir',required=True);p.add_argument('--output-dir',required=True)
p=sp.add_parser('verify');p.add_argument('bundle');p.add_argument('--sha256')
a=ap.parse_args()
try:r=build(Path(a.run_dir),Path(a.output_dir)) if a.cmd=='build' else verify(Path(a.bundle),sha_file=a.sha256)
except ResultError as e:print(e,file=sys.stderr);raise SystemExit(2)
print(json.dumps(r,indent=2,sort_keys=True))
