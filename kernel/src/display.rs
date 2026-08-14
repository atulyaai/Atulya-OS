use bootloader_api::info::{FrameBufferInfo, PixelFormat};

#[derive(Clone, Copy)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Linearly interpolate between two colors. `t` is 0..256 (0 = self, 256 = other).
    pub fn lerp(self, other: Rgb, t: u16) -> Rgb {
        let t = if t > 256 { 256 } else { t };
        let inv = 256 - t;
        Rgb {
            r: ((self.r as u16 * inv + other.r as u16 * t) >> 8) as u8,
            g: ((self.g as u16 * inv + other.g as u16 * t) >> 8) as u8,
            b: ((self.b as u16 * inv + other.b as u16 * t) >> 8) as u8,
        }
    }

    /// Multiply color brightness by factor (0..256, where 256 = full brightness).
    pub fn dim(self, factor: u16) -> Rgb {
        let f = if factor > 256 { 256 } else { factor };
        Rgb {
            r: ((self.r as u16 * f) >> 8) as u8,
            g: ((self.g as u16 * f) >> 8) as u8,
            b: ((self.b as u16 * f) >> 8) as u8,
        }
    }
}

pub struct Display<'a> {
    pub buffer: &'a mut [u8],
    pub backbuffer: &'a mut [u8],
    pub info: FrameBufferInfo,
}

impl<'a> Display<'a> {
    pub fn swap_buffers(&mut self) {
        let len = self.backbuffer.len().min(self.buffer.len());
        self.buffer[..len].copy_from_slice(&self.backbuffer[..len]);
    }

    pub fn width(&self) -> usize {
        self.info.width
    }

    pub fn height(&self) -> usize {
        self.info.height
    }

    pub fn clear(&mut self, color: Rgb) {
        for y in 0..self.info.height {
            for x in 0..self.info.width {
                self.pixel(x, y, color);
            }
        }
    }

    pub fn rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: Rgb) {
        for row in y..y.saturating_add(height).min(self.info.height) {
            for col in x..x.saturating_add(width).min(self.info.width) {
                self.pixel(col, row, color);
            }
        }
    }

    pub fn rect_outline(&mut self, x: usize, y: usize, width: usize, height: usize, color: Rgb) {
        self.rect(x, y, width, 2, color);
        self.rect(x, y + height.saturating_sub(2), width, 2, color);
        self.rect(x, y, 2, height, color);
        self.rect(x + width.saturating_sub(2), y, 2, height, color);
    }

    /// Draw a 1px outline rectangle.
    pub fn rect_outline_1px(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        color: Rgb,
    ) {
        // Top edge
        for col in x..x.saturating_add(width).min(self.info.width) {
            self.pixel(col, y, color);
        }
        // Bottom edge
        let bottom = y + height.saturating_sub(1);
        for col in x..x.saturating_add(width).min(self.info.width) {
            self.pixel(col, bottom, color);
        }
        // Left edge
        for row in y..y.saturating_add(height).min(self.info.height) {
            self.pixel(x, row, color);
        }
        // Right edge
        let right = x + width.saturating_sub(1);
        for row in y..y.saturating_add(height).min(self.info.height) {
            self.pixel(right, row, color);
        }
    }

    /// Filled circle at (cx, cy) with given radius.
    pub fn circle_filled(&mut self, cx: usize, cy: usize, radius: usize, color: Rgb) {
        let r = radius as isize;
        for dy in -r..=r {
            let py = cy as isize + dy;
            if py < 0 || py >= self.info.height as isize { continue; }
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r {
                    let px = cx as isize + dx;
                    if px >= 0 && px < self.info.width as isize {
                        self.pixel(px as usize, py as usize, color);
                    }
                }
            }
        }
    }

    /// Filled circle with alpha blending.
    pub fn circle_alpha(&mut self, cx: usize, cy: usize, radius: usize, color: Rgb, alpha: u16) {
        let r = radius as isize;
        let a = if alpha > 256 { 256 } else { alpha };
        let inv = 256 - a;
        for dy in -r..=r {
            let py = cy as isize + dy;
            if py < 0 || py >= self.info.height as isize { continue; }
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r {
                    let px = cx as isize + dx;
                    if px >= 0 && px < self.info.width as isize {
                        let bg = self.read_pixel(px as usize, py as usize);
                        let blended = Rgb::new(
                            ((color.r as u16 * a + bg.r as u16 * inv) >> 8) as u8,
                            ((color.g as u16 * a + bg.g as u16 * inv) >> 8) as u8,
                            ((color.b as u16 * a + bg.b as u16 * inv) >> 8) as u8,
                        );
                        self.pixel(px as usize, py as usize, blended);
                    }
                }
            }
        }
    }

    /// Draw a glowing orb using multiple layers of alpha circles.
    pub fn draw_glow_orb(&mut self, cx: usize, cy: usize, radius: usize, color: Rgb) {
        // Multi-layered radial gradient for a smoother "energy" look
        for i in (0..15).rev() {
            let r = radius + (i * radius / 3);
            let alpha = (128 / (i + 1)) as u16;
            self.circle_alpha(cx, cy, r, color, alpha);
        }
        // Core glow
        self.circle_alpha(cx, cy, radius, color, 200);
        self.circle_alpha(cx, cy, radius / 2, Rgb::new(255, 255, 255), 100);
    }

    /// Blend two colors with a ratio (0..256)
    pub fn blend(c1: Rgb, c2: Rgb, ratio: u16) -> Rgb {
        c1.lerp(c2, ratio)
    }

    /// Rectangle with a vertical gradient from `top_color` to `bottom_color`.
    pub fn gradient_rect_v(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        top_color: Rgb,
        bottom_color: Rgb,
    ) {
        if height == 0 {
            return;
        }
        for row in 0..height {
            let t = ((row as u32 * 256) / height as u32) as u16;
            let color = top_color.lerp(bottom_color, t);
            let py = y + row;
            if py >= self.info.height {
                break;
            }
            for col in x..x.saturating_add(width).min(self.info.width) {
                self.pixel(col, py, color);
            }
        }
    }

    /// Rectangle with a horizontal gradient from `left_color` to `right_color`.
    pub fn gradient_rect_h(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        left_color: Rgb,
        right_color: Rgb,
    ) {
        if width == 0 {
            return;
        }
        for col in 0..width {
            let t = ((col as u32 * 256) / width as u32) as u16;
            let color = left_color.lerp(right_color, t);
            let px = x + col;
            if px >= self.info.width {
                break;
            }
            for row in y..y.saturating_add(height).min(self.info.height) {
                self.pixel(px, row, color);
            }
        }
    }

    /// Horizontal gradient with rounded top corners.
    pub fn gradient_rect_h_rounded_top(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        r: usize,
        left_color: Rgb,
        right_color: Rgb,
    ) {
        if width == 0 {
            return;
        }
        for col in 0..width {
            let t = ((col as u32 * 256) / width as u32) as u16;
            let color = left_color.lerp(right_color, t);
            let px = x + col;
            if px >= self.info.width {
                break;
            }
            let dx = if col < r {
                Some(r - col)
            } else if col >= width - r {
                Some(col - (width - r) + 1)
            } else {
                None
            };
            for row in 0..height {
                let py = y + row;
                if py >= self.info.height {
                    break;
                }
                let dy = if row < r {
                    Some(r - row)
                } else {
                    None
                };

                if let (Some(cx), Some(cy)) = (dx, dy) {
                    if cx * cx + cy * cy > r * r {
                        continue;
                    }
                }

                self.pixel(px, py, color);
            }
        }
    }

    /// Draw a rectangle with simulated alpha blending over a base color.
    /// `alpha` is 0..256 where 256 = fully opaque.
    pub fn rect_alpha(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        color: Rgb,
        alpha: u16,
    ) {
        let a = if alpha > 256 { 256 } else { alpha };
        let inv = 256 - a;
        for row in y..y.saturating_add(height).min(self.info.height) {
            for col in x..x.saturating_add(width).min(self.info.width) {
                let bg = self.read_pixel(col, row);
                let blended = Rgb::new(
                    ((color.r as u16 * a + bg.r as u16 * inv) >> 8) as u8,
                    ((color.g as u16 * a + bg.g as u16 * inv) >> 8) as u8,
                    ((color.b as u16 * a + bg.b as u16 * inv) >> 8) as u8,
                );
                self.pixel(col, row, blended);
            }
        }
    }

    /// Draw a rounded rectangle with simulated alpha blending and frosted glass dithering.
    pub fn rect_rounded_alpha(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        r: usize,
        color: Rgb,
        alpha: u16,
    ) {
        let a = if alpha > 256 { 256 } else { alpha };
        let inv = 256 - a;
        for row in y..y.saturating_add(height).min(self.info.height) {
            let dy = if row < y + r {
                Some(y + r - row)
            } else if row >= y + height - r {
                Some(row - (y + height - r) + 1)
            } else {
                None
            };
            for col in x..x.saturating_add(width).min(self.info.width) {
                let dx = if col < x + r {
                    Some(x + r - col)
                } else if col >= x + width - r {
                    Some(col - (x + width - r) + 1)
                } else {
                    None
                };

                if let (Some(cx), Some(cy)) = (dx, dy) {
                    if cx * cx + cy * cy > r * r {
                        continue;
                    }
                }

                // Frosted glass dithering noise (+/- 4 to color component)
                let noise = if (col + row) % 2 == 0 { 4 } else { 0 };
                let cr = color.r.saturating_add(noise);
                let cg = color.g.saturating_add(noise);
                let cb = color.b.saturating_add(noise);

                let bg = self.read_pixel(col, row);
                let blended = Rgb::new(
                    ((cr as u16 * a + bg.r as u16 * inv) >> 8) as u8,
                    ((cg as u16 * a + bg.g as u16 * inv) >> 8) as u8,
                    ((cb as u16 * a + bg.b as u16 * inv) >> 8) as u8,
                );
                self.pixel(col, row, blended);
            }
        }
    }

    /// Draw a rounded rectangle outline.
    pub fn rect_rounded_outline(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        r: usize,
        color: Rgb,
    ) {
        // Draw straight edges
        self.rect(x + r, y, width.saturating_sub(2 * r), 1, color); // Top
        self.rect(x + r, y + height - 1, width.saturating_sub(2 * r), 1, color); // Bottom
        self.rect(x, y + r, 1, height.saturating_sub(2 * r), color); // Left
        self.rect(x + width - 1, y + r, 1, height.saturating_sub(2 * r), color); // Right

        // Draw corners
        for dy in 0..r {
            for dx in 0..r {
                let dist2 = dx * dx + dy * dy;
                let r2 = r * r;
                if dist2 >= r2.saturating_sub(r) && dist2 <= r2 + r {
                    self.pixel(x + r - dx, y + r - dy, color);
                    self.pixel(x + width - r + dx, y + r - dy, color);
                    self.pixel(x + r - dx, y + height - r + dy, color);
                    self.pixel(x + width - r + dx, y + height - r + dy, color);
                }
            }
        }
    }

    /// Read back a pixel color from the framebuffer.
    pub fn read_pixel(&self, x: usize, y: usize) -> Rgb {
        if x >= self.info.width || y >= self.info.height {
            return Rgb::new(0, 0, 0);
        }
        let byte_offset = (y * self.info.stride + x) * self.info.bytes_per_pixel;
        if byte_offset + self.info.bytes_per_pixel > self.backbuffer.len() {
            return Rgb::new(0, 0, 0);
        }
        match self.info.pixel_format {
            PixelFormat::Rgb => Rgb::new(
                self.backbuffer[byte_offset],
                self.backbuffer[byte_offset + 1],
                self.backbuffer[byte_offset + 2],
            ),
            PixelFormat::Bgr => Rgb::new(
                self.backbuffer[byte_offset + 2],
                self.backbuffer[byte_offset + 1],
                self.backbuffer[byte_offset],
            ),
            PixelFormat::U8 => {
                let v = self.backbuffer[byte_offset];
                Rgb::new(v, v, v)
            }
            _ => Rgb::new(0, 0, 0),
        }
    }

    pub fn pixel_signed(&mut self, x: isize, y: isize, color: Rgb) {
        if x >= 0 && y >= 0 {
            self.pixel(x as usize, y as usize, color);
        }
    }

    pub fn pixel(&mut self, x: usize, y: usize, color: Rgb) {
        if x >= self.info.width || y >= self.info.height {
            return;
        }

        let byte_offset = y * self.info.stride + x;
        let byte_offset = byte_offset * self.info.bytes_per_pixel;
        if byte_offset + self.info.bytes_per_pixel > self.backbuffer.len() {
            return;
        }

        match self.info.pixel_format {
            PixelFormat::Rgb => {
                self.backbuffer[byte_offset] = color.r;
                self.backbuffer[byte_offset + 1] = color.g;
                self.backbuffer[byte_offset + 2] = color.b;
            }
            PixelFormat::Bgr => {
                self.backbuffer[byte_offset] = color.b;
                self.backbuffer[byte_offset + 1] = color.g;
                self.backbuffer[byte_offset + 2] = color.r;
            }
            PixelFormat::U8 => {
                self.backbuffer[byte_offset] =
                    ((color.r as u16 + color.g as u16 + color.b as u16) / 3) as u8;
            }
            _ => {}
        }
    }

    /// Draw a circle outline using Bresenham's circle algorithm.
    pub fn circle_outline(&mut self, cx: usize, cy: usize, radius: usize, color: Rgb) {
        let mut x = radius as isize;
        let mut y = 0isize;
        let mut err = 0isize;

        while x >= y {
            self.pixel_signed(cx as isize + x, cy as isize + y, color);
            self.pixel_signed(cx as isize + y, cy as isize + x, color);
            self.pixel_signed(cx as isize - y, cy as isize + x, color);
            self.pixel_signed(cx as isize - x, cy as isize + y, color);
            self.pixel_signed(cx as isize - x, cy as isize - y, color);
            self.pixel_signed(cx as isize - y, cy as isize - x, color);
            self.pixel_signed(cx as isize + y, cy as isize - x, color);
            self.pixel_signed(cx as isize + x, cy as isize - y, color);

            y += 1;
            if err <= 0 {
                err += 2 * y + 1;
            } else {
                x -= 1;
                err -= 2 * x - 2 * y + 1;
            }
        }
    }

    /// Draw a dotted circle outline — every `gap`-th pixel is drawn.
    pub fn dotted_circle_outline(
        &mut self,
        cx: usize,
        cy: usize,
        radius: usize,
        gap: usize,
        color: Rgb,
    ) {
        let gap = gap.max(2);
        let mut x = radius as isize;
        let mut y = 0isize;
        let mut err = 0isize;
        let mut idx = 0usize;

        while x >= y {
            if idx % gap == 0 {
                self.pixel_signed(cx as isize + x, cy as isize + y, color);
                self.pixel_signed(cx as isize + y, cy as isize + x, color);
                self.pixel_signed(cx as isize - y, cy as isize + x, color);
                self.pixel_signed(cx as isize - x, cy as isize + y, color);
                self.pixel_signed(cx as isize - x, cy as isize - y, color);
                self.pixel_signed(cx as isize - y, cy as isize - x, color);
                self.pixel_signed(cx as isize + y, cy as isize - x, color);
                self.pixel_signed(cx as isize + x, cy as isize - y, color);
            }
            idx += 1;
            y += 1;
            if err <= 0 {
                err += 2 * y + 1;
            } else {
                x -= 1;
                err -= 2 * x - 2 * y + 1;
            }
        }
    }

    /// Draw radial tick marks on a circle arc.
    ///
    /// `n_ticks` tick marks are evenly spaced around the circle at `radius`.
    /// Each tick extends from `radius - inner_len` to `radius + outer_len`.
    /// `rotation_deg` rotates all ticks (in degrees, 0..360).
    /// Uses integer-approximation trig from `crate::math`.
    pub fn draw_arc_ticks(
        &mut self,
        cx: usize,
        cy: usize,
        radius: usize,
        n_ticks: usize,
        inner_len: usize,
        outer_len: usize,
        rotation_deg: i32,
        color: Rgb,
    ) {
        if n_ticks == 0 {
            return;
        }
        for i in 0..n_ticks {
            let angle = rotation_deg + (i as i32 * 360 / n_ticks as i32);
            let c = crate::math::cosish(angle);
            let s = crate::math::sinish(angle);
            let inner = radius.saturating_sub(inner_len) as isize;
            let outer = (radius + outer_len) as isize;
            let x0 = cx as isize + c * inner / 1024;
            let y0 = cy as isize + s * inner / 1024;
            let x1 = cx as isize + c * outer / 1024;
            let y1 = cy as isize + s * outer / 1024;
            // Use signed line drawing
            if x0 >= 0 && y0 >= 0 && x1 >= 0 && y1 >= 0 {
                self.draw_line(x0 as usize, y0 as usize, x1 as usize, y1 as usize, color);
            }
        }
    }

    /// Fast horizontal line fill (used for filled shapes and beam effects).
    pub fn hline(&mut self, x0: usize, x1: usize, y: usize, color: Rgb) {
        if y >= self.info.height {
            return;
        }
        let start = x0.min(self.info.width);
        let end = x1.min(self.info.width);
        for x in start..end {
            self.pixel(x, y, color);
        }
    }

    /// Additive pixel blend — adds color values (clamped to 255).
    pub fn pixel_add(&mut self, x: usize, y: usize, color: Rgb) {
        if x >= self.info.width || y >= self.info.height {
            return;
        }
        let cur = self.read_pixel(x, y);
        self.pixel(
            x,
            y,
            Rgb::new(
                cur.r.saturating_add(color.r),
                cur.g.saturating_add(color.g),
                cur.b.saturating_add(color.b),
            ),
        );
    }

    /// Draw a line using Bresenham's line algorithm.
    pub fn draw_line(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, color: Rgb) {
        let mut x0 = x0 as isize;
        let mut y0 = y0 as isize;
        let x1 = x1 as isize;
        let y1 = y1 as isize;

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.pixel_signed(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// Alpha-blend a pixel onto the backbuffer (alpha 0..255).
    pub fn pixel_alpha(&mut self, x: usize, y: usize, color: Rgb, alpha: u8) {
        if x >= self.info.width || y >= self.info.height || alpha == 0 {
            return;
        }
        if alpha == 255 {
            self.pixel(x, y, color);
            return;
        }
        let cur = self.read_pixel(x, y);
        let blended = cur.lerp(color, (alpha as u16) * 256 / 255);
        self.pixel(x, y, blended);
    }

    /// Blit RGBA sprite data with per-pixel alpha channel.
    pub fn blit_rgba_sprite(&mut self, x: usize, y: usize, w: usize, h: usize, data: &[u8]) {
        for row in 0..h {
            let py = y + row;
            if py >= self.info.height {
                break;
            }
            for col in 0..w {
                let px = x + col;
                if px >= self.info.width {
                    break;
                }
                let offset = (row * w + col) * 4;
                if offset + 3 < data.len() {
                    let r = data[offset];
                    let g = data[offset + 1];
                    let b = data[offset + 2];
                    let a = data[offset + 3];
                    if a > 0 {
                        self.pixel_alpha(px, py, Rgb::new(r, g, b), a);
                    }
                }
            }
        }
    }
}
