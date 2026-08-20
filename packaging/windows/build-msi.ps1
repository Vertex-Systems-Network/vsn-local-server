param([string]$Version="",[string]$SourceDir="target\release",[string]$OutputDir="dist")
if([string]::IsNullOrWhiteSpace($Version)){$Version=(Get-Content (Join-Path $PSScriptRoot "..\..\VERSION")).Trim()}
$ErrorActionPreference="Stop"
if(!(Test-Path (Join-Path $SourceDir 'vsn-extension-appcontainer.exe'))){& (Join-Path $PSScriptRoot 'build-extension-appcontainer.ps1') -OutputDir $SourceDir | Out-Null}
foreach($name in @('vsn-agent.exe','vsn.exe','vsn-updater-helper.exe','vsn-extension-appcontainer.exe')){if(!(Test-Path (Join-Path $SourceDir $name))){throw "Missing release binary: $name"}}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
if(-not (Get-Command wix -ErrorAction SilentlyContinue)){throw "WiX v4 CLI not found. Install with: dotnet tool install --global wix"}
$out=Join-Path $OutputDir "vsn-runtime-$Version-x64.msi"
wix build "$PSScriptRoot\VSN.wxs" -arch x64 -d "SourceDir=$((Resolve-Path $SourceDir).Path)" -d "ProductVersion=$Version" -o $out
if(!(Test-Path $out)){throw "WiX did not create $out"}
Write-Host "msi=$out"
