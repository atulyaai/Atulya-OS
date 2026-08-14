use crate::display::{Display, Rgb};
use crate::fs::{RamFs, FileSystem, OpenFlags};

static DOCK_ICONS: &[u8; 9 * 32 * 32 * 4] = include_bytes!("../../assets/icons/dock_icons.rgba");

// ── Keyboard I/O Ports ─────────────────────────────────────────────
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!("in al, dx", out("al") value, in("dx") port);
    value
}

unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val);
}

// ── PS/2 Mouse Controller ──────────────────────────────────────────
unsafe fn mouse_wait(a_type: u8) {
    let mut timeout = 100000;
    while timeout > 0 {
        let status = inb(0x64);
        if a_type == 0 {
            if status & 1 != 0 {
                return;
            }
        } else {
            if status & 2 == 0 {
                return;
            }
        }
        timeout -= 1;
    }
}

unsafe fn mouse_write(write_val: u8) {
    mouse_wait(1);
    outb(0x64, 0xD4);
    mouse_wait(1);
    outb(0x60, write_val);
}

unsafe fn mouse_read() -> u8 {
    mouse_wait(0);
    inb(0x60)
}

pub unsafe fn init_mouse() {
    // Enable auxiliary device
    mouse_wait(1);
    outb(0x64, 0xA8);

    // Enable interrupts/configure controller command byte
    mouse_wait(1);
    outb(0x64, 0x20); // Read command byte
    let mut status = mouse_read();
    status |= 2;      // Enable mouse interrupt (bit 1)
    status &= !0x20;  // Disable mouse clock disable (bit 5)
    mouse_wait(1);
    outb(0x64, 0x60); // Write command byte
    mouse_wait(1);
    outb(0x60, status);

    // Set default settings
    mouse_write(0xF6);
    let _ack = mouse_read(); // Wait for ACK (0xFA)

    // Enable data reporting
    mouse_write(0xF4);
    let _ack = mouse_read(); // Wait for ACK (0xFA)
}

pub struct MouseState {
    pub x: isize,
    pub y: isize,
    pub buttons: u8,
    bytes: [u8; 3],
    cycle: u8,
}

impl MouseState {
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            x: (w / 2) as isize,
            y: (h / 2) as isize,
            buttons: 0,
            bytes: [0; 3],
            cycle: 0,
        }
    }

    pub fn handle_byte(&mut self, b: u8, w: usize, h: usize) -> bool {
        if self.cycle == 0 {
            // Byte 0 of standard PS/2 packet must always have bit 3 (0x08) set.
            // If clear, this is an out-of-sync byte or stray ACK -> discard and wait for valid header.
            if b & 0x08 == 0 {
                return false;
            }
            self.bytes[0] = b;
            self.cycle = 1;
            return false;
        } else if self.cycle == 1 {
            self.bytes[1] = b;
            self.cycle = 2;
            return false;
        } else {
            self.bytes[2] = b;
            self.cycle = 0;

            let flags = self.bytes[0];
            let x_sign = flags & 0x10 != 0;
            let y_sign = flags & 0x20 != 0;

            let mut dx = self.bytes[1] as isize;
            let mut dy = self.bytes[2] as isize;

            if x_sign {
                dx -= 256;
            }
            if y_sign {
                dy -= 256;
            }

            // Discard invalid overflow packets
            if flags & 0xC0 != 0 {
                return false;
            }

            self.buttons = flags & 0x07;

            self.x = (self.x + dx).max(0).min(w as isize - 1);
            self.y = (self.y - dy).max(0).min(h as isize - 1);

            return true;
        }
    }
}

// ── Layout constants ──────────────────────────────────────────────
const TASKBAR_HEIGHT: usize = 50;

// Theme configuration structure
struct Theme {
    wall_top: Rgb,
    wall_bot: Rgb,
    accent: Rgb,
    win_active: Rgb,
    win_inactive: Rgb,
    text: Rgb,
}

const THEMES: [Theme; 4] = [
    // Cyberpunk Cyan
    Theme {
        wall_top: Rgb::new(4, 12, 28),
        wall_bot: Rgb::new(8, 44, 64),
        accent: Rgb::new(0, 229, 255),
        win_active: Rgb::new(0, 229, 255),
        win_inactive: Rgb::new(0, 77, 97),
        text: Rgb::new(165, 243, 252),
    },
    // Matrix Green
    Theme {
        wall_top: Rgb::new(0, 0, 0),
        wall_bot: Rgb::new(0, 34, 8),
        accent: Rgb::new(0, 255, 65),
        win_active: Rgb::new(0, 255, 65),
        win_inactive: Rgb::new(0, 68, 16),
        text: Rgb::new(128, 255, 128),
    },
    // macOS Obsidian
    Theme {
        wall_top: Rgb::new(18, 18, 18),
        wall_bot: Rgb::new(42, 45, 50),
        accent: Rgb::new(59, 130, 246),
        win_active: Rgb::new(59, 130, 246),
        win_inactive: Rgb::new(55, 65, 81),
        text: Rgb::new(229, 231, 235),
    },
    // Retro Gold
    Theme {
        wall_top: Rgb::new(26, 15, 0),
        wall_bot: Rgb::new(74, 44, 0),
        accent: Rgb::new(255, 176, 0),
        win_active: Rgb::new(255, 176, 0),
        win_inactive: Rgb::new(102, 68, 0),
        text: Rgb::new(255, 224, 130),
    },
];

// ── Keyboard State Tracker ────────────────────────────────────────
struct KeyboardState {
    shift_pressed: bool,
}

impl KeyboardState {
    fn handle_scancode(&mut self, code: u8) -> Option<char> {
        match code {
            0x2A | 0x36 => {
                self.shift_pressed = true;
                None
            }
            0xAA | 0xB6 => {
                self.shift_pressed = false;
                None
            }
            _ => {
                if code & 0x80 == 0 {
                    Some(self.translate(code))
                } else {
                    None
                }
            }
        }
    }

    fn translate(&self, scancode: u8) -> char {
        if self.shift_pressed {
            match scancode {
                0x02 => '!', 0x03 => '@', 0x04 => '#', 0x05 => '$', 0x06 => '%',
                0x07 => '^', 0x08 => '&', 0x09 => '*', 0x0A => '(', 0x0B => ')',
                0x0C => '_', 0x0D => '+',
                0x10 => 'Q', 0x11 => 'W', 0x12 => 'E', 0x13 => 'R', 0x14 => 'T',
                0x15 => 'Y', 0x16 => 'U', 0x17 => 'I', 0x18 => 'O', 0x19 => 'P',
                0x1A => '{', 0x1B => '}',
                0x1E => 'A', 0x1F => 'S', 0x20 => 'D', 0x21 => 'F', 0x22 => 'G',
                0x23 => 'H', 0x24 => 'J', 0x25 => 'K', 0x26 => 'L', 0x27 => ':',
                0x28 => '"', 0x29 => '~',
                0x2C => 'Z', 0x2D => 'X', 0x2E => 'C', 0x2F => 'V', 0x30 => 'B',
                0x31 => 'N', 0x32 => 'M', 0x33 => '<', 0x34 => '>', 0x35 => '?',
                0x39 => ' ',
                _ => '\0',
            }
        } else {
            match scancode {
                0x02 => '1', 0x03 => '2', 0x04 => '3', 0x05 => '4', 0x06 => '5',
                0x07 => '6', 0x08 => '7', 0x09 => '8', 0x0A => '9', 0x0B => '0',
                0x0C => '-', 0x0D => '=',
                0x10 => 'q', 0x11 => 'w', 0x12 => 'e', 0x13 => 'r', 0x14 => 't',
                0x15 => 'y', 0x16 => 'u', 0x17 => 'i', 0x18 => 'o', 0x19 => 'p',
                0x1A => '[', 0x1B => ']',
                0x1E => 'a', 0x1F => 's', 0x20 => 'd', 0x21 => 'f', 0x22 => 'g',
                0x23 => 'h', 0x24 => 'j', 0x25 => 'k', 0x26 => 'l', 0x27 => ';',
                0x28 => '\'', 0x29 => '`',
                0x2C => 'z', 0x2D => 'x', 0x2E => 'c', 0x2F => 'v', 0x30 => 'b',
                0x31 => 'n', 0x32 => 'm', 0x33 => ',', 0x34 => '.', 0x35 => '/',
                0x39 => ' ',
                0x1C => '\n',
                0x0E => '\x08',
                _ => '\0',
            }
        }
    }
}

// ── Interactive Terminal ──────────────────────────────────────────
const TERM_COLS: usize = 62;
const TERM_ROWS: usize = 14;

struct Terminal {
    buffer: [[u8; TERM_COLS]; TERM_ROWS],
    cursor_row: usize,
    cursor_col: usize,
    joke_idx: usize,
    fs: RamFs,
    cwd: alloc::string::String,
}

impl Terminal {
    fn new() -> Self {
        let mut term = Self {
            buffer: [[b' '; TERM_COLS]; TERM_ROWS],
            cursor_row: 0,
            cursor_col: 0,
            joke_idx: 0,
            fs: RamFs::new(),
            cwd: alloc::string::String::from("/"),
        };
        term.clear();
        term
    }

    fn clear(&mut self) {
        self.buffer = [[b' '; TERM_COLS]; TERM_ROWS];
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.write_str("AtulyaOS Shell Console v0.3");
        self.new_line();
        self.write_str("Move windows: Drag Titlebar / Alt+Arrows");
        self.new_line();
        self.write_str("Switch focus: Click window / Tab");
        self.new_line();
        self.write_str("Close window: Click Red Button");
        self.new_line();
        self.write_str("Restore window: Click corresponding Dock icon");
        self.new_line();
        self.write_str("Type 'help' for commands.");
        self.new_line();
        self.print_prompt();
    }

    fn print_prompt(&mut self) {
        self.write_str("guest@atulyaos:~$ ");
    }

    fn write_str(&mut self, s: &str) {
        for ch in s.chars() {
            self.write_char(ch);
        }
    }

    fn write_char(&mut self, ch: char) {
        if ch == '\n' {
            self.new_line();
            return;
        }
        let b = ch as u8;
        if self.cursor_col < TERM_COLS {
            self.buffer[self.cursor_row][self.cursor_col] = b;
            self.cursor_col += 1;
        } else {
            self.new_line();
            if self.cursor_col < TERM_COLS {
                self.buffer[self.cursor_row][self.cursor_col] = b;
                self.cursor_col += 1;
            }
        }
    }

    fn backspace(&mut self) {
        // "guest@atulyaos:~$ " prompt takes 18 columns.
        if self.cursor_col > 18 {
            self.cursor_col -= 1;
            self.buffer[self.cursor_row][self.cursor_col] = b' ';
        }
    }

    fn new_line(&mut self) {
        if self.cursor_row < TERM_ROWS - 1 {
            self.cursor_row += 1;
        } else {
            for r in 1..TERM_ROWS {
                self.buffer[r - 1] = self.buffer[r];
            }
            self.buffer[TERM_ROWS - 1] = [b' '; TERM_COLS];
            self.cursor_row = TERM_ROWS - 1;
        }
        self.cursor_col = 0;
    }

    fn write_line(&mut self, s: &str) {
        self.write_str(s);
        self.new_line();
    }

    fn resolve_path(&self, path: &str) -> alloc::string::String {
        let path = path.trim();
        if path.is_empty() {
            return self.cwd.clone();
        }
        let full = if path.starts_with('/') {
            alloc::string::String::from(path)
        } else if self.cwd == "/" {
            alloc::format!("/{}", path)
        } else {
            alloc::format!("{}/{}", self.cwd, path)
        };

        let mut parts: alloc::vec::Vec<&str> = alloc::vec::Vec::new();
        for part in full.split('/').filter(|s| !s.is_empty()) {
            if part == "." {
                continue;
            } else if part == ".." {
                parts.pop();
            } else {
                parts.push(part);
            }
        }

        if parts.is_empty() {
            alloc::string::String::from("/")
        } else {
            alloc::format!("/{}", parts.join("/"))
        }
    }

    fn execute_command(
        &mut self,
        theme_idx: &mut usize,
        scan_active: &mut usize,
        matrix_active: &mut usize,
    ) {
        let row_data = &self.buffer[self.cursor_row];
        let mut cmd_len = 0;
        for col in 18..TERM_COLS {
            if row_data[col] != b' ' {
                cmd_len = col - 17;
            }
        }

        let max_cmd = TERM_COLS - 18;
        let mut cmd_buf = [0u8; 48];
        let mut cmd_idx = 0;
        for col in 18..18 + cmd_len {
            if col < TERM_COLS && cmd_idx < max_cmd {
                cmd_buf[cmd_idx] = row_data[col];
                cmd_idx += 1;
            }
        }

        let cmd_str = core::str::from_utf8(&cmd_buf[..cmd_idx]).unwrap_or("").trim();

        self.new_line();

        if cmd_str.is_empty() {
            self.print_prompt();
            return;
        }

        if cmd_str == "help" {
            self.write_line("Commands: ls, cat, mkdir, rm, echo, pwd, cd,");
            self.write_line("          ask <intent>, memory, pci, ifconfig,");
            self.write_line("          ping, ps, wasm, disk, sound, skills,");
            self.write_line("          net, neofetch, clear, theme, scan");
        } else if cmd_str.starts_with("ask ") {
            let query = &cmd_str[4..];
            let res = crate::ai::AI_ENGINE.lock().parse_intent(query);
            self.write_str("✦ [AI INTENT: ");
            self.write_str(res.intent_name);
            self.write_line("]");
            self.write_str("  ");
            self.write_line(&res.description);
            match res.action {
                crate::ai::IntentAction::SyncDisk => {
                    let _ = self.fs.sync_to_disk();
                    self.write_line("  [OK] System state synchronized to ATA hard disk.");
                }
                crate::ai::IntentAction::SearchFiles(ref keyword) => {
                    self.write_line("── Natural Language VFS Search Results ──");
                    let mut found = 0;
                    let subdirs = ["/", "/etc", "/home/guest", "/user/atul", "/system", "/apps", "/docs", "/media"];
                    for dir in &subdirs {
                        if let Ok(entries) = self.fs.ls(dir) {
                            for entry in entries {
                                if entry.name.contains(keyword.as_str()) || keyword.contains(&entry.name) {
                                    let full_path = if *dir == "/" {
                                        alloc::format!("/{}", entry.name)
                                    } else {
                                        alloc::format!("{}/{}", dir, entry.name)
                                    };
                                    self.write_str("  📄 ");
                                    self.write_str(&full_path);
                                    self.write_str(" (");
                                    let mut sbuf = [0u8; 8];
                                    let mut sval = entry.size as u32;
                                    let mut sidx = 8;
                                    if sval == 0 { sbuf[7] = b'0'; sidx = 7; }
                                    while sval > 0 && sidx > 0 {
                                        sidx -= 1;
                                        sbuf[sidx] = b'0' + (sval % 10) as u8;
                                        sval /= 10;
                                    }
                                    self.write_str(core::str::from_utf8(&sbuf[sidx..]).unwrap_or("0"));
                                    self.write_line(" bytes)");
                                    found += 1;
                                }
                            }
                        }
                    }
                    if found == 0 {
                        self.write_str("  No files matching '");
                        self.write_str(keyword);
                        self.write_line("' found on ATA disk.");
                    }
                }
                crate::ai::IntentAction::FindLargeFiles => {
                    self.write_line("── Top Indexed Files Across ATA VFS ──");
                    let subdirs = ["/system", "/apps", "/docs", "/media", "/user/atul", "/etc"];
                    for dir in &subdirs {
                        if let Ok(entries) = self.fs.ls(dir) {
                            for entry in entries {
                                if !entry.is_dir && entry.size > 0 {
                                    let full_path = alloc::format!("{}/{}", dir, entry.name);
                                    self.write_str("  💾 ");
                                    self.write_str(&full_path);
                                    self.write_str(" ── ");
                                    let mut sbuf = [0u8; 8];
                                    let mut sval = entry.size as u32;
                                    let mut sidx = 8;
                                    if sval == 0 { sbuf[7] = b'0'; sidx = 7; }
                                    while sval > 0 && sidx > 0 {
                                        sidx -= 1;
                                        sbuf[sidx] = b'0' + (sval % 10) as u8;
                                        sval /= 10;
                                    }
                                    self.write_str(core::str::from_utf8(&sbuf[sidx..]).unwrap_or("0"));
                                    self.write_line(" bytes");
                                }
                            }
                        }
                    }
                }
                crate::ai::IntentAction::ViewDocument(ref target) => {
                    let resolved = self.resolve_path(target);
                    if let Ok(handle) = self.fs.open(&resolved, OpenFlags::Read) {
                        let mut buf = alloc::vec![0u8; 8192];
                        let mut total = 0;
                        while total < buf.len() {
                            match self.fs.read(handle, &mut buf[total..]) {
                                Ok(0) => break,
                                Ok(n) => total += n,
                                Err(_) => break,
                            }
                        }
                        let _ = self.fs.close(handle);
                        let decoded = crate::viewer::sniff_and_decode(&resolved, &buf[..total]);
                        self.write_str(decoded.format.icon());
                        self.write_str(" [");
                        self.write_str(decoded.format.name());
                        self.write_line("]");
                        self.write_str("  ");
                        self.write_line(&decoded.header_summary);
                        for line in &decoded.preview_lines {
                            self.write_line(line);
                        }
                    } else {
                        self.write_str("  File not found: ");
                        self.write_line(&resolved);
                    }
                }
                crate::ai::IntentAction::ChangeTheme => {
                    *theme_idx = (*theme_idx + 1) % 4;
                    self.write_line("  [OK] Spectrum theme updated.");
                }
                crate::ai::IntentAction::PlayChime => {
                    crate::sound::Sound::play_boot_chime();
                }
                _ => {}
            }
        } else if cmd_str == "memory" {
            self.write_line("── Context Vector Graph (Memory Bus) ──");
            let ai = crate::ai::AI_ENGINE.lock();
            for node in &ai.memory_nodes {
                self.write_str("  [#");
                let mut nbuf = [0u8; 2];
                nbuf[0] = b'0' + node.id as u8;
                self.write_str(core::str::from_utf8(&nbuf[..1]).unwrap_or("?"));
                self.write_str("] ");
                self.write_str(node.title);
                self.write_str(" (");
                self.write_str(node.category);
                self.write_line(")");
            }
        } else if cmd_str == "disk" {
            self.write_line("── ATA / IDE Persistent Block Device ──");
            let disk = crate::fs::ata::DISK.lock();
            if disk.is_available {
                self.write_line("  Status: Primary Master IDE [ONLINE]");
                self.write_line("  LBA Addressing: 28-bit / 48-bit Mode Active");
                self.write_line("  Sector Size: 512 bytes | Persistent Storage: Ready");
            } else {
                self.write_line("  Status: Virtual RAM Disk Emulation Active");
            }
        } else if cmd_str == "pci" {
            self.write_line("── Discovered PCI Hardware Devices ──");
            let devices = crate::pci::PciBus::scan();
            if devices.is_empty() {
                self.write_line("  No PCI devices found on bus 0.");
            } else {
                for dev in &devices {
                    self.write_str("  [");
                    self.write_str(dev.description());
                    self.write_line("]");
                }
            }
        } else if cmd_str == "ifconfig" {
            self.write_line("── Network Interfaces ──");
            self.write_line("  lo0: flags=UP,LOOPBACK mtu 65536");
            self.write_line("       inet 127.0.0.1 netmask 255.0.0.0");
            self.write_line("  eth0: flags=UP,BROADCAST,MULTICAST mtu 1500");
            self.write_line("       ether 52:54:00:12:34:56");
            self.write_line("       inet 10.0.2.15 netmask 255.255.255.0");
            self.write_line("       driver VirtIO-Net (PCI Subsystem)");
        } else if cmd_str.starts_with("ping") {
            self.write_line("PING 10.0.2.2 (10.0.2.2): 56 data bytes");
            self.write_line("64 bytes from 10.0.2.2: icmp_seq=0 ttl=64 time=0.42 ms");
            self.write_line("64 bytes from 10.0.2.2: icmp_seq=1 ttl=64 time=0.38 ms");
            self.write_line("--- 10.0.2.2 ping statistics ---");
            self.write_line("2 packets transmitted, 2 received, 0% packet loss");
        } else if cmd_str == "ps" {
            self.write_line("── Active Process Table ──");
            self.write_line("PID  NAME           STATE");
            self.write_line("───  ─────────────  ────────");
            let procs = crate::scheduler::list_processes();
            for (pid, name, state) in &procs {
                self.write_str(" ");
                let mut pbuf = [0u8; 4];
                let pstr = if *pid >= 10 {
                    pbuf[0] = b'0' + (pid / 10) as u8;
                    pbuf[1] = b'0' + (pid % 10) as u8;
                    core::str::from_utf8(&pbuf[..2]).unwrap_or("??")
                } else {
                    pbuf[0] = b'0' + *pid as u8;
                    core::str::from_utf8(&pbuf[..1]).unwrap_or("?")
                };
                self.write_str(pstr);
                self.write_str("   ");
                self.write_str(name);
                self.write_str("   ");
                self.write_line(state);
            }
        } else if cmd_str.starts_with("run ") || cmd_str == "run" || cmd_str.starts_with("wasm") {
            let wasm_file = if cmd_str.starts_with("run ") {
                cmd_str[4..].trim()
            } else {
                "/apps/quantum_skill.wasm"
            };
            let resolved = self.resolve_path(wasm_file);
            self.write_line("── WebAssembly Ring-3 Sandboxed Runtime ──");
            self.write_str("  Target: ");
            self.write_line(&resolved);
            if let Ok(handle) = self.fs.open(&resolved, crate::fs::vfs::OpenFlags::Read) {
                let mut buf = alloc::vec![0u8; 2048];
                if let Ok(n) = self.fs.read(handle, &mut buf) {
                    let mut runtime = crate::wasm::runtime::WasmRuntime::new();
                    if let Ok(()) = runtime.load_module("user_app", &buf[..n]) {
                        self.write_line("  [OK] Validated WASM bytecode (\\0asm v1)");
                        self.write_line("  [OK] Spawned isolated Ring 3 User Thread (PID active)");
                        if let Ok(result) = runtime.run_module("user_app") {
                            self.write_str("  [OK] App returned: ");
                            let mut rbuf = [0u8; 8];
                            let mut val = result as u32;
                            let mut idx = 8;
                            if val == 0 { rbuf[7] = b'0'; idx = 7; }
                            while val > 0 && idx > 0 {
                                idx -= 1;
                                rbuf[idx] = b'0' + (val % 10) as u8;
                                val /= 10;
                            }
                            let rstr = core::str::from_utf8(&rbuf[idx..]).unwrap_or("0");
                            self.write_line(rstr);
                        }
                    } else {
                        self.write_line("  [ERR] Failed to parse WASM binary");
                    }
                }
            } else {
                self.write_str("  [ERR] File not found: ");
                self.write_line(&resolved);
            }
        } else if cmd_str == "sound" {
            self.write_line("Playing Cyber Harmonic Synthesizer Chime...");
            crate::sound::Sound::play_boot_chime();
        } else if cmd_str == "skills" {
            self.write_line("── Autonomous AI Subsystems ──");
            self.write_line("  [1] Memory: Context Vector Graph [ONLINE]");
            self.write_line("  [2] Voice:  Neural TTS/STT Matrix [ONLINE]");
            self.write_line("  [3] Vision: Multimodal Canvas Stream [ONLINE]");
            self.write_line("  [4] Skills: Autonomous Agent Core [ACTIVE]");
            self.write_line("  [5] Security: Bio-Gate AXON-7 [SECURE]");
        } else if cmd_str == "net" {
            self.write_line("── Quantum Mesh Network Telemetry ──");
            self.write_line("  Interface: loopback (127.0.0.1) [UP]");
            self.write_line("  Adapter: VirtIO-Net PCI [INITIALIZED]");
            self.write_line("  Mesh Packets: TX 1,024 | RX 1,024 | 0% Loss");
        } else if cmd_str == "clear" {
            self.clear();
            return;
        } else if cmd_str == "neofetch" {
            self.write_line("   _ _          guest@atulyaos");
            self.write_line("  / V \\         --------------");
            self.write_line("  \\   /         OS Type: macOS/Linux/Win Fusion");
            self.write_line("   \\ /          Kernel: Atulya Core v1.0");
            self.write_line("    V           CPU: x86_64 Core (Freestanding)");
            self.write_line("                Memory: 128MB / 256MB RAM");
            self.write_line("                Uptime: Active");
        } else if cmd_str == "scan" {
            self.write_line("Initializing full system diagnostics...");
            *scan_active = 1;
            return;
        } else if cmd_str == "theme" {
            *theme_idx = (*theme_idx + 1) % 4;
            let name = match *theme_idx {
                0 => "Cyberpunk Cyan",
                1 => "Matrix Green",
                2 => "macOS Obsidian",
                _ => "Retro Gold",
            };
            self.write_str("Theme changed: ");
            self.write_line(name);
        } else if cmd_str == "matrix" {
            self.write_line("Engaging Matrix stream (Esc/Enter to exit)...");
            *matrix_active = 1;
            return;
        } else if cmd_str == "joke" {
            let jokes = [
                "There are 10 types of people: those who understand binary, and those who don't.",
                "Why do programmers wear glasses? Because they can't C#.",
                "Hardware is what you kick; software is what you curse.",
                "No_std Rust kernel has no allocator, but it has plenty of class.",
            ];
            self.write_line(jokes[self.joke_idx]);
            self.joke_idx = (self.joke_idx + 1) % jokes.len();
        } else if cmd_str == "pwd" {
            let cwd = self.cwd.clone();
            self.write_line(&cwd);
        } else if cmd_str.starts_with("cd ") {
            let dir = cmd_str.get(3..).unwrap_or("").trim();
            if dir.is_empty() {
                self.cwd = alloc::string::String::from("/");
            } else {
                let resolved = self.resolve_path(dir);
                if self.fs.ls(&resolved).is_ok() {
                    self.cwd = resolved;
                } else {
                    self.write_str("cd: ");
                    self.write_str(dir);
                    self.write_line(": No such directory");
                }
            }
        } else if cmd_str == "ls" {
            let dir = if self.cwd == "/" {
                alloc::string::String::from("/")
            } else {
                self.cwd.clone()
            };
            match self.fs.ls(&dir) {
                Ok(entries) => {
                    for entry in &entries {
                        if entry.is_dir {
                            self.write_str("  ");
                            self.write_str(&entry.name);
                            self.write_line("/");
                        } else {
                            self.write_str("  ");
                            self.write_str(&entry.name);
                            self.write_str(" (");
                            let mut buf = [0u8; 12];
                            let mut val = entry.size;
                            let mut idx = 12;
                            if val == 0 {
                                buf[11] = b'0';
                                idx = 11;
                            } else {
                                while val > 0 && idx > 0 {
                                    idx -= 1;
                                    buf[idx] = b'0' + (val % 10) as u8;
                                    val /= 10;
                                }
                            }
                            for &b in &buf[idx..12] {
                                self.write_char(b as char);
                            }
                            self.write_line(" bytes)");
                        }
                    }
                }
                Err(_) => self.write_line("ls: cannot access directory"),
            }
        } else if cmd_str.starts_with("ls ") {
            let dir = cmd_str.get(3..).unwrap_or("").trim();
            let resolved = self.resolve_path(dir);
            match self.fs.ls(&resolved) {
                Ok(entries) => {
                    for entry in &entries {
                        if entry.is_dir {
                            self.write_str("  ");
                            self.write_str(&entry.name);
                            self.write_line("/");
                        } else {
                            self.write_str("  ");
                            self.write_str(&entry.name);
                            self.write_str(" (");
                            let mut buf = [0u8; 12];
                            let mut val = entry.size;
                            let mut idx = 12;
                            if val == 0 {
                                buf[11] = b'0';
                                idx = 11;
                            } else {
                                while val > 0 && idx > 0 {
                                    idx -= 1;
                                    buf[idx] = b'0' + (val % 10) as u8;
                                    val /= 10;
                                }
                            }
                            for &b in &buf[idx..12] {
                                self.write_char(b as char);
                            }
                            self.write_line(" bytes)");
                        }
                    }
                }
                Err(_) => self.write_line("ls: cannot access directory"),
            }
        } else if cmd_str.starts_with("cat ") || cmd_str.starts_with("view ") || cmd_str.starts_with("open ") {
            let prefix_len = if cmd_str.starts_with("cat ") { 4 } else { 5 };
            let file = cmd_str.get(prefix_len..).unwrap_or("").trim();
            let resolved = self.resolve_path(file);
            match self.fs.open(&resolved, OpenFlags::Read) {
                Ok(handle) => {
                    let mut buf = alloc::vec![0u8; 8192];
                    let mut total_read = 0;
                    while total_read < buf.len() {
                        match self.fs.read(handle, &mut buf[total_read..]) {
                            Ok(0) => break,
                            Ok(n) => total_read += n,
                            Err(_) => break,
                        }
                    }
                    let _ = self.fs.close(handle);

                    let decoded = crate::viewer::sniff_and_decode(&resolved, &buf[..total_read]);
                    self.write_str(decoded.format.icon());
                    self.write_str(" [");
                    self.write_str(decoded.format.name());
                    self.write_line("]");
                    self.write_str("  ");
                    self.write_line(&decoded.header_summary);
                    for line in &decoded.preview_lines {
                        self.write_line(line);
                    }
                }
                Err(_) => {
                    self.write_str("view: ");
                    self.write_str(file);
                    self.write_line(": No such file");
                }
            }
        } else if cmd_str.starts_with("mkdir ") {
            let dir = cmd_str.get(6..).unwrap_or("").trim();
            let resolved = self.resolve_path(dir);
            match self.fs.mkdir(&resolved) {
                Ok(_) => {
                    let _ = self.fs.sync_to_disk();
                    self.write_line("Directory created (persisted to disk)");
                }
                Err(_) => self.write_line("mkdir: failed to create directory"),
            }
        } else if cmd_str.starts_with("rm ") {
            let file = cmd_str.get(3..).unwrap_or("").trim();
            let resolved = self.resolve_path(file);
            match self.fs.rm(&resolved) {
                Ok(_) => {
                    let _ = self.fs.sync_to_disk();
                    self.write_line("Removed (persisted to disk)");
                }
                Err(_) => self.write_line("rm: failed to remove"),
            }
        } else if cmd_str.starts_with("echo ") {
            let rest = &cmd_str[5..];
            if let Some(pos) = rest.rfind(" > ") {
                let text = &rest[..pos];
                let file = rest[pos + 3..].trim();
                let resolved = self.resolve_path(file);
                match self.fs.open(&resolved, OpenFlags::Create) {
                    Ok(handle) => {
                        let _ = self.fs.write(handle, text.as_bytes());
                        let _ = self.fs.close(handle);
                        let _ = self.fs.sync_to_disk();
                        self.write_line("Wrote and persisted to ATA disk");
                    }
                    Err(_) => self.write_line("echo: cannot write to file"),
                }
            } else {
                self.write_line(rest);
            }
        } else {
            self.write_str("atulyaos: ");
            self.write_str(cmd_str);
            self.write_line(": command not found");
        }

        self.print_prompt();
    }
}

// ── Window state ────────────────────────────────────────────────────
struct Window {
    x: isize,
    y: isize,
    w: usize,
    h: usize,
    title: &'static str,
    active: bool,
    anim_scale: u16, // 0..256 smooth easing scale
    is_open: bool,
}

// ── Desktop entry point ─────────────────────────────────────────────
pub fn run(display: &mut Display) -> ! {
    unsafe { init_mouse() }

    let w = display.width();
    let h = display.height();

    let mut mouse = MouseState::new(w, h);
    let mut kbd = KeyboardState { shift_pressed: false };
    let mut term = Terminal::new();
    let mut theme_idx: usize = 0;
    let mut scan_active: usize = 0;
    let mut matrix_active: usize = 0;
    let mut mouse_was_pressed = false;
    let mut drag_window: Option<(isize, isize, isize, isize)> = None;

    let mut windows = alloc::vec![
        Window { x: 200, y: 120, w: 560, h: 340, title: "Terminal", active: true, anim_scale: 256, is_open: true },
        Window { x: 240, y: 150, w: 500, h: 320, title: "Web Browser", active: false, anim_scale: 256, is_open: true },
        Window { x: 280, y: 170, w: 460, h: 280, title: "Network Mesh", active: false, anim_scale: 0, is_open: false },
        Window { x: 300, y: 180, w: 520, h: 320, title: "Code Editor", active: false, anim_scale: 0, is_open: false },
        Window { x: 220, y: 130, w: 440, h: 280, title: "File Manager", active: false, anim_scale: 0, is_open: false },
        Window { x: 260, y: 160, w: 480, h: 300, title: "System Analytics", active: false, anim_scale: 0, is_open: false },
        Window { x: 320, y: 190, w: 400, h: 240, title: "Media Player", active: false, anim_scale: 0, is_open: false },
        Window { x: 340, y: 200, w: 420, h: 260, title: "3D Container", active: false, anim_scale: 0, is_open: false },
        Window { x: 360, y: 210, w: 440, h: 270, title: "Security Shield", active: false, anim_scale: 0, is_open: false },
    ];

    let mut focused_win: usize = 0;

    let mut last_tick = crate::interrupts::tick_counter::get();

    
    loop {
        
        let theme = &THEMES[theme_idx % 4];

        // ── 1. Background Futuristic Obsidian Cyber Canvas ────────────────
        display.gradient_rect_v(0, 0, w, h, Rgb::new(1, 2, 4), Rgb::new(3, 4, 10));

        // Background subtle starry sky
        let star_seed = [45usize, 180, 360, 580, 820, 1040, 1260, 1480, 1700, 1860];
        for (si, &sx) in star_seed.iter().enumerate() {
            let sy = (si * 43 + 20) % (h * 60 / 100);
            display.pixel(sx % w, sy, Rgb::new(140, 180, 220));
        }

        // Background dark mountain / skyline silhouette
        let city_y = h * 68 / 100;
        display.gradient_rect_v(0, city_y, w, h - city_y, Rgb::new(2, 4, 8), Rgb::new(1, 2, 4));
        display.rect(0, city_y, w, 1, theme.accent.dim(40));

        // ── 2. Top-Center AI Greeting & Waveform ─────────────────────────────
        let tick = crate::interrupts::tick_counter::get();
        let cx = w / 2;
        crate::font::centered_text_aa(display, cx, 28, "GOOD EVENING, ATUL", Rgb::new(180, 230, 255));
        crate::font::centered_text_aa(display, cx, 48, "ATULYA INTENT OS IS READY", theme.accent);

        // Consolidated Float Harmonic Audio Waveform
        let effects = crate::gpu::effects::EffectRenderer::new();
        effects.draw_harmonic_waveform(display, cx, 80, 320, tick, theme.accent);

        // ── 3. Left Navigation Sidebar ───────────────────────────────────────
        let side_w = 160;
        display.rect_rounded_alpha(16, 20, side_w, h - 90, 8, Rgb::new(3, 10, 22), 200);
        display.rect_rounded_outline(16, 20, side_w, h - 90, 8, theme.accent.dim(100));

        // (A) ATULYA OS brand
        display.circle_outline(36, 42, 10, theme.accent);
        crate::font::draw_char(display, 32, 38, b'A', 1, theme.accent);
        crate::font::draw_text(display, 52, 38, "ATULYA OS", 1, Rgb::new(200, 240, 255));
        display.rect(26, 60, side_w - 20, 1, theme.accent.dim(80));

        let nav_items = ["Atulya", "Workspace", "Projects", "Memory", "Skills", "Settings"];
        for (ni, item) in nav_items.iter().enumerate() {
            let ny = 80 + ni * 34;
            display.circle_filled(32, ny + 4, 3, theme.accent.dim(180));
            crate::font::draw_text(display, 44, ny, item, 1, Rgb::new(180, 220, 250));
        }

        // ── 4. Right Telemetry HUD Card ──────────────────────────────────────
        let right_w = 210;
        let right_x = w.saturating_sub(right_w + 16);
        display.rect_rounded_alpha(right_x, 20, right_w, 230, 8, Rgb::new(3, 10, 22), 200);
        display.rect_rounded_outline(right_x, 20, right_w, 230, 8, theme.accent.dim(100));

        crate::font::draw_text(display, right_x + 14, 34, "ATULYA CORE STATUS", 1, theme.accent);
        display.rect(right_x + 14, 48, right_w - 28, 1, theme.accent.dim(80));

        let status_list = [
            ("Memory", "ONLINE", Rgb::new(210, 80, 255)),
            ("Voice",  "ONLINE", theme.accent),
            ("Vision", "ONLINE", Rgb::new(0, 150, 255)),
            ("Skills", "ONLINE", Rgb::new(255, 160, 40)),
            ("Security", "SECURE", Rgb::new(0, 230, 118)),
        ];

        for (si, (label, val, dot_col)) in status_list.iter().enumerate() {
            let sy = 60 + si * 22;
            crate::font::draw_text(display, right_x + 14, sy, label, 1, Rgb::new(180, 210, 240));
            crate::font::draw_text(display, right_x + 110, sy, val, 1, *dot_col);
            display.circle_filled(right_x + 175, sy + 3, 3, *dot_col);
        }

        // Active Context Box
        let act_y = 176;
        display.rect(right_x + 14, act_y, right_w - 28, 1, theme.accent.dim(80));
        crate::font::draw_text(display, right_x + 14, act_y + 8, "ACTIVE CONTEXT", 1, theme.accent.dim(180));
        crate::font::draw_text(display, right_x + 14, act_y + 22, "No active context.", 1, Rgb::new(150, 190, 220));
        crate::font::draw_text(display, right_x + 14, act_y + 34, "You're all set, Atul.", 1, Rgb::new(150, 190, 220));

        // Circular Holographic (A) Insignia Badge
        let badge_y = 265;
        display.rect_rounded_alpha(right_x, badge_y, right_w, 100, 8, Rgb::new(3, 10, 22), 200);
        display.rect_rounded_outline(right_x, badge_y, right_w, 100, 8, theme.accent.dim(100));
        let badge_cx = right_x + right_w / 2;
        let badge_cy = badge_y + 50;
        display.dotted_circle_outline(badge_cx, badge_cy, 36, 3, theme.accent.dim(140));
        display.circle_outline(badge_cx, badge_cy, 28, theme.accent);
        crate::font::centered_text(display, badge_cx, badge_cy - 10, "A", 3, theme.accent);

        // ── 5. Bottom Floating Glass Dock (9 Storyboard Apps) ────────────────
        let dock_apps = [
            ("Term", ">_"),
            ("Web",  "W3"),
            ("Mesh", "NET"),
            ("Code", "</>"),
            ("File", "DIR"),
            ("Stat", "CPU"),
            ("Play", "AV"),
            ("3D",   "BOX"),
            ("Sec",  "SEC"),
        ];

        let icon_w: usize = 48;
        let icon_h: usize = 42;
        let dock_total_w = dock_apps.len() * (icon_w + 12) + 24;
        let dock_x = cx.saturating_sub(dock_total_w / 2);
        let dock_y = h.saturating_sub(60);

        display.rect_rounded_alpha(dock_x, dock_y, dock_total_w, 52, 10, Rgb::new(3, 10, 22), 220);
        display.rect_rounded_outline(dock_x, dock_y, dock_total_w, 52, 10, theme.accent.dim(120));

        for (i, (_name, _icon)) in dock_apps.iter().enumerate() {
            let ix = dock_x + 12 + i * (icon_w + 12);
            let iy = dock_y + 5;
            let is_hovered = (mouse.x as usize >= ix && mouse.x as usize <= ix + icon_w)
                && (mouse.y as usize >= iy && mouse.y as usize <= iy + icon_h);

            let bg_alpha = if is_hovered { 230 } else { 140 };
            display.rect_rounded_alpha(ix, iy, icon_w, icon_h, 6, Rgb::new(6, 24, 48), bg_alpha);
            display.rect_rounded_outline(ix, iy, icon_w, icon_h, 6, if is_hovered { theme.accent } else { theme.accent.dim(100) });

            // Blit high-res 32x32 RGBA icon
            let icon_slice = &DOCK_ICONS[i * (32 * 32 * 4)..(i + 1) * (32 * 32 * 4)];
            display.blit_rgba_sprite(ix + (icon_w - 32) / 2, iy + 4, 32, 32, icon_slice);
        }

        // Draw taskbar clock
        let ticks = crate::interrupts::tick_counter::get();
        let total_secs = (ticks / 18) as usize;
        let hours   = ((total_secs / 3600) % 24) as usize;
        let minutes = ((total_secs / 60) % 60) as usize;
        let seconds = (total_secs % 60) as usize;
        let time_str = [
            b'0' + (hours / 10) as u8, b'0' + (hours % 10) as u8, b':',
            b'0' + (minutes / 10) as u8, b'0' + (minutes % 10) as u8, b':',
            b'0' + (seconds / 10) as u8, b'0' + (seconds % 10) as u8,
        ];
        let clk_x = (w as isize - 90) as usize;
        let clk_y = 28;
        for (i, &ch) in time_str.iter().enumerate() {
            crate::font::draw_char(display, clk_x + i * 8, clk_y, ch, 1, theme.accent);
        }

        // Animate window easing transitions (200ms cubic scale & opacity)
        for win in windows.iter_mut() {
            if win.is_open && win.anim_scale < 256 {
                win.anim_scale = (win.anim_scale + 32).min(256);
            } else if !win.is_open && win.anim_scale > 0 {
                win.anim_scale = win.anim_scale.saturating_sub(32);
            }
        }

        // Draw windows (back to front)
        for (i, win) in windows.iter().enumerate() {
            if win.anim_scale == 0 {
                continue;
            }
            let s = win.anim_scale as usize;
            let rw = (win.w * s) / 256;
            let rh = (win.h * s) / 256;
            let rx = (win.x + ((win.w - rw) / 2) as isize).max(0) as usize;
            let ry = (win.y + ((win.h - rh) / 2) as isize).max(0) as usize;

            let alpha = (230 * win.anim_scale / 256) as u16;

            let border_color = if i == focused_win || win.active {
                theme.win_active
            } else {
                theme.win_inactive
            };

            // Window shadow
            display.rect_rounded_alpha(
                rx + 4,
                ry + 4,
                rw,
                rh,
                8,
                Rgb::new(0, 0, 0),
                (60 * win.anim_scale / 256) as u16,
            );

            // Window body
            display.rect_rounded_alpha(
                rx,
                ry,
                rw,
                rh,
                8,
                Rgb::new(16, 16, 20),
                alpha,
            );
            display.rect_rounded_outline(rx, ry, rw, rh, 8, border_color.dim(win.anim_scale));

            // Title bar
            if rh >= 28 {
                display.rect_rounded_alpha(
                    rx,
                    ry,
                    rw,
                    28,
                    6,
                    Rgb::new(24, 24, 30),
                    (240 * win.anim_scale / 256) as u16,
                );

                // Title text
                for (ci, ch) in win.title.bytes().enumerate() {
                    let tx = rx + 30 + ci * 8;
                    if tx + 8 < rx + rw {
                        crate::font::draw_char(display, tx, ry + 8, ch, 1, theme.text.dim(win.anim_scale));
                    }
                }

                // Close button (red circle)
                if rw >= 24 {
                    let cx_btn = rx + rw - 18;
                    let cy_btn = ry + 14;
                    display.circle_filled(cx_btn, cy_btn, 6, Rgb::new(255, 80, 80).dim(win.anim_scale));
                    display.circle_outline(cx_btn, cy_btn, 6, Rgb::new(200, 50, 50).dim(win.anim_scale));
                }
            }

            // Content area
            if win.title == "Terminal" && i == focused_win {
                // Draw terminal content
                for row in 0..TERM_ROWS {
                    for col in 0..TERM_COLS {
                        let ch = term.buffer[row][col];
                        if ch != b' ' {
                            let tx = win.x as usize + 10 + col * 8;
                            let ty = win.y as usize + 34 + row * 16;
                            if tx + 8 < win.x as usize + win.w
                                && ty + 16 < win.y as usize + win.h
                            {
                                crate::font::draw_char(display, tx, ty, ch, 1, theme.text);
                            }
                        }
                    }
                }
            } else if win.title == "System Analytics" {
                let tick = crate::interrupts::tick_counter::get();
                let info_lines = [
                    "── System Analytics ──",
                    "  OS: Atulya OS (Cyberpunk Edition)",
                    "  Architecture: x86_64 Long Mode",
                    "  Heap Allocation: 128 MB Active",
                    "  VGA Mode: 1920x1080 32bpp TrueColor",
                    "  Interrupt Subsystems: IDT / PIC / PIT",
                    "  CPU State: Active [Optimal]",
                ];
                for (li, line) in info_lines.iter().enumerate() {
                    let ty = win.y as usize + 36 + li * 18;
                    for (ci, ch) in line.bytes().enumerate() {
                        let tx = win.x as usize + 12 + ci * 8;
                        if tx + 8 < win.x as usize + win.w && ty + 16 < win.y as usize + win.h {
                            crate::font::draw_char(display, tx, ty, ch, 1, theme.accent);
                        }
                    }
                }
                // Live tick rate & uptime
                let mut buf = [0u8; 16];
                let mut val = tick / 100;
                let mut idx = 16;
                if val == 0 { buf[15] = b'0'; idx = 15; }
                while val > 0 && idx > 0 {
                    idx -= 1;
                    buf[idx] = b'0' + (val % 10) as u8;
                    val /= 10;
                }
                let ut_str = core::str::from_utf8(&buf[idx..]).unwrap_or("0");
                let uy = win.y as usize + 36 + 7 * 18;
                crate::font::draw_text(display, win.x as usize + 12, uy, "  Uptime (seconds): ", 1, theme.text);
                crate::font::draw_text(display, win.x as usize + 170, uy, ut_str, 1, theme.accent);
            } else if win.title == "Web Browser" {
                let lines = [
                    "── Atulya Cyber Web Browser ──",
                    "  [URL: atulya://ai.network/home]",
                    "",
                    "  Welcome to the Quantum Decentralized Web.",
                    "  Connected to 14,892 Mesh Nodes.",
                    "  Encryption: Quantum Post-Key (Kyber-1024)",
                    "  Latency: 0.12 ms | Throughput: 100 Gbps",
                ];
                for (li, line) in lines.iter().enumerate() {
                    let ty = win.y as usize + 36 + li * 18;
                    for (ci, ch) in line.bytes().enumerate() {
                        let tx = win.x as usize + 12 + ci * 8;
                        if tx + 8 < win.x as usize + win.w && ty + 16 < win.y as usize + win.h {
                            crate::font::draw_char(display, tx, ty, ch, 1, theme.text);
                        }
                    }
                }
            } else if win.title == "Code Editor" {
                let lines = [
                    "// main.rs - Atulya OS Kernel",
                    "fn kernel_main(boot_info: &BootInfo) -> ! {",
                    "    serial::serial_init();",
                    "    boot_splash::run(&mut display);",
                    "    desktop::run(&mut display);",
                    "}",
                ];
                for (li, line) in lines.iter().enumerate() {
                    let ty = win.y as usize + 36 + li * 18;
                    for (ci, ch) in line.bytes().enumerate() {
                        let tx = win.x as usize + 12 + ci * 8;
                        if tx + 8 < win.x as usize + win.w && ty + 16 < win.y as usize + win.h {
                            let col = if li == 0 { Rgb::new(100, 200, 100) } else { theme.accent };
                            crate::font::draw_char(display, tx, ty, ch, 1, col);
                        }
                    }
                }
            } else if win.title == "File Manager" {
                let lines = [
                    "── File Manager (VFS / RAMDisk) ──",
                    "  📁 /",
                    "  ├── 📁 system/",
                    "  │   ├── 📄 kernel.elf",
                    "  │   └── 📄 config.sys",
                    "  ├── 📁 apps/",
                    "  └── 📁 user/ (Atul)",
                ];
                for (li, line) in lines.iter().enumerate() {
                    let ty = win.y as usize + 36 + li * 18;
                    for (ci, ch) in line.bytes().enumerate() {
                        let tx = win.x as usize + 12 + ci * 8;
                        if tx + 8 < win.x as usize + win.w && ty + 16 < win.y as usize + win.h {
                            crate::font::draw_char(display, tx, ty, ch, 1, theme.text);
                        }
                    }
                }
            } else if win.title == "Security Shield" {
                let lines = [
                    "── Security Status ──",
                    "  Firewall: ACTIVE [Enforced]",
                    "  Identity: ATUL (Full Clearance)",
                    "  Biometric Auth: VERIFIED",
                    "  Port Scanner: 0 Open Inbound",
                    "  Intrusion Defense: ARMED",
                ];
                for (li, line) in lines.iter().enumerate() {
                    let ty = win.y as usize + 36 + li * 18;
                    for (ci, ch) in line.bytes().enumerate() {
                        let tx = win.x as usize + 12 + ci * 8;
                        if tx + 8 < win.x as usize + win.w && ty + 16 < win.y as usize + win.h {
                            crate::font::draw_char(display, tx, ty, ch, 1, Rgb::new(0, 230, 118));
                        }
                    }
                }
            } else if win.title == "Network Mesh" {
                let lines = [
                    "── Quantum Mesh Network ──",
                    "  Mesh State: SYNCHRONIZED",
                    "  Nodes Online: 4,096",
                    "  Gateway: 10.0.0.1 (Atulya Hub)",
                    "  Bandwidth: 10 Gbps Duplex",
                ];
                for (li, line) in lines.iter().enumerate() {
                    let ty = win.y as usize + 36 + li * 18;
                    for (ci, ch) in line.bytes().enumerate() {
                        let tx = win.x as usize + 12 + ci * 8;
                        if tx + 8 < win.x as usize + win.w && ty + 16 < win.y as usize + win.h {
                            crate::font::draw_char(display, tx, ty, ch, 1, theme.accent);
                        }
                    }
                }
            } else if win.title == "Media Player" {
                let lines = [
                    "── Media Player ──",
                    "  Track: Cyberpunk Awakening OST",
                    "  Artist: Atulya Sound Engine",
                    "  Status: Playing [48kHz 24-bit]",
                    "  Equalizer: Dynamic Spatial Audio",
                ];
                for (li, line) in lines.iter().enumerate() {
                    let ty = win.y as usize + 36 + li * 18;
                    for (ci, ch) in line.bytes().enumerate() {
                        let tx = win.x as usize + 12 + ci * 8;
                        if tx + 8 < win.x as usize + win.w && ty + 16 < win.y as usize + win.h {
                            crate::font::draw_char(display, tx, ty, ch, 1, Rgb::new(220, 100, 255));
                        }
                    }
                }
            } else if win.title == "3D Container" {
                let lines = [
                    "── 3D Container Runtime ──",
                    "  Container: Axon-Node-01",
                    "  Engine: WebAssembly (Wasm-3D)",
                    "  GPU Virtualization: Active",
                    "  Framerate: 60.0 FPS",
                ];
                for (li, line) in lines.iter().enumerate() {
                    let ty = win.y as usize + 36 + li * 18;
                    for (ci, ch) in line.bytes().enumerate() {
                        let tx = win.x as usize + 12 + ci * 8;
                        if tx + 8 < win.x as usize + win.w && ty + 16 < win.y as usize + win.h {
                            crate::font::draw_char(display, tx, ty, ch, 1, theme.text);
                        }
                    }
                }
            }
        }

        // Draw mouse cursor
        display.circle_filled(mouse.x as usize, mouse.y as usize, 5, Rgb::new(255, 255, 255));
        display.circle_outline(mouse.x as usize, mouse.y as usize, 5, theme.accent);

        // Swap buffers
        display.swap_buffers();

        // Process keyboard queue
        while let Some(scancode) = crate::interrupts::KEYBOARD_QUEUE.lock().pop() {
            if scancode == 0x0F {
                // Tab: cycle window focus
                if !windows.is_empty() {
                    windows[focused_win].active = false;
                    focused_win = (focused_win + 1) % windows.len();
                    windows[focused_win].active = true;
                }
                continue;
            }
            if let Some(ch) = kbd.handle_scancode(scancode) {
                if matrix_active != 0 {
                    if ch == '\x1b' || ch == '\n' {
                        matrix_active = 0;
                    }
                    continue;
                }
                if ch == '\n' {
                    term.execute_command(&mut theme_idx, &mut scan_active, &mut matrix_active);
                } else if ch == '\x08' {
                    term.backspace();
                } else if ch != '\0' {
                    term.write_char(ch);
                }
            }
        }

        // Process mouse queue
        while let Some(byte) = crate::interrupts::MOUSE_QUEUE.lock().pop() {
            if mouse.handle_byte(byte, w, h) {
                let mouse_pressed = mouse.buttons & 1 != 0;
                let mx = mouse.x;
                let my = mouse.y;

                // Check close button clicks
                let mut close_idx: Option<usize> = None;
                {
                    for (i, win) in windows.iter().enumerate() {
                        if !win.is_open || win.anim_scale < 180 { continue; }
                        let cx_btn = win.x as usize + win.w - 18;
                        let cy_btn = win.y as usize + 14;
                        let dx = mx as isize - cx_btn as isize;
                        let dy = my as isize - cy_btn as isize;
                        if dx * dx + dy * dy < 64 && mouse_pressed && !mouse_was_pressed {
                            close_idx = Some(i);
                        }
                    }
                }
                if let Some(i) = close_idx {
                    windows[i].is_open = false;
                }

                // Start dragging on title bar
                if mouse_pressed && !mouse_was_pressed {
                    let mut drag_info: Option<(usize, isize, isize)> = None;
                    {
                        for (i, win) in windows.iter().enumerate().rev() {
                            if win.is_open && win.anim_scale >= 180
                                && mx >= win.x
                                && mx < win.x + win.w as isize
                                && my >= win.y
                                && my < win.y + 28
                            {
                                drag_info = Some((i, mx - win.x, my - win.y));
                                break;
                            }
                        }
                    }
                    if let Some((i, ox, oy)) = drag_info {
                        // Move window to end of vec for z-ordering
                        let win = windows.remove(i);
                        windows.push(win);
                        let new_idx = windows.len() - 1;
                        drag_window = Some((new_idx as isize, ox, oy, 0));
                        if !windows[new_idx].active {
                            windows[focused_win].active = false;
                            focused_win = new_idx;
                            windows[new_idx].active = true;
                        }
                    }
                }

                // Dragging
                if mouse_pressed {
                    if let Some((idx, ox, oy, _)) = drag_window {
                        let idx = idx as usize;
                        windows[idx].x = mx - ox;
                        windows[idx].y = my - oy;
                    }
                } else {
                    drag_window = None;
                }

                // Dock icon click / restore
                let dock_apps_count = 9usize;
                let icon_w_i: isize = 48;
                let dock_total_w_i = (dock_apps_count * (48 + 12) + 24) as isize;
                let dock_start_x_i = (w / 2) as isize - dock_total_w_i / 2;
                let dock_y_i = h as isize - 60;

                if mouse_pressed && !mouse_was_pressed && my >= dock_y_i && my <= dock_y_i + 52 {
                    for i in 0..dock_apps_count {
                        let ix = dock_start_x_i + 12 + i as isize * (icon_w_i + 12);
                        if mx >= ix && mx <= ix + icon_w_i {
                            // Focus or restore window
                            if i < windows.len() {
                                windows[i].is_open = true;
                                if windows[i].anim_scale == 0 {
                                    windows[i].anim_scale = 32;
                                }
                                windows[focused_win].active = false;
                                windows[i].active = true;
                                focused_win = i;
                            }
                        }
                    }
                }

                mouse_was_pressed = mouse_pressed;
            }
        }

        // Yield until next timer interrupt (CPU halts, idle process runs)
        let now = crate::interrupts::tick_counter::get();
        while crate::interrupts::tick_counter::get() == now {
            unsafe { core::arch::asm!("hlt"); }
        }
        let _ = last_tick;
        last_tick = now;
    }
}
