#!/usr/bin/env python3
import json, sys
from pathlib import Path
from jsonschema import Draft202012Validator

root=Path(__file__).resolve().parents[1]
contracts=root/'contracts'
failed=[]
checked=0
for path in sorted(contracts.glob('*.schema.json')):
    try:
        Draft202012Validator.check_schema(json.loads(path.read_text(encoding='utf-8')))
        checked+=1
        print('SCHEMA OK',path.relative_to(root))
    except Exception as e:
        failed.append(f'{path}: {e}')

kind_schema={
 'database':'database-provider.schema.json','runtime':'runtime-provider.schema.json',
 'service':'service-provider.schema.json','project':'project-provider.schema.json',
 'container':'container-provider.schema.json','network':'network-provider.schema.json',
 'cloud':'cloud-provider.schema.json'
}
validators={k:Draft202012Validator(json.loads((contracts/v).read_text(encoding='utf-8'))) for k,v in kind_schema.items()}
for path in sorted((root/'providers').rglob('manifest.json')):
    try:
        obj=json.loads(path.read_text(encoding='utf-8')); kind=obj.get('kind')
        if kind not in validators: raise ValueError(f'unknown provider kind {kind!r}')
        validators[kind].validate(obj); checked+=1
        print('MANIFEST OK',path.relative_to(root))
    except Exception as e: failed.append(f'{path}: {e}')
print(f'validated={checked}')
if failed:
    for item in failed: print('FAIL',item,file=sys.stderr)
    sys.exit(1)
