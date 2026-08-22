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
    pub fn dispatch(
        sys_num: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
        arg4: u64,
        _arg5: u64,
        _arg6: u64,
    ) -> i64 {
        dispatch_posix_syscall(sys_num as usize, arg1 as usize, arg2 as usize, arg3 as usize, arg4 as usize) as i64
    }
}

/// Verify that a buffer pointer and length reside in valid canonical user address space.
#[inline]
pub fn is_valid_user_ptr(ptr: usize, len: usize) -> bool {
    if ptr < 0x1000 || len == 0 {
        return false;
    }
    match ptr.checked_add(len) {
        Some(end) => end <= 0x0000_7FFF_FFFF_FFFF, // Canonical Ring 3 user boundary
        None => false,
    }
}

pub fn sys_read(fd: usize, buf: usize, count: usize) -> isize {
    if !is_valid_user_ptr(buf, count) {
        return -14; // EFAULT
    }
    let buf_ptr = buf as *mut u8;
    if fd == 0 {
        // Stdin from keyboard queue
        let mut read_bytes = 0;
        x86_64::instructions::interrupts::without_interrupts(|| {
            let mut q = crate::interrupts::KEYBOARD_QUEUE.lock();
            while read_bytes < count {
                if let Some(ch) = q.pop() {
                    unsafe { *buf_ptr.add(read_bytes) = ch; }
                    read_bytes += 1;
                } else {
                    break;
                }
            }
        });
        return read_bytes as isize;
    }
    -9 // EBADF
}

pub fn sys_write(fd: usize, buf: usize, count: usize) -> isize {
    if !is_valid_user_ptr(buf, count) {
        return -14; // EFAULT
    }
    let buf_ptr = buf as *const u8;
    if fd == 1 || fd == 2 {
        let slice = unsafe { core::slice::from_raw_parts(buf_ptr, count.min(8192)) };
        if let Ok(s) = core::str::from_utf8(slice) {
            crate::serial::serial_write_line(s);
        }
        return count as isize;
    }
    -9 // EBADF
}

pub fn dispatch_posix_syscall(sys_num: usize, arg1: usize, arg2: usize, arg3: usize, _arg4: usize) -> isize {
    match sys_num as u64 {
        LINUX_SYS_READ => sys_read(arg1, arg2, arg3),
        LINUX_SYS_WRITE => sys_write(arg1, arg2, arg3),
        LINUX_SYS_OPEN => 3, // Virtual standard file descriptor
        LINUX_SYS_CLOSE => 0,
        LINUX_SYS_GETPID => 100, // Ring 3 main process PID
        LINUX_SYS_BRK => {
            let addr = arg1;
            if addr == 0 {
                0x0000_7000_0000_0000 // Return current heap break base
            } else {
                addr as isize // Grow user heap
            }
        }
        LINUX_SYS_UNAME => {
            if !is_valid_user_ptr(arg1, 390) {
                return -14; // EFAULT
            }
            let ptr = arg1 as *mut u8;
            let uts_sysname = b"AtulyaOS\0";
            let uts_release = b"1.0.0-sovereign\0";
            let uts_version = b"Sovereign Kernel (Freestanding x86_64)\0";
            let uts_machine = b"x86_64\0";
            unsafe {
                core::ptr::copy_nonoverlapping(uts_sysname.as_ptr(), ptr, uts_sysname.len());
                core::ptr::copy_nonoverlapping(uts_release.as_ptr(), ptr.add(65), uts_release.len());
                core::ptr::copy_nonoverlapping(uts_version.as_ptr(), ptr.add(130), uts_version.len());
                core::ptr::copy_nonoverlapping(uts_machine.as_ptr(), ptr.add(195), uts_machine.len());
            }
            0
        }
        LINUX_SYS_EXIT | LINUX_SYS_EXIT_GROUP => {
            crate::serial::serial_write_line("POSIX Process Exited.");
            0
        }
        _ => -38, // ENOSYS (Function not implemented)
    }
}
