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

        RamFs {
            files,
            open_files: BTreeMap::new(),
            next_handle: 1,
        }
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
