

use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task
//    , TaskStatus
};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use crate::config::MAX_USIZE;

/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Arc<dyn Mutex> = if !blocking {
        Arc::new(MutexSpin::new())
    } else {
        Arc::new(MutexBlocking::new())
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = Some((MAX_USIZE,mutex));
        id as isize
    } else {
        process_inner.mutex_list.push(Some((MAX_USIZE,mutex)));
        process_inner.mutex_list.len() as isize - 1
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();    
    let node = process_inner.mutex_list[mutex_id].as_ref().unwrap();
    let taskid = node.0;
    let current_task = current_task();
    let current_tid = current_task.as_ref().unwrap().get_task_id();
    // let mut status = TaskStatus::Ready;
    // for task in &process_inner.tasks{
    //     match task {
    //         Some(tcb)=>{
    //             if tcb.get_task_id() == taskid{
    //                 status = tcb.inner_exclusive_access().task_status;
    //             }
    //         },
    //         None=>{}
    //     }
    // }
    if taskid == MAX_USIZE{
        drop(process_inner);
        drop(process);
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();    
        let node = process_inner.mutex_list[mutex_id].as_mut().unwrap();
        let mutex = Arc::clone(&node.1);
        let taskid = current_task.unwrap().get_task_id();
        node.0 = taskid;
        drop(process_inner);
        drop(process);
        mutex.lock();
    }else if node.1.get_locktype() == 1{
        let mutex = Arc::clone(&node.1);
        drop(process_inner);
        drop(process);
        mutex.lock();
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();    
        let node = process_inner.mutex_list[mutex_id].as_mut().unwrap();
        let taskid = current_task.unwrap().get_task_id();
        node.0 = taskid;
    }else if node.1.get_locktype() == 2 && taskid !=current_tid{
        let mutex = Arc::clone(&node.1);
        drop(current_task);
        drop(process_inner);
        drop(process);
        mutex.lock(); 
    }else if node.1.get_locktype() == 1{

    }else{
        drop(current_task);
        drop(process_inner);
        drop(process);
        return -0xdead;  
    }
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();    
    let node = process_inner.mutex_list[mutex_id].as_mut();

    match node{
        Some((tid,_mutex))=>{
            if *tid == MAX_USIZE{
                return 0;
            }
            let mutex = Arc::clone(&_mutex);                
                match mutex.get_next_task() {
                    Some(taskid) =>{
                        error!("error");
                        *tid= taskid;
                    },
                    None=>{
                        *tid = MAX_USIZE;
                    }
                }
            drop(process_inner);
            drop(process);            
            mutex.unlock();  
        },
        None =>{}
    }    
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));  
        for task in &process_inner.tasks{
            match task {
                Some(task) =>{
                    if let Some(elem) = task.inner_exclusive_access().semphore_list.get_mut(id) {
                        *elem = 0;
                    }
                    if let Some(elem) = task.inner_exclusive_access().semphore_need.get_mut(id) {
                        *elem = 0;
                    }
                },
                None =>{},
            }
        }
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        for task in &process_inner.tasks{
            match task {
                Some(task) =>{                    
                    task.inner_exclusive_access().semphore_list.push(0);
                    task.inner_exclusive_access().semphore_need.push(0);
                },
                None =>{},
            }
        }
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let task = current_task().unwrap();
    
    if *task.inner_exclusive_access().semphore_list.get_mut(sem_id).unwrap()  > 0{
        *task.inner_exclusive_access().semphore_list.get_mut(sem_id).unwrap() -= 1;
    }    
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up(sem_id);
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );    
    let task = current_task().unwrap();
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);    
    if let Some(elem) = task.inner_exclusive_access().semphore_need.get_mut(sem_id) {
        *elem += 1;
    }
    if process.detect_deadlock(){
        if let Some(elem) = task.inner_exclusive_access().semphore_need.get_mut(sem_id) {
            *elem -= 1;
        }
        return  -0xDEAD;
    }
    
    sem.down();    
    if let Some(elem) = task.inner_exclusive_access().semphore_need.get_mut(sem_id) {
        *elem -= 1;
    }
    if let Some(elem) = task.inner_exclusive_access().semphore_list.get_mut(sem_id) {
        *elem += 1;
    }    
    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(&process_inner.mutex_list[mutex_id].as_ref().unwrap().1);
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    let process = current_process();
    match _enabled{
        0usize =>{
            process.inner_exclusive_access().deadlock_detected = false;
            0
        },
        1usize =>{
            process.inner_exclusive_access().deadlock_detected = true;            
            0
        },
        _ => -1,
    }        
}
