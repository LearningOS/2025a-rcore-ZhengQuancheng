//! Process management syscalls
//!
use alloc::sync::Arc;

use crate::{
    config::PAGE_SIZE, 
    fs::{open_file, OpenFlags}, 
    mm::{translated_byte_buffer, translated_refmut, translated_str, MapPermission, VirtAddr}, 
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next,
    }, 
    timer::get_time_us
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    //trace!("kernel: sys_yield");
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
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    //trace!("kernel: sys_waitpid");
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
    trace!("kernel:pid[{}] sys_get_time", current_task().unwrap().pid.0);
    // 获取当前时间
    let us = get_time_us();
    let tv = TimeVal { // ch3
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    // 将应用地址空间中的一段缓冲区 _ts 转化为在内核地址空间直接读写的字节切片向量 bufs
    let bufs = translated_byte_buffer(
        current_user_token(),
        _ts as *const u8,
        core::mem::size_of::<TimeVal>()
    );
    // 将 tv 转为字节数组, 方便数据复制
    let src = unsafe {
        core::slice::from_raw_parts(
            &tv as *const TimeVal as *const u8,
            core::mem::size_of::<TimeVal>(),
        )
    };
    // 将 src 分批次复制到 bufs 中
    let mut offset = 0;
    for buf in bufs {
        let len = core::cmp::min(buf.len(), src.len() - offset);
        buf[..len].copy_from_slice(&src[offset..offset + len]);
        offset += len;
    }
    0
}

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel:pid[{}] sys_mmap", current_task().unwrap().pid.0);
    // 检查 start 是否页对齐
    if _start % PAGE_SIZE != 0 {
        return -1;
    }
    // 检查 len 是否符合要求
    if _len == 0 {
        return 0;
    }
    // 检查权限是否合法
    let mut perm = MapPermission::U;
    if _port & !0b0111 != 0 { // 权限只能是 0b000 ~ 0b111
        return -1;
    }
    if _port & 0b111 == 0 { // 权限不能全为 0, ch4_mmap3.rs:21
        return -1;
    }
    if _port & 0b001 != 0 { // 可读取
        perm |= MapPermission::R;
    }
    if _port & 0b010 != 0 { // 可写入
        perm |= MapPermission::W;
    }
    if _port & 0b100 != 0 { // 可执行
        perm |= MapPermission::X;
    }
    // 映射内存
    let sva = VirtAddr::from(_start);
    let eva = VirtAddr::from(_start + _len);
    let task = current_task().unwrap();
    let result = task.inner_exclusive_access().memory_set.map_pages(sva, eva, perm);
    match result {
        true => 0,
        false => -1,
    }
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel:pid[{}] sys_munmap", current_task().unwrap().pid.0);
    // 检查 start 是否页对齐
    if _start % PAGE_SIZE != 0 {
        return -1;
    }
    // 取消映射内存
    let sva = VirtAddr::from(_start);
    let eva = VirtAddr::from(_start + _len);
    let task = current_task().unwrap();
    let result = task.inner_exclusive_access().memory_set.unmap_pages(sva, eva);
    match result {
        true => 0,
        false => -1,
    }
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
    trace!("kernel:pid[{}] sys_spawn", current_task().unwrap().pid.0);
    let path = translated_str(current_user_token(), _path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let current_task = current_task().unwrap();
        let app_data = app_inode.read_all();
        let spawned_task = current_task.spawn(app_data.as_slice());
        let spawned_pid = spawned_task.pid.0;
        add_task(spawned_task);
        spawned_pid as isize
    } else {
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!("kernel:pid[{}] sys_set_priority", current_task().unwrap().pid.0);
    if _prio < 2 {
        return -1;
    }
    let task = current_task().unwrap();
    task.set_priority(_prio as usize);
    _prio
}
