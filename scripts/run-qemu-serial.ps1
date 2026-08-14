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
Write-Host "Boot image: $($imageInfo.FullName)"
Write-Host "Boot image size: $([math]::Round($imageInfo.Length / 1MB, 2)) MB"

& $qemu -accel whpx -m 256M -vga std -serial stdio -drive "format=raw,file=$image"
