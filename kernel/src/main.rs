#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(abi_x86_interrupt)]
#![allow(dead_code)]

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
pub mod posix;
pub mod gguf;
pub mod security;
pub mod antivirus;
pub mod vision;
mod limine;

use core::panic::PanicInfo;
use display::{Display, FrameBufferInfo, PixelFormat};
use limine::{
    BaseRevision, FramebufferRequest, HhdmRequest, MemmapRequest,
    PagingModeRequest, RequestsEndMarker, RequestsStartMarker,
};

const BACKBUFFER_SIZE: usize = 16 * 1024 * 1024;

// ── Limine boot requests (scanned by the bootloader from the .requests section).
#[used]
#[link_section = ".requests_start"]
static REQUESTS_START: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[link_section = ".requests"]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[link_section = ".requests"]
static FRAMEBUFFER: FramebufferRequest = FramebufferRequest::new();

#[used]
#[link_section = ".requests"]
static MEMMAP: MemmapRequest = MemmapRequest::new();

#[used]
#[link_section = ".requests"]
static HHDM: HhdmRequest = HhdmRequest::new();

#[used]
#[link_section = ".requests"]
static PAGING: PagingModeRequest = PagingModeRequest::PREFER_MAXIMUM;

#[used]
#[link_section = ".requests_end"]
static REQUESTS_END: RequestsEndMarker = RequestsEndMarker::new();

static mut BACKBUFFER: [u8; BACKBUFFER_SIZE] = [0; BACKBUFFER_SIZE];
static mut FB_BUFFER_PTR: *mut u8 = core::ptr::null_mut();
static mut FB_BUFFER_LEN: usize = 0;

const KERNEL_END: u64 = 0x2100000;

#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    if !BASE_REVISION.is_supported() {
        serial::serial_init();
        serial::serial_write_line("ATULYAOS: limine base revision not supported");
        loop {
            core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }

    serial::serial_init();
    serial::serial_write_line("ATULYAOS starting (limine)...");

    // ── Framebuffer ────────────────────────────────────────────────────────
    let fb_resp = FRAMEBUFFER
        .response()
        .expect("framebuffer response missing");
    let fbs = fb_resp.data().framebuffers();
    let fb = *fbs.first().expect("no framebuffer advertised");
    let buffer = if fb.size() > BACKBUFFER_SIZE {
        panic!("Framebuffer is larger than the kernel backbuffer");
    } else {
        core::slice::from_raw_parts_mut(fb.address() as *mut u8, fb.size())
    };
    let backbuffer = unsafe { &mut BACKBUFFER[..buffer.len()] };

    let bytes_per_pixel = (fb.bpp as usize + 7) / 8;
    let stride = fb.pitch as usize / bytes_per_pixel.max(1);
    let pixel_format = if fb.memory_model == 1 {
        PixelFormat::Rgb
    } else {
        PixelFormat::Unknown
    };
    let info = FrameBufferInfo {
        byte_len: buffer.len(),
        width: fb.width as usize,
        height: fb.height as usize,
        stride,
        pixel_format,
        bytes_per_pixel,
    };

    let mut display = Display { buffer, backbuffer, info };

    serial::serial_write_line("ATULYAOS framebuffer online.");

    // ── Heap (pick the largest usable memory region after the kernel).
    let hhdm = HHDM
        .response()
        .map(|r| r.data().offset)
        .unwrap_or(0);
    let (heap_phys, heap_size) = largest_heap_region_after(&MEMMAP, KERNEL_END)
        .unwrap_or_else(|| panic!("No usable memory region found for heap"));
    let heap_virt = heap_phys + hhdm;
    allocator::init(heap_virt as usize, heap_size as usize);
    serial::serial_write_hex(heap_phys);
    serial::serial_write_hex(heap_size);
    serial::serial_write_line("Heap initialized");

    gdt::init();
    syscall::init();

    let _ = fs::ata::DISK.lock().init();

    serial::serial_write_line("About to init interrupts...");
    interrupts::init();
    serial::serial_write_line("Interrupts initialized.");

    // Calibrate wall-clock timing before any animation runs.
    timer::calibrate();

    serial::serial_write_line("About to init scheduler...");
    scheduler::init();
    ai::init();

    unsafe {
        FB_BUFFER_PTR = display.buffer.as_mut_ptr();
        FB_BUFFER_LEN = display.buffer.len();
    }

    boot_splash::run(&mut display);

    // Enable hardware interrupts for interactive login & desktop.
    interrupts::enable();

    // ── Holographic Biometric Login Gate ────────────────────────────────────
    let mut login_gate = login::LoginGate::new();
    login_gate.run(&mut display);

    // ── Quantum Glass Window Manager & AI Intent Desktop ───────────────────
    serial::serial_write_line("Launching Atulya Desktop...");
    crate::desktop::run(&mut display);
}

fn largest_heap_region_after(req: &MemmapRequest, kernel_end: u64) -> Option<(u64, u64)> {
    let resp = req.response()?;
    let mut best_start = 0u64;
    let mut best_size = 0u64;
    for ent in resp.data().entries() {
        if ent.type_ != limine::MEMMAP_USABLE {
            continue;
        }
        let start = core::cmp::max(ent.base, kernel_end);
        if start >= ent.base + ent.length {
            continue;
        }
        let size = ent.base + ent.length - start;
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
