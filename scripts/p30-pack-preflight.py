#!/usr/bin/env python3
from __future__ import annotations
import argparse,json,os,platform,shutil,sys
from pathlib import Path
from p30_platform import canonical_platform
ROOT=Path(__file__).resolve().parents[1]
PACKS=ROOT/'certification/p30-runner-packs.json'
ENV_REQ={
 'operations':['VSN_P30_CONTROL_LOAD_COMMAND','VSN_P30_HA_COMMAND','VSN_P30_DR_COMMAND','VSN_P30_VAULT_ROTATION_COMMAND'],
 'independent-review':['VSN_P30_PENTEST_VERIFY_COMMAND'],
}
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--pack',required=True);ap.add_argument('--json-output');a=ap.parse_args()
 doc=json.loads(PACKS.read_text());pack=next((x for x in doc['packs'] if x['id']==a.pack),None)
 if not pack:raise SystemExit(f'unknown pack: {a.pack}')
 host=canonical_platform(); host_match=pack['platform']=='cross-platform' or pack['platform']==host
 missing_tools=[x for x in pack['required_tools'] if not shutil.which(x)]
 missing_env=[x for x in ENV_REQ.get(pack['id'],[]) if not os.environ.get(x)]
 out={
  'schema_version':1,'product_version':doc['product_version'],'pack_id':pack['id'],'execution_mode':pack['execution_mode'],
  'declared_platform':pack['platform'],'host':host,'host_match':host_match,'required_tools':pack['required_tools'],
  'missing_required_tools':missing_tools,'required_environment':ENV_REQ.get(pack['id'],[]),'missing_environment':missing_env,
  'controls':pack['controls'],'ready':bool(host_match and not missing_tools and not missing_env)
 }
 text=json.dumps(out,indent=2,sort_keys=True);print(text)
 if a.json_output:Path(a.json_output).write_text(text+'\n')
 return 0 if out['ready'] else 3
if __name__=='__main__':raise SystemExit(main())
