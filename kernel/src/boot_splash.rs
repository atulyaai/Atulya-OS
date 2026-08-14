//! Thin boot splash entrypoint.
//!
//! Rendering, animation timing, assets, and transitions live in the boot,
//! scenes, graphics, and components modules. This file intentionally only
//! delegates to the boot manager.

use crate::boot::boot_manager::BootManager;
use crate::display::Display;

pub fn run(display: &mut Display) {
    BootManager::run(display);
}
