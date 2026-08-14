use crate::display::Display;
use crate::boot::awakening::Awakening;

pub struct BootManager;

impl BootManager {
    pub fn run(display: &mut Display) {
        // "The Awakening v2" — a storyboard-faithful 12-phase cinematic boot
        // sequence with concentric rings, HUD panels, knowledge graph,
        // identity beam, and workspace transition.
        let mut boot = Awakening::new();
        boot.run(display);
    }
}
