param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if (-not $IsWindows) { throw '02.09 reconciliation requires Windows' }
if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw '02.09 reconciliation requires GitHub-hosted execution' }
if (-not $env:HEAD_REF) { throw 'HEAD_REF is required' }

$utf8NoBom = New-Object System.Text.UTF8Encoding($false)

function Replace-Literal([string]$Path, [string]$Old, [string]$New) {
    $resolved = Resolve-Path $Path
    $text = [IO.File]::ReadAllText($resolved)
    if (-not $text.Contains($Old)) {
        throw "canonical replacement target not found in $Path: $Old"
    }
    [IO.File]::WriteAllText($resolved, $text.Replace($Old, $New), $utf8NoBom)
}

$cert = 'certification/pkg02-usable-local-beta-v1.json'
Replace-Literal $cert '"done": 8' '"done": 9'
Replace-Literal $cert '"percent": 29.63' '"percent": 33.33'
Replace-Literal $cert '"active_task": "02.09"' '"active_task": "02.10"'
$old0209 = '{"id":"02.09","name":"Runtime inventory, registry and audit","status":"IN_PROGRESS","depends_on":["02.08"]},'
$new0209 = '{"id":"02.09","name":"Runtime inventory, registry and audit","status":"DONE","depends_on":["02.08"],"evidence":"Exact-head GitHub-hosted Windows run 32564170799 job 97010237204 passed Rust/Cargo 1.97.1 format/Clippy/tests, locked release Agent/CLI build, bounded concurrent hostile runtime probes, deterministic exact/unique provider inventory, malformed registry fail-closed behavior, duplicate/unknown/install-root-escape/dangling-activation audit detection, and audit-chain verification. Artifact 9473712178 digest sha256:fb16f9bbba45fbd03a4e896b55b2cb73efd2a302a12c464eb11a20b0041ddc3 contains evidence.json digest sha256:a9aee1b98489a9cdd0cd728609e3c3d8b6bbb01ea6d15ca35cd162913579e02d on exact source head bdb8ca7ffb84b27553b9789d1fd3f424ef681719 using GitHub-hosted Windows/X64 and loopback IPC 127.0.0.1:39731."},'
Replace-Literal $cert $old0209 $new0209
Replace-Literal $cert '{"id":"02.10","name":"Trusted runtime catalog and archive safety","status":"BLOCKED","depends_on":["02.09"]},' '{"id":"02.10","name":"Trusted runtime catalog and archive safety","status":"IN_PROGRESS","depends_on":["02.09"]},'

$status = 'docs/MASTER-EXECUTION-STATUS.json'
Replace-Literal $status '"active_task": "02.09"' '"active_task": "02.10"'
Replace-Literal $status '{"id":"PKG-02","name":"Usable Local Server Beta","done":8,"required":27,"percent":29.63,"status":"IN_PROGRESS"}' '{"id":"PKG-02","name":"Usable Local Server Beta","done":9,"required":27,"percent":33.33,"status":"IN_PROGRESS"}'
$oldNote = '"PKG-02 progress is 8/27 = 29.63%; 02.09 Runtime inventory, registry and audit is ACTIVE and no later task may be counted before its prerequisite passes."'
$newNote = '"02.09 PASS: exact-head GitHub-hosted Windows run 32564170799 job 97010237204 passed runtime inventory/registry/audit acceptance on source bdb8ca7ffb84b27553b9789d1fd3f424ef681719; artifact 9473712178 digest sha256:fb16f9bbba45fbd03a4e896b55b2cb73efd2a302a12c464eb11a20b0041ddc3; evidence.json digest sha256:a9aee1b98489a9cdd0cd728609e3c3d8b6bbb01ea6d15ca35cd162913579e02d.",`n    "02.09 verified bounded concurrent hostile runtime probes, deterministic exact/unique provider inventory, malformed registry non-zero failure with zero-byte stdout, duplicate/unknown/install-root-escape/dangling-activation audit detection, valid audit chain, GitHub-hosted Windows/X64, and loopback IPC 127.0.0.1:39731.",`n    "PKG-02 progress is 9/27 = 33.33%; 02.10 Trusted runtime catalog and archive safety is ACTIVE and no later task may be counted before its prerequisite passes."'
Replace-Literal $status $oldNote $newNote

$readme = 'README.md'
Replace-Literal $readme 'Current genuine PKG-02 progress: `8/27 = 29.63%`.' 'Current genuine PKG-02 progress: `9/27 = 33.33%`.'
Replace-Literal $readme '- `02.01` Local Agent lifecycle, `02.02` authenticated local IPC protocol enforcement, `02.03` CLI core operator path, `02.04` Desktop authenticated Agent bridge and Overview states, `02.05` workspace root persistence and containment, `02.06` project detection and dependency analysis, `02.07` project template catalog and deterministic bootstrap-plan acceptance, and `02.08` bounded retry-safe project bootstrap execution are DONE with real acceptance evidence.' '- `02.01` Local Agent lifecycle through `02.09` runtime inventory, registry and audit are DONE with real sequential acceptance evidence, including bounded retry-safe project bootstrap execution and fail-closed runtime metadata handling.'
Replace-Literal $readme '- Active task: `02.09` — Runtime inventory, registry and audit acceptance across provider-reported runtimes.' '- Active task: `02.10` — Trusted runtime catalog verification, signature/trust failure handling and archive path-safety acceptance.'

$planPath = 'docs/MASTER-EXECUTION-PLAN.md'
$planResolved = Resolve-Path $planPath
$plan = [IO.File]::ReadAllText($planResolved)
$block = @'
## Current blocker
`02.01` through `02.09` are DONE with real sequential acceptance evidence. `02.09` exact-head GitHub-hosted Windows run `32564170799`, job `97010237204`, artifact `9473712178`, artifact digest `sha256:fb16f9bbba45fbd03a4e896b55b2cb73efd2a302a12c464eb11a20b0041ddc3`, and evidence digest `sha256:a9aee1b98489a9cdd0cd728609e3c3d8b6bbb01ea6d15ca35cd162913579e02d` passed on source head `bdb8ca7ffb84b27553b9789d1fd3f424ef681719`. Acceptance verified bounded concurrent hostile runtime probes, deterministic exact/unique provider inventory, malformed registry fail-closed behavior with zero-byte stdout, duplicate registration, unknown runtime, managed install-root escape and dangling activation detection, a valid audit chain, GitHub-hosted Windows/X64 execution and loopback IPC `127.0.0.1:39731`. PKG-02 is `9/27 = 33.33%`. Execute `02.10` Trusted runtime catalog and archive safety next; do not count `02.11` or later before `02.10` is genuinely verified.
'@
$updatedPlan = [regex]::Replace($plan, '(?s)## Current blocker\r?\n.*\z', $block.TrimEnd() + "`n")
if ($updatedPlan -eq $plan) { throw 'MASTER-EXECUTION-PLAN current blocker replacement failed' }
[IO.File]::WriteAllText($planResolved, $updatedPlan, $utf8NoBom)

$workflowPath = '.github/workflows/pkg02-task-0209-runtime-inventory-registry.yml'
$workflowResolved = Resolve-Path $workflowPath
$workflowLines = @([IO.File]::ReadAllLines($workflowResolved))
$workflowLines = @($workflowLines | Where-Object { $_ -ne '      HEAD_REF: ${{ github.event.pull_request.head.ref }}' })
$permissionIndex = [Array]::IndexOf($workflowLines, '  contents: write')
if ($permissionIndex -lt 0) { throw 'temporary workflow write permission not found' }
$workflowLines[$permissionIndex] = '  contents: read'
$stepStart = [Array]::IndexOf($workflowLines, '      - name: Reconcile certified 02.09 canonical state')
$verifyStart = [Array]::IndexOf($workflowLines, '      - name: Verify exact source and GitHub-hosted Windows runner')
if ($stepStart -lt 0 -or $verifyStart -le $stepStart) { throw 'temporary reconciliation step boundaries not found' }
$cleanWorkflow = @()
if ($stepStart -gt 0) { $cleanWorkflow += $workflowLines[0..($stepStart - 1)] }
$cleanWorkflow += $workflowLines[$verifyStart..($workflowLines.Count - 1)]
[IO.File]::WriteAllText($workflowResolved, (($cleanWorkflow -join "`n") + "`n"), $utf8NoBom)

git rm -- 'scripts/self-hosted/pkg02-0209-reconcile.ps1'
if ($LASTEXITCODE -ne 0) { throw 'failed to remove temporary reconciliation helper' }
git diff --check
if ($LASTEXITCODE -ne 0) { throw 'canonical reconciliation produced invalid whitespace' }

git config user.name 'github-actions[bot]'
git config user.email '41898282+github-actions[bot]@users.noreply.github.com'
git add $cert $status $readme $planPath $workflowPath
git commit -m 'docs(pkg02): certify 02.09 and activate 02.10'
if ($LASTEXITCODE -ne 0) { throw 'canonical reconciliation commit failed' }
git push origin ("HEAD:refs/heads/" + $env:HEAD_REF)
if ($LASTEXITCODE -ne 0) { throw 'canonical reconciliation push failed' }
throw 'canonical reconciliation committed; final exact-head certification must rerun'
