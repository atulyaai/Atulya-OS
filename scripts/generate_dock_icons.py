import os
from PIL import Image, ImageDraw, ImageFont

def generate_icons():
    out_dir = os.path.join("assets", "icons")
    os.makedirs(out_dir, exist_ok=True)
    icon_size = 32

    # 9 apps: Terminal, Web, Mesh, Code, File, Stats, Media, 3D, Security
    icons = []
    
    # Palette
    CYAN = (0, 229, 255, 255)
    GREEN = (0, 230, 118, 255)
    MAGENTA = (220, 60, 255, 255)
    AMBER = (255, 170, 40, 255)
    BLUE = (40, 140, 255, 255)
    GOLD = (255, 215, 0, 255)

    # 1. Terminal (>_)
    img = Image.new("RGBA", (icon_size, icon_size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([2, 2, 29, 29], radius=6, fill=(10, 20, 15, 220), outline=GREEN, width=2)
    d.line([(8, 10), (14, 15), (8, 20)], fill=GREEN, width=2)
    d.line([(16, 21), (23, 21)], fill=GREEN, width=2)
    icons.append(("terminal", img))

    # 2. Web Browser (Globe)
    img = Image.new("RGBA", (icon_size, icon_size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([2, 2, 29, 29], radius=6, fill=(8, 20, 35, 220), outline=CYAN, width=2)
    d.ellipse([7, 7, 24, 24], outline=CYAN, width=2)
    d.ellipse([11, 7, 20, 24], outline=CYAN, width=1)
    d.line([(7, 15), (24, 15)], fill=CYAN, width=1)
    icons.append(("browser", img))

    # 3. Mesh (Lattice Network)
    img = Image.new("RGBA", (icon_size, icon_size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([2, 2, 29, 29], radius=6, fill=(25, 10, 35, 220), outline=MAGENTA, width=2)
    pts = [(16, 7), (8, 23), (24, 23), (16, 16)]
    d.line([(16, 7), (8, 23), (24, 23), (16, 7)], fill=MAGENTA, width=1)
    for p in pts:
        d.ellipse([p[0]-2, p[1]-2, p[0]+2, p[1]+2], fill=(255, 255, 255, 255))
    icons.append(("mesh", img))

    # 4. Code Editor ({ ; })
    img = Image.new("RGBA", (icon_size, icon_size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([2, 2, 29, 29], radius=6, fill=(30, 20, 10, 220), outline=AMBER, width=2)
    d.line([(10, 10), (7, 15), (10, 20)], fill=AMBER, width=2)
    d.line([(21, 10), (24, 15), (21, 20)], fill=AMBER, width=2)
    d.line([(14, 21), (17, 10)], fill=(255, 255, 255, 220), width=2)
    icons.append(("code", img))

    # 5. File Manager (Folder)
    img = Image.new("RGBA", (icon_size, icon_size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([2, 2, 29, 29], radius=6, fill=(10, 25, 45, 220), outline=BLUE, width=2)
    d.polygon([(6, 10), (13, 10), (16, 13), (25, 13), (25, 23), (6, 23)], fill=(20, 60, 110, 220), outline=BLUE)
    icons.append(("file", img))

    # 6. System Analytics (Pulse Graph)
    img = Image.new("RGBA", (icon_size, icon_size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([2, 2, 29, 29], radius=6, fill=(10, 30, 25, 220), outline=GREEN, width=2)
    d.line([(6, 20), (11, 20), (15, 9), (19, 23), (22, 16), (26, 16)], fill=GREEN, width=2)
    icons.append(("stats", img))

    # 7. Media Player (Play Button)
    img = Image.new("RGBA", (icon_size, icon_size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([2, 2, 29, 29], radius=6, fill=(35, 10, 30, 220), outline=MAGENTA, width=2)
    d.polygon([(12, 10), (22, 16), (12, 22)], fill=(255, 120, 255, 255), outline=MAGENTA)
    icons.append(("media", img))

    # 8. 3D Engine (Isometric Cube)
    img = Image.new("RGBA", (icon_size, icon_size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([2, 2, 29, 29], radius=6, fill=(15, 20, 35, 220), outline=CYAN, width=2)
    d.polygon([(16, 8), (24, 12), (16, 16), (8, 12)], outline=CYAN, fill=(20, 50, 80, 180))
    d.line([(16, 16), (16, 24)], fill=CYAN, width=1)
    d.line([(8, 12), (8, 20), (16, 24), (24, 20), (24, 12)], fill=CYAN, width=1)
    icons.append(("3d", img))

    # 9. Security Shield (Lock Shield)
    img = Image.new("RGBA", (icon_size, icon_size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([2, 2, 29, 29], radius=6, fill=(30, 25, 10, 220), outline=GOLD, width=2)
    d.polygon([(16, 7), (24, 10), (24, 17), (16, 24), (8, 17), (8, 10)], fill=(80, 60, 20, 180), outline=GOLD, width=2)
    icons.append(("security", img))

    # Pack into single 9 * 32 * 32 * 4 bytes buffer
    raw_rgba = bytearray()
    sheet = Image.new("RGBA", (32 * 9, 32), (0, 0, 0, 0))
    for i, (name, icon) in enumerate(icons):
        icon.save(os.path.join(out_dir, f"{name}.png"))
        sheet.paste(icon, (i * 32, 0))
        raw_rgba.extend(icon.tobytes())

    sheet.save(os.path.join(out_dir, "dock_icons_sheet.png"))
    bin_path = os.path.join(out_dir, "dock_icons.rgba")
    with open(bin_path, "wb") as f:
        f.write(raw_rgba)

    print(f"Generated 9 dock icons: {bin_path} ({len(raw_rgba)} bytes, 9x32x32 RGBA)")

if __name__ == "__main__":
    generate_icons()
