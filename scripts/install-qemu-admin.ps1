$ErrorActionPreference = "Stop"

$installer = "C:\tmp\qemu-w64-setup-20260501.exe"

if (-not (Test-Path $installer)) {
    Invoke-WebRequest `
        -Uri "https://qemu.weilnetz.de/w64/qemu-w64-setup-20260501.exe" `
        -OutFile $installer
}

Start-Process -FilePath $installer -ArgumentList "/S" -Verb RunAs -Wait

$qemu = "C:\Program Files\qemu\qemu-system-x86_64.exe"
if (-not (Test-Path $qemu)) {
    throw "QEMU was not found at $qemu after installation."
}

& $qemu --version
