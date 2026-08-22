#!/usr/bin/env python3
import sys
import os
import fatfs

def main():
    img_path = r"D:\Atulya Tantra\Atulya OS\limine\hdd.img"
    size_mb = 32
    
    kernel_path = r"D:\Atulya Tantra\Atulya OS\target\x86_64-unknown-none\debug\atulyaos-kernel"
    cfg_path = r"D:\Atulya Tantra\Atulya OS\limine\limine.cfg"
    bios_path = r"D:\Atulya Tantra\Atulya OS\limine\extracted\limine-binary\limine-bios.sys"
    
    # Create raw image
    with open(img_path, 'wb') as f:
        f.truncate(32 * 1024 * 1024)
    
    # Create FAT32 using fatfs
    disk = fatfs.RamDisk(img_path, 32 * 1024 * 1024)
    part = fatfs.Partition(disk)
    
    # Create FAT32 filesystem
    part.mkfs()
    
    # Mount
    part.mount()
    
    # Create boot directory
    part.mkdir('/boot')
    
    # Copy kernel
    with open(kernel_path, 'rb') as src:
        fh = part.open('/boot/atulyaos-kernel', 'wb')
        fh.write(open(kernel_path, 'rb').read())
        fh.close()
    print("  kernel -> /boot/atulyaos-kernel")
    
    # Copy limine.cfg
    with open(cfg_path, 'rb') as src:
        fh = part.open('/limine.cfg', 'wb')
        fh.write(open(cfg_path, 'rb').read())
        fh.close()
    print("  limine.cfg -> /limine.cfg")
    
    # Copy limine-bios.sys
    with open(bios_path, 'rb') as src:
        fh = part.open('/limine-bios.sys', 'wb')
        fh.write(open(bios_path, 'rb').read())
        fh.close()
    print("  limine-bios.sys -> /limine-bios.sys")
    
    # Also copy to /boot/limine-bios.sys
    with open(bios_path, 'rb') as src:
        fh = part.open('/boot/limine-bios.sys', 'wb')
        fh.write(open(bios_path, 'rb').read())
        fh.close()
    print("  limine-bios.sys -> /boot/limine-bios.sys")
    
    print("Image created successfully")

if __name__ == '__main__':
    main()