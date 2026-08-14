import os
from PIL import Image, ImageDraw, ImageFont

def generate_atlas():
    out_dir = os.path.join("assets", "fonts")
    os.makedirs(out_dir, exist_ok=True)

    char_w = 9
    char_h = 16
    font_size = 14

    # Try clean modern system fonts
    font_paths = [
        r"C:\Windows\Fonts\segoeui.ttf",
        r"C:\Windows\Fonts\arial.ttf",
        r"C:\Windows\Fonts\calibri.ttf",
        r"C:\Windows\Fonts\consola.ttf"
    ]
    font = None
    for p in font_paths:
        if os.path.exists(p):
            try:
                font = ImageFont.truetype(p, font_size)
                print(f"Loaded font: {p}")
                break
            except Exception as e:
                pass

    if font is None:
        font = ImageFont.load_default()

    # ASCII 32 (' ') to 126 ('~') = 95 characters
    # Stored sequentially in 1D array of (95 * 9 * 16) bytes
    total_chars = 95
    atlas_img = Image.new("L", (char_w * total_chars, char_h), 0)
    draw = ImageDraw.Draw(atlas_img)

    raw_bytes = bytearray()

    for i in range(total_chars):
        ascii_code = 32 + i
        ch = chr(ascii_code)
        
        # Single char canvas
        char_img = Image.new("L", (char_w, char_h), 0)
        cdraw = ImageDraw.Draw(char_img)
        
        # Center glyph in box
        bbox = cdraw.textbbox((0, 0), ch, font=font)
        gw = bbox[2] - bbox[0]
        gh = bbox[3] - bbox[1]
        x_off = max(0, (char_w - gw) // 2)
        y_off = max(0, (char_h - gh) // 2 - 1)
        
        cdraw.text((x_off, y_off), ch, fill=255, font=font)
        
        # Paste into atlas
        atlas_img.paste(char_img, (i * char_w, 0))
        raw_bytes.extend(char_img.tobytes())

    # Save PNG preview
    atlas_img.save(os.path.join(out_dir, "font_16_preview.png"))

    # Save raw 8-bit alpha coverage bytes
    bin_path = os.path.join(out_dir, "font_16_alpha.bin")
    with open(bin_path, "wb") as f:
        f.write(raw_bytes)

    print(f"Generated font atlas: {bin_path} ({len(raw_bytes)} bytes, 95 chars @ 9x16)")

if __name__ == "__main__":
    generate_atlas()
