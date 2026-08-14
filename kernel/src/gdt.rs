//! gdt.rs — Global Descriptor Table, Task State Segment (TSS), and Ring 3 User Mode.
//!
//! Enforces CPU privilege rings and hardware memory protection:
//!   - Ring 0: Kernel Space (Full hardware I/O & memory access)
//!   - Ring 3: User Space (Sandboxed applications & WASM skills)
//!   - TSS RSP0: Dedicated kernel stack for privilege-level transitions

use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;
use spin::Lazy;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

static TSS: Lazy<TaskStateSegment> = Lazy::new(|| {
    let mut tss = TaskStateSegment::new();
    
    // Dedicated stack for double faults (IST 0)
    static mut DOUBLE_FAULT_STACK: [u8; 4096 * 4] = [0; 4096 * 4];
    let stack_start = VirtAddr::from_ptr(&raw const DOUBLE_FAULT_STACK);
    let stack_end = stack_start + (4096 * 4);
    tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = stack_end;

    // Dedicated RSP0 kernel stack for Ring 3 -> Ring 0 transitions
    static mut KERNEL_RSP0_STACK: [u8; 4096 * 4] = [0; 4096 * 4];
    let rsp0_start = VirtAddr::from_ptr(&raw const KERNEL_RSP0_STACK);
    let rsp0_end = rsp0_start + (4096 * 4);
    tss.privilege_stack_table[0] = rsp0_end;

    tss
});

pub struct Selectors {
    pub kernel_code: SegmentSelector,
    pub kernel_data: SegmentSelector,
    pub user_data: SegmentSelector,
    pub user_code: SegmentSelector,
    pub tss: SegmentSelector,
}

static GDT: Lazy<(GlobalDescriptorTable, Selectors)> = Lazy::new(|| {
    let mut gdt = GlobalDescriptorTable::new();
    let kernel_code = gdt.append(Descriptor::kernel_code_segment());
    let kernel_data = gdt.append(Descriptor::kernel_data_segment());
    let user_data = gdt.append(Descriptor::user_data_segment());
    let user_code = gdt.append(Descriptor::user_code_segment());
    let tss = gdt.append(Descriptor::tss_segment(&TSS));

    (
        gdt,
        Selectors {
            kernel_code,
            kernel_data,
            user_data,
            user_code,
            tss,
        },
    )
});

/// Initialize GDT, Segment Registers, and Task State Segment.
pub fn init() {
    use x86_64::instructions::segmentation::{Segment, CS, DS, ES, SS};
    use x86_64::instructions::tables::load_tss;

    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.kernel_code);
        DS::set_reg(GDT.1.kernel_data);
        ES::set_reg(GDT.1.kernel_data);
        SS::set_reg(GDT.1.kernel_data);
        load_tss(GDT.1.tss);
    }

    crate::serial::serial_write_line("GDT + TSS + Ring 3 Selectors initialized.");
}

pub fn get_selectors() -> &'static Selectors {
    &GDT.1
}
