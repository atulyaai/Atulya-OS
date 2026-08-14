# AtulyaOS

AtulyaOS is being built first as a tiny real OS milestone:

```text
Bootloader
Kernel
Screen
ATULYAOS
```

The long-term direction is Rust-first:

```text
Bootloader  -> minimal assembly / Rust bootloader support
Kernel      -> Rust
Runtime     -> Rust
Skills      -> WASM
Orb/UI      -> TypeScript/WebGL later
```

## Current Milestone

Boot a Rust kernel through a bootloader-provided framebuffer and draw pixels:

```text
ATULYAOS
Animated orb
Framebuffer graphics mode
```

## Tooling Needed

- Rust toolchain through `rustup`
- QEMU for first boot tests
- Later: VMware image export

## Build Shape

This project starts from a freestanding Rust kernel. It now uses a real
framebuffer path instead of VGA text mode, so the kernel can draw pixels for
boot visuals, images, panels, and later windows.

## Boot Splash (Storyboard Frames)

The boot animation is **Choice 1 from the boot-graphics options**: the
bootloader blits 12 pre-rendered full-screen panels. This is the same
approach Apple / Windows / most desktop OSes use because it is fast,
predictable, and visually identical every time.

The storyboard image itself is **procedurally generated** by
`scripts/generate_boot_storyboard.py` (PIL + numpy), so the same
panels come out pixel-identical on every run - no AI variance, no
hand-painting, no stale assets.

Rebuild the boot art end-to-end:

```powershell
python "F:\Atulya OS\scripts\generate_boot_storyboard.py"
python "F:\Atulya OS\scripts\convert_boot_frames.py"
```

1. `generate_boot_storyboard.py`  -> `assets/images/boot_storyboard.png` (4x3 grid)
2. `convert_boot_frames.py`       -> `assets/boot_frames/frame_NN.rgb` (12 raw panels)
3. Rebuild kernel                 -> `boot_splash.rs` blits the panels to the framebuffer

To retune the look (colors, orb size, HUD positions, wordmark), edit the
constants at the top of `scripts/generate_boot_storyboard.py` and rerun
both scripts. The grid layout (4 cols x 3 rows, 58% top / 42% bottom)
must stay in sync with `convert_boot_frames.py` (`BOTTOM_FRAC`).

For the **desktop** (post-boot), the long-term plan is Choice 3
(real-time GPU/shader renderer with bloom + particles) - that lives
in `kernel/src/desktop.rs` and is a separate workstream from the
boot splash.

## Commands

```powershell
powershell -ExecutionPolicy Bypass -File "F:\Atulya OS\scripts\build.ps1"
powershell -ExecutionPolicy Bypass -File "F:\Atulya OS\scripts\run-qemu.ps1"
```
