# AtulyaOS Implementation Plan: All 6 Features

## Overview

Implementation of 6 major features across 5 phases, with strict dependency ordering.

## Dependency Graph

```
Phase 0: Heap + Memory + Interrupts  (foundation)
    │
    ├── Phase 1a: Process Scheduling
    ├── Phase 1b: Filesystem
    │
    ├── Phase 2a: Network Stack
    ├── Phase 2b: WASM Runtime
    │
    ├── Phase 3: GPU/Shader Renderer (VirtIO + shader interpreter)
    │
    └── Phase 4: UEFI Boot Testing
```

## Phase 0: OS Foundation

### 0a. Heap Allocator

**New files:**
- `kernel/src/allocator.rs`

**Modified files:**
- `kernel/Cargo.toml` — add `linked-list-allocator = "0.10"`, `spin = "0.9"`, `x86_64 = "0.15"`
- `kernel/src/main.rs` — add `#![feature(alloc_error_handler)]`, `#[global_allocator]`, init call

**Implementation steps:**
1. Add `linked-list-allocator`, `spin`, `x86_64` to kernel dependencies
2. Create `allocator.rs` with a `LockedHeap` wrapper
3. In `kernel_main`, after getting `BootInfo`:
   - Find a usable memory region from `boot_info.memory_map` that's after the kernel binary and framebuffer
   - Calculate heap start/end addresses
   - Call `allocator::init(heap_start, heap_size)`
4. Add `#[alloc_error_handler]` for OOM

### 0b. Virtual Memory

**New files:**
- `kernel/src/memory.rs`

**Implementation steps:**
1. Use `x86_64::structures::page::OffsetPageTable` (physical offset from bootloader info)
2. Create `MemoryMapper` struct wrapping the page table
3. Implement `map_to()` for heap pages
4. Implement `identity_map()` for MMIO regions (VGA at 0xB8000, PCI config at 0xCF8)
5. Map the framebuffer physical address into kernel virtual space

### 0c. Interrupts (IDT + PIC + Timer)

**New files:**
- `kernel/src/interrupts.rs`

**Modified files:**
- `kernel/src/desktop.rs` — replace PS/2 polling with event queue
- `kernel/src/main.rs` — init interrupts before desktop

**Implementation steps:**
1. Initialize PIC: `ChainedPics::new(0x20, 0x28).initialize()`
2. Create `InterruptDescriptorTable` with handlers:
   - IRQ 0 (0x20): PIT timer — increment global tick counter
   - IRQ 1 (0x21): Keyboard — push scancode to event queue
   - IRQ 12 (0x2C): Mouse — push mouse byte to event queue
   - 0x08: Double fault handler
   - 0x0E: Page fault handler (debug log)
3. Configure PIT channel 0: mode 3, divisor = 1193182 / 100 = 11932 (100 Hz)
4. Create event ring buffer for keyboard/mouse:
   ```rust
   static KEYBOARD_QUEUE: Mutex<RingBuffer<u8, 256>> = ...;
   static MOUSE_QUEUE: Mutex<RingBuffer<u8, 256>> = ...;
   ```
5. Refactor `desktop.rs`:
   - Remove `unsafe { inb(0x64) }` polling loop
   - Read from `KEYBOARD_QUEUE` and `MOUSE_QUEUE` instead
   - Keep the same parsing logic (3-byte mouse packets, scancode translation)

## Phase 1a: Process Scheduling

**New files:**
- `kernel/src/process.rs`
- `kernel/src/scheduler.rs`

**Implementation steps:**
1. Define `Process` struct:
   ```rust
   struct Process {
       pid: u32,
       state: ProcessState,
       stack: AllocatedStack,  // 4KB kernel stack
       saved_registers: CpuRegisters,
       priority: u8,
   }
   ```
2. Implement context switch (inline asm):
   - Save callee-saved registers (RBX, RBP, R12-R15, RSP) of current process
   - Load saved registers of next process
   - `ret` to next process's instruction pointer
3. Round-robin scheduler:
   - Timer IRQ 0 calls `scheduler::tick()`
   - Every 10 ticks (100ms), context switch to next Ready process
4. PID 0: idle process (`hlt` loop)
5. PID 1: desktop process (migrate `desktop::run` into it)
6. Spawn new processes from terminal: `run <name>`

## Phase 1b: Filesystem

**New files:**
- `kernel/src/fs/mod.rs`
- `kernel/src/fs/vfs.rs`
- `kernel/src/fs/ramdisk.rs`

**Implementation steps:**
1. VFS trait:
   ```rust
   pub trait Filesystem {
       fn open(&self, path: &str) -> Result<FileHandle, FsError>;
       fn read(&self, handle: FileHandle, buf: &mut [u8]) -> Result<usize, FsError>;
       fn write(&mut self, handle: FileHandle, buf: &[u8]) -> Result<usize, FsError>;
       fn ls(&self, path: &str) -> Result<&[DirEntry], FsError>;
       fn mkdir(&mut self, path: &str) -> Result<(), FsError>;
   }
   ```
2. RAM disk: embed TAR archive at compile time via `include_bytes!`
3. Parse TAR headers, expose as read-only filesystem
4. Add to terminal: `ls`, `cat`, `mkdir`, `touch`, `rm` commands

## Phase 2a: Network Stack

**New files:**
- `kernel/src/net/mod.rs`
- `kernel/src/net/nic.rs`
- `kernel/src/pci.rs`

**New dependency:** `smoltcp`, `virtio-drivers`

**Implementation steps:**
1. PCI enumeration: read config space at 0xCF8/0xCFC
2. VirtIO-Net driver via `virtio-drivers` crate
3. smoltcp integration: `EthernetInterface`, TCP/UDP sockets
4. Terminal commands: `ping`, `ifconfig`

## Phase 2b: WASM Skills Runtime

**New files:**
- `kernel/src/wasm/mod.rs`
- `kernel/src/wasm/runtime.rs`

**New dependency:** `wasmi = "0.45"` (no_std)

**Implementation steps:**
1. Load `.wasm` bytes from filesystem
2. Create wasmi `Engine` and `Store`
3. Register host functions:
   - `env.draw_pixel(x, y, r, g, b)`
   - `env.draw_rect(x, y, w, h, r, g, b)`
   - `env.draw_text(x, y, ptr, len)`
   - `env.get_screen_size() -> (w, h)`
   - `env.read_input() -> u32`
   - `env.get_ticks() -> u64`
4. Instantiate and run WASM module's `main` export
5. Terminal: `skills list`, `run <skill.wasm>`

## Phase 3: GPU/Shader Renderer

**New files:**
- `kernel/src/gpu/mod.rs`
- `kernel/src/gpu/shader.rs`
- `kernel/src/gpu/virtio_gpu.rs`

**Implementation steps:**
1. VirtIO GPU driver via `virtio-drivers` crate
   - 2D resource creation, scanout, transfer
   - Fall back to CPU rendering if no GPU
2. Shader interpreter (CPU-side):
   - Define fragment shader: `fn main(uv: vec2) -> vec4`
   - Simple language or WGSL subset
   - Per-pixel execution on CPU
3. Desktop effects:
   - Window blur (glassmorphism)
   - Bloom on glowing elements
   - Per-pixel lighting on dock icons

## Phase 4: UEFI Boot Testing

**New files:**
- `scripts/run-qemu-uefi.ps1`

**Implementation steps:**
1. Download OVMF firmware
2. Create QEMU launch script using UEFI image + OVMF
3. Test both BIOS and UEFI boot paths
4. Update README with UEFI instructions

## Estimated File Count

| Phase | New Files | Modified Files |
|-------|-----------|----------------|
| 0 | 3 | 3 |
| 1a | 2 | 2 |
| 1b | 3 | 1 |
| 2a | 3 | 2 |
| 2b | 2 | 1 |
| 3 | 3 | 2 |
| 4 | 1 | 2 |
| **Total** | **17** | **13** |
