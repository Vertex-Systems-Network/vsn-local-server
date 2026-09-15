#!/usr/bin/env python3
"""Synthetic audit-only probe for the future PKG-03 03.25 final evidence index."""

from __future__ import annotations

import argparse, copy, hashlib, json, re
from pathlib import Path

HEX40=re.compile(r"^[0-9a-f]{40}$")
HEX64=re.compile(r"^[0-9a-f]{64}$")
TASKS=tuple(f"03.{i:02d}" for i in range(2,25))
PACKAGES=("nsis-current-user.exe","nsis-per-machine.exe","vsn-platform.msi","VSN Dev Platform.exe")

class ContractError(ValueError): pass

def req(ok,msg):
    if not ok: raise ContractError(msg)

def sha(label): return hashlib.sha256(label.encode()).hexdigest()

def validate(index):
    req(index.get("schema_version")==1,"schema mismatch")
    req(index.get("task_id")=="03.25","task mismatch")
    req(index.get("synthetic_audit_only") is True,"audit marker required")
    source=index.get("source_commit")
    req(isinstance(source,str) and HEX40.fullmatch(source),"invalid source SHA")

    deps=index.get("dependencies")
    req(isinstance(deps,list) and len(deps)==len(TASKS),"dependency count mismatch")
    by_task={}
    for row in deps:
        req(isinstance(row,dict),"dependency row invalid")
        tid=row.get("task_id")
        req(tid in TASKS and tid not in by_task,f"invalid/duplicate dependency {tid}")
        req(row.get("status")=="DONE",f"dependency {tid} not DONE")
        digest=row.get("evidence_sha256")
        req(isinstance(digest,str) and HEX64.fullmatch(digest),f"invalid evidence digest {tid}")
        by_task[tid]=row
    req(set(by_task)==set(TASKS),"dependency set incomplete")

    signing=index.get("signing")
    req(isinstance(signing,dict) and signing.get("task_id")=="03.22","03.22 signing evidence missing")
    req(signing.get("production_accepted") is True,"03.22 production signing not accepted")
    req(signing.get("source_commit")==source,"03.22 source mismatch")

    provenance=index.get("provenance")
    req(isinstance(provenance,dict) and provenance.get("task_id")=="03.23","03.23 provenance evidence missing")
    req(provenance.get("accepted") is True,"03.23 not accepted")
    req(provenance.get("source_commit")==source,"03.23 source mismatch")

    vm=index.get("vm_matrix")
    req(isinstance(vm,dict) and vm.get("task_id")=="03.24","03.24 VM evidence missing")
    req(vm.get("accepted") is True,"03.24 not accepted")
    req(vm.get("source_commit")==source,"03.24 source mismatch")
    req(vm.get("all_rows_pass") is True,"03.24 matrix not all PASS")
    if vm.get("real_reboot_claimed") is True:
        req(vm.get("same_machine_verified") is True,"03.24 reboot lacks same-machine proof")

    def package_map(obj,label):
        rows=obj.get("packages")
        req(isinstance(rows,list) and len(rows)==len(PACKAGES),f"{label} package set mismatch")
        result={}
        for row in rows:
            name=row.get("file_name") if isinstance(row,dict) else None
            req(name in PACKAGES and name not in result,f"{label} invalid package {name}")
            digest=row.get("sha256")
            req(isinstance(digest,str) and HEX64.fullmatch(digest),f"{label} invalid digest {name}")
            result[name]=digest
        req(set(result)==set(PACKAGES),f"{label} package names incomplete")
        return result

    s=package_map(signing,"03.22")
    p=package_map(provenance,"03.23")
    v=package_map(vm,"03.24")
    req(s==p==v,"package subject-byte continuity broken across 03.22/03.23/03.24")

    req(index.get("full_regression_pass") is True,"final regression gate not PASS")
    req(index.get("independent_verification_pass") is True,"independent verification missing")
    req(index.get("tracked_drift_zero") is True,"tracked drift not zero")
    req(index.get("secret_leak_scan_pass") is True,"secret leak scan failed")

    handoff=index.get("pkg04_handoff")
    req(isinstance(handoff,dict),"PKG-04 handoff missing")
    digest=handoff.get("sha256")
    req(isinstance(digest,str) and HEX64.fullmatch(digest),"PKG-04 handoff digest invalid")
    req(handoff.get("pkg04_activated") is False,"PKG-04 must remain inactive")
    req(index.get("pkg03_complete_projected") is False,"03.25 evidence must not itself project PKG-03 COMPLETE")


def fixture():
    source="4"*40
    packages=[{"file_name":n,"sha256":sha("signed:"+n)} for n in PACKAGES]
    return {
        "schema_version":1,"task_id":"03.25","synthetic_audit_only":True,"source_commit":source,
        "dependencies":[{"task_id":t,"status":"DONE","evidence_sha256":sha("evidence:"+t)} for t in TASKS],
        "signing":{"task_id":"03.22","production_accepted":True,"source_commit":source,"packages":copy.deepcopy(packages)},
        "provenance":{"task_id":"03.23","accepted":True,"source_commit":source,"packages":copy.deepcopy(packages)},
        "vm_matrix":{"task_id":"03.24","accepted":True,"source_commit":source,"all_rows_pass":True,"real_reboot_claimed":True,"same_machine_verified":True,"packages":copy.deepcopy(packages)},
        "full_regression_pass":True,"independent_verification_pass":True,"tracked_drift_zero":True,"secret_leak_scan_pass":True,
        "pkg04_handoff":{"sha256":sha("pkg04-handoff"),"pkg04_activated":False},"pkg03_complete_projected":False,
    }


def self_test():
    base=fixture(); cases=[]
    validate(copy.deepcopy(base)); cases.append({"name":"complete-exact-head-final-index","result":"PASS","expected":"PASS"})
    def reject(name,mutate):
        x=copy.deepcopy(base); mutate(x)
        try: validate(x)
        except ContractError as e:
            cases.append({"name":name,"result":"REJECT","expected":"REJECT","reason":str(e)}); return
        raise AssertionError("negative fixture passed: "+name)
    reject("reject-incomplete-dependency",lambda x:x["dependencies"].pop())
    reject("reject-dependency-not-done",lambda x:x["dependencies"][0].__setitem__("status","BLOCKED"))
    reject("reject-nonproduction-signing",lambda x:x["signing"].__setitem__("production_accepted",False))
    reject("reject-provenance-source-mismatch",lambda x:x["provenance"].__setitem__("source_commit","5"*40))
    reject("reject-rebuilt-package-substitution",lambda x:x["provenance"]["packages"][0].__setitem__("sha256",sha("rebuilt")))
    reject("reject-vm-failure",lambda x:x["vm_matrix"].__setitem__("all_rows_pass",False))
    reject("reject-reboot-without-same-machine",lambda x:x["vm_matrix"].__setitem__("same_machine_verified",False))
    reject("reject-final-regression-failure",lambda x:x.__setitem__("full_regression_pass",False))
    reject("reject-secret-leak",lambda x:x.__setitem__("secret_leak_scan_pass",False))
    reject("reject-premature-pkg04-activation",lambda x:x["pkg04_handoff"].__setitem__("pkg04_activated",True))
    reject("reject-premature-pkg03-complete",lambda x:x.__setitem__("pkg03_complete_projected",True))
    req(len(cases)==12,"case count drift")
    return {"schema_version":1,"audit":"PKG-03 03.25 final evidence contract","mode":"synthetic-only","production_evidence_consumed":False,"canonical_state_changed":False,"implementation_authority":False,"summary":{"pass":1,"negative_rejections":11,"total":12},"cases":cases}


def main():
    p=argparse.ArgumentParser(); p.add_argument("--self-test",action="store_true",required=True); p.add_argument("--output",required=True); a=p.parse_args()
    report=self_test(); out=Path(a.output); out.parent.mkdir(parents=True,exist_ok=True); out.write_text(json.dumps(report,indent=2,sort_keys=True)+"\n")
    print(json.dumps(report["summary"],sort_keys=True)); print("AUDIT_ONLY_0325_FINAL_EVIDENCE_CONTRACT=PASS"); return 0

if __name__=="__main__": raise SystemExit(main())
