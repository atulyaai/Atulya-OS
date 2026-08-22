$ErrorActionPreference = "Stop"

Set-Location (Resolve-Path (Join-Path $PSScriptRoot ".."))

Write-Host "Starting AtulyaOS Build..." -ForegroundColor Cyan
$startTime = Get-Date

# Remove the artificial job limit to allow multi-core compilation
if ($env:CARGO_BUILD_JOBS) { Remove-Item Env:CARGO_BUILD_JOBS }

cargo build -p atulyaos-kernel --target x86_64-unknown-none --release
cargo build --release

$endTime = Get-Date
$duration = $endTime - $startTime
Write-Host "Atulya OS Release Build Completed in $([math]::Round($duration.TotalSeconds, 2)) seconds." -ForegroundColor Green
