#!/usr/bin/env python3
from __future__ import annotations
import argparse,json
from datetime import datetime,timezone,timedelta
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def dt(v):
 try:return datetime.fromisoformat(str(v).replace('Z','+00:00')) if v else None
 except:return None
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--evidence',default=str(ROOT/'docs/release-evidence-current.json'));ap.add_argument('--warning-days',type=int,default=2);ap.add_argument('--require-no-expired',action='store_true');a=ap.parse_args();d=json.loads(Path(a.evidence).read_text());now=datetime.now(timezone.utc);fresh=[];soon=[];expired=[];invalid=[];nonpass=[]
 for r in d.get('checks',[]):
  cid=r.get('id');st=r.get('status')
  if st!='pass':nonpass.append(cid);continue
  rec=dt(r.get('recorded_at'));days=max(1,int(r.get('max_age_days') or 7))
  if not rec:invalid.append(cid);continue
  deadline=rec+timedelta(days=days);remain=(deadline-now).total_seconds()/86400
  row={'id':cid,'expires_at':deadline.isoformat(),'remaining_days':round(remain,2),'max_age_days':days}
  if remain<0:expired.append(row)
  elif remain<=max(0,a.warning_days):soon.append(row)
  else:fresh.append(row)
 out={'schema_version':1,'product_version':d.get('product_version'),'candidate_id':d.get('candidate_id'),'fresh':fresh,'expiring_soon':soon,'expired':expired,'invalid_timestamp':invalid,'nonpass_count':len(nonpass),'policy_ok':not expired and not invalid}
 print(json.dumps(out,indent=2,sort_keys=True));raise SystemExit(2 if a.require_no_expired and not out['policy_ok'] else 0)
if __name__=='__main__':main()
