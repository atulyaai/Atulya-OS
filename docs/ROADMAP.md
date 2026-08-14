# 🗺️ Atulya OS — Official Engineering Roadmap

> **The Intent Operating System**: A freestanding, memory-safe, multi-layered x86_64 OS written in pure Rust (`no_std`) with an autonomous AI core and hardware-isolated execution boundaries.

---

## 🏛️ Architectural Sequence (Dependency-Ordered)

```
┌─────────────────────────────────────────────────────────────┐
│  STEP 1: ATA Disk Persistence                               │  [COMPLETED]
│  • Hardware IDE PIO driver (Ports 0x1F0-0x1F7)              │
│  • On-disk superblock & sector serialization (LBA 2048)     │
│  • State & user workspace survive across system reboots     │
├─────────────────────────────────────────────────────────────┤
│  STEP 2: Ring 3 User-Mode & Hardware Syscall Gate           │  [IN PROGRESS]
│  • GDT User Code (0x23) / User Data (0x1B) + TSS RSP0       │
│  • MSR-based SYSCALL/SYSRET (IA32_LSTAR, IA32_STAR)         │
│  • Hardware-enforced fault isolation between apps & kernel  │
├─────────────────────────────────────────────────────────────┤
│  STEP 3: Sandboxed WASM Skills Runtime                      │  [IN PROGRESS]
│  • WebAssembly stack machine bytecode execution             │
│  • Host graphics, math, and AI memory graph bindings        │
│  • Dynamic skill loading from /apps/*.wasm via shell        │
├─────────────────────────────────────────────────────────────┤
│  STEP 4: Consolidated Float Shader GPU Compositor           │  [COMPLETED]
│  • Unified graphics pipeline in kernel/src/gpu/             │
│  • True floating-point audio waveforms, bloom, & glass      │
├─────────────────────────────────────────────────────────────┤
│  STEP 5: VirtIO Network Driver & TCP/IP Stack               │  [PLANNED]
│  • PCI Bus Enumerator + VirtIO-Net packet streaming         │
│  • smoltcp TCP/IP stack for cloud AI model synchronicity    │
└─────────────────────────────────────────────────────────────┘
```

---

## ⚡ Syscall ABI Specification (`kernel/src/syscall.rs`)

Atulya OS uses the fast hardware `syscall` / `sysretq` instruction pair on x86_64.

### Register Calling Convention
- **RAX**: Syscall Number
- **RDI**: Argument 1
- **RSI**: Argument 2
- **RDX**: Argument 3
- **R10**: Argument 4 (replaces RCX which is overwritten by hardware `syscall` instruction)
- **R8**:  Argument 5
- **R9**:  Argument 6
- **RAX**: Return Value

### Syscall Table
| Number | Identifier | Description | Arguments | Return |
| :--- | :--- | :--- | :--- | :--- |
| `0` | `SYS_EXIT` | Terminate current user task | `code: i32` | `!` |
| `1` | `SYS_PRINT` | Print string to terminal | `ptr: *const u8, len: usize` | `bytes_written` |
| `2` | `SYS_READ` | Read from file / standard input | `handle: u64, buf: *mut u8, len: usize` | `bytes_read` |
| `3` | `SYS_WRITE` | Write to file / persistent disk | `handle: u64, buf: *const u8, len: usize` | `bytes_written` |
| `4` | `SYS_YIELD` | Yield CPU timeslice to next task | `None` | `0` |
| `5` | `SYS_GET_TICK` | Read global system tick count | `None` | `ticks: u64` |
| `6` | `SYS_ALLOC` | Allocate memory page for user task | `pages: usize` | `virt_addr: u64` |
| `7` | `SYS_INTENT` | Dispatch prompt to AI Intent Subsystem | `prompt_ptr: *const u8, len: usize` | `intent_id: u32` |

---

## 📁 Memory & Privilege Layout

```
0x0000_0000_0000_0000 ─┐
                      │  User Space (Ring 3)
                      │  • WASM Skills Memory (64KB - 4MB)
                      │  • User Stack
0x0000_7FFF_FFFF_FFFF ─┘
═══════════════════════ (Non-canonical Address Gap)
0xFFFF_8000_0000_0000 ─┐
                      │  Kernel Space (Ring 0)
                      │  • Kernel Binary (.text, .rodata, .data)
                      │  • Framebuffer Linear Backbuffer (16MB)
                      │  • Heap Allocator (Linked List)
                      │  • TSS RSP0 Kernel Stack (Privilege Transitions)
0xFFFF_FFFF_FFFF_FFFF ─┘
```
