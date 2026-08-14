//! syscall.rs — Hardware Syscall / Sysret Interface & User-Mode Boundary for Atulya OS.
//!
//! Configures x86_64 fast system call MSR registers:
//!   - IA32_EFER (0xC0000080): Enable SCE (System Call Extensions)
//!   - IA32_STAR (0xC0000081): Kernel (0x08) & User (0x1B/0x23) segment base selectors
//!   - IA32_LSTAR (0xC0000082): Target RIP address for naked `syscall_entry`
//!   - IA32_FMASK (0xC0000084): RFLAGS mask to clear interrupt flag (IF=0) on entry

use core::arch::naked_asm;

const MSR_EFER: u32 = 0xC000_0080;
const MSR_STAR: u32 = 0xC000_0081;
const MSR_LSTAR: u32 = 0xC000_0082;
const MSR_FMASK: u32 = 0xC000_0084;

// Syscall Numbers
pub const SYS_EXIT: u64 = 0;
pub const SYS_PRINT: u64 = 1;
pub const SYS_READ: u64 = 2;
pub const SYS_WRITE: u64 = 3;
pub const SYS_YIELD: u64 = 4;
pub const SYS_GET_TICK: u64 = 5;
pub const SYS_ALLOC: u64 = 6;
pub const SYS_INTENT: u64 = 7;

#[inline]
unsafe fn wrmsr(msr: u32, val: u64) {
    let low = (val & 0xFFFF_FFFF) as u32;
    let high = ((val >> 32) & 0xFFFF_FFFF) as u32;
    core::arch::asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high, options(nomem, nostack, preserves_flags));
}

#[inline]
unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    core::arch::asm!("rdmsr", in("ecx") msr, out("eax") low, out("edx") high, options(nomem, nostack, preserves_flags));
    ((high as u64) << 32) | (low as u64)
}

/// Initialize fast hardware SYSCALL / SYSRET MSRs.
pub fn init() {
    unsafe {
        // 1. Enable SCE (System Call Extension) in IA32_EFER (Bit 0)
        let efer = rdmsr(MSR_EFER);
        wrmsr(MSR_EFER, efer | 1);

        // 2. Configure IA32_STAR
        // Bits 47:32 -> Kernel CS (0x08) and Kernel SS (0x10)
        // Bits 63:48 -> User CS (0x20) and User SS (0x18)
        let star = ((0x0018_0008u64) << 32) | (0x0020_0008u64 << 48);
        wrmsr(MSR_STAR, star);

        // 3. Set LSTAR to our naked assembly syscall handler entry point
        wrmsr(MSR_LSTAR, syscall_entry as *const () as usize as u64);

        // 4. Set FMASK to clear Interrupt Flag (IF=0x200) and Trap Flag (TF=0x100)
        wrmsr(MSR_FMASK, 0x300);
    }

    crate::serial::serial_write_line("Hardware SYSCALL / SYSRET subsystem online.");
}

/// Low-level naked assembly entry point for `syscall`.
#[no_mangle]
#[unsafe(naked)]
unsafe extern "C" fn syscall_entry() {
    naked_asm!(
        // Save user registers on current stack
        "push rcx", // User RIP saved by hardware in RCX
        "push r11", // User RFLAGS saved by hardware in R11
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        // Call high-level Rust syscall dispatcher
        // Arguments: rdi (arg1), rsi (arg2), rdx (arg3), r10 (arg4), rax (syscall number)
        "mov rcx, r10", // Pass 4th argument in RCX per C ABI
        "call {handle_syscall}",

        // Restore user registers
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",
        "pop r11", // User RFLAGS
        "pop rcx", // User RIP

        // Return to Ring 3 User Space
        "sysretq",
        handle_syscall = sym handle_syscall,
    );
}

/// High-level Rust syscall dispatcher.
#[no_mangle]
extern "C" fn handle_syscall(
    arg1: u64,
    arg2: u64,
    _arg3: u64,
    _arg4: u64,
    syscall_nr: u64,
) -> u64 {
    match syscall_nr {
        SYS_EXIT => {
            crate::serial::serial_write_line("User task called SYS_EXIT.");
            0
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
        SYS_YIELD => {
            crate::scheduler::check_and_schedule();
            0
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
            crate::serial::serial_write_line("Unknown syscall requested.");
            u64::MAX
        }
    }
}
