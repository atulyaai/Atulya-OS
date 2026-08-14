import os
import math
from PIL import Image

image_path = r"F:\Atulya OS\assets\images\atulyaos_logo.png"
output_path = r"F:\Atulya OS\kernel\src\logo_data.rs"

if not os.path.exists(image_path):
    print(f"Error: image not found at {image_path}")
    exit(1)

img = Image.open(image_path)
img = img.convert("RGB")
img = img.resize((256, 256), Image.Resampling.LANCZOS)

bytes_data = []
for y in range(256):
    for x in range(256):
        r, g, b = img.getpixel((x, y))
        
        # 1. Radial mask (fade to black at edges)
        dx = x - 128
        dy = y - 128
        dist = math.sqrt(dx*dx + dy*dy)
        
        if dist > 115:
            radial_fade = 0.0
        elif dist > 75:
            radial_fade = 1.0 - (dist - 75) / 40.0
        else:
            radial_fade = 1.0
            
        r = int(r * radial_fade)
        g = int(g * radial_fade)
        b = int(b * radial_fade)
        
        # 2. Dark threshold mapping (quadratic dimming for dark background pixels)
        brightness = (r + g + b) / 3.0
        if brightness < 50:
            factor = (brightness / 50.0) ** 2.5
            r = int(r * factor)
            g = int(g * factor)
            b = int(b * factor)
            
        # Clamp to absolute zero if very dim
        if r < 13 and g < 13 and b < 13:
            r = 0
            g = 0
            b = 0
            
        bytes_data.append(r)
        bytes_data.append(g)
        bytes_data.append(b)

with open(output_path, "w") as f:
    f.write("pub const LOGO_WIDTH: usize = 256;\n")
    f.write("pub const LOGO_HEIGHT: usize = 256;\n\n")
    f.write("pub const LOGO_DATA: &[u8; 196608] = &[\n")
    
    # Write chunks
    for i in range(0, len(bytes_data), 12):
        chunk = bytes_data[i:i+12]
        line = ", ".join(str(val) for val in chunk)
        f.write(f"    {line},\n")
        
    f.write("];\n")

print("Conversion complete successfully.")
