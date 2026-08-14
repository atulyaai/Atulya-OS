/// Integer-approximation sine. Returns value in range -1024..1024.
/// Input is degrees (0..360).
pub fn sinish(deg: i32) -> isize {
    let mut d = deg % 360;
    if d < 0 {
        d += 360;
    }

    let sign = if d >= 180 { -1 } else { 1 };
    let local = if d >= 180 { d - 180 } else { d };
    let triangle = if local <= 90 { local } else { 180 - local };
    sign * ((triangle as isize * 1024) / 90)
}

/// Integer-approximation cosine. Returns value in range -1024..1024.
pub fn cosish(deg: i32) -> isize {
    sinish(deg + 90)
}

/// Busy-wait delay loop.
pub fn delay(cycles: usize) {
    for _ in 0..cycles {
        core::hint::spin_loop();
    }
}

/// Integer square root (floor).
pub fn isqrt(n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}
