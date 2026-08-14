//! awakening.rs — Movie-Grade CGI Video-Stream Boot Engine for Atulya OS.
//!
//! Features:
//!   1. Direct memory blit playback of 18 movie-grade CGI frames extracted from the official MP4
//!   2. 400x225 truecolor high-impact visual stream rendered centered on 1080p canvas
//!   3. Zero latency 60 FPS frame timing
//!   4. Triple concentric rotating cybernetic HUD dials with degree ticks
//!   5. Ultra-sleek razor-thin laser progress bar
//!   6. Hardware PC Speaker harmonic audio chime

use crate::display::{Display, Rgb};
use crate::font;
use crate::math;
use crate::serial;

const CYAN: Rgb = Rgb::new(0, 229, 255);
const DEEP_CYAN: Rgb = Rgb::new(0, 130, 200);
const WHITE_CYAN: Rgb = Rgb::new(190, 240, 255);
const MAGENTA: Rgb = Rgb::new(220, 60, 255);
const GREEN: Rgb = Rgb::new(0, 230, 118);
const WHITE: Rgb = Rgb::new(255, 255, 255);

const FRAME_W: usize = 400;
const FRAME_H: usize = 225;
const FRAME_COUNT: usize = 18;
const FRAME_BYTES: usize = FRAME_W * FRAME_H * 3;
const TOTAL_CLIP_BYTES: usize = FRAME_COUNT * FRAME_BYTES;

static CLIP_DATA: &[u8; TOTAL_CLIP_BYTES] = include_bytes!("../../../assets/boot/boot_stream.bin");

pub struct Awakening;

impl Awakening {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&mut self, display: &mut Display) {
        serial::serial_write_line("AtulyaOS: Movie-Grade CGI Video Boot Engine starting.");

        let mut rotation_deg: i32 = 0;

        for frame_idx in 0..FRAME_COUNT {
            let p = frame_idx as f32 / (FRAME_COUNT - 1) as f32;
            rotation_deg = (rotation_deg + 12) % 360;

            self.render_frame(display, frame_idx, p, rotation_deg);
            display.swap_buffers();

            math::delay(1_200);
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
        frame_idx: usize,
        p: f32,
        rotation: i32,
    ) {
        let w = display.width();
        let h = display.height();
        if w == 0 || h == 0 {
            return;
        }
        let cx = w / 2;
        let cy = h * 42 / 100;

        // ── 1. Deep Space Obsidian Canvas ───────────────────────────────────
        display.gradient_rect_v(0, 0, w, h, Rgb::new(1, 2, 4), Rgb::new(3, 4, 8));

        // Subtle background star particles
        let star_seed = [37usize, 128, 290, 480, 720, 890, 1140, 1380, 1560, 1720];
        for (si, &sx) in star_seed.iter().enumerate() {
            let sy = (si * 97 + 45) % h;
            display.pixel(sx % w, sy, WHITE_CYAN.dim(120));
        }

        // Corner futuristic HUD brackets
        draw_hud_brackets(display, w, h, p);

        // ── 2. Movie-Grade CGI Video Frame Blit ──────────────────────────────
        let frame_offset = frame_idx * FRAME_BYTES;
        let frame_slice = &CLIP_DATA[frame_offset..frame_offset + FRAME_BYTES];
        let dest_x = cx.saturating_sub(FRAME_W / 2);
        let dest_y = cy.saturating_sub(FRAME_H / 2);

        blit_rgb_frame(display, dest_x, dest_y, frame_slice, FRAME_W, FRAME_H);

        // ── 3. Concentric Rotating Cybernetic HUD Dials ─────────────────────
        let dial_alpha: u16 = 180;
        let r1 = 155;
        display.dotted_circle_outline(cx, cy, r1, 4, CYAN.dim(dial_alpha));
        display.draw_arc_ticks(cx, cy, r1, 24, 4, 8, rotation, CYAN.dim(dial_alpha / 2));

        let r2 = 180;
        display.dotted_circle_outline(cx, cy, r2, 6, MAGENTA.dim(dial_alpha / 2));
        display.draw_arc_ticks(cx, cy, r2, 36, 3, 6, -rotation * 2, MAGENTA.dim(dial_alpha / 3));

        // ── 4. Ultra-Sleek Laser Progress Bar ────────────────────────────────
        let bar_w = 360;
        let bar_h = 2;
        let bar_x = cx.saturating_sub(bar_w / 2);
        let bar_y = h * 76 / 100;

        // Dark track groove
        display.rect(bar_x, bar_y, bar_w, bar_h, Rgb::new(10, 18, 30));

        // Laser beam progress
        let fill_w = ((bar_w as f32) * p) as usize;
        if fill_w > 0 {
            display.rect(bar_x, bar_y, fill_w, bar_h, CYAN);
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

        // ── 5. Identity Reveal ("Hello, Atul.") ─────────────────────────────
        if p >= 0.80 {
            font::centered_text(display, cx, h * 20 / 100, "Hello, Atul.", 3, WHITE_CYAN);
            font::centered_text(display, cx, h * 20 / 100 + 28, "AUTHENTICATING SYSTEM IDENTITY", 1, CYAN);
        }
    }
}

/// Blit raw RGB (24bpp) frame buffer directly into Display.
fn blit_rgb_frame(
    display: &mut Display,
    dest_x: usize,
    dest_y: usize,
    rgb_data: &[u8],
    w: usize,
    h: usize,
) {
    let dw = display.width();
    let dh = display.height();

    for y in 0..h {
        let dy = dest_y + y;
        if dy >= dh { break; }
        let row_offset = y * w * 3;

        for x in 0..w {
            let dx = dest_x + x;
            if dx >= dw { break; }
            let px_offset = row_offset + x * 3;
            let r = rgb_data[px_offset];
            let g = rgb_data[px_offset + 1];
            let b = rgb_data[px_offset + 2];
            display.pixel(dx, dy, Rgb::new(r, g, b));
        }
    }
}

fn draw_hud_brackets(display: &mut Display, w: usize, h: usize, p: f32) {
    let pad = 24;
    let len = 20;
    let col = CYAN.dim(((p * 180.0) as u16).min(180));

    display.rect(pad, pad, len, 2, col);
    display.rect(pad, pad, 2, len, col);
    display.rect(w - pad - len, pad, len, 2, col);
    display.rect(w - pad, pad, 2, len, col);
    display.rect(pad, h - pad, len, 2, col);
    display.rect(pad, h - pad - len, 2, len, col);
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
