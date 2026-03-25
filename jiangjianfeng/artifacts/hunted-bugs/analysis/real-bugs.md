# Real Bugs

本文件只保留当前更接近真实问题的报告，并把重复命中收敛成 bug family。

## B1. `ExfatInode::reclaim_space`

- 来源：
  - `cycle 1`
  - `aa deadlock 1`
- 位置：
  - `kernel/src/fs/fs_impls/exfat/inode.rs:812`
  - `kernel/src/fs/fs_impls/exfat/inode.rs:816`
- 结论：真实问题

分析：

- 函数先获取 `self.inner.write()`，并把 guard 绑定到局部变量。
- 在这个 guard 仍然存活时，又再次调用 `self.inner.write()`。
- 这会形成同线程对同一把 `RwMutex` 的重复写锁获取，属于直接 self-deadlock。

建议：

- 复用第一次拿到的写锁，或在重入前显式结束第一次 guard 的生命周期。

## B2. `VsockStreamSocket::addr`

- 来源：
  - atomic-mode `G1`
- 位置：
  - `kernel/src/net/socket/vsock/stream/socket.rs:293`
- 模式：
  - `RwLock(read) -> Mutex(lock)`
- 结论：真实问题

分析：

- 外层先持有 `self.status.read()`。
- 之后调用 `Init::bound_addr()`，内部再获取 `bound_addr.lock()`。
- 这是明显的 atomic mode 违规。

## B3. `VsockStreamSocket::bind`

- 来源：
  - atomic-mode `G2`
- 位置：
  - `kernel/src/net/socket/vsock/stream/socket.rs:146`
- 模式：
  - `RwLock(read) -> Mutex(lock)`
- 结论：真实问题

分析：

- 与 `addr()` 同类。
- 当前实现直接在持有 `self.status.read()` 的作用域里调用 `init.bind(addr)`。
- `init.bind()` 内部会进入 sleeping `Mutex` 路径。

## B4. `MessageReceiver::bind`

- 来源：
  - atomic-mode `G3`
- 位置：
  - `kernel/src/net/socket/unix/datagram/message.rs:159`
- 模式：
  - `SpinLock(lock) -> RwMutex(read)`
- 结论：真实问题

分析：

- 先持有 `self.addr.lock()`。
- 然后进入 VFS 路径解析。
- 路径解析过程中会获取 `RwMutex` 读锁。
- 这是典型的“持自旋锁进入会睡眠路径”。

## B5. `Condvar::wait_timeout` 超时分支

- 来源：
  - atomic-mode `G4`
- 位置：
  - `kernel/src/process/sync/condvar.rs:172`
- 模式：
  - `SpinLock(lock) -> Mutex(lock)`
- 结论：真实问题

分析：

- 超时分支里先持有 `self.counter.lock()`。
- `counter` 还活着时就执行 `lock.lock()`。
- 这是很直接的 atomic mode 违规，修复边界也清楚。

建议：

- 先 drop `counter`，再重新获取 `lock`。

## B6. `CryptoLog` 家族

- 来源：
  - atomic-mode `G5-G7`
- 位置：
  - `kernel/comps/mlsdisk/src/layers/1-crypto/crypto_log.rs:257`
  - `kernel/comps/mlsdisk/src/layers/1-crypto/crypto_log.rs:274`
  - `kernel/comps/mlsdisk/src/layers/1-crypto/crypto_log.rs:283`
- 模式：
  - `RwLock(read/write) -> Mutex(lock)`
- 结论：真实问题

分析：

- `read()`、`append()`、`flush()` 都会先持有 `self.mht.read()` 或 `self.mht.write()`。
- 后续流程会进入 `MhtStorage.crypt_buf: Mutex<CryptBuf>`。
- 原始 case 有多条，但根因是同一个：外层 `mht` 的 `RwLock` 持锁范围过长。

## B7. `MlsDisk::{sync, write, writev}`

- 来源：
  - atomic-mode `G8-G10`
- 位置：
  - `kernel/comps/mlsdisk/src/layers/5-disk/mlsdisk.rs:197`
  - `kernel/comps/mlsdisk/src/layers/5-disk/mlsdisk.rs:205`
  - `kernel/comps/mlsdisk/src/layers/5-disk/mlsdisk.rs:212`
- 模式：
  - `RwLock(read/write) -> Mutex(lock)` 以及 `RwLock -> Condvar`
- 结论：真实问题

分析：

- `write_sync_region.read()` / `write_sync_region.write()` 在整个慢路径上一直持有。
- 在这段作用域里继续调用 `self.inner.write*()` / `self.inner.sync()`。
- 后续路径会触达大量 sleeping primitive，如 `DataBuf`、`AllocTable`、`TxLogStore`、`Journal`、`MemTableManager`、`Condvar` 等。
- 这说明问题不是单点遗漏，而是整体锁设计让 atomic mode 覆盖了太长的工作路径。

## B8. `TxLsmTree` 家族

- 来源：
  - atomic-mode `G11-G14`
- 位置：
  - `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:444`
  - `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:476`
  - `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:520`
  - `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:521`
  - `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:675`
  - `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:681`
  - `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:683`
  - `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:689`
- 模式：
  - `RwLock(read) -> Mutex(lock)`
- 结论：真实问题

分析：

- 外层通常先持有 `self.sst_manager.read()` 或 `immutable_memtable()` 返回的 `RwLockReadGuard`。
- 在这个 guard 仍然活着时，继续进入 `SSTable.cache`、`tx_log_store`、`CurrentTx.tx_table` 等 sleeping 路径。
- 多条 raw case 本质上对应的是同一个设计问题：读锁作用域覆盖了整个事务/日志/查询慢路径。

## 优先级建议

如果要按修复难度和收益排序，建议先看：

1. `Condvar::wait_timeout`
2. `VsockStreamSocket::{addr, bind}`
3. `MessageReceiver::bind`
4. `CryptoLog`
5. `TxLsmTree`
6. `MlsDisk::{write, writev, sync}`
