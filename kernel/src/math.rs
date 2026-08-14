/// Quarter-wave sine lookup table, 0..90 degrees, scaled to 0..1024.
/// (sin(deg) * 1024, rounded)
const SIN_LUT: [i32; 91] = [
    0, 18, 36, 54, 71, 89, 107, 125, 143, 160, 178, 195, 213, 230, 248, 265,
    282, 299, 316, 333, 350, 367, 384, 400, 416, 433, 449, 465, 481, 496, 512,
    527, 543, 558, 573, 587, 602, 616, 630, 644, 658, 672, 685, 698, 711, 724,
    737, 749, 761, 773, 784, 796, 807, 818, 828, 839, 849, 859, 868, 878, 887,
    896, 904, 912, 920, 928, 935, 943, 949, 956, 962, 968, 974, 979, 984, 989,
    994, 998, 1002, 1005, 1008, 1011, 1014, 1016, 1018, 1020, 1022, 1023, 1023,
    1024, 1024,
];

/// True integer sine (via lookup table + quarter-wave symmetry).
/// Returns value in range -1024..1024. Input is degrees (0..360).
///
/// Previously this was a linear "triangle wave" approximation, which made
/// every rotating/orbiting element in the boot animation (particle orbits,
/// HUD dial ticks, pulse glow) move at a constant angular rate with visible
/// kinks at the 0/90/180/270 degree marks instead of the natural ease-in/
/// ease-out of real circular motion. That's a big part of why the animation
/// reads as mechanical/cheap rather than smooth.
pub fn sinish(deg: i32) -> isize {
    let mut d = deg % 360;
    if d < 0 {
        d += 360;
    }

    let (quadrant_sign, local) = match d {
        0..=90 => (1, d),
        91..=180 => (1, 180 - d),
        181..=270 => (-1, d - 180),
        _ => (-1, 360 - d),
    };

    quadrant_sign * SIN_LUT[local as usize] as isize
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
