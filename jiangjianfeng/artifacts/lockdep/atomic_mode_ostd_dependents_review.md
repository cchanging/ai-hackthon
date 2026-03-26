<!-- SPDX-License-Identifier: MPL-2.0 -->

# lockdep atomic mode 报告复核（所有依赖 `ostd` 的 crate）

历史说明：

- 本文是特定时间点的 atomic-mode 报告复核。
- 当前 analyzer 的 interprocedural precision 已经比本文写作时更强，因此 raw case 数量和
  分组形态可能与本文不同；但 atomic-mode 规则本身与这里分析的代码风险类型仍然一致。

本文整理了当前分支对“所有依赖 `ostd` 的 workspace crate”运行 `lockdep` 后，
新增 atomic mode 规则得到的结果，并对每一条报出的 case 做源码级分析。

## 1. 分析范围

本次分析的包集合，是 workspace 中所有显式依赖 `ostd` 的 crate，共 22 个：

```text
aster-bigtcp
aster-block
aster-cmdline
aster-console
aster-framebuffer
aster-i8042
aster-input
aster-kernel
aster-logger
aster-mlsdisk
aster-network
aster-pci
aster-softirq
aster-systree
aster-time
aster-uart
aster-util
aster-virtio
osdk-frame-allocator
osdk-heap-allocator
osdk-test-kernel
xarray
```

执行命令：

```bash
cargo run --manifest-path tools/lockdep/Cargo.toml --bin cargo-lockdep -- \
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
  -p xarray \
  --target x86_64-unknown-none \
  -- --quiet
```

本次 `lockdep` 新规则的判定口径是：

- 当前持有 `SpinLock` / `RwLock` 时，再去获取 `Mutex` / `RwMutex`；
- 既包括函数内直接获取；
- 也包括当前 `lockdep` 已支持的“持锁后调用某函数，而该函数入口就会获取 sleeping lock”的跨函数摘要路径。

因此，这个规则本质上是在抓“在 atomic mode 下进入 sleeping lock”的代码路径。

## 2. 总体结果

运行结果摘要：

- 共分析 22 个 crate。
- 共报出 75 条 atomic mode violation。
- 这 75 条只分布在 2 个 crate：
  - `aster-kernel`: 4 条
  - `aster-mlsdisk`: 71 条
- 其余 20 个依赖 `ostd` 的 crate 本轮未报出 atomic mode case。

把 75 条 raw case 按源码位置归并后，可收敛为 14 组问题：

| 组 | raw case | 数量 | 位置 | 结论 |
| --- | --- | ---: | --- | --- |
| G1 | 1 | 1 | `kernel/src/net/socket/vsock/stream/socket.rs:293` | 真实问题 |
| G2 | 2 | 1 | `kernel/src/net/socket/vsock/stream/socket.rs:146` | 真实问题 |
| G3 | 3 | 1 | `kernel/src/net/socket/unix/datagram/message.rs:159` | 真实问题 |
| G4 | 4 | 1 | `kernel/src/process/sync/condvar.rs:172` | 真实问题 |
| G5 | 5-7 | 3 | `kernel/comps/mlsdisk/src/layers/1-crypto/crypto_log.rs:274` | 真实问题 |
| G6 | 8-10 | 3 | `kernel/comps/mlsdisk/src/layers/1-crypto/crypto_log.rs:283` | 真实问题 |
| G7 | 11-13 | 3 | `kernel/comps/mlsdisk/src/layers/1-crypto/crypto_log.rs:257` | 真实问题 |
| G8 | 14-37 | 24 | `kernel/comps/mlsdisk/src/layers/5-disk/mlsdisk.rs:212` | 真实问题 |
| G9 | 38-50 | 13 | `kernel/comps/mlsdisk/src/layers/5-disk/mlsdisk.rs:197` | 真实问题 |
| G10 | 51-63 | 13 | `kernel/comps/mlsdisk/src/layers/5-disk/mlsdisk.rs:205` | 真实问题 |
| G11 | 64-70 | 7 | `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:675/681/683/689` | 真实问题 |
| G12 | 71-73 | 3 | `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:520/521` | 真实问题 |
| G13 | 74 | 1 | `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:476` | 真实问题 |
| G14 | 75 | 1 | `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:444` | 真实问题 |

重要说明：

- `mlsdisk` 中的大量重复，不代表有几十个彼此独立的问题。
- 更准确地说，是少数“外层 `RwLock` 持锁范围过大”的根因，被当前跨函数摘要展开成了多条 reachable sleeping-lock 边。
- 这些重复边依然有价值，因为它们说明“同一个 atomic-mode 违例会通向很多具体的 sleeping primitive”。

## 3. 分组分析

### G1. case 1

- raw case: `1`
- 位置: `kernel/src/net/socket/vsock/stream/socket.rs:293`
- 模式: `RwLock(read) -> Mutex(lock)`

源码：

```rust
fn addr(&self) -> Result<SocketAddr> {
    let inner = self.status.read();
    let addr = match &*inner {
        Status::Init(init) => init.bound_addr(),
        Status::Listen(listen) => Some(listen.addr()),
        Status::Connected(connected) => Some(connected.local_addr()),
    };
    ...
}
```

而 `Init::bound_addr()` 会执行：

```rust
pub fn bound_addr(&self) -> Option<VsockSocketAddr> {
    *self.bound_addr.lock()
}
```

分析：

- `self.status.read()` 是 `RwLock` 读锁，属于 atomic mode；
- `init.bound_addr()` 内部获取 `Mutex`；
- 读锁在整个 `match` 期间都还活着，没有先释放。

结论：

- 这是直接的 `RwLock(read) -> Mutex(lock)`；
- 属于高置信度真实问题。

### G2. case 2

- raw case: `2`
- 位置: `kernel/src/net/socket/vsock/stream/socket.rs:146`
- 模式: `RwLock(read) -> Mutex(lock)`

源码：

```rust
fn bind(&self, sockaddr: SocketAddr) -> Result<()> {
    let addr = VsockSocketAddr::try_from(sockaddr)?;
    let inner = self.status.read();
    match &*inner {
        Status::Init(init) => init.bind(addr),
        ...
    }
}
```

而 `Init::bind()` 内部会多次获取 `self.bound_addr.lock()`。

分析：

- 外层先拿 `self.status.read()`；
- 然后在 `Status::Init(init)` 分支里直接调用 `init.bind(addr)`；
- `Init::bind()` 用的是 sleeping `Mutex`；
- 当前实现不像 `connect()` / `listen()` 那样先 clone 出 `init` 再释放外层 `RwLock`。

结论：

- 这是高置信度真实问题。

### G3. case 3

- raw case: `3`
- 位置: `kernel/src/net/socket/unix/datagram/message.rs:159`
- 模式: `SpinLock(lock) -> RwMutex(read)`

源码：

```rust
pub(super) fn bind(&self, addr_to_bind: UnixSocketAddr) -> Result<()> {
    let mut addr = self.addr.lock();

    if addr.is_some() {
        return addr_to_bind.bind_unnamed();
    }

    let bound_addr = addr_to_bind.bind()?;
    ...
}
```

其中 `UnixSocketAddr::bind()` 对路径型地址会走到：

```rust
let path = ns::create_socket_file(&path_name)?;
```

而 `create_socket_file()` 内部会获取路径解析器的 `RwMutex` 读锁：

```rust
let path_resolver = fs_ref.resolver().read();
```

分析：

- `self.addr.lock()` 是 `SpinLock`；
- 在持有这把自旋锁时，又进入 VFS 路径解析并获取 `RwMutex`；
- 这不是误报，链路是直接可见的。

结论：

- 高置信度真实问题。

### G4. case 4

- raw case: `4`
- 位置: `kernel/src/process/sync/condvar.rs:172`
- 模式: `SpinLock(lock) -> Mutex(lock)`

源码：

```rust
match res {
    Ok(()) => Ok((lock.lock(), false)),
    Err(_) => {
        let mut counter = self.counter.lock();
        counter.waiter_count -= 1;
        Err(LockErr::Timeout((lock.lock(), true)))
    }
}
```

分析：

- `self.counter` 是 `SpinLock`；
- 在超时分支里，`counter` 还活着时就执行 `lock.lock()` 重新获取 `Mutex`；
- 这正是 atomic mode 规则要拦截的模式。

结论：

- 高置信度真实问题；
- 修复方向也很明确：先 drop `counter`，再重拿 `lock`。

### G5. case 5-7

- raw case: `5-7`
- 位置: `kernel/comps/mlsdisk/src/layers/1-crypto/crypto_log.rs:274`
- 模式: `RwLock(write) -> Mutex(lock)`

源码：

```rust
pub fn append(&self, buf: BufRef) -> Result<()> {
    ...
    self.mht.write().append_data_nodes(data_nodes)
}
```

这 3 条 raw case 都指向同一把 `MhtStorage.crypt_buf: Mutex<CryptBuf>`，
只是落在不同 helper 路径上：

- `Mht::<L>::do_build`
- `PreviousBuild::collect_incomplete_nodes`

分析：

- `self.mht.write()` 是 `RwLock` 写锁；
- 后续 build / collect 流程里会去拿 `crypt_buf.lock()`；
- 外层 `RwLock` 没有提前释放。

结论：

- 真实问题；
- 3 条 raw case 属于同一个根因的展开，不是 3 个独立 bug。

### G6. case 8-10

- raw case: `8-10`
- 位置: `kernel/comps/mlsdisk/src/layers/1-crypto/crypto_log.rs:283`
- 模式: `RwLock(write) -> Mutex(lock)`

源码：

```rust
pub fn flush(&self) -> Result<()> {
    self.mht.write().flush()
}
```

这 3 条 raw case 仍然都落到 `MhtStorage.crypt_buf: Mutex<CryptBuf>`，
只是经过的内部 helper 和 G5 相同。

分析：

- `flush()` 与 `append()` 一样，持有 `self.mht.write()`；
- 之后进入会获取 `crypt_buf` 的路径；
- 违反 atomic mode。

结论：

- 真实问题。

### G7. case 11-13

- raw case: `11-13`
- 位置: `kernel/comps/mlsdisk/src/layers/1-crypto/crypto_log.rs:257`
- 模式: `RwLock(read) -> Mutex(lock)`

源码：

```rust
pub fn read(&self, pos: Lbid, buf: BufMut) -> Result<()> {
    let mut search_ctx = SearchCtx::new(pos, buf);
    self.mht.read().search(&mut search_ctx)?;
    ...
}
```

这 3 条 raw case 也都落到 `MhtStorage.crypt_buf: Mutex<CryptBuf>`，
分别来自：

- `AppendDataBuf::search_data_nodes`
- `Mht::<L>::search_hierarchy`

分析：

- `self.mht.read()` 期间调用 `search()`；
- `search()` 的不同分支都会读底层存储，并获取 `crypt_buf.lock()`；
- 这是直接的 `RwLock(read) -> Mutex(lock)`。

结论：

- 真实问题。

### G8. case 14-37

- raw case: `14-37`
- 位置: `kernel/comps/mlsdisk/src/layers/5-disk/mlsdisk.rs:212`
- 模式: `RwLock(write) -> Mutex(lock)`

源码：

```rust
pub fn sync(&self) -> Result<()> {
    let _wguard = self.inner.write_sync_region.write();
    self.inner.sync().unwrap();
    ...
}
```

`self.inner.sync()` 后续会继续进入：

- `flush_data_buf()`
- `logical_block_table.sync()`
- `block_validity_table.do_compaction(...)`
- `tx_log_store.sync()`

这些路径再继续展开，会触达很多 sleeping lock，包括但不限于：

- `DataBuf.buf`
- `DataBuf.is_full.inner`
- `AllocTable.bitmap`
- `AllocTable.num_free.inner`
- `TxProvider.tx_table`
- `TxLogStore` / `RawLogStore` / `Journal` 的内部 `Mutex`
- `WalAppendTx.inner`
- `Compactor.handle`
- `MemTableManager.mutable`
- `MemTableManager.is_full.inner`
- `os::Condvar::wait()` 里的 `Mutex`

分析：

- `write_sync_region` 是 `RwLock<()>`；
- `sync()` 在持有其写锁的整个期间执行大量明显可能睡眠的工作；
- raw case 数量很多，是因为当前摘要把所有 reachable 的 sleeping lock 都展开了；
- 但这并不削弱问题真实性，反而说明持锁范围已经把整个慢路径都罩住了。

结论：

- 真实问题；
- 这是本轮最值得优先处理的一组之一。

### G9. case 38-50

- raw case: `38-50`
- 位置: `kernel/comps/mlsdisk/src/layers/5-disk/mlsdisk.rs:197`
- 模式: `RwLock(read) -> Mutex(lock)`

源码：

```rust
pub fn write(&self, lba: Lba, buf: BufRef) -> Result<()> {
    self.check_rw_args(lba, buf.nblocks())?;
    let _rguard = self.inner.write_sync_region.read();
    self.inner.write(lba, buf)
}
```

分析：

- `write_sync_region.read()` 也是 `RwLock` 读锁；
- `self.inner.write()` 会继续落入 `flush_data_buf()`、`TxLsmTree::put()`、`WalAppendTx`、
  `DataBuf`、`AllocTable`、`MemTableManager`、`Compactor` 等 sleeping primitive；
- 13 条 raw case 都是这一根因的不同 reachable target。

结论：

- 真实问题。

### G10. case 51-63

- raw case: `51-63`
- 位置: `kernel/comps/mlsdisk/src/layers/5-disk/mlsdisk.rs:205`
- 模式: `RwLock(read) -> Mutex(lock)`

源码：

```rust
pub fn writev(&self, lba: Lba, bufs: &[BufRef]) -> Result<()> {
    self.check_rw_args(lba, ...)?;
    let _rguard = self.inner.write_sync_region.read();
    self.inner.writev(lba, bufs)
}
```

分析：

- 与 G9 本质相同，只是入口换成了 `writev()`；
- raw case 的 sleeping target 也与 G9 基本一致；
- 仍然是持有 `RwLock` 读锁后进入明显会睡眠的写路径。

结论：

- 真实问题。

### G11. case 64-70

- raw case: `64-70`
- 位置:
  - `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:675`
  - `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:681`
  - `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:683`
  - `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:689`
- 模式: `RwLock(read) -> Mutex(lock)`

源码关键片段：

```rust
let sst_manager = self.sst_manager.read();
for (&id, sst) in ssts.filter(...) {
    let mut synced_records_iter = sst
        .iter(master_sync_id, true, &tx_log_store, Some(&listener))
        .peekable();

    if synced_records_iter.peek().is_some() {
        let new_log = tx_log_store.create_log(bucket)?;
        let new_sst =
            SSTable::build(synced_records_iter, master_sync_id, &new_log, None)?;
        ...
        continue;
    }

    tx_log_store.delete_log(id)?;
    ...
}
```

分析：

- 外层 `self.sst_manager.read()` 在整个循环期间一直持有；
- 同一持锁区间内又做了：
  - `sst.iter(...)`，会碰到 `SSTable.cache: Mutex<_>`
  - `tx_log_store.create_log(...)`
  - `SSTable::build(...)`
  - `tx_log_store.delete_log(...)`
  - `CurrentTx` 上的 `tx_table` 访问
- 这 7 条 raw case 对应的是上面这些不同的 sleeping target。

结论：

- 真实问题；
- 根因是 `sst_manager.read()` 持锁范围覆盖了完整 migration 事务。

### G12. case 71-73

- raw case: `71-73`
- 位置:
  - `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:520`
  - `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:521`
- 模式: `RwLock(read) -> Mutex(lock)`

源码关键片段：

```rust
let immutable_memtable = self.memtable_manager.immutable_memtable();
let records_iter = immutable_memtable.iter();
let sync_id = immutable_memtable.sync_id();

let sst = SSTable::build(records_iter, sync_id, &tx_log, Some(&event_listener))?;
self.tx_log_store.delete_log(wal_id)?;
```

分析：

- `immutable_memtable()` 返回 `RwLockReadGuard`；
- 在这个 guard 还活着时，就继续执行：
  - `SSTable::build(...)`
  - `tx_log_store.delete_log(...)`
  - `CurrentTx` 的 `tx_table` 访问
- 因此形成 3 条 raw case。

结论：

- 真实问题。

### G13. case 74

- raw case: `74`
- 位置: `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:476`
- 模式: `RwLock(read) -> Mutex(lock)`

源码：

```rust
let sst_manager = self.sst_manager.read();
...
sst.access_range(range_query_ctx, &self.tx_log_store)?;
```

分析：

- `sst_manager.read()` 持有期间，`sst.access_range(...)` 会访问 `SSTable.cache`；
- `SSTable.cache` 是 `Mutex<LruCache<...>>`；
- 这是直接的 `RwLock(read) -> Mutex(lock)`。

结论：

- 真实问题。

### G14. case 75

- raw case: `75`
- 位置: `kernel/comps/mlsdisk/src/layers/4-lsm/tx_lsm_tree.rs:444`
- 模式: `RwLock(read) -> Mutex(lock)`

源码：

```rust
let sst_manager = self.sst_manager.read();
...
if let Ok(target_value) = sst.access_point(key, &self.tx_log_store) {
    return Ok(target_value);
}
```

分析：

- 与 G13 相同，只是 `access_point()` 是单点读取；
- 同样会进入 `SSTable.cache: Mutex<_>`；
- 外层 `sst_manager.read()` 未先释放。

结论：

- 真实问题。

## 4. 结论

这 75 条 atomic mode 报告，当前看都更接近真实问题，而不是分析器误报。

更准确地说：

- `aster-kernel` 有 4 条高置信度真实问题；
- `aster-mlsdisk` 有 71 条报告，但它们主要收敛为 10 组真实根因；
- 其中最值得优先处理的是：
  - `MlsDisk::{write, writev, sync}` 对 `write_sync_region` 的持锁范围；
  - `TxLsmTree` 在持有 `sst_manager` / `immutable_memtable` 的 `RwLock` 期间进入事务/日志路径；
  - `CryptoLog` 在持有 `mht` 的 `RwLock` 时进入 `crypt_buf: Mutex<_>`；
  - `Condvar::wait_timeout` 超时分支里 `SpinLock -> Mutex` 的直接违例。

如果后续要逐步修复，建议优先顺序是：

1. `Condvar::wait_timeout`
2. `VsockStreamSocket::{bind, addr}`
3. `MessageReceiver::bind`
4. `CryptoLog`
5. `TxLsmTree`
6. `MlsDisk::{write, writev, sync}`

原因是前 4 组更局部、修复边界更清晰；后两组涉及 `mlsdisk` 的整体锁设计，改动面会更大。
