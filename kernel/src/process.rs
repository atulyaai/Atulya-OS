use core::sync::atomic::{AtomicU32, Ordering};

static NEXT_PID: AtomicU32 = AtomicU32::new(1);

/// Saved CPU register state for context switching.
/// Layout must match the assembly in context_switch/restore_context.
/// RIP is NOT saved here — it's captured via RSP pointing at the
/// interrupt frame (which iretq pops to get RIP/CS/RFLAGS/RSP/SS).
#[repr(C)]
pub struct Context {
    pub rax: u64,       // offset 0*8
    pub rbx: u64,       // offset 1*8
    pub rcx: u64,       // offset 2*8
    pub rdx: u64,       // offset 3*8
    pub rsi: u64,       // offset 4*8
    pub rdi: u64,       // offset 5*8
    pub rbp: u64,       // offset 6*8
    pub r8: u64,        // offset 7*8
    pub r9: u64,        // offset 8*8
    pub r10: u64,       // offset 9*8
    pub r11: u64,       // offset 10*8
    pub r12: u64,       // offset 11*8
    pub r13: u64,       // offset 12*8
    pub r14: u64,       // offset 13*8
    pub r15: u64,       // offset 14*8
    pub rflags: u64,    // offset 15*8
    pub rsp: u64,       // offset 16*8 — must point at interrupt frame for iretq
}

impl Context {
    pub fn new_idle() -> Self {
        Context {
            rflags: 0x200, // IF=1 (enable interrupts)
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0,
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rsp: 0, // set below after stack allocation
        }
    }

    pub fn new_kernel_thread(entry: extern "C" fn(), stack_base: *mut u8, stack_size: usize) -> Self {
        let rsp = (stack_base as usize + stack_size) as u64;
        unsafe {
            let frame_top = (stack_base as usize + stack_size) as *mut u64;
            // Build a 3-value frame matching CPU interrupt format: [RIP, CS, RFLAGS].
            // Stack grows downward. frame_top is one past the end.
            core::ptr::write(frame_top.sub(3), entry as u64);  // RIP
            core::ptr::write(frame_top.sub(2), 0x08u64);       // CS
            core::ptr::write(frame_top.sub(1), 0x202u64);      // RFLAGS (IF=1 + bit1)
        }

        Context {
            rflags: 0x202,
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0,
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rsp: rsp - 24, // point at RIP (start of 3-value frame)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Ready,
    Running,
    Blocked,
    Dead,
}

pub struct Process {
    pub pid: u32,
    pub state: ProcessState,
    pub priority: u8,
    pub ctx: Context,
    stack_base: *mut u8,
    stack_size: usize,
}

unsafe impl Send for Process {}
unsafe impl Sync for Process {}

const STACK_SIZE: usize = 4096 * 4;

extern "C" fn idle_entry() {
    loop {
        unsafe { core::arch::asm!("hlt"); }
    }
}

impl Process {
    pub fn new_idle() -> Self {
        let pid = NEXT_PID.fetch_add(1, Ordering::Relaxed);
        let layout = core::alloc::Layout::from_size_align(STACK_SIZE, 16).unwrap();
        let stack_base = unsafe { alloc::alloc::alloc(layout) };
        if stack_base.is_null() {
            panic!("Failed to allocate idle process stack");
        }

        let mut ctx = Context::new_idle();
        // Set RSP to top of kernel stack (where the interrupt frame will be)
        ctx.rsp = (stack_base as usize + STACK_SIZE) as u64;

        Process {
            pid,
            state: ProcessState::Ready,
            priority: 0,
            ctx,
            stack_base,
            stack_size: STACK_SIZE,
        }
    }

    pub fn new_kernel_thread(entry: extern "C" fn(), priority: u8) -> Self {
        let pid = NEXT_PID.fetch_add(1, Ordering::Relaxed);
        let layout = core::alloc::Layout::from_size_align(STACK_SIZE, 16).unwrap();
        let stack_base = unsafe { alloc::alloc::alloc(layout) };
        if stack_base.is_null() {
            panic!("Failed to allocate process stack");
        }

        let ctx = Context::new_kernel_thread(entry, stack_base, STACK_SIZE);

        Process {
            pid,
            state: ProcessState::Ready,
            priority,
            ctx,
            stack_base,
            stack_size: STACK_SIZE,
        }
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn state(&self) -> ProcessState {
        self.state
    }

    pub fn set_state(&mut self, state: ProcessState) {
        self.state = state;
    }

    pub fn ctx(&self) -> &Context {
        &self.ctx
    }

    pub fn ctx_mut(&mut self) -> *mut Context {
        &mut self.ctx as *mut Context
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        let layout = core::alloc::Layout::from_size_align(self.stack_size, 16).unwrap();
        unsafe {
            alloc::alloc::dealloc(self.stack_base, layout);
        }
    }
}

#[no_mangle]
#[unsafe(naked)]
pub unsafe extern "C" fn restore_context(ctx: *const Context) {
    // Load ALL registers from context. We read saved RIP from the interrupt
    // frame at [ctx.rsp] and jump to it via R10 (the only register we don't
    // fully restore — R10 gets the saved RIP instead of ctx.r10).
    core::arch::naked_asm!(
        // Load all GP registers from context
        "mov rbx, [rdi + 0x08]",
        "mov rcx, [rdi + 0x10]",
        "mov rdx, [rdi + 0x18]",
        "mov rsi, [rdi + 0x20]",
        "mov rbp, [rdi + 0x30]",
        "mov r8,  [rdi + 0x38]",
        "mov r9,  [rdi + 0x40]",
        "mov r11, [rdi + 0x50]",
        "mov r12, [rdi + 0x58]",
        "mov r13, [rdi + 0x60]",
        "mov r14, [rdi + 0x68]",
        "mov r15, [rdi + 0x70]",
        // Load RSP pointing at interrupt frame [RIP, CS, RFLAGS]
        "mov rsp, [rdi + 0x80]",
        // Read saved RIP from interrupt frame into R10
        "mov r10, [rsp]",
        // Load RAX from context (rdi still valid)
        "mov rax, [rdi + 0x00]",
        // Restore RDI from context (reads old rdi, writes new rdi — atomic)
        "mov rdi, [rdi + 0x28]",
        // Skip the interrupt frame [RIP, CS, RFLAGS] = 24 bytes
        "add rsp, 24",
        "sti",
        "jmp r10",
    );
}
