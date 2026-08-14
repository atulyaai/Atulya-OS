use crate::display::{Display, Rgb};
use crate::fs::{RamFs, FileSystem, OpenFlags};

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
        self.bytes[self.cycle as usize] = b;
        self.cycle += 1;

        if self.cycle == 3 {
            self.cycle = 0;

            let flags = self.bytes[0];
            if flags & 0x08 == 0 {
                return false;
            }

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

            self.buttons = flags & 0x07;

            self.x = (self.x + dx).max(0).min(w as isize - 1);
            self.y = (self.y - dy).max(0).min(h as isize - 1);

            return true;
        }
        false
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
            self.write_line("          clear, neofetch, theme, joke, scan, matrix");
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
        } else if cmd_str.starts_with("cat ") {
            let file = cmd_str.get(4..).unwrap_or("").trim();
            let resolved = self.resolve_path(file);
            match self.fs.open(&resolved, OpenFlags::Read) {
                Ok(handle) => {
                    let mut buf = [0u8; 256];
                    loop {
                        match self.fs.read(handle, &mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
                                self.write_str(s);
                            }
                            Err(_) => break,
                        }
                    }
                    let _ = self.fs.close(handle);
                    self.new_line();
                }
                Err(_) => {
                    self.write_str("cat: ");
                    self.write_str(file);
                    self.write_line(": No such file");
                }
            }
        } else if cmd_str.starts_with("mkdir ") {
            let dir = cmd_str.get(6..).unwrap_or("").trim();
            let resolved = self.resolve_path(dir);
            match self.fs.mkdir(&resolved) {
                Ok(_) => self.write_line("Directory created"),
                Err(_) => self.write_line("mkdir: failed to create directory"),
            }
        } else if cmd_str.starts_with("rm ") {
            let file = cmd_str.get(3..).unwrap_or("").trim();
            let resolved = self.resolve_path(file);
            match self.fs.rm(&resolved) {
                Ok(_) => self.write_line("Removed"),
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
        Window { x: 120, y: 60, w: 560, h: 340, title: "Terminal", active: true },
        Window { x: 200, y: 100, w: 480, h: 300, title: "System Monitor", active: false },
        Window { x: -9999, y: -9999, w: 420, h: 260, title: "Files", active: false },
        Window { x: -9999, y: -9999, w: 380, h: 220, title: "Settings", active: false },
    ];

    let mut focused_win: usize = 0;

    let mut last_tick = crate::interrupts::tick_counter::get();

    let mut loop_count: u64 = 0;
    loop {
        loop_count += 1;
        let theme = &THEMES[theme_idx % 4];

        // Draw background gradient
        display.gradient_rect_v(0, 0, w, h, theme.wall_top, theme.wall_bot);

        // Draw taskbar
        display.rect_alpha(0, h - TASKBAR_HEIGHT, w, TASKBAR_HEIGHT, theme.accent, 40);
        display.rect(0, h - TASKBAR_HEIGHT, w, 2, theme.accent);

        // Dock icons
        let dock_icons = ["Terminal", "Monitor", "Files", "Settings"];
        let icon_w: usize = 60;
        let icon_h: usize = 30;
        let dock_start_x = (w / 2) - (dock_icons.len() * (icon_w + 10)) / 2;
        for (i, name) in dock_icons.iter().enumerate() {
            let ix = dock_start_x + i * (icon_w + 10);
            let iy = h - TASKBAR_HEIGHT + 10;
            let bg = if i == 0 { theme.accent } else { theme.win_inactive };
            display.rect_rounded_alpha(ix, iy, icon_w, icon_h, 6, bg, 180);
            let tx = ix + 6;
            let ty = iy + 10;
            for (ci, ch) in name.bytes().enumerate() {
                if tx + ci * 8 < w && ty < h {
                    crate::font::draw_char(display, tx + ci * 8, ty, ch, 1, theme.text);
                }
            }
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
        let cx = (w as isize - 90) as usize;
        let cy = h - 35;
        for (i, &ch) in time_str.iter().enumerate() {
            crate::font::draw_char(display, cx + i * 8, cy, ch, 1, theme.text);
        }

        // Draw windows (back to front)
        for (i, win) in windows.iter().enumerate() {
            let border_color = if i == focused_win || win.active {
                theme.win_active
            } else {
                theme.win_inactive
            };

            // Window shadow
            display.rect_rounded_alpha(
                win.x as usize + 4,
                win.y as usize + 4,
                win.w,
                win.h,
                8,
                Rgb::new(0, 0, 0),
                60,
            );

            // Window body
            display.rect_rounded_alpha(
                win.x as usize,
                win.y as usize,
                win.w,
                win.h,
                8,
                Rgb::new(16, 16, 20),
                230,
            );
            display.rect_rounded_outline(win.x as usize, win.y as usize, win.w, win.h, 8, border_color);

            // Title bar
            display.rect_rounded_alpha(
                win.x as usize,
                win.y as usize,
                win.w,
                28,
                6,
                Rgb::new(24, 24, 30),
                240,
            );

            // Title text
            for (ci, ch) in win.title.bytes().enumerate() {
                let tx = win.x as usize + 30 + ci * 8;
                if tx < win.x as usize + win.w {
                    crate::font::draw_char(display, tx, win.y as usize + 8, ch, 1, theme.text);
                }
            }

            // Close button (red circle)
            let cx_btn = win.x as usize + win.w - 18;
            let cy_btn = win.y as usize + 14;
            display.circle_filled(cx_btn, cy_btn, 6, Rgb::new(255, 80, 80));
            display.circle_outline(cx_btn, cy_btn, 6, Rgb::new(200, 50, 50));

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
            } else if win.title == "System Monitor" {
                let tick = crate::interrupts::tick_counter::get();
                let _uptime_secs = tick / 100;
                let info_lines = [
                    "── System Monitor ──",
                    "",
                    "  AtulyaOS Kernel v0.3",
                    "  Architecture: x86_64",
                    "",
                    "  Timer ticks: ",
                    "  Uptime:      ",
                    "  Heap: Active",
                    "  Interrupts:  Enabled",
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
                // Dynamic info: tick count
                let tick_str = {
                    let mut buf = [0u8; 20];
                    let mut val = tick;
                    let mut idx = 20;
                    if val == 0 {
                        buf[19] = b'0';
                        idx = 19;
                    } else {
                        while val > 0 && idx > 0 {
                            idx -= 1;
                            buf[idx] = b'0' + (val % 10) as u8;
                            val /= 10;
                        }
                    }
                    let mut out = [0u8; 20];
                    let len = 20 - idx;
                    out[..len].copy_from_slice(&buf[idx..]);
                    // Prepend "  Ticks: "
                    let prefix = b"  Ticks: ";
                    let mut result = [0u8; 40];
                    let mut ri = 0;
                    for &b in prefix.iter() {
                        result[ri] = b;
                        ri += 1;
                    }
                    for &b in &out[..len] {
                        result[ri] = b;
                        ri += 1;
                    }
                    result
                };
                for ci in 0..tick_str.len().min(30) {
                    let tx = win.x as usize + 12 + ci * 8;
                    let ty = win.y as usize + 36 + 5 * 18;
                    if tx + 8 < win.x as usize + win.w && ty + 16 < win.y as usize + win.h {
                        crate::font::draw_char(display, tx, ty, tick_str[ci], 1, theme.text);
                    }
                }
            } else if win.title == "Files" {
                let lines = [
                    "── Files ──",
                    "",
                    "  Use the Terminal for",
                    "  filesystem operations.",
                    "",
                    "  Commands: ls, cat, mkdir,",
                    "  rm, echo, cd, pwd",
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
            } else if win.title == "Settings" {
                let theme_names = ["Cyberpunk Cyan", "Matrix Green", "macOS Obsidian", "Retro Gold"];
                let current = theme_names[theme_idx % 4];
                let lines = [
                    "── Settings ──",
                    "",
                    "  Theme: ",
                    "",
                    "  Type 'theme' in",
                    "  Terminal to cycle.",
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
                // Draw current theme name
                for (ci, ch) in current.bytes().enumerate() {
                    let tx = win.x as usize + 12 + 8 * 8;
                    if tx + 8 < win.x as usize + win.w {
                        crate::font::draw_char(display, tx + ci * 8, win.y as usize + 36 + 2 * 18, ch, 1, theme.accent);
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
                    windows[i].x = -9999;
                    windows[i].y = -9999;
                }

                // Start dragging on title bar
                if mouse_pressed && !mouse_was_pressed {
                    let mut drag_info: Option<(usize, isize, isize)> = None;
                    {
                        for (i, win) in windows.iter().enumerate().rev() {
                            if mx >= win.x
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

                // Dock icon restore
                if mouse_pressed && !mouse_was_pressed && my >= h as isize - TASKBAR_HEIGHT as isize {
                    let dock_icons_count = 4usize;
                    let icon_w_i: isize = 60;
                    let dock_start_x_i = (w / 2) as isize - (dock_icons_count as isize * (icon_w_i + 10)) / 2;
                    for i in 0..dock_icons_count {
                        let ix = dock_start_x_i + i as isize * (icon_w_i + 10);
                        let iy = h as isize - TASKBAR_HEIGHT as isize + 10;
                        if mx >= ix && mx < ix + icon_w_i && my >= iy && my < iy + 30 {
                            if i < windows.len() && windows[i].x < -1000 {
                                windows[i].x = 100 + i as isize * 40;
                                windows[i].y = 60 + i as isize * 30;
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
