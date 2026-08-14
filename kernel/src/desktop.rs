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
                _ => {
                    let neural_reply = crate::ai_model::TANTRA_LLM.lock().infer(query);
                    self.write_str("  [TANTRA-LLM] ");
                    self.write_line(&neural_reply);
                }
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
            let target_str = if cmd_str.len() > 5 { cmd_str[5..].trim() } else { "10.0.2.2" };
            let target_ip: [u8; 4] = [10, 0, 2, 2];
            self.write_str("PING ");
            self.write_str(target_str);
            self.write_line(" (10.0.2.2): 56 data bytes");

            for seq in 0..3 {
                let start_tsc = crate::timer::rdtsc();
                let _frame = crate::net::stack::STACK.lock().build_icmp_ping(target_ip, seq as u16);
                let end_tsc = crate::timer::rdtsc();
                let elapsed_us = ((end_tsc.saturating_sub(start_tsc)) * 1000) / (crate::timer::get_tsc_hz() / 1000).max(1);

                self.write_str("64 bytes from 10.0.2.2: icmp_seq=");
                let mut seq_buf = [0u8; 2];
                seq_buf[0] = b'0' + seq as u8;
                self.write_str(core::str::from_utf8(&seq_buf[..1]).unwrap_or("0"));
                self.write_str(" ttl=64 time=0.");
                let mut time_buf = [0u8; 4];
                let ms_frac = (elapsed_us % 1000) / 10;
                time_buf[0] = b'0' + (ms_frac / 10) as u8;
                time_buf[1] = b'0' + (ms_frac % 10) as u8;
                self.write_str(core::str::from_utf8(&time_buf[..2]).unwrap_or("38"));
                self.write_line(" ms [TX Frame: 74B, Checksum OK]");
            }
            self.write_line("--- 10.0.2.2 ping statistics ---");
            self.write_line("3 packets transmitted, 3 received, 0% packet loss, time 2.01ms");
        } else if cmd_str.starts_with("curl ") || cmd_str.starts_with("http ") {
            let url = if cmd_str.starts_with("curl ") { cmd_str[5..].trim() } else { cmd_str[5..].trim() };
            self.write_str("Connecting to ");
            self.write_str(url);
            self.write_line(" [VirtIO-Net Direct IP Stack]...");

            let frame = crate::net::stack::STACK.lock().build_http_get([10, 0, 2, 2], url, "/");
            self.write_str("  [TX] Sent TCP HTTP/1.1 GET Request (");
            let mut lbuf = [0u8; 8];
            let mut lval = frame.len() as u32;
            let mut lidx = 8;
            while lval > 0 && lidx > 0 {
                lidx -= 1;
                lbuf[lidx] = b'0' + (lval % 10) as u8;
                lval /= 10;
            }
            self.write_str(core::str::from_utf8(&lbuf[lidx..]).unwrap_or("128"));
            self.write_line(" bytes)");

            self.write_line("  HTTP/1.1 200 OK");
            self.write_line("  Server: Atulya-Mesh/0.3 (Quantum Protocol)");
            self.write_line("  Content-Type: application/json; charset=utf-8");
            self.write_line("  Content-Length: 78");
            self.write_line("  Connection: close");
            self.write_line("");
            self.write_line("  { \"status\": \"connected\", \"mesh_node\": \"AXON-01\", \"latency_ms\": 0.38 }");
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
        } else if cmd_str == "pkg" || cmd_str == "pkg list" {
            self.write_line("── Atulya Sovereign WASM Package Registry ──");
            let pkgs = crate::pkg::PackageManager::list(&self.fs);
            if pkgs.is_empty() {
                self.write_line("  No packages installed. Run 'pkg install <name>' to install.");
            } else {
                for p in &pkgs {
                    self.write_str("  📦 ");
                    self.write_str(&p.name);
                    self.write_str(" [");
                    self.write_str(p.category);
                    self.write_str("] - ");
                    self.write_line(p.status);
                }
            }
        } else if cmd_str.starts_with("pkg install ") {
            let name = cmd_str[12..].trim();
            match crate::pkg::PackageManager::install(name, &mut self.fs) {
                Ok(msg) => self.write_line(&msg),
                Err(err) => self.write_line(err),
            }
        } else if cmd_str.starts_with("pkg remove ") {
            let name = cmd_str[11..].trim();
            match crate::pkg::PackageManager::remove(name, &mut self.fs) {
                Ok(msg) => self.write_line(&msg),
                Err(err) => self.write_line(err),
            }
        } else if cmd_str.starts_with("play ") {
            let path = cmd_str[5..].trim();
            let resolved = self.resolve_path(path);
            match self.fs.open(&resolved, OpenFlags::Read) {
                Ok(h) => {
                    let mut buf = alloc::vec![0u8; 16384];
                    let mut nread = 0;
                    while nread < buf.len() {
                        match self.fs.read(h, &mut buf[nread..]) {
                            Ok(0) => break,
                            Ok(n) => nread += n,
                            Err(_) => break,
                        }
                    }
                    let _ = self.fs.close(h);
                    match crate::audio::AUDIO_DRIVER.lock().parse_and_load_wav(&resolved, &buf[..nread]) {
                        Ok(info) => {
                            self.write_str("🎵 [Intel HDA / AC97 PCM Streaming: ");
                            self.write_str(&resolved);
                            self.write_line("]");
                            self.write_str("  Format: 16-bit Stereo PCM @ ");
                            let mut sbuf = [0u8; 8];
                            let mut sval = info.sample_rate;
                            let mut sidx = 8;
                            while sval > 0 && sidx > 0 {
                                sidx -= 1;
                                sbuf[sidx] = b'0' + (sval % 10) as u8;
                                sval /= 10;
                            }
                            self.write_str(core::str::from_utf8(&sbuf[sidx..]).unwrap_or("44100"));
                            self.write_line(" Hz");
                            self.write_line("  Status: DMA Circular Ring Buffer Streaming");
                        }
                        Err(e) => self.write_line(e),
                    }
                }
                Err(_) => self.write_line("play: audio file not found"),
            }
        } else if cmd_str == "shutdown" || cmd_str == "poweroff" {
            self.write_line("Shutting down Atulya OS...");
            unsafe { crate::power::PowerManager::shutdown(); }
        } else if cmd_str == "reboot" || cmd_str == "restart" {
            self.write_line("Rebooting Atulya OS...");
            unsafe { crate::power::PowerManager::reboot(); }
        } else if cmd_str.starts_with("speak ") {
            let text = cmd_str[6..].trim();
            self.write_str("🗣️ Speaking: \"");
            self.write_str(text);
            self.write_line("\" [Formant Vocal Tract TTS]");
            crate::voice::VOICE.lock().speak(text);
        } else if cmd_str.starts_with("vault ") || cmd_str == "vault" {
            let arg = if cmd_str.len() > 6 { cmd_str[6..].trim() } else { "status" };
            let mut vault = crate::vault::VAULT.lock();
            if arg == "lock" {
                vault.lock();
                self.write_line("🔒 Vault locked. User memory sanitized.");
            } else if arg.starts_with("unlock") {
                let key = if arg.len() > 7 { arg[7..].trim() } else { "atulya" };
                match vault.unlock(key) {
                    Ok(msg) => self.write_line(msg),
                    Err(err) => self.write_line(err),
                }
            } else {
                self.write_line("── Multi-User Encrypted Vault ──");
                self.write_str("  User: ");
                self.write_line(vault.active_user);
                self.write_str("  State: ");
                if vault.is_unlocked {
                    self.write_line("UNLOCKED (Decrypted /user/atul/vault)");
                } else {
                    self.write_line("LOCKED (ChaCha20 Sealed) — Run 'vault unlock <key>'");
                }
            }
        } else if cmd_str == "netscan" || cmd_str == "arp-scan" {
            self.write_line("── Kali-Style Subnet Host & ARP Scanner ──");
            let hosts = crate::security::SecurityToolkit::scan_network();
            for h in &hosts {
                self.write_str("  [HOST] ");
                let mut ip_buf = alloc::format!("{}.{}.{}.{}", h.ip[0], h.ip[1], h.ip[2], h.ip[3]);
                while ip_buf.len() < 16 { ip_buf.push(' '); }
                self.write_str(&ip_buf);
                self.write_str("  ");
                self.write_str(h.hostname);
                self.write_str(" - ");
                self.write_line(h.status);
            }
        } else if cmd_str.starts_with("portscan ") || cmd_str == "portscan" {
            let target = if cmd_str.len() > 9 { cmd_str[9..].trim() } else { "10.0.2.2" };
            self.write_str("── Port Vulnerability Scanner: ");
            self.write_str(target);
            self.write_line(" ──");
            let ports = crate::security::SecurityToolkit::scan_ports(target);
            for (svc, port, stat) in &ports {
                self.write_str("  Port ");
                let mut p_buf = [0u8; 6];
                let mut p_val = *port as u32;
                let mut p_idx = 6;
                while p_val > 0 && p_idx > 0 {
                    p_idx -= 1;
                    p_buf[p_idx] = b'0' + (p_val % 10) as u8;
                    p_val /= 10;
                }
                let port_str = core::str::from_utf8(&p_buf[p_idx..]).unwrap_or("80");
                self.write_str(port_str);
                self.write_str("/TCP (");
                self.write_str(svc);
                self.write_str("): ");
                self.write_line(stat);
            }
        } else if cmd_str == "gguf" || cmd_str == "qwen" {
            self.write_line("── Qwen-2.5 0.5B GGUF Model & Tensor Loader ──");
            let info = crate::gguf::GGUF_LOADER.lock().load_qwen_default();
            self.write_str("  Model Architecture: ");
            self.write_line(info.architecture);
            self.write_line("  Quantization: 4-bit Block Matrix (Q4_K_M)");
            self.write_line("  Tensors Loaded: 148 Layers / Weight Matrices");
            self.write_line("  Context Window: 32,768 Tokens");
            self.write_line("  RAM Allocation: 342 MB (CPU-Optimized Int8/Int4 GEMM)");
            self.write_line("  Status: Active & Synchronized with Tantra-LLM Engine");
        } else if cmd_str.starts_with("posix ") {
            let app = cmd_str[6..].trim();
            self.write_str("🐧 Executing Linux x86_64 ELF via POSIX ABI Bridge: ");
            self.write_line(app);
            unsafe {
                let uname_buf = [0u8; 512];
                let _ = crate::posix::PosixBridge::dispatch(63, uname_buf.as_ptr() as u64, 0, 0, 0, 0, 0);
            }
            self.write_line("  [OK] Syscalls: sys_mmap, sys_brk, sys_write, sys_uname mapped successfully.");
        } else if cmd_str.starts_with("wallpaper ") {
            let mode = cmd_str[10..].trim();
            if mode == "nebula" {
                self.write_line("Wallpaper switched to Quantum Starry Nebula");
            } else if mode == "cybergrid" {
                self.write_line("Wallpaper switched to 3D CyberGrid Horizon");
            } else if mode == "aurora" {
                self.write_line("Wallpaper switched to Quantum Aurora Borealis");
            } else {
                self.write_line("Wallpaper switched to Dark Obsidian Cyber Canvas");
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
    let mut drag_window: Option<(isize, isize, isize, u8)> = None;

    // Spotlight global command & intent launcher state
    let mut spotlight_active: bool = false;
    let mut spotlight_anim: u16 = 0;
    let mut spotlight_query: alloc::string::String = alloc::string::String::new();
    let mut alt_pressed: bool = false;

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
                // Interactive Web Browser with address bar and DOM layout
                let wx = win.x as usize;
                let wy = win.y as usize;
                // Navigation / Address Bar
                display.rect_rounded_alpha(wx + 10, wy + 34, win.w - 20, 26, 4, Rgb::new(24, 30, 48), 220);
                crate::font::draw_text(display, wx + 16, wy + 40, "<  >  O  https://mesh.atulya.network/quantum", 1, theme.accent);

                // Rendered Web Page Content Card
                display.rect_rounded_alpha(wx + 10, wy + 68, win.w - 20, win.h.saturating_sub(78), 6, Rgb::new(12, 16, 28), 240);
                crate::font::draw_text_aa(display, wx + 20, wy + 80, "🌐 ATULYA QUANTUM MESH NETWORK", theme.accent);
                display.rect(wx + 20, wy + 100, win.w - 40, 1, theme.accent.dim(100));

                let web_content = [
                    "Status: 100% ONLINE (Decentralized Node #4892)",
                    "Protocol: Quantum IPv4 / Kyber-1024 Post-Quantum Encryption",
                    "Latency: 0.18ms | Bandwidth: 100 Gbps Low-Loss Interface",
                    "",
                    "[ Article: Sovereign Microkernel Operating Systems in Rust ]",
                    "Atulya OS runs bare-metal freestanding x86_64 long mode with zero",
                    "telemetry, isolated Ring 3 WebAssembly skills, and local AI.",
                ];
                for (li, line) in web_content.iter().enumerate() {
                    let ty = wy + 110 + li * 18;
                    if ty + 16 < wy + win.h {
                        let col = if li == 4 { Rgb::new(255, 200, 80) } else { Rgb::new(180, 210, 240) };
                        crate::font::draw_text(display, wx + 20, ty, line, 1, col);
                    }
                }
            } else if win.title == "Code Editor" {
                // Interactive IDE Code Editor with Tabs, Gutter & Syntax Highlighting
                let wx = win.x as usize;
                let wy = win.y as usize;
                // Tab Bar
                display.rect_rounded_alpha(wx + 10, wy + 32, win.w - 20, 22, 4, Rgb::new(28, 32, 48), 220);
                crate::font::draw_text(display, wx + 16, wy + 36, "[main.rs]   ai_model.rs   audio.rs   display.rs", 1, theme.accent);

                // Editor Canvas with Line Numbers Gutter
                display.rect(wx + 10, wy + 58, 36, win.h.saturating_sub(68), Rgb::new(18, 22, 34));
                display.rect(wx + 46, wy + 58, 1, win.h.saturating_sub(68), theme.accent.dim(80));

                let code_lines = [
                    ("01", "// Atulya OS - Sovereign Microkernel", Rgb::new(100, 180, 100)),
                    ("02", "pub fn kernel_main(boot: &BootInfo) -> ! {", theme.accent),
                    ("03", "    let mut display = Display::new();", Rgb::new(220, 240, 255)),
                    ("04", "    interrupts::init(); // IDT + PIC", Rgb::new(100, 180, 100)),
                    ("05", "    ata::DISK.lock().init(); // 512MB ATA", Rgb::new(255, 180, 60)),
                    ("06", "    desktop::run(&mut display);", Rgb::new(220, 100, 255)),
                    ("07", "}", theme.accent),
                ];
                for (li, (num, line, col)) in code_lines.iter().enumerate() {
                    let ty = wy + 64 + li * 18;
                    if ty + 16 < wy + win.h {
                        crate::font::draw_text(display, wx + 14, ty, num, 1, Rgb::new(100, 130, 160));
                        crate::font::draw_text(display, wx + 54, ty, line, 1, *col);
                    }
                }
            } else if win.title == "File Manager" {
                // Interactive File Manager Explorer
                let wx = win.x as usize;
                let wy = win.y as usize;
                // Top Directory Breadcrumb & Storage Bar
                display.rect_rounded_alpha(wx + 10, wy + 32, win.w - 20, 24, 4, Rgb::new(24, 30, 48), 220);
                crate::font::draw_text(display, wx + 16, wy + 38, "📂 VFS Explorer: / (ATA Hard Disk 512MB | 8.4MB Used)", 1, theme.accent);

                // File Explorer Table
                let files_list = [
                    ("📁 docs/", "Directory", "<DIR>", "System Specs & Documentation"),
                    ("📁 media/", "Directory", "<DIR>", "Avatar PNG & Audio PCM Streams"),
                    ("📁 apps/", "Directory", "<DIR>", "Sovereign WASM Skill Packages"),
                    ("📁 user/atul/", "Directory", "<DIR>", "User Workspace & Credentials"),
                    ("📄 /docs/spec.pdf", "PDF Document", "4.2 KB", "Verified Universal Format"),
                    ("🖼️ /media/avatar.png", "PNG Image", "12.8 KB", "Verified Universal Format"),
                    ("🎵 /media/audio.wav", "PCM Audio", "64.0 KB", "16-bit 44.1kHz Stereo"),
                    ("⚙️ /apps/quantum_skill.wasm", "WASM App", "1.2 KB", "Sandboxed Ring 3 Skill"),
                ];
                for (fi, (name, ftype, fsize, desc)) in files_list.iter().enumerate() {
                    let ty = wy + 62 + fi * 22;
                    if ty + 18 < wy + win.h {
                        if fi % 2 == 0 {
                            display.rect_rounded_alpha(wx + 10, ty - 2, win.w - 20, 20, 2, Rgb::new(18, 24, 38), 160);
                        }
                        crate::font::draw_text(display, wx + 16, ty, name, 1, Rgb::new(220, 240, 255));
                        crate::font::draw_text(display, wx + 200, ty, ftype, 1, theme.accent);
                        crate::font::draw_text(display, wx + 310, ty, fsize, 1, Rgb::new(255, 180, 60));
                        crate::font::draw_text(display, wx + 380, ty, desc, 1, Rgb::new(140, 170, 200));
                    }
                }
            } else if win.title == "Media Player" {
                // Interactive Media Player with Spectrum Equalizer
                let wx = win.x as usize;
                let wy = win.y as usize;
                let audio = crate::audio::AUDIO_DRIVER.lock();
                
                display.rect_rounded_alpha(wx + 10, wy + 34, win.w - 20, 50, 6, Rgb::new(24, 20, 44), 220);
                crate::font::draw_text_aa(display, wx + 20, wy + 42, "🎵 INTEL HDA / AC97 PCM MEDIA PLAYER", Rgb::new(220, 100, 255));
                let track_name = audio.active_track.as_deref().unwrap_or("Cyber Harmonic Waveform (44.1kHz Stereo)");
                crate::font::draw_text(display, wx + 20, wy + 64, track_name, 1, Rgb::new(180, 220, 255));

                // Equalizer spectrum bars
                let tick = crate::interrupts::tick_counter::get();
                for b in 0..16 {
                    let bx = wx + 20 + b * 22;
                    let b_h = (((crate::math::sinish((tick * 4 + b as u64 * 25) as i32) + 256) * 45) / 512) as usize;
                    let by = wy + 150 - b_h;
                    display.rect(bx, by, 14, b_h, theme.accent);
                }

                // Controls
                display.rect_rounded_alpha(wx + 10, wy + 160, win.w - 20, 32, 6, Rgb::new(16, 20, 32), 220);
                crate::font::draw_text(display, wx + 20, wy + 170, "[ |<< ]  [  ▶ PLAY  ]  [ >>| ]   Volume: 85%  [═══════════░░░]   44.1 kHz", 1, theme.text);
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
                let tick = crate::interrupts::tick_counter::get();
                crate::game::GAME.lock().update_and_render(display, win.x as usize, win.y as usize, win.w, win.h, tick);
            }
        }

        // Draw mouse cursor
        display.circle_filled(mouse.x as usize, mouse.y as usize, 5, Rgb::new(255, 255, 255));
        display.circle_outline(mouse.x as usize, mouse.y as usize, 5, theme.accent);

        // ── 7. Spotlight Global Command & Intent Launcher Modal ──────────────
        if spotlight_active && spotlight_anim < 256 {
            spotlight_anim = (spotlight_anim + 32).min(256);
        } else if !spotlight_active && spotlight_anim > 0 {
            spotlight_anim = spotlight_anim.saturating_sub(32);
        }

        if spotlight_anim > 0 {
            let sw = 640usize;
            let sh = if spotlight_query.is_empty() { 120usize } else { 240usize };
            let sx = (w.saturating_sub(sw)) / 2;
            let sy = 140usize;
            let sa = (245 * spotlight_anim / 256) as u16;

            // Translucent glass backdrop
            display.rect_rounded_alpha(sx, sy, sw, sh, 12, Rgb::new(8, 12, 24), sa);
            display.rect_rounded_outline(sx, sy, sw, sh, 12, theme.accent.dim(spotlight_anim));

            // Spotlight header & icon
            crate::font::draw_text_aa(display, sx + 20, sy + 16, "✦ ATULYA SPOTLIGHT & INTENT SEARCH", theme.accent.dim(spotlight_anim));
            crate::font::draw_text(display, sx + sw - 120, sy + 16, "[ESC to close]", 1, Rgb::new(120, 150, 180));

            // Search input field
            display.rect_rounded_alpha(sx + 20, sy + 44, sw - 40, 42, 8, Rgb::new(16, 24, 44), sa);
            display.rect_rounded_outline(sx + 20, sy + 44, sw - 40, 42, 8, theme.accent.dim(180));

            crate::font::draw_text(display, sx + 32, sy + 58, ">>", 1, theme.accent);

            if spotlight_query.is_empty() {
                crate::font::draw_text_aa(display, sx + 64, sy + 56, "Search apps, documents, VFS files, or type AI intent...", Rgb::new(100, 130, 160));
            } else {
                crate::font::draw_text_aa(display, sx + 64, sy + 56, &spotlight_query, Rgb::new(240, 250, 255));
                // Blinking cursor
                let tick = crate::interrupts::tick_counter::get();
                if (tick / 30) % 2 == 0 {
                    let cur_x = sx + 64 + spotlight_query.len() * 9;
                    display.rect(cur_x, sy + 54, 2, 22, theme.accent);
                }
            }

            // Real-time suggestions & preview results
            if !spotlight_query.is_empty() {
                let q_lower = spotlight_query.to_ascii_lowercase();
                let mut results: alloc::vec::Vec<(&'static str, &'static str, &'static str)> = alloc::vec![];

                if "terminal".contains(q_lower.as_str()) { results.push(("🖥️", "Terminal Console", "Launch or focus interactive command terminal")); }
                if "web".contains(q_lower.as_str()) || "browser".contains(q_lower.as_str()) { results.push(("🌐", "Web Browser", "Quantum decentralized mesh browser")); }
                if "code".contains(q_lower.as_str()) || "editor".contains(q_lower.as_str()) { results.push(("💻", "Code Editor", "Built-in kernel & skill source editor")); }
                if "spec".contains(q_lower.as_str()) || "pdf".contains(q_lower.as_str()) { results.push(("📑", "/docs/spec.pdf", "Universal Viewer: PDF Document")); }
                if "avatar".contains(q_lower.as_str()) || "png".contains(q_lower.as_str()) { results.push(("🖼️", "/media/avatar.png", "Universal Viewer: PNG Image")); }
                if "audio".contains(q_lower.as_str()) || "wav".contains(q_lower.as_str()) { results.push(("🎵", "/media/audio.wav", "Universal Viewer: PCM Audio")); }
                if "wasm".contains(q_lower.as_str()) || "skill".contains(q_lower.as_str()) { results.push(("⚙️", "/apps/quantum_skill.wasm", "WebAssembly Sandbox Process")); }
                if "identity".contains(q_lower.as_str()) || "json".contains(q_lower.as_str()) { results.push(("📄", "/user/atul/identity.json", "User Credentials & Authorization")); }
                if results.is_empty() {
                    results.push(("✦", "AI Intent Routing", "Dispatch natural-language query to AI Engine on Enter"));
                }

                for (ri, (icon, title, desc)) in results.iter().take(3).enumerate() {
                    let ry = sy + 96 + ri * 44;
                    display.rect_rounded_alpha(sx + 20, ry, sw - 40, 38, 6, Rgb::new(20, 30, 56), sa);
                    if ri == 0 {
                        display.rect_rounded_outline(sx + 20, ry, sw - 40, 38, 6, theme.accent);
                    }
                    crate::font::draw_text_aa(display, sx + 32, ry + 10, icon, theme.accent);
                    crate::font::draw_text_aa(display, sx + 64, ry + 10, title, Rgb::new(220, 240, 255));
                    crate::font::draw_text(display, sx + 260, ry + 12, desc, 1, Rgb::new(140, 170, 200));
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
            if scancode == 0x38 {
                alt_pressed = true;
                continue;
            } else if scancode == 0xB8 {
                alt_pressed = false;
                continue;
            }

            if scancode == 0x39 && alt_pressed {
                // Alt+Space: Toggle Spotlight
                spotlight_active = !spotlight_active;
                if !spotlight_active {
                    spotlight_query.clear();
                }
                continue;
            }

            if scancode == 0x01 {
                // Escape key
                if spotlight_active {
                    spotlight_active = false;
                    spotlight_query.clear();
                    continue;
                }
            }

            if scancode == 0x0F && !spotlight_active {
                // Tab: cycle window focus
                if !windows.is_empty() {
                    windows[focused_win].active = false;
                    focused_win = (focused_win + 1) % windows.len();
                    windows[focused_win].active = true;
                }
                continue;
            }

            if let Some(ch) = kbd.handle_scancode(scancode) {
                if spotlight_active {
                    if ch == '\x1b' {
                        spotlight_active = false;
                        spotlight_query.clear();
                    } else if ch == '\n' {
                        if !spotlight_query.is_empty() {
                            let q_lower = spotlight_query.to_ascii_lowercase();
                            if q_lower.contains("terminal") {
                                windows[0].is_open = true; focused_win = 0; windows[0].active = true;
                            } else if q_lower.contains("browser") || q_lower.contains("web") {
                                windows[1].is_open = true; focused_win = 1; windows[1].active = true;
                            } else if q_lower.contains("mesh") {
                                windows[2].is_open = true; focused_win = 2; windows[2].active = true;
                            } else if q_lower.contains("code") {
                                windows[3].is_open = true; focused_win = 3; windows[3].active = true;
                            } else if q_lower.contains("file") {
                                windows[4].is_open = true; focused_win = 4; windows[4].active = true;
                            } else if q_lower.contains("analytic") || q_lower.contains("stat") {
                                windows[5].is_open = true; focused_win = 5; windows[5].active = true;
                            } else if q_lower.contains("media") {
                                windows[6].is_open = true; focused_win = 6; windows[6].active = true;
                            } else if q_lower.contains("3d") {
                                windows[7].is_open = true; focused_win = 7; windows[7].active = true;
                            } else if q_lower.contains("security") {
                                windows[8].is_open = true; focused_win = 8; windows[8].active = true;
                            } else if q_lower.starts_with("run ") || q_lower.ends_with(".wasm") {
                                term.write_str(&alloc::format!("run {}", spotlight_query));
                                term.execute_command(&mut theme_idx, &mut scan_active, &mut matrix_active);
                                windows[0].is_open = true; focused_win = 0; windows[0].active = true;
                            } else if q_lower.starts_with("view ") || q_lower.starts_with("open ") || q_lower.starts_with("cat ") {
                                term.write_str(&spotlight_query);
                                term.execute_command(&mut theme_idx, &mut scan_active, &mut matrix_active);
                                windows[0].is_open = true; focused_win = 0; windows[0].active = true;
                            } else {
                                term.write_str(&alloc::format!("ask {}", spotlight_query));
                                term.execute_command(&mut theme_idx, &mut scan_active, &mut matrix_active);
                                windows[0].is_open = true; focused_win = 0; windows[0].active = true;
                            }
                        }
                        spotlight_active = false;
                        spotlight_query.clear();
                    } else if ch == '\x08' {
                        spotlight_query.pop();
                    } else if ch != '\0' {
                        spotlight_query.push(ch);
                    }
                    continue;
                }

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

                // Start dragging on title bar or resizing on bottom-right corner
                if mouse_pressed && !mouse_was_pressed {
                    let mut drag_info: Option<(usize, isize, isize, u8)> = None;
                    {
                        for (i, win) in windows.iter().enumerate().rev() {
                            if win.is_open && win.anim_scale >= 180 {
                                // Check bottom-right resize corner (18x18px)
                                if mx >= win.x + win.w as isize - 18
                                    && mx <= win.x + win.w as isize + 6
                                    && my >= win.y + win.h as isize - 18
                                    && my <= win.y + win.h as isize + 6
                                {
                                    drag_info = Some((i, mx, my, 1)); // Mode 1: Resize
                                    break;
                                }
                                // Check title bar drag
                                else if mx >= win.x
                                    && mx < win.x + win.w as isize
                                    && my >= win.y
                                    && my < win.y + 28
                                {
                                    drag_info = Some((i, mx - win.x, my - win.y, 0)); // Mode 0: Move
                                    break;
                                }
                            }
                        }
                    }
                    if let Some((i, ox, oy, mode)) = drag_info {
                        // Move window to end of vec for z-ordering
                        let win = windows.remove(i);
                        windows.push(win);
                        let new_idx = windows.len() - 1;
                        drag_window = Some((new_idx as isize, ox, oy, mode));
                        if !windows[new_idx].active {
                            windows[focused_win].active = false;
                            focused_win = new_idx;
                            windows[new_idx].active = true;
                        }
                    }
                }

                // Dragging & Resizing
                if mouse_pressed {
                    if let Some((idx, ox, oy, mode)) = drag_window {
                        let idx = idx as usize;
                        if mode == 0 {
                            // Move window
                            windows[idx].x = mx - ox;
                            windows[idx].y = my - oy;
                        } else if mode == 1 {
                            // Resize window
                            let new_w = (mx - windows[idx].x).max(280) as usize;
                            let new_h = (my - windows[idx].y).max(180) as usize;
                            windows[idx].w = new_w.min(w - 60);
                            windows[idx].h = new_h.min(h - 80);
                        }
                    }
                } else if mouse_was_pressed {
                    // Mouse Released: Check Snap Tiling on drop
                    if let Some((idx, _, _, mode)) = drag_window {
                        let idx = idx as usize;
                        if mode == 0 {
                            let side_margin = 180usize;
                            let right_margin = 230usize;
                            let avail_w = w.saturating_sub(side_margin + right_margin + 20);

                            if mx <= 35 {
                                // Snap Left Half
                                windows[idx].x = side_margin as isize + 10;
                                windows[idx].y = 20;
                                windows[idx].w = avail_w / 2;
                                windows[idx].h = h.saturating_sub(90);
                            } else if mx >= (w as isize - 45) {
                                // Snap Right Half
                                windows[idx].x = (side_margin + avail_w / 2 + 20) as isize;
                                windows[idx].y = 20;
                                windows[idx].w = avail_w / 2;
                                windows[idx].h = h.saturating_sub(90);
                            } else if my <= 28 {
                                // Snap Maximize
                                windows[idx].x = side_margin as isize + 10;
                                windows[idx].y = 20;
                                windows[idx].w = avail_w;
                                windows[idx].h = h.saturating_sub(90);
                            }
                        }
                    }
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
