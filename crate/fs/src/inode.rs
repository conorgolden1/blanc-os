
extern crate alloc;





use alloc::{sync::Weak, sync::Arc, vec::Vec};
use bitflags::bitflags;
use spin::{Mutex};


pub trait INode: Send + Sync {
    /// Returns the Metadata of the Indoe
    fn metadata(&self) -> Result<Metadata, FileSystemError>;

    /// Read bytes into file
    fn read(&self,  count : usize, buffer : &mut [u8]);

    /// Read bytes into file at an offset
    fn pread(&self, offset : usize, count : usize, buffer : &mut [u8]);

    /// Write bytes into file
    fn write(&self, count : usize, buffer : &[u8]);

    /// Write bytes into file at an offset
    fn pwrite(&self, offset : usize, count : usize, buffer : &[u8]);

    /// Open a file returning the file descriptor
    fn open(&self, name : &str, o_flag : OFlags) -> Result<(), FileSystemError>;

    /// Close a file
    fn close(&self) -> Result<(), FileSystemError>;

    /// Creates a new directory in the filesystem with a given name
    fn mkdir(&self, name : &str) -> Result<(), FileSystemError>;

    /// Finds a directory in the filesystem
    fn find_dir(&self, name :&str) -> Result<Arc<dyn INode>, FileSystemError>;

    /// Return the filesystem type that this inode belongs too
    fn filesystem(&self) -> Weak<dyn FileSystem>; 

}

pub struct Metadata {
    /// Unique ID of a file's metadata
    id : usize,

    /// See ['FileTypeFlags']
    file_type : FileTypeFlags,

    /// Size of the File that the Inode represents, 0 if not file
    size : usize,

    
}

impl Metadata {
    pub fn new(id: usize, file_type: FileTypeFlags, size: usize) -> Self { 
        Self { id, file_type, size } 
    }

    
    /// Identifies if the file is a directory type, true if it is, false otherwise
    pub fn is_directory(&self) -> bool {
        self.file_type == FileTypeFlags::DIRECTORY
    }

    /// Get a reference to the metadata's id.
    pub fn id(&self) -> &usize {
        &self.id
    }

    /// Get a reference to the metadata's file type.
    pub fn file_type(&self) -> &FileTypeFlags {
        &self.file_type
    }

    /// Get a reference to the metadata's size.
    pub fn size(&self) -> &usize {
        &self.size
    }
}














/// Trait representing a whole filesystem starting from the root node
pub trait FileSystem : Send + Sync{
    fn root_dir(&self) -> Arc<dyn INode>; 
}










/// Enum representing the inner contents of a file. The file contents depend on the
/// file type of the inode.
#[derive(Debug)]
pub enum FileContents {
    /// This variant expresses a *normal file* (akin: A file that actually stores data
    /// in bytes) and is protected by a spin lock.
    Content(Mutex<Vec<u8>>),

    // /// If the file type of the inode is [FileType::Device], in that case this variant
    // /// is used.
    // Device(Arc<DevINode>),

    /// This file does *not* and *cannot* have any contents in bytes. This is useful
    /// in the cases of directories.
    None,
}




#[derive(Debug)]
pub enum FileSystemError {
    NotSupported,
    EntryExists,
    EntryNotFound,
    Busy,
}



impl Default for FileContents {
    fn default() -> Self {
        Self::Content(Mutex::new(Vec::new()))
    }
}





bitflags! {
    /// Virtual File System Node Flags
    /// Each flag indicates the type of Node that is being represented in the filesystem
    pub struct FileTypeFlags : u32 {
        /// Inode is a file type
        const FILE = 0x01;
        /// Inode is a directory type
        const DIRECTORY = 0x02;
        /// A file that refers to a device (such as a terminal device file)
        /// or that has special properties (such as /dev/null).
        const CHARDEVICE = 0x03;
        /// A file that refers to a device. 
        ///
        /// A block special file is normally distinguished from a character special 
        /// file by providing access to the device in a manner such that the 
        /// hardware characteristics of the device are not visible.
        const BLOCKDEVICE = 0x04;
        /// An object identical to a FIFO which has no links in the file hierarchy.
        const PIPE = 0x05;
        /// A type of file with the property that when the file is encountered during 
        /// pathname resolution, a string stored by the file is used to modify the 
        /// pathname resolution.
        const SYMLINK = 0x06;
        /// Either the system root directory or a directory
        const MOUNTPOINT = 0x08;
    }

    /// Flags that determine the method in which the file is to be opened 
    /// (whether it should be read only, read/write, whether it should be cleared when opened, etc).
    pub struct OFlags : u8 {
        /// Open the file so that it is read only.
        const O_RDONLY = 1;
        /// Open the file so that it is write only.
        const O_WRONLY = 1 << 1;
        /// Open the file so that it can be read from and written to.
        const O_RDWR = 1 << 2; 
        /// Append new information to the end of the file.
        const O_APPEND = 1 << 3;
        /// Initially clear all data from the file.
        const O_TRUNC = 1 << 4;
        /// If the file does not exist, create it. If the O_CREAT option is used, then you must include the third parameter.
        const O_CREAT = 1 << 5;
        /// Combined with the O_CREAT option, it ensures that the caller must create the file. If the file already exists, the call will fail.
        const O_EXCL = 1 << 6;
    }
}

impl Default for FileTypeFlags {
    fn default() -> Self {
        FileTypeFlags::DIRECTORY
    }
}


   

