#!/usr/bin/env python3
# Minimal FAT32 image writer for Atulya OS boot image

import os
import struct
import sys

SECTOR = 512

def write_fat32_image(out_path, size_mb, files):
    size = size_mb * 1024 * 1024
    sectors = size // 512
    
    # FAT32 parameters
    fat_sectors = max(32, (sectors // 8192) + 1)  # ~1 FAT sector per 4MB
    reserved = 1
    fat_start = reserved
    fat2_start = fat_start + fat_sectors
    root_dir_sectors = 32
    data_start = fat2_start + fat_sectors + root_dir_sectors
    cluster_sectors = 8
    
    with open(out_path, 'wb') as f:
        f.seek(size - 1)
        f.write(b'\x00')
    
    with open(out_path, 'r+b') as f:
        # MBR + FAT32 BPB (sector 0)
        buf = bytearray(512)
        buf[0:3] = b'\xEB\x58\x90'  # JMP SHORT + NOP
        buf[3:11] = b'MSDOS5.0'    # OEM
        struct.pack_into('<H', buf, 11, 512)   # bytes/sector
        buf[13] = 8                    # sectors/cluster
        struct.pack_into('<H', buf, 14, 1)    # reserved sectors
        buf[16] = 2                    # num FATs
        struct.pack_into('<H', buf, 17, 0)    # root entries (0 for FAT32)
        struct.pack_into('<H', buf, 19, 0)    # total sectors (0 for FAT32)
        buf[21] = 0xF8                 # media descriptor
        struct.pack_into('<H', buf, 22, 0)    # sectors/FAT (0 for FAT32)
        struct.pack_into('<H', buf, 24, 63)   # sectors/track
        struct.pack_into('<H', buf, 26, 255)  # heads
        struct.pack_into('<I', buf, 28, 0)    # hidden sectors
        struct.pack_into('<I', buf, 32, sectors)  # total sectors
        
        # FAT32 extended BPB
        struct.pack_into('<I', buf, 36, fat_sectors)  # sectors/FAT
        struct.pack_into('<I', buf, 40, 0)    # flags
        struct.pack_into('<H', buf, 44, 0)    # version
        struct.pack_into('<I', buf, 46, 2)    # root cluster = 2
        struct.pack_into('<H', buf, 50, 1)    # FSInfo sector
        struct.pack_into('<H', buf, 52, 6)    # backup boot sector
        buf[54:64] = b'\x00' * 10      # reserved
        buf[64] = 0x80                 # drive number
        buf[65] = 0                    # reserved
        buf[66] = 0x29                 # extended boot sig
        struct.pack_into('<I', buf, 67, 0x12345678)  # volume ID
        buf[71:82] = b'ATULYA-OS '     # volume label
        buf[82:90] = b'FAT32   '       # FS type
        buf[510] = 0x55
        buf[511] = 0xAA
        
        f.seek(0)
        f.write(buf)
        
        # FAT tables
        fat = bytearray(fat_sectors * 512)
        # Cluster 0: media descriptor
        struct.pack_into('<I', fat, 0, 0x0FFFFFF8)
        # Cluster 1: reserved
        struct.pack_into('<I', fat, 4, 0x0FFFFFFF)
        # Cluster 2: root directory
        struct.pack_into('<I', fat, 8, 0x0FFFFFFF)
        
        # Write FAT1
        f.seek(fat_start * 512)
        f.write(fat)
        # Write FAT2
        f.seek(fat2_start * 512)
        f.write(fat)
        
        # Root directory (cluster 2) - create directory entries
        # For simplicity, we just create a minimal valid FS
        # In a real implementation we'd write proper directory entries
        
        # Files are written starting at data area
        data_area = data_start * 512
        cluster_size = 8 * 512  # 4KB clusters
        
        print(f"Created FAT32 image: {out_path} ({size_mb}MB)")
        print("NOTE: This is a minimal skeleton - real implementation needs directory entries")
        return True

if __name__ == '__main__':
    if len(sys.argv) < 4:
        print(f"Usage: {sys.argv[0]} <out.img> <size_mb> <src:dst>...")
        sys.exit(1)
    
    out_path = sys.argv[1]
    size_mb = int(sys.argv[2])
    files = sys.argv[3:]
    
    write_fat32_image(out_path, int(sys.argv[2]), sys.argv[3:])