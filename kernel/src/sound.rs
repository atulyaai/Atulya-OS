//! sound.rs — PC Speaker Sound Synthesizer for Atulya OS.
//!
//! Uses PIT Channel 2 (Ports 0x42 & 0x43) and PC Speaker control (Port 0x61)
//! to generate sci-fi audio frequencies, harmonic power-on chimes, and UI clicks.

use crate::math;

const PIT_FREQUENCY: u32 = 1_193_180;

/// Output byte to I/O port.
#[inline]
unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
}

/// Input byte from I/O port.
#[inline]
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
    value
}

pub struct Sound;

impl Sound {
    /// Play a square-wave tone at the given frequency in Hz.
    pub fn play_tone(freq: u32) {
        if freq == 0 {
            Self::stop_tone();
            return;
        }

        let div = (PIT_FREQUENCY / freq) as u16;

        unsafe {
            // Configure PIT Channel 2: Mode 3 (Square Wave), 16-bit binary
            outb(0x43, 0xB6);
            outb(0x42, (div & 0xFF) as u8);
            outb(0x42, (div >> 8) as u8);

            // Enable PC Speaker output (bits 0 and 1 of port 0x61)
            let ctrl = inb(0x61);
            if ctrl & 0x03 != 0x03 {
                outb(0x61, ctrl | 0x03);
            }
        }
    }

    /// Stop PC Speaker sound output.
    pub fn stop_tone() {
        unsafe {
            let ctrl = inb(0x61) & !0x03;
            outb(0x61, ctrl);
        }
    }

    /// Play a note for a given duration in spin cycles.
    pub fn beep(freq: u32, duration_cycles: usize) {
        Self::play_tone(freq);
        math::delay(duration_cycles);
        Self::stop_tone();
    }

    /// Play the signature Atulya OS Cyber Harmonic Boot Chime.
    pub fn play_boot_chime() {
        // Futuristic rising harmonic chord (C5 -> E5 -> G5 -> C6 -> E6 shimmer)
        let notes = [523, 659, 784, 1046, 1318];
        for &freq in &notes {
            Self::beep(freq, 6_000);
            math::delay(1_000);
        }
    }

    /// Play an interactive UI click / auth success sound.
    pub fn play_auth_chime() {
        Self::beep(880, 20_000);
        Self::beep(1174, 35_000);
    }
}
