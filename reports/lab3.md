# Lab2 

## 编程作业

- `sys_get_time`, `sys_mmap` 和 `sys_munmap` 的实现较 lab2 的实现区别不大.
- `sys_spawn` 参考 `sys_fork` 和 `sys_exec` 的实现. 
    - 在 `src/syscall/process.rs` 的 `sys_spawn`, 调用 `get_app_data_by_name` 根据应用名称获取数据, 若应用不存在, 则返回 `-1`; 若存在应用, 则调用 `current_task.spawn` 创建新进程 `spawned_task`. `current_task` 是新进程的父进程. 退出之前, 将 `spawned_task` 加入 TaskManager 的就绪队列 (`add_task`).
    - 在 `src/task/task.rs` 的 `TaskControlBlock` 中, 添加新的方法 `spawn`, 其实现参考同级的 `new`, `fork` 和 `exec`. `spawn` 与 `new` 方法相比, 需要为新创建的进程指定父进程: `self.inner_exclusive_access().children.push(task_control_block.clone());`. 可认为主动调用 `spawn` 的进程为父进程, 被创建的进程为子进程.
- `stride` 调度算法
    - `sys_set_priority` 的实现较为简单, 暂不赘述.
    - 在 `src/task/manager.rs` 中实现 `stride_scheduler`, 可参考题目要求进行实现. 但是, 在实现过程中, 遇到一个非常严重的问题: 进程中存在两个进程 0 和 8, 进程调度器始终不会调度进程 8, 导致 8 号进程不会退出. 输出内容如下, 可以看到进程 0 的 stride 始终小于进程 8 的. 当进程 8 的 stride 非常大 (18446744073709551600 = usize::MAX - 15), 进程 0 一直会追赶进程 8, 两者非常接近时, 进程 0 的 stride 加上 pass 虽然会超过进程 0, 但是会导致整数溢出回绕, 进程 0 的 stride 会再次从较小值追赶进程 8. 由于进程 0 的 stride 加上 pass 超过进程 0 而不超过usize::MAX 的概率太小, 故宏观来看进程调度器不会选择进程 8.
        ```
        ===========> pid:8 stride:18446744073709551600 priority:16
        ===========> pid:0 stride:8070450532247912697 priority:16
        ===========> pid:8 stride:18446744073709551600 priority:16
        ===========> pid:0 stride:9223372036854759672 priority:16
        ===========> pid:8 stride:18446744073709551600 priority:16
        ===========> pid:0 stride:10376293541461606647 priority:16
        ===========> pid:8 stride:18446744073709551600 priority:16
        ===========> pid:0 stride:11529215046068453622 priority:16
        ===========> pid:8 stride:18446744073709551600 priority:16
        ===========> pid:0 stride:12682136550675300597 priority:16
        ===========> pid:8 stride:18446744073709551600 priority:16
        ===========> pid:0 stride:13835058055282147572 priority:16
        ===========> pid:8 stride:18446744073709551600 priority:16
        ===========> pid:0 stride:14987979559888994547 priority:16
        ===========> pid:8 stride:18446744073709551600 priority:16
        ===========> pid:0 stride:16140901064495841522 priority:16
        ===========> pid:8 stride:18446744073709551600 priority:16
        ===========> pid:0 stride:17293822569102688497 priority:16
        ===========> pid:8 stride:18446744073709551600 priority:16
        ===========> pid:0 stride:18446744073709535472 priority:16
        ===========> pid:8 stride:18446744073709551600 priority:16
        ===========> pid:0 stride:1152921504606830831 priority:16
        ===========> pid:8 stride:18446744073709551600 priority:16
        ===========> pid:0 stride:2305843009213677806 priority:16
        ```
    + 正确实现参考下面**问答作业**的分析.

## 问答作业

1. 两个 pass = 10 的进程, 使用 8bit 无符号整形储存 stride,  p1.stride = 255, p2.stride = 250, 在 p2 执行一个时间片后, 理论上下一次应该 p1 执行. 实际情况是轮到 p1 执行吗？为什么？
    + 实际情况是很可能不会轮到 p1 执行. 
    + 初始状态为 p1.stride = 255, p2.stride = 250. 因为 250 < 255, 调度器选择 p2 执行. 
    + p2 执行后, 其 stride 更新. 步长 pass 为 10, 则 p2.new_stride = p2.stride + pass = 250 + 10 = 260, 因为使用的是 8-bit 无符号整数(范围 0-255), 260 会发生溢出: 260 % 256 = 4. 所以 p2.new_stride 实际上变成了 4.
    + 下一次调度时, 调度器比较 p1.stride = 255 和 p2.stride = 4. 因为 4 < 255, 调度器会再次选择 p2 执行.
2. 在不考虑溢出的情况下, 在进程优先级全部 >= 2 的情况下, 如果严格按照算法执行, 那么 STRIDE_MAX – STRIDE_MIN <= BigStride / 2. 为什么?
    + 简单证明: `STRIDE_MAX – STRIDE_MIN <= BigStride / 2`
    + `pass = BigStride / priority`, 因 `priority >= 2` 则 `pass <= BigStride / 2`. 即任何进程在执行一次后, 其 `stride` 的增量不会超过 BigStride 的一半.
    + 假设某个时刻第一次出现 `STRIDE_MAX – STRIDE_MIN > BigStride / 2` 的情况. 在此前一次调度, 有一个进程 P1 被选中调度, 其 P1.stride 为当时最小, P2.stride 的值次之(亦可相等). 当 P1.stride 加上 P1.pass 之后, 导致 `P1.stride + P1.pass - P2.stride > BigStride / 2`, 对其变形得 `P1.pass > (BigStride / 2) + (P2.stride - P1.stride)`, 而 `0 <= P2.stride - P1.stride <= BigStride / 2`, 故 `P1.pass > BigStride / 2`. 但是由于 `pass <= BigStride / 2`, 故假设不成立. 所以任意进程 `priority >= 2`, 其 `STRIDE_MAX – STRIDE_MIN <= BigStride / 2` 得证.
3. 任意进程的 `stride` 的真实差距不超过 `u64::MAX / 2`. 若不满足此, 则发生了环绕.
    + 代码实现如下: 
        ```
        use core::cmp::Ordering;

        const BIG_STRIDE: u64 = u64::MAX; 

        struct Stride(u64);

        impl PartialOrd for Stride {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                let diff = self.0.wrapping_sub(other.0);
                if diff == 0 {
                    Some(Ordering::Equal) 
                } else if diff < BIG_STRIDE / 2 {
                    Some(Ordering::Less)
                } else {
                    Some(Ordering::Greater)
                }
            }
        }

        ```
