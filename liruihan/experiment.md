## Day 1: Plan

 - 不与已有代码对齐
> 你要多读一下已有的代码，尽量按照已有代码的风格设计（除非已有代码的涉及真的有扎实理由不能满足需要）
>
> 比如 kernel/src/net/socket/ip/stream/mod.rs 里面叫 State，你就不要叫 SocketState，然后那里 State 的划分是 Init、Connecting、Connected、Listen，你就先看看这样划分是不是能满足你的需要，你会发现其实你没有必要创建 Init 和 Bound 两个类型的，直接在里面用 Option 就好了，同理 Closed 类型也不需要存在，Connection 即使关了也会维持在 Connected 状态，这样就好了，不会引入问题，除非你能说出单列 Closed 有啥好处

 - 不会设计锁保护内容
> ConnectionInner 中不需要无锁 atomic 访问的可以合并成一个带锁的，rx_queue、peer_buf_alloc、peer_fwd_cnt总是一起访问，合并到一个锁里会更好

 - 不会设计锁保护内容 x2
> VsockDevice 的锁分太细了，queue 和 buffer 没必要分成单独的锁，它们的锁总是要一起获取的

 - 不理解代码
> tx_buffers 中只有 pending 的 buffer，没有在 queue 中的 buffer，这个不行，你看看 virtio net 的设计，TX queue 里的 buffer 也要找地方存储，同样 tx queue, pending buffer, tx buffer 没必要分成单独的锁了

 - 不理解代码 x2
> StreamObserver 和 pollee 重了，保留 pollee 即可，StreamObserver 在 TCP 中用是因为 bigtcp 看不到 Pollee

 - 忽视基本规律 （x2，后面改其他东西的时候又忘了并改坏了）
> PendingTx 应该要在 virtio 模块里保存吧，但是 virtio 模块里是看不到 ConnectionInner 的，所以这里可能需要通过 Box 保存一个 trait

 - 不会使用引入计数
> RxPayload 里面先有个 Arc<RxPayloadInner>，再有个 Arc<ReceivedBuffer> 的意义是啥？不要这样使用 Arc，每使用一个 Arc 你需要说明这里为什么必须要有引用计数，如果这个类型保存数据（比如 buffer），不需要有引用计数，那么让外面的用户持有 Arc 即可，比如 Arc<RxPacket>

 - 不会使用引用计数 x2
> *Prompt:* Arc<RxBuffer> 中的 Arc 有什么意义吗？没有的话，应该全换成 RxBuffer
>
> *Reply:* 先检查 `10-plan` 里 `Arc<RxBuffer>` 的落点；如果这里确实没有共享需求，我会直接把文档改成 `RxBuffer`。 

**Tip:** 多次生成 plan 会造成信息丢失，可以考虑让它自己复制一份 plan 在上面进行修改，或者开个新的 session 让它把历史全过一边也可以发现这类问题

## Day 2: Code

plan 很难做到尽善尽美，有些细节很难在 review 过程中注意到，而实现时会出现问题，这次 codex 大概中断了 10 次提出对 plan 的修改，这样会比较影响效率因为也很难专心做其他事情
 - 其中大概一半左右首选方案也是对的，但如果无监督直接做大概率最终实现会偏移 plan，例如最后几次没仔细看就加了一些非预期的 API

最终实现的代码客观上说比目前星绽主线中已经合入的版本方便我继续修改（必经是按照我的 design 去 review 过很多轮的），不过细节问题仍然很多（下面代码）

 - lock order / lock time 与 heap allocation 的权衡
```diff
-        let mut completions = Vec::new();
-        let mut tx = self.lock_tx();
```

 - 破坏 type invariant、且与另一 API 不自洽的 API 实现
```diff
-    pub fn drain_used(&mut self) {
-        while self.state.queue.can_pop() {
-            let Ok((token, _len)) = self.state.queue.pop_used() else {
-                break;
-            };
-            self.state.inflight[token as usize] = None;
-        }
-    }
```

 - 冗余设计的且缺少复用的类型
```diff
-struct EventState {
-    queue: VirtQueue,
-    buffers: Vec<Option<EventBuffer>>,
-}
```

 - 不当的能在硬 IRQ 中访问的锁设置
```diff
-    rx_pending: SpinLock<BTreeSet<String>, BottomHalfDisabled>,
-    tx_pending: SpinLock<BTreeSet<String>, BottomHalfDisabled>,
```

  - 混乱的函数实现 （下面是重写后的）
```diff
+        let tx_queue = TxQueue::new(transport.as_mut());
+        let rx_queue = RxQueue::new(transport.as_mut());
+        let event_queue = EventQueue::new(transport.as_mut());
```

 - 不良定义的 API 导致的重复加锁：
```rust
    pub(super) fn update_tx_cnt(&self, bytes: usize) {
        let mut state = self.state.lock();
        state.credit.tx_cnt = state.credit.tx_cnt.saturating_add(bytes as u32);
    }
 
    pub(super) fn mark_credit_reported(&self) {
        let mut state = self.state.lock();
        state.credit.last_reported_fwd_cnt = state.credit.local_fwd_cnt;
    }
```
```rust
        self.inner.update_tx_cnt(payload_len);
        if matches!(submit, TxSubmit::SubmittedToQueue) {
            self.inner.mark_credit_reported();
        }

```

 - 潜在的条件竞争（尽管窗口很小）
```rust
        timer.lock().set_timeout(Timeout::After(duration));

        let mut timer_state = self.timer.lock();
        if let Some(old_timer) = timer_state.replace(ConnectionTimerState { generation, timer }) {
            old_timer.timer.lock().cancel();
        }
```

**反思:** 总体来看这样搞 plan 和 code 都不是很高效，完全不如自己从头写一份 design 给它看，或者两天的时间如果已经有明确 design，直接写代码的时间怕是也够了

**经验:** 我感觉它处理的最好的部分是维护 vsock 的状态机（建立连接、shutdown、reset 等对状态的影响），这块我其实之前的 design 中也没有去想，因为其实算实现细节而且不影响什么全局的 API （不像锁顺序、队列结构影响大），而且 virtio spec 中也描述不清楚，codex 自动找 Linux 实现对标，生成的结果虽有待进一步仔细核查，但看上去还是比较像模像样的

