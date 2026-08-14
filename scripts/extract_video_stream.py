import os
import torch
import torchvision.io as io
from PIL import Image
import numpy as np

def extract():
    video_path = r"C:\Users\subli\Downloads\System_boot_displaying_brand_logo_202608150135.mp4"
    if not os.path.exists(video_path):
        video_path = r"C:\Users\subli\Downloads\Storyboard_grid_morphs_into_sphere_202608150135.mp4"

    print(f"Reading video: {video_path}")
    vframes, aframes, info = io.read_video(video_path, pts_unit='sec')
    total_frames = vframes.shape[0]
    print(f"Total video frames: {total_frames}, shape: {vframes.shape}, fps: {info.get('video_fps', 24)}")

    out_dir = os.path.join("assets", "boot_video")
    os.makedirs(out_dir, exist_ok=True)

    # We select 24 high-impact keyframes across the timeline for a fast 1.5s boot sequence
    target_count = 24
    indices = np.linspace(0, total_frames - 1, target_count, dtype=int)

    # Target display size for blitting (e.g. 480x270 or 640x360 centered)
    target_w, target_h = 480, 270

    for i, idx in enumerate(indices):
        frame_tensor = vframes[idx].numpy() # shape (H, W, 3) uint8
        img = Image.fromarray(frame_tensor)
        img_resized = img.resize((target_w, target_h), Image.Resampling.LANCZOS)
        
        # Save PNG preview
        img_resized.save(os.path.join(out_dir, f"frame_{i:02d}.png"))

        # Save raw RGB bytes
        rgb_data = img_resized.tobytes()
        with open(os.path.join(out_dir, f"frame_{i:02d}.rgb"), "wb") as f:
            f.write(rgb_data)

    print(f"Successfully extracted {target_count} frames to {out_dir} ({target_w}x{target_h})")

if __name__ == "__main__":
    extract()
