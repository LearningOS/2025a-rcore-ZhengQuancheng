use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
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
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        // Update available vector for deadlock detector
        if process_inner.enable_deadlock_detection {
            process_inner.mutex_deadlock_detector.update_available(id, 1);
        }
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        let mutex_id = process_inner.mutex_list.len() - 1;
        // Update available vector for deadlock detector
        if process_inner.enable_deadlock_detection {
            process_inner.mutex_deadlock_detector.update_available(mutex_id, 1);
        }
        mutex_id as isize
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
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    // Check Deadlock !!!
    if process_inner.enable_deadlock_detection {
        let tid = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
        // Update need matrix for deadlock detector
        process_inner.mutex_deadlock_detector.update_need(tid, mutex_id, 1);
        // Check safety
        if !process_inner.mutex_deadlock_detector.check_safety() {
            log::debug!("Deadlock Detector: Deadlock Detected for tid: {}, mutex_id: {}", tid, mutex_id);
            // Deadlock detected, roll back the state
            process_inner.mutex_deadlock_detector.update_need(tid, mutex_id, -1);
            return -0xdead;
        }
        log::debug!("Deadlock Detector: No Deadlock Detected for tid: {}, mutex_id: {}", tid, mutex_id);
    }
    drop(process_inner);
    drop(process);
    mutex.lock();
    // Update available vector, need and allocation matrix for deadlock detector
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if process_inner.enable_deadlock_detection {
        let tid = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
        process_inner.mutex_deadlock_detector.update_available(mutex_id, -1);
        process_inner.mutex_deadlock_detector.update_need(tid, mutex_id, -1);
        process_inner.mutex_deadlock_detector.update_allocation(tid, mutex_id, 1);
    }
    drop(process_inner);
    drop(process);
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
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    // Update available vector and allocation matrix for deadlock detector
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if process_inner.enable_deadlock_detection {
        let tid = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
        process_inner.mutex_deadlock_detector.update_available(mutex_id, 1);
        process_inner.mutex_deadlock_detector.update_allocation(tid, mutex_id, -1);
    }
    drop(process_inner);
    drop(process);
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
        // Update available vector for deadlock detector
        if process_inner.enable_deadlock_detection {
            process_inner.semaphore_deadlock_detector.update_available(id, res_count as isize);
        }
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        let sem_id = process_inner.semaphore_list.len() - 1;
        // Update available vector for deadlock detector
        if process_inner.enable_deadlock_detection {
            process_inner.semaphore_deadlock_detector.update_available(sem_id, res_count as isize);
        }
        sem_id
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
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up();
    // Update available vector and allocation matrix for deadlock detector
    let mut process_inner = process.inner_exclusive_access();
    if process_inner.enable_deadlock_detection {
        let tid = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
        process_inner.semaphore_deadlock_detector.update_available(sem_id, 1);
        process_inner.semaphore_deadlock_detector.update_allocation(tid, sem_id, -1);
    }
    drop(process_inner);
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
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    // Check Deadlock !!!
    if process_inner.enable_deadlock_detection {
        let tid = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
        // Update need matrix for deadlock detector
        process_inner.semaphore_deadlock_detector.update_need(tid, sem_id, 1);
        // Check safety
        if !process_inner.semaphore_deadlock_detector.check_safety() {
            log::debug!("Deadlock Detector: Deadlock Detected for tid: {}, sem_id: {}", tid, sem_id);
            // Deadlock detected, roll back the state
            process_inner.semaphore_deadlock_detector.update_need(tid, sem_id, -1);
            return -0xdead;
        }
        log::debug!("Deadlock Detector: No Deadlock Detected for tid: {}, sem_id: {}", tid, sem_id);
    }
    drop(process_inner);
    sem.down();
    // Update available vector, need and allocation matrix for deadlock detector
    let mut process_inner = process.inner_exclusive_access();
    if process_inner.enable_deadlock_detection {
        let tid = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
        process_inner.semaphore_deadlock_detector.update_available(sem_id, -1);
        process_inner.semaphore_deadlock_detector.update_need(tid, sem_id, -1);
        process_inner.semaphore_deadlock_detector.update_allocation(tid, sem_id, 1);
    }
    drop(process_inner);
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
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect");
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.enable_deadlock_detection = _enabled != 0;
    0
}
