use crate::display::{Display, Rgb};
use crate::graphics::asset_manager::{AssetManager, BootFrame};
use crate::graphics::renderer::Renderer;

pub struct SceneDef {
    pub name: &'static str,
    pub duration_ms: u32,
    pub fade_ms: u32,
    pub frame: BootFrame,
}

pub struct SceneManager {
    pub scenes: &'static [SceneDef],
    pub time_ms: u32,
    pub done: bool,
}

impl SceneManager {
    pub const fn new(scenes: &'static [SceneDef]) -> Self {
        Self {
            scenes,
            time_ms: 0,
            done: false,
        }
    }

    pub fn tick(&mut self, dt_ms: u32) {
        if self.done {
            return;
        }

        self.time_ms = self.time_ms.saturating_add(dt_ms);
        if self.scenes.is_empty() {
            self.done = true;
            return;
        }

        let last = self.scenes.len() - 1;
        let last_end = self.scene_start(last) + self.scenes[last].duration_ms;
        if self.time_ms >= last_end {
            self.done = true;
        }
    }

    fn scene_start(&self, idx: usize) -> u32 {
        if idx == 0 {
            return 0;
        }

        let mut sum = 0u32;
        let mut i = 0usize;
        while i < idx {
            sum = sum.saturating_add(self.scenes[i].duration_ms);
            i += 1;
        }

        sum.saturating_sub((idx as u32).saturating_mul(self.scenes[idx].fade_ms))
    }

    fn scene_alpha(&self, idx: usize) -> u16 {
        let scene = &self.scenes[idx];
        let start = self.scene_start(idx);
        let end = start + scene.duration_ms;
        if self.time_ms < start || self.time_ms >= end {
            return 0;
        }

        let local = self.time_ms - start;
        let fade = scene.fade_ms.max(1);
        if local < fade {
            return ((local as u64 * 256) / fade as u64) as u16;
        }

        let fade_out_start = scene.duration_ms.saturating_sub(fade);
        if local >= fade_out_start {
            let remaining = scene.duration_ms - local;
            return ((remaining as u64 * 256) / fade as u64) as u16;
        }

        256
    }

    pub fn render(&self, display: &mut Display) {
        display.clear(Rgb::new(0, 0, 0));
        for i in 0..self.scenes.len() {
            let alpha = self.scene_alpha(i);
            if alpha == 0 {
                continue;
            }
            let image = AssetManager::boot_frame(self.scenes[i].frame);
            Renderer::new(display).draw_image(image, alpha);
        }
    }
}
