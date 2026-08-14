#!/usr/bin/env python3
"""
generate_boot_storyboard.py
============================
Procedurally render the 12-panel AtulyaOS boot storyboard.

Why procedural?
---------------
Diffusion / image generators cannot reproduce the exact same set of
panels on demand - you get a slightly different orb, ring, or HUD
every time. Rendering them with PIL + numpy gives a deterministic,
pixel-identical result every run, and the art is fully owned by the
repo (a few constants at the top of this file).

Pipeline
--------
1. Run this script            -> assets/images/boot_storyboard.png
2. Run convert_boot_frames.py -> assets/boot_frames/frame_*.rgb
3. Rebuild the kernel         -> boot splash plays the storyboard.

Grid layout (top 58% of the canvas, 4 columns x 3 rows)
-------------------------------------------------------
        col 0      col 1      col 2      col 3
row 0   00%        12%        25%        40%
row 1   55%        65%        75%        85%
row 2   100%       KNOWLEDGE  HELLO ATUL PREPARING
                    GRAPH

The bottom 42% of the canvas is left for the desktop mockup and is
not touched by this script (it stays whatever was there before).
"""

from __future__ import annotations

import math
import os
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

# ----------------------------------------------------------------------------
# Paths
# ----------------------------------------------------------------------------
ROOT = Path(r"F:\Atulya OS")
OUT = ROOT / "assets" / "images" / "boot_storyboard.png"

# ----------------------------------------------------------------------------
# Canvas
# ----------------------------------------------------------------------------
CANVAS_W = 1024
CANVAS_H = 819
BOTTOM_FRAC = 0.42          # matches convert_boot_frames.py
TOP_H = int(CANVAS_H * (1.0 - BOTTOM_FRAC))   # 475
COLS, ROWS = 4, 3
CELL_W = CANVAS_W // COLS                       # 256
CELL_H = TOP_H // ROWS                          # 158

# ----------------------------------------------------------------------------
# Palette
# ----------------------------------------------------------------------------
BG_OUTER = (3, 5, 14)
BG_INNER = (12, 18, 42)
CYAN     = (96, 220, 255)
CYAN_DIM = (50, 130, 200)
MAGENTA  = (200, 90, 255)
MAGENTA_D = (130, 55, 200)
ORANGE   = (255, 150, 70)
WHITE    = (240, 248, 255)
DIM      = (130, 150, 200)

# Per-HUD label color
MEM_C = MAGENTA
VOI_C = CYAN
VIS_C = CYAN
SKI_C = ORANGE


# ----------------------------------------------------------------------------
# Fonts (Windows). Falls back to PIL default if none of these exist.
# ----------------------------------------------------------------------------
def _find_font(bold: bool) -> str | None:
    candidates = [
        (r"C:\Windows\Fonts\seguisb.ttf" if bold else r"C:\Windows\Fonts\segoeui.ttf"),
        (r"C:\Windows\Fonts\arialbd.ttf"  if bold else r"C:\Windows\Fonts\arial.ttf"),
        (r"C:\Windows\Fonts\verdanab.ttf" if bold else r"C:\Windows\Fonts\verdana.ttf"),
    ]
    for c in candidates:
        if os.path.isfile(c):
            return c
    return None


def font(size: int, bold: bool = False) -> ImageFont.ImageFont:
    p = _find_font(bold)
    if p is None:
        return ImageFont.load_default()
    return ImageFont.truetype(p, size)


# ============================================================================
# Drawing primitives
# ============================================================================

def radial_bg(w: int, h: int, inner: tuple, outer: tuple,
              cx: float | None = None, cy: float | None = None,
              gamma: float = 1.6) -> Image.Image:
    arr = np.zeros((h, w, 3), dtype=np.float32)
    cx = w / 2 if cx is None else cx
    cy = h / 2 if cy is None else cy
    y, x = np.mgrid[0:h, 0:w]
    dx = x - cx
    dy = y - cy
    r = np.sqrt(dx * dx + dy * dy)
    r_max = math.sqrt(cx * cx + cy * cy)
    t = np.clip(r / r_max, 0, 1) ** gamma
    for c, (i, o) in enumerate(zip(inner, outer)):
        arr[..., c] = i * (1 - t) + o * t
    return Image.fromarray(arr.clip(0, 255).astype(np.uint8))


def plasma_color(t: np.ndarray) -> tuple:
    """Cyan -> magenta -> orange -> cyan color cycle. t in 0..1."""
    cr = np.where(t < 0.33,
                  CYAN[0] + (MAGENTA[0] - CYAN[0]) * (t / 0.33),
                  np.where(t < 0.66,
                           MAGENTA[0] + (ORANGE[0] - MAGENTA[0]) * ((t - 0.33) / 0.33),
                           ORANGE[0] + (CYAN[0] - ORANGE[0]) * ((t - 0.66) / 0.34)))
    cg = np.where(t < 0.33,
                  CYAN[1] + (MAGENTA[1] - CYAN[1]) * (t / 0.33),
                  np.where(t < 0.66,
                           MAGENTA[1] + (ORANGE[1] - MAGENTA[1]) * ((t - 0.33) / 0.33),
                           ORANGE[1] + (CYAN[1] - ORANGE[1]) * ((t - 0.66) / 0.34)))
    cb = np.where(t < 0.33,
                  CYAN[2] + (MAGENTA[2] - CYAN[2]) * (t / 0.33),
                  np.where(t < 0.66,
                           MAGENTA[2] + (ORANGE[2] - MAGENTA[2]) * ((t - 0.33) / 0.33),
                           ORANGE[2] + (CYAN[2] - ORANGE[2]) * ((t - 0.66) / 0.34)))
    return cr, cg, cb


def plasma_ring(canvas: Image.Image, cx: float, cy: float, base_r: float,
                petals: int = 4, phase: float = 0.0,
                squash: float = 0.78, thickness: float = 5.0,
                color_phase: float = 0.0, intensity: float = 1.0) -> Image.Image:
    """A 4-petal plasma ring centered on (cx, cy)."""
    arr = np.array(canvas).astype(np.float32)
    h, w, _ = arr.shape
    y, x = np.mgrid[0:h, 0:w]
    dx = x - cx
    dy = (y - cy) * squash
    r = np.sqrt(dx * dx + dy * dy)
    theta = np.arctan2(dy, dx)

    petal = 1.0 + 0.16 * np.cos(petals * theta + phase)
    target = base_r * petal
    profile = np.exp(-((r - target) ** 2) / (2 * thickness * thickness))

    t = ((theta + color_phase) / (2 * math.pi)) % 1.0
    cr, cg, cb = plasma_color(t)

    prof = profile * intensity
    arr[..., 0] = arr[..., 0] * (1 - prof) + cr * prof
    arr[..., 1] = arr[..., 1] * (1 - prof) + cg * prof
    arr[..., 2] = arr[..., 2] * (1 - prof) + cb * prof
    return Image.fromarray(arr.clip(0, 255).astype(np.uint8))


def core_glow(canvas: Image.Image, cx: float, cy: float, radius: float,
              color: tuple = (255, 255, 255), halo: tuple = CYAN,
              halo_strength: float = 0.35) -> Image.Image:
    arr = np.array(canvas).astype(np.float32)
    h, w, _ = arr.shape
    y, x = np.mgrid[0:h, 0:w]
    dx = x - cx
    dy = y - cy
    r2 = dx * dx + dy * dy
    core = np.exp(-r2 / (2 * (radius * 0.35) ** 2))
    halo_p = np.exp(-r2 / (2 * (radius * 0.95) ** 2))
    for c, v in enumerate(color):
        arr[..., c] = np.clip(arr[..., c] + core * v, 0, 255)
    for c, v in enumerate(halo):
        arr[..., c] = np.clip(arr[..., c] + halo_p * v * halo_strength, 0, 255)
    return Image.fromarray(arr.clip(0, 255).astype(np.uint8))


def dotted_ring(canvas: Image.Image, cx: float, cy: float, radius: float,
                color: tuple, n_dots: int = 64, dot_r: float = 0.7,
                squash: float = 0.85, alpha: float = 1.0) -> Image.Image:
    arr = np.array(canvas).astype(np.float32)
    h, w, _ = arr.shape
    for i in range(n_dots):
        a = 2 * math.pi * i / n_dots
        x = cx + radius * math.cos(a)
        y = cy + radius * math.sin(a) * squash
        if 0 <= x < w and 0 <= y < h:
            ix, iy = int(round(x)), int(round(y))
            for dy in range(-1, 2):
                for dx in range(-1, 2):
                    if dx * dx + dy * dy <= 1:
                        ny, nx = iy + dy, ix + dx
                        if 0 <= ny < h and 0 <= nx < w:
                            arr[ny, nx] = arr[ny, nx] * (1 - alpha) + np.array(color) * alpha
    return Image.fromarray(arr.clip(0, 255).astype(np.uint8))


def draw_arc(canvas: Image.Image, cx: float, cy: float, radius: float,
             start: float, end: float, color: tuple,
             width: int = 1, squash: float = 0.85) -> Image.Image:
    arr = np.array(canvas)
    h, w, _ = arr.shape
    n = max(8, int(abs(end - start) * radius))
    for i in range(n + 1):
        a = start + (end - start) * i / n
        x = cx + radius * math.cos(a)
        y = cy + radius * math.sin(a) * squash
        if 0 <= x < w and 0 <= y < h:
            for dy in range(-width // 2, width // 2 + 1):
                for dx in range(-width // 2, width // 2 + 1):
                    if dx * dx + dy * dy <= (width / 2) ** 2:
                        ny, nx = int(y) + dy, int(x) + dx
                        if 0 <= ny < h and 0 <= nx < w:
                            arr[ny, nx] = color
    return Image.fromarray(arr)


def draw_hud_rings(canvas: Image.Image, cx: float, cy: float) -> Image.Image:
    """Concentric dotted HUD rings + a few short bright arcs."""
    for r, n, c, a in [
        (78, 96, CYAN_DIM,   0.45),
        (66, 80, MAGENTA_D,  0.35),
        (54, 64, CYAN,       0.55),
        (44, 48, CYAN,       0.65),
    ]:
        canvas = dotted_ring(canvas, cx, cy, r, c, n_dots=n, dot_r=0.7, alpha=a)
    # bright short arcs for visual interest
    canvas = draw_arc(canvas, cx, cy, 72, math.radians(15),  math.radians(60),  CYAN,    width=1)
    canvas = draw_arc(canvas, cx, cy, 72, math.radians(200), math.radians(245), MAGENTA, width=1)
    canvas = draw_arc(canvas, cx, cy, 50, math.radians(-25), math.radians(25),  CYAN,    width=1)
    canvas = draw_arc(canvas, cx, cy, 60, math.radians(105), math.radians(135), MAGENTA, width=1)
    return canvas


def wordmark(canvas: Image.Image, cx: float, cy: float,
             text: str = "ATULYA", sub: str = "OS",
             size: int = 17, sub_size: int = 10) -> Image.Image:
    img = canvas.copy()
    f1 = font(size, bold=True)
    f2 = font(sub_size, bold=True)
    draw = ImageDraw.Draw(img)
    bbox = draw.textbbox((0, 0), text, font=f1)
    tw = bbox[2] - bbox[0]
    th = bbox[3] - bbox[1]
    x0 = cx - tw / 2
    y0 = cy - th - 2
    # glow pass
    glow = Image.new("RGB", img.size, (0, 0, 0))
    gd = ImageDraw.Draw(glow)
    gd.text((x0, y0), text, font=f1, fill=(120, 200, 255))
    glow = glow.filter(ImageFilter.GaussianBlur(radius=2.0))
    glow = Image.eval(glow, lambda v: min(255, int(v * 1.6)))
    img = Image.blend(img, glow.convert("RGB"), 0.65)
    draw = ImageDraw.Draw(img)
    draw.text((x0, y0), text, font=f1, fill=WHITE)
    # OS sub
    bbox2 = draw.textbbox((0, 0), sub, font=f2)
    tw2 = bbox2[2] - bbox2[0]
    draw.text((cx - tw2 / 2, cy + 4), sub, font=f2, fill=CYAN)
    return img


def bottom_text(canvas: Image.Image, cx: float, cy: float,
                percent: str, status: str) -> Image.Image:
    img = canvas.copy()
    draw = ImageDraw.Draw(img)
    fp = font(15, bold=True)
    fs = font(9, bold=True)
    bbox = draw.textbbox((0, 0), percent, font=fp)
    tw = bbox[2] - bbox[0]
    draw.text((cx - tw / 2, cy - 14), percent, font=fp, fill=WHITE)
    # letter-spaced status
    spaced = "  ".join(list(status))
    bbox2 = draw.textbbox((0, 0), spaced, font=fs)
    tw2 = bbox2[2] - bbox2[0]
    draw.text((cx - tw2 / 2, cy + 6), spaced, font=fs, fill=CYAN)
    return img


# ----------------------------------------------------------------------------
# HUD labels (Memory / Voice / Vision / Skills)
# ----------------------------------------------------------------------------

def _icon_memory(draw: ImageDraw.ImageDraw, x: int, y: int, c: tuple) -> None:
    """4 vertical bars."""
    for i, h in enumerate([6, 9, 11, 7]):
        bx = x + i * 4
        draw.rectangle([bx, y - h, bx + 2, y], outline=c, width=1)


def _icon_voice(draw: ImageDraw.ImageDraw, x: int, y: int, c: tuple) -> None:
    """3 vertical bars of different heights (waveform)."""
    for i, h in enumerate([5, 10, 6]):
        bx = x + i * 4
        draw.rectangle([bx, y - h, bx + 2, y], outline=c, width=1)


def _icon_vision(draw: ImageDraw.ImageDraw, x: int, y: int, c: tuple) -> None:
    """Eye: ellipse + pupil."""
    draw.ellipse([x - 6, y - 4, x + 6, y + 4], outline=c, width=1)
    draw.ellipse([x - 2, y - 2, x + 2, y + 2], fill=c)


def _icon_skills(draw: ImageDraw.ImageDraw, x: int, y: int, c: tuple) -> None:
    """Atom: central dot + 3 thin ellipses at angles."""
    draw.ellipse([x - 1, y - 1, x + 1, y + 1], fill=c)
    for ang in (0, 60, 120):
        a = math.radians(ang)
        # approximate rotated ellipse as a small line
        dx = 7 * math.cos(a)
        dy = 4 * math.sin(a)
        draw.line([(x - dx, y - dy), (x + dx, y + dy)], fill=c, width=1)


def hud_label(canvas: Image.Image, x: int, y: int, color: tuple,
              label: str, status: str, icon: str) -> Image.Image:
    """Draw a HUD block at (x, y) with the given label, status, icon."""
    img = canvas.copy()
    draw = ImageDraw.Draw(img)
    f_label = font(10, bold=True)
    f_status = font(8, bold=False)
    # icon
    if icon == "memory":
        _icon_memory(draw, x, y, color)
    elif icon == "voice":
        _icon_voice(draw, x, y, color)
    elif icon == "vision":
        _icon_vision(draw, x, y, color)
    elif icon == "skills":
        _icon_skills(draw, x, y, color)
    # label text (right of icon)
    draw.text((x + 16, y - 10), label, font=f_label, fill=color)
    # status (below)
    draw.text((x + 16, y - 1), status, font=f_status, fill=color)
    return img


# ----------------------------------------------------------------------------
# Per-panel renderers
# ----------------------------------------------------------------------------

def panel_background() -> Image.Image:
    return radial_bg(CELL_W, CELL_H, BG_INNER, BG_OUTER, gamma=1.4)


def render_orb(canvas: Image.Image, cx: float, cy: float,
               intensity: float = 1.0, with_hud_rings: bool = True) -> Image.Image:
    """Draw the plasma orb centered on (cx, cy) at full strength."""
    out = canvas
    # outer faint plasma
    out = plasma_ring(out, cx, cy, base_r=32, petals=4, phase=0.0,
                      thickness=4.5, squash=0.78, intensity=0.55 * intensity)
    # main ring
    out = plasma_ring(out, cx, cy, base_r=24, petals=4, phase=math.pi / 4,
                      thickness=3.2, squash=0.78, intensity=0.95 * intensity)
    # bright core
    out = core_glow(out, cx, cy, radius=8, color=(255, 255, 255), halo=CYAN, halo_strength=0.5)
    if with_hud_rings:
        out = draw_hud_rings(out, cx, cy)
    return out


def render_panel_00(canvas: Image.Image) -> Image.Image:
    """0% SYSTEM POWER ON - just a tiny star."""
    arr = np.array(canvas).astype(np.float32)
    h, w, _ = arr.shape
    y, x = np.mgrid[0:h, 0:w]
    dx = x - w / 2
    dy = y - h / 2 - 12
    r2 = dx * dx + dy * dy
    star = np.exp(-r2 / 2.0)
    halo = np.exp(-r2 / 80.0) * 0.6
    arr[..., 0] = np.clip(arr[..., 0] + star * 255 + halo * 40, 0, 255)
    arr[..., 1] = np.clip(arr[..., 1] + star * 255 + halo * 100, 0, 255)
    arr[..., 2] = np.clip(arr[..., 2] + star * 255 + halo * 220, 0, 255)
    img = Image.fromarray(arr.clip(0, 255).astype(np.uint8))
    return bottom_text(img, w / 2, h - 32, "00%", "SYSTEM POWER ON")


def render_panel_01(canvas: Image.Image) -> Image.Image:
    """12% ENERGY INITIALIZING - orb appears, no rings."""
    cx, cy = CELL_W / 2, CELL_H / 2 - 6
    out = canvas
    out = plasma_ring(out, cx, cy, base_r=30, petals=4, phase=0.0,
                      thickness=5.0, squash=0.78, intensity=0.55)
    out = plasma_ring(out, cx, cy, base_r=22, petals=4, phase=math.pi / 4,
                      thickness=3.5, squash=0.78, intensity=0.95)
    out = core_glow(out, cx, cy, radius=7, color=(255, 255, 255), halo=CYAN, halo_strength=0.5)
    return bottom_text(out, CELL_W / 2, CELL_H - 32, "12%", "ENERGY INITIALIZING")


def render_panel_02(canvas: Image.Image) -> Image.Image:
    """25% CORE FORMATION - orb + faint rings."""
    cx, cy = CELL_W / 2, CELL_H / 2 - 6
    out = render_orb(canvas, cx, cy, intensity=0.85, with_hud_rings=False)
    # soft rings only
    for r, n, c, a in [
        (60, 72, CYAN_DIM, 0.4),
        (48, 56, CYAN,     0.5),
    ]:
        out = dotted_ring(out, cx, cy, r, c, n_dots=n, dot_r=0.6, alpha=a)
    return bottom_text(out, CELL_W / 2, CELL_H - 32, "25%", "CORE FORMATION")


def render_panel_03(canvas: Image.Image) -> Image.Image:
    """40% ATULYA CORE ONLINE - orb + rings + wordmark."""
    cx, cy = CELL_W / 2, CELL_H / 2 - 6
    out = render_orb(canvas, cx, cy, intensity=1.0, with_hud_rings=True)
    out = wordmark(out, cx, cy)
    return bottom_text(out, CELL_W / 2, CELL_H - 32, "40%", "ATULYA CORE ONLINE")


def render_panel_04(canvas: Image.Image) -> Image.Image:
    """55% MEMORY ONLINE."""
    cx, cy = CELL_W / 2, CELL_H / 2 - 6
    out = render_orb(canvas, cx, cy, with_hud_rings=True)
    out = wordmark(out, cx, cy)
    out = hud_label(out, 20, 36, MEM_C, "MEMORY", "Online", "memory")
    return bottom_text(out, CELL_W / 2, CELL_H - 32, "55%", "MEMORY ONLINE")


def render_panel_05(canvas: Image.Image) -> Image.Image:
    """65% VOICE ONLINE."""
    cx, cy = CELL_W / 2, CELL_H / 2 - 6
    out = render_orb(canvas, cx, cy, with_hud_rings=True)
    out = wordmark(out, cx, cy)
    out = hud_label(out, 20, 36, MEM_C, "MEMORY", "Online", "memory")
    out = hud_label(out, CELL_W - 60, 36, VOI_C, "VOICE", "Online", "voice")
    return bottom_text(out, CELL_W / 2, CELL_H - 32, "65%", "VOICE ONLINE")


def render_panel_06(canvas: Image.Image) -> Image.Image:
    """75% VISION ONLINE."""
    cx, cy = CELL_W / 2, CELL_H / 2 - 6
    out = render_orb(canvas, cx, cy, with_hud_rings=True)
    out = wordmark(out, cx, cy)
    out = hud_label(out, 20, 36,  MEM_C, "MEMORY", "Online",  "memory")
    out = hud_label(out, 20, 72,  VOI_C, "VOICE",  "Online",  "voice")
    out = hud_label(out, CELL_W - 60, 36, VIS_C, "VISION", "Online", "vision")
    return bottom_text(out, CELL_W / 2, CELL_H - 32, "75%", "VISION ONLINE")


def render_panel_07(canvas: Image.Image) -> Image.Image:
    """85% SKILLS ONLINE - SKILLS still Initializing on this frame."""
    cx, cy = CELL_W / 2, CELL_H / 2 - 6
    out = render_orb(canvas, cx, cy, with_hud_rings=True)
    out = wordmark(out, cx, cy)
    out = hud_label(out, 20, 36,         MEM_C, "MEMORY", "Online",      "memory")
    out = hud_label(out, 20, 72,         VOI_C, "VOICE",  "Online",      "voice")
    out = hud_label(out, CELL_W - 60, 36, VIS_C, "VISION", "Online",      "vision")
    out = hud_label(out, CELL_W - 60, 72, SKI_C, "SKILLS", "Initializing", "skills")
    return bottom_text(out, CELL_W / 2, CELL_H - 32, "85%", "SKILLS ONLINE")


def render_panel_08(canvas: Image.Image) -> Image.Image:
    """100% ALL SYSTEMS ONLINE - all 4 HUDs Online."""
    cx, cy = CELL_W / 2, CELL_H / 2 - 6
    out = render_orb(canvas, cx, cy, with_hud_rings=True)
    out = wordmark(out, cx, cy)
    out = hud_label(out, 20, 36,         MEM_C, "MEMORY", "Online", "memory")
    out = hud_label(out, 20, 72,         VOI_C, "VOICE",  "Online", "voice")
    out = hud_label(out, CELL_W - 60, 36, VIS_C, "VISION", "Online", "vision")
    out = hud_label(out, CELL_W - 60, 72, SKI_C, "SKILLS", "Online", "skills")
    return bottom_text(out, CELL_W / 2, CELL_H - 32, "100%", "ALL SYSTEMS ONLINE")


def render_panel_09(canvas: Image.Image) -> Image.Image:
    """KNOWLEDGE GRAPH ACTIVATED - network of nodes + small orb."""
    out = canvas.copy()
    arr = np.array(out).astype(np.float32)
    h, w, _ = arr.shape
    cx, cy = CELL_W / 2, CELL_H / 2 - 6

    # background brighter in middle
    y, x = np.mgrid[0:h, 0:w]
    dx = x - cx
    dy = y - cy
    r2 = dx * dx + dy * dy
    bg = np.exp(-r2 / 2500.0) * 18
    arr[..., 0] = np.clip(arr[..., 0] + bg * 0.4, 0, 255)
    arr[..., 1] = np.clip(arr[..., 1] + bg * 0.3, 0, 255)
    arr[..., 2] = np.clip(arr[..., 2] + bg, 0, 255)
    out = Image.fromarray(arr.clip(0, 255).astype(np.uint8))

    # knowledge graph: ~22 nodes
    rng = np.random.default_rng(seed=91)
    nodes = []
    for _ in range(22):
        nx = rng.uniform(12, CELL_W - 12)
        ny = rng.uniform(12, CELL_H - 30)
        nodes.append((nx, ny))

    # ensure 1-2 nodes near the center for visual focus
    nodes.append((cx - 12, cy - 6))
    nodes.append((cx + 14, cy + 8))

    draw_img = out.copy()
    draw = ImageDraw.Draw(draw_img)
    # edges first
    for i, (x1, y1) in enumerate(nodes):
        # connect to 2-3 nearest
        dists = sorted(
            [(math.hypot(x1 - x2, y1 - y2), j) for j, (x2, y2) in enumerate(nodes) if j != i]
        )
        for d, j in dists[:2]:
            if d < 70:
                x2, y2 = nodes[j]
                # color fades by distance
                t = max(0, 1 - d / 70)
                col = (
                    int(60 + 140 * t),
                    int(60 + 100 * t),
                    int(120 + 135 * t),
                )
                draw.line([(x1, y1), (x2, y2)], fill=col, width=1)
    # nodes
    for i, (x, y) in enumerate(nodes):
        if i >= 22:
            # center focus node - bigger, brighter
            for r, c, a in [(5, WHITE, 1.0), (3, CYAN, 1.0)]:
                draw.ellipse([x - r, y - r, x + r, y + r], fill=c)
        else:
            color = (CYAN if i % 3 == 0 else (MAGENTA if i % 3 == 1 else ORANGE))
            for r, c, a in [(2.4, color, 0.6), (1.2, WHITE, 0.9)]:
                draw.ellipse([x - r, y - r, x + r, y + r], fill=c)
    out = draw_img

    # small orb in the middle
    out = plasma_ring(out, cx, cy, base_r=18, petals=4, phase=0.0,
                      thickness=2.5, squash=0.78, intensity=0.7)
    out = core_glow(out, cx, cy, radius=5, color=(255, 255, 255), halo=CYAN, halo_strength=0.4)
    return bottom_text(out, CELL_W / 2, CELL_H - 28, "", "KNOWLEDGE  GRAPH  ACTIVATED")


def render_panel_10(canvas: Image.Image) -> Image.Image:
    """IDENTITY RESTORED - HELLO ATUL with vertical light beam."""
    out = canvas.copy()
    arr = np.array(out).astype(np.float32)
    h, w, _ = arr.shape
    cx, cy = CELL_W / 2, CELL_H / 2 - 18

    # vertical light beam
    y, x = np.mgrid[0:h, 0:w]
    dx = x - cx
    beam = np.exp(-(dx ** 2) / 6.0) * 90
    arr[..., 0] = np.clip(arr[..., 0] + beam * 0.4, 0, 255)
    arr[..., 1] = np.clip(arr[..., 1] + beam * 0.7, 0, 255)
    arr[..., 2] = np.clip(arr[..., 2] + beam, 0, 255)
    # tighter bright core of beam
    beam_core = np.exp(-(dx ** 2) / 1.0) * 160
    arr[..., 0] = np.clip(arr[..., 0] + beam_core * 0.7, 0, 255)
    arr[..., 1] = np.clip(arr[..., 1] + beam_core, 0, 255)
    arr[..., 2] = np.clip(arr[..., 2] + beam_core, 0, 255)
    out = Image.fromarray(arr.clip(0, 255).astype(np.uint8))

    # concentric ground rings
    for r, a in [(50, 0.6), (40, 0.8), (30, 1.0), (20, 1.2)]:
        out = draw_arc(out, cx, h - 30, r, math.radians(170), math.radians(370), CYAN, width=1)
    out = draw_arc(out, cx, h - 30, 60, math.radians(160), math.radians(380), MAGENTA, width=1)

    # small portal orb
    out = plasma_ring(out, cx, cy, base_r=24, petals=4, phase=0.0,
                      thickness=3.5, squash=0.78, intensity=0.85)
    out = core_glow(out, cx, cy, radius=6, color=(255, 255, 255), halo=CYAN, halo_strength=0.5)

    # text: IDENTITY RESTORED small, HELLO ATUL big
    draw = ImageDraw.Draw(out)
    f1 = font(8, bold=True)
    f2 = font(15, bold=True)
    s1 = "IDENTITY  RESTORED"
    s2 = "HELLO ATUL"
    b1 = draw.textbbox((0, 0), s1, font=f1)
    b2 = draw.textbbox((0, 0), s2, font=f2)
    draw.text((cx - (b1[2] - b1[0]) / 2, h - 30), s1, font=f1, fill=DIM)
    draw.text((cx - (b2[2] - b2[0]) / 2, h - 18), s2, font=f2, fill=CYAN)
    return out


def render_panel_11(canvas: Image.Image) -> Image.Image:
    """PREPARING WORKSPACE - cityscape + portal ring rising from the city."""
    out = canvas.copy()
    arr = np.array(out).astype(np.float32)
    h, w, _ = arr.shape
    cx, cy = CELL_W / 2, CELL_H / 2 - 4

    # subtle background wash
    y, x = np.mgrid[0:h, 0:w]
    dx = x - cx
    dy = y - cy
    r2 = dx * dx + dy * dy
    bg = np.exp(-r2 / 4000.0) * 14
    arr[..., 2] = np.clip(arr[..., 2] + bg, 0, 255)
    out = Image.fromarray(arr.clip(0, 255).astype(np.uint8))

    # big portal ring rising from city
    out = plasma_ring(out, cx, cy + 14, base_r=42, petals=4, phase=0.0,
                      thickness=2.0, squash=0.95, intensity=0.85, color_phase=0.0)
    out = plasma_ring(out, cx, cy + 14, base_r=48, petals=4, phase=math.pi / 4,
                      thickness=1.5, squash=0.95, intensity=0.6, color_phase=math.pi / 2)
    out = plasma_ring(out, cx, cy + 14, base_r=55, petals=4, phase=0.0,
                      thickness=1.2, squash=0.95, intensity=0.4, color_phase=math.pi)

    # vertical beam from top
    dx2 = x - cx
    beam = np.exp(-(dx2 ** 2) / 8.0) * 60
    arr2 = np.array(out).astype(np.float32)
    arr2[..., 0] = np.clip(arr2[..., 0] + beam * 0.4, 0, 255)
    arr2[..., 1] = np.clip(arr2[..., 1] + beam * 0.6, 0, 255)
    arr2[..., 2] = np.clip(arr2[..., 2] + beam, 0, 255)
    out = Image.fromarray(arr2.clip(0, 255).astype(np.uint8))

    # small inner orb
    out = plasma_ring(out, cx, cy, base_r=20, petals=4, phase=0.0,
                      thickness=2.5, squash=0.78, intensity=0.7)
    out = core_glow(out, cx, cy, radius=4, color=(255, 255, 255), halo=CYAN, halo_strength=0.4)

    # cityscape silhouette
    rng = np.random.default_rng(seed=7)
    draw = ImageDraw.Draw(out)
    ground = h - 36
    # ground line glow
    draw.line([(0, ground), (w, ground)], fill=(80, 40, 140), width=1)
    # buildings: rectangles of varying height
    x_cursor = 0
    while x_cursor < w:
        bw = int(rng.integers(3, 9))
        bh = int(rng.integers(8, 28))
        col = (15, 10, 35)
        draw.rectangle([x_cursor, ground - bh, x_cursor + bw, ground], fill=col)
        # a few window dots
        if rng.random() < 0.4:
            wy = ground - int(rng.integers(2, bh - 1))
            wx = x_cursor + int(rng.integers(1, bw - 1))
            draw.ellipse([wx, wy, wx + 1, wy + 1], fill=ORANGE)
        x_cursor += bw + int(rng.integers(0, 3))

    # base of beam
    arr3 = np.array(out).astype(np.float32)
    dx3 = x - cx
    dy3 = y - (ground - 1)
    beam_base = np.exp(-(dx3 ** 2) / 3.0) * np.maximum(0, -dy3) * 1.0
    beam_base = np.where(beam_base > 0, beam_base, 0)
    arr3[..., 0] = np.clip(arr3[..., 0] + beam_base * 200, 0, 255)
    arr3[..., 1] = np.clip(arr3[..., 1] + beam_base * 220, 0, 255)
    arr3[..., 2] = np.clip(arr3[..., 2] + beam_base * 255, 0, 255)
    out = Image.fromarray(arr3.clip(0, 255).astype(np.uint8))

    # text
    draw = ImageDraw.Draw(out)
    fs = font(10, bold=True)
    s = "PREPARING  WORKSPACE"
    b = draw.textbbox((0, 0), s, font=fs)
    draw.text((cx - (b[2] - b[0]) / 2, h - 18), s, font=fs, fill=CYAN)
    return out


PANELS = [
    render_panel_00, render_panel_01, render_panel_02, render_panel_03,
    render_panel_04, render_panel_05, render_panel_06, render_panel_07,
    render_panel_08, render_panel_09, render_panel_10, render_panel_11,
]


# ============================================================================
# Compose the storyboard
# ============================================================================

def compose_storyboard() -> Image.Image:
    """Render the 4x3 grid of panels into the top 58% of the canvas."""
    # start with the existing canvas so the bottom 42% (desktop mockup) is preserved
    if OUT.is_file():
        canvas = Image.open(OUT).convert("RGB")
        if canvas.size != (CANVAS_W, CANVAS_H):
            canvas = Image.new("RGB", (CANVAS_W, CANVAS_H), (0, 0, 0))
    else:
        canvas = Image.new("RGB", (CANVAS_W, CANVAS_H), (0, 0, 0))

    draw = ImageDraw.Draw(canvas)
    # black out the top 58% so we paint fresh panels on a clean slate
    draw.rectangle([0, 0, CANVAS_W, TOP_H], fill=(0, 0, 0))

    for idx, render in enumerate(PANELS):
        row = idx // COLS
        col = idx % COLS
        x0 = col * CELL_W
        y0 = row * CELL_H
        panel_bg = panel_background()
        panel = render(panel_bg)
        canvas.paste(panel, (x0, y0))
    return canvas


def main() -> None:
    OUT.parent.mkdir(parents=True, exist_ok=True)
    img = compose_storyboard()
    img.save(OUT, "PNG", optimize=True)
    print(f"wrote {OUT} ({img.size[0]}x{img.size[1]})")


if __name__ == "__main__":
    main()
