# Lockdep Review for `ostd` and Direct Dependents

日期：2026-03-24

Historical note:

- This document is a snapshot review of one older workspace-wide run.
- Since then, lockdep has gained stronger local wrapper/callback propagation and several
  correctness fixes; treat this file as historical triage context, not as the current report.

本文档整理了当前分支对以下范围执行 `lockdep` 后得到的结果：

- `ostd`
- workspace 中所有直接依赖 `ostd` 的 crate

## 1. 分析范围

本次共分析 23 个 crate：

- `ostd`
- `aster-bigtcp`
- `aster-block`
- `aster-cmdline`
- `aster-console`
- `aster-framebuffer`
- `aster-i8042`
- `aster-input`
- `aster-kernel`
- `aster-logger`
- `aster-mlsdisk`
- `aster-network`
- `aster-pci`
- `aster-softirq`
- `aster-systree`
- `aster-time`
- `aster-uart`
- `aster-util`
- `aster-virtio`
- `osdk-frame-allocator`
- `osdk-heap-allocator`
- `osdk-test-kernel`
- `xarray`

这些 crate 是通过 `cargo metadata` 自动筛选得到的：

- 包名为 `ostd`
- 或 `Cargo.toml` 依赖中直接包含 `ostd`

## 2. 执行命令

执行方式等价于：

```bash
cargo run --manifest-path tools/lockdep/Cargo.toml --bin cargo-lockdep -- \
  --target x86_64-unknown-none \
  -p aster-bigtcp \
  -p aster-block \
  -p aster-cmdline \
  -p aster-console \
  -p aster-framebuffer \
  -p aster-i8042 \
  -p aster-input \
  -p aster-kernel \
  -p aster-logger \
  -p aster-mlsdisk \
  -p aster-network \
  -p aster-pci \
  -p aster-softirq \
  -p aster-systree \
  -p aster-time \
  -p aster-uart \
  -p aster-util \
  -p aster-virtio \
  -p osdk-frame-allocator \
  -p osdk-heap-allocator \
  -p osdk-test-kernel \
  -p ostd \
  -p xarray -- --quiet
```

## 3. 总体结果

本次输出：

- 23 个 crate
- 25383 个 MIR-backed function
- 2963 个 lock event
- 810 个 lock edge
- 2 个 cycle
- 0 个 IRQ conflict
- 2 个 AA/self-loop

需要注意：

- 这 2 个 cycle 与 2 个 AA/self-loop 实际上对应同两处代码；
- 本轮没有发现 IRQ conflict；
- 相比之前的结果，跨类型锁类误合并导致的 IRQ 告警已经明显减少。

## 4. 各 crate 概览

锁相关边数量较多的 crate 主要是：

- `aster-kernel`: 2209 event, 605 edge
- `aster-mlsdisk`: 292 event, 167 edge
- `ostd`: 121 event, 5 edge
- `aster-network`: 46 event, 11 edge
- `aster-bigtcp`: 61 event, 8 edge
- `aster-virtio`: 83 event, 7 edge

其余 crate 的边数较少，或没有形成全局报告中的可疑 cycle。

## 5. 告警逐条复核

### 5.1 `ExfatInode::reclaim_space`

涉及报告：

- `cycle 1`
- `aa deadlock 1`

位置：

- [`kernel/src/fs/fs_impls/exfat/inode.rs:812`](/root/asterinas-codex/kernel/src/fs/fs_impls/exfat/inode.rs#L812)
- [`kernel/src/fs/fs_impls/exfat/inode.rs:816`](/root/asterinas-codex/kernel/src/fs/fs_impls/exfat/inode.rs#L816)

关键代码：

```rust
pub(super) fn reclaim_space(&self) -> Result<()> {
    let inner = self.inner.write();
    let fs = inner.fs();
    let fs_guard = fs.lock();
    self.inner.write().resize(0, &fs_guard)?;
    self.inner.read().page_cache.resize(0)?;
    Ok(())
}
```

结论：

- 这是一个真实问题。

理由：

- 第 1 次 `self.inner.write()` 的 guard 保存在 `inner` 中；
- 在 `inner` 仍然活着的作用域里，又调用了 `self.inner.write()`；
- 对同一个 `RwMutex` 再次获取写锁会等待前一次写锁释放；
- 但前一次写锁就在当前栈帧里，因此这里会形成同线程自锁；
- 后面的 `self.inner.read()` 在正常执行路径上也不可达。

判断：

- `cycle 1`: 真实问题
- `aa deadlock 1`: 真实问题

建议修复方向：

- 复用第一次拿到的 `inner` 写锁；
- 或显式缩小 `inner` 的生命周期，在重新加锁前先释放它。

### 5.2 `OverlayInode::unlink`

涉及报告：

- `cycle 2`
- `aa deadlock 2`

位置：

- [`kernel/src/fs/fs_impls/overlayfs/fs.rs:349`](/root/asterinas-codex/kernel/src/fs/fs_impls/overlayfs/fs.rs#L349)
- [`kernel/src/fs/fs_impls/overlayfs/fs.rs:361`](/root/asterinas-codex/kernel/src/fs/fs_impls/overlayfs/fs.rs#L361)
- [`kernel/src/fs/fs_impls/overlayfs/fs.rs:771`](/root/asterinas-codex/kernel/src/fs/fs_impls/overlayfs/fs.rs#L771)

相关代码：

```rust
pub fn unlink(&self, name: &str) -> Result<()> {
    let mut upper_guard = self.upper.lock();
    if upper_guard.is_none() {
        drop(upper_guard);
        self.build_upper_recursively_if_needed()?;
        upper_guard = self.upper.lock();
    }
    // ...
}
```

```rust
fn build_upper_recursively_if_needed(&self) -> Result<Arc<dyn Inode>> {
    let mut upper_guard = self.upper.lock();
    if let Some(upper) = upper_guard.as_ref() {
        return Ok(upper.clone());
    }
    // ...
}
```

结论：

- 当前更像误报，不像真实 self-lock。

理由：

- `unlink()` 在调用 `build_upper_recursively_if_needed()` 之前显式执行了 `drop(upper_guard)`；
- 因此进入 `build_upper_recursively_if_needed()` 时，并不存在同一个 `self.upper` guard 仍然存活的情况；
- `build_upper_recursively_if_needed()` 确实会再次获取 `self.upper.lock()`，但这是“释放后再获取”，不是直接递归自锁；
- 该函数内部还有递归调用父 inode 的 `build_upper_recursively_if_needed()`，这说明这里仍然存在“不同对象同字段锁”的实例级区分问题；
- 当前 lockdep 报告把这条路径压缩成了 `OverlayInode.upper` 上的自环，更接近实例敏感度不足造成的残余误报。

判断：

- `cycle 2`: 倾向误报
- `aa deadlock 2`: 倾向误报

需要额外关注的一点：

- 虽然这条不是“同一把锁自锁”，但 `build_upper_recursively_if_needed()` 自身确实可能在较长时间内持有 `upper` 锁并递归向父节点传播，这段设计仍值得单独审视。

## 6. 最终结论

本轮结果可以概括为：

- 真实问题：1 条
- 倾向误报：1 条
- IRQ conflict：0 条

也就是说，目前在 “`ostd` + 所有直接依赖 `ostd` 的 crate” 这个范围内，`lockdep` 的主要有效发现是：

- `ExfatInode::reclaim_space` 中对 `self.inner` 的重复写锁获取

而 `OverlayInode::unlink` 这条结果更像当前 lock class / 实例敏感度仍不够精细时留下的残余误报。

## 7. 后续建议

建议按下面顺序继续推进：

1. 先修复 `ExfatInode::reclaim_space` 的真实死锁问题。
2. 单独复查 `OverlayInode::unlink` / `build_upper_recursively_if_needed()` 的锁设计。
3. 继续提升 lockdep 的实例敏感度，尤其是：
   - 同类型不同对象上的同字段锁；
   - 递归 helper 中的对象来源链；
   - `self.upper` 与 `parent.upper` 这类路径的区分。
