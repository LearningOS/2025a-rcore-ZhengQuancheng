//! Process management syscalls
use crate::{
    config::PAGE_SIZE, 
    mm::{translated_byte_buffer, MapPermission, VirtAddr}, 
    task::{change_program_brk, check_addr_readable, check_addr_writable, current_user_token, exit_current_and_run_next, get_syscall_count, map_pages, unmap_pages, suspend_current_and_run_next}, 
    timer::get_time_us
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
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
    trace!("kernel: sys_get_time");
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

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    match _trace_request {
        0 => {
            if !check_addr_readable(_id) {
                return -1;
            }
            let bufs = translated_byte_buffer(current_user_token(), _id as *const u8, 1);
            bufs[0][0] as isize
        }
        1 => {
            if !check_addr_writable(_id) {
                return -1;
            }
            let mut bufs = translated_byte_buffer(current_user_token(), _id as *mut u8, 1);
            bufs[0][0] = _data as u8;
            0
        }
        2 => {
            get_syscall_count(_id) as isize
        }
        _ => {
            trace!("sys_trace: invalid trace_request {}", _trace_request);
            -1
        }
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap");
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
    match map_pages(sva, eva, perm) {
        true => 0,
        false => -1,
    } 
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap");
    // 检查 start 是否页对齐
    if _start % PAGE_SIZE != 0 {
        return -1;
    }
    // 取消映射内存
    let sva = VirtAddr::from(_start);
    let eva = VirtAddr::from(_start + _len);
    match unmap_pages(sva, eva) {
        true => 0,
        false => -1,
    }
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
