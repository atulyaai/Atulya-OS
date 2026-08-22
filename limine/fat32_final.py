#!/usr/bin/env python3
# Correct FAT32 image with MBR partition - proper layout

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
    
    # Layout for 32MB image with partition at sector 63
    part_start = 63
    part_sectors = (size_mb * 1024 * 1024 // 512) - 63
    
    fat_sectors = 245
    reserved = 1
    fat1_start = reserved
    fat2_start = fat1_start + 245
    root_dir_sectors = 32
    data_start = reserved + 2*245 + 32  # 1 + 490 + 32 = 523 (relative to partition)
    part_data_start = part_start + data_start
    
    with open(out_path, 'wb') as f:
        f.truncate(size_mb * 1024 * 1024)
    
    with open(out_path, 'r+b') as f:
        # === Sector 0: MBR ===
        mbr = bytearray(512)
        # Bootstrap code (minimal)
        mbr[0:3] = b'\xEB\x58\x90'
        mbr[3:11] = b'MSDOS5.0'
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
        struct.pack_into('<I', mbr, 36, 245)
        struct.pack_into('<I', mbr, 40, 0)
        struct.pack_into('<H', mbr, 44, 0)
        struct.pack_into('<I', mbr, 46, 2)
        struct.pack_into('<H', mbr, 50, 1)
        struct.pack_into('<H', mbr, 52, 6)
        mbr[54:64] = b'\x00'*10
        mbr[64] = 0x80; mbr[65] = 0; mbr[66] = 0x29
        struct.pack_into('<I', mbr, 67, 0x12345678)
        mbr[71:82] = b'ATULYA-OS '
        mbr[82:90] = b'FAT32   '
        mbr[-2] = 0x55; mbr[-1] = 0xAA
        
        # Partition table entry (offset 446)
        # Bootable, FAT32 LBA (0x0C), LBA start=63, size=total-63
        mbr[446] = 0x80  # bootable
        mbr[447] = 0x01  # CHS start (not used)
        mbr[448] = 0x01
        mbr[449] = 0x00
        mbr[450] = 0x0C  # FAT32 LBA
        mbr[451] = 0xFE
        mbr[452] = 0xFF
        mbr[453] = 0xFF
        struct.pack_into('<I', mbr, 454, 63)
        struct.pack_into('<I', mbr, 458, 65536 - 63)
        # Entries 2-4 empty
        for i in range(1, 4):
            off = 446 + i * 16
            mbr[off:off+16] = b'\x00'*16
        
        mbr[-2] = 0x55; mbr[-1] = 0xAA
        f.seek(0)
        f.write(mbr)
        
        # === Sector 63: Partition Boot Sector (FAT32 BPB) ===
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
        
        # FATs (relative to partition start)
        fat = bytearray(245 * 512)
        struct.pack_into('<I', fat, 0, 0x0FFFFFF8)
        struct.pack_into('<I', fat, 4, 0x0FFFFFFF)
        struct.pack_into('<I', fat, 8, 0x0FFFFFFF)   # cluster 2
        struct.pack_into('<I', fat, 12, 0x0FFFFFFF)  # 3
        struct.pack_into('<I', fat, 16, 0x0FFFFFFF)  # 4
        struct.pack_into('<I', fat, 20, 0x0FFFFFFF)  # 5
        
        f.seek((63 + 1) * 512)
        f.write(fat)
        f.seek((63 + 1 + 245) * 512)
        f.write(fat)
        
        # Root dir (cluster 2) at data_start = 63 + 1 + 2*245 + 32 = 523
        root_sector = 63 + 1 + 2*245 + 32
        f.seek(root_sector * 512)
        
        # Root dir entries
        root = bytearray(512)
        root[0:11] = b'ATULYA-OS '
        root[11] = 0x08
        f.write(root)
        
        # Kernel entry (cluster 3)
        kernel_data = open(r"D:\Atulya Tantra\Atulya OS\target\x86_64-unknown-none\debug\atulyaos-kernel", 'rb').read()
        e = bytearray(32)
        e[0:11] = b'ATULYAOSKEL'
        e[11] = 0x20
        struct.pack_into('<H', e, 20, 0)
        struct.pack_into('<H', e, 26, 3)
        struct.pack_into('<I', e, 28, len(kernel_data))
        f.seek((63 + 1 + 2*245 + 32) * 512 + 32)
        f.write(e)
        
        # limine.cfg (cluster 4)
        cfg_data = open(r"D:\Atulya Tantra\Atulya OS\limine\limine.cfg", 'rb').read()
        e2 = bytearray(32)
        e2[0:11] = b'LIMINE  CFG'
        e2[11] = 0x20
        struct.pack_into('<H', e2, 20, 0)
        struct.pack_into('<H', e2, 26, 4)
        struct.pack_into('<I', e2, 28, len(cfg_data))
        f.seek((63 + 1 + 2*245 + 32) * 512 + 64)
        f.write(e2)
        
        # limine-bios.sys (cluster 5)
        bios_data = open(r"D:\Atulya Tantra\Atulya OS\limine\extracted\limine-binary\limine-bios.sys", 'rb').read()
        e3 = bytearray(32)
        e3[0:11] = b'LIMINEBIOSYS'
        e3[11] = 0x20
        struct.pack_into('<H', e3, 20, 0)
        struct.pack_into('<H', e3, 26, 5)
        struct.pack_into('<I', e3, 28, len(bios_data))
        f.seek((63 + 1 + 2*245 + 32) * 512 + 96)
        f.write(e3)
        
        # Write file data
        data_start_abs = 63 + 1 + 2*245 + 32  # sector 523
        cluster_size = 8  # sectors
        
        # Kernel at cluster 3
        f.seek((63 + 1 + 2*245 + 32 + 8) * 512)
        with open(r"D:\Atulya Tantra\Atulya OS\target\x86_64-unknown-none\debug\atulyaos-kernel", 'rb') as kf:
            f.write(kf.read())
        
        # limine.cfg at cluster 4
        f.seek((63 + 1 + 2*245 + 32 + 16) * 512)
        with open(r"D:\Atulya Tantra\Atulya OS\limine\limine.cfg", 'rb') as cf:
            f.write(cf.read())
        
        # limine-bios.sys at cluster 5
        f.seek((63 + 1 + 2*245 + 32 + 24) * 512)
        with open(r"D:\Atulya Tantra\Atulya OS\limine\extracted\limine-binary\limine-bios.sys", 'rb') as bf:
            f.write(bf.read())

    print(f"Created correct FAT32 image: {out_path}")

if __name__ == '__main__':
    if len(sys.argv) != 3:
        print("Usage: fat32_final.py <out.img> <size_mb>")
        sys.exit(1)
    create_boot_image(sys.argv[1], int(sys.argv[2]))
    print("Done")