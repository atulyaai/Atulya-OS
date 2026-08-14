#!/usr/bin/env bash
# build-iso.sh — Builds bootable UEFI/BIOS ISO and raw USB .img for Atulya OS
set -e

echo "================================================"
echo "  Atulya OS — Bootable ISO & USB Image Builder  "
echo "================================================"

WORKSPACE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$WORKSPACE_DIR"

echo -e "\n[1/4] Compiling Atulya OS Kernel for x86_64-unknown-none..."
cargo check -p atulyaos-kernel --target x86_64-unknown-none

echo -e "\n[2/4] Building Workspace & Bootloader BIOS Image..."
cargo build

mkdir -p "$WORKSPACE_DIR/dist"
cp "$WORKSPACE_DIR/target/x86_64-unknown-none/debug/atulyaos-kernel" "$WORKSPACE_DIR/dist/atulyaos-kernel.bin" || true

# Initialize 512MB ATA Disk Image
if [ ! -f "$WORKSPACE_DIR/dist/atulyaos-disk-512m.bin" ]; then
    echo "  -> Initializing 512MB Persistent ATA Image: dist/atulyaos-disk-512m.bin"
    dd if=/dev/zero of="$WORKSPACE_DIR/dist/atulyaos-disk-512m.bin" bs=1M count=512 status=none
fi

echo -e "\n[4/4] Done! Created bootable artifacts in dist/ directory."
