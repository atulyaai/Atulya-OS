use alloc::boxed::Box;
use crate::display::Rgb;

#[derive(Clone, Copy)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub fn to_rgb(self) -> Rgb {
        let r = (self.x * 255.0) as u32;
        let g = (self.y * 255.0) as u32;
        let b = (self.z * 255.0) as u32;
        Rgb::new(r.min(255) as u8, g.min(255) as u8, b.min(255) as u8)
    }

    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

pub fn fsqrt(x: f32) -> f32 {
    if x <= 0.0 { return 0.0; }
    let mut guess = x * 0.5;
    for _ in 0..20 {
        guess = (guess + x / guess) * 0.5;
    }
    guess
}

pub fn fsin(x: f32) -> f32 {
    let pi = 3.14159265;
    let mut a = x % (2.0 * pi);
    if a < 0.0 { a += 2.0 * pi; }
    let a2 = a * a;
    let a3 = a2 * a;
    let a5 = a3 * a2;
    let a7 = a5 * a2;
    a - a3 / 6.0 + a5 / 120.0 - a7 / 5040.0
}

pub fn fcos(x: f32) -> f32 {
    fsin(x + 1.5707963)
}

pub enum ShaderOp {
    SolidColor(Vec4),
    Gradient { top: Vec4, bottom: Vec4 },
    RadialGradient { center: Vec2, radius: f32, inner: Vec4, outer: Vec4 },
    Noise { scale: f32, color: Vec4 },
    Pulse { speed: f32, color: Vec4 },
    Combine(Box<ShaderOp>, Box<ShaderOp>, BlendMode),
}

impl ShaderOp {
    fn evaluate(&self, x: f32, y: f32, w: f32, h: f32, tick: u64) -> Vec4 {
        match self {
            ShaderOp::SolidColor(color) => *color,
            ShaderOp::Gradient { top, bottom } => {
                let t = if h > 0.0 { y / h } else { 0.0_f32 };
                let t = t.clamp(0.0_f32, 1.0_f32);
                Vec4::new(
                    top.x + (bottom.x - top.x) * t,
                    top.y + (bottom.y - top.y) * t,
                    top.z + (bottom.z - top.z) * t,
                    1.0,
                )
            }
            ShaderOp::RadialGradient { center, radius, inner, outer } => {
                let dx = x - center.x;
                let dy = y - center.y;
                let dist = fsqrt(dx * dx + dy * dy);
                let t = (dist / *radius).clamp(0.0_f32, 1.0_f32);
                Vec4::new(
                    inner.x + (outer.x - inner.x) * t,
                    inner.y + (outer.y - inner.y) * t,
                    inner.z + (outer.z - inner.z) * t,
                    1.0,
                )
            }
            ShaderOp::Noise { scale, color } => {
                let hash = ((x * *scale) as u32).wrapping_mul(2654435761)
                    ^ ((y * *scale) as u32).wrapping_mul(2246822519)
                    ^ (tick as u32).wrapping_mul(3266489917);
                let noise = (hash & 0xFF) as f32 / 255.0_f32;
                Vec4::new(color.x * noise, color.y * noise, color.z * noise, 1.0)
            }
            ShaderOp::Pulse { speed, color } => {
                let phase = fsin(tick as f32 * *speed * 0.001) * 0.5 + 0.5;
                Vec4::new(color.x * phase, color.y * phase, color.z * phase, 1.0)
            }
            ShaderOp::Combine(a, b, mode) => {
                let ca = a.evaluate(x, y, w, h, tick);
                let cb = b.evaluate(x, y, w, h, tick);
                match mode {
                    BlendMode::Add => Vec4::new(
                        (ca.x + cb.x).min(1.0_f32),
                        (ca.y + cb.y).min(1.0_f32),
                        (ca.z + cb.z).min(1.0_f32),
                        1.0,
                    ),
                    BlendMode::Multiply => Vec4::new(ca.x * cb.x, ca.y * cb.y, ca.z * cb.z, 1.0),
                    BlendMode::Screen => Vec4::new(
                        (1.0 - (1.0 - ca.x) * (1.0 - cb.x)).min(1.0_f32),
                        (1.0 - (1.0 - ca.y) * (1.0 - cb.y)).min(1.0_f32),
                        (1.0 - (1.0 - ca.z) * (1.0 - cb.z)).min(1.0_f32),
                        1.0,
                    ),
                    BlendMode::Overlay => {
                        let ov = |a: f32, b: f32| -> f32 {
                            if a < 0.5 { 2.0 * a * b } else { 1.0 - 2.0 * (1.0 - a) * (1.0 - b) }
                        };
                        Vec4::new(ov(ca.x, cb.x), ov(ca.y, cb.y), ov(ca.z, cb.z), 1.0)
                    }
                }
            }
        }
    }
}

pub enum BlendMode {
    Add,
    Multiply,
    Overlay,
    Screen,
}

pub struct Shader {
    ops: alloc::vec::Vec<ShaderOp>,
}

impl Shader {
    pub fn new() -> Self {
        Self { ops: alloc::vec::Vec::new() }
    }

    pub fn solid(r: f32, g: f32, b: f32) -> Self {
        let mut s = Self::new();
        s.ops.push(ShaderOp::SolidColor(Vec4::new(r, g, b, 1.0)));
        s
    }

    pub fn gradient(top: Vec4, bottom: Vec4) -> Self {
        let mut s = Self::new();
        s.ops.push(ShaderOp::Gradient { top, bottom });
        s
    }

    pub fn evaluate(&self, x: f32, y: f32, w: f32, h: f32, tick: u64) -> Vec4 {
        if self.ops.is_empty() {
            return Vec4::new(0.0, 0.0, 0.0, 1.0);
        }
        self.ops[0].evaluate(x, y, w, h, tick)
    }
}
