$ErrorActionPreference = "Stop"

Write-Host "AtulyaOS tool setup"
Write-Host "This script installs/checks Rust and QEMU for local boot testing."

if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    Write-Host "Rust is not installed. Download rustup-init.exe from https://rustup.rs/"
    Write-Host "Then run: rustup toolchain install nightly --component rust-src --component llvm-tools-preview"
} else {
    rustup toolchain install nightly --component rust-src --component llvm-tools-preview
    cargo install bootimage
}

if (-not (Get-Command qemu-system-x86_64 -ErrorAction SilentlyContinue)) {
    Write-Host "QEMU is not installed or not on PATH."
    Write-Host "Install QEMU for Windows, then ensure qemu-system-x86_64.exe is on PATH."
}
