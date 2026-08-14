use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileHandle(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    NotADirectory,
    NotAFile,
    NoSpace,
    InvalidPath,
    TooManyOpenFiles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenFlags {
    Read,
    Write,
    Create,
    Truncate,
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: usize,
}

pub trait FileSystem {
    fn open(&mut self, path: &str, flags: OpenFlags) -> Result<FileHandle, FsError>;
    fn read(&mut self, handle: FileHandle, buf: &mut [u8]) -> Result<usize, FsError>;
    fn write(&mut self, handle: FileHandle, buf: &[u8]) -> Result<usize, FsError>;
    fn close(&mut self, handle: FileHandle) -> Result<(), FsError>;
    fn ls(&self, path: &str) -> Result<Vec<DirEntry>, FsError>;
    fn mkdir(&mut self, path: &str) -> Result<(), FsError>;
    fn touch(&mut self, path: &str) -> Result<(), FsError>;
    fn rm(&mut self, path: &str) -> Result<(), FsError>;
    fn stat(&self, path: &str) -> Result<usize, FsError>;
}
