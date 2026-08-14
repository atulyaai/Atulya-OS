//! power.rs — ACPI Power Management & Hardware Reset Controller for Atulya OS.
//!
//! Provides clean ACPI shutdown, keyboard controller 8042 soft-reboot, and hardware power status.

use x86_64::instructions::port::Port;

pub struct PowerManager;

impl PowerManager {
    /// Perform clean ACPI / QEMU hardware shutdown.
    pub unsafe fn shutdown() -> ! {
        crate::serial::serial_write_line("System: Initiating ACPI Clean Shutdown...");
        
        // 1. QEMU / Bochs ACPI Poweroff port (0x604 / 0xB004)
        Port::<u16>::new(0x604).write(0x2000);
        Port::<u16>::new(0xB004).write(0x2000);
        Port::<u16>::new(0x4004).write(0x3400);

        // 2. Fallback: disable interrupts and halt loop
        x86_64::instructions::interrupts::disable();
        loop {
            x86_64::instructions::hlt();
        }
    }

    /// Perform soft system reset via 8042 Keyboard Controller / Triple Fault.
    pub unsafe fn reboot() -> ! {
        crate::serial::serial_write_line("System: Initiating Hardware Soft Reboot...");

        // Pulse CPU Reset line via 8042 Keyboard Controller Port 0x64
        let mut p64 = Port::<u8>::new(0x64);
        p64.write(0xFE);

        // Fallback: Trigger triple fault with invalid IDT
        let null_idt = x86_64::structures::DescriptorTablePointer {
            limit: 0,
            base: x86_64::VirtAddr::new(0),
        };
        x86_64::instructions::tables::lidt(&null_idt);
        core::arch::asm!("int3", options(noreturn));
    }
}
