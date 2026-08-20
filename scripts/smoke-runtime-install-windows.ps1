$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$cli = Join-Path $root "target\debug\vsn.exe"
$fixture = Join-Path $env:TEMP "vsn-runtime-smoke"
Remove-Item $fixture -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $fixture | Out-Null
$artifact = Join-Path $fixture "fake-runtime.cmd"
Set-Content -Path $artifact -Value "@echo off`r`necho fake-runtime 1.0.0`r`n" -NoNewline
$hash = (Get-FileHash -Algorithm SHA256 $artifact).Hash.ToLowerInvariant()
$urlPath = $artifact.Replace('\','/')
$catalog = @{
  schema_version = 1
  provider = "vsn-smoke"
  runtimes = @(@{
    runtime = "fake-runtime"
    version = "1.0.0"
    artifacts = @(@{
      os = "windows"
      arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "x86" }
      url = "file://$urlPath"
      sha256 = $hash
      archive = "binary"
      executable_relpath = "fake-runtime.cmd"
    })
  })
}
$catalogPath = Join-Path $fixture "catalog.json"
$catalog | ConvertTo-Json -Depth 8 | Set-Content $catalogPath
& $cli runtime catalog $catalogPath
& $cli runtime install $catalogPath fake-runtime 1.0.0
& $cli runtime activate "$root" fake-runtime 1.0.0
& $cli runtime registry
& $cli runtime uninstall fake-runtime 1.0.0
Write-Host "Runtime install/activate/uninstall smoke test PASS"
