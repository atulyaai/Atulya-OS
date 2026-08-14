from PIL import Image
import math

img = Image.open(r"d:\Atulya Tantra\Atulya OS\assets\images\atulyaos_logo.png").convert("RGBA")
w, h = img.size

# The center of the 1024x1024 logo contains the glowing sphere (approx x: 260..764, y: 260..764)
crop_size = 560
left = (w - crop_size) // 2
top = (h - crop_size) // 2
orb = img.crop((left, top, left + crop_size, top + crop_size))

# Resize to 280x280 with Lanczos
orb = orb.resize((280, 280), Image.Resampling.LANCZOS)

# Create a smooth circular alpha feather
alpha_mask = Image.new("L", (280, 280), 0)
for y in range(280):
    for x in range(280):
        dx = x - 140
        dy = y - 140
        dist = math.sqrt(dx*dx + dy*dy)
        if dist < 110:
            a = 255
        elif dist < 138:
            a = int(255 * (1.0 - (dist - 110) / 28.0))
        else:
            a = 0
        alpha_mask.putpixel((x, y), a)

orb.putalpha(alpha_mask)
orb.save(r"d:\Atulya Tantra\Atulya OS\assets\boot\orb_hero.png")

# Also save raw RGBA
raw_rgba = bytearray()
for y in range(280):
    for x in range(280):
        r, g, b, a = orb.getpixel((x, y))
        raw_rgba.extend([r, g, b, a])

with open(r"d:\Atulya Tantra\Atulya OS\assets\boot\orb_hero.rgba", "wb") as f:
    f.write(raw_rgba)

print("Saved orb_hero.png and orb_hero.rgba (280x280 RGBA, length:", len(raw_rgba), "bytes)")
