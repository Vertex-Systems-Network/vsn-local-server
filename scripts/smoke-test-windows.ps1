$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

cargo fmt --all -- --check
cargo test --workspace
cargo build --workspace
node "$PSScriptRoot\check-contracts.mjs"

$agentPath = Join-Path $PSScriptRoot "..\target\debug\vsn-agent.exe"
$cliPath = Join-Path $PSScriptRoot "..\target\debug\vsn.exe"
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")

$agent = Start-Process -FilePath $agentPath -PassThru -NoNewWindow
try {
    Start-Sleep -Milliseconds 1200
    & $cliPath ping
    & $cliPath status
    & $cliPath machine
    & $cliPath security
    & $cliPath config show
    & $cliPath audit verify
    & $cliPath process list
    & $cliPath process metrics $agent.Id
    & $cliPath port list
    & $cliPath port check 49731
    & $cliPath runtime list
    & $cliPath runtime registry
    & $cliPath container backends
    & $cliPath project detect "$repoRoot"
    & $cliPath project dependencies "$repoRoot"
    & $cliPath domain plan vsn-self.test 49731
    & $cliPath db workspace relational
    & $cliPath db workspace vector
    & $cliPath db ui-demo
    & $cliPath vault list
    & $cliPath remote status
    Write-Host "VSN 0.4 local smoke test PASS"
}
finally {
    if ($null -ne $agent -and -not $agent.HasExited) {
        Stop-Process -Id $agent.Id -Force
    }
}
