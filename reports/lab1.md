# 1

[rustsbi] RustSBI version 0.3.0-alpha.2, adapting to RISC-V SBI v1.0.0

运行 bad_addr
[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
[kernel] Panicked at src/syscall/fs.rs:11 called `Result::unwrap()` on an `Err` value: Utf8Error { valid_up_to: 3, error_len: Some(1) }

运行 bad_instructions
[kernel] IllegalInstruction in application, kernel killed it.
[kernel] Panicked at src/trap/mod.rs:72 Unsupported trap Exception(LoadFault), stval = 0x18!

运行 bad_reg
[kernel] IllegalInstruction in application, kernel killed it.
[kernel] Panicked at src/trap/mod.rs:72 Unsupported trap Exception(LoadFault), stval = 0x18!

# 2

a0是__restore传参，push_context 的返回值 即内核栈压入 Trap 上下文之后的栈顶

sstatus 是 Supervisor Status 寄存器，用于保存特权级别的状态，如中断使能、异常处理状态等。
sepc 是 Supervisor Exception Program Counter 寄存器，用于保存引发异常的指令地址。这个寄存器在异常返回时帮助恢复程序执行。
sscratch 是一个临时保存寄存器，在特权模式下可以用于保存执行上下文中需要临时存储的值，通常在异常处理中用作临时存储区域

x2管理栈帧，要基于它来找到每个寄存器应该被保存到的正确的位置。s4不被使用

现在 sp 重新指向用户栈栈顶，sscratch 也依然保存进入 Trap 之前的状态并指向内核栈栈顶。

sret会回到U

在L13一行之前 sp 指向用户栈， sscratch 指向内核栈（原因稍后说明），现在 sp 指向内核栈， sscratch 指向用户栈。

```rust
core::arch::asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") id
        );
```

ecall时进入S

# 荣誉准则

在完成本次实验的过程（含此前学习的过程）中，我曾通过微信群助教发言确定了syscall计数数组的拷贝

我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。




