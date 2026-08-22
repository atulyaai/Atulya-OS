//! db.rs — Unified System Database & State Store for Atulya OS.
//!
//! Provides a centralized, zero-boilerplate single database for all OS subsystems:
//!   - Typed Key-Value Storage (Strings, Integers, Booleans)
//!   - System Preferences (Theme, Volume, Brightness, Wallpaper)
//!   - User Identity & Security Credentials (PIN, Biometrics, Vault Status)
//!   - AI Context & Conversation History
//!   - Persistent Disk Synchronization to `/system/config.db` on the ATA drive

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use spin::Mutex;

#[derive(Clone, Debug)]
pub struct DbEntry {
    pub key: String,
    pub value: String,
}

pub struct SystemDatabase {
    entries: Vec<DbEntry>,
}

impl SystemDatabase {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Initialize default system database values.
    pub fn init_defaults(&mut self) {
        self.set("user.name", "Atul");
        self.set("user.clearance", "AXON-7 Sovereign Administrator");
        self.set("user.avatar", "/media/avatar.png");
        self.set("user.pin", "atulya");
        
        self.set_int("system.theme", 0); // 0 = Obsidian, 1 = Matrix, 2 = Neon Cyber, 3 = Solar Aurora
        self.set_int("system.volume", 85);
        self.set_int("system.brightness", 100);
        self.set_bool("system.audio_enabled", true);

        self.set("ai.model", "Qwen-2.5-0.5B-GGUF");
        self.set_bool("ai.voice_tts", true);
        self.set("ai.persona", "JARVIS Sovereign Assistant");

        self.set("net.ip", "10.0.2.15");
        self.set("net.gateway", "10.0.2.2");
        self.set("net.status", "100 Gbps Low-Loss Mesh Online");

        self.set_bool("security.vault_armed", true);
        self.set("security.cipher", "ChaCha20-256");
        self.set_int("security.threats_blocked", 0);
    }

    /// Get string value for a key.
    pub fn get(&self, key: &str) -> Option<String> {
        self.entries.iter().find(|e| e.key == key).map(|e| e.value.clone())
    }

    /// Set string value for a key.
    pub fn set(&mut self, key: &str, value: &str) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.key == key) {
            entry.value = String::from(value);
        } else {
            self.entries.push(DbEntry {
                key: String::from(key),
                value: String::from(value),
            });
        }
    }

    /// Get integer value for a key.
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.parse::<i64>().ok())
    }

    /// Set integer value for a key.
    pub fn set_int(&mut self, key: &str, value: i64) {
        self.set(key, &format!("{}", value));
    }

    /// Get boolean value for a key.
    pub fn get_bool(&self, key: &str) -> bool {
        self.get(key).map(|v| v == "true" || v == "1").unwrap_or(false)
    }

    /// Set boolean value for a key.
    pub fn set_bool(&mut self, key: &str, value: bool) {
        self.set(key, if value { "true" } else { "false" });
    }

    /// Enumerate all keys starting with a prefix (e.g. "user.", "system.").
    pub fn list_prefix(&self, prefix: &str) -> Vec<DbEntry> {
        self.entries.iter().filter(|e| e.key.starts_with(prefix)).cloned().collect()
    }

    /// Serialize database to raw text for ATA disk persistence.
    pub fn serialize(&self) -> String {
        let mut out = String::new();
        for entry in &self.entries {
            out.push_str(&entry.key);
            out.push('=');
            out.push_str(&entry.value);
            out.push('\n');
        }
        out
    }

    /// Deserialize database from raw text loaded from ATA disk.
    pub fn deserialize(&mut self, data: &str) {
        for line in data.lines() {
            let trimmed = line.trim();
            if let Some(idx) = trimmed.find('=') {
                let key = &trimmed[..idx];
                let val = &trimmed[idx + 1..];
                self.set(key, val);
            }
        }
    }
}

pub static DB: Mutex<SystemDatabase> = Mutex::new(SystemDatabase::new());

pub fn init() {
    let mut db = DB.lock();
    db.init_defaults();
    crate::serial::serial_write_line("Unified System Database (DB) initialized.");
}
