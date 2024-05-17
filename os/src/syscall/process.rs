//! Process management syscalls

use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next,current_user_token, TaskStatus,
    }, timer::get_time_us,
    mm::{PageTable,VirtAddr},
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    error!("kernel: sys_get_time");
    let _us = get_time_us();
    let _sec:u64 = (_us / 1_000_000) as u64;
    let _usec:u64 = (_us / 1_000_000) as u64;
    let binding = [_sec,_usec];
     
    let _current_token = current_user_token();
    let _page_table = PageTable::from_token(_current_token);

    let mut _address = _ts as usize;

    let mut _va = VirtAddr::from(_address);
    let mut vpn = _va.floor(); 
    let mut ppn = _page_table.translate(vpn).unwrap().ppn();
    let mut page =  ppn.get_bytes_array();
    let mut _offset = _va.page_offset();

    let mut _index =0;
    unsafe{
        let time = binding.align_to::<u8>().1;        
        for i in time {
            error!("data :{}",*i);        
            _address += 8*_index;
            _va = VirtAddr::from(_address);
            if vpn != _va.floor(){
                vpn = _va.floor();
                ppn = _page_table.translate(vpn).unwrap().ppn();
                page =  ppn.get_bytes_array();            
            }
            _offset = _va.page_offset();
            page[_offset] = *i;
            _index += 1;
        }
    }   
    
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
