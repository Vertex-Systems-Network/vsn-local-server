#!/usr/bin/env python3
"""Static/offline validation for VSN 0.13 source artifacts.

This deliberately does not pretend to replace cargo/type/native builds.
"""
from __future__ import annotations
import json, os, re, sys, tomllib, xml.etree.ElementTree as ET
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
errors=[]
counts={"json":0,"cargo":0,"rust":0,"yaml":0,"plist":0,"local_paths":0}

# JSON parse
for p in sorted(list((ROOT/'contracts').rglob('*.json'))+list((ROOT/'providers').rglob('*.json'))):
    try: json.loads(p.read_text(encoding='utf-8')); counts['json']+=1
    except Exception as e: errors.append(f"JSON {p.relative_to(ROOT)}: {e}")

# Machine-readable roadmap status
try:
    roadmap=json.loads((ROOT/'docs/roadmap-status.json').read_text(encoding='utf-8'))
    phases=roadmap.get('phases',[])
    if len(phases)!=31 or {p.get('id') for p in phases}!={f'P{i}' for i in range(31)}:
        errors.append('roadmap-status.json must contain exactly P0..P30')
    if roadmap.get('product_version')!='0.13.0': errors.append('roadmap-status.json product_version must be 0.13.0')
    try:
        from jsonschema import Draft202012Validator
        schema=json.loads((ROOT/'contracts/roadmap-status-v1.schema.json').read_text(encoding='utf-8'))
        Draft202012Validator(schema).validate(roadmap)
    except ImportError:
        pass
    except Exception as e:
        errors.append(f'Roadmap schema: {e}')
except Exception as e:
    errors.append(f'Roadmap status: {e}')

# Cargo parse and local path dependencies
manifests=sorted(ROOT.rglob('Cargo.toml'))
for p in manifests:
    try:
        data=tomllib.loads(p.read_text(encoding='utf-8')); counts['cargo']+=1
        tables=[]
        for key in ('dependencies','dev-dependencies','build-dependencies'):
            if isinstance(data.get(key),dict): tables.append(data[key])
        for target in (data.get('target') or {}).values():
            if isinstance(target,dict):
                for key in ('dependencies','dev-dependencies','build-dependencies'):
                    if isinstance(target.get(key),dict): tables.append(target[key])
        for table in tables:
            for name,spec in table.items():
                if isinstance(spec,dict) and 'path' in spec:
                    target=(p.parent/spec['path']).resolve()
                    counts['local_paths']+=1
                    if not target.exists(): errors.append(f"Cargo path {p.relative_to(ROOT)} -> {name}: missing {target}")
    except Exception as e: errors.append(f"Cargo {p.relative_to(ROOT)}: {e}")

# Workspace members
try:
    ws=tomllib.loads((ROOT/'Cargo.toml').read_text())['workspace']['members']
    for member in ws:
        if not (ROOT/member/'Cargo.toml').is_file(): errors.append(f"Workspace member missing Cargo.toml: {member}")
except Exception as e: errors.append(f"Workspace: {e}")

# Basic Rust structural sanity (balanced delimiters after removing strings/comments approximately).
def strip_rust(src:str)->str:
    out=[]; i=0; n=len(src)
    while i<n:
        if src.startswith('//',i):
            j=src.find('\n',i); i=n if j<0 else j+1; out.append('\n'); continue
        if src.startswith('/*',i):
            depth=1; i+=2
            while i<n and depth:
                if src.startswith('/*',i): depth+=1; i+=2
                elif src.startswith('*/',i): depth-=1; i+=2
                else: i+=1
            continue
        # raw strings r#"..."# (arbitrary # count)
        m=re.match(r'r(#+)?"',src[i:])
        if m:
            hashes=m.group(1) or ''; i+=len(m.group(0)); end='"'+hashes; j=src.find(end,i); i=n if j<0 else j+len(end); out.append('""'); continue
        if src[i]=='"':
            i+=1
            while i<n:
                if src[i]=='\\': i+=2; continue
                if src[i]=='"': i+=1; break
                i+=1
            out.append('""'); continue
        if src[i]=="'": # char or lifetime; only treat obvious quoted char as a char literal
            j=i+1
            if j<n and src[j]=='\\': j+=2
            else: j+=1
            if j<n and src[j]=="'": i=j+1; out.append("''"); continue
        out.append(src[i]); i+=1
    return ''.join(out)
for p in sorted(ROOT.rglob('*.rs')):
    counts['rust']+=1; clean=strip_rust(p.read_text(encoding='utf-8'))
    stack=[]; pairs={')':'(',']':'[','}':'{'}
    for idx,ch in enumerate(clean):
        if ch in '([{': stack.append((ch,idx))
        elif ch in pairs:
            if not stack or stack[-1][0]!=pairs[ch]: errors.append(f"Rust delimiter {p.relative_to(ROOT)} near {idx}"); break
            stack.pop()
    else:
        if stack: errors.append(f"Rust delimiter unclosed {p.relative_to(ROOT)}: {stack[-1][0]}")

# YAML parse if PyYAML exists
try:
    import yaml
    for p in sorted(list(ROOT.rglob('*.yml'))+list(ROOT.rglob('*.yaml'))):
        yaml.safe_load(p.read_text()); counts['yaml']+=1
except ImportError:
    pass
except Exception as e: errors.append(f"YAML: {e}")

for p in sorted(ROOT.rglob('*.plist')):
    try: ET.parse(p); counts['plist']+=1
    except Exception as e: errors.append(f"plist {p.relative_to(ROOT)}: {e}")


# 0.13 integration anchors: static presence only, not compiler proof.
required={
 'crates/vsn-stream/src/lib.rs':['open_stream_at','next_in_seq','next_out_seq'],
 'crates/vsn-control-store/src/lib.rs':['vsn_control_stream_relays','vsn_control_stream_frames','SharedStreamCheckpoint','upsert_stream_checkpoint','stream_frames_after','vsn_control_sessions','SharedSessionRecord','session_by_token_hash','touch_session','revoke_account_sessions','session_count'],
 'crates/vsn-cloud/src/lib.rs':['CloudCliSnapshotRequest','CloudCliCloneRequest','cloud_cli_snapshot','cloud_cli_clone','acknowledge_crash_consistency','confirm_new_instance','create-image','machine-images'],
 'cloud/control-plane/src/main.rs':['0.13.0','ensure_shared_relay_loaded','reopen_recoverable_relays','backfill_shared_sessions_once','store_account_session','revoke_account_sessions_for','SCIM_USER_SCHEMA','scim_create_user','scim_replace_user','scim_delete_user','control.scim.manage'],
 'apps/agent/src/main.rs':['0.13.0','cloud.cli.snapshot','cloud.cli.clone','vsn_resume_input_seq','vsn_resume_output_seq','terminal stream cannot be auto-reconstructed'],
 'apps/cli/src/main.rs':['0.13.0','cli-snapshot','cli-clone'],
 'contracts/stream-resume-checkpoint-v1.schema.json':['VSN Shared Stream Resume Checkpoint v1','resume_token_hash','acked_input_seq'],
 'contracts/scim-user-v1.schema.json':['VSN SCIM 2.0 User Provisioning Input','userName','roles'],
 'contracts/cloud-cli-snapshot-clone-v1.schema.json':['VSN Cloud CLI Snapshot / Clone Requests','acknowledge_crash_consistency','confirm_new_instance'],
 'contracts/control-plane-permissions-v1.json':['control.scim.manage'],
 'providers/examples/cloud-aws-cli/manifest.json':['snapshot','clone'],
 'providers/examples/cloud-azure-cli/manifest.json':['snapshot'],
 'providers/examples/cloud-gcp-cli/manifest.json':['snapshot','clone'],
 'docs/scim-runbook.md':['control.scim.manage','PATCH','Groups'],
 'docs/shared-auth-sessions.md':['vsn_control_sessions','SHA-256','backfills'],
 'docs/cloud-snapshot-clone-runbook.md':['acknowledge_crash_consistency','confirm_new_instance','Azure'],
}
for rel,needles in required.items():
    p=ROOT/rel
    if not p.is_file(): errors.append(f'Missing 0.13 integration file: {rel}'); continue
    text=p.read_text(encoding='utf-8')
    for needle in needles:
        if needle not in text: errors.append(f'Missing 0.13 integration anchor {needle} in {rel}')

# No build caches in source artifact
for banned in ('target','node_modules','dist'):
    for p in ROOT.rglob(banned):
        if p.is_dir(): errors.append(f"Generated directory included: {p.relative_to(ROOT)}")

print(json.dumps({"ok":not errors,"counts":counts,"errors":errors},indent=2))
sys.exit(1 if errors else 0)
