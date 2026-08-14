from PIL import Image, ImageDraw, ImageFont
import math

def create_banner():
    width, height = 1200, 520
    banner = Image.new("RGBA", (width, height), (3, 5, 10, 255))
    draw = ImageDraw.Draw(banner)

    # 1. Subtle deep gradient and grid
    for y in range(height):
        ratio = y / height
        r = int(2 + ratio * 4)
        g = int(4 + ratio * 8)
        b = int(10 + ratio * 20)
        draw.line([(0, y), (width, y)], fill=(r, g, b, 255))

    # Tech grid dots
    for x in range(0, width, 40):
        for y in range(0, height, 40):
            draw.point((x, y), fill=(0, 180, 255, 40))

    # Cyber horizon line
    draw.line([(0, int(height * 0.78)), (width, int(height * 0.78))], fill=(0, 229, 255, 60), width=1)

    # 2. Paste Logo Orb
    try:
        logo = Image.open("assets/images/atulyaos_logo.png").convert("RGBA")
        logo_thumb = logo.resize((360, 360), Image.Resampling.LANCZOS)
        orb_x = 100
        orb_y = int((height - 360) / 2)
        banner.paste(logo_thumb, (orb_x, orb_y), logo_thumb)
    except Exception as e:
        print(f"Logo load error: {e}")

    # 3. Typography & Text
    # Title
    try:
        font_large = ImageFont.truetype("arial.ttf", 54)
        font_sub = ImageFont.truetype("arial.ttf", 22)
        font_tag = ImageFont.truetype("arial.ttf", 16)
        font_desc = ImageFont.truetype("arial.ttf", 18)
    except:
        font_large = font_sub = font_tag = font_desc = ImageFont.load_default()

    text_x = 520
    draw.text((text_x, 110), "ATULYA OS", fill=(220, 245, 255, 255), font=font_large)
    draw.text((text_x, 180), "THE INTENT OPERATING SYSTEM", fill=(0, 229, 255, 255), font=font_sub)
    draw.text((text_x, 225), "Autonomous AI Core • 60 FPS Cyberpunk Compositor • Rust no_std", fill=(200, 215, 230, 220), font=font_desc)
    draw.text((text_x, 260), "Holographic Biometrics (AXON-7) • WebAssembly Skills • VirtIO Net", fill=(140, 170, 200, 200), font=font_desc)

    # Badges / Pills
    badges = ["RUST 2024", "x86_64 BARE-METAL", "TRUECOLOR FRAMEBUFFER", "WASM RUNTIME", "VFS & RAMDISK"]
    bx = text_x
    by = 320
    for b in badges:
        bw = len(b) * 10 + 20
        draw.rounded_rectangle([bx, by, bx + bw, by + 28], radius=6, fill=(10, 24, 45, 200), outline=(0, 180, 255, 120))
        draw.text((bx + 10, by + 5), b, fill=(0, 229, 255, 255), font=font_tag)
        bx += bw + 12

    # Corner brackets
    pad = 20
    blen = 24
    col = (0, 229, 255, 180)
    # TL
    draw.line([(pad, pad), (pad + blen, pad)], fill=col, width=2)
    draw.line([(pad, pad), (pad, pad + blen)], fill=col, width=2)
    # TR
    draw.line([(width - pad - blen, pad), (width - pad, pad)], fill=col, width=2)
    draw.line([(width - pad, pad), (width - pad, pad + blen)], fill=col, width=2)
    # BL
    draw.line([(pad, height - pad), (pad + blen, height - pad)], fill=col, width=2)
    draw.line([(pad, height - pad - blen), (pad, height - pad)], fill=col, width=2)
    # BR
    draw.line([(width - pad - blen, height - pad), (width - pad, height - pad)], fill=col, width=2)
    draw.line([(width - pad, height - pad - blen), (width - pad, height - pad)], fill=col, width=2)

    banner.save("assets/images/banner.png")
    print("Saved assets/images/banner.png (1200x520)")

if __name__ == "__main__":
    create_banner()
