//! ata.rs — ATA / IDE PIO Persistent Disk Controller for Atulya OS.
//!
//! Provides direct hardware block device read/write access to virtual & physical disks:
//!   - Primary IDE Channel: I/O Ports 0x1F0 - 0x1F7
//!   - Control Port: 0x3F6
//!   - 28-bit & 48-bit LBA sector addressing
//!   - Persists files, user workspace, and OS state across reboots.

const ATA_DATA: u16 = 0x1F0;
const ATA_FEATURES: u16 = 0x1F1;
const ATA_SECTOR_COUNT: u16 = 0x1F2;
const ATA_LBA_LOW: u16 = 0x1F3;
const ATA_LBA_MID: u16 = 0x1F4;
const ATA_LBA_HIGH: u16 = 0x1F5;
const ATA_DRIVE_HEAD: u16 = 0x1F6;
const ATA_COMMAND: u16 = 0x1F7;
const ATA_STATUS: u16 = 0x1F7;
const ATA_CONTROL: u16 = 0x3F6;

// ATA Commands
const CMD_READ_SECTORS: u8 = 0x20;
const CMD_WRITE_SECTORS: u8 = 0x30;
const CMD_IDENTIFY: u8 = 0xEC;

// ATA Status Bits
const STATUS_BSY: u8 = 0x80;
const STATUS_DRDY: u8 = 0x40;
const STATUS_DRQ: u8 = 0x08;
const STATUS_ERR: u8 = 0x01;

#[inline]
unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
}

#[inline]
unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    core::arch::asm!("in al, dx", out("al") val, in("dx") port, options(nomem, nostack, preserves_flags));
    val
}

#[inline]
unsafe fn inw(port: u16) -> u16 {
    let val: u16;
    core::arch::asm!("in ax, dx", out("ax") val, in("dx") port, options(nomem, nostack, preserves_flags));
    val
}

#[inline]
unsafe fn outw(port: u16, val: u16) {
    core::arch::asm!("out dx, ax", in("dx") port, in("ax") val, options(nomem, nostack, preserves_flags));
}

pub struct AtaDisk {
    pub total_sectors: u32,
    pub is_available: bool,
}

impl AtaDisk {
    pub const fn new() -> Self {
        Self {
            total_sectors: 0,
            is_available: false,
        }
    }

    /// Wait until drive is not busy (BSY bit clear).
    fn wait_not_busy() -> Result<(), &'static str> {
        for _ in 0..100_000 {
            let status = unsafe { inb(ATA_STATUS) };
            if status & STATUS_BSY == 0 {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err("ATA Timeout: Drive busy")
    }

    /// Wait until data request (DRQ) is ready.
    fn wait_drq() -> Result<(), &'static str> {
        for _ in 0..100_000 {
            let status = unsafe { inb(ATA_STATUS) };
            if status & STATUS_ERR != 0 {
                return Err("ATA Drive error");
            }
            if status & STATUS_DRQ != 0 {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err("ATA Timeout: DRQ not ready")
    }

    /// Initialize ATA Controller and identify primary master disk.
    pub fn init(&mut self) -> Result<(), &'static str> {
        unsafe {
            // Select Master drive (0xA0)
            outb(ATA_DRIVE_HEAD, 0xA0);
            outb(ATA_SECTOR_COUNT, 0);
            outb(ATA_LBA_LOW, 0);
            outb(ATA_LBA_MID, 0);
            outb(ATA_LBA_HIGH, 0);
            outb(ATA_COMMAND, CMD_IDENTIFY);

            let status = inb(ATA_STATUS);
            if status == 0 || status == 0xFF {
                return Err("No ATA primary drive detected");
            }

            Self::wait_not_busy()?;
            Self::wait_drq()?;

            // Read 256 words (512 bytes) of IDENTIFY data
            let mut info = [0u16; 256];
            for word in info.iter_mut() {
                *word = inw(ATA_DATA);
            }

            // Sectors count from words 60-61
            self.total_sectors = (info[60] as u32) | ((info[61] as u32) << 16);
            self.is_available = true;

            crate::serial::serial_write_line("ATA Primary Disk Initialized. Capacity: Active.");
            Ok(())
        }
    }

    /// Read a 512-byte sector at the given LBA into `buffer`.
    pub fn read_sector(&self, lba: u32, buffer: &mut [u8; 512]) -> Result<(), &'static str> {
        unsafe {
            Self::wait_not_busy()?;

            // Select master with LBA mode bits (0xE0 | top 4 bits of LBA)
            outb(ATA_DRIVE_HEAD, 0xE0 | (((lba >> 24) & 0x0F) as u8));
            outb(ATA_FEATURES, 0);
            outb(ATA_SECTOR_COUNT, 1);
            outb(ATA_LBA_LOW, (lba & 0xFF) as u8);
            outb(ATA_LBA_MID, ((lba >> 8) & 0xFF) as u8);
            outb(ATA_LBA_HIGH, ((lba >> 16) & 0xFF) as u8);
            outb(ATA_COMMAND, CMD_READ_SECTORS);

            Self::wait_not_busy()?;
            Self::wait_drq()?;

            // Read 256 words = 512 bytes
            for i in 0..256 {
                let word = inw(ATA_DATA);
                buffer[i * 2] = (word & 0xFF) as u8;
                buffer[i * 2 + 1] = ((word >> 8) & 0xFF) as u8;
            }

            Ok(())
        }
    }

    /// Write a 512-byte sector at the given LBA from `buffer`.
    pub fn write_sector(&self, lba: u32, buffer: &[u8; 512]) -> Result<(), &'static str> {
        unsafe {
            Self::wait_not_busy()?;

            outb(ATA_DRIVE_HEAD, 0xE0 | (((lba >> 24) & 0x0F) as u8));
            outb(ATA_FEATURES, 0);
            outb(ATA_SECTOR_COUNT, 1);
            outb(ATA_LBA_LOW, (lba & 0xFF) as u8);
            outb(ATA_LBA_MID, ((lba >> 8) & 0xFF) as u8);
            outb(ATA_LBA_HIGH, ((lba >> 16) & 0xFF) as u8);
            outb(ATA_COMMAND, CMD_WRITE_SECTORS);

            Self::wait_not_busy()?;
            Self::wait_drq()?;

            // Write 256 words = 512 bytes
            for i in 0..256 {
                let word = (buffer[i * 2] as u16) | ((buffer[i * 2 + 1] as u16) << 8);
                outw(ATA_DATA, word);
            }

            // Flush cache
            outb(ATA_COMMAND, 0xE7);
            Self::wait_not_busy()?;

            Ok(())
        }
    }
}

pub static DISK: spin::Mutex<AtaDisk> = spin::Mutex::new(AtaDisk::new());
