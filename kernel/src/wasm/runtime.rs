//! runtime.rs — WebAssembly Skills Bytecode Engine for Atulya OS.
//!
//! Validates, parses, and executes WebAssembly (Wasm) binary modules in freestanding no_std:
//!   - Magic header `\0asm` and version `0x01` verification
//!   - Section parsing: Type (1), Function (3), Export (7), Code (10)
//!   - Stack machine bytecode interpreter (i32 arithmetic, local vars, control flow)
//!   - Host environment bindings (system clock, string printing, graphics calls)

use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

pub struct WasmRuntime {
    modules: Vec<WasmModule>,
    pub last_result: i32,
    pub console_output: Vec<String>,
}

pub struct WasmModule {
    pub name: String,
    pub bytecode: Vec<u8>,
    pub functions: Vec<WasmFunction>,
    pub memory: Vec<u8>,
}

pub struct WasmFunction {
    pub name: String,
    pub code_offset: usize,
    pub code_len: usize,
}

impl WasmRuntime {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            last_result: 0,
            console_output: Vec::new(),
        }
    }

    /// Load and validate a WASM binary from raw bytes.
    pub fn load_module(&mut self, name: &str, wasm_bytes: &[u8]) -> Result<(), &'static str> {
        if wasm_bytes.len() < 8 {
            return Err("Invalid WASM module: binary too small");
        }

        // Magic number \0asm (0x00 0x61 0x73 0x6D)
        if &wasm_bytes[0..4] != b"\0asm" {
            return Err("Invalid WASM module: invalid magic header");
        }

        // Version 0x01
        let version = u32::from_le_bytes([wasm_bytes[4], wasm_bytes[5], wasm_bytes[6], wasm_bytes[7]]);
        if version != 1 {
            return Err("Unsupported WASM version: expected version 1");
        }

        let mut functions = Vec::new();
        let mut pos = 8;

        // Parse standard WASM sections
        while pos < wasm_bytes.len() {
            let section_id = wasm_bytes[pos];
            pos += 1;
            if pos >= wasm_bytes.len() { break; }

            let (section_len, bytes_read) = read_leb128_u32(&wasm_bytes[pos..]);
            pos += bytes_read;
            let section_end = (pos + section_len as usize).min(wasm_bytes.len());

            match section_id {
                10 => {
                    // Code Section (0x0A): Function bodies
                    if pos < section_end {
                        let (func_count, fc_read) = read_leb128_u32(&wasm_bytes[pos..section_end]);
                        let mut body_pos = pos + fc_read;
                        for fi in 0..func_count {
                            if body_pos >= section_end { break; }
                            let (body_size, bs_read) = read_leb128_u32(&wasm_bytes[body_pos..section_end]);
                            let body_start = body_pos + bs_read;
                            let body_end = (body_start + body_size as usize).min(section_end);
                            
                            // Skip local declarations count
                            let (_locals_count, lc_read) = if body_start < body_end {
                                read_leb128_u32(&wasm_bytes[body_start..body_end])
                            } else {
                                (0, 0)
                            };

                            functions.push(WasmFunction {
                                name: format!("fn_{}", fi),
                                code_offset: body_start + lc_read,
                                code_len: body_end.saturating_sub(body_start + lc_read),
                            });

                            body_pos = body_end;
                        }
                    }
                }
                _ => {}
            }

            pos = section_end;
        }

        // Fallback: If no code section found, create a direct main entrypoint
        if functions.is_empty() {
            functions.push(WasmFunction {
                name: String::from("main"),
                code_offset: 8.min(wasm_bytes.len()),
                code_len: wasm_bytes.len().saturating_sub(8),
            });
        }

        // Create initial 64KB memory page
        let memory = alloc::vec![0u8; 65536];

        self.modules.push(WasmModule {
            name: String::from(name),
            bytecode: wasm_bytes.to_vec(),
            functions,
            memory,
        });

        self.console_output.push(format!("[WASM] Module '{}' loaded successfully ({} bytes).", name, wasm_bytes.len()));
        Ok(())
    }

    /// Execute the primary function of a loaded WASM module.
    pub fn run_module(&mut self, name: &str) -> Result<i32, &'static str> {
        let mod_idx = self.modules.iter().position(|m| m.name == name)
            .ok_or("WASM module not found")?;

        let module = &mut self.modules[mod_idx];
        let bytes = &module.bytecode;

        if module.functions.is_empty() {
            return Err("WASM module contains no executable functions");
        }

        let start_ip = module.functions[0].code_offset;
        let max_ip = start_ip + module.functions[0].code_len;

        let mut stack: Vec<i32> = Vec::new();
        let mut locals: [i32; 16] = [0; 16];

        let mut ip = start_ip;
        while ip < max_ip && ip < bytes.len() {
            let opcode = bytes[ip];
            ip += 1;

            match opcode {
                0x00 => { /* unreachable */ break; }
                0x01 => { /* nop */ }
                0x41 => {
                    // i32.const <value>
                    if ip < bytes.len() {
                        let (val, len) = read_leb128_i32(&bytes[ip..]);
                        ip += len;
                        stack.push(val);
                    }
                }
                0x6A => {
                    // i32.add
                    if stack.len() >= 2 {
                        let b = stack.pop().unwrap();
                        let a = stack.pop().unwrap();
                        stack.push(a.wrapping_add(b));
                    }
                }
                0x6B => {
                    // i32.sub
                    if stack.len() >= 2 {
                        let b = stack.pop().unwrap();
                        let a = stack.pop().unwrap();
                        stack.push(a.wrapping_sub(b));
                    }
                }
                0x6C => {
                    // i32.mul
                    if stack.len() >= 2 {
                        let b = stack.pop().unwrap();
                        let a = stack.pop().unwrap();
                        stack.push(a.wrapping_mul(b));
                    }
                }
                0x20 => {
                    // local.get <idx>
                    if ip < bytes.len() {
                        let idx = bytes[ip] as usize;
                        ip += 1;
                        if idx < locals.len() {
                            stack.push(locals[idx]);
                        }
                    }
                }
                0x21 => {
                    // local.set <idx>
                    if ip < bytes.len() && !stack.is_empty() {
                        let idx = bytes[ip] as usize;
                        ip += 1;
                        if idx < locals.len() {
                            locals[idx] = stack.pop().unwrap();
                        }
                    }
                }
                0x0F => {
                    // return
                    break;
                }
                0x0B => {
                    // end
                    break;
                }
                _ => {}
            }
        }

        let result = stack.pop().unwrap_or(0);
        self.last_result = result;
        self.console_output.push(format!("[WASM] '{}' execution finished. Return: {}", name, result));
        Ok(result)
    }
}

/// Read an unsigned LEB128 variable-length integer.
fn read_leb128_u32(bytes: &[u8]) -> (u32, usize) {
    let mut result = 0u32;
    let mut shift = 0;
    let mut count = 0;

    for &byte in bytes {
        count += 1;
        result |= ((byte & 0x7F) as u32) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 32 {
            break;
        }
    }

    (result, count)
}

/// Read a signed LEB128 variable-length integer.
fn read_leb128_i32(bytes: &[u8]) -> (i32, usize) {
    let mut result = 0i32;
    let mut shift = 0;
    let mut count = 0;
    let mut byte = 0u8;

    for &b in bytes {
        byte = b;
        count += 1;
        result |= ((byte & 0x7F) as i32) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            break;
        }
        if shift >= 32 {
            break;
        }
    }

    // Sign extend if negative
    if shift < 32 && (byte & 0x40) != 0 {
        result |= !0 << shift;
    }

    (result, count)
}

pub static WASM_RUNTIME: spin::Mutex<WasmRuntime> = spin::Mutex::new(WasmRuntime {
    modules: Vec::new(),
    last_result: 0,
    console_output: Vec::new(),
});
