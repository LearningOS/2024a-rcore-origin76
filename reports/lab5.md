# 1

在thread0退出时会执行add_stopping_task来确保ref有效并进行资源回收逻辑，

```rust
let mut process_inner = process.inner_exclusive_access();
        process_inner.children.clear();
        // deallocate other data in user space i.e. program code/data section
        process_inner.memory_set.recycle_data_pages();
        // drop file descriptors
        process_inner.fd_table.clear();
        // remove all tasks
        process_inner.tasks.clear();
```

其余资源采用Arc自动管理。

tcb会被sem和mutex的waitlist持有引用，不需要手动管理。

# 2

主要区别在于锁的释放时机和唤醒线程时的锁状态

先将locked置为false，再唤醒等待线程。可能导致被唤醒的线程和其他试图获取锁的线程同时竞争，被唤醒的线程不一定能立即获得锁

第二种实现如果有等待线程，保持locked为true并唤醒线程相当于直接将锁移交给等待队列中的第一个线程

避免了不必要的竞争，保证了FIFO顺序的锁获取


# 荣誉准则

在完成本次实验的过程（含此前学习的过程）中，我参考了[博客](http://lordaeronesz.top/2024/11/04/2024%E5%BC%80%E6%BA%90%E6%93%8D%E4%BD%9C%E7%B3%BB%E7%BB%9F%E8%AE%AD%E7%BB%83%E8%90%A5-rCore-Chapter8%E7%BB%83%E4%B9%A0/)的实现思路。，主要在于Alloc和Need数组的生成部分。

我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。