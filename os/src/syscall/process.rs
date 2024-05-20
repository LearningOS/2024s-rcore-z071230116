//! Process management syscalls



use crate::{
    config::{MAX_SYSCALL_NUM,PAGE_SIZE},
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next,current_user_token, TaskStatus,
        get_current_status,get_systimecall_times,get_run_times,get_current_aera
    }, timer::get_time_us,
    mm::{PageTable,VirtAddr,MapPermission,get_pypage_num
    },
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
    
    let _us = get_time_us();    
    let _sec:u64 = (_us / 1_000_000) as u64;
    let _usec:u64 = (_us % 1_000_000) as u64;      
    let time = [_sec,_usec];
     
    let _current_token = current_user_token();
    let _page_table = PageTable::from_token(_current_token);

    let mut _address = _ts as usize;

    let mut _va = VirtAddr::from(_address);
    let mut vpn = _va.floor(); 
    let mut ppn = _page_table.translate(vpn).unwrap().ppn();
    let mut page =  ppn.get_bytes_array();
    let mut _offset = _va.page_offset();

    for items in time{
        let item  = items.to_ne_bytes();
        for i in item {                        
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
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    let status = get_current_status();
    let syscall_times = get_systimecall_times();
    
    let _current_token = current_user_token();
    let _page_table = PageTable::from_token(_current_token);

    let mut _address = _ti as usize;
    let mut _va = VirtAddr::from(_address);
    let mut vpn = _va.floor(); 
    let mut ppn = _page_table.translate(vpn).unwrap().ppn();
    let mut page =  ppn.get_bytes_array();
    let mut _offset = _va.page_offset(); 
    let _info_address = (&mut page[_offset] as *const _) as *mut TaskInfo;
    unsafe {
        (*_info_address).status =   status;    
    }
    for items in syscall_times{
        let item  = items.to_ne_bytes();
        for i in item {                        
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
    }
    let time: usize = ((get_run_times()/1_000_000 & 0xffff) * 1000 + get_run_times()%1_000_000/ 1000) as usize;

    for i in time.to_ne_bytes() {                        
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


    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    let vnums = (_len -1 + PAGE_SIZE) / PAGE_SIZE;

    if _start % PAGE_SIZE !=0 || _port & !0x7 != 0 || _port & 0x7 == 0 || vnums > get_pypage_num(){
        return -1;
    }
    
    let mut permission = MapPermission::from_bits((_port as u8) << 1).unwrap();
    permission.set(MapPermission::U, true);

    let start_va = _start.into();
    let end_va = (_start+_len).into();
    let memory = get_current_aera();
    if memory.has_maped(_start,vnums){
        error!("this is has_maped!");
        return -1;
    }

    memory.insert_framed_area( start_va, end_va, permission);            
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {

    if _start % PAGE_SIZE !=0 || _len % PAGE_SIZE != 0{
        return -1;
    }
    let vnums = (_len -1 + PAGE_SIZE) / PAGE_SIZE;

    let memory = get_current_aera();
    if memory.is_all_map(_start,vnums) == false{
        return -1;
    }
    memory.unmap_len(_start,_len);
    0
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
