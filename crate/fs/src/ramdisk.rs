//! Simulated Ram disk for startup functionality
extern crate alloc;

use core::fmt::Debug;
use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::{collections::BTreeMap, sync::Weak, string::String};

use crate::inode::{FileContents, FileSystem, FileSystemError, INode, Metadata};
use crate::inode::{OFlags, FileTypeFlags};

use printer::{print, println};
use spin::{Mutex, RwLock};
use lazy_static::lazy_static;

lazy_static! {
    /// Global Reference to the FileSystem on RAM
    pub static ref RAMFS : Arc<RamFs> = RamFs::new();
}

/// Structure which contains identifiers, links, data, and metadata about a file
/// or directory.
#[derive(Default)]
pub struct RamNode {
    id : usize,
    name : String,
    children : BTreeMap<String, Arc<LockedRamINode>>,
    filesystem : Weak<RamFs>,
    file_type : FileTypeFlags,
    contents : FileContents,
}

#[allow(clippy::from_over_into)]
/// TODO? DOC?
impl Into<Metadata> for RamNode {
    fn into(self) -> Metadata {
        if self.file_type == FileTypeFlags::FILE {
            match self.contents {
                FileContents::Content(contents) => {
                    return Metadata::new(self.id, self.file_type, contents.lock().len())
                },
                _ => unreachable!()
            }
        } else {
            Metadata::new(self.id, self.file_type, 0)
        }
        
    }
}

/// Wrapper Struct around a Read/Write Lock on a [RamNode]
/// 
/// The wrapper struct implements [INode] functionality for
/// filesystem interfacing
pub struct LockedRamINode(RwLock<RamNode>);


impl LockedRamINode {
    /// Create a new LockedRamNode from a [RwLock]
    pub fn new(ram_node : RamNode) -> LockedRamINode {
        Self ( RwLock::new(ram_node) )
    }
}

impl Debug for LockedRamINode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let this = self.0.read();
        f.debug_struct("RamNode")
            .field("id", &this.id)
            .field("name", &this.name)
            .field("file_type", &this.file_type)
            .field("contents", &this.contents)
            .finish()
    }
}



impl INode for LockedRamINode {
    /// Interpret [Metadata] from Ramnode information
    fn metadata(&self) -> Result<Metadata, FileSystemError> {
        let this = self.0.read();
        let size = match &this.contents {
            FileContents::Content(bytes) => bytes.lock().len(), // Temporary value dropped and lock is unlocked!
            _ => 0x00,
        };
        Ok(Metadata::new(this.id, this.file_type, size))
    }

    /// Calls [pread](LockedRamINode::pread()) at offset 0 and reads
    /// size n bytes into the buffer
    fn read(&self, count : usize, buffer : &mut [u8]) {
        self.pread(0, count, buffer)
    }

    /// Read n size bytes into the buffer at an offset
    fn pread(&self, offset : usize, count : usize, buffer : &mut [u8]) {
        let this = self.0.read();
        if this.file_type != FileTypeFlags::FILE { return }
        match &this.contents {
            FileContents::Content(mut_cont) => {
                let content = mut_cont.lock();
                
                for index in 0..count as usize {
                    let y = content.get(offset as usize).unwrap();
                    buffer[index as usize] = *y; 
                }
            },
            FileContents::None => unreachable!("File Type is File but doesn't have content"),
        }
    }

    /// Calls [pwrite](LockedRamINode::wprite()) at offset 0 and writes
    /// size n bytes from buffer into file
    fn write(&self, count : usize, buffer : &[u8]) {
        self.pwrite(0, count, buffer)
    }

    /// Write n size bytes into the file from the buffer at an offset
    fn pwrite(&self, offset : usize, count : usize, buffer : &[u8]) {
        let this = self.0.read();
        if this.file_type != FileTypeFlags::FILE { return }
        match &this.contents {
            FileContents::Content(mut_cont) => {
                let mut content = mut_cont.lock();
                
                for index in 0..count as usize {
                    let y = content.get_mut(offset as usize).unwrap();
                    *y = buffer[index as usize];
                }
            }
            FileContents::None => todo!(),
        }
    }

    // TODO HANDLE OFLAGS
    /// Open a new file with a given name, if caller is not a directory 
    /// Err(EntryNotFound) is returned. Does not replace a file with the same
    /// name
    fn open(&self, name : &str, _ : OFlags) -> Result<(), FileSystemError> {
        let mut this = self.0.write();

        if !(this.file_type != FileTypeFlags::DIRECTORY || this.file_type != FileTypeFlags::MOUNTPOINT) { 
            return Err(FileSystemError::EntryNotFound) 
        }
        
        let empty_contents = Mutex::new(Vec::<u8>::new());

        let new_file = RAMFS.allocate_inode(name, FileTypeFlags::FILE, FileContents::Content(empty_contents));
        
        this.children.try_insert(String::from(name), new_file).ok();

        Ok(())
    }

    #[doc(hidden)]
    fn close(&self) -> Result<(), FileSystemError> {
        unimplemented!("No need to implement close in a RAM format since we don't flush to disk")
    }

    /// Create and link a new directory from the caller.
    ///
    /// Caller must be a directory else Err(EntryNotFound) is returned.
    /// Does not replace a directory with the same name
    /// TODO ADD SELF REFRENCE SYMLINK AND PARENT REFERENCE SYMLINK
    fn mkdir(&self, name : &str) -> Result<(), FileSystemError> {
        let mut this = self.0.write();

        if !(this.file_type != FileTypeFlags::DIRECTORY || this.file_type != FileTypeFlags::MOUNTPOINT) { 
            return Err(FileSystemError::EntryNotFound) 
        }

        let new_directory =  RAMFS.allocate_inode(name, FileTypeFlags::DIRECTORY, FileContents::None);

        this.children.try_insert(String::from(name), new_directory).ok();
        Ok(())
    }

    /// Find and return the directory inside this directory,
    ///
    /// Caller must be a directory else Err(EntryNotFound) is returned.
    /// 
    fn find_dir(&self, name : &str) -> Result<Arc<dyn INode>, FileSystemError> {
        let this = self.0.read();
        let child = this.children.get(name).ok_or(FileSystemError::EntryNotFound)?;
        if child.0.read().file_type != FileTypeFlags::DIRECTORY { 
            return Err(FileSystemError::EntryNotFound) 
        }
        Ok(child.clone())
    }

    fn filesystem(&self) -> Weak<dyn FileSystem> {
        self.0.read().filesystem.clone()
    }
}

pub struct RamFs {
    pub root_inode : Arc<LockedRamINode>,
    next_id : AtomicUsize,
}

impl RamFs {
    pub fn new() -> Arc<Self> { 
        let root_inode = Arc::new(LockedRamINode::new(
            RamNode {
                name: String::from("/"),
                filesystem: Weak::default(),
                children: BTreeMap::new(),
                id: 0,
                contents : FileContents::None,
                file_type : FileTypeFlags::MOUNTPOINT,
            }));

       Arc::new(Self {
            root_inode,
            next_id : AtomicUsize::new(0x00),
        })
    }

    fn allocate_inode(&self, name : &str, file_type: FileTypeFlags, contents: FileContents) -> Arc<LockedRamINode> {
        Arc::new(
            LockedRamINode::new(
                {
                    RamNode {
                        name: String::from(name),
                        filesystem: Weak::default(),
                        children: BTreeMap::new(),
                        id: self.next_id.fetch_add(1, Ordering::SeqCst),
                        contents,
                        file_type,
                    }
                }
            )
        )
    }

    
}

impl Debug for RamFs {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RamFs").field("root_inode", &self.root_inode).field("next_id", &self.next_id).finish()
    }
}


impl FileSystem for RamFs {
    fn root_dir(&self) -> Arc<dyn INode> {
        self.root_inode.clone()
    }
}


unsafe impl Send for RamFs {

}
unsafe impl Sync for RamFs {

}

unsafe impl Send for LockedRamINode {

}
unsafe impl Sync for LockedRamINode {

}


pub fn print_filesystem() {
    let root = &RAMFS.root_inode;
    println!("{:#?}", root);
    print_children(root);
    
}


fn print_children(parent : &LockedRamINode) {
    let node = parent.0.read();
    for (_, child) in node.children.iter() {
        println!("{:#?}", child);

        if child.0.read().file_type == FileTypeFlags::DIRECTORY {
            print_children(child)
        }
    }
}