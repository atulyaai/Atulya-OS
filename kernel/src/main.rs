#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(abi_x86_interrupt)]
#![allow(dead_code)] // Silence warnings on scaffolded modules (gpu, net, wasm, memory) not yet wired.

extern crate alloc;

mod boot_splash;
mod boot;
mod desktop;
mod display;
mod font;
mod math;
mod serial;
mod timer;
mod allocator;
mod memory;
mod interrupts;
mod process;
mod scheduler;
mod fs;
mod net;
mod wasm;
mod gpu;
mod login;
mod sound;
mod pci;
mod gdt;
mod ai;
mod syscall;
pub mod viewer;
pub mod pkg;
pub mod audio;
pub mod ai_model;
pub mod power;
pub mod voice;
pub mod game;
pub mod vault;

use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use core::panic::PanicInfo;
use display::Display;

static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::FixedAddress(0x1_0000_0000));
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

const BACKBUFFER_SIZE: usize = 16 * 1024 * 1024;
const PHYS_MEM_OFFSET: u64 = 0x1_0000_0000;
static mut BACKBUFFER: [u8; BACKBUFFER_SIZE] = [0; BACKBUFFER_SIZE];

static mut FB_BUFFER_PTR: *mut u8 = core::ptr::null_mut();
static mut FB_BUFFER_LEN: usize = 0;
static mut FB_INFO: bootloader_api::info::FrameBufferInfo = bootloader_api::info::FrameBufferInfo {
    byte_len: 0, width: 0, height: 0, stride: 0,
    pixel_format: bootloader_api::info::PixelFormat::Rgb,
    bytes_per_pixel: 0,
};

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    serial::serial_init();
    serial::serial_write_line("ATULYAOS starting...");

    let kernel_end: u64 = 0x2100000;
    let (heap_phys_start, heap_size) = largest_heap_region_after(boot_info, kernel_end)
        .unwrap_or_else(|| panic!("No usable memory region found for heap"));

    let framebuffer = boot_info
        .framebuffer
        .as_mut()
        .expect("AtulyaOS requires a boot framebuffer");

    let info = framebuffer.info();
    let buffer = framebuffer.buffer_mut();
    if buffer.len() > BACKBUFFER_SIZE {
        panic!("Framebuffer is larger than the kernel backbuffer");
    }

    let backbuffer = unsafe { &mut BACKBUFFER[..buffer.len()] };
    let mut display = Display { buffer, backbuffer, info };

    serial::serial_write_line("ATULYAOS framebuffer online.");
    serial::serial_write_hex(heap_phys_start);
    serial::serial_write_hex(heap_size);

    let heap_virt = heap_phys_start + PHYS_MEM_OFFSET;
    allocator::init(heap_virt as usize, heap_size as usize);
    serial::serial_write_line("Heap initialized");

    gdt::init();
    syscall::init();

    let _ = fs::ata::DISK.lock().init();

    serial::serial_write_line("About to init interrupts...");
    interrupts::init();
    serial::serial_write_line("Interrupts initialized.");

    // Calibrate real wall-clock timing (TSC vs PIT) before any animation
    // runs. This only polls hardware registers — safe before
    // interrupts::enable(), and it's what makes boot_splash/login timing
    // match real seconds instead of "however fast this CPU spins".
    timer::calibrate();

    serial::serial_write_line("About to init scheduler...");
    scheduler::init();

    ai::init();

    unsafe {
        FB_BUFFER_PTR = display.buffer.as_mut_ptr();
        FB_BUFFER_LEN = display.buffer.len();
        FB_INFO = display.info;
    }

    boot_splash::run(&mut display);

    // Enable hardware interrupts for interactive login & desktop
    interrupts::enable();

    // ── Holographic Biometric Login Gate ────────────────────────────────
    let mut login_gate = login::LoginGate::new();
    login_gate.run(&mut display);

    // ── Quantum Glass Window Manager & AI Intent Desktop ────────────────
    serial::serial_write_line("Launching Atulya Desktop...");
    crate::desktop::run(&mut display);
}

fn largest_heap_region_after(boot_info: &BootInfo, kernel_end: u64) -> Option<(u64, u64)> {
    let mut best_start = 0;
    let mut best_size = 0;

    for region in boot_info.memory_regions.iter() {
        if region.kind != bootloader_api::info::MemoryRegionKind::Usable {
            continue;
        }

        let start = core::cmp::max(region.start, kernel_end);
        if start >= region.end {
            continue;
        }

        let size = region.end - start;
        if size > best_size {
            best_start = start;
            best_size = size;
        }
    }

    (best_size >= 1024 * 1024).then_some((best_start, best_size))
}

#[alloc_error_handler]
fn alloc_error_handler(_layout: core::alloc::Layout) -> ! {
    serial::serial_write_line("ALLOC ERROR");
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial::serial_write_line("KERNEL PANIC:");
    if let Some(loc) = info.location() {
        serial::serial_write_line(loc.file());
    }
    loop {
        core::hint::spin_loop();
    }
}
