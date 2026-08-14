pub struct Transition {
    pub fade_ms: u32,
    pub pulse_alpha: u16,
}

impl Transition {
    pub const fn cinematic() -> Self {
        Self {
            fade_ms: 300,
            pulse_alpha: 96,
        }
    }
}
