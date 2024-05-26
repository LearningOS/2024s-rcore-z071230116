//! File and filesystem-related syscalls

use crate::fs::{open_file, find_file,link_inode,unlink_inode,OpenFlags, Stat};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer,VirtAddr,PageTable};
use crate::task::{current_task, current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
pub fn sys_fstat(_fd: usize, _st: *mut Stat) -> isize {
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if _fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[_fd] {
        let file = file.clone();
        drop(inner);
        let _stat = file.get_stat();
        let _current_token = current_user_token();
        let _page_table = PageTable::from_token(_current_token);
        let mut _address = _st as usize;
        let mut _va = VirtAddr::from(_address);
        let mut vpn = _va.floor(); 
        let mut ppn = _page_table.translate(vpn).unwrap().ppn();
        let mut page =  ppn.get_bytes_array();
        let mut _offset = _va.page_offset(); 
        let bytes:[u8;80] = unsafe {     
            core::mem::transmute(_stat)
        }; 
        for i in bytes {                        
            _va = VirtAddr::from(_address);
            if vpn != _va.floor(){
                vpn = _va.floor();
                ppn = _page_table.translate(vpn).unwrap().ppn();
                page =  ppn.get_bytes_array();            
            }
            _offset = _va.page_offset();
            page[_offset] = i;
            _address += 1 as usize;
        }
        return 0;
    }
    -1
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(_old_name: *const u8, _new_name: *const u8) -> isize {
    let token = current_user_token();
    let old_path = translated_str(token, _old_name);
    let new_path = translated_str(token, _new_name);
    if let Some(old_inode) = find_file(old_path.as_str()){
        let node_id = old_inode.get_inode_id();       
        if link_inode(new_path.as_str(),node_id){
            error!("old node id is {}",node_id);
            old_inode.add_link_num();
            return 0;
        }
        
    }
    -1
//find the old_name disknode
//create the node and node is disknode
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(_name: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, _name);
    if let Some(inode) = find_file(path.as_str()){   
        error!("**********************");     
        if unlink_inode(path.as_str()){
            inode.sub_link_num();
            if inode.get_link_num() == 0{

            }
            return 0;
        }
        
    }else{
        error!("no this path {}",path.as_str());
    }
    -1
}
