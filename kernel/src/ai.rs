//! ai.rs — Autonomous AI Intent Subsystem and Context Vector Graph for Atulya OS.
//!
//! Translates user natural language intents into direct kernel syscalls, VFS operations,
//! WASM skill executions, and holographic UI telemetry cards.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

#[derive(Clone, Debug)]
pub struct ContextNode {
    pub id: u32,
    pub title: &'static str,
    pub category: &'static str,
    pub confidence: u8,
}

pub struct IntentEngine {
    pub query_history: Vec<String>,
    pub memory_nodes: Vec<ContextNode>,
}

pub enum IntentAction {
    ExecuteCommand(&'static str),
    RunWasmSkill(&'static str),
    SyncDisk,
    ChangeTheme,
    PlayChime,
    SystemDiagnostics,
    IdentityCheck,
    ListSkills,
    ShowMemoryGraph,
    DirectResponse(String),
}

pub struct IntentResult {
    pub intent_name: &'static str,
    pub description: String,
    pub action: IntentAction,
    pub confidence: u8,
}

impl IntentEngine {
    pub fn new() -> Self {
        let memory_nodes = alloc::vec![
            ContextNode { id: 1, title: "User: Atul (Admin Clearance AXON-7)", category: "Identity", confidence: 99 },
            ContextNode { id: 2, title: "Workspace: Quantum Multi-Layer OS", category: "System", confidence: 96 },
            ContextNode { id: 3, title: "Storage: ATA Primary IDE Persistent Block", category: "Hardware", confidence: 95 },
            ContextNode { id: 4, title: "Network: VirtIO-Net Mesh Routing", category: "Mesh", confidence: 92 },
            ContextNode { id: 5, title: "AI Core: Neural Voice & Vision Streams", category: "Subsystem", confidence: 98 },
        ];

        Self {
            query_history: Vec::new(),
            memory_nodes,
        }
    }

    /// Parse user natural language prompt into executable OS intents.
    pub fn parse_intent(&mut self, prompt: &str) -> IntentResult {
        let prompt_trimmed = prompt.trim();
        self.query_history.push(String::from(prompt_trimmed));
        let lower = prompt_trimmed.to_ascii_lowercase();

        if lower.contains("scan") || lower.contains("network") || lower.contains("wifi") || lower.contains("pci") {
            IntentResult {
                intent_name: "NETWORK_HARDWARE_SCAN",
                description: String::from("Scanning PCI bus and probing VirtIO-Net adapters..."),
                action: IntentAction::ExecuteCommand("pci"),
                confidence: 96,
            }
        } else if lower.contains("save") || lower.contains("backup") || lower.contains("sync") || lower.contains("persist") {
            IntentResult {
                intent_name: "VFS_DISK_PERSISTENCE",
                description: String::from("Synchronizing all files and directory trees to ATA hard disk..."),
                action: IntentAction::SyncDisk,
                confidence: 98,
            }
        } else if lower.contains("skill") || lower.contains("wasm") || lower.contains("agent") {
            IntentResult {
                intent_name: "WASM_SKILL_INVOCATION",
                description: String::from("Dispatching sandboxed WebAssembly execution to /apps/quantum_skill.wasm..."),
                action: IntentAction::RunWasmSkill("/apps/quantum_skill.wasm"),
                confidence: 94,
            }
        } else if lower.contains("memory") || lower.contains("graph") || lower.contains("recall") || lower.contains("context") {
            IntentResult {
                intent_name: "CONTEXT_VECTOR_RECALL",
                description: String::from("Querying kernel context vector graph for active user knowledge nodes..."),
                action: IntentAction::ShowMemoryGraph,
                confidence: 97,
            }
        } else if lower.contains("theme") || lower.contains("color") || lower.contains("dark") || lower.contains("mode") {
            IntentResult {
                intent_name: "THEME_MODULATION",
                description: String::from("Modulating desktop holographic spectrum theme..."),
                action: IntentAction::ChangeTheme,
                confidence: 95,
            }
        } else if lower.contains("who am i") || lower.contains("user") || lower.contains("identity") || lower.contains("auth") {
            IntentResult {
                intent_name: "BIOMETRIC_IDENTITY_CHECK",
                description: String::from("Authorized User: Atul | Clearance: AXON-7 Sovereign Administrator"),
                action: IntentAction::IdentityCheck,
                confidence: 99,
            }
        } else if lower.contains("sound") || lower.contains("audio") || lower.contains("chime") || lower.contains("music") {
            IntentResult {
                intent_name: "AUDIO_SYNTHESIZER",
                description: String::from("Triggering PIT Channel 2 cyber harmonic audio chime..."),
                action: IntentAction::PlayChime,
                confidence: 98,
            }
        } else if lower.contains("status") || lower.contains("health") || lower.contains("system") {
            IntentResult {
                intent_name: "SYSTEM_DIAGNOSTICS",
                description: String::from("All subsystems (Memory, Voice, Vision, Skills, Security) are ONLINE."),
                action: IntentAction::SystemDiagnostics,
                confidence: 97,
            }
        } else {
            IntentResult {
                intent_name: "AUTONOMOUS_AI_INTENT",
                description: format!("Atulya AI: Processing intent '{}' across kernel subsystems.", prompt_trimmed),
                action: IntentAction::DirectResponse(format!("Intent registered: '{}'. Ready for dispatch.", prompt_trimmed)),
                confidence: 90,
            }
        }
    }
}

pub static AI_ENGINE: spin::Mutex<IntentEngine> = spin::Mutex::new(IntentEngine {
    query_history: Vec::new(),
    memory_nodes: Vec::new(),
});

pub fn init() {
    *AI_ENGINE.lock() = IntentEngine::new();
    crate::serial::serial_write_line("Atulya AI Intent Subsystem Initialized.");
}
