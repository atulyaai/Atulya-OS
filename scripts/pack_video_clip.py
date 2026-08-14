import os
import torch
import torchvision.io as io
from PIL import Image
import numpy as np

def pack_clip():
    video_path = r"C:\Users\subli\Downloads\System_boot_displaying_brand_logo_202608150135.mp4"
    print(f"Reading {video_path}...")
    vframes, _, _ = io.read_video(video_path, pts_unit='sec')
    total_frames = vframes.shape[0]

    # Select 18 frames spanning the 3D geometry morph and ATULYA OS brand explosion
    indices = np.linspace(0, int(total_frames * 0.85), 18, dtype=int)
    w, h = 400, 225

    out_file = os.path.join("assets", "boot", "boot_stream.bin")
    os.makedirs(os.path.dirname(out_file), exist_ok=True)

    with open(out_file, "wb") as f:
        for idx in indices:
            frame = vframes[idx].numpy()
            img = Image.fromarray(frame).resize((w, h), Image.Resampling.LANCZOS)
            f.write(img.tobytes())

    # Also save login backdrop plate (frame at 92%)
    login_frame_idx = int(total_frames * 0.92)
    login_frame = vframes[login_frame_idx].numpy()
    login_img = Image.fromarray(login_frame).resize((400, 225), Image.Resampling.LANCZOS)
    login_img.save(os.path.join("assets", "boot", "login_plate.png"))
    with open(os.path.join("assets", "boot", "login_plate.rgb"), "wb") as f:
        f.write(login_img.tobytes())

    print(f"Packed 18 frames into {out_file} (Total bytes: {18 * w * h * 3})")
    print(f"Saved login_plate.rgb (400x225)")

if __name__ == "__main__":
    pack_clip()
