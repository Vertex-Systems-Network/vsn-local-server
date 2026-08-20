$ErrorActionPreference = "Stop"
Write-Host "VSN development prerequisite check"

function Check-Command($name) {
    $cmd = Get-Command $name -ErrorAction SilentlyContinue
    if ($null -eq $cmd) {
        Write-Host "MISSING: $name"
        return $false
    }
    Write-Host "FOUND: $name -> $($cmd.Source)"
    return $true
}

$git = Check-Command "git"
$rust = Check-Command "rustc"
$cargo = Check-Command "cargo"
$node = Check-Command "node"

if (-not ($git -and $rust -and $cargo -and $node)) {
    Write-Host "One or more prerequisites are missing. This script intentionally does not auto-install system software."
    exit 1
}

Write-Host "Prerequisites look ready."
Write-Host "Next: cargo test --workspace"
