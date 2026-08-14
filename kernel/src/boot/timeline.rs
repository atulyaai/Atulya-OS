use crate::boot::boot_state::BootStage;
use crate::display::{Display, Rgb};
use crate::font;
use crate::math;

const TOTAL_MS: u32 = 2_000;

#[derive(Clone, Copy)]
struct OrbModel {
    radius: usize,
    glow: u16,
    rotation: i32,
    energy: u16,
    pulse: u16,
}

impl OrbModel {
    const fn new() -> Self {
        Self {
            radius: 2,
            glow: 0,
            rotation: 0,
            energy: 0,
            pulse: 0,
        }
    }
}

pub struct BootTimeline {
    elapsed_ms: u32,
    stage: BootStage,
    orb: OrbModel,
}

impl BootTimeline {
    pub const fn new() -> Self {
        Self {
            elapsed_ms: 0,
            stage: BootStage::PowerOn,
            orb: OrbModel::new(),
        }
    }

    pub fn tick(&mut self, dt_ms: u32) {
        self.elapsed_ms = self.elapsed_ms.saturating_add(dt_ms).min(TOTAL_MS);
        self.stage = self.stage_for_time();
        self.orb.rotation = (self.orb.rotation + (dt_ms as i32 / 2).max(1)) % 360;
        self.orb.pulse = ((self.orb.pulse as u32 + dt_ms * 3) % 1024) as u16;
        self.orb.energy = progress(self.elapsed_ms, 0, 4_000);

        let grow = progress(self.elapsed_ms, 0, 4_000) as usize;
        let formed = progress(self.elapsed_ms, 2_000, 4_000) as usize;
        self.orb.radius = 3 + (formed * 88 / 256);
        self.orb.glow = (grow as u16).max(progress(self.elapsed_ms, 4_000, TOTAL_MS) / 2);
    }

    pub fn done(&self) -> bool {
        self.elapsed_ms >= TOTAL_MS
    }

    pub fn render(&self, display: &mut Display) {
        let w = display.width();
        let h = display.height();
        let cx = w / 2;
        let cy = h / 2;

        self.draw_background(display, w, h);
        self.draw_workspace_emergence(display, w, h, cx, cy);
        self.draw_knowledge_graph(display, w, h, cx, cy);
        self.draw_orb(display, cx, cy);
        self.draw_rings(display, cx, cy);
        self.draw_hud(display, w, h);
        self.draw_status_text(display, w, h);
    }

    fn stage_for_time(&self) -> BootStage {
        match self.elapsed_ms {
            0..=1_999 => BootStage::PowerOn,
            2_000..=3_999 => BootStage::CoreFormation,
            4_000..=5_999 => BootStage::CoreOnline,
            6_000..=7_999 => BootStage::MemoryOnline,
            8_000..=9_999 => BootStage::VoiceOnline,
            10_000..=11_999 => BootStage::VisionOnline,
            12_000..=13_999 => BootStage::SkillsOnline,
            14_000..=15_999 => BootStage::KnowledgeGraph,
            16_000..=17_999 => BootStage::IdentityRestore,
            _ => BootStage::WorkspaceReady,
        }
    }

    fn draw_background(&self, display: &mut Display, w: usize, h: usize) {
        let open = progress(self.elapsed_ms, 0, 4_000);
        let top = Rgb::new(0, 0, 4).lerp(Rgb::new(4, 10, 28), open);
        let bottom = Rgb::new(0, 0, 8).lerp(Rgb::new(3, 24, 44), open);
        display.gradient_rect_v(0, 0, w, h, top, bottom);

        let grid_alpha = progress(self.elapsed_ms, 5_000, 14_000) / 5;
        if grid_alpha > 0 {
            let grid = Rgb::new(0, 170, 255).dim(grid_alpha);
            let step = (w / 24).max(32);
            let mut x = 0usize;
            while x < w {
                for y in (0..h).step_by(7) {
                    display.pixel(x, y, grid);
                }
                x += step;
            }
            let mut y = 0usize;
            while y < h {
                for x in (0..w).step_by(7) {
                    display.pixel(x, y, grid);
                }
                y += step;
            }
        }
    }

    fn draw_workspace_emergence(&self, display: &mut Display, w: usize, h: usize, cx: usize, cy: usize) {
        let alpha = progress(self.elapsed_ms, 18_000, 20_000);
        if alpha == 0 {
            return;
        }

        let sky = Rgb::new(5, 16, 42).dim(alpha);
        let ground = Rgb::new(10, 42, 70).dim(alpha);
        display.gradient_rect_v(0, h * 2 / 3, w, h / 3, sky, ground);

        let horizon = h * 2 / 3;
        let beam = Rgb::new(0, 220, 255).dim(alpha);
        display.rect_alpha(cx.saturating_sub(2), cy, 4, horizon.saturating_sub(cy), beam, alpha / 2);
        for i in 0..9 {
            let x0 = cx.saturating_sub(i * 24);
            let x1 = (cx + i * 24).min(w.saturating_sub(1));
            let y = horizon + i * 8;
            if y < h {
                display.draw_line(x0, y, x1, y, beam);
            }
        }

        let side_w = w / 7;
        display.rect_alpha(0, 0, side_w, h, Rgb::new(2, 8, 18), alpha / 2);
        display.rect_alpha(w.saturating_sub(side_w), 0, side_w, h, Rgb::new(2, 8, 18), alpha / 3);

        let dock_w = w / 3;
        let dock_x = (w - dock_w) / 2;
        let dock_y = h.saturating_sub(72);
        display.rect_rounded_alpha(dock_x, dock_y, dock_w, 44, 8, Rgb::new(5, 14, 28), alpha / 2);
        for i in 0..7 {
            let ix = dock_x + 18 + i * ((dock_w - 36) / 7);
            display.rect_rounded_alpha(ix, dock_y + 8, 28, 28, 5, Rgb::new(0, 155, 255), alpha / 4);
            display.rect_rounded_outline(ix, dock_y + 8, 28, 28, 5, Rgb::new(0, 210, 255).dim(alpha));
        }
    }

    fn draw_knowledge_graph(&self, display: &mut Display, w: usize, h: usize, cx: usize, cy: usize) {
        let graph = progress(self.elapsed_ms, 6_000, 16_000);
        if graph == 0 {
            return;
        }

        let nodes = [
            (-210, -88), (-150, 72), (-82, -132), (-36, 104),
            (84, -112), (132, 60), (214, -72), (246, 122),
            (-286, 24), (306, 8), (-16, -196), (24, 194),
        ];

        let visible = (nodes.len() * graph as usize) / 256;
        let line = Rgb::new(130, 64, 255).dim((graph / 3).max(18));
        for i in 0..visible {
            let (ax, ay) = nodes[i];
            let x0 = signed_add(cx, ax * w as isize / 960);
            let y0 = signed_add(cy, ay * h as isize / 540);
            if i > 0 {
                let target = if i % 3 == 0 { 0 } else { i - 1 };
                let (bx, by) = nodes[target];
                let x1 = signed_add(cx, bx * w as isize / 960);
                let y1 = signed_add(cy, by * h as isize / 540);
                display.draw_line(x0, y0, x1, y1, line);
            }
        }

        for i in 0..visible {
            let (nx, ny) = nodes[i];
            let x = signed_add(cx, nx * w as isize / 960);
            let y = signed_add(cy, ny * h as isize / 540);
            let pulse = math::sinish(self.orb.rotation + i as i32 * 31).unsigned_abs() as u16;
            let color = Rgb::new(0, 210, 255).lerp(Rgb::new(255, 70, 240), pulse / 4);
            display.circle_alpha(x, y, 2 + (pulse as usize / 400), color, graph.min(180));
        }
    }

    fn draw_orb(&self, display: &mut Display, cx: usize, cy: usize) {
        let blue = Rgb::new(0, 210, 255);
        let purple = Rgb::new(180, 64, 255);
        let orange = Rgb::new(255, 116, 48);
        let pulse = math::sinish(self.orb.pulse as i32).unsigned_abs();
        let radius = self.orb.radius + (pulse * 6 / 1024);

        for i in (1..=8).rev() {
            let r = radius + i * 10;
            let a = (self.orb.glow / (i as u16 + 1)).min(72);
            display.circle_alpha(cx, cy, r, blue, a);
            display.circle_alpha(cx, cy, r + 3, purple, a / 2);
        }

        display.circle_alpha(cx, cy, radius, Rgb::new(7, 18, 44), 210);
        display.circle_alpha(cx, cy, radius / 2, blue, 52 + self.orb.energy / 3);

        let formed = progress(self.elapsed_ms, 2_000, 4_000);
        if formed > 0 {
            for i in 0..28 {
                let angle = self.orb.rotation + i * 13;
                let c = math::cosish(angle);
                let s = math::sinish(angle);
                let inner = radius as isize * 64 / 100;
                let outer = radius as isize;
                let x0 = (cx as isize + c * inner / 1024) as usize;
                let y0 = (cy as isize + s * inner / 1024) as usize;
                let x1 = (cx as isize + c * outer / 1024) as usize;
                let y1 = (cy as isize + s * outer / 1024) as usize;
                let color = if i % 5 == 0 { orange } else if i % 2 == 0 { purple } else { blue };
                display.draw_line(x0, y0, x1, y1, color.dim(formed));
            }
        }

        let label_alpha = progress(self.elapsed_ms, 3_300, 4_400);
        if label_alpha > 0 {
            centered_text(display, cx, cy.saturating_sub(14), "ATULYA", 2, Rgb::new(185, 245, 255).dim(label_alpha));
            centered_text(display, cx, cy + 8, "OS", 1, Rgb::new(155, 120, 255).dim(label_alpha));
        }
    }

    fn draw_rings(&self, display: &mut Display, cx: usize, cy: usize) {
        let ring_base = self.orb.radius + 28;
        let memory = progress(self.elapsed_ms, 6_000, 8_000);
        let voice = progress(self.elapsed_ms, 8_000, 10_000);
        let vision = progress(self.elapsed_ms, 10_000, 12_000);
        let skills = progress(self.elapsed_ms, 12_000, 14_000);
        let intelligence = progress(self.elapsed_ms, 14_000, 16_000);

        self.draw_ring(display, cx, cy, ring_base, memory, Rgb::new(190, 80, 255), 0);
        self.draw_ring(display, cx, cy, ring_base + 18, voice, Rgb::new(0, 220, 255), 23);
        self.draw_ring(display, cx, cy, ring_base + 36, vision, Rgb::new(0, 132, 255), 51);
        self.draw_ring(display, cx, cy, ring_base + 54, skills, Rgb::new(255, 152, 42), 79);
        self.draw_ring(display, cx, cy, ring_base + 72, intelligence, Rgb::new(110, 255, 180), 103);
    }

    fn draw_ring(&self, display: &mut Display, cx: usize, cy: usize, r: usize, alpha: u16, color: Rgb, offset: i32) {
        if alpha == 0 {
            return;
        }
        let dim = color.dim(alpha);
        display.circle_outline(cx, cy, r, dim);
        if alpha > 160 {
            display.circle_outline(cx, cy, r + 1, dim);
        }

        for i in 0..18 {
            if (i * 256 / 18) as u16 > alpha {
                continue;
            }
            let angle = self.orb.rotation + offset + i as i32 * 20;
            let c = math::cosish(angle);
            let s = math::sinish(angle);
            let inner = r as isize - 4;
            let outer = r as isize + 8;
            let x0 = (cx as isize + c * inner / 1024) as usize;
            let y0 = (cy as isize + s * inner / 1024) as usize;
            let x1 = (cx as isize + c * outer / 1024) as usize;
            let y1 = (cy as isize + s * outer / 1024) as usize;
            display.draw_line(x0, y0, x1, y1, dim);
        }
    }

    fn draw_hud(&self, display: &mut Display, w: usize, h: usize) {
        let hud = progress(self.elapsed_ms, 6_000, 14_000);
        if hud == 0 {
            return;
        }

        let left_x = w / 12;
        let right_x = w.saturating_sub(w / 12 + 148);
        let mid_y = h / 2;
        self.draw_subsystem(display, left_x, mid_y.saturating_sub(88), "MEMORY", progress(self.elapsed_ms, 6_000, 8_000), Rgb::new(190, 80, 255));
        self.draw_subsystem(display, left_x, mid_y + 28, "VOICE", progress(self.elapsed_ms, 8_000, 10_000), Rgb::new(0, 220, 255));
        self.draw_subsystem(display, right_x, mid_y.saturating_sub(52), "VISION", progress(self.elapsed_ms, 10_000, 12_000), Rgb::new(0, 132, 255));
        self.draw_subsystem(display, right_x, mid_y + 68, "SKILLS", progress(self.elapsed_ms, 12_000, 14_000), Rgb::new(255, 152, 42));
        self.draw_subsystem(display, (w - 180) / 2, h.saturating_sub(118), "INTELLIGENCE", progress(self.elapsed_ms, 14_000, 16_000), Rgb::new(110, 255, 180));
    }

    fn draw_subsystem(&self, display: &mut Display, x: usize, y: usize, name: &str, alpha: u16, color: Rgb) {
        if alpha == 0 {
            return;
        }
        let col = color.dim(alpha);
        font::draw_text(display, x, y, name, 1, col);
        let state = if alpha < 220 { "Initializing" } else { "Online" };
        font::draw_text(display, x, y + 13, state, 1, Rgb::new(170, 225, 255).dim(alpha));
        display.circle_alpha(x.saturating_sub(14), y + 6, 5, col, alpha);
    }

    fn draw_status_text(&self, display: &mut Display, w: usize, h: usize) {
        let status = match self.stage {
            BootStage::PowerOn => "SYSTEM POWER ON",
            BootStage::CoreFormation => "ENERGY INITIALIZING",
            BootStage::CoreOnline => "ATULYA CORE ONLINE",
            BootStage::MemoryOnline => "MEMORY ONLINE",
            BootStage::VoiceOnline => "VOICE ONLINE",
            BootStage::VisionOnline => "VISION ONLINE",
            BootStage::SkillsOnline => "SKILLS ONLINE",
            BootStage::KnowledgeGraph => "KNOWLEDGE GRAPH ACTIVATED",
            BootStage::IdentityRestore => "IDENTITY RESTORED",
            BootStage::WorkspaceReady => {
                if self.elapsed_ms < 19_400 {
                    "PREPARING WORKSPACE"
                } else {
                    "ATULYA IS READY"
                }
            }
        };

        let pct = (self.elapsed_ms.min(TOTAL_MS) * 100) / TOTAL_MS;
        let mut buf = [0u8; 4];
        let pct_str = percent_text(pct, &mut buf);
        let y = h.saturating_sub(54);
        centered_text(display, w / 2, y, pct_str, 1, Rgb::new(180, 230, 255));
        centered_text(display, w / 2, y + 18, status, 1, Rgb::new(120, 220, 255));

        if self.elapsed_ms >= 16_000 {
            let alpha = progress(self.elapsed_ms, 16_000, 18_000);
            centered_text(display, w / 2, h / 2 + h / 5, "HELLO ATUL", 2, Rgb::new(0, 220, 255).dim(alpha));
        }
    }
}

fn progress(now: u32, start: u32, end: u32) -> u16 {
    if now <= start {
        return 0;
    }
    if now >= end {
        return 256;
    }
    let span = end - start;
    (((now - start) as u64 * 256) / span as u64) as u16
}

fn signed_add(base: usize, delta: isize) -> usize {
    if delta < 0 {
        base.saturating_sub((-delta) as usize)
    } else {
        base.saturating_add(delta as usize)
    }
}

fn centered_text(display: &mut Display, cx: usize, y: usize, text: &str, scale: usize, color: Rgb) {
    let width = font::text_width(text, scale);
    font::draw_text(display, cx.saturating_sub(width / 2), y, text, scale, color);
}

fn percent_text<'a>(pct: u32, buf: &'a mut [u8; 4]) -> &'a str {
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
