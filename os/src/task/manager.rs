//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::config::BIG_STRIDE;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
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
        self.stride_scheduler()
    }
    /// fifo scheduler
    #[allow(unused)]
    fn fifo_scheduler(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
    /// stride scheduler
    #[allow(unused)]
    fn stride_scheduler(&mut self) -> Option<Arc<TaskControlBlock>> {
        // 若果就绪队列为空, 返回 None
        if self.ready_queue.is_empty() {
            return None;
        }
        // 找到 stride 最小的进程
        let mut min_index = 0;
        let mut min_stride = self.ready_queue[0].inner_exclusive_access().stride;
        for (i, task) in self.ready_queue.iter().enumerate() {
            let task_inner = task.inner_exclusive_access();
            // !!!!!!!!!!!!!!
            if task_inner.stride.wrapping_sub(min_stride) <= BIG_STRIDE / 2 {
                min_index = i;
                min_stride = task_inner.stride;
            }
        }
        // 从就绪队列中取出该进程, 并更新其 stride
        let task = self.ready_queue.remove(min_index).unwrap();
        {
            let mut task_inner = task.inner_exclusive_access();
            task_inner.stride = task_inner.stride.wrapping_add(BIG_STRIDE / task_inner.priority);
        }
        Some(task)
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
