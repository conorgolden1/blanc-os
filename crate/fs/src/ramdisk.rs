extern crate alloc;

use core::fmt::Debug;
use core::sync::atomic::{AtomicUsize, Ordering};
use printer::{print, println};

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::{collections::BTreeMap, sync::Weak, string::String};
use spin::{Mutex, RwLock, RwLockReadGuard};

use crate::inode::{Directory, FileContents, FileSystem, FileSystemError, INode, Metadata};
use crate::inode::{OFlags, FileTypeFlags};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref RAMFS : Arc<RamFs> = RamFs::new();
}


#[derive(Default)]
pub struct RamNode {
    id : usize,
    parent : Weak<Arc<dyn INode>>,
    name : String,
    children : BTreeMap<String, Arc<LockedRamINode>>,
    filesystem : Weak<RamFs>,
    file_type : FileTypeFlags,
    contents : FileContents,
}


pub struct LockedRamINode(RwLock<RamNode>);


impl LockedRamINode {
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
    fn metadata(&self) -> Result<Metadata, FileSystemError> {
        let this = self.0.read();
        let size = match &this.contents {
            FileContents::Content(bytes) => bytes.lock().len(), // Temporary value dropped and lock is unlocked!
            _ => 0x00,
        };
        Ok(Metadata::new(this.id, this.file_type, size))
    }

    fn read(&self, size : u32, buffer : &mut [u8]) {

        self.pread(0, size, buffer)
    }

    fn pread(&self, offset : u32, size : u32, buffer : &mut [u8]) {
        let this = self.0.read();
        if this.file_type != FileTypeFlags::FILE { return }
        match &this.contents {
            FileContents::Content(mut_cont) => {
                let content = mut_cont.lock();
                
                for index in 0..size as usize {
                    let y = content.get(offset as usize).unwrap();
                    buffer[index as usize] = *y; 
                }
            },
            FileContents::None => todo!(),
        }
    }

    fn write(&self, size : u32, buffer : &[u8]) {
        self.pwrite(0, size, buffer)
    }

    fn pwrite(&self, offset : u32, size : u32, buffer : &[u8]) {
        let this = self.0.read();
        if this.file_type != FileTypeFlags::FILE { return }
        match &this.contents {
            FileContents::Content(mut_cont) => {
                let mut content = mut_cont.lock();
                
                for index in 0..size as usize {
                    let y = content.get_mut(offset as usize).unwrap();
                    *y = buffer[index as usize];
                }
            }
            FileContents::None => todo!(),
        }
    }
    // TODO HANDLE OFLAGS
    fn open(&self, name : &str, o_flag : OFlags) -> Result<(), FileSystemError> {
        let mut this = self.0.write();

        if !(this.file_type != FileTypeFlags::DIRECTORY || this.file_type != FileTypeFlags::MOUNTPOINT) { 
            return Err(FileSystemError::EntryNotFound) 
        }
        
        let empty_contents = Mutex::new(Vec::<u8>::new());

        let new_file = RAMFS.allocate_inode(name, FileTypeFlags::FILE, FileContents::Content(empty_contents));
        
        this.children.try_insert(String::from(name), new_file).ok();

        Ok(())
    }

    fn close(&self) -> Result<(), FileSystemError> {
        unimplemented!("No need to implement close in a RAM format since we don't flush to disk")
    }

    fn mkdir(&self, name : &str) -> Result<(), FileSystemError> {
        let mut this = self.0.write();

        if !(this.file_type != FileTypeFlags::DIRECTORY || this.file_type != FileTypeFlags::MOUNTPOINT) { 
            return Err(FileSystemError::EntryNotFound) 
        }

        let new_directory =  RAMFS.allocate_inode(name, FileTypeFlags::DIRECTORY, FileContents::None);

        this.children.try_insert(String::from(name), new_directory).ok();
        Ok(())
    }

    fn find_dir<'a>(&'a self, dir : Arc<&'a Directory<'a>>, name : &str) -> Result<Directory, FileSystemError> {
        let this = self.0.read();
        let child = this.children.get(name).ok_or(FileSystemError::EntryNotFound)?;
        let clone = dir.clone();
        Ok(Directory::new(Some(clone), child.clone(), String::from(name)))
    }

    fn filesystem(&self) -> Weak<dyn FileSystem> {
        self.0.read().filesystem.clone()
    }
}

pub struct RamFs {
    pub root_inode : Arc<LockedRamINode>,
    root_dir : Directory<'static>,
    next_id : AtomicUsize,
}

impl RamFs {
    pub fn new() -> Arc<Self> { 
        let root_inode = Arc::new(LockedRamINode::new(
            RamNode {
                parent: Weak::default(),
                name: String::from("/"),
                filesystem: Weak::default(),
                children: BTreeMap::new(),
                id: 0,
                contents : FileContents::None,
                file_type : FileTypeFlags::MOUNTPOINT,
            }));


        
        let root_dir = Directory::new(None, root_inode.clone(), String::from("/"));


        let ramfs = Arc::new(Self {
            root_inode,
            root_dir,
            next_id : AtomicUsize::new(0x00),
        });
        let copy: Arc<dyn FileSystem> = ramfs.clone();

        ramfs.root_dir.filesystem.call_once( || Arc::downgrade(&copy));


        ramfs
    }

    fn allocate_inode(&self, name : &str, file_type: FileTypeFlags, contents: FileContents) -> Arc<LockedRamINode> {
        Arc::new(
            LockedRamINode::new(
                {
                    RamNode {
                        parent: Weak::default(),
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

    let root = root.0.read();
    
    for (_, child) in root.children.iter() {
        println!("{:#?}", child);

        if child.0.read().file_type == FileTypeFlags::DIRECTORY {
            print_children(child)
        }
    }
    
}


fn print_children(parent : &LockedRamINode) {
    println!("{:#?}", parent);

    let node = parent.0.read();
    
    for (_, child) in node.children.iter() {
        println!("{:#?}", child);

        if child.0.read().file_type == FileTypeFlags::DIRECTORY {
            print_children(child)
        }
    }
    
    
}