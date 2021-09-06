//! TODO DOCUMENT
extern crate alloc;

use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::{string::{ToString}, sync::Arc, vec::Vec};
use spin::RwLock;
use crate::inode::{FileSystemError, INode, OFlags};



pub struct FileHandle {
    pub fd: usize,
    pub inode : Arc<dyn INode>,
    pub offset: AtomicUsize,
    pub flags : OFlags,
}

impl FileHandle {
    /// Create a new file handle
    pub fn new(fd: usize, inode: Arc<dyn INode>, flags: OFlags) -> Self {
         Self { 
             fd, 
             inode : inode.clone(), 
             offset : AtomicUsize::new(0), 
             flags 
            } 
        }

    pub fn read(&self, buffer: &mut [u8], count : usize) {
        let offset = self.offset.load(Ordering::SeqCst);
        self.inode.pread(offset, count, buffer);
    }

    pub fn write(&self, buffer: &mut [u8], count : usize) {
        let offset = self.offset.load(Ordering::SeqCst);
        self.inode.pwrite(offset, count, buffer)
    }


}

type LockedFileHandle = RwLock<Vec<Option<Arc<FileHandle>>>>;

pub struct FileTable(LockedFileHandle);

impl FileTable {
    pub fn new() -> Self {
        let mut table = Vec::new();
        table.resize(256, None);

        Self(RwLock::new(table))
    }

    pub fn get_handle(&self, fd: usize) -> Option<Arc<FileHandle>> {
        let files = self.0.read();
        match &files.get(fd) {
            Some(Some(handle)) => Some(handle.clone()),
            _ => None,
        }
    }

    pub fn open_file(&self, node : Arc<dyn INode>, flags: OFlags) -> Result<usize, FileSystemError> {
        let mut files = self.0.write();

        if let Some((i, f)) = files.iter_mut().enumerate().find(|e| e.1.is_none()) {
            let handle = Arc::new(FileHandle::new(i, node, flags));

            handle.inode.open(i.to_string().as_str(), flags)?;
            *f = Some(handle);
            return Ok(i)
        } 

        Err(FileSystemError::Busy)
    }
}

impl Default for FileTable {
    fn default() -> Self {
        Self::new()
    }
}