//!Implementation of [`TaskManager`]

use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
use crate::config::MAX_PID;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        
        let mut tap = 0;
        if let Some(first)  = self.ready_queue.get(tap){
            let mut min_stride = first.inner_exclusive_access().stride;
            
            let read_queue = self.ready_queue.iter();
            let mut index = 0;
            for item in read_queue{
                let index_stride = item.inner_exclusive_access().stride;
                if  index_stride < min_stride{
                    min_stride = index_stride;
                    tap = index;
                }
                index += 1;
            }
            return Some(self.ready_queue.remove(tap).unwrap());
        }
        None
    }

    /// get pid number
    pub fn pid_num(&self) -> usize{
        self.ready_queue.len()
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}

/// is max 
pub fn is_full() -> bool{
    let num = TASK_MANAGER.exclusive_access().pid_num();
    error!("the num is {}",num);
    if MAX_PID <= num{
        return true;
    }
    false
}
