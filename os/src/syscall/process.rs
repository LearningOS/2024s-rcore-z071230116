//! Process management syscalls
use core::borrow::BorrowMut;

use alloc::sync::Arc;

use crate::{
    config::{MAX_SYSCALL_NUM,PAGE_SIZE},
    loader::get_app_data_by_name,
    mm::{PageTable,VirtAddr,MapPermission,get_pypage_num,translated_refmut, translated_str},
    task::{
        add_task, is_full,current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,TaskControlBlock
    },
    timer::get_time_us,
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
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
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
    let status = current_task().unwrap().get_status().unwrap();
    let syscall_times = current_task().unwrap().get_systimecall_times().unwrap();
    
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
    let run_time =current_task().unwrap().borrow_mut().get_run_times().unwrap(); 
    let time: usize = ((run_time/1_000_000 & 0xffff) * 1000 + run_time%1_000_000/ 1000) as usize;

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

/// YOUR JOB: Implement mmap.
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
    let current = current_task().unwrap();
    let memory = current.get_memory_set();
    if memory.has_maped(_start,vnums){
        error!("this is has_maped!");
        return -1;
    }

    memory.insert_framed_area( start_va, end_va, permission);            
    0
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    if _start % PAGE_SIZE !=0 || _len % PAGE_SIZE != 0{
        return -1;
    }
    let vnums = (_len -1 + PAGE_SIZE) / PAGE_SIZE;

    let current = current_task().unwrap();
    let memory = current.get_memory_set();
    if memory.is_all_map(_start,vnums) == false{
        return -1;
    }
    memory.unmap_len(_start,_len);
    0
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if is_full() {
        error!("this is full");
        return  -1 ;
    }
    let token = current_user_token();
    let current_task = current_task().unwrap();    
    let path = translated_str(token, _path);
    
    if let Some(data) = get_app_data_by_name(path.as_str()) {      
        let tcb = Arc::new(TaskControlBlock::new(data));
        tcb.inner_exclusive_access().parent = Some(Arc::downgrade(&current_task));
        current_task.inner_exclusive_access().children.push(tcb.clone());
        //let _taskcontext = tcb.inner_exclusive_access().task_cx;
        add_task(tcb.clone()); 
        
        tcb.pid.0 as isize
    } else {
        error!("no exist the file {}",path.as_str());
        -1
    }

}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(

        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if _prio < 2{
        return -1;
    }
    current_task().unwrap().inner_exclusive_access().priority = _prio as usize;
    _prio   
}
