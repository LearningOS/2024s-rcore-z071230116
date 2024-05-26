//! `Arc<Inode>` -> `OSInodeInner`: In order to open files concurrently
//! we need to wrap `Inode` into `Arc`,but `Mutex` in `Inode` prevents
//! file systems from being accessed simultaneously
//!
//! `UPSafeCell<OSInodeInner>` -> `OSInode`: for static `ROOT_INODE`,we
//! need to wrap `OSInodeInner` into `UPSafeCell`
use super::{File, StatMode,Stat};
use crate::drivers::BLOCK_DEVICE;
use crate::mm::UserBuffer;
use crate::sync::UPSafeCell;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::*;
use easy_fs::{EasyFileSystem, Inode};
use lazy_static::*;

/// inode in memory
/// A wrapper around a filesystem inode
/// to implement File trait atop
pub struct OSInode {
    readable: bool,
    writable: bool,
    inner: UPSafeCell<OSInodeInner>,    
}
/// The OS inode inner in 'UPSafeCell'
pub struct OSInodeInner {
    offset: usize,
    inode: Arc<Inode>,
}

impl OSInode {
    /// create a new inode in memory
    pub fn new(readable: bool, writable: bool, inode: Arc<Inode>) -> Self {
        Self {
            readable,
            writable,
            inner: unsafe { UPSafeCell::new(OSInodeInner { offset: 0, inode}) },
        }
    }
    /// read all data from the inode
    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.exclusive_access();
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop {
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buffer[..len]);
        }
        v
    }

    ///  read node
    pub fn read_node(&self) -> Arc<Inode>{
        self.inner.exclusive_access().inode.clone()

    }


    /// add link num
    pub fn add_link_num(&self){
        let inode =&self.inner.exclusive_access().inode;        
        inode.add_nlink();
    }

    /// sub link num
    pub fn sub_link_num(&self){
        let inode =&self.inner.exclusive_access().inode;        
        inode.sub_nlink();
    }

    /// add link 
    pub fn get_inode_id(&self) -> u32{
        let inode =&self.inner.exclusive_access().inode;
        inode.get_inode_id()
    }

    /// get link num
    pub fn get_link_num(&self) ->u32{
        self.inner.exclusive_access().inode.get_nlink()
    }
}

lazy_static! {
    pub static ref ROOT_INODE: Arc<Inode> = {
        let efs = EasyFileSystem::open(BLOCK_DEVICE.clone());
        Arc::new(EasyFileSystem::root_inode(&efs))
    };
}

/// List all apps in the root directory
pub fn list_apps() {
    println!("/**** APPS ****");
    for app in ROOT_INODE.ls() {
        println!("{}", app);
    }
    println!("**************/");
}

bitflags! {
    ///  The flags argument to the open() system call is constructed by ORing together zero or more of the following values:
    pub struct OpenFlags: u32 {
        /// readyonly
        const RDONLY = 0;
        /// writeonly
        const WRONLY = 1 << 0;
        /// read and write
        const RDWR = 1 << 1;
        /// create new file
        const CREATE = 1 << 9;
        /// truncate file size to 0
        const TRUNC = 1 << 10;
    }
}

impl OpenFlags {
    /// Do not check validity for simplicity
    /// Return (readable, writable)
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::WRONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}

/// Open a file
pub fn open_file(name: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    ROOT_INODE.ls();
    let (readable, writable) = flags.read_write();
    if flags.contains(OpenFlags::CREATE) {
        if let Some(inode) = ROOT_INODE.find(name) {
            error!("open exist inode id is {}",inode.get_inode_id());
            // clear size
            inode.clear();                        
            Some(Arc::new(OSInode::new(readable, writable, inode)))
        } else {
            // create file
            ROOT_INODE
                .create(name)
                .map(|inode| {
                    error!("open not exist inode id is {}",inode.get_inode_id());
                    Arc::new(OSInode::new(readable, writable, inode))
                })
        }
    } else {
        ROOT_INODE.find(name).map(|inode| {
            if flags.contains(OpenFlags::TRUNC) {
                inode.clear();
            }            
            error!("name is {}",name);
            error!("open read only inode id is {}",inode.get_inode_id());
            Arc::new(OSInode::new(readable, writable, inode))
        })
    }
}

/// find a file 
/// 
pub fn find_file(name: &str) ->Option<Arc<OSInode>>{
    if let Some(inode) = ROOT_INODE.find(name){        
        Some(Arc::new(OSInode::new(true, false, inode)))
    }else{
        None
    }
}

/// link inode
pub fn link_inode(name: &str,node_id :u32) -> bool{
    ROOT_INODE.link_node(name, node_id )
}

/// unlink_inode
pub fn unlink_inode(name:&str) -> bool{
    error!("enter indeos unlink");
    ROOT_INODE.unlink_node(name)
}

impl File for OSInode {
    fn readable(&self) -> bool {
        self.readable
    }
    fn writable(&self) -> bool {
        self.writable
    }
    fn read(&self, mut buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_read_size = 0usize;
        for slice in buf.buffers.iter_mut() {
            let read_size = inner.inode.read_at(inner.offset, *slice);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            total_read_size += read_size;
        }
        total_read_size
    }
    fn write(&self, buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_write_size = 0usize;
        for slice in buf.buffers.iter() {
            let write_size = inner.inode.write_at(inner.offset, *slice);
            assert_eq!(write_size, slice.len());
            inner.offset += write_size;
            total_write_size += write_size;
        }
        total_write_size
    }

    fn get_stat(&self) -> Stat{                

        let inner = self.inner.exclusive_access();
        let _inode = &inner.inode;
        let ino = _inode.get_inode_id() as u64;
        let tisk_node = _inode.is_dir_node();
        let nlink = _inode.get_nlink();
        let mut mode = StatMode::FILE;
        if tisk_node == true{
            mode = StatMode::DIR;
        }
        let stat = Stat {
            dev: 0u64,
            ino: ino,    
            mode: mode,
            nlink: nlink,
            pad: [0; 7],
        };
        stat
    }
}
