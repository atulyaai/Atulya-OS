/// All phases of the Atulya OS boot sequence, matching the storyboard.
///
/// Each variant maps to a percentage band and a visual stage of
/// "The Awakening" boot animation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BootPhase {
    /// 0-4 %   — pitch-black, single pulsating blue-white singularity.
    Void,
    /// 4-12 %  — singularity ignites into fluid plasma cloud.
    Genesis,
    /// 12-25 % — dotted concentric orbital rings emerge around the core.
    CoreFormation,
    /// 25-40 % — "ATULYA OS" text appears; multiple glowing concentric rings.
    CoreOnline,
    /// 40-55 % — top-left HUD panel: MEMORY (magenta, equalizer bars).
    MemoryOnline,
    /// 55-65 % — top-right HUD panel: VOICE (cyan, audio waveform).
    VoiceOnline,
    /// 65-75 % — right HUD panel: VISION (cyan, eye icon).
    VisionOnline,
    /// 75-85 % — bottom-right HUD panel: SKILLS (amber, atomic orbital).
    SkillsOnline,
    /// 85-92 % — all 4 subsystem panels illuminated, core at full bloom.
    AllOnline,
    /// 92-96 % — neural constellation graph expands from core.
    KnowledgeGraph,
    /// 96-98 % — vertical cyan beam, "HELLO ATUL".
    IdentityRestored,
    /// 98-100 % — light flare, transition to desktop.
    WorkspaceReady,
}

impl BootPhase {
    /// Map a progress value (0.0 .. 1.0) to the corresponding boot phase.
    pub fn from_progress(p: f32) -> Self {
        match () {
            _ if p < 0.04 => Self::Void,
            _ if p < 0.12 => Self::Genesis,
            _ if p < 0.25 => Self::CoreFormation,
            _ if p < 0.40 => Self::CoreOnline,
            _ if p < 0.55 => Self::MemoryOnline,
            _ if p < 0.65 => Self::VoiceOnline,
            _ if p < 0.75 => Self::VisionOnline,
            _ if p < 0.85 => Self::SkillsOnline,
            _ if p < 0.92 => Self::AllOnline,
            _ if p < 0.96 => Self::KnowledgeGraph,
            _ if p < 0.98 => Self::IdentityRestored,
            _ => Self::WorkspaceReady,
        }
    }

    /// Status text shown at the bottom of the screen during this phase.
    pub fn status_text(&self) -> &'static str {
        match self {
            Self::Void => "SYSTEM POWER ON",
            Self::Genesis => "ENERGY INITIALIZING",
            Self::CoreFormation => "CORE FORMATION",
            Self::CoreOnline => "ATULYA CORE ONLINE",
            Self::MemoryOnline => "MEMORY ONLINE",
            Self::VoiceOnline => "VOICE ONLINE",
            Self::VisionOnline => "VISION ONLINE",
            Self::SkillsOnline => "SKILLS ONLINE",
            Self::AllOnline => "ALL SYSTEMS ONLINE",
            Self::KnowledgeGraph => "KNOWLEDGE GRAPH ACTIVATED",
            Self::IdentityRestored => "IDENTITY RESTORED",
            Self::WorkspaceReady => "ATULYA IS READY",
        }
    }
}
