//! timer.rs — Real wall-clock timing for boot-time animation.
//!
//! `math::delay()` is a raw spin-loop with no relationship to real time —
//! its duration depends entirely on host CPU speed, which is why the boot
//! animation ran in ~1s instead of the intended 1.6s. This module fixes
//! that by calibrating the CPU's TSC (timestamp counter) against the PIT
//! (Programmable Interval Timer), which is already programmed by
//! `interrupts::init_pit()` before `boot_splash::run()` is called — we
//! just poll its countdown register directly instead of waiting for an
//! IRQ, so this works even before `interrupts::enable()` is called.
//!
//! Once calibrated, `now_ms()` / `delay_ms()` give real, hardware-speed-
//! independent millisecond timing for the boot/login animations.

use core::sync::atomic::{AtomicU64, Ordering};

const PIT_FREQ_HZ: u64 = 1_193_182;
const PIT_CHANNEL0_DATA: u16 = 0x40;
const PIT_COMMAND: u16 = 0x43;

static TSC_HZ: AtomicU64 = AtomicU64::new(0);
static TSC_AT_BOOT: AtomicU64 = AtomicU64::new(0);

#[inline(always)]
pub fn rdtsc() -> u64 {
    // SAFETY: RDTSC is available on all x86_64 CPUs we target.
    unsafe {
        let hi: u32;
        let lo: u32;
        core::arch::asm!("rdtsc", out("eax") lo, out("edx") hi, options(nomem, nostack));
        ((hi as u64) << 32) | (lo as u64)
    }
}

unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") value);
}

unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!("in al, dx", out("al") value, in("dx") port);
    value
}

/// Latch and read PIT channel 0's current 16-bit countdown value.
/// Does not require interrupts — this is a direct hardware read.
fn read_pit_count() -> u16 {
    unsafe {
        // Latch command for channel 0 (freezes the count for reading).
        outb(PIT_COMMAND, 0x00);
        let lo = inb(PIT_CHANNEL0_DATA) as u16;
        let hi = inb(PIT_CHANNEL0_DATA) as u16;
        (hi << 8) | lo
    }
}

/// Calibrate TSC frequency by polling the PIT's free-running countdown
/// (channel 0, already programmed to rate-generator mode at 100Hz by
/// `interrupts::init_pit()`) across a fixed number of PIT ticks.
///
/// Must be called once, early, before relying on `now_ms()`/`delay_ms()`.
/// Safe to call before `interrupts::enable()` — this only polls hardware
/// registers, it doesn't touch the IDT/PIC.
pub fn calibrate() {
    // The PIT channel 0 divider was set for ~100Hz (~11932 counts,
    // counting down from ~11932 to 0 and reloading). We measure the TSC
    // delta across roughly 3 full countdown reloads (~30ms) — enough to
    // average out jitter from the read/latch overhead without stalling
    // boot noticeably.
    const RELOADS_TO_WAIT: u32 = 3;

    let start_count = read_pit_count();
    let start_tsc = rdtsc();

    let mut last_count = start_count;
    let mut reloads = 0u32;

    // A reload is detected when the count jumps UP (it counts down, then
    // wraps back to the divider value). Bound the loop generously in case
    // something's off, so we never hang boot forever.
    let mut safety = 20_000_000u32;
    while reloads < RELOADS_TO_WAIT && safety > 0 {
        let count = read_pit_count();
        if count > last_count {
            reloads += 1;
        }
        last_count = count;
        safety -= 1;
    }

    let end_tsc = rdtsc();

    if safety == 0 {
        // PIT read failed for some reason (unexpected hardware/hypervisor
        // behavior) — fall back to a conservative assumed TSC frequency
        // (1 GHz) rather than dividing by zero / leaving timing broken.
        TSC_HZ.store(1_000_000_000, Ordering::Relaxed);
    } else {
        let elapsed_pit_counts = (start_count as u64) + (reloads as u64 - 1) * divider_estimate()
            + (divider_estimate() - last_count as u64);
        let elapsed_tsc = end_tsc.saturating_sub(start_tsc);
        if elapsed_pit_counts > 0 {
            let hz = elapsed_tsc.saturating_mul(PIT_FREQ_HZ) / elapsed_pit_counts;
            TSC_HZ.store(hz.max(1), Ordering::Relaxed);
        } else {
            TSC_HZ.store(1_000_000_000, Ordering::Relaxed);
        }
    }

    TSC_AT_BOOT.store(rdtsc(), Ordering::Relaxed);

    crate::serial::serial_write_line("Timer calibrated (TSC vs PIT).");
}

fn divider_estimate() -> u64 {
    PIT_FREQ_HZ / 100 // matches TARGET_HZ = 100 in interrupts::init_pit()
}

/// Get calibrated TSC frequency in Hertz.
pub fn get_tsc_hz() -> u64 {
    TSC_HZ.load(Ordering::Relaxed).max(1_000_000_000)
}

/// Milliseconds elapsed since `calibrate()` was called.
pub fn now_ms() -> u32 {
    let hz = TSC_HZ.load(Ordering::Relaxed).max(1);
    let base = TSC_AT_BOOT.load(Ordering::Relaxed);
    let delta = rdtsc().saturating_sub(base);
    ((delta.saturating_mul(1000)) / hz) as u32
}

/// Busy-wait for real, calibrated milliseconds (not spin-loop guesswork).
pub fn delay_ms(ms: u32) {
    let hz = TSC_HZ.load(Ordering::Relaxed).max(1);
    let target_cycles = hz.saturating_mul(ms as u64) / 1000;
    let start = rdtsc();
    while rdtsc().saturating_sub(start) < target_cycles {
        core::hint::spin_loop();
    }
}
