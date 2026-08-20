#!/usr/bin/env python3
"""Static/offline validation for VSN 0.6 source artifacts.

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

# No build caches in source artifact
for banned in ('target','node_modules','dist'):
    for p in ROOT.rglob(banned):
        if p.is_dir(): errors.append(f"Generated directory included: {p.relative_to(ROOT)}")

print(json.dumps({"ok":not errors,"counts":counts,"errors":errors},indent=2))
sys.exit(1 if errors else 0)
