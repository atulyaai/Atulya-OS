# Changelog — Atulya-OS

All notable changes to Atulya-OS are documented here.
Follows [Semantic Versioning](https://semver.org/) and [Keep a Changelog](https://keepachangelog.com/).

---

## [Unreleased]
- WebAssembly skill runtime (WASM sandbox)
- VirtIO-Net network stack
- Persistent filesystem (FAT32 over VirtIO-Block)

---

## [v0.4.0] — 2026-09-20
### Added
- 60 FPS double-buffered TrueColor (32bpp) framebuffer compositor
- Glass-style window manager with transparency and blur layers
- Holographic biometrics HUD (AXON-7 subsystem)
- Keyboard and PS/2 mouse input drivers

### Changed
- Bootloader rewritten in pure Rust (was mixed NASM/Rust)
- Compositor moved to dedicated CPU core via APIC

---

## [v0.3.0] — 2026-08-15
### Added
- x86_64 long mode bootstrap with custom GDT/IDT
- Physical memory manager (bitmap allocator)
- Virtual memory manager (4-level page tables)
- QEMU + OVMF UEFI boot target

---

## [v0.2.0] — 2026-07-01
### Added
- `no_std` Rust kernel skeleton
- VGA text-mode output
- Panic handler and stack unwinder

---

## [v0.1.0] — 2026-06-01
### Added
- Initial repo structure
- Bootloader stub
- README and project vision
