# Lockdep 报告复核

日期：2026-03-24

历史说明：

- 本文是 2026-03-24 当时结果的源码复核快照，不等于当前实现状态。
- 之后已经落地了多项精度改进与 review 修复，包括：
  - 结构化 lock class
  - local callable / callback wrapper / returned-guard 传播
  - CFG join IRQ state merge 修复
  - 分支相关真实反向锁序边保留
- 因此本文里关于“误报根因”的分析仍有参考价值，但具体结论不应直接外推到当前版本输出。

本文件复核了当前分支对所有直接依赖 `ostd` 的 crate 运行 `lockdep` 后得到的告警。

本次全量结果：

- 3 个 `cycle`
- 1 个 `irq conflict`
- 4 个 `aa/self-loop`

注意：这 8 条并不是 8 个彼此独立的问题。其中：

- `cycle 1` 和 `aa deadlock 1` 指向同一个 epoll 位置；
- `cycle 3` 和 `aa deadlock 3` 指向同一个 overlayfs 位置；
- `irq conflict 1` 和 `aa deadlock 4` 指向同一个 IRQ 重入类问题。

## 总结

结论如下：

- 真实问题：1 条
- 误报：7 条

唯一明确的真实问题是：

- `aa deadlock 2`: `ExfatInode::reclaim_space`

其余报告都更像当前 lockdep 原型的精度问题，主要是：

- 锁类标识不稳定，使用了类似 `arg1.*.fieldN`、`local12.*.field0` 这种过于粗糙的键；
- 缺少对象实例级区分，导致不同对象上的同名字段锁被合并；
- 在少数情况下，把“持锁状态下调用只读 helper”误当成再次加锁。

## 1. `cycle 1` 和 `aa deadlock 1`

涉及位置：

- `kernel/src/events/epoll/entry.rs:330`
- `kernel/src/events/epoll/entry.rs:320`
- `kernel/src/events/epoll/entry.rs:225`
- `kernel/src/events/epoll/entry.rs:234`

相关代码：

```rust
let mut entries = self.entries.lock();

if !observer.is_ready(&entries) {
    observer.set_ready(&entries);
    entries.push_back(observer.weak_entry().clone())
}

self.pollee.notify(IoEvents::IN);
```

结论：

- 误报。

理由：

- 这个函数里对 `self.entries` 只有一次显式加锁，即 `self.entries.lock()`。
- `observer.is_ready(&entries)` 和 `observer.set_ready(&entries)` 只是接收一个 `SpinLockGuard` 引用，用它来表达“调用时必须已持有 ready-list 锁”，这两个 helper 本身并不会再次获取 `self.entries`。
- `is_ready` / `set_ready` 的实现只是对 `AtomicBool` 做 `load/store`，并没有新的锁操作。
- 因而，源码中不存在 “`ReadySet::push` 在同一路径里对同一个 `SpinLock` 连续加两次锁” 这一事实。

判断：

- `cycle 1`: 误报
- `aa deadlock 1`: 误报

更可能的根因：

- 分析器把“以 guard 作为参数的 helper 调用”错误地归并成了新的锁相关事件，或者把同一个函数内的持锁状态过度近似成了自环。

## 2. `cycle 2`

涉及位置：

- `kernel/comps/mlsdisk/src/layers/5-disk/block_alloc.rs:389`
- `kernel/src/fs/vfs/notify/inotify.rs:160`
- `kernel/comps/mlsdisk/src/layers/1-crypto/crypto_log.rs:274`
- `kernel/src/process/process/process_group.rs:75`
- `kernel/comps/mlsdisk/src/layers/5-disk/block_alloc.rs:369`

lockdep 给出的链路大致是：

1. `Mutex(lock)` -> `SpinLock(lock)`
2. `SpinLock(lock)` -> `RwLock(write)`
3. `RwLock(write)` -> `Mutex(lock)`
4. `Mutex(lock)` -> `Mutex(lock)`
5. `Mutex(lock)` -> `Mutex(lock)`

结论：

- 误报。

理由：

- 这条 witness 跨越了 `mlsdisk`、`inotify`、`process_group` 三组完全不同的数据结构，没有看到任何共享对象或统一锁层级。
- 报告中的锁类名是匿名的，例如：
  - `arg1.*.field1`
  - `arg1.*.field0`
  - `arg1.*.field2`
  - `local12.*.field0`
- 这些名字缺少类型名、定义点、实例来源，不能稳定标识“全局上同一个锁类”。
- 当前 witness 更像是不同函数里“第 N 个字段上的锁”被粗暴合并后，拼出了一条并不存在的全局环。
- 从源码看：
  - [`BlockAlloc::update_alloc_table`](kernel/comps/mlsdisk/src/layers/5-disk/block_alloc.rs) 只是在本地顺序获取 `diff_table`、`num_free`、`bitmap`；
  - [`InotifyFile::add_watch`](kernel/src/fs/vfs/notify/inotify.rs) 获取的是 `watch_map`；
  - [`CryptoLog::append`](kernel/comps/mlsdisk/src/layers/1-crypto/crypto_log.rs) 获取的是 `self.mht.write()`；
  - [`ProcessGroup::broadcast_signal`](kernel/src/process/process/process_group.rs) 获取的是 `self.inner.lock()`。
- 没有证据表明这些边对应的是同一个运行时锁集合，也没有证据表明存在可闭合的实际 ABBA 链路。

判断：

- `cycle 2`: 误报

更可能的根因：

- 锁类身份过于粗糙，导致跨类型、跨模块的锁被错误合并。

## 3. `cycle 3` 和 `aa deadlock 3`

涉及位置：

- `kernel/src/fs/fs_impls/overlayfs/fs.rs:780`
- `kernel/src/fs/fs_impls/overlayfs/fs.rs:772`

相关代码：

```rust
let mut upper_guard = self.upper.lock();
if let Some(upper) = upper_guard.as_ref() {
    return Ok(upper.clone());
}

let parent_upper = self
    .parent
    .as_ref()
    .unwrap()
    .build_upper_recursively_if_needed()?;
```

结论：

- 误报。

理由：

- 这里对 `self.upper` 的显式加锁只有一次。
- 后续递归调用发生在 `parent` 上，锁的是父 inode 的 `upper`，不是当前 inode 的 `upper`。
- 这条报告之所以形成 `Mutex(lock) -> Mutex(lock)` 自环，是因为分析器把所有 `OverlayInode.upper` 都合并成了同一个抽象锁类。
- 但 `self.upper` 和 `parent.upper` 是不同对象上的不同 mutex，不能因为字段名相同就视为同一把锁。

需要额外说明的一点：

- 这里虽然不是“同一把锁自锁”，但代码确实存在一个设计问题：它会在持有子节点 `upper` 锁时递归获取父节点 `upper` 锁，而且源码自己也留了 `FIXME`。
- 这说明“持锁范围偏长”是值得继续审视的，但它不是本次 lockdep 报告所声称的那个 `self_lock`。

判断：

- `cycle 3`: 误报
- `aa deadlock 3`: 误报

更可能的根因：

- 缺少对象实例级锁类区分，把同类型对象上的同字段锁混成了一个节点。

## 4. `irq conflict 1` 和 `aa deadlock 4`

涉及位置：

- `kernel/comps/uart/src/console.rs:52`
- `kernel/src/fs/fs_impls/ramfs/fs.rs:709`

相关代码：

```rust
for callback in self.callbacks.lock().iter() {
    (callback)(reader.clone());
}
```

```rust
fn atime(&self) -> Duration {
    self.metadata.lock().atime
}
```

结论：

- 误报。

理由：

- 中断侧拿的是 `UartConsole.callbacks`：
  - `kernel/comps/uart/src/console.rs`
  - 字段类型是 `SpinLock<Vec<&'static ConsoleCallback>, LocalIrqDisabled>`
- 任务侧拿的是 `RamInode.metadata`：
  - `kernel/src/fs/fs_impls/ramfs/fs.rs`
  - 字段类型是 `SpinLock<InodeMeta>`
- 这两个锁属于完全不同的结构体、不同字段、不同子系统。
- lockdep 把它们都映射成了同一个抽象类 `arg1.*.field1`，说明这里的锁类键没有包含足够的类型和实例信息。
- 因此，这并不是“同一把锁既在 HardIRQ 上下文中获取，又在可中断任务上下文中获取”。

需要额外说明的一点：

- `trigger_input_callbacks()` 确实是在持有 `callbacks` 锁时执行回调，这种模式本身值得单独审查；
- 但这属于“回调在 IRQ/持锁上下文里执行是否安全”的问题，不等于当前这条 lockdep 报告声称的“与 `RamInode.metadata` 是同一把锁并发生 IRQ 重入”。

判断：

- `irq conflict 1`: 误报
- `aa deadlock 4` (`irq_reentry`): 误报

更可能的根因：

- IRQ 检测建立在错误的锁类合并之上，导致不同类型的 `SpinLock` 被错误视为同一类。

## 5. `aa deadlock 2`

涉及位置：

- `kernel/src/fs/fs_impls/exfat/inode.rs:813`
- `kernel/src/fs/fs_impls/exfat/inode.rs:816`
- `kernel/src/fs/fs_impls/exfat/inode.rs:817`

相关代码：

```rust
let inner = self.inner.write();
let fs = inner.fs();
let fs_guard = fs.lock();
self.inner.write().resize(0, &fs_guard)?;
self.inner.read().page_cache.resize(0)?;
```

结论：

- 真实问题。

理由：

- 第 1 行已经拿到了 `self.inner.write()` 的写锁，并将 guard 保存在 `inner` 中。
- 在 `inner` 仍然活着的情况下，第 4 行再次调用 `self.inner.write()`。
- `ostd::sync::RwMutex::write()` 的语义是：只有在“没有 writer / upreader / reader”时才能成功获取写锁。
- 因此第二次 `write()` 会等待第一次 `write()` 释放，形成同线程自锁。
- 第 5 行的 `self.inner.read()` 也同样发生在第一次写锁 guard 仍然活着的作用域里；正常情况下代码甚至走不到这一步。

判断：

- `aa deadlock 2`: 真实问题

建议修复方向：

- 复用第一次拿到的 `inner` 写锁，不要再次调用 `self.inner.write()`；
- 若确实需要分阶段操作，应当显式缩小 guard 作用域，在重新加锁前先 `drop(inner)`；
- 还要检查 `page_cache.resize(0)` 是否必须在写锁内完成，还是可以在更新 inode 状态后、释放写锁后单独处理。

## 误报归因汇总

当前报告里最主要的误报来源有三类：

1. 锁类身份不稳定

- 典型表现：`arg1.*.field0`、`arg1.*.field1`、`arg1.*.field5`、`local12.*.field0`
- 这些名字无法可靠区分：
  - 不同类型；
  - 不同对象实例；
  - 不同函数里的局部来源。

2. 缺少对象实例敏感性

- 同一个结构体字段名，例如 `OverlayInode.upper`，在不同 inode 实例上应当视为不同锁对象；
- 当前分析把它们合并成了同一个节点，进而构造出假的自环。

3. 对“持锁状态下 helper 调用”的建模过粗

- 像 `Observer::is_ready(&entries)` 这种 API 只是要求调用者已经持锁；
- 它本身不代表新的加锁事件。

## 最终结论

这批报告里，当前唯一应当当作真实 bug 处理的是：

- `kernel/src/fs/fs_impls/exfat/inode.rs:812` 的 `ExfatInode::reclaim_space`

其余报告目前都更适合视为 lockdep 原型误报，不建议直接据此修改业务代码。更优先的方向是先提升分析器精度，尤其是：

- 让锁类标识包含稳定的类型/定义点/实例来源；
- 避免把不同对象上的同字段锁合并；
- 改善对 guard-parameter helper 的建模。
