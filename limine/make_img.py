#!/usr/bin/env python3
import sys
import fatfs

def main():
    if len(sys.argv) < 4:
        print(f"Usage: {sys.argv[0]} <out.img> <size_mb> <src:dst>...")
        sys.exit(1)
    
    img_path = sys.argv[1]
    size_mb = int(sys.argv[2])
    pairs = sys.argv[3:]
    
    # Create raw image
    with open(img_path, 'wb') as f:
        f.truncate(size_mb * 1024 * 1024)
    
    # Create disk and partition
    disk = fatfs.RamDisk(img_path, size_mb * 1024 * 1024)
    part = fatfs.Partition(disk)
    
    # Create FAT32 filesystem
    part.mkfs()
    
    # Mount
    part.mount()
    
    # Create boot directory
    part.mkdir('/boot')
    
    # Copy files
    for pair in sys.argv[3:]:
        src, dst = pair.split(':', 1)
        data = open(src, 'rb').read()
        fh = part.open(dst, 'wb')
        fh.write(data)
        fh.close()
        print(f"  {src} -> {dst} ({len(data)} bytes)")
    
    print(f"Created bootable FAT32 image: {sys.argv[1]} ({sys.argv[2]}MB)")

if __name__ == '__main__':
    main()