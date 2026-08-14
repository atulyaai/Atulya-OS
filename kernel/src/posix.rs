//! posix.rs — Linux x86_64 ABI Compatibility & POSIX Syscall Bridge for Atulya OS.

pub const LINUX_SYS_READ: u64 = 0;
pub const LINUX_SYS_WRITE: u64 = 1;
pub const LINUX_SYS_OPEN: u64 = 2;
pub const LINUX_SYS_CLOSE: u64 = 3;
pub const LINUX_SYS_STAT: u64 = 4;
pub const LINUX_SYS_LSEEK: u64 = 8;
pub const LINUX_SYS_MMAP: u64 = 9;
pub const LINUX_SYS_BRK: u64 = 12;
pub const LINUX_SYS_IOCTL: u64 = 16;
pub const LINUX_SYS_GETPID: u64 = 39;
pub const LINUX_SYS_EXIT: u64 = 60;
pub const LINUX_SYS_UNAME: u64 = 63;
pub const LINUX_SYS_GETCWD: u64 = 79;
pub const LINUX_SYS_EXIT_GROUP: u64 = 231;

pub struct PosixBridge;

impl PosixBridge {
    /// Dispatch a Linux x86_64 ABI syscall from Ring 3 user application.
    pub unsafe fn dispatch(
        sys_num: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
        _arg4: u64,
        _arg5: u64,
        _arg6: u64,
    ) -> i64 {
        match sys_num {
            LINUX_SYS_READ => {
                let fd = arg1 as i32;
                let buf_ptr = arg2 as *mut u8;
                let count = arg3 as usize;
                if buf_ptr.is_null() || count == 0 { return 0; }

                if fd == 0 {
                    // Stdin from keyboard queue
                    let mut read_bytes = 0;
                    while read_bytes < count {
                        if let Some(ch) = crate::interrupts::KEYBOARD_QUEUE.lock().pop() {
                            *buf_ptr.add(read_bytes) = ch;
                            read_bytes += 1;
                        } else {
                            break;
                        }
                    }
                    return read_bytes as i64;
                }
                -9 // EBADF
            }
            LINUX_SYS_WRITE => {
                let fd = arg1 as i32;
                let buf_ptr = arg2 as *const u8;
                let count = arg3 as usize;
                if buf_ptr.is_null() || count == 0 { return 0; }

                if fd == 1 || fd == 2 {
                    // Stdout / Stderr to serial COM1 debug and console
                    let slice = core::slice::from_raw_parts(buf_ptr, count.min(4096));
                    if let Ok(s) = core::str::from_utf8(slice) {
                        crate::serial::serial_write_str(s);
                    }
                    return count as i64;
                }
                -9 // EBADF
            }
            LINUX_SYS_OPEN => {
                // Open path from VFS
                let path_ptr = arg1 as *const u8;
                if path_ptr.is_null() { return -14; } // EFAULT
                
                // Return a synthetic file descriptor
                3 // First user fd
            }
            LINUX_SYS_CLOSE => 0,
            LINUX_SYS_GETPID => 1001,
            LINUX_SYS_BRK => {
                // Dynamic memory expansion (heap allocation)
                let req_brk = arg1 as usize;
                if req_brk == 0 {
                    return 0x4000_0000; // Default user break address
                }
                req_brk as i64
            }
            LINUX_SYS_MMAP => {
                // Return anonymous user memory space
                let len = arg2 as usize;
                let layout = core::alloc::Layout::from_size_align(len.max(4096), 4096).unwrap();
                let mem = alloc::alloc::alloc_zeroed(layout);
                if mem.is_null() {
                    -12 // ENOMEM
                } else {
                    mem as i64
                }
            }
            LINUX_SYS_UNAME => {
                let buf = arg1 as *mut u8;
                if !buf.is_null() {
                    // Populate utsname structure: sysname, nodename, release, version, machine
                    let sysname = b"AtulyaOS-LinuxABI\0";
                    let nodename = b"axon-quantum\0";
                    let release = b"6.8.0-atulya\0";
                    let version = b"#1 SMP Sovereign Rust\0";
                    let machine = b"x86_64\0";

                    core::ptr::copy_nonoverlapping(sysname.as_ptr(), buf.add(0), sysname.len());
                    core::ptr::copy_nonoverlapping(nodename.as_ptr(), buf.add(65), nodename.len());
                    core::ptr::copy_nonoverlapping(release.as_ptr(), buf.add(130), release.len());
                    core::ptr::copy_nonoverlapping(version.as_ptr(), buf.add(195), version.len());
                    core::ptr::copy_nonoverlapping(machine.as_ptr(), buf.add(260), machine.len());
                }
                0
            }
            LINUX_SYS_EXIT | LINUX_SYS_EXIT_GROUP => {
                crate::serial::serial_write_line("POSIX Process Exited Cleanly.");
                0
            }
            _ => {
                crate::serial::serial_write_str("POSIX Syscall Unimplemented: ");
                crate::serial::serial_write_hex(sys_num);
                -38 // ENOSYS (Function not implemented)
            }
        }
    }
}
