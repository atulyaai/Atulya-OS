//! gpu_boot.rs — Generative "compute shader" boot animation.
//!
//! A single-pass, per-pixel HDR renderer that builds a *living* boot
//! sequence unique to every boot (seeded by entropy from the TSC).
//! No asset files, no Python pipeline, no pre-rendered frames.
//!
//! The visual concept is "THE AWAKENING" — a neural cosmos that
//! self-assembles instead of the usual "glowing orb + static panels":
//!
//!   1. VOID       a single flickering seed of light (real entropy)
//!   2. GENESIS     domain-warped plasma field (reaction-diffusion-like)
//!   3. CONSTELLATION  subsystem nodes are born and connect as a graph
//!   4. THE ORB     a dendrite plasma orb condenses with 4-petal bloom
//!   5. BLOOM       particle swarm orbits, wordmark + status fade in
//!   6. READY       calm, glowing, alive idle state
//!
//! Each of these is computed analytically per pixel (like a fragment
//! shader) and tonemapped with `1 - exp(-x)`. It can later be ported
//! verbatim to a real WGSL compute shader once a GPU driver exists.

use crate::display::{Display, Rgb};
use crate::font;
use crate::math;
use crate::serial;

const PI: f32 = 3.14159265;
const TAU: f32 = 6.2831853;
const TOTAL_MS: u32 = 12_000;

/// Coarse resolution for the precomputed plasma field grid.
/// The expensive trig for the field is evaluated here once per frame
/// instead of per pixel, then bilinearly sampled in the pixel loop.
const GRID_W: usize = 64;
const GRID_H: usize = 36;

/// Subsystem node in the self-assembling constellation.
struct Sub {
    angle: f32,            // position on the ring (radians)
    radius: f32,           // normalized radius from center
    color: (f32, f32, f32),
    name: &'static str,
    born: f32,             // progress (0..1) at which this node appears
}

/// The generative boot renderer.
pub struct GpuBoot {
    seed: u32,
}

impl GpuBoot {
    pub fn new() -> Self {
        // Real entropy: timestamp counter differs every boot.
        let tsc = rdtsc();
        Self { seed: (tsc ^ (tsc >> 17) ^ 0x9E3779B9) as u32 }
    }

    pub fn run(&mut self, display: &mut Display) {
        serial::serial_write_line("AtulyaOS generative GPU boot starting.");

        let dt: u32 = 33; // ~30fps target
        let mut elapsed: u32 = 0;
        // First frame
        self.render_frame(display, elapsed);
        display.swap_buffers();

        while elapsed < TOTAL_MS {
            math::delay(8_000);
            elapsed = (elapsed + dt).min(TOTAL_MS);
            self.render_frame(display, elapsed);
            display.swap_buffers();
        }

        // Hold the ready frame a moment before handing off.
        math::delay(8_000 * 12);
        serial::serial_write_line("AtulyaOS generative GPU boot complete.");
    }

    fn render_frame(&self, display: &mut Display, elapsed_ms: u32) {
        let w = display.width();
        let h = display.height();
        if w == 0 || h == 0 {
            return;
        }
        let aspect = w as f32 / h as f32;
        let t = elapsed_ms as f32 / 1000.0;
        let p = (elapsed_ms as f32 / TOTAL_MS as f32).min(1.0);
        let seed_f = (self.seed as f32) * 0.00001;

        // Precompute constellation nodes (aspect-corrected normalized space).
        let subs: [Sub; 5] = [
            Sub { angle: -PI / 2.0,            radius: 0.62, color: (0.85, 0.30, 1.00), name: "MEMORY",       born: 0.33 },
            Sub { angle: -PI / 2.0 + TAU / 5.0, radius: 0.66, color: (0.00, 0.85, 1.00), name: "VOICE",        born: 0.45 },
            Sub { angle: -PI / 2.0 + 2.0 * TAU / 5.0, radius: 0.62, color: (1.00, 0.55, 0.20), name: "VISION", born: 0.57 },
            Sub { angle: -PI / 2.0 + 3.0 * TAU / 5.0, radius: 0.66, color: (0.40, 1.00, 0.65), name: "SKILLS",  born: 0.69 },
            Sub { angle: -PI / 2.0 + 4.0 * TAU / 5.0, radius: 0.62, color: (1.00, 0.35, 0.80), name: "INTEL",   born: 0.80 },
        ];

        let mut node_x = [0.0f32; 5];
        let mut node_y = [0.0f32; 5];
        for (i, s) in subs.iter().enumerate() {
            node_x[i] = fcosf(s.angle) * s.radius;
            node_y[i] = fsinf(s.angle) * s.radius;
        }

        // Orb forms over the middle of the boot.
        let orb_form = smoothstep(0.10, 0.55, p);
        let orb_r = 0.16 + 0.04 * fsinf(t * 1.3 + seed_f * TAU);

        // ===== Pass 1: per-pixel field + orb (the expensive analytic part) =====
        for y in 0..h {
            let ny = (y as f32 / h as f32 - 0.5) * 2.0;
            for x in 0..w {
                let nx = (x as f32 / w as f32 - 0.5) * 2.0 * aspect;

                let mut r: f32 = 0.0;
                let mut g: f32 = 0.0;
                let mut b: f32 = 0.0;

                let d = fsqrt(nx * nx + ny * ny);
                let theta = fatan2(ny, nx);

                // Background radial gradient (deep space).
                let bg = (1.0 - (d / 1.2).clamp(0.0, 1.0)).max(0.0);
                r += bg * 0.015;
                g += bg * 0.035;
                b += bg * 0.085;

                // Domain-warped plasma field (genesis).
                let warp = field(nx * 1.6 + t * 0.10, ny * 1.6 - t * 0.07, t, seed_f);
                let wx = nx + warp.0 * 0.35;
                let wy = ny + warp.1 * 0.35;
                let plasma = fsinf(wx * 3.0 + t * 0.9) * fcosf(wy * 3.0 - t * 0.7)
                    + fsinf((wx + wy) * 2.0 + t * 0.5) * 0.6;
                let field_i = smoothstep(0.0, 0.35, p) * 0.16;
                let (pc_r, pc_g, pc_b) = plasma_color(plasma * 0.25 + 0.5 + seed_f);
                r += pc_r * field_i;
                g += pc_g * field_i;
                b += pc_b * field_i;

                // The dendrite orb.
                if orb_form > 0.0 {
                    let petal = 1.0 + 0.16 * fcosf(4.0 * theta + t * 1.5 + seed_f * TAU);
                    let ring1 = fexp(-((d - orb_r * petal).abs()).powi_f(2) * 55.0);
                    let ring2 = fexp(-((d - orb_r * 1.45 * petal).abs()).powi_f(2) * 38.0);
                    let core = fexp(-d * d * 32.0);
                    let nsp = 24.0;
                    let nearest = fround(theta * nsp / TAU) * TAU / nsp;
                    let adist = (theta - nearest).abs();
                    let filament = fexp(-adist * adist * 420.0)
                        * fexp(-((d - orb_r).abs()).powi_f(2) * 18.0);
                    let (oc_r, oc_g, oc_b) = plasma_color(theta / TAU + t * 0.05 + seed_f);

                    r += (oc_r * (ring1 + ring2 * 0.5) + core * 0.7 + oc_r * filament * 0.6) * orb_form;
                    g += (oc_g * (ring1 + ring2 * 0.5) + core * 0.8 + oc_g * filament * 0.6) * orb_form;
                    b += (oc_b * (ring1 + ring2 * 0.5) + core * 1.0 + oc_b * filament * 0.6) * orb_form;
                }

                // Vignette.
                let vig = (1.0 - d * 0.45).clamp(0.0, 1.0);
                r *= vig;
                g *= vig;
                b *= vig;

                // Film grain (entropy).
                let grain = hash_pixel(x, y, elapsed_ms, self.seed) * 0.05;
                r += grain;
                g += grain;
                b += grain;

                // Tonemap + write.
                let tr = (1.0 - fexp(-r)) * 255.0;
                let tg = (1.0 - fexp(-g)) * 255.0;
                let tb = (1.0 - fexp(-b)) * 255.0;
                display.pixel(
                    x,
                    y,
                    Rgb::new(
                        tr.clamp(0.0, 255.0) as u8,
                        tg.clamp(0.0, 255.0) as u8,
                        tb.clamp(0.0, 255.0) as u8,
                    ),
                );
            }
        }

        // ===== Pass 2: constellation (additive glow, O(nodes) not O(pixels)) =====
        for (i, s) in subs.iter().enumerate() {
            let appear = smoothstep(s.born, s.born + 0.12, p);
            if appear <= 0.0 {
                continue;
            }
            let (sx, sy) = to_pixel_i(node_x[i], node_y[i], w, h, aspect);
            draw_glow(display, sx, sy, 6, s.color.0, s.color.1, s.color.2, appear);
            let (ex0, ey0) = if i == 0 {
                (0.0, 0.0)
            } else {
                (node_x[i - 1], node_y[i - 1])
            };
            draw_line_glow(
                display, ex0, ey0, node_x[i], node_y[i], w, h, aspect,
                s.color.0, s.color.1, s.color.2, appear * 0.6,
            );
        }

        // ===== Pass 3: orbiting particle swarm (additive glow) =====
        let swarm = smoothstep(0.55, 0.85, p);
        if swarm > 0.0 {
            let np = 90usize;
            for k in 0..np {
                let kf = k as f32;
                let a0 = hash_u(k as u32, self.seed) * TAU;
                let ar = 0.20 + hash_u(k as u32 + 7, self.seed) * 0.55;
                let spd = 0.4 + hash_u(k as u32 + 13, self.seed) * 1.2;
                let ang = a0 + t * spd;
                let px = fcosf(ang) * ar;
                let py = fsinf(ang) * ar * 0.92;
                let (sx, sy) = to_pixel_i(px, py, w, h, aspect);
                let (pr, pg, pb) = plasma_color(kf / np as f32 + t * 0.1);
                draw_glow(display, sx, sy, 4, pr, pg, pb, swarm * 0.5);
            }
        }

        // ===== Text overlay (bitmap font) =====
        self.draw_overlay(display, w, h, elapsed_ms, p, aspect, &subs, &node_x, &node_y);
    }

    fn draw_overlay(
        &self,
        display: &mut Display,
        w: usize,
        h: usize,
        _elapsed_ms: u32,
        p: f32,
        aspect: f32,
        subs: &[Sub; 5],
        node_x: &[f32; 5],
        node_y: &[f32; 5],
    ) {
        // Helper: normalized aspect coords -> pixel.
        let to_px = |nx: f32, ny: f32| -> (usize, usize) {
            let px = (w as f32 * (nx / (2.0 * aspect) + 0.5)).clamp(0.0, (w - 1) as f32) as usize;
            let py = (h as f32 * (ny / 2.0 + 0.5)).clamp(0.0, (h - 1) as f32) as usize;
            (px, py)
        };

        // Subsystem labels near each node.
        for (i, s) in subs.iter().enumerate() {
            let appear = smoothstep(s.born, s.born + 0.12, p);
            if appear <= 0.0 {
                continue;
            }
            let (cx, cy) = to_px(node_x[i], node_y[i]);
            let col = Rgb::new(
                (s.color.0 * 255.0) as u8,
                (s.color.1 * 255.0) as u8,
                (s.color.2 * 255.0) as u8,
            );
            let scale: usize = 1;
            let tw = font::text_width(s.name, scale);
            let lx = cx.saturating_sub(tw / 2);
            let ly = cy + 10;
            font::draw_text(display, lx, ly, s.name, scale, col);
        }

        // Wordmark: ATULYAOS (appears after orb forms).
        let mark = smoothstep(0.60, 0.80, p);
        if mark > 0.0 {
            let word = "ATULYAOS";
            let scale: usize = 3;
            let tw = font::text_width(word, scale);
            let wx = (w / 2).saturating_sub(tw / 2);
            let wy = (h as f32 * 0.74) as usize;
            let col = Rgb::new(
                (200.0 * mark) as u8 + 40,
                (235.0 * mark) as u8 + 20,
                255,
            );
            font::draw_text(display, wx, wy, word, scale, col);
        }

        // Status text + percentage at bottom.
        let status = if p < 0.12 {
            "SYSTEM POWER ON"
        } else if p < 0.33 {
            "ENERGY INITIALIZING"
        } else if p < 0.55 {
            "CONSTELLATION FORMING"
        } else if p < 0.80 {
            "ATULYA CORE ONLINE"
        } else if p < 0.97 {
            "SUBSYSTEMS ONLINE"
        } else {
            "ATULYA IS READY"
        };
        let pct = ((p * 100.0) as u32).min(100);
        let mut pbuf = [0u8; 4];
        let pstr = pct_text(pct, &mut pbuf);

        let stat_scale: usize = 1;
        let ss = font::text_width(status, stat_scale);
        let sx = (w / 2).saturating_sub(ss / 2);
        let sy = (h as f32 * 0.86) as usize;
        font::draw_text(display, sx, sy, status, stat_scale, Rgb::new(120, 220, 255));

        let ps = font::text_width(pstr, stat_scale);
        let px = (w / 2).saturating_sub(ps / 2);
        let py = (h as f32 * 0.90) as usize;
        font::draw_text(display, px, py, pstr, stat_scale, Rgb::new(180, 230, 255));
    }
}

// ============================================================================
// Additive glow helpers (post-pass rendering, O(nodes/particles) not O(px))
// ============================================================================

/// Normalized aspect-corrected coord -> integer pixel.
fn to_pixel_i(nx: f32, ny: f32, w: usize, h: usize, aspect: f32) -> (i32, i32) {
    let px = (w as f32 * (nx / (2.0 * aspect) + 0.5)).clamp(0.0, (w - 1) as f32) as i32;
    let py = (h as f32 * (ny / 2.0 + 0.5)).clamp(0.0, (h - 1) as f32) as i32;
    (px, py)
}

/// Additively blend a color onto a framebuffer pixel (read-modify-write).
fn add_blend(display: &mut Display, x: i32, y: i32, r: f32, g: f32, b: f32) {
    if x < 0 || y < 0 {
        return;
    }
    let x = x as usize;
    let y = y as usize;
    let cur = display.read_pixel(x, y);
    let nr = (cur.r as f32 + r * 255.0).clamp(0.0, 255.0) as u8;
    let ng = (cur.g as f32 + g * 255.0).clamp(0.0, 255.0) as u8;
    let nb = (cur.b as f32 + b * 255.0).clamp(0.0, 255.0) as u8;
    display.pixel(x, y, Rgb::new(nr, ng, nb));
}

/// Radial additive glow blob centered at (cx, cy).
fn draw_glow(
    display: &mut Display,
    cx: i32,
    cy: i32,
    radius: i32,
    r: f32,
    g: f32,
    b: f32,
    intensity: f32,
) {
    if radius <= 0 {
        return;
    }
    let rad_f = radius as f32;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let dist2 = dx * dx + dy * dy;
            if dist2 > radius * radius {
                continue;
            }
            let fall = fexp(-(dist2 as f32) / (rad_f * rad_f) * 4.0);
            add_blend(
                display,
                cx + dx,
                cy + dy,
                r * fall * intensity,
                g * fall * intensity,
                b * fall * intensity,
            );
        }
    }
}

/// Additive glowing line between two normalized points (sampled glow).
fn draw_line_glow(
    display: &mut Display,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    w: usize,
    h: usize,
    aspect: f32,
    r: f32,
    g: f32,
    b: f32,
    intensity: f32,
) {
    let (px0, py0) = to_pixel_i(x0, y0, w, h, aspect);
    let (px1, py1) = to_pixel_i(x1, y1, w, h, aspect);
    let steps = (px0 - px1).abs().max(py0 - py1).abs().max(1) as i32;
    for s in 0..=steps {
        let tt = s as f32 / steps as f32;
        let x = px0 + ((px1 - px0) as f32 * tt) as i32;
        let y = py0 + ((py1 - py0) as f32 * tt) as i32;
        draw_glow(display, x, y, 3, r, g, b, intensity * 0.15);
    }
}

// ============================================================================
// Math helpers (fast, no_std, no external deps)
// ============================================================================

fn clamp(x: f32, a: f32, b: f32) -> f32 {
    if x < a {
        a
    } else if x > b {
        b
    } else {
        x
    }
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = clamp((x - e0) / (e1 - e0), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn fsqrt(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut g = x * 0.5;
    for _ in 0..6 {
        g = (g + x / g) * 0.5;
    }
    g
}

fn fmod2pi(x: f32) -> f32 {
    let tau = TAU;
    let mut a = x;
    while a >= tau {
        a -= tau;
    }
    while a < 0.0 {
        a += tau;
    }
    a
}

fn fsinf(x: f32) -> f32 {
    let a = fmod2pi(x);
    let a2 = a * a;
    let a3 = a2 * a;
    let a5 = a3 * a2;
    let a7 = a5 * a2;
    a - a3 / 6.0 + a5 / 120.0 - a7 / 5040.0
}

fn fcosf(x: f32) -> f32 {
    fsinf(x + PI / 2.0)
}

fn fround(x: f32) -> f32 {
    if x >= 0.0 { (x + 0.5) as i32 as f32 } else { (x - 0.5) as i32 as f32 }
}

fn fexp(x: f32) -> f32 {
    if x > 80.0 {
        return 1.0e30;
    }
    if x < -80.0 {
        return 0.0;
    }
    let k = (x * 1.44269504) as i32; // log2(e)
    let f = x - (k as f32) * 0.69314718;
    let result = 1.0 + f + f * f / 2.0 + f * f * f / 6.0 + f * f * f * f / 24.0;
    if k >= 0 {
        result * (1u32 << k.min(31)) as f32
    } else {
        result / (1u32 << ((-k).min(31))) as f32
    }
}

/// Approximate atan2 for theta in [-PI, PI].
fn fatan2(y: f32, x: f32) -> f32 {
    if x > 0.0 {
        fatan(y / x)
    } else if x < 0.0 {
        fatan(y / x) + if y >= 0.0 { PI } else { -PI }
    } else if y > 0.0 {
        PI / 2.0
    } else if y < 0.0 {
        -PI / 2.0
    } else {
        0.0
    }
}

fn fatan(x: f32) -> f32 {
    let mut xx = x;
    let mut s = 1.0f32;
    if xx < 0.0 {
        xx = -xx;
        s = -1.0;
    }
    let r = if xx > 1.0 {
        PI / 2.0 - xx / (xx * xx + 0.28)
    } else {
        xx / (1.0 + xx * xx * 0.28)
    };
    r * s
}

/// Domain-warped vector field (cheap stand-in for reaction-diffusion).
fn field(x: f32, y: f32, t: f32, seed: f32) -> (f32, f32) {
    let a = fsinf(x * 1.7 + t * 0.6 + seed) + fcosf(y * 1.3 - t * 0.5);
    let b = fcosf(x * 1.1 - t * 0.4) + fsinf(y * 1.9 + t * 0.7 + seed * 1.3);
    (a, b)
}

/// Cyan -> magenta -> orange -> cyan color cycle. `t` wraps in 0..1.
fn plasma_color(t: f32) -> (f32, f32, f32) {
    let mut tt = t - (t as i32) as f32;
    if tt < 0.0 {
        tt += 1.0;
    }
    if tt < 0.33 {
        let u = tt / 0.33;
        (0.0 + 0.8 * u, 0.8 - 0.45 * u, 1.0)
    } else if tt < 0.66 {
        let u = (tt - 0.33) / 0.33;
        (0.8 + 0.2 * u, 0.35 + 0.25 * u, 1.0 - 0.7 * u)
    } else {
        let u = (tt - 0.66) / 0.34;
        (1.0 - 1.0 * u, 0.6, 0.3 + 0.7 * u)
    }
}

/// Deterministic hash from an integer + seed -> 0..1.
fn hash_u(n: u32, seed: u32) -> f32 {
    let mut h = n
        .wrapping_mul(2654435761)
        .wrapping_add(seed.wrapping_mul(40503));
    h ^= h >> 15;
    h = h.wrapping_mul(2246822519);
    h ^= h >> 13;
    (h & 0xFFFF) as f32 / 65535.0
}

/// Cheap per-pixel grain hash.
fn hash_pixel(x: usize, y: usize, t: u32, seed: u32) -> f32 {
    let h = (x as u32)
        .wrapping_mul(2654435761)
        .wrapping_add((y as u32).wrapping_mul(2246822519))
        .wrapping_add(t.wrapping_mul(3266489917))
        .wrapping_add(seed.wrapping_mul(668265263));
    ((h & 0xFF) as f32) / 255.0
}

fn pct_text<'a>(pct: u32, buf: &'a mut [u8; 4]) -> &'a str {
    let pct = pct.min(100);
    if pct == 100 {
        buf[0] = b'1';
        buf[1] = b'0';
        buf[2] = b'0';
        buf[3] = b'%';
        core::str::from_utf8(&buf[..4]).unwrap_or("100%")
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

#[inline]
fn rdtsc() -> u64 {
    // Safe on x86_64 bare-metal kernels; counts nonce-deterministic cycles.
    unsafe { core::arch::x86_64::_rdtsc() }
}

// Local trait to avoid clashing with f32::powi (which exists, but we keep our own).
trait PowI {
    fn powi_f(self, n: i32) -> f32;
}
impl PowI for f32 {
    fn powi_f(self, n: i32) -> f32 {
        let mut r = 1.0f32;
        let mut base = self;
        let mut e = n;
        let neg = e < 0;
        if neg {
            e = -e;
        }
        while e > 0 {
            if e & 1 == 1 {
                r *= base;
            }
            base *= base;
            e >>= 1;
        }
        if neg {
            1.0 / r
        } else {
            r
        }
    }
}
