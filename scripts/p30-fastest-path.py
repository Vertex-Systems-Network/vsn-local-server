#!/usr/bin/env python3
from __future__ import annotations
import json,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
EVID=ROOT/'scripts/release-evidence.py'; VERSION=(ROOT/'VERSION').read_text().strip(); PACKS=ROOT/'certification/p30-runner-packs.json'
ORDER=['linux-core','windows-core','macos-core','security-nightly','operations','independent-review']
def score(n,total=21):
    p30=100.0 if n>=total else 66.0+34.0*n/total; overall=(3000.0+p30)/31.0
    return round(100*n/total,2),round(p30,2),round(overall,2)
def main():
    evidence=ROOT/'docs/release-evidence-current.json'
    ev=json.loads(subprocess.check_output([sys.executable,str(EVID),'evaluate','--file',str(evidence)],text=True))
    ledger=json.loads(evidence.read_text());bad=set(ev.get('invalid_provenance',[]))|set(ev.get('expired',[]));done={r['id'] for r in ledger['checks'] if r.get('status')=='pass' and r.get('id') not in bad}
    packs={p['id']:p for p in json.loads(PACKS.read_text())['packs']}; running=len(done);out=[]
    for pid in ORDER:
        pack=packs[pid];remaining=[x for x in pack['controls'] if x not in done];running+=len(remaining);cert,p30,overall=score(running)
        out.append({'stage':pack['description'],'pack_id':pid,'execution_mode':pack['execution_mode'],'controls':remaining,'new_valid_passes':len(remaining),'cumulative_valid_passes':running,'certification_percent':cert,'p30_exact_percent':p30,'overall_exact_percent':overall})
    payload={'schema_version':1,'product_version':VERSION,'current_valid_passes':len(done),'required':21,'stages':out}
    print(json.dumps(payload,indent=2));(ROOT/'docs/p30-fastest-path.json').write_text(json.dumps(payload,indent=2)+'\n')
    md=[f'# P30 Fastest Completion Path — {VERSION}','',f'Current valid evidence: **{len(done)}/21**.','', '| Pack | Mode | New controls | Cumulative | P30 exact | Overall exact |','|---|---|---:|---:|---:|---:|']
    for s in out:md.append(f"| `{s['pack_id']}` | {s['execution_mode']} | {s['new_valid_passes']} | {s['cumulative_valid_passes']}/21 | {s['p30_exact_percent']:.2f}% | {s['overall_exact_percent']:.2f}% |")
    md+=['','## Controls','']
    for s in out:md.append(f"**{s['pack_id']}** — "+(', '.join(f'`{x}`' for x in s['controls']) if s['controls'] else 'already satisfied'))
    (ROOT/'docs/p30-fastest-path.md').write_text('\n'.join(md)+'\n')
if __name__=='__main__':raise SystemExit(main())
