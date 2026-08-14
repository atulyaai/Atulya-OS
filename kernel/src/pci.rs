//! pci.rs — PCI Bus Enumerator and Hardware Device Driver for Atulya OS.
//!
//! Uses PCI Configuration Space I/O Ports:
//!   - 0xCF8: PCI Configuration Address Port
//!   - 0xCFC: PCI Configuration Data Port

use alloc::vec::Vec;

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

#[derive(Clone, Debug)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_id: u8,
    pub subclass_id: u8,
    pub bar0: u32,
    pub irq: u8,
}

impl PciDevice {
    pub fn description(&self) -> &'static str {
        match (self.vendor_id, self.device_id) {
            (0x1234, 0x1111) => "QEMU Standard VGA Graphics Controller",
            (0x1AF4, 0x1000) => "VirtIO Network Interface Card (Legacy)",
            (0x1AF4, 0x1041) => "VirtIO Network Interface Card (Modern)",
            (0x1AF4, 0x1050) => "VirtIO 3D GPU Accelerated Display",
            (0x8086, 0x1237) => "Intel 440FX PCI Host Bridge",
            (0x8086, 0x7000) => "Intel PIIX3 PCI-to-ISA Bridge",
            (0x8086, 0x7010) => "Intel PIIX3 IDE Controller",
            (0x8086, 0x7113) => "Intel PIIX4 ACPI Power Controller",
            (0x8086, 0x100E) => "Intel e1000 Gigabit Ethernet",
            _ => match self.class_id {
                0x01 => "Mass Storage Controller",
                0x02 => "Network Controller",
                0x03 => "Display Controller",
                0x04 => "Multimedia Audio Device",
                0x06 => "Bridge Device",
                _ => "Generic PCI Device",
            },
        }
    }
}

pub struct PciBus;

impl PciBus {
    /// Read 32-bit dword from PCI configuration space.
    pub unsafe fn read_config_dword(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
        let address = ((bus as u32) << 16)
            | ((slot as u32) << 11)
            | ((func as u32) << 8)
            | ((offset as u32) & 0xFC)
            | 0x80000000;

        let mut addr_port = x86_64::instructions::port::Port::<u32>::new(PCI_CONFIG_ADDRESS);
        let mut data_port = x86_64::instructions::port::Port::<u32>::new(PCI_CONFIG_DATA);

        addr_port.write(address);
        data_port.read()
    }

    /// Read 16-bit word from PCI configuration space.
    pub unsafe fn read_config_word(bus: u8, slot: u8, func: u8, offset: u8) -> u16 {
        let dword = Self::read_config_dword(bus, slot, func, offset);
        ((dword >> ((offset & 2) * 8)) & 0xFFFF) as u16
    }

    /// Scan all PCI buses, slots, and functions to discover hardware devices.
    pub fn scan() -> Vec<PciDevice> {
        let mut devices = Vec::new();

        for bus in 0..=8 {
            for slot in 0..32 {
                let vendor_id = unsafe { Self::read_config_word(bus, slot, 0, 0x00) };
                if vendor_id == 0xFFFF || vendor_id == 0x0000 {
                    continue;
                }

                let device_id = unsafe { Self::read_config_word(bus, slot, 0, 0x02) };
                let class_subclass = unsafe { Self::read_config_word(bus, slot, 0, 0x0A) };
                let class_id = (class_subclass >> 8) as u8;
                let subclass_id = (class_subclass & 0xFF) as u8;

                let bar0 = unsafe { Self::read_config_dword(bus, slot, 0, 0x10) };
                let intr = unsafe { Self::read_config_word(bus, slot, 0, 0x3C) };
                let irq = (intr & 0xFF) as u8;

                devices.push(PciDevice {
                    bus,
                    device: slot,
                    function: 0,
                    vendor_id,
                    device_id,
                    class_id,
                    subclass_id,
                    bar0,
                    irq,
                });
            }
        }

        devices
    }

    /// Find VirtIO network device if attached.
    pub fn find_virtio_net() -> Option<PciDevice> {
        let devices = Self::scan();
        devices.into_iter().find(|d| d.vendor_id == 0x1AF4 && (d.device_id == 0x1000 || d.device_id == 0x1041))
    }
}
