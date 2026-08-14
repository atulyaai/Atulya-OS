use crate::display::{Display, Rgb};
use super::shader::Shader;

pub struct EffectRenderer;

impl EffectRenderer {
    pub fn new() -> Self {
        Self
    }

    pub fn apply_shader(
        &self,
        display: &mut Display,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        shader: &Shader,
        tick: u64,
    ) {
        let dw = display.width();
        let dh = display.height();
        for row in 0..height {
            let py = y + row;
            if py >= dh {
                break;
            }
            for col in 0..width {
                let px = x + col;
                if px >= dw {
                    break;
                }
                let fx = px as f32;
                let fy = py as f32;
                let fw = dw as f32;
                let fh = dh as f32;
                let color = shader.evaluate(fx, fy, fw, fh, tick);
                display.pixel(px, py, color.to_rgb());
            }
        }
    }

    pub fn draw_glass_panel(
        &self,
        display: &mut Display,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        radius: usize,
        tint: Rgb,
        alpha: u16,
    ) {
        display.rect_rounded_alpha(x, y, width, height, radius, tint, alpha);
        display.rect_rounded_outline(x, y, width, height, radius, Rgb::new(255, 255, 255));
    }

    pub fn draw_bloom(
        &self,
        display: &mut Display,
        cx: usize,
        cy: usize,
        radius: usize,
        color: Rgb,
        intensity: u16,
    ) {
        for i in (0..10u16).rev() {
            let r = radius + (i as usize) * radius / 5;
            let a = intensity / (i + 2);
            display.circle_alpha(cx, cy, r, color, a);
        }
        display.circle_alpha(cx, cy, radius / 3, color, 200);
    }

    pub fn draw_energy_ring(
        &self,
        display: &mut Display,
        cx: usize,
        cy: usize,
        radius: usize,
        color: Rgb,
        tick: u64,
    ) {
        let segments = 36;
        let pi = 3.14159265f32;
        for i in 0..segments {
            let angle = (i as u64 * 360 / segments as u64 + tick) % 360;
            let rad = angle as f32 * pi / 180.0;

            // Inline fsin approximation (Taylor series)
            let mut a = rad % (2.0 * pi);
            if a < 0.0 { a += 2.0 * pi; }
            let a2 = a * a;
            let a3 = a2 * a;
            let a5 = a3 * a2;
            let a7 = a5 * a2;
            let sin_val = a - a3 / 6.0 + a5 / 120.0 - a7 / 5040.0;

            let mut a2_val = rad + 1.5707963;
            a2_val = a2_val % (2.0 * pi);
            if a2_val < 0.0 { a2_val += 2.0 * pi; }
            let a2_2 = a2_val * a2_val;
            let a2_3 = a2_2 * a2_val;
            let a2_5 = a2_3 * a2_2;
            let a2_7 = a2_5 * a2_2;
            let cos_val = a2_val - a2_3 / 6.0 + a2_5 / 120.0 - a2_7 / 5040.0;

            let x = cx as f32 + (radius as f32 * cos_val);
            let y = cy as f32 + (radius as f32 * sin_val);
            let brightness = ((tick + i as u64) % 10) as u16;
            let c = color.dim(128 + brightness * 12);
            display.circle_filled(x as usize, y as usize, 2, c);
        }
    }
}
