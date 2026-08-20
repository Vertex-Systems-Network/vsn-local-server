#!/usr/bin/env python3
from __future__ import annotations
import argparse,json,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def main():
    ap=argparse.ArgumentParser();ap.add_argument('--evidence',default=str(ROOT/'docs/release-evidence-current.json'));ap.add_argument('--roadmap',default=str(ROOT/'docs/roadmap-status.json'));ap.add_argument('--write',action='store_true');a=ap.parse_args()
    p=subprocess.run([sys.executable,str(ROOT/'scripts/release-evidence.py'),'evaluate','--file',a.evidence],text=True,stdout=subprocess.PIPE,check=True)
    ev=json.loads(p.stdout);satisfied=int(ev['satisfied']);required=max(1,int(ev['required']));cert_exact=100.0*satisfied/required;p30_exact=100.0 if ev['certified'] else 66.0+34.0*satisfied/required;p30=100 if ev['certified'] else round(p30_exact)
    roadmap=json.loads(Path(a.roadmap).read_text());phase=next(x for x in roadmap['phases'] if x['id']=='P30');phase['completion_percent']=p30;phase['status']='done' if ev['certified'] else 'pending';phase['note']=f"Evidence-driven certification: {ev['satisfied']}/{ev['required']} controls currently valid; P30 = 66% source-ready floor + certification evidence contribution. Stable 1.0 is {'certified' if ev['certified'] else 'not claimed'}."
    overall_exact=sum(float(x['completion_percent']) for x in roadmap['phases'] if x['id']!='P30')/len(roadmap['phases']) + p30_exact/len(roadmap['phases'])
    overall=round(overall_exact)
    roadmap['overall_completion_percent']=overall
    roadmap['overall_completion_exact_percent']=round(overall_exact,4)
    roadmap['source_completion_percent']=100.0
    roadmap['certification_completion_percent']=round(cert_exact,4)
    roadmap['p30_completion_exact_percent']=round(p30_exact,4)
    roadmap['stable_release_certified']=bool(ev['certified']);roadmap['release_candidate_id']=ev.get('candidate_id')
    out={'release_candidate_id':ev.get('candidate_id'),'p30_completion_percent':p30,'overall_completion_percent':overall,'evidence_completion_percent':round(cert_exact,4),'p30_completion_exact_percent':round(p30_exact,4),'overall_completion_exact_percent':round(overall_exact,4),'certified':ev['certified'],'satisfied':ev['satisfied'],'required':ev['required'],'blocked':ev.get('blocked',[]),'expired':ev.get('expired',[]),'pending':ev.get('pending',[])}
    print(json.dumps(out,indent=2,sort_keys=True))
    if a.write:Path(a.roadmap).write_text(json.dumps(roadmap,indent=2)+"\n")
if __name__=='__main__':main()
