# Lab1

## 编程作业

### 实现思路

按照要求实现 `os/src/syscall/process.rs` 的 `sys_tarce`, 其前两点要求较为简单, 难点在于查询当前任务调用编号为 id 的系统调用的次数. 为此可在所有系统调用都必须经过的 `syscall` 函数 (位于 `os/src/syscall/mod.rs`) 处统计. 为简单起见, 使用数组 `sys_counts` 作为系统调用次数统计的容器, 其应位于 `TaskManagerInner`.

### 遇到问题

+ 问题: 
    + 实现 `sys_trace` 系统调用时, 在 `TaskControlBlock` 中添加 `sys_counts` (类型为 `[usize; 512]`) 字段, 执行 `make run` 可正常运行, 但执行 `cd ci-user && make test CHAPTER=3` 却出现卡死.

+ 解决办法: 
    + 将 `sys_counts` (类型为 `[usize; 512]`) 字段移动至 `TaskManagerInner` 即可.

+ 问题分析:
    + `TASK_MANAGER` (`TaskManager` 的实例)在栈上初始化, 有可能发生栈溢出
    + 在`解决办法`的基础上, 再为 `TaskControlBlock` 添加 `arr` (类型为 `[usize; 512]`) 字段, 其会导致 QEMU 卡死.
    + 减少应用程序的数量, 当保留 4 个应用程序时(原本 7 个), 不会导致 QEMU 卡死. 恢复应用程序的数量.
    + 将 `arr` 的数组长度改为 256, 程序可正常运行; 将数组 `arr` 的长度恢复 512.
    + 修改 `entry.asm` 文件, 将 `boot_stack` 的大小由 `4096 * 16 Byte` 改为 `4096 * 32 Byte`, 程序可正常运行.
    + 由以上尝试, 可基本确定为 `TASK_MANAGER` 初始化时 `boot_stack` 发生栈溢出.
    + 为什么发生栈溢出? `arr` 的大小为 `512 * 8 * 16 (MAX_APP_NUM) = 4096 * 16 Byte`, `TaskControlBlock` 再加上其他字段, 其大小已超出 `boot_stack` 的大小, 这会导致 `TASK_MANAGER` 在 `boot_stack` 栈上初始化, 确定导致栈溢出!
    + 为什么`解决办法`不会导致栈溢出? 将 `sys_counts` (类型为 `[usize; 512]`) 字段移动至 `TaskManagerInner`, `TaskManager` 的初始化没有经过栈, 其直接在 lazy_static 的静态内存区域被分配.

## 简答作业

1. bad testcase  
    + SBI: RustSBI-QEMU Version 0.2.0-alpha.2
    + 报错信息
        - `ch2b_bad_address.rs`: [kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
        - `ch2b_bad_instructions.rs`: [kernel] IllegalInstruction in application, kernel killed it.
        - `ch2b_bad_register.rs`: [kernel] IllegalInstruction in application, kernel killed it.
2. `trap.S`  
    + (1) 刚进入 __restore 时，sp 指向当前 Task 的 TrapContext, 其位于当前 Task 的内核栈. __restore 有两种使用场景: 其一为运行第一个 Task 的跳板; 其二为恢复 TrapContext, 使被 Trap 打断的 Task 继续运行.  
    + (2) 从 TrapContext 恢复 sstatus/sepc/sscratch 寄存器的值. sstatus 描述运行 Task 的很多状态信息; sepc 保存了发生 Trap 时的位置, 通过 sret 指令可返回到正常的执行流程; sscratch 指向用户栈, sp 指向内核栈, 通过 csrrw 交换 sscratch 和 sp, sscratch 更像临时的容器, 方便实现栈的切换.
    + (3) x2 为栈指针寄存器 sp, x4 为线程指针寄存器 tp. 执行 __restore 时, sp 指向当前内核栈(TrapContex), 以方便恢复其他寄存器, 在 sret 之前, 可从 sscratch 恢复原内核栈. 应用程序一般不使用 tp 寄存器.
    + (4) sp -> 用户栈, sscratch -> 内核栈
    + (5) sret 指令会发生状态切换. sret 指令执行时, Privilege <- sstatus.SPP, 而在用户程序执行时发生 Trap, sstatus.SPP <- UserMode.
    + (6) sp -> 内核栈, sscratch -> 用户栈
    + (7) 用户应用程序执行 ecall、非法指令等.
