#!/usr/bin/env python3
"""Show exact P30 runner packs, prerequisites and next actionable controls."""
from __future__ import annotations
import argparse,json,platform,shutil,subprocess,sys
from pathlib import Path
from p30_platform import canonical_platform
ROOT=Path(__file__).resolve().parents[1]; PACKS=ROOT/'certification/p30-runner-packs.json'; EVID=ROOT/'scripts/release-evidence.py'
def main():
    ap=argparse.ArgumentParser();ap.add_argument('--evidence',default=str(ROOT/'docs/release-evidence-current.json'));ap.add_argument('--json-output');ap.add_argument('--markdown-output');a=ap.parse_args()
    packs=json.loads(PACKS.read_text());ledger=json.loads(Path(a.evidence).read_text());ev=json.loads(subprocess.check_output([sys.executable,str(EVID),'evaluate','--file',a.evidence],text=True));by={x['id']:x for x in ledger['checks']};host=canonical_platform();rows=[]
    for pack in packs['packs']:
        missing=[t for t in pack['required_tools'] if not shutil.which(t)];pending=[c for c in pack['controls'] if by.get(c,{}).get('status')!='pass'];host_match=pack['platform']=='cross-platform' or pack['platform']==host
        rows.append({**pack,'host_match':host_match,'missing_required_tools':missing,'remaining_controls':pending,'ready_on_current_host':bool(pack['execution_mode']=='ci' and host_match and not missing and pending)})
    out={'schema_version':1,'product_version':packs['product_version'],'host':host,'certification_satisfied':ev['satisfied'],'certification_required':ev['required'],'packs':rows};text=json.dumps(out,indent=2,sort_keys=True);print(text)
    if a.json_output:Path(a.json_output).write_text(text+'\n')
    if a.markdown_output:
        md=[f'# P30 Runner Plan — {packs["product_version"]}','',f'Host: **{host}** · Evidence: **{ev["satisfied"]}/{ev["required"]}**','', '| Pack | Mode | Platform | Remaining | Missing required tools | Ready here |','|---|---|---|---:|---|---|']
        for r in rows:md.append(f"| `{r['id']}` | {r['execution_mode']} | {r['platform']} | {len(r['remaining_controls'])} | {', '.join(r['missing_required_tools']) or 'none'} | {'yes' if r['ready_on_current_host'] else 'no'} |")
        md+=['','## Remaining controls','']
        for r in rows:md.append(f"**{r['id']}** — "+(', '.join(f'`{x}`' for x in r['remaining_controls']) if r['remaining_controls'] else 'complete'))
        Path(a.markdown_output).write_text('\n'.join(md)+'\n')
if __name__=='__main__':raise SystemExit(main())
