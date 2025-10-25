# Lab5

## 编程作业
+ 按照题目的提示, 即可实现死锁检测 `DeadlockDetector`, 主要代码在 `os/src/sync/deadlock_detector.rs`. 对 `mutex` 和 `semaphore` 操作的 `syscall` 中调用 `DeadlockDetector` 的接口. 
+ `DeadlockDetector` 后运行死锁检测相关的测试用例, 发现无法直接卡死. 仔细阅读并运行测试用例, 发现调用 sleep 函数之前的代码可运行, 之后的代码无法运行. 检查 os 代码, `sys_get_time` 未实现, 猜测其为测试用例卡死的原因. 实现 `sys_get_time` 后, 程序正常运行.

## 简答作业
1. 当主线程退出导致整个进程退出时, 需要回收该进程拥有的所有资源. 
    + 需要回收的资源有哪些?
        + 地址空间 MemorySet、线程内核栈 KernelStack、(线程)任务控制块 TaskControlBlock、文件描述符表、进程控制块 ProcessControlBlock、Mutex、Semaphore、Condvar 等
    + 其他线程的 TaskControlBlock 可能在哪些位置被引用?
        + TimerCondVar、Condvar、MutexBlocking、Semaphore、TaskManager、ProcessControlBlock、Processor 等
2. 对比以下两种 Mutex 中的实现, 二者有什么区别? 这些区别可能会导致什么问题? \
    + 主要区别
        + lock 函数: Mutex1 有循环 loop, Mutex2 没有.
        + unlock 函数: Mutex2 尝试直接将锁直接交给下一个等待的任务, 并未归还给系统. Mutex1 则是先归还锁.
