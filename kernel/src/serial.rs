pub fn serial_init() {
    unsafe {
        outb(0x3f8 + 1, 0x00);
        outb(0x3f8 + 3, 0x80);
        outb(0x3f8, 0x03);
        outb(0x3f8 + 1, 0x00);
        outb(0x3f8 + 3, 0x03);
        outb(0x3f8 + 2, 0xc7);
        outb(0x3f8 + 4, 0x0b);
    }
}

pub fn serial_write_str(text: &str) {
    for byte in text.bytes() {
        serial_write_byte(byte);
    }
}

pub fn serial_write_line(text: &str) {
    serial_write_str(text);
    serial_write_byte(b'\r');
    serial_write_byte(b'\n');
}

pub fn serial_write_byte(byte: u8) {
    unsafe {
        for _ in 0..10_000 {
            if inb(0x3f8 + 5) & 0x20 != 0 {
                outb(0x3f8, byte);
                return;
            }
        }
    }
}

pub fn serial_write_hex(val: u64) {
    let mut buf = [0u8; 18];
    buf[0] = b'0';
    buf[1] = b'x';
    let mut i = 2;
    let mut started = false;
    let hex = b"0123456789abcdef";
    let mut shift: i32 = 60;
    while shift >= 0 {
        let nib = ((val >> shift) & 0xF) as usize;
        if nib != 0 || started || shift == 0 {
            buf[i] = hex[nib];
            i += 1;
            started = true;
        }
        shift -= 4;
    }
    if i == 2 {
        buf[i] = b'0';
        i += 1;
    }
    serial_write_line(core::str::from_utf8(&buf[..i]).unwrap_or("0x?"));
}

unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") value);
}

unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!("in al, dx", out("al") value, in("dx") port);
    value
}
