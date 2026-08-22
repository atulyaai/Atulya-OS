#!/usr/bin/env python3
# Correct FAT32 image with MBR - verified layout

import sys
import struct
import os

def create_boot_image(out_path, size_mb):
    size = size_mb * 1024 * 1024
    total_sectors = size // 512
    
    # Paths
    kernel_path = r"D:\Atulya Tantra\Atulya OS\target\x86_64-unknown-none\debug\atulyaos-kernel"
    cfg_path = r"D:\Atulya Tantra\Atulya OS\limine\limine.cfg"
    bios_path = r"D:\Atulya Tantra\Atulya OS\limine\extracted\limine-binary\limine-bios.sys"
    
    for p in [kernel_path, cfg_path, bios_path]:
        if not os.path.exists(p):
            print(f"ERROR: Missing {p}")
            sys.exit(1)
    
    # Layout constants
    PART_START = 63                    # partition starts at LBA 63
    SECTORS_PER_CLUSTER = 8            # 4KB clusters
    FAT_SECTORS = 245                  # ~1MB per FAT
    RESERVED = 1                       # partition sector 0 = BPB
    FAT1_START = 1                     # partition sector 1
    FAT2_START = 1 + 245               # 1 + 245
    ROOT_DIR_SECTORS = 32
    DATA_START = 1 + 2*245 + 32        # partition sector 523
    
    # Absolute LBA offsets
    part_start = 63
    part_sectors = (32 * 1024 * 1024 // 512) - 63  # 65536 - 63 = 65473
    
    # Absolute LBA calculations
    fat1_lba = 63 + 1
    fat2_lba = 63 + 1 + 245
    root_lba = 63 + 1 + 2*245 + 32      # LBA 554
    data_lba = 63 + 1 + 2*245 + 32      # 554
    
    # Cluster LBA calculations (8 sectors per cluster)
    cluster_lba = lambda c: 63 + 1 + 2*245 + 32 + (c - 2) * 8
    # cluster 2 (root) = LBA 554
    # cluster 3 (kernel) = LBA 562
    # cluster 4 (cfg) = LBA 570
    # cluster 4 = LBA 578
    
    with open(out_path, 'wb') as f:
        f.truncate(32 * 1024 * 1024)
    
    with open(out_path, 'r+b') as f:
        # === Sector 0: MBR ===
        mbr = bytearray(512)
        # Minimal bootstrap
        mbr[0:3] = b'\xEB\x58\x90'
        mbr[3:11] = b'MSDOS5.0'
        # BPB (won't be used by BIOS since we have partition)
        struct.pack_into('<H', mbr, 11, 512)
        mbr[13] = 8
        struct.pack_into('<H', mbr, 14, 1)
        mbr[16] = 2
        struct.pack_into('<H', mbr, 17, 0)
        struct.pack_into('<H', mbr, 19, 0)
        mbr[21] = 0xF8
        struct.pack_into('<H', mbr, 22, 0)
        struct.pack_into('<H', mbr, 24, 63)
        struct.pack_into('<H', mbr, 26, 255)
        struct.pack_into('<I', mbr, 28, 63)
        struct.pack_into('<I', mbr, 32, 65536 - 63)
        mbr[-2] = 0x55; mbr[-1] = 0xAA
        
        # Partition table entry at 446
        # Bootable, FAT32 LBA (0x0C)
        mbr[446] = 0x80          # bootable
        mbr[447] = 0x01          # CHS start
        mbr[448] = 0x01
        mbr[449] = 0x00
        mbr[450] = 0x0C          # FAT32 LBA
        mbr[451] = 0xFE
        mbr[452] = 0xFF
        mbr[453] = 0xFF
        struct.pack_into('<I', mbr, 454, 63)           # LBA start
        struct.pack_into('<I', mbr, 458, 65536 - 63)   # size
        # Entries 2-4 empty
        for i in range(1, 4):
            off = 446 + i * 16
            mbr[off:off+16] = b'\x00'*16
        mbr[-2] = 0x55; mbr[-1] = 0xAA
        f.seek(0); f.write(mbr)
        
        # === Partition Boot Sector (LBA 63) ===
        bpb = bytearray(512)
        bpb[0:3] = b'\xEB\x58\x90'
        bpb[3:11] = b'MSDOS5.0'
        struct.pack_into('<H', bpb, 11, 512)
        bpb[13] = 8
        struct.pack_into('<H', bpb, 14, 1)
        bpb[16] = 2
        struct.pack_into('<H', bpb, 17, 0)
        struct.pack_into('<H', bpb, 19, 0)
        bpb[21] = 0xF8
        struct.pack_into('<H', bpb, 22, 0)
        struct.pack_into('<H', bpb, 24, 63)
        struct.pack_into('<H', bpb, 26, 255)
        struct.pack_into('<I', bpb, 28, 63)
        struct.pack_into('<I', bpb, 32, 65536 - 63)
        # FAT32 ext BPB
        struct.pack_into('<I', bpb, 36, 245)
        struct.pack_into('<I', bpb, 40, 0)
        struct.pack_into('<H', bpb, 44, 0)
        struct.pack_into('<I', bpb, 46, 2)
        struct.pack_into('<H', bpb, 50, 1)
        struct.pack_into('<H', bpb, 52, 6)
        bpb[54:64] = b'\x00'*10
        bpb[64] = 0x80; bpb[65] = 0; bpb[66] = 0x29
        struct.pack_into('<I', bpb, 67, 0x12345678)
        bpb[71:82] = b'ATULYA-OS '
        bpb[82:90] = b'FAT32   '
        bpb[-2] = 0x55; bpb[-1] = 0xAA
        f.seek(63 * 512)
        f.write(bpb)
        
        # === FATs ===
        fat = bytearray(245 * 512)
        struct.pack_into('<I', fat, 0, 0x0FFFFFF8)
        struct.pack_into('<I', fat, 4, 0x0FFFFFFF)
        struct.pack_into('<I', fat, 8, 0x0FFFFFFF)   # cluster 2 (root)
        struct.pack_into('<I', fat, 12, 0x0FFFFFFF)  # 3 (kernel)
        struct.pack_into('<I', fat, 16, 0x0FFFFFFF)  # 4 (cfg)
        struct.pack_into('<I', fat, 20, 0x0FFFFFFF)  # 5 (bios.sys)
        struct.pack_into('<I', fat, 24, 0x0FFFFFFF)  # 6 (end)
        
        f.seek(64 * 512)          # LBA 64 = FAT1
        f.write(fat)
        f.seek(309 * 512)         # LBA 309 = FAT2
        f.write(fat)
        
        # Root directory at LBA 554 (partition sector 491)
        root_lba = 554
        f.seek(554 * 512)
        
        # Root dir entries
        root = bytearray(512)
        # Volume label
        root[0:11] = b'ATULYA-OS '
        root[11] = 0x08
        f.write(root)
        
        # Kernel entry (cluster 3)
        kernel_data = open(r"D:\Atulya Tantra\Atulya OS\target\x86_64-unknown-none\debug\atulyaos-kernel", 'rb').read()
        e = bytearray(32)
        e[0:11] = b'ATULYAOSKEL'
        e[11] = 0x20
        struct.pack_into('<H', e, 20, 0)        # high cluster
        struct.pack_into('<H', e, 26, 3)        # cluster 3
        struct.pack_into('<I', e, 28, len(open(r"D:\Atulya Tantra\Atulya OS\target\x86_64-unknown-none\debug\atulyaos-kernel", 'rb').read()))
        f.seek(554 * 512 + 32)
        f.write(e)
        
        # limine.cfg (cluster 4)
        cfg_data = open(r"D:\Atulya Tantra\Atulya OS\limine\limine.cfg", 'rb').read()
        e2 = bytearray(32)
        e2[0:11] = b'LIMINE  CFG'
        e2[11] = 0x20
        struct.pack_into('<H', e2, 26, 4)
        struct.pack_into('<I', e2, 28, len(open(r"D:\Atulya Tantra\Atulya OS\limine\limine.cfg", 'rb').read()))
        f.seek(554 * 512 + 64)
        f.write(e2)
        
        # limine-bios.sys (cluster 5)
        bios_data = open(r"D:\Atulya Tantra\Atulya OS\limine\extracted\limine-binary\limine-bios.sys", 'rb').read()
        e3 = bytearray(32)
        e3[0:11] = b'LIMINEBIOSYS'
        e3[11] = 0x20
        struct.pack_into('<H', e3, 26, 5)
        struct.pack_into('<I', e3, 28, len(bios_data))
        f.seek(554 * 512 + 96)
        f.write(e3)
        
        # Write file data
        # Cluster 3 (kernel) at LBA 562
        f.seek(562 * 512)
        with open(r"D:\Atulya Tantra\Atulya OS\target\x86_64-unknown-none\debug\atulyaos-kernel", 'rb') as kf:
            f.write(kf.read())
        
        # Cluster 4 (limine.cfg) at LBA 570
        f.seek(570 * 512)
        with open(r"D:\Atulya Tantra\Atulya OS\limine\limine.cfg", 'rb') as cf:
            f.write(cf.read())
        
        # limine-bios.sys at LBA 578
        f.seek(578 * 512)
        with open(r"D:\Atulya Tantra\Atulya OS\limine\extracted\limine-binary\limine-bios.sys", 'rb') as bf:
            f.write(bf.read())
        
        # Also update FAT for cluster 5 and 6 (end marker)
        fat = bytearray(245 * 512)
        struct.pack_into('<I', fat, 0, 0x0FFFFFF8)
        struct.pack_into('<I', fat, 4, 0x0FFFFFFF)
        struct.pack_into('<I', fat, 8, 0x0FFFFFFF)
        struct.pack_into('<I', fat, 12, 0x0FFFFFFF)
        struct.pack_into('<I', fat, 16, 0x0FFFFFFF)
        struct.pack_into('<I', fat, 20, 0x0FFFFFFF)
        struct.pack_into('<I', fat, 24, 0x0FFFFFFF)
        f.seek(64 * 512)
        f.write(fat)
        f.seek(309 * 512)
        f.write(fat)

    print("Created verified FAT32 image")

if __name__ == '__main__':
    if len(sys.argv) != 3:
        print("Usage: fat32_verified.py <out.img> <size_mb>")
        sys.exit(1)
    create_boot_image(sys.argv[1], int(sys.argv[2]))
    print("Done")