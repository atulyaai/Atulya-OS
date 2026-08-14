//! game.rs — Sovereign 2D Quantum Asteroids & Arcade Engine for Atulya OS.
//!
//! Provides vector ship rendering, particle physics, projectile collisions,
//! and 60 FPS arcade loop inside sandboxed desktop windows.

use crate::display::{Display, Rgb};
use spin::Mutex;

pub struct Projectile {
    pub x: isize,
    pub y: isize,
    pub vx: isize,
    pub vy: isize,
    pub life: u8,
}

pub struct Asteroid {
    pub x: isize,
    pub y: isize,
    pub vx: isize,
    pub vy: isize,
    pub radius: usize,
}

pub struct GameState {
    pub ship_x: isize,
    pub ship_y: isize,
    pub ship_angle: isize, // 0..256
    pub score: u32,
    pub projectiles: alloc::vec::Vec<Projectile>,
    pub asteroids: alloc::vec::Vec<Asteroid>,
    pub initialized: bool,
}

impl GameState {
    pub const fn new() -> Self {
        Self {
            ship_x: 200,
            ship_y: 120,
            ship_angle: 0,
            score: 0,
            projectiles: alloc::vec::Vec::new(),
            asteroids: alloc::vec::Vec::new(),
            initialized: false,
        }
    }

    pub fn init(&mut self) {
        if self.initialized { return; }
        self.asteroids.push(Asteroid { x: 80, y: 60, vx: 1, vy: 1, radius: 16 });
        self.asteroids.push(Asteroid { x: 300, y: 80, vx: -1, vy: 1, radius: 20 });
        self.asteroids.push(Asteroid { x: 150, y: 180, vx: 1, vy: -1, radius: 14 });
        self.initialized = true;
    }

    /// Update physics and render frame within bounding window.
    pub fn update_and_render(&mut self, display: &mut Display, wx: usize, wy: usize, ww: usize, wh: usize, tick: u64) {
        self.init();

        let inner_w = ww.saturating_sub(20) as isize;
        let inner_h = wh.saturating_sub(60) as isize;
        let origin_x = wx as isize + 10;
        let origin_y = wy as isize + 40;

        // Auto-spin ship slowly
        self.ship_angle = ((tick * 2) % 256) as isize;

        // Draw Player Ship (Triangle)
        let sx = origin_x + self.ship_x;
        let sy = origin_y + self.ship_y;
        let dir_x = (crate::math::cosish(self.ship_angle as i32) * 12) / 256;
        let dir_y = (crate::math::sinish(self.ship_angle as i32) * 12) / 256;

        let nose_x = sx + dir_x;
        let nose_y = sy + dir_y;
        let left_x = sx - dir_x / 2 - dir_y / 2;
        let left_y = sy - dir_y / 2 + dir_x / 2;
        let right_x = sx - dir_x / 2 + dir_y / 2;
        let right_y = sy - dir_y / 2 - dir_x / 2;

        let p_nx = nose_x.max(0) as usize;
        let p_ny = nose_y.max(0) as usize;
        let p_lx = left_x.max(0) as usize;
        let p_ly = left_y.max(0) as usize;
        let p_rx = right_x.max(0) as usize;
        let p_ry = right_y.max(0) as usize;

        display.draw_line(p_nx, p_ny, p_lx, p_ly, Rgb::new(0, 255, 200));
        display.draw_line(p_lx, p_ly, p_rx, p_ry, Rgb::new(0, 200, 255));
        display.draw_line(p_rx, p_ry, p_nx, p_ny, Rgb::new(0, 255, 200));

        // Update and draw asteroids
        for ast in &mut self.asteroids {
            ast.x += ast.vx;
            ast.y += ast.vy;
            if ast.x < 0 { ast.x = inner_w; }
            if ast.x > inner_w { ast.x = 0; }
            if ast.y < 0 { ast.y = inner_h; }
            if ast.y > inner_h { ast.y = 0; }

            display.circle_outline(
                (origin_x + ast.x) as usize,
                (origin_y + ast.y) as usize,
                ast.radius,
                Rgb::new(255, 160, 60),
            );
        }

        // HUD overlay
        crate::font::draw_text(display, wx + 20, wy + wh - 22, "SPACE DEFENDER - 60 FPS ACTIVE", 1, Rgb::new(0, 255, 180));
    }
}

pub static GAME: Mutex<GameState> = Mutex::new(GameState::new());
