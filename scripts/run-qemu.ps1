$ErrorActionPreference = "Stop"

Set-Location (Resolve-Path (Join-Path $PSScriptRoot ".."))

$qemuCandidates = @(
    "C:\Program Files\qemu\qemu-system-x86_64.exe",
    "D:\Program Files\qemu\qemu-system-x86_64.exe",
    "qemu-system-x86_64"
)

$qemu = $null
foreach ($candidate in $qemuCandidates) {
    if ((Test-Path $candidate) -or (Get-Command $candidate -ErrorAction SilentlyContinue)) {
        $qemu = $candidate
        break
    }
}

if (-not $qemu) {
    throw "QEMU not found."
}

$searchRoots = @(
    (Join-Path (Get-Location) "target"),
    (Join-Path (Get-Location) "target_build")
)

$image = Get-ChildItem -Path $searchRoots -Recurse -Filter "atulyaos-bios.img" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1 -ExpandProperty FullName

if (-not $image) {
    throw "Boot image not found. Run scripts\build.ps1 once, then rerun this script."
}

$imageInfo = Get-Item -LiteralPath $image
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  Atulya OS -- Sovereign Hardware Virtualization" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "Boot image: $($imageInfo.FullName)"
Write-Host "Boot image size: $([math]::Round($imageInfo.Length / 1MB, 2)) MB"
Write-Host "Audio: Enabled (Intel HD Audio / DirectSound PCM Engine)" -ForegroundColor Green
Write-Host "Storage: Primary IDE ATA 512MB Attached" -ForegroundColor Green

# Ensure 512MB ATA Disk exists
$diskPath = Join-Path (Get-Location) "dist\atulyaos-disk-512m.bin"
if (!(Test-Path $diskPath)) {
    $distDir = Join-Path (Get-Location) "dist"
    if (!(Test-Path $distDir)) { New-Item -ItemType Directory -Path $distDir | Out-Null }
    $diskFile = [System.IO.File]::Create($diskPath)
    $diskFile.SetLength(512 * 1024 * 1024)
    $diskFile.Close()
}

$oldPreference = $ErrorActionPreference
$ErrorActionPreference = "Continue"

# Launch QEMU with Intel HD Audio, DirectSound, Primary ATA Disk, and VirtIO-Net
& $qemu `
    -m 512M `
    -vga std `
    -drive "format=raw,file=$image,index=0,media=disk" `
    -drive "format=raw,file=$diskPath,if=ide,index=1,media=disk" `
    -audiodev "dsound,id=snd0" `
    -device "intel-hda" `
    -device "hda-duplex,audiodev=snd0" `
    -netdev "user,id=net0" `
    -device "virtio-net-pci,netdev=net0"

$ErrorActionPreference = $oldPreference
