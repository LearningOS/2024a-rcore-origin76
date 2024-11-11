# 1

因为每一步的步长都小于$\frac{stride}{2}$,考虑最大pass的上一步，此时它是最小pass所以被执行，执行后的新的最小pass一定大于原最小pass，而步长小于$\frac{stride}{2}$

```rust
impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let diff = self.0.wrapping_sub(other.0);
        if diff == 0 {
            None 
        } else if diff < (BigStride / 2) {
             Some(Ordering::Greater)
        } else {
             Some(Ordering::Less)
        }
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}
```

# 荣誉准则

在完成本次实验的过程（含此前学习的过程）中，我没有获取guide和bookv3以外的任何指导。 

我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

