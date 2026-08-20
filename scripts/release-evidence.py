#!/usr/bin/env python3
from __future__ import annotations
import argparse, hashlib, json
from datetime import datetime, timezone, timedelta
from pathlib import Path

REQUIRED = [
    "rust-windows","rust-linux","rust-macos","desktop-build","dashboard-build",
    "msi-install-uninstall","deb-install-uninstall","pkg-install-uninstall",
    "updater-windows","updater-linux","updater-macos","windows-authenticode","macos-notarization",
    "rustsec-audit","fuzz-remote-protocol","fuzz-stream-open","control-load-slo",
    "ha-failover","dr-restore","vault-key-rotation","penetration-test"
]
VALID={"pending","pass","fail","waived","blocked"}
DEFAULT_MAX_AGE_DAYS={
    "penetration-test":90,"ha-failover":30,"dr-restore":30,"control-load-slo":14,
    "rustsec-audit":14,"fuzz-remote-protocol":14,"fuzz-stream-open":14,
}
def now_dt(): return datetime.now(timezone.utc)
def now(): return now_dt().isoformat()
def blank(version,candidate_id=None):
    return {"schema_version":4,"product_version":version,"candidate_id":candidate_id,"updated_at":now(),"checks":[{
        "id":x,"status":"pending","platform":"cross-platform","evidence":None,"artifact_sha256":None,
        "runner_attestation_sha256":None,"runner_attestation_ref":None,
        "run_url":None,"commit_sha":None,"notes":"","recorded_at":None,"max_age_days":DEFAULT_MAX_AGE_DAYS.get(x,7)
    } for x in REQUIRED]}
def load(path):
    p=Path(path)
    if not p.exists(): raise SystemExit(f"evidence file not found: {p}")
    d=json.loads(p.read_text())
    if d.get("schema_version") not in {1,2,3,4} or not isinstance(d.get("checks"),list): raise SystemExit("invalid evidence file")
    if d.get("schema_version") in {1,2,3}:
        upgraded=blank(d.get("product_version","unknown"),d.get("candidate_id")); by={x.get("id"):x for x in d["checks"]}
        for row in upgraded["checks"]:
            if row["id"] in by: row.update(by[row["id"]])
        d=upgraded
    return d
def save(path,d):
    d["schema_version"]=4;d["updated_at"]=now();Path(path).write_text(json.dumps(d,indent=2,sort_keys=True)+"\n")
def sha256(path):
    h=hashlib.sha256()
    with open(path,"rb") as f:
        for chunk in iter(lambda:f.read(1024*1024),b""):h.update(chunk)
    return h.hexdigest()
def parse_time(v):
    if not v:return None
    try:return datetime.fromisoformat(v.replace("Z","+00:00"))
    except Exception:return None
def pass_has_provenance(row):
    if row.get("status") != "pass": return True
    if not parse_time(row.get("recorded_at")): return False
    run_url=str(row.get("run_url") or "")
    # Local/self-hosted ad-hoc runs must bind PASS to both the result artifact and a runner attestation.
    if run_url.startswith("local://") or run_url.startswith("file://") or run_url in {"local","self-hosted"}:
        return bool(row.get("artifact_sha256") and row.get("runner_attestation_sha256") and row.get("runner_attestation_ref"))
    # Hosted CI/reviewer evidence must identify the run, commit and evidence subject.
    return bool(run_url and row.get("evidence") and row.get("commit_sha"))

def evaluate(d,allow_waivers=False):
    candidate_bound=bool(d.get("candidate_id") and isinstance(d.get("candidate_id"),str) and len(d.get("candidate_id"))==64)
    by={x.get("id"):x for x in d["checks"]}; missing=[x for x in REQUIRED if x not in by]
    invalid=[x for x in d["checks"] if x.get("status") not in VALID]
    invalid_provenance=[cid for cid in REQUIRED if cid in by and not pass_has_provenance(by[cid])]
    expired=[]; nowv=now_dt()
    for cid in REQUIRED:
        row=by.get(cid,{})
        if row.get("status")!="pass":continue
        recorded=parse_time(row.get("recorded_at")); days=int(row.get("max_age_days") or DEFAULT_MAX_AGE_DAYS.get(cid,7))
        if not recorded or nowv-recorded>timedelta(days=max(1,days)): expired.append(cid)
    satisfied=[]
    for cid in REQUIRED:
        st=by.get(cid,{}).get("status")
        if cid in expired or cid in invalid_provenance:continue
        if st=="pass" or (allow_waivers and st=="waived"): satisfied.append(cid)
    failed=[x for x in REQUIRED if by.get(x,{}).get("status")=="fail"]
    blocked=[x for x in REQUIRED if by.get(x,{}).get("status")=="blocked"]
    waived=[x for x in REQUIRED if by.get(x,{}).get("status")=="waived"]
    pending=[x for x in REQUIRED if x not in satisfied and x not in failed and x not in blocked and x not in waived and x not in expired]
    percent=round(len(satisfied)*100/len(REQUIRED))
    certified=candidate_bound and not missing and not invalid and not invalid_provenance and not failed and not blocked and not pending and not expired and (allow_waivers or not waived)
    return {"certified":certified,"candidate_bound":candidate_bound,"candidate_id":d.get("candidate_id"),"completion_percent":percent,"required":len(REQUIRED),"satisfied":len(satisfied),"failed":failed,"blocked":blocked,"waived":waived,"expired":expired,"pending":pending,"missing":missing,"invalid_entries":len(invalid),"invalid_provenance":invalid_provenance}
def main():
    ap=argparse.ArgumentParser();sub=ap.add_subparsers(dest="cmd",required=True)
    p=sub.add_parser("init");p.add_argument("--version",required=True);p.add_argument("--output",required=True);p.add_argument("--candidate",required=True)
    p=sub.add_parser("record");p.add_argument("--file",required=True);p.add_argument("--id",required=True,choices=REQUIRED);p.add_argument("--status",required=True,choices=sorted(VALID));p.add_argument("--platform",default="cross-platform");p.add_argument("--evidence");p.add_argument("--artifact");p.add_argument("--runner-attestation");p.add_argument("--runner-attestation-ref");p.add_argument("--run-url");p.add_argument("--commit-sha");p.add_argument("--notes",default="");p.add_argument("--max-age-days",type=int)
    p=sub.add_parser("evaluate");p.add_argument("--file",required=True);p.add_argument("--report");p.add_argument("--require-certified",action="store_true");p.add_argument("--allow-waivers",action="store_true")
    p=sub.add_parser("merge");p.add_argument("--version",required=True);p.add_argument("--output",required=True);p.add_argument("--candidate");p.add_argument("inputs",nargs="+")
    a=ap.parse_args()
    if a.cmd=="init":save(a.output,blank(a.version,a.candidate));return
    if a.cmd=="record":
        d=load(a.file);target=next((x for x in d["checks"] if x.get("id")==a.id),None)
        if target is None: target={"id":a.id};d["checks"].append(target)
        target.update({"status":a.status,"platform":a.platform,"evidence":a.evidence,"artifact_sha256":sha256(a.artifact) if a.artifact else None,"runner_attestation_sha256":sha256(a.runner_attestation) if a.runner_attestation else None,"runner_attestation_ref":a.runner_attestation_ref,"run_url":a.run_url,"commit_sha":a.commit_sha,"notes":a.notes,"recorded_at":now()})
        if a.max_age_days: target["max_age_days"]=max(1,a.max_age_days)
        save(a.file,d);return
    if a.cmd=="merge":
        first=load(a.inputs[0]); candidate=a.candidate or first.get("candidate_id")
        if not candidate: raise SystemExit("candidate id required for merge")
        out=blank(a.version,candidate);by={x["id"]:x for x in out["checks"]}
        # Newest evidence wins among substantive rows; a merely blocked/pending runner must never erase substantive certification evidence.
        # Among substantive rows the newest result wins; exact-time ambiguity fails closed.
        substantive={"pass","fail","waived"}; priority={"pending":0,"blocked":1,"waived":2,"pass":3,"fail":4}
        def choose(cur,inc):
            cs=cur.get("status"); ins=inc.get("status"); csub=cs in substantive; isub=ins in substantive
            if csub and not isub:return cur
            if isub and not csub:return inc
            ct=parse_time(cur.get("recorded_at")); it=parse_time(inc.get("recorded_at"))
            if it and not ct:return inc
            if ct and not it:return cur
            if it and ct:
                if it>ct:return inc
                if ct>it:return cur
            return inc if priority.get(ins,-1)>=priority.get(cs,-1) else cur
        for src in a.inputs:
            d=load(src)
            if d.get("product_version")!=a.version: raise SystemExit(f"version mismatch in {src}")
            if d.get("candidate_id")!=candidate: raise SystemExit(f"candidate mismatch in {src}")
            for x in d["checks"]:
                cid=x.get("id")
                if cid in by:by[cid]=choose(by[cid],x)
        out["checks"]=[by[x] for x in REQUIRED];save(a.output,out);return
    if a.cmd=="evaluate":
        d=load(a.file);r=evaluate(d,a.allow_waivers);text=json.dumps(r,indent=2,sort_keys=True);print(text)
        if a.report:Path(a.report).write_text(text+"\n")
        if a.require_certified and not r["certified"]:raise SystemExit(2)
if __name__=="__main__":main()
