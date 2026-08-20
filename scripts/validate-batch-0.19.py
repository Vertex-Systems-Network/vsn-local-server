#!/usr/bin/env python3
"""Static/offline validation for VSN 0.19 source artifacts.

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
    if roadmap.get('product_version')!='0.19.0': errors.append('roadmap-status.json product_version must be 0.19.0')
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


# 0.19 integration anchors: static presence only, not compiler proof.
required={
 'crates/vsn-vault/src/lib.rs':['VaultRotationResult','rotate_master_key','restore_recovery_key','retire_recovery_key','VaultKeyHistory','delete_credential'],
 'crates/vsn-ai/src/lib.rs':['ExecuteRequest','ExecutionReport','EvaluationCase','EvaluationReport','evaluate_json','unrestricted_shell_allowed'],
 'apps/agent/src/main.rs':['0.19.0','runtime.repair','ai.execute','ai.evaluate','vault.rotate','vault.restore','vault.retire','marketplace.resolve-update-channel','extension.sandbox-capabilities','extension.exec','network.dns-start','resolver-apply','terminal.pty.recovery.list'],
 'apps/cli/src/main.rs':['0.19.0','runtime.repair','ai.evaluate','key-history','resolve-update-channel','sandbox-capabilities','pty-recovery-list'],
 'crates/vsn-marketplace/src/lib.rs':['MarketplaceRevocation','ChannelUpdateResolution','resolve_update_channel','channels','is_revoked'],
 'crates/vsn-extension/src/lib.rs':['ResolvedProvider','resolve_providers','SandboxExecRequest','run_sandboxed','bubblewrap','--unshare-net'],
 'crates/vsn-network/src/lib.rs':['DnsResolverPlan','run_dns_server','OsResolverStatus','apply_os_test_resolver','remove_os_test_resolver','127.0.0.1'],
 'crates/vsn-core/src/lib.rs':['runtime_repair','vault_restore','vault_retire','marketplace_resolve_update_channel','extension_exec','ai_evaluate','terminal_pty_recovery_list','dns_os_apply'],
 'cloud/control-plane/src/main.rs':['0.19.0','/ready','/v1/admin/ops','async fn readiness','async fn ops_status','VSN_SLO_CONTROL_P95_MS'],
 'scripts/release-evidence.py':['vault-key-rotation','penetration-test','require-certified','completion_percent'],
 'docs/release-evidence-0.19.json':['vault-key-rotation','penetration-test'],
 'contracts/release-certification-evidence-v1.schema.json':['VSN Release Certification Evidence v1'],
 'contracts/local-dns-v1.schema.json':['VSN Local .test DNS Resolver Plan v1'],
 'contracts/secret-vault-v2.schema.json':['VSN Vault File v2','key_id'],
 'contracts/ai-execution-v1.schema.json':['VSN Structured AI Execution Request v1'],
 'contracts/extension-provider-resolution-v1.schema.json':['VSN Installed Extension Provider Resolution v1'],
 'contracts/marketplace-index-v1.schema.json':['revocations'],
 'contracts/preview-websocket-v1.schema.json':['VSN Local Preview WebSocket Relay v1'],
 'crates/vsn-database-native/src/lib.rs':['postgres_job_start','postgres_job_cancel','postgres_read_transaction_start','BEGIN READ ONLY'],
 'packaging/windows/VSN.wxs':['VSNAgent','vsn-updater-helper.exe','ServiceInstall','Name="PATH"'],
 'packaging/linux/build-deb.sh':['dpkg-deb'],
 'packaging/macos/sign-notarize.sh':['productsign','notarytool'],
 '.github/workflows/release-gate.yml':['validate-batch-0.19.py','release-evidence.py','windows-msi','linux-deb','macos-pkg','cargo audit'],
 '.github/workflows/security-nightly.yml':['cargo fuzz run remote_protocol','cargo fuzz run stream_open','cargo audit'],
 '.github/workflows/release-signing.yml':['production-signing','VSN_WINDOWS_PFX_B64','sign-notarize-ci.sh'],
 'scripts/load-control-plane.py':['concurrency','p95'],
 'scripts/smoke-batch-0.19-linux.sh':['cargo clippy','build-deb.sh','smoke-updater-helper.py'],
 'scripts/smoke-batch-0.19-macos.sh':['cargo clippy','build-pkg.sh','smoke-updater-helper.py'],
 'scripts/smoke-batch-0.19-windows.ps1':['cargo clippy','build-msi.ps1','pg-native-job-list'],
 'docs/roadmap-status.json':['0.19.0','overall_completion_percent','completion_percent'],
 'docs/BATCH-0.19.md':['Vault rotation','AI execution','release evidence','.test DNS'],
 'docs/P30-release-readiness-0.19.md':['21-control','release-evidence.py'],
 'contracts/extension-sandbox-exec-v1.schema.json':['VSN Extension Sandbox Execution'],
 'contracts/vault-key-lifecycle-v1.schema.json':['Vault'],
 'contracts/pty-recovery-v1.schema.json':['PTY'],
 'contracts/ai-evaluation-suite-v1.schema.json':['AI'],
 'contracts/runtime-repair-v1.schema.json':['Runtime'],
 'contracts/os-resolver-status-v1.schema.json':['Resolver'],
 'contracts/container-mutation-v1.schema.json':['Container'],
 'crates/vsn-container/src/lib.rs':['ContainerBuildRequest','image_pull','image_build','remove_resource','run_bounded'],
 'docs/BATCH-0.19.md':['Runtime lifecycle','Vault lifecycle','Bubblewrap','Marketplace channels'],
 'docs/STATUS-0.19.md':['85%'],
 'docs/vault-key-lifecycle.md':['vault retire'],
 'docs/extension-sandbox.md':['Bubblewrap'],
 'docs/os-resolver-integration.md':['network-admin'],
} 

for rel,needles in required.items():
    p=ROOT/rel
    if not p.is_file(): errors.append(f'Missing 0.19 integration file: {rel}'); continue
    text=p.read_text(encoding='utf-8')
    for needle in needles:
        if needle not in text: errors.append(f'Missing 0.19 integration anchor {needle} in {rel}')

# Roadmap percentages must be explicit and mathematically consistent.
try:
    roadmap=json.loads((ROOT/'docs/roadmap-status.json').read_text())
    values=[int(p['completion_percent']) for p in roadmap['phases']]
    if any(v<0 or v>100 for v in values): errors.append('roadmap completion_percent must be 0..100')
    expected=round(sum(values)/len(values))
    if roadmap.get('overall_completion_percent')!=expected: errors.append(f'roadmap overall_completion_percent must equal rounded phase mean {expected}')
except Exception as e: errors.append(f'Roadmap percentage validation: {e}')

# No build caches in source artifact
for banned in ('target','node_modules','dist'):
    for p in ROOT.rglob(banned):
        if p.is_dir(): errors.append(f"Generated directory included: {p.relative_to(ROOT)}")

print(json.dumps({"ok":not errors,"counts":counts,"errors":errors},indent=2))
sys.exit(1 if errors else 0)
