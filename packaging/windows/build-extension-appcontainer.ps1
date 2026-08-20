param([string]$OutputDir='target\release')
$ErrorActionPreference='Stop'
$cl=Get-Command cl.exe -ErrorAction SilentlyContinue
if(-not $cl){throw 'MSVC cl.exe not found; run from a Visual Studio Developer PowerShell or install the C++ build tools'}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$src=(Resolve-Path (Join-Path $PSScriptRoot '..\..\native\windows\vsn-extension-appcontainer.cpp')).Path
$out=(Join-Path (Resolve-Path $OutputDir).Path 'vsn-extension-appcontainer.exe')
& $cl.Source /nologo /std:c++20 /O2 /EHsc /W4 $src /Fe:$out userenv.lib advapi32.lib ole32.lib
if($LASTEXITCODE -ne 0){throw "AppContainer helper compile failed: $LASTEXITCODE"}
if(!(Test-Path $out)){throw 'AppContainer helper output missing'}
Write-Output $out
