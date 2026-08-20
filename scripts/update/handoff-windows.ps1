param([Parameter(Mandatory=$true)][string]$Helper,[Parameter(Mandatory=$true)][string]$RequestJson,[string]$ServiceName="VSNAgent")
$ErrorActionPreference="Stop"
$wasRunning=$false
try{$svc=Get-Service -Name $ServiceName -ErrorAction Stop;$wasRunning=$svc.Status -eq 'Running';if($wasRunning){Stop-Service -Name $ServiceName -Force; $svc.WaitForStatus('Stopped',[TimeSpan]::FromSeconds(30))}}catch{Write-Warning "VSN Agent service was not running or not installed: $($_.Exception.Message)"}
try{$response=$RequestJson | & $Helper;if($LASTEXITCODE -ne 0){throw "updater helper failed with exit code $LASTEXITCODE"};$response | Write-Output}
finally{if($wasRunning){Start-Service -Name $ServiceName}}
