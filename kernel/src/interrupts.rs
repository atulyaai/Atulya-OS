use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode, HandlerFunc};
use pic8259::ChainedPics;
use lazy_static::lazy_static;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.divide_error.set_handler_fn(divide_error_handler);
        idt.debug.set_handler_fn(debug_handler);
        idt.non_maskable_interrupt.set_handler_fn(non_maskable_interrupt_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.overflow.set_handler_fn(overflow_handler);
        idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.device_not_available.set_handler_fn(device_not_available_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);

        // Timer: use naked assembly stub for context switching.
        unsafe {
            let handler: HandlerFunc = core::mem::transmute(timer_asm_stub as *const ());
            idt[0x20].set_handler_fn(handler);
        }
        idt[0x21].set_handler_fn(keyboard_interrupt_handler);
        idt[0x2C].set_handler_fn(mouse_interrupt_handler);

        idt
    };
}

static PICS: spin::Mutex<ChainedPics> = spin::Mutex::new(
    unsafe { ChainedPics::new(0x20, 0x28) }
);

fn init_pit() {
    const PIT_FREQ: u32 = 1_193_182;
    const TARGET_HZ: u32 = 100;
    const DIVIDER: u16 = (PIT_FREQ / TARGET_HZ) as u16;

    unsafe {
        x86_64::instructions::port::Port::new(0x43).write(0x34u8);
        x86_64::instructions::port::Port::new(0x40).write((DIVIDER & 0xFF) as u8);
        x86_64::instructions::port::Port::new(0x40).write((DIVIDER >> 8) as u8);
    }
}

pub fn init() {
    IDT.load();
    unsafe {
        PICS.lock().initialize();
        // Unmask IRQ0 (timer), IRQ1 (keyboard), IRQ2 (cascade) on Master PIC (Port 0x21)
        // and IRQ12 (PS/2 mouse, line 4) on Slave PIC (Port 0xA1)
        x86_64::instructions::port::Port::<u8>::new(0x21).write(0xF8);
        x86_64::instructions::port::Port::<u8>::new(0xA1).write(0xEF);
    }
    init_pit();
    enable_sse();
    init_ps2_hardware();
    crate::serial::serial_write_line("IDT + PIC + PIT + PS/2 Mouse initialized.");
}

pub fn init_ps2_hardware() {
    unsafe {
        let mut p64 = x86_64::instructions::port::Port::<u8>::new(0x64);
        let mut p60 = x86_64::instructions::port::Port::<u8>::new(0x60);

        // 1. Enable auxiliary device (mouse)
        ps2_wait_write();
        p64.write(0xA8);

        // 2. Read command byte
        ps2_wait_write();
        p64.write(0x20);
        ps2_wait_read();
        let mut status = p60.read();

        // 3. Enable mouse interrupt (bit 1) and keyboard interrupt (bit 0), disable clock disables (bits 4 & 5)
        status |= 0x03;
        status &= !0x30;
        ps2_wait_write();
        p64.write(0x60);
        ps2_wait_write();
        p60.write(status);

        // 4. Set defaults
        ps2_mouse_write(0xF6);
        let _ = ps2_mouse_read();

        // 5. Enable data streaming
        ps2_mouse_write(0xF4);
        let _ = ps2_mouse_read();
    }
}

unsafe fn ps2_wait_write() {
    let mut p = x86_64::instructions::port::Port::<u8>::new(0x64);
    for _ in 0..100_000 {
        if p.read() & 0x02 == 0 { return; }
        core::hint::spin_loop();
    }
}

unsafe fn ps2_wait_read() {
    let mut p = x86_64::instructions::port::Port::<u8>::new(0x64);
    for _ in 0..100_000 {
        if p.read() & 0x01 != 0 { return; }
        core::hint::spin_loop();
    }
}

unsafe fn ps2_mouse_write(val: u8) {
    let mut p64 = x86_64::instructions::port::Port::<u8>::new(0x64);
    let mut p60 = x86_64::instructions::port::Port::<u8>::new(0x60);
    ps2_wait_write();
    p64.write(0xD4);
    ps2_wait_write();
    p60.write(val);
}

unsafe fn ps2_mouse_read() -> u8 {
    let mut p60 = x86_64::instructions::port::Port::<u8>::new(0x60);
    ps2_wait_read();
    p60.read()
}

pub fn enable_sse() {
    unsafe {
        use core::arch::asm;
        let mut cr0: u64;
        asm!("mov {}, cr0", out(reg) cr0);
        cr0 &= !(1 << 2); // clear EM
        cr0 |= 1 << 1;    // set MP
        asm!("mov cr0, {}", in(reg) cr0);

        let mut cr4: u64;
        asm!("mov {}, cr4", out(reg) cr4);
        cr4 |= 1 << 9;  // set OSFXSR
        cr4 |= 1 << 10; // set OSXMMEXCPT
        asm!("mov cr4, {}", in(reg) cr4);
    }
    crate::serial::serial_write_line("SSE enabled.");
}

pub fn enable() {
    x86_64::instructions::interrupts::enable();
    crate::serial::serial_write_line("Interrupts enabled.");
}

// ── Naked timer interrupt stub ─────────────────────────────────────
//
// Saves ALL GP registers + RFLAGS before any Rust code runs, then
// calls timer_schedule_rust for scheduling logic. Returns next
// Context* or null.
//
// Stack layout after all pushes (16 pushes × 8 = 128 bytes):
//   [RSP+0]:   RFLAGS (pushfq)
//   [RSP+8]:   R15
//   ...
//   [RSP+120]: RAX
//   [RSP+128]: CPU-pushed RIP
//   [RSP+136]: CPU-pushed CS
//   [RSP+144]: CPU-pushed RFLAGS

#[unsafe(naked)]
pub unsafe extern "C" fn timer_asm_stub() {
    core::arch::naked_asm!(
        // Save all GP registers + RFLAGS
        "push rax",
        "push rbx",
        "push rcx",
        "push rdx",
        "push rsi",
        "push rdi",
        "push rbp",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "pushfq",

        // Send EOI to PIC
        "mov al, 0x20",
        "out 0x20, al",

        // Pass pointer to saved register block as first argument
        "mov rdi, rsp",
        "call {schedule}",

        // rax = next Context* or null
        "test rax, rax",
        "jz 2f",

        // Switch: load next context and iretq
        "mov rdi, rax",
        "jmp {restore}",

        // No switch: restore all registers and iretq
        "2:",
        "popfq",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rbp",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop rcx",
        "pop rbx",
        "pop rax",
        "iretq",

        schedule = sym timer_schedule_rust,
        restore = sym crate::process::restore_context,
    );
}

/// Called from the naked timer stub. Receives a pointer to the saved
/// register block on the stack. Returns next Context* or null.
#[no_mangle]
pub extern "C" fn timer_schedule_rust(block: *const u64) -> *const crate::process::Context {
    let should_switch = crate::scheduler::tick();

    if !should_switch {
        return core::ptr::null();
    }

    let current_ctx = crate::scheduler::current_ctx_mut();
    if current_ctx.is_null() {
        return core::ptr::null();
    }

    // Save register values from the assembly-saved block into the process context.
    unsafe {
        let b = block;
        (*current_ctx).rflags = *b.add(0);
        (*current_ctx).r15    = *b.add(1);
        (*current_ctx).r14    = *b.add(2);
        (*current_ctx).r13    = *b.add(3);
        (*current_ctx).r12    = *b.add(4);
        (*current_ctx).r11    = *b.add(5);
        (*current_ctx).r10    = *b.add(6);
        (*current_ctx).r9     = *b.add(7);
        (*current_ctx).r8     = *b.add(8);
        (*current_ctx).rbp    = *b.add(9);
        (*current_ctx).rdi    = *b.add(10);
        (*current_ctx).rsi    = *b.add(11);
        (*current_ctx).rdx    = *b.add(12);
        (*current_ctx).rcx    = *b.add(13);
        (*current_ctx).rbx    = *b.add(14);
        (*current_ctx).rax    = *b.add(15);
        // CPU interrupt frame starts at block+128 (after 16 pushes)
        (*current_ctx).rsp = b.add(16) as u64;
    }

    crate::scheduler::pick_next_and_switch()
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let scancode: u8 = unsafe {
        x86_64::instructions::port::Port::new(0x60).read()
    };
    keyboard_handler::handle_scancode(scancode);
    unsafe {
        PICS.lock().notify_end_of_interrupt(0x21);
    }
}

extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let data: u8 = unsafe {
        x86_64::instructions::port::Port::new(0x60).read()
    };
    mouse_handler::handle_byte(data);
    unsafe {
        PICS.lock().notify_end_of_interrupt(0x2C);
    }
}

extern "x86-interrupt" fn divide_error_handler(_stack_frame: InterruptStackFrame) {
    panic!("Divide error");
}

extern "x86-interrupt" fn debug_handler(_stack_frame: InterruptStackFrame) {}

extern "x86-interrupt" fn non_maskable_interrupt_handler(_stack_frame: InterruptStackFrame) {
    panic!("Non-maskable interrupt");
}

extern "x86-interrupt" fn breakpoint_handler(_stack_frame: InterruptStackFrame) {}

extern "x86-interrupt" fn overflow_handler(_stack_frame: InterruptStackFrame) {}

extern "x86-interrupt" fn bound_range_exceeded_handler(_stack_frame: InterruptStackFrame) {
    panic!("Bound range exceeded");
}

extern "x86-interrupt" fn invalid_opcode_handler(_stack_frame: InterruptStackFrame) {
    panic!("Invalid opcode");
}

extern "x86-interrupt" fn device_not_available_handler(_stack_frame: InterruptStackFrame) {}

extern "x86-interrupt" fn double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("Double fault");
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use core::arch::asm;
    let cr2: u64;
    unsafe { asm!("mov {}, cr2", out(reg) cr2) }
    crate::serial::serial_write_line("PAGE_FAULT");
    crate::serial::serial_write_line("CR2:");
    crate::serial::serial_write_hex(cr2);
    crate::serial::serial_write_line("RIP:");
    crate::serial::serial_write_hex(stack_frame.instruction_pointer.as_u64());
    crate::serial::serial_write_line("ERR:");
    crate::serial::serial_write_hex(error_code.bits() as u64);
    crate::serial::serial_write_line("CS:");
    crate::serial::serial_write_hex(stack_frame.code_segment.0 as u64);
    crate::serial::serial_write_line("RFLAGS:");
    crate::serial::serial_write_hex(stack_frame.cpu_flags.bits());
    panic!("Page fault");
}

mod keyboard_handler {
    pub fn handle_scancode(scancode: u8) {
        super::KEYBOARD_QUEUE.lock().push(scancode);
    }
}

mod mouse_handler {
    pub fn handle_byte(byte: u8) {
        super::MOUSE_QUEUE.lock().push(byte);
    }
}

pub mod tick_counter {
    use core::sync::atomic::{AtomicU64, Ordering};

    static TICK_COUNT: AtomicU64 = AtomicU64::new(0);

    pub fn increment() {
        TICK_COUNT.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get() -> u64 {
        TICK_COUNT.load(Ordering::Relaxed)
    }
}

pub static KEYBOARD_QUEUE: spin::Mutex<RingBuffer<u8, 256>> = spin::Mutex::new(RingBuffer::new());
pub static MOUSE_QUEUE: spin::Mutex<RingBuffer<u8, 256>> = spin::Mutex::new(RingBuffer::new());

pub struct RingBuffer<T, const N: usize> {
    buffer: [Option<T>; N],
    head: usize,
    tail: usize,
}

impl<T: Copy, const N: usize> RingBuffer<T, N> {
    pub const fn new() -> Self {
        Self {
            buffer: [None; N],
            head: 0,
            tail: 0,
        }
    }

    pub fn push(&mut self, value: T) {
        if self.buffer[self.tail].is_some() {
            self.head = (self.head + 1) % N;
        }
        self.buffer[self.tail] = Some(value);
        self.tail = (self.tail + 1) % N;
    }

    pub fn pop(&mut self) -> Option<T> {
        let value = self.buffer[self.head].take();
        if value.is_some() {
            self.head = (self.head + 1) % N;
        }
        value
    }
}