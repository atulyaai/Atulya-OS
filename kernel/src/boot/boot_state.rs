#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BootStage {
    PowerOn,
    CoreFormation,
    CoreOnline,
    MemoryOnline,
    VoiceOnline,
    VisionOnline,
    SkillsOnline,
    KnowledgeGraph,
    IdentityRestore,
    WorkspaceReady,
}
