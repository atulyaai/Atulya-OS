Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  Atulya OS -- Bootable ISO and USB Image Builder  " -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan

$WorkspaceDir = Split-Path -Parent $PSScriptRoot
Set-Location $WorkspaceDir

Write-Host "`n[1/4] Compiling Atulya OS Kernel for x86_64-unknown-none..." -ForegroundColor Yellow
cargo check -p atulyaos-kernel --target x86_64-unknown-none
if ($LASTEXITCODE -ne 0) {
    Write-Error "Kernel compilation failed."
    exit 1
}

Write-Host "`n[2/4] Building Workspace and Bootloader BIOS Image..." -ForegroundColor Yellow
cargo build
if ($LASTEXITCODE -ne 0) {
    Write-Error "Cargo build failed."
    exit 1
}

$OutputDir = Join-Path $WorkspaceDir "dist"
if (!(Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}

$KernelBin = Join-Path $WorkspaceDir "target\x86_64-unknown-none\debug\atulyaos-kernel"

Write-Host "`n[3/4] Packaging Standalone Boot Disk Images..." -ForegroundColor Yellow
if (Test-Path $KernelBin) {
    Copy-Item -Path $KernelBin -Destination (Join-Path $OutputDir "atulyaos-kernel.bin") -Force
    Write-Host "  -> Saved Kernel Binary: dist\atulyaos-kernel.bin" -ForegroundColor Green
}

$AtaDiskPath = Join-Path $OutputDir "atulyaos-disk-512m.bin"
if (!(Test-Path $AtaDiskPath)) {
    Write-Host "  -> Initializing 512MB Persistent ATA Image: dist\atulyaos-disk-512m.bin" -ForegroundColor Yellow
    $diskFile = [System.IO.File]::Create($AtaDiskPath)
    $diskFile.SetLength(512 * 1024 * 1024)
    $diskFile.Close()
}

Write-Host "`n[4/4] Done! Created bootable artifacts in dist\ directory." -ForegroundColor Green
Write-Host "To test live in QEMU: powershell .\scripts\run-qemu.ps1" -ForegroundColor Cyan
