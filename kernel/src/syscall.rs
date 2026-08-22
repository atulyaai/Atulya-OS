//! syscall.rs — Fast Ring 3 to Ring 0 Syscall/Sysret Subsystem for Atulya OS.
//!
//! Provides x86_64 fast `syscall` & `sysretq` hardware instructions:
//!   - MSR EFER (0xC0000080): Enables SCE (System Call Extension)
//!   - MSR STAR (0xC0000081): Configures Ring 0 & Ring 3 GDT Code/Data segment selectors
//!   - MSR LSTAR (0xC0000082): Sets 64-bit kernel entry point address (`syscall_entry`)
//!   - MSR FMASK (0xC0000084): Clears RFLAGS during syscall transition
//!   - Dedicated Kernel Stack switching to protect kernel from untrusted user stacks

use core::arch::naked_asm;

pub const SYS_EXIT: u64 = 0;
pub const SYS_YIELD: u64 = 1;
pub const SYS_READ: u64 = 2;
pub const SYS_WRITE: u64 = 3;
pub const SYS_SPAWN: u64 = 4;
pub const SYS_TIME: u64 = 5;
pub const SYS_ALLOC: u64 = 6;
pub const SYS_PRINT: u64 = 0x10;
pub const SYS_GET_TICK: u64 = 0x20;
pub const SYS_INTENT: u64 = 0x30;

const MSR_EFER: u32 = 0xC0000080;
const MSR_STAR: u32 = 0xC0000081;
const MSR_LSTAR: u32 = 0xC0000082;
const MSR_FMASK: u32 = 0xC0000084;

#[inline]
unsafe fn wrmsr(msr: u32, val: u64) {
    let low = (val & 0xFFFF_FFFF) as u32;
    let high = (val >> 32) as u32;
    core::arch::asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
        options(nostack, preserves_flags)
    );
}

// Dedicated 16KB Kernel Syscall Stack
static mut KERNEL_SYSCALL_STACK: [u8; 16384] = [0; 16384];
pub static mut SAVED_USER_RSP: u64 = 0;

pub fn init() {
    unsafe {
        // 1. Enable System Call Extensions (SCE) in EFER
        let mut efer_low: u32;
        let mut efer_high: u32;
        core::arch::asm!("rdmsr", in("ecx") MSR_EFER, out("eax") efer_low, out("edx") efer_high);
        efer_low |= 1; // Bit 0: SCE
        wrmsr(MSR_EFER, ((efer_high as u64) << 32) | (efer_low as u64));

        // 2. Set STAR MSR:
        // Bits 47:32 -> Kernel CS (0x08), Kernel SS (0x10)
        // Bits 63:48 -> User CS base (0x18), User SS (0x20)
        let star = (0x0008u64 << 32) | (0x0018u64 << 48);
        wrmsr(MSR_STAR, star);

        // 3. Set LSTAR to our naked assembly syscall handler entry point
        wrmsr(MSR_LSTAR, syscall_entry as *const () as usize as u64);

        // 4. Set FMASK to clear Interrupt Flag (IF=0x200) and Trap Flag (TF=0x100)
        wrmsr(MSR_FMASK, 0x300);
    }

    crate::serial::serial_write_line("Hardware SYSCALL / SYSRET subsystem online (Isolated Kernel Stack).");
}

/// Low-level naked assembly entry point for `syscall`.
#[no_mangle]
#[unsafe(naked)]
unsafe extern "C" fn syscall_entry() {
    naked_asm!(
        // 1. Save user RSP to scratch variable and switch to Kernel Syscall Stack
        "mov [rip + {user_rsp_var}], rsp",
        "lea rsp, [rip + {kernel_stack_top}]",

        // 2. Save all user registers
        "push rcx", // Hardware-saved user RIP
        "push r11", // Hardware-saved user RFLAGS
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "push r8",
        "push r9",
        "push r10",
        "push rdx",
        "push rsi",
        "push rdi",

        // 3. Call high-level Rust syscall dispatcher
        // Arguments: rdi (arg1), rsi (arg2), rdx (arg3), rcx=r10 (arg4), r8=rax (syscall number)
        "mov rcx, r10",
        "mov r8, rax",
        "call {handle_syscall}",

        // 4. Restore user registers (RAX holds return value)
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",
        "pop r11", // Restore user RFLAGS
        "pop rcx", // Restore user RIP

        // 5. Restore user RSP and return to Ring 3
        "mov rsp, [rip + {user_rsp_var}]",
        "sysretq",

        user_rsp_var = sym SAVED_USER_RSP,
        kernel_stack_top = sym KERNEL_SYSCALL_STACK_TOP,
        handle_syscall = sym handle_syscall,
    );
}

// Marker pointer pointing to the top of the 16KB stack
#[no_mangle]
static mut KERNEL_SYSCALL_STACK_TOP: *mut u8 = unsafe {
    // Top of stack is base + 16384 (stacks grow downwards)
    (&raw mut KERNEL_SYSCALL_STACK as *mut u8).add(16384)
};

/// High-level Rust syscall dispatcher.
#[no_mangle]
extern "C" fn handle_syscall(
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    syscall_nr: u64,
) -> u64 {
    match syscall_nr {
        SYS_EXIT => {
            crate::serial::serial_write_line("User task called SYS_EXIT.");
            0
        }
        SYS_YIELD => {
            crate::scheduler::check_and_schedule();
            0
        }
        SYS_READ => {
            // Read from VFS file handle
            crate::posix::sys_read(arg1 as usize, arg2 as usize, arg3 as usize) as u64
        }
        SYS_WRITE => {
            // Write to VFS file handle
            crate::posix::sys_write(arg1 as usize, arg2 as usize, arg3 as usize) as u64
        }
        SYS_ALLOC => {
            let size = arg1 as usize;
            if size > 0 && size <= 1024 * 1024 {
                let layout = core::alloc::Layout::from_size_align(size, 8).unwrap();
                let ptr = unsafe { alloc::alloc::alloc_zeroed(layout) };
                ptr as usize as u64
            } else {
                0
            }
        }
        SYS_TIME => {
            crate::interrupts::tick_counter::get() * 10 // Convert 100Hz ticks to milliseconds
        }
        SYS_PRINT => {
            let ptr = arg1 as *const u8;
            let len = arg2 as usize;
            if !ptr.is_null() && len > 0 && len < 4096 {
                let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
                if let Ok(s) = core::str::from_utf8(slice) {
                    crate::serial::serial_write_line(s);
                }
            }
            len as u64
        }
        SYS_GET_TICK => {
            crate::interrupts::tick_counter::get()
        }
        SYS_INTENT => {
            let ptr = arg1 as *const u8;
            let len = arg2 as usize;
            if !ptr.is_null() && len > 0 && len < 512 {
                let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
                if let Ok(prompt) = core::str::from_utf8(slice) {
                    let mut ai = crate::ai::AI_ENGINE.lock();
                    let res = ai.parse_intent(prompt);
                    crate::serial::serial_write_line(&res.description);
                    return res.confidence as u64;
                }
            }
            0
        }
        _ => {
            // Forward Linux POSIX syscall numbers (0..=350) to the POSIX compatibility bridge
            crate::posix::dispatch_posix_syscall(syscall_nr as usize, arg1 as usize, arg2 as usize, arg3 as usize, arg4 as usize) as u64
        }
    }
}
