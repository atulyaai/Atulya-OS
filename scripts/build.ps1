$ErrorActionPreference = "Stop"

Set-Location (Resolve-Path (Join-Path $PSScriptRoot ".."))

Write-Host "Starting AtulyaOS Build..." -ForegroundColor Cyan
$startTime = Get-Date

# Remove the artificial job limit to allow multi-core compilation
if ($env:CARGO_BUILD_JOBS) { Remove-Item Env:CARGO_BUILD_JOBS }

cargo build --quiet

$endTime = Get-Date
$duration = $endTime - $startTime
Write-Host "Build Completed in $($duration.TotalSeconds) seconds." -ForegroundColor Green
