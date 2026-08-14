use alloc::string::String;
use alloc::vec::Vec;

pub struct WasmRuntime {
    modules: Vec<WasmModule>,
    last_result: i32,
}

struct WasmModule {
    name: String,
    data: Vec<u8>,
}

impl WasmRuntime {
    pub fn new() -> Self {
        Self { modules: Vec::new(), last_result: 0 }
    }

    pub fn load_module(&mut self, name: &str, wasm_bytes: &[u8]) -> Result<(), &'static str> {
        if wasm_bytes.len() < 8 {
            return Err("Invalid WASM module: too small");
        }
        if &wasm_bytes[0..4] != b"\0asm" {
            return Err("Invalid WASM module: bad magic number");
        }
        self.modules.push(WasmModule {
            name: String::from(name),
            data: wasm_bytes.to_vec(),
        });
        Ok(())
    }

    pub fn list_modules(&self) -> Vec<&str> {
        self.modules.iter().map(|m| m.name.as_str()).collect()
    }

    pub fn has_module(&self, name: &str) -> bool {
        self.modules.iter().any(|m| m.name == name)
    }

    pub fn run_module(&mut self, _name: &str) -> Result<i32, &'static str> {
        Err("WASM execution pending wasmi integration")
    }

    pub fn last_result(&self) -> i32 {
        self.last_result
    }
}
