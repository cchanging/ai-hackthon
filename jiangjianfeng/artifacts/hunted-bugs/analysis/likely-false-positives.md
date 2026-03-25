# Likely False Positives

本文件记录这次扫描中更像误报的命中，避免把所有 `lockdep` 报告都直接当成真实 bug。

## F1. `ext2::write_lock_two_inodes`

- 来源：
  - 当前扫描的 `cycle 2`
- 位置：
  - `kernel/src/fs/fs_impls/ext2/inode.rs:949`
  - `kernel/src/fs/fs_impls/ext2/inode.rs:953`
- 结论：当前更像误报

分析：

- 这个 helper 会先按 `ino` 排序，再获取第二把 inode 写锁。
- 报告里的 `A -> B` 和 `B -> A` 来自同一函数中互斥的两个分支。
- 运行时顺序是 canonical order，不是任意交换顺序。

为什么还会报：

- 当前分析器对 ordered double-lock helper 的路径敏感度还不够。
- 它看到了两个分支上的相反顺序，但没有保留 “这是由同一个全序比较决定的互斥分支”。

## F2. `futex::lock_bucket_pairs`

- 来源：
  - 当前扫描的 `cycle 3`
- 位置：
  - `kernel/src/process/posix_thread/futex.rs:377`
  - `kernel/src/process/posix_thread/futex.rs:382`
- 结论：当前更像误报

分析：

- 这个 helper 会先比较 bucket index，再决定获取顺序。
- 因此两条相反的锁序边并不会在同一输入上同时成立。
- 这是典型的“有序双锁 helper 被路径合并成假环”的情况。

为什么还会报：

- 当前全局图摘要仍然把互斥分支上的边合并了。
- 这类模式需要更强的 ordered-helper 识别或显式路径条件传播。

## 不在本轮发现中的类别

- `single-lock IRQ safety violation`: `0`
- `IRQ dependency violation`: `0`

也就是说，这次全仓库扫描里没有新的 IRQ 类 bug 命中。
