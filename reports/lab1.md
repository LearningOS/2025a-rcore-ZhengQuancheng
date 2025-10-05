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

## 流程分析

- 任务切换主要由 __switch 完成, 其有三条主要的执行流程:
  - 流程1: 启动第一个 Task
    - rust_main -> run_first_task -> __switch -> __restore -> 第一个 Task 入口
  - 流程2: 主动执行 sys_yield 或被 TimerInterrupt 打断
    Task A -> __alltraps -> trap_handler -> TimerInterrupt/sys_yield  -> suspend_current_and_run_next -> run_next_task -> __switch -> Task B
  - 流程3: 主动执行 sys_exit 或被 FatalException 打断
    - Task A -> __alltraps -> trap_handler -> sys_exit/FatalException -> exit_current_and_run_next -> run_next_task -> __switch -> Task B
- 流程 1 和 2 切换到 Task B 执行, 首先执行 Task B 的内核执行流程, 
  - 后续流程主要分为两种情况:
    - Task B 第一次执行, 从 __switch 返回的执行流程为: 
      - __switch -> __restore -> Task B 入口
    - Task B 主动执行 sys_yield 或被 TimerInterrupt 中断
      - __switch -> run_next_task -> suspend_current_and_run_next -> trap_handler -> __restore -> Task B 位置
  - 为什么不会经过 exit_current_and_run_next ?
    - 内核态禁用中断, 已退出的任务状态为 Exited, 不会被调度器选中
- TrapContext 和 TaskContext 有啥区别?
  - TrapContext 中保存 Task 陷入 S Mode 前 CPU 的状态; TaskContext 中保存在 S Mode 下进行 Trap 处理过程中调用 __switch 之前的 CPU 状态.
  - 当 __switch 恢复 TaskContext 之后会继续处理 Trap, __restore 恢复 TrapContext 后则回到 User Mode 继续执行 Task.
  - Task 切换由编译器辅助完成, 其中 s0...s11、ra 为 Callee Saved 寄存器, 被调函数 __switch 需要保存和恢复这些寄存器的值. 而其他寄存器则无需保存, 或由调用者保存(编译器辅助完成). 故 TaskContext 包含 s0...s11、ra, 除此之外还包含 sp, 其保存 Task 内核栈的栈顶.
  - __switch 可看作函数, 与普通函数不同之处在于其可换(Task的内核)栈
- 进入 __restore 时 sp 指向什么内容？为什么一定会恰好指向此？
  - 调用流程1：从 trap_handler 返回. 
    - sp 为 Callee-Saved 寄存器, 调用函数前后 sp 的值不变. Task A 可能在 trap_handler 中发生任务切换, 即调用 __switch 函数保存 Task A 状态并切换到其他 Task. Task A 后面一定会再次被切换回来, 可看作从 __switch 函数返回. Task A 执行 __switch 前后 sp 未发生变化. 即将执行 __switch 看作一次普通的函数调用.  一次普通的 Trap 处理不一定会任务切换, 任务切换与否不影响 sp 值的变化, 分析执行流程对 sp 影响时可忽略 __switch.
    - 一般来说, 一次普通的 Trap 处理流程大概为 __alltraps -> trap_handler -> __restore. 将 trap_handler 看作普通的函数调用, 其执行前后 sp 的值未发生改变, 故 sp 的值取决于 __alltraps.  在 __alltraps 中, 首先执行 csrrw sp, sscratch, sp 使 sp 指向内核栈, 执行 addi sp, sp, -34*8 分配 TrapContext. 故刚进入 __restore 时 sp 指向 TrapContext.
  - 调用流程2: 执行第一个 Task
    - rust_main -> run_first_task -> __switch -> __restore -> 第一个 Task 入口
    - 此执行流程与上不同, 进入 __restore 前, __switch 执行 ld sp, 8(a1) 指令影响 sp. a1 保存传入 __switch 的第二个参数 next_task_cx_ptr, 其在 TASK_MANAGER 被初始化时被设置.
    - 分析代码可知, push_context 和 init_app_cx 返回值指向 Task 内核栈的 TrapContext, 在初始化 TASK_MANAGER 时指向 goto_restore 函数以初始化 TaskContext, 其 sp 字段指向 TrapContext, ra 字段指向 __restore.
    - a1 指向即将被切换到的 TrapContext: __switch 执行 ld sp, 8(a1) 使 sp 指向 TrapContext, 执行 ld ra, 0(a1) 使 ra 执行 __restore, 执行 ret 指令则跳转到 __restore 执行.
    - 综上分析, 进入 __restore 时 sp 指向当前 Task 的 TrapContext.
- 进入 __alltraps 开始时 sscratch 指向内核栈? 其在什么地方被设置? 发生任务切换会对其有什么影响?
  - 进入 __restore 时 sp 指向内核栈 TrapContext
  - __restore 负责从 Task 的 TrapContext 恢复用户栈地址到 sscratch [ld t2, 2*8(sp), csrw sscratch, t2], 退出 __restore 时再将内核栈地址交给 sscratch [csrrw sp, sscratch, sp].
  - 执行第一个 Task 和 恢复 Task 时, 都会经过 __restore, 此时 sscratch 保存当前 Task 的内核栈地址.
  - 每次进入 __alltraps, 会分配 TrapContext 存储空间, 再将当前 Task 的状态保存到 TrapContext. 从 __restore 退出时, 根据 TrapContext 恢复 Task 状态, 并归还 TrapContext 存储空间.

## 相关总结
+ [rCore ch1](https://wxoads5exs.feishu.cn/docx/LrpndzlZ8oPyenx1Pq4cWqUlnkg)
+ [rCore ch2](https://wxoads5exs.feishu.cn/docx/OPRbdlNeaoyHiTxsqaZcEwo2nXf)
+ [rCore ch3](https://wxoads5exs.feishu.cn/docx/XVbIdHgmbobnDhxBWzQcSAXAnIh)
