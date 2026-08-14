//! login.rs — Holographic Biometric Login Gate for Atulya OS.
//!
//! Recreates the authorization screen from the boot sequence storyboard:
//!   - Centered glowing hexagonal fingerprint scanner with pulse animations
//!   - Interactive passcode input bar with blinking caret
//!   - Left HUD panel: Live System Metrics (CPU, RAM, Temp, Network)
//!   - Right HUD panel: Security Clearance Telemetry (Firewall, Bio-Auth, System ID: AXON-7)
//!   - Unlocks upon clicking the fingerprint sensor, typing "atulya", or pressing Enter.

use crate::display::{Display, Rgb};
use crate::font;
use crate::math;
use crate::serial;

const CYAN: Rgb = Rgb::new(0, 229, 255);
const DEEP_CYAN: Rgb = Rgb::new(0, 140, 200);
const LIGHT_CYAN: Rgb = Rgb::new(120, 220, 255);
const WHITE_CYAN: Rgb = Rgb::new(180, 240, 255);
const MAGENTA: Rgb = Rgb::new(220, 60, 255);
const GREEN: Rgb = Rgb::new(0, 230, 118);
const RED: Rgb = Rgb::new(255, 60, 60);
const DARK_BG: Rgb = Rgb::new(4, 10, 24);

pub struct LoginGate {
    passcode: [u8; 16],
    pass_len: usize,
    pulse: u16,
    rotation: i32,
    auth_success: bool,
    auth_error: bool,
    unlock_timer: u32,
}

impl LoginGate {
    pub fn new() -> Self {
        Self {
            passcode: [0; 16],
            pass_len: 0,
            pulse: 0,
            rotation: 0,
            auth_success: false,
            auth_error: false,
            unlock_timer: 0,
        }
    }

    /// Run the interactive login loop until user authorizes.
    pub fn run(&mut self, display: &mut Display) {
        serial::serial_write_line("AtulyaOS Login Gate online.");

        let mut mouse_x = (display.width() / 2) as isize;
        let mut mouse_y = (display.height() * 2 / 3) as isize;
        let mut mouse_bytes = [0u8; 3];
        let mut mouse_cycle = 0u8;
        let mut mouse_was_pressed = false;

        let mut kbd_shift = false;

        while !self.auth_success || self.unlock_timer < 30 {
            self.pulse = (self.pulse + 12) % 1024;
            self.rotation = (self.rotation + 2) % 360;

            if self.auth_success {
                self.unlock_timer += 1;
            }

            self.render(display, mouse_x, mouse_y);
            display.swap_buffers();

            // Process keyboard input
            while let Some(scancode) = crate::interrupts::KEYBOARD_QUEUE.lock().pop() {
                match scancode {
                    0x2A | 0x36 => kbd_shift = true,
                    0xAA | 0xB6 => kbd_shift = false,
                    0x1C => {
                        // Enter key -> Submit passcode / Quick unlock
                        self.try_unlock();
                    }
                    0x0E => {
                        // Backspace
                        if self.pass_len > 0 {
                            self.pass_len -= 1;
                            self.auth_error = false;
                        }
                    }
                    _ => {
                        if scancode & 0x80 == 0 {
                            let ch = translate_scancode(scancode, kbd_shift);
                            if ch.is_ascii_graphic() || ch == ' ' {
                                if self.pass_len < 16 {
                                    self.passcode[self.pass_len] = ch as u8;
                                    self.pass_len += 1;
                                    self.auth_error = false;
                                }
                            }
                        }
                    }
                }
            }

            // Process mouse input
            while let Some(b) = crate::interrupts::MOUSE_QUEUE.lock().pop() {
                mouse_bytes[mouse_cycle as usize] = b;
                mouse_cycle += 1;

                if mouse_cycle == 3 {
                    mouse_cycle = 0;
                    let flags = mouse_bytes[0];
                    if flags & 0x08 != 0 {
                        let mut dx = mouse_bytes[1] as isize;
                        let mut dy = mouse_bytes[2] as isize;
                        if flags & 0x10 != 0 { dx -= 256; }
                        if flags & 0x20 != 0 { dy -= 256; }

                        let w = display.width() as isize;
                        let h = display.height() as isize;
                        mouse_x = (mouse_x + dx).max(0).min(w - 1);
                        mouse_y = (mouse_y - dy).max(0).min(h - 1);

                        let mouse_pressed = flags & 1 != 0;

                        // Click on fingerprint scanner or submit button
                        if mouse_pressed && !mouse_was_pressed {
                            let cx = display.width() / 2;
                            let cy = display.height() * 42 / 100;
                            let dist2 = (mouse_x - cx as isize).pow(2) + (mouse_y - cy as isize).pow(2);
                            if dist2 < 55 * 55 {
                                // Biometric Touch Trigger
                                self.try_biometric_unlock();
                            }

                            // Click on passcode submit button
                            let btn_x = (cx + 125) as isize;
                            let btn_y = (display.height() * 56 / 100) as isize;
                            if (mouse_x - btn_x).abs() < 24 && (mouse_y - btn_y).abs() < 16 {
                                self.try_unlock();
                            }
                        }
                        mouse_was_pressed = mouse_pressed;
                    }
                }
            }

            // Real, calibrated ~60fps pacing (was an uncalibrated spin
            // count — same bug as the boot animation, same fix).
            crate::timer::delay_ms(16);
        }

        serial::serial_write_line("AtulyaOS Login Gate: ACCESS GRANTED.");
    }

    fn try_unlock(&mut self) {
        if !self.auth_success {
            crate::sound::Sound::play_auth_chime();
        }
        self.auth_success = true;
        self.auth_error = false;
    }

    fn try_biometric_unlock(&mut self) {
        if !self.auth_success {
            crate::sound::Sound::play_auth_chime();
        }
        self.auth_success = true;
        self.auth_error = false;
    }

    fn render(&self, display: &mut Display, mx: isize, my: isize) {
        let w = display.width();
        let h = display.height();
        let cx = w / 2;

        // Background dark obsidian space
        display.gradient_rect_v(0, 0, w, h, Rgb::new(1, 2, 4), Rgb::new(3, 4, 8));

        // Background subtle grid lines
        let grid_col = Rgb::new(0, 100, 160).dim(25);
        for x in (0..w).step_by(48) {
            for y in (0..h).step_by(8) {
                display.pixel(x, y, grid_col);
            }
        }

        // Header Title
        let header_color = if self.auth_success { GREEN } else { CYAN };
        font::centered_text(display, cx, 36, "ATULYA OS", 3, header_color);
        font::centered_text(display, cx, 68, "SECURE LOGIN - AUTHORIZATION REQUIRED", 1, WHITE_CYAN);

        // ── Center: Hexagonal Fingerprint Hologram ──────────────────────────
        let fp_y = h * 42 / 100;
        self.draw_biometric_scanner(display, cx, fp_y);

        // ── Center-Bottom: Passcode Input Box ───────────────────────────────
        let pass_y = h * 56 / 100;
        self.draw_passcode_box(display, cx, pass_y);

        // ── Left HUD Panel: System Metrics ──────────────────────────────────
        let left_x = w / 14;
        let hud_y = h * 30 / 100;
        self.draw_left_hud(display, left_x, hud_y);

        // ── Right HUD Panel: Security Telemetry ─────────────────────────────
        let right_x = w - (w / 14) - 220;
        self.draw_right_hud(display, right_x, hud_y);

        // ── Unlock Burst / Status Banner ────────────────────────────────────
        if self.auth_success {
            let banner_y = h * 68 / 100;
            display.rect_rounded_alpha(cx.saturating_sub(150), banner_y, 300, 36, 6, Rgb::new(0, 40, 20), 200);
            display.rect_rounded_outline(cx.saturating_sub(150), banner_y, 300, 36, 6, GREEN);
            font::centered_text(display, cx, banner_y + 10, "ACCESS GRANTED - WELCOME ATUL", 1, GREEN);
        } else {
            let hint_y = h * 68 / 100;
            font::centered_text(display, cx, hint_y, "TOUCH FINGERPRINT OR PRESS ENTER TO UNLOCK", 1, LIGHT_CYAN.dim(180));
        }

        // ── Draw Mouse Cursor ───────────────────────────────────────────────
        display.circle_filled(mx as usize, my as usize, 4, WHITE_CYAN);
        display.circle_outline(mx as usize, my as usize, 5, CYAN);
    }

    fn draw_biometric_scanner(&self, display: &mut Display, cx: usize, cy: usize) {
        let scanner_color = if self.auth_success { GREEN } else { CYAN };
        let pulse_val = math::sinish(self.pulse as i32).unsigned_abs() as usize;

        // Outer rotating ring dials
        let r_outer = 48 + (pulse_val * 4 / 1024);
        display.dotted_circle_outline(cx, cy, r_outer, 3, scanner_color.dim(180));
        display.draw_arc_ticks(cx, cy, r_outer, 18, 3, 6, self.rotation, scanner_color.dim(120));

        // Inner glowing hex/circle pad
        display.circle_alpha(cx, cy, 38, DARK_BG, 220);
        display.circle_alpha(cx, cy, 38, scanner_color, 40 + (pulse_val as u16 / 20));
        display.circle_outline(cx, cy, 38, scanner_color);

        // Fingerprint ridge lines
        for ridge in 1..=4 {
            let rr = ridge * 7;
            display.circle_outline(cx, cy + 2, rr, scanner_color.dim(160));
        }
        display.circle_filled(cx, cy + 2, 2, scanner_color);

        // Label below scanner
        let label = if self.auth_success { "BIOMETRIC VERIFIED" } else { "BIOMETRIC SENSOR READY" };
        font::centered_text(display, cx, cy + 46, label, 1, scanner_color);
    }

    fn draw_passcode_box(&self, display: &mut Display, cx: usize, cy: usize) {
        let box_w = 260;
        let box_h = 36;
        let box_x = cx.saturating_sub(box_w / 2);

        // Input container
        display.rect_rounded_alpha(box_x, cy, box_w, box_h, 6, DARK_BG, 220);
        display.rect_rounded_outline(box_x, cy, box_w, box_h, 6, if self.auth_error { RED } else { CYAN });

        // Passcode dots (or placeholder)
        if self.pass_len == 0 {
            font::draw_text_alpha(display, box_x + 16, cy + 12, "ENTER PASSCODE...", 1, WHITE_CYAN, 120);
        } else {
            for i in 0..self.pass_len {
                let dot_x = box_x + 20 + i * 14;
                display.circle_filled(dot_x, cy + 18, 3, CYAN);
            }
        }

        // Blinking caret cursor
        if self.pulse % 512 < 256 {
            let caret_x = box_x + 20 + self.pass_len * 14;
            display.rect(caret_x, cy + 10, 2, 16, CYAN);
        }

        // Submit Arrow Button `[ > ]`
        let btn_x = box_x + box_w - 32;
        display.rect_rounded_alpha(btn_x, cy + 6, 24, 24, 4, Rgb::new(0, 80, 120), 200);
        display.rect_rounded_outline(btn_x, cy + 6, 24, 24, 4, CYAN);
        font::draw_text(display, btn_x + 8, cy + 10, ">", 1, WHITE_CYAN);
    }

    fn draw_left_hud(&self, display: &mut Display, x: usize, y: usize) {
        let panel_w = 220;
        let panel_h = 170;

        display.rect_rounded_alpha(x, y, panel_w, panel_h, 6, DARK_BG, 180);
        display.rect_rounded_outline(x, y, panel_w, panel_h, 6, DEEP_CYAN.dim(160));

        font::draw_text(display, x + 14, y + 12, "SYSTEM METRICS", 1, CYAN);
        display.rect(x + 14, y + 26, panel_w - 28, 1, DEEP_CYAN.dim(100));

        let metrics = [
            ("CPU LOAD", "14% [OPTIMAL]"),
            ("MEMORY", "12.8 GB / 64 GB"),
            ("SYSTEM TEMP", "38 C"),
            ("BOOT STATUS", "100% ONLINE"),
            ("NETWORK", "SECURED (MESH)"),
        ];

        for (i, (label, val)) in metrics.iter().enumerate() {
            let row_y = y + 36 + i * 24;
            font::draw_text(display, x + 14, row_y, label, 1, WHITE_CYAN.dim(180));
            font::draw_text(display, x + 108, row_y, val, 1, GREEN);
        }
    }

    fn draw_right_hud(&self, display: &mut Display, x: usize, y: usize) {
        let panel_w = 220;
        let panel_h = 170;

        display.rect_rounded_alpha(x, y, panel_w, panel_h, 6, DARK_BG, 180);
        display.rect_rounded_outline(x, y, panel_w, panel_h, 6, MAGENTA.dim(160));

        font::draw_text(display, x + 14, y + 12, "SECURITY CLEARANCE", 1, MAGENTA);
        display.rect(x + 14, y + 26, panel_w - 28, 1, MAGENTA.dim(100));

        let status_items = [
            ("FIREWALL", "ACTIVE (ENCRYPTED)"),
            ("BIO-AUTH", "INITIALIZED"),
            ("USER CLEARANCE", "ADMIN (ATUL)"),
            ("SYSTEM ID", "AXON-7"),
            ("KERNEL VERSION", "ATULYA v0.1.0"),
        ];

        for (i, (label, val)) in status_items.iter().enumerate() {
            let row_y = y + 36 + i * 24;
            font::draw_text(display, x + 14, row_y, label, 1, WHITE_CYAN.dim(180));
            font::draw_text(display, x + 108, row_y, val, 1, CYAN);
        }
    }
}

fn translate_scancode(code: u8, shift: bool) -> char {
    if shift {
        match code {
            0x10..=0x19 => b"QWERTYUIOP"[(code - 0x10) as usize] as char,
            0x1E..=0x26 => b"ASDFGHJKL"[(code - 0x1E) as usize] as char,
            0x2C..=0x32 => b"ZXCVBNM"[(code - 0x2C) as usize] as char,
            0x39 => ' ',
            _ => '\0',
        }
    } else {
        match code {
            0x10..=0x19 => b"qwertyuiop"[(code - 0x10) as usize] as char,
            0x1E..=0x26 => b"asdfghjkl"[(code - 0x1E) as usize] as char,
            0x2C..=0x32 => b"zxcvbnm"[(code - 0x2C) as usize] as char,
            0x02..=0x0B => b"1234567890"[(code - 0x02) as usize] as char,
            0x39 => ' ',
            _ => '\0',
        }
    }
}
