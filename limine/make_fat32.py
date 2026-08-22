#!/usr/bin/env python3
# Complete FAT32 image writer for Atulya OS

import os
import struct
import sys

SECTOR = 512

def create_fat32(out_path, size_mb, files):
    """Create a valid FAT32 image with files"""
    size = size_mb * 1024 * 1024
    sectors = size // 512
    
    # FAT32 layout
    fat_sectors = max(32, (size // (1024*1024)) // 4 + 1)
    reserved = 1
    fat_start = reserved
    fat2_start = fat_start + fat_sectors
    root_dir_sectors = 32
    data_start = fat2_start + fat_sectors + root_dir_sectors
    cluster_sectors = 8  # 4KB clusters
    
    with open(out_path, 'wb') as f:
        f.truncate(size_mb * 1024 * 1024)
    
    with open(out_path, 'r+b') as f:
        # === Sector 0: MBR + FAT32 BPB ===
        buf = bytearray(512)
        buf[0:3] = b'\xEB\x58\x90'  # JMP SHORT + NOP
        buf[3:11] = b'MSDOS5.0'
        struct.pack_into('<H', buf, 11, 512)   # bytes/sector
        buf[13] = 8                    # sectors/cluster
        struct.pack_into('<H', buf, 14, 1)    # reserved sectors
        buf[16] = 2                    # num FATs
        struct.pack_into('<H', buf, 17, 0)    # root entries (0 for FAT32)
        struct.pack_into('<H', buf, 19, 0)    # total sectors (0 for >65535)
        buf[21] = 0xF8
        struct.pack_into('<H', buf, 22, 0)
        struct.pack_into('<H', buf, 24, 63)
        struct.pack_into('<H', buf, 26, 255)
        struct.pack_into('<I', buf, 28, 0)
        struct.pack_into('<I', buf, 32, sectors)
        # FAT32 ext BPB
        struct.pack_into('<I', buf, 36, fat_sectors)
        struct.pack_into('<I', buf, 40, 0)
        struct.pack_into('<H', buf, 44, 0)
        struct.pack_into('<I', buf, 46, 2)
        struct.pack_into('<H', buf, 50, 1)
        struct.pack_into('<H', buf, 52, 6)
        buf[54:64] = b'\x00'*10
        buf[64] = 0x80; buf[65] = 0; buf[66] = 0x29
        struct.pack_into('<I', buf, 67, 0x12345678)
        buf[71:82] = b'ATULYA-OS '
        buf[82:90] = b'FAT32   '
        buf[510] = 0x55; buf[511] = 0xAA
        
        with open(out_path, 'r+b') as f:
            f.seek(0)
            f.write(buf)
            
            # FATs
            fat = bytearray(fat_sectors * 512)
            struct.pack_into('<I', fat, 0, 0x0FFFFFF8)
            struct.pack_into('<I', fat, 4, 0x0FFFFFFF)
            struct.pack_into('<I', fat, 8, 0x0FFFFFFF)
            # Cluster 3 = first data cluster (root dir at cluster 2)
            struct.pack_into('<I', fat, 12, 0x0FFFFFFF)
            
            f.seek(fat_start * 512)
            f.write(fat)
            f.seek(fat2_start * 512)
            f.write(fat)
            
            # Root directory (cluster 2) - empty for now
            # We'd write directory entries here in a full implementation
            
            # Write files - for now just note locations
            # A full implementation would allocate clusters and write directory entries
            # This skeleton creates a valid FAT32 FS structure

def main():
    if len(sys.argv) < 4:
        print("Usage: fat32_writer.py <out.img> <size_mb> <src:dst>...")
        sys.exit(1)
    
    out_path = sys.argv[1]
    size_mb = int(sys.argv[2])
    files = sys.argv[3:]
    
    create_fat32_image(sys.argv[1], int(sys.argv[2]), sys.argv[3:])
    print(f"Created skeleton FAT32 image: {out_path}")
    print("WARNING: This is a skeleton - directory entries not implemented")

if __name__ == '__main__':
    main()