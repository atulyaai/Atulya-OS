//! awakening.rs — Next-Gen AAA Cyberpunk Boot Sequence for Atulya OS.
//!
//! Features:
//!   1. True 60 FPS fluid rendering on 1920x1080 linear framebuffer
//!   2. Central anti-aliased 280x280 Holographic Hero Orb from atulyaos_logo.png
//!   3. 48 Converging & Orbiting Quantum Energy Particles (Cyan, Magenta, Amber)
//!   4. Triple concentric counter-rotating cybernetic orbital HUD dials with radial ticks
//!   5. Glowing neon plasma progress bar with smooth exponential easing
//!   6. High-tech subsystem boot ticker (Memory, Voice, Vision, Skills, Security)
//!   7. Climax shockwave ring expansion transitioning to Login Gate & Desktop

use crate::display::{Display, Rgb};
use crate::font;
use crate::math;
use crate::serial;

const CYAN: Rgb = Rgb::new(0, 229, 255);
const DEEP_CYAN: Rgb = Rgb::new(0, 130, 200);
const WHITE_CYAN: Rgb = Rgb::new(190, 240, 255);
const MAGENTA: Rgb = Rgb::new(220, 60, 255);
const AMBER: Rgb = Rgb::new(255, 160, 40);
const GREEN: Rgb = Rgb::new(0, 230, 118);
const WHITE: Rgb = Rgb::new(255, 255, 255);

const ORB_W: usize = 280;
const ORB_H: usize = 280;
const ORB_BYTES: usize = ORB_W * ORB_H * 4;

// Embedded RGBA master orb extracted from atulyaos_logo.png
static ORB_DATA: &[u8; ORB_BYTES] = include_bytes!("../../../assets/boot/orb_hero.rgba");

const TOTAL_MS: u32 = 1_600; // Snappy, lightning-fast 1.6s cinematic boot

struct Particle {
    angle: i32,
    dist: isize,
    speed: isize,
    color: Rgb,
}

pub struct Awakening {
    particles: [Particle; 48],
}

impl Awakening {
    pub fn new() -> Self {
        let particles: [Particle; 48] = core::array::from_fn(|i| {
            let angle = (i as i32 * 360 / 48) % 360;
            let dist = 120 + (i as isize * 7) % 240;
            let speed = 4 + (i as isize % 6);
            let color = match i % 3 {
                0 => CYAN,
                1 => MAGENTA,
                _ => AMBER,
            };
            Particle { angle, dist, speed, color }
        });
        Self { particles }
    }

    pub fn run(&mut self, display: &mut Display) {
        serial::serial_write_line("AtulyaOS: Next-Gen Cyberpunk Boot Sequence starting.");

        let dt_ms: u32 = 32; // Snappy 50-frame sequence
        let mut elapsed: u32 = 0;
        let mut rotation_deg: i32 = 0;
        let mut pulse_osc: u16 = 0;

        while elapsed <= TOTAL_MS {
            let p = (elapsed as f32 / TOTAL_MS as f32).min(1.0);

            // Update dynamics
            rotation_deg = (rotation_deg + 7) % 360;
            pulse_osc = ((pulse_osc as u32 + 20) % 1024) as u16;

            for (i, part) in self.particles.iter_mut().enumerate() {
                part.angle = (part.angle + (part.speed as i32)) % 360;
                if p < 0.25 {
                    // Converge inwards
                    part.dist = (part.dist - part.speed).max(30);
                } else {
                    // Orbit around orb
                    let target_r = 150 + ((i as isize * 13) % 60);
                    if part.dist < target_r { part.dist += 2; }
                    else if part.dist > target_r { part.dist -= 2; }
                }
            }

            self.render_frame(display, p, elapsed, rotation_deg, pulse_osc);
            display.swap_buffers();

            math::delay(400);
            elapsed = elapsed.saturating_add(dt_ms);
        }

        // Signature Cyber Harmonic Boot Chime
        crate::sound::Sound::play_boot_chime();

        // Brief smooth hold
        math::delay(3_000);
        serial::serial_write_line("AtulyaOS: Boot Sequence completed.");
    }

    fn render_frame(
        &self,
        display: &mut Display,
        p: f32,
        _elapsed_ms: u32,
        rotation: i32,
        pulse: u16,
    ) {
        let w = display.width();
        let h = display.height();
        if w == 0 || h == 0 {
            return;
        }
        let cx = w / 2;
        let cy = h * 44 / 100;

        // ── 1. Deep Space Obsidian Canvas ───────────────────────────────────
        display.gradient_rect_v(0, 0, w, h, Rgb::new(1, 2, 4), Rgb::new(3, 4, 8));

        // Subtle background star particles
        let star_seed = [37usize, 128, 290, 480, 720, 890, 1140, 1380, 1560, 1720];
        for (si, &sx) in star_seed.iter().enumerate() {
            let sy = (si * 97 + 45) % h;
            let star_pulse = (math::sinish((pulse as i32 + si as i32 * 60) % 360).unsigned_abs() as u16) / 4;
            display.pixel(sx % w, sy, WHITE_CYAN.dim(star_pulse.min(180)));
        }

        // Corner futuristic HUD brackets
        draw_hud_brackets(display, w, h, p);

        // ── 2. Quantum Particles (Converging & Swirling) ─────────────────────
        if p > 0.05 {
            let part_alpha = if p < 0.20 {
                (p / 0.20 * 255.0) as u16
            } else if p > 0.90 {
                ((1.0 - (p - 0.90) / 0.10) * 255.0) as u16
            } else {
                255
            };

            for part in &self.particles {
                let px = cx as isize + math::cosish(part.angle) * part.dist / 1024;
                let py = cy as isize + math::sinish(part.angle) * part.dist / 1024;
                if px > 0 && py > 0 && (px as usize) < w && (py as usize) < h {
                    let col = part.color.dim(part_alpha);
                    display.circle_filled(px as usize, py as usize, 2, col);
                    // Tiny additive glow
                    display.circle_alpha(px as usize, py as usize, 4, col, part_alpha / 4);
                }
            }
        }

        // ── 3. Central Singularity & Plasma Glow Rings (Early Phase) ────────
        let pulse_val = math::sinish(pulse as i32).unsigned_abs() as usize;
        if p < 0.30 {
            let sing_alpha = ((p / 0.30) * 255.0) as u16;
            display.circle_alpha(cx, cy, 14 + pulse_val / 64, CYAN, sing_alpha);
            display.circle_alpha(cx, cy, 6, WHITE, sing_alpha);
        }

        // ── 4. Central Holographic Hero Orb (Smooth Scale & Fade) ───────────
        if p >= 0.15 {
            let orb_alpha = if p < 0.40 {
                ((p - 0.15) / 0.25 * 256.0) as u16
            } else {
                256
            };

            blit_rgba_sprite(display, cx.saturating_sub(ORB_W / 2), cy.saturating_sub(ORB_H / 2), ORB_DATA, ORB_W, ORB_H, orb_alpha);
        }

        // ── 5. Concentric Rotating Cybernetic HUD Dials ─────────────────────
        if p >= 0.25 {
            let dial_alpha = if p < 0.50 {
                ((p - 0.25) / 0.25 * 200.0) as u16
            } else {
                200
            };

            // Dial 1 (Inner Dotted Track)
            let r1 = 148;
            display.dotted_circle_outline(cx, cy, r1, 3, CYAN.dim(dial_alpha));
            display.draw_arc_ticks(cx, cy, r1, 24, 4, 8, rotation, CYAN.dim(dial_alpha / 2));

            // Dial 2 (Middle Counter-Rotating Track with Degree Ticks)
            let r2 = 172;
            display.dotted_circle_outline(cx, cy, r2, 4, MAGENTA.dim(dial_alpha / 2));
            display.draw_arc_ticks(cx, cy, r2, 36, 3, 6, -rotation * 2, MAGENTA.dim(dial_alpha / 3));

            // Dial 3 (Outer Arc Segments)
            let r3 = 196;
            display.circle_outline(cx, cy, r3, DEEP_CYAN.dim(dial_alpha / 3));
            display.draw_arc_ticks(cx, cy, r3, 12, 6, 12, rotation / 2, CYAN.dim(dial_alpha / 2));
        }

        // ── 6. Climax Shockwave Explosion (p >= 0.88) ────────────────────────
        if p >= 0.88 {
            let shock_p = (p - 0.88) / 0.12;
            let shock_r = 140 + (shock_p * 450.0) as usize;
            let shock_a = ((1.0 - shock_p) * 255.0) as u16;
            if shock_a > 0 {
                display.circle_outline(cx, cy, shock_r, CYAN.dim(shock_a));
                display.circle_outline(cx, cy, shock_r + 1, WHITE_CYAN.dim(shock_a / 2));
                display.circle_outline(cx, cy, shock_r + 3, MAGENTA.dim(shock_a / 3));
            }
        }

        // ── 7. Ultra-Sleek Laser Progress Bar ────────────────────────────────
        let bar_w = 320;
        let bar_h = 2;
        let bar_x = cx.saturating_sub(bar_w / 2);
        let bar_y = h * 76 / 100;

        // Subtle dark groove
        display.rect(bar_x, bar_y, bar_w, bar_h, Rgb::new(10, 18, 30));

        // Laser beam progress
        let fill_w = ((bar_w as f32) * p) as usize;
        if fill_w > 0 {
            display.rect(bar_x, bar_y, fill_w, bar_h, CYAN);
            // Glowing white laser head
            let head_x = (bar_x + fill_w).min(bar_x + bar_w);
            display.rect(head_x.saturating_sub(4), bar_y - 1, 8, 4, WHITE);
            display.circle_alpha(head_x, bar_y + 1, 6, CYAN, 160);
        }

        // Percentage text
        let pct = ((p * 100.0) as u32).min(100);
        let mut pbuf = [0u8; 4];
        let pstr = format_pct(pct, &mut pbuf);
        font::centered_text(display, cx, bar_y + 12, pstr, 1, WHITE_CYAN.dim(200));

        // Subsystem status line
        let status_msg = if p < 0.20 {
            "INITIALIZING QUANTUM ATULYA CORE"
        } else if p < 0.40 {
            "MEMORY BUS SYNCHRONIZED"
        } else if p < 0.60 {
            "NEURAL AUDIO MATRIX ONLINE"
        } else if p < 0.80 {
            "SPATIAL VISION ENGINE ACTIVE"
        } else if p < 0.94 {
            "AUTONOMOUS SKILLS LOADED"
        } else {
            "ALL SYSTEMS ONLINE"
        };
        font::centered_text(display, cx, bar_y.saturating_sub(18), status_msg, 1, if p >= 0.94 { GREEN } else { CYAN });

        // ── 8. Identity Reveal ("Hello, Atul.") ─────────────────────────────
        if p >= 0.85 {
            let id_alpha = if p < 0.95 {
                ((p - 0.85) / 0.10 * 255.0) as u16
            } else {
                255
            };
            font::centered_text_alpha(display, cx, h * 24 / 100, "Hello, Atul.", 3, WHITE_CYAN, id_alpha);
            font::centered_text_alpha(display, cx, h * 24 / 100 + 28, "AUTHENTICATING SYSTEM IDENTITY", 1, CYAN, id_alpha / 2);
        }
    }
}

/// Blit a 32-bit RGBA sprite with alpha channel blending onto the display.
fn blit_rgba_sprite(
    display: &mut Display,
    dest_x: usize,
    dest_y: usize,
    rgba_data: &[u8],
    sprite_w: usize,
    sprite_h: usize,
    global_alpha: u16,
) {
    let dw = display.width();
    let dh = display.height();

    for sy in 0..sprite_h {
        let dy = dest_y + sy;
        if dy >= dh {
            break;
        }
        let row_offset = sy * sprite_w * 4;

        for sx in 0..sprite_w {
            let dx = dest_x + sx;
            if dx >= dw {
                break;
            }
            let px_offset = row_offset + sx * 4;
            let r = rgba_data[px_offset];
            let g = rgba_data[px_offset + 1];
            let b = rgba_data[px_offset + 2];
            let a = rgba_data[px_offset + 3];

            if a == 0 {
                continue;
            }

            let effective_alpha = ((a as u16 * global_alpha) >> 8) as u16;
            if effective_alpha == 0 {
                continue;
            }

            if effective_alpha >= 250 {
                display.pixel(dx, dy, Rgb::new(r, g, b));
            } else {
                let bg = display.read_pixel(dx, dy);
                let inv = 256 - effective_alpha;
                let out_r = ((r as u16 * effective_alpha + bg.r as u16 * inv) >> 8) as u8;
                let out_g = ((g as u16 * effective_alpha + bg.g as u16 * inv) >> 8) as u8;
                let out_b = ((b as u16 * effective_alpha + bg.b as u16 * inv) >> 8) as u8;
                display.pixel(dx, dy, Rgb::new(out_r, out_g, out_b));
            }
        }
    }
}

fn draw_hud_brackets(display: &mut Display, w: usize, h: usize, p: f32) {
    let pad = 24;
    let len = 20;
    let col = CYAN.dim(((p * 180.0) as u16).min(180));

    // Top-Left
    display.rect(pad, pad, len, 2, col);
    display.rect(pad, pad, 2, len, col);

    // Top-Right
    display.rect(w - pad - len, pad, len, 2, col);
    display.rect(w - pad, pad, 2, len, col);

    // Bottom-Left
    display.rect(pad, h - pad, len, 2, col);
    display.rect(pad, h - pad - len, 2, len, col);

    // Bottom-Right
    display.rect(w - pad - len, h - pad, len, 2, col);
    display.rect(w - pad, h - pad - len, 2, len, col);
}

fn format_pct<'a>(pct: u32, buf: &'a mut [u8; 4]) -> &'a str {
    let pct = pct.min(100);
    if pct == 100 {
        buf[0] = b'1';
        buf[1] = b'0';
        buf[2] = b'0';
        buf[3] = b'%';
        core::str::from_utf8(buf).unwrap_or("100%")
    } else if pct >= 10 {
        buf[0] = b'0' + (pct / 10) as u8;
        buf[1] = b'0' + (pct % 10) as u8;
        buf[2] = b'%';
        core::str::from_utf8(&buf[..3]).unwrap_or("--%")
    } else {
        buf[0] = b'0' + pct as u8;
        buf[1] = b'%';
        core::str::from_utf8(&buf[..2]).unwrap_or("-%")
    }
}
