use crate::display::Display;
use crate::gpu_boot::GpuBoot;

pub struct BootManager;

impl BootManager {
    pub fn run(display: &mut Display) {
        // Generative GPU-style boot: a living, entropy-seeded animation
        // rendered procedurally per-pixel. No assets required.
        let mut boot = GpuBoot::new();
        boot.run(display);
    }
}
