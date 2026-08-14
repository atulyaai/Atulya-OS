#!/usr/bin/env python3
"""Extract the 12 boot storyboard panels and write raw RGB binaries for the kernel."""

from __future__ import annotations

import math
import os
from pathlib import Path

from PIL import Image

ROOT = Path(r"F:\Atulya OS")
IMAGE_PATH = ROOT / "assets" / "images" / "boot_storyboard.png"
OUT_DIR = ROOT / "assets" / "boot_frames"
FRAME_W = 480
FRAME_H = 270
COLS = 4
ROWS = 3
# Bottom panel is the desktop mockup; top ~58% holds the 12 boot frames.
BOTTOM_FRAC = 0.42


def main() -> None:
    if not IMAGE_PATH.is_file():
        raise SystemExit(f"Missing storyboard image: {IMAGE_PATH}")

    img = Image.open(IMAGE_PATH).convert("RGB")
    w, h = img.size
    top_h = int(h * (1.0 - BOTTOM_FRAC))
    cell_w = w // COLS
    cell_h = top_h // ROWS

    OUT_DIR.mkdir(parents=True, exist_ok=True)

    index = 0
    for row in range(ROWS):
        for col in range(COLS):
            x0 = col * cell_w
            y0 = row * cell_h
            x1 = x0 + cell_w if col < COLS - 1 else w
            y1 = y0 + cell_h if row < ROWS - 1 else top_h
            panel = img.crop((x0, y0, x1, y1))
            panel = panel.resize((FRAME_W, FRAME_H), Image.Resampling.LANCZOS)
            out_path = OUT_DIR / f"frame_{index:02d}.rgb"
            panel.tobytes("raw", "RGB")
            out_path.write_bytes(panel.tobytes())
            print(f"wrote {out_path.name} ({FRAME_W}x{FRAME_H}) from crop ({x0},{y0})-({x1},{y1})")
            index += 1

    meta_path = OUT_DIR / "meta.txt"
    meta_path.write_text(f"width={FRAME_W}\nheight={FRAME_H}\nframes={index}\n", encoding="ascii")
    print(f"Done: {index} frames -> {OUT_DIR}")


if __name__ == "__main__":
    main()
