$ErrorActionPreference='Stop'
$Root=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $Root
if((Get-Content VERSION).Trim() -ne '0.31.0'){throw 'VERSION mismatch'}
python scripts/validate-batch-0.31.py
python scripts/test-p30-pack.py
python scripts/source-readiness.py
python scripts/release-candidate.py verify --root . --file docs/release-candidate-current.json
python scripts/p30-pack-preflight.py --pack windows-core *> $env:TEMP\vsn-p30-windows-preflight.json
if($LASTEXITCODE -notin @(0,3)){throw "preflight failed: $LASTEXITCODE"}
