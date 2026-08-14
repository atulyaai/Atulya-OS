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
    pub cr3: u64,       // offset 17*8 (0x88) — hardware PML4 page table root
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
            cr3: 0,
        }
    }

    pub fn new_kernel_thread(entry: extern "C" fn(), stack_base: *mut u8, stack_size: usize) -> Self {
        let rsp = (stack_base as usize + stack_size) as u64;
        unsafe {
            let frame_top = (stack_base as usize + stack_size) as *mut u64;
            // Build the canonical 5-value x86_64 iretq frame: [SS, RSP, RFLAGS, CS, RIP]
            core::ptr::write(frame_top.sub(5), entry as u64);  // RIP
            core::ptr::write(frame_top.sub(4), 0x08u64);       // CS (Kernel Code Selector 0x08)
            core::ptr::write(frame_top.sub(3), 0x202u64);      // RFLAGS (IF=1 + bit1)
            core::ptr::write(frame_top.sub(2), rsp);           // RSP (Kernel Stack Top)
            core::ptr::write(frame_top.sub(1), 0x10u64);       // SS (Kernel Data Selector 0x10)
        }

        Context {
            rflags: 0x202,
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0,
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rsp: rsp - 40, // point at RIP (start of 5-value iretq frame)
            cr3: 0,
        }
    }

    pub fn new_user_thread(entry: usize, user_stack_top: usize, kernel_stack_base: *mut u8, kernel_stack_size: usize) -> Self {
        let rsp = (kernel_stack_base as usize + kernel_stack_size) as u64;
        unsafe {
            let frame_top = (kernel_stack_base as usize + kernel_stack_size) as *mut u64;
            // Build a 5-value frame matching CPU Ring 0 -> Ring 3 interrupt format:
            // [SS, RSP, RFLAGS, CS, RIP]
            core::ptr::write(frame_top.sub(5), entry as u64);          // RIP
            core::ptr::write(frame_top.sub(4), (0x20 | 3) as u64);     // CS (User Code Selector 0x23)
            core::ptr::write(frame_top.sub(3), 0x202u64);              // RFLAGS (IF=1 + bit1)
            core::ptr::write(frame_top.sub(2), user_stack_top as u64);  // RSP (User Stack Top)
            core::ptr::write(frame_top.sub(1), (0x18 | 3) as u64);     // SS (User Data Selector 0x1B)
        }

        Context {
            rflags: 0x202,
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0,
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rsp: rsp - 40, // point at RIP (start of 5-value iretq frame)
            cr3: 0,
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
    pub name: &'static str,
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
            name: "idle_kernel",
            state: ProcessState::Ready,
            priority: 0,
            ctx,
            stack_base,
            stack_size: STACK_SIZE,
        }
    }

    pub fn new_kernel_thread(name: &'static str, entry: extern "C" fn(), priority: u8) -> Self {
        let pid = NEXT_PID.fetch_add(1, Ordering::Relaxed);
        let layout = core::alloc::Layout::from_size_align(STACK_SIZE, 16).unwrap();
        let stack_base = unsafe { alloc::alloc::alloc(layout) };
        if stack_base.is_null() {
            panic!("Failed to allocate process stack");
        }

        let ctx = Context::new_kernel_thread(entry, stack_base, STACK_SIZE);

        Process {
            pid,
            name,
            state: ProcessState::Ready,
            priority,
            ctx,
            stack_base,
            stack_size: STACK_SIZE,
        }
    }

    /// Create an isolated Ring 3 User-Mode process thread with its own user stack.
    pub fn new_user_thread(name: &'static str, entry: usize, priority: u8) -> Self {
        let pid = NEXT_PID.fetch_add(1, Ordering::Relaxed);
        let layout = core::alloc::Layout::from_size_align(STACK_SIZE, 16).unwrap();
        
        // Allocate Kernel Stack (used for interrupts and syscall handling)
        let kernel_stack_base = unsafe { alloc::alloc::alloc(layout) };
        // Allocate User Stack (used when executing in Ring 3)
        let user_stack_base = unsafe { alloc::alloc::alloc(layout) };

        if kernel_stack_base.is_null() || user_stack_base.is_null() {
            panic!("Failed to allocate user process stacks");
        }

        let user_stack_top = user_stack_base as usize + STACK_SIZE;
        let mut ctx = Context::new_user_thread(entry, user_stack_top, kernel_stack_base, STACK_SIZE);
        ctx.cr3 = unsafe { crate::memory::create_user_pml4() };

        Process {
            pid,
            name,
            state: ProcessState::Ready,
            priority,
            ctx,
            stack_base: kernel_stack_base,
            stack_size: STACK_SIZE,
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
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
    // Load ALL registers from Context, set RSP to the iretq frame, and execute iretq.
    // The CPU hardware automatically checks CS selector in the frame:
    //   - If Ring 0 (CS=0x08): pops [RIP, CS, RFLAGS] and resumes in kernel space.
    //   - If Ring 3 (CS=0x23): pops [RIP, CS, RFLAGS, RSP, SS] and drops to user space.
    core::arch::naked_asm!(
        // Check and reload hardware CR3 if non-zero
        "mov rax, [rdi + 0x88]",
        "test rax, rax",
        "jz 1f",
        "mov cr3, rax",
        "1:",

        // Load RSP pointing directly at the iretq frame
        "mov rsp, [rdi + 0x80]",

        // Restore all GP registers from context
        "mov rbx, [rdi + 0x08]",
        "mov rcx, [rdi + 0x10]",
        "mov rdx, [rdi + 0x18]",
        "mov rsi, [rdi + 0x20]",
        "mov rbp, [rdi + 0x30]",
        "mov r8,  [rdi + 0x38]",
        "mov r9,  [rdi + 0x40]",
        "mov r10, [rdi + 0x48]",
        "mov r11, [rdi + 0x50]",
        "mov r12, [rdi + 0x58]",
        "mov r13, [rdi + 0x60]",
        "mov r14, [rdi + 0x68]",
        "mov r15, [rdi + 0x70]",

        // Restore RAX
        "mov rax, [rdi + 0x00]",

        // Restore RDI (last GP register)
        "mov rdi, [rdi + 0x28]",

        // Return via CPU hardware iretq
        "iretq",
    );
}

/// Transition CPU directly from Ring 0 to Ring 3 User Mode using `iretq`.
pub unsafe fn enter_user_mode(entry_point: usize, user_stack_top: usize) -> ! {
    let user_data = 0x18 | 3; // User Data Selector 0x1B
    let user_code = 0x20 | 3; // User Code Selector 0x23
    let rflags = 0x202u64;    // IF=1 (Enable interrupts in user mode)

    core::arch::asm!(
        // Push 5-value 64-bit iretq stack frame:
        // [SS, RSP, RFLAGS, CS, RIP]
        "push {ss}",
        "push {rsp}",
        "push {rflags}",
        "push {cs}",
        "push {rip}",

        // Set user data segment registers
        "mov ax, {ds:x}",
        "mov ds, ax",
        "mov es, ax",

        // Execute privilege drop to Ring 3
        "iretq",
        ss = in(reg) user_data as u64,
        rsp = in(reg) user_stack_top as u64,
        rflags = in(reg) rflags,
        cs = in(reg) user_code as u64,
        rip = in(reg) entry_point as u64,
        ds = in(reg) user_data,
        options(noreturn),
    );
}
