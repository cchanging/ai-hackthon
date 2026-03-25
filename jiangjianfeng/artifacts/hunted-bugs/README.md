# Found Bugs

这个目录收集的是一次针对 `/root/asterinas-codex` 的 `lockdep` 仓库级扫描产物。

扫描范围：

- 所有直接或间接依赖 `ostd` 的 workspace crate
- 共 `24` 个 crate
- 目标架构：`x86_64-unknown-none`

本目录内容：

- `commands.md`
  记录本次找 bug 用到的命令
- `tool-output.txt`
  记录本次 `cargo-lockdep` 的关键终端输出
- `analysis/real-bugs.md`
  对高置信度真实问题的分组分析
- `analysis/likely-false-positives.md`
  对当前更像误报的报告分析

本次扫描总结果：

- `3` 个 potential lock cycle
- `75` 个 atomic-mode violation
- `0` 个 single-lock IRQ safety violation
- `0` 个 IRQ dependency violation
- `1` 个 AA/self-loop deadlock

结论摘要：

- 高置信度真实问题主要来自：
  - `ExfatInode::reclaim_space`
  - `VsockStreamSocket::{addr, bind}`
  - `MessageReceiver::bind`
  - `Condvar::wait_timeout` 的超时路径
  - `aster-mlsdisk` 中多处 `RwLock -> Mutex` / `RwLock -> Condvar` 风险
- 当前更像误报的报告主要有：
  - `ext2::write_lock_two_inodes`
  - `futex::lock_bucket_pairs`

说明：

- `75` 条 atomic-mode 原始命中并不代表 `75` 个互相独立的 bug。
- 这些命中已经按源码根因收敛成若干 bug family，见 `analysis/real-bugs.md`。
