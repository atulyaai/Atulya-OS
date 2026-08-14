use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use super::vfs::{FsError, FileHandle, DirEntry, FileSystem, OpenFlags};

const MAX_FILES: usize = 256;
const MAX_OPEN: usize = 32;

struct Inode {
    data: Vec<u8>,
    is_dir: bool,
}

pub struct RamFs {
    files: BTreeMap<String, Inode>,
    open_files: BTreeMap<FileHandle, OpenFile>,
    next_handle: u64,
}

struct OpenFile {
    path: String,
    offset: usize,
    flags: OpenFlags,
}

fn normalize_path(path: &str) -> Result<String, FsError> {
    let path = path.trim();
    if path.is_empty() || path == "/" {
        return Ok(String::from("/"));
    }
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Ok(String::from("/"));
    }
    Ok(format!("/{}", parts.join("/")))
}

impl RamFs {
    pub fn new() -> Self {
        let mut files = BTreeMap::new();
        files.insert(String::from("/"), Inode { data: Vec::new(), is_dir: true });

        // Create default filesystem contents
        let etc_entries = ["hostname", "motd", "version"];
        for name in &etc_entries {
            files.insert(format!("/etc/{}", name), Inode { data: Vec::new(), is_dir: false });
        }
        files.insert(String::from("/etc/hostname"), Inode {
            data: b"atulyaos".to_vec(),
            is_dir: false,
        });
        files.insert(String::from("/etc/motd"), Inode {
            data: b"Welcome to AtulyaOS\nThe Intent Computer System\n".to_vec(),
            is_dir: false,
        });
        files.insert(String::from("/etc/version"), Inode {
            data: b"0.3.0\n".to_vec(),
            is_dir: false,
        });
        files.insert(String::from("/tmp"), Inode { data: Vec::new(), is_dir: true });
        files.insert(String::from("/home"), Inode { data: Vec::new(), is_dir: true });
        files.insert(String::from("/home/guest"), Inode { data: Vec::new(), is_dir: true });
        files.insert(String::from("/home/guest/readme.txt"), Inode {
            data: b"AtulyaOS - The Intent Computer System\nFuses macOS dock/menubar, Linux terminal,\nand Windows tiling window layouts.\n".to_vec(),
            is_dir: false,
        });
        files.insert(String::from("/user"), Inode { data: Vec::new(), is_dir: true });
        files.insert(String::from("/user/atul"), Inode { data: Vec::new(), is_dir: true });
        files.insert(String::from("/user/atul/welcome.txt"), Inode {
            data: b"Welcome Atul to your personal Quantum OS workspace.\nAll subsystems (Memory, Voice, Vision, Skills, Security) are active.\n".to_vec(),
            is_dir: false,
        });
        files.insert(String::from("/user/atul/identity.json"), Inode {
            data: b"{\n  \"user\": \"Atul\",\n  \"clearance\": \"Admin\",\n  \"system_id\": \"AXON-7\",\n  \"status\": \"Authorized\"\n}\n".to_vec(),
            is_dir: false,
        });
        files.insert(String::from("/system"), Inode { data: Vec::new(), is_dir: true });
        files.insert(String::from("/system/config.sys"), Inode {
            data: b"OS_THEME=cyberpunk_cyan\nREFRESH_RATE=60\nQUANTUM_MESH=enabled\n".to_vec(),
            is_dir: false,
        });
        files.insert(String::from("/docs"), Inode { data: Vec::new(), is_dir: true });
        files.insert(String::from("/docs/spec.pdf"), Inode {
            data: b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Count 1 >>\nendobj\nxref\n0 3\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n".to_vec(),
            is_dir: false,
        });
        files.insert(String::from("/media"), Inode { data: Vec::new(), is_dir: true });
        files.insert(String::from("/media/avatar.png"), Inode {
            data: alloc::vec![
                0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, // PNG Header
                0x00, 0x00, 0x00, 0x0D, b'I', b'H', b'D', b'R',
                0x00, 0x00, 0x01, 0x00, // 256 px width
                0x00, 0x00, 0x01, 0x00, // 256 px height
                0x08, 0x06, 0x00, 0x00, 0x00, // 8-bit RGBA
                0x00, 0x00, 0x00, 0x00,
            ],
            is_dir: false,
        });
        files.insert(String::from("/media/audio.wav"), Inode {
            data: alloc::vec![
                b'R', b'I', b'F', b'F', 0x24, 0x00, 0x00, 0x00, b'W', b'A', b'V', b'E',
                b'f', b'm', b't', b' ', 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, // 2 channels
                0x44, 0xAC, 0x00, 0x00, // 44100 Hz
                0x10, 0xB1, 0x02, 0x00, 0x04, 0x00, 0x10, 0x00, // 16-bit
                b'd', b'a', b't', b'a', 0x00, 0x00, 0x00, 0x00,
            ],
            is_dir: false,
        });

        let mut fs = RamFs {
            files,
            open_files: BTreeMap::new(),
            next_handle: 1,
        };

        // Attempt to restore persistent state from ATA hard disk
        let _ = fs.restore_from_disk();
        fs
    }

    /// Synchronize RAM filesystem state to ATA disk (LBA 2048).
    pub fn sync_to_disk(&self) -> Result<usize, &'static str> {
        let disk = crate::fs::ata::DISK.lock();
        if !disk.is_available {
            return Err("ATA disk not available");
        }

        let mut sector = [0u8; 512];
        // Superblock Header
        sector[0..12].copy_from_slice(b"ATULYA_FS_V1");
        let file_count = self.files.len() as u32;
        sector[12..16].copy_from_slice(&file_count.to_le_bytes());

        // Write Superblock at LBA 2048
        disk.write_sector(2048, &sector)?;

        let mut current_lba = 2049;
        for (path, inode) in &self.files {
            let mut entry_sec = [0u8; 512];
            let path_bytes = path.as_bytes();
            let plen = path_bytes.len().min(128);
            entry_sec[0] = plen as u8;
            entry_sec[1] = if inode.is_dir { 1 } else { 0 };
            entry_sec[2..6].copy_from_slice(&(inode.data.len() as u32).to_le_bytes());
            entry_sec[8..8 + plen].copy_from_slice(&path_bytes[..plen]);

            disk.write_sector(current_lba, &entry_sec)?;
            current_lba += 1;

            // Write file data chunks
            let mut d_offset = 0;
            while d_offset < inode.data.len() {
                let mut data_sec = [0u8; 512];
                let chunk = (inode.data.len() - d_offset).min(512);
                data_sec[..chunk].copy_from_slice(&inode.data[d_offset..d_offset + chunk]);
                disk.write_sector(current_lba, &data_sec)?;
                current_lba += 1;
                d_offset += chunk;
            }
        }

        crate::serial::serial_write_line("VFS: Successfully synchronized state to ATA persistent disk.");
        Ok(self.files.len())
    }

    /// Restore filesystem state from ATA disk.
    pub fn restore_from_disk(&mut self) -> Result<usize, &'static str> {
        let disk = crate::fs::ata::DISK.lock();
        if !disk.is_available {
            return Err("ATA disk not available");
        }

        let mut sector = [0u8; 512];
        disk.read_sector(2048, &mut sector)?;

        if &sector[0..12] != b"ATULYA_FS_V1" {
            // First time boot - initialize disk format
            drop(disk);
            let _ = self.sync_to_disk();
            return Ok(0);
        }

        let file_count = u32::from_le_bytes([sector[12], sector[13], sector[14], sector[15]]) as usize;
        let mut restored_files = BTreeMap::new();
        let mut current_lba = 2049;

        for _ in 0..file_count {
            let mut entry_sec = [0u8; 512];
            disk.read_sector(current_lba, &mut entry_sec)?;
            current_lba += 1;

            let plen = entry_sec[0] as usize;
            if plen == 0 || plen > 128 {
                continue;
            }
            let is_dir = entry_sec[1] != 0;
            let file_size = u32::from_le_bytes([entry_sec[2], entry_sec[3], entry_sec[4], entry_sec[5]]) as usize;
            
            let path = match core::str::from_utf8(&entry_sec[8..8 + plen]) {
                Ok(s) => String::from(s),
                Err(_) => continue,
            };

            let mut data = Vec::with_capacity(file_size);
            let mut d_offset = 0;
            while d_offset < file_size {
                let mut data_sec = [0u8; 512];
                disk.read_sector(current_lba, &mut data_sec)?;
                current_lba += 1;

                let chunk = (file_size - d_offset).min(512);
                data.extend_from_slice(&data_sec[..chunk]);
                d_offset += chunk;
            }

            restored_files.insert(path, Inode { data, is_dir });
        }

        self.files = restored_files;
        crate::serial::serial_write_line("VFS: Successfully restored filesystem from ATA disk.");

        Ok(self.files.len())
    }

    fn parent_path(path: &str) -> Option<String> {
        let trimmed = path.trim_end_matches('/');
        trimmed.rfind('/').map(|i| {
            if i == 0 {
                String::from("/")
            } else {
                trimmed[..i].to_string()
            }
        })
    }

    fn filename(path: &str) -> Option<&str> {
        let trimmed = path.trim_end_matches('/');
        trimmed.rfind('/').map(|i| &trimmed[i + 1..])
    }
}

impl FileSystem for RamFs {
    fn open(&mut self, path: &str, flags: OpenFlags) -> Result<FileHandle, FsError> {
        let path = normalize_path(path)?;

        match flags {
            OpenFlags::Create | OpenFlags::Write | OpenFlags::Truncate => {
                // Ensure parent exists
                if let Some(parent) = Self::parent_path(&path) {
                    if !self.files.contains_key(&parent) {
                        return Err(FsError::NotFound);
                    }
                }
            }
            OpenFlags::Read => {
                if !self.files.contains_key(&path) {
                    return Err(FsError::NotFound);
                }
            }
        }

        if let Some(inode) = self.files.get(&path) {
            if inode.is_dir {
                return Err(FsError::NotAFile);
            }
        } else if flags == OpenFlags::Read {
            return Err(FsError::NotFound);
        } else {
            self.files.insert(path.clone(), Inode { data: Vec::new(), is_dir: false });
        }

        if flags == OpenFlags::Truncate {
            if let Some(inode) = self.files.get_mut(&path) {
                inode.data.clear();
            }
        }

        let handle = FileHandle(self.next_handle);
        self.next_handle += 1;
        self.open_files.insert(handle, OpenFile { path, offset: 0, flags });
        Ok(handle)
    }

    fn read(&mut self, handle: FileHandle, buf: &mut [u8]) -> Result<usize, FsError> {
        let open_file = self.open_files.get(&handle).ok_or(FsError::NotFound)?;
        let path = open_file.path.clone();
        let offset = open_file.offset;

        let inode = self.files.get(&path).ok_or(FsError::NotFound)?;
        let available = inode.data.len().saturating_sub(offset);
        let to_read = buf.len().min(available);

        buf[..to_read].copy_from_slice(&inode.data[offset..offset + to_read]);

        if let Some(of) = self.open_files.get_mut(&handle) {
            of.offset += to_read;
        }

        Ok(to_read)
    }

    fn write(&mut self, handle: FileHandle, buf: &[u8]) -> Result<usize, FsError> {
        let open_file = self.open_files.get(&handle).ok_or(FsError::NotFound)?;
        if open_file.flags == OpenFlags::Read {
            return Err(FsError::PermissionDenied);
        }

        let path = open_file.path.clone();
        let offset = open_file.offset;

        let inode = self.files.get_mut(&path).ok_or(FsError::NotFound)?;
        let new_len = offset + buf.len();
        if new_len > inode.data.len() {
            inode.data.resize(new_len, 0);
        }
        inode.data[offset..offset + buf.len()].copy_from_slice(buf);

        if let Some(of) = self.open_files.get_mut(&handle) {
            of.offset += buf.len();
        }

        Ok(buf.len())
    }

    fn close(&mut self, handle: FileHandle) -> Result<(), FsError> {
        self.open_files.remove(&handle).ok_or(FsError::NotFound)?;
        Ok(())
    }

    fn ls(&self, path: &str) -> Result<Vec<DirEntry>, FsError> {
        let path = normalize_path(path)?;

        if !self.files.contains_key(&path) {
            return Err(FsError::NotFound);
        }

        let prefix = if path == "/" {
            String::from("/")
        } else {
            format!("{}/", path)
        };

        let mut entries = Vec::new();
        for (key, inode) in &self.files {
            if key == &path {
                continue;
            }
            if key.starts_with(&prefix) {
                let remainder = &key[prefix.len()..];
                if !remainder.contains('/') {
                    entries.push(DirEntry {
                        name: remainder.to_string(),
                        is_dir: inode.is_dir,
                        size: inode.data.len(),
                    });
                }
            }
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    fn mkdir(&mut self, path: &str) -> Result<(), FsError> {
        let path = normalize_path(path)?;

        if self.files.contains_key(&path) {
            return Err(FsError::AlreadyExists);
        }

        if let Some(parent) = Self::parent_path(&path) {
            if !self.files.contains_key(&parent) {
                return Err(FsError::NotFound);
            }
        }

        self.files.insert(path, Inode { data: Vec::new(), is_dir: true });
        Ok(())
    }

    fn touch(&mut self, path: &str) -> Result<(), FsError> {
        let path = normalize_path(path)?;

        if self.files.contains_key(&path) {
            return Ok(());
        }

        if let Some(parent) = Self::parent_path(&path) {
            if !self.files.contains_key(&parent) {
                return Err(FsError::NotFound);
            }
        }

        self.files.insert(path, Inode { data: Vec::new(), is_dir: false });
        Ok(())
    }

    fn rm(&mut self, path: &str) -> Result<(), FsError> {
        let path = normalize_path(path)?;

        if path == "/" {
            return Err(FsError::PermissionDenied);
        }

        let inode = self.files.get(&path).ok_or(FsError::NotFound)?;
        if inode.is_dir {
            let prefix = format!("{}/", path);
            let has_children = self.files.keys().any(|k| k.starts_with(&prefix) && k != &path);
            if has_children {
                return Err(FsError::PermissionDenied);
            }
        }

        self.files.remove(&path);
        Ok(())
    }

    fn stat(&self, path: &str) -> Result<usize, FsError> {
        let path = normalize_path(path)?;
        self.files.get(&path)
            .map(|inode| inode.data.len())
            .ok_or(FsError::NotFound)
    }
}
