param(
  [string]$StageDir = 'target/pkg03/03.10'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
  throw '03.10 Windows payload staging requires Windows.'
}

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Push-Location $RepoRoot
try {
  $actualHead = (git rev-parse HEAD).Trim()
  if (-not $actualHead) { throw 'Unable to bind 03.10 staging to a source commit.' }

  cargo build --locked --release -p vsn -p vsn-agent
  if ($LASTEXITCODE -ne 0) { throw "03.10 CLI/Agent release build failed with exit code $LASTEXITCODE." }

  $sourceCli = Join-Path $RepoRoot 'target/release/vsn.exe'
  $sourceAgent = Join-Path $RepoRoot 'target/release/vsn-agent.exe'
  foreach ($path in @($sourceCli,$sourceAgent)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Expected release payload missing: $path" }
    if ((Get-Item -LiteralPath $path).Length -le 0) { throw "Release payload is empty: $path" }
  }

  $stageRoot = if ([IO.Path]::IsPathRooted($StageDir)) { $StageDir } else { Join-Path $RepoRoot $StageDir }
  if (Test-Path -LiteralPath $stageRoot) { Remove-Item -LiteralPath $stageRoot -Recurse -Force }
  New-Item -ItemType Directory -Force -Path $stageRoot | Out-Null

  $stageCli = Join-Path $stageRoot 'vsn.exe'
  $stageAgent = Join-Path $stageRoot 'vsn-agent.exe'
  Copy-Item -LiteralPath $sourceCli -Destination $stageCli
  Copy-Item -LiteralPath $sourceAgent -Destination $stageAgent

  function Get-Hash([string]$Path) {
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
  }

  $cliSourceHash = Get-Hash $sourceCli
  $agentSourceHash = Get-Hash $sourceAgent
  $cliStageHash = Get-Hash $stageCli
  $agentStageHash = Get-Hash $stageAgent
  if ($cliSourceHash -ne $cliStageHash) { throw 'Staged vsn.exe hash differs from release output.' }
  if ($agentSourceHash -ne $agentStageHash) { throw 'Staged vsn-agent.exe hash differs from release output.' }

  $unexpected = @(Get-ChildItem -LiteralPath $stageRoot -File | Where-Object { $_.Name -notin @('vsn.exe','vsn-agent.exe') })
  if ($unexpected.Count -ne 0) { throw "Unexpected staged payload(s): $($unexpected.Name -join ', ')" }

  $manifest = [ordered]@{
    schema_version = 1
    package_id = 'PKG-03'
    task_id = '03.10'
    source_commit = $actualHead
    cargo_locked = $true
    staged = @(
      [ordered]@{
        id = 'cli'
        source = 'target/release/vsn.exe'
        stage = 'target/pkg03/03.10/vsn.exe'
        destination = 'bin/vsn.exe'
        size_bytes = (Get-Item -LiteralPath $stageCli).Length
        sha256 = $cliStageHash
      },
      [ordered]@{
        id = 'agent'
        source = 'target/release/vsn-agent.exe'
        stage = 'target/pkg03/03.10/vsn-agent.exe'
        destination = 'bin/vsn-agent.exe'
        size_bytes = (Get-Item -LiteralPath $stageAgent).Length
        sha256 = $agentStageHash
      }
    )
  }

  # The Tauri hook invokes Windows PowerShell on standard Windows hosts, while
  # CI itself may use PowerShell 7. Write deterministic UTF-8 without BOM using
  # .NET so the staging contract behaves identically in both shells.
  $stageJson = ($manifest | ConvertTo-Json -Depth 8) + [Environment]::NewLine
  $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
  [IO.File]::WriteAllText((Join-Path $stageRoot 'stage.json'), $stageJson, $utf8NoBom)
  Write-Host ($manifest | ConvertTo-Json -Depth 8)
} finally {
  Pop-Location
}
