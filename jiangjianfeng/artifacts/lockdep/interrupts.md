# 当前 lockdep 中断相关分析现状

本文总结当前 `tools/lockdep` 原型对中断相关死锁分析已经做到的事情、实际采用的判定方式，以及还没有覆盖的部分。

状态更新：

- 本文档已经同步到当前实现，包括：
  - 显式 `IrqExecutionState`
  - local callable alias / callback wrapper / nested callback wrapper 传播
  - returned guard 通过本地 wrapper 传播
  - review 中修复的 CFG join IRQ 状态合流问题
- 但跨 crate 传播仍未实现，`ordered_helpers` 仍然 inert。

结论先行：

- 现在的实现已经能识别一小组 Asterinas 特定的 IRQ 入口，并把这些入口上下文传播到 crate 内直接调用链上。
- 它已经能给加锁事件、锁序边和 lock usage state 打上 IRQ 相关信息，并生成 `single_lock_irq_violation` / `irq_dependency_violation`；`irq conflict` / `irq_reentry` 目前主要作为兼容输出保留。
- 但它还不是 Linux lockdep 风格的“完整 IRQ 安全状态机”。
  当前实现虽然已经维护了一组 usage bits，并据此做 safe/unsafe 规则检查，但底层仍然主要依赖启发式上下文分类与局部 guard 语义，而不是显式跟踪完整的 IRQ flag 状态机。

## 1. 当前实现到底做了什么

### 1.1 先识别 IRQ 入口，再把上下文传播出去

中断相关分析的入口在 `tools/lockdep/analysis/src/collect.rs`。
当前内建识别三类注册点：

- `IrqLine::on_active` -> `HardIrqTopHalf`
- `register_bottom_half_handler_l1` -> `BottomHalfL1`
- `register_bottom_half_handler_l2` -> `BottomHalfL2`

除此之外，`lockdep.toml` 里的 `irq_entries` 也已经可以补充额外的 IRQ 入口描述。

入口识别方式是扫描 MIR 里的 `Call` terminator，然后把回调参数解析成目标函数。当前能解析的回调形态主要是：

- 直接函数项
- 赋给 local 的闭包/函数项

随后，分析器会构建 crate 内“直接调用图”，把这些非 `Task` 上下文沿着 crate 内直接调用边向下传播。最终每个函数都会拿到一个 `contexts` 集合；如果没有识别到任何特殊上下文，则默认是 `Task`。

这意味着当前上下文传播的能力是“有的”，但边界也很明确：

- 只看 crate-local summary propagation
- 已能覆盖一部分 local callable alias / callback wrapper / nested callback wrapper
- 不做跨 crate 的上下文摘要传播
- 不做 trait object、函数指针逃逸后的精确传播

### 1.2 上下文不是固定标签，会在函数体内继续细化

当前原型识别的上下文一共 6 个：

- `Task`
- `TaskIrqDisabled`
- `HardIrqTopHalf`
- `BottomHalfL1`
- `BottomHalfL1IrqDisabled`
- `BottomHalfL2`

函数体内的上下文细化主要来自两类信息：

1. `DisabledLocalIrqGuard`
2. 获取锁后是否会在持锁期间关本地 IRQ

当前判断规则比较直接：

- 若当前有活跃的 `DisabledLocalIrqGuard`，则 `Task` 会提升成 `TaskIrqDisabled`，`BottomHalfL1` 会提升成 `BottomHalfL1IrqDisabled`
- 若当前持有的锁具有 `LocalIrqDisabled` 语义，或是 `WriteIrqDisabled` 且本次获取是 `write`，也会把上下文视为“IRQ disabled”

这里有一个对 `RwLock<WriteIrqDisabled>` 的专门建模：

- `write()` 仍然被视为 `WriteIrqDisabled`
- `read()` 会被降成 `PreemptDisabled`

这解决了“同一个 `RwLock` 的读路径不关 IRQ、写路径关 IRQ”这个 Asterinas 特有语义，否则 IRQ 相关报告会明显失真。

补充一点：当前文件原先只写了 `disable_irq().lock()`，但 `SpinLock` API 的真实语义还需要说清楚。

在当前 OSTD 实现里：

- 默认的 `SpinLock<T>` 实际上是 `SpinLock<T, PreemptDisabled>`
- 只有 `SpinLock<T, PreemptDisabled>` 提供 `disable_irq(&self) -> &SpinLock<T, LocalIrqDisabled>`
- 这个 `disable_irq()` 本身只是把锁的 guard behavior 从 `PreemptDisabled` 升级成 `LocalIrqDisabled` 的只读视图转换
- 它不会在调用点立刻创建 `DisabledLocalIrqGuard`
- 真正关闭本地 IRQ 的动作发生在后续 `lock()` / `try_lock()` 时，因为这时会通过 `LocalIrqDisabled::guard()` 调用 `disable_local()`

也就是说，对当前实现而言：

- `SpinLock::disable_irq()` 不是一个独立的“先关 IRQ，再做别的”的 primitive
- 它的效果体现在“这次加锁的 guard behavior 是 `LocalIrqDisabled`”
- `SpinLock<T, LocalIrqDisabled>` 这种静态声明方式，与 `SpinLock<T, PreemptDisabled>::disable_irq().lock()` 这种临时升级方式，在分析里都会落到同一类 `LocalIrqDisabled` 语义上

因此 lockdep 当前对这类 API 的建模重点不是“看见了一个名叫 `disable_irq` 的调用”，而是“这次 acquire 最终是否以 `LocalIrqDisabled` 的 guard behavior 发生”。

### 1.3 输出层面已经带有中断上下文信息

当前每个函数 artifact 里都已经带上：

- `contexts`
- `lock_events`
- `lock_edges`

其中事件和边都带 `context` 字段。也就是说，中断相关信息已经进入了事实层，而不只是终端上最后做一个字符串匹配。

## 2. 当前 IRQ 相关报告是怎么出来的

### 2.1 `single_lock_irq_violation` 是当前主报告

当前全局 IRQ 检查的主路径在 `tools/lockdep/src/main.rs` 中，大致分三步：

1. 先聚合每个函数 artifact 里的 `lock_usage_states`
2. 得到每个 lock mode 的全局 usage bits
3. 基于这些 bits 生成单锁和多锁两类 IRQ 违规

其中单锁规则对应 `find_single_lock_irq_violations`。

当前聚合的 usage bits 至少包括：

- `used_in_hardirq`
- `used_in_softirq`
- `used_with_hardirq_enabled`
- `used_with_hardirq_disabled`
- `used_with_softirq_enabled`
- `used_with_softirq_disabled`

当前单锁规则是：

- 同一 lock mode 既 `used_in_hardirq`，又 `used_with_hardirq_enabled`
- 同一 lock mode 既 `used_in_softirq`，又 `used_with_softirq_enabled`

也就是当前已经在做：

- `hardirq_safe_vs_unsafe`
- `softirq_safe_vs_unsafe`

### 2.2 `irq_dependency_violation` 是当前的多锁规则

当前也已经实现了基于锁序边的依赖规则，对应 `find_irq_dependency_violations`。

当前规则是：

- 若 `from` lock 被证明 `used_in_hardirq`，而 `to` lock 被证明 `used_with_hardirq_enabled`，则报 `hardirq_safe_to_unsafe`
- 若 `from` lock 被证明 `used_in_softirq`，而 `to` lock 被证明 `used_with_softirq_enabled`，则报 `softirq_safe_to_unsafe`

这已经不再只是“同一把锁在中断/可中断上下文都出现过”，而是会检查锁序边是否把 safe 锁导向 unsafe 锁。

### 2.3 `irq conflict` / `irq_reentry` 目前是兼容层

当前 `irq conflict` 已经不是主分析结果。
实现上它是由 `single_lock_irq_violation` 向后兼容折叠出来的摘要；
而 `irq_reentry` 仍然是把这个兼容摘要继续折叠成 AA 风格的 `A -> A` 展示。

换句话说：

- `single_lock_irq_violation` / `irq_dependency_violation` 是当前主规则输出
- `irq conflict` 是兼容旧视图的摘要
- `irq_reentry` 是把兼容摘要当成 `A -> A` 风险来展示

当前 `irq_reentry` 只会摘取一条 interrupt site 和一条 interruptible site 作为示例点，不会给出更完整的路径见证。

## 3. 当前已经覆盖到哪些中断场景

从实现和 fixture 测试看，当前原型已经覆盖了这些场景：

- top half 上下文识别
- L1 bottom half 上下文识别
- L2 bottom half 上下文识别
- `SpinLock<T, PreemptDisabled>::disable_irq()` 升级到 `LocalIrqDisabled` 后的 `lock()` / `try_lock()`
- 直接声明为 `SpinLock<T, LocalIrqDisabled>` 的加锁路径
- `RwLock<WriteIrqDisabled>` 的读写区分
- 每函数 `lock_usage_states`
- `single_lock_irq_violation`
- `irq_dependency_violation`
- 兼容层 `irq conflict`
- 由 IRQ 重入冲突导出的 `irq_reentry` AA 报告

对应 fixture 在：

- `tools/lockdep/tests/fixtures/ostd-lockdep-cases/src/lib.rs`
- `tools/lockdep/tests/fixture_cases.rs`

## 4. 现阶段的主要局限

如果按“对结论正确性和覆盖面的影响”排序，当前中断冲突分析相对 Linux lockdep 的主要局限可以概括为：

1. 还没有完整的 Linux lockdep 风格 IRQ 状态机
2. 上下文传播范围有限，尤其缺跨 crate 和更复杂调用形态的传播
3. IRQ 入口识别仍然偏硬编码，新的封装层容易漏掉
4. `DisabledLocalIrqGuard` 和 IRQ flag 的数据流跟踪还比较浅
5. 对更复杂的 IRQ 参与型死锁，仍主要依赖 usage bits 和普通锁图
6. 缺少 Linux 风格的 nested locking / subclass 机制，后续规则一加强就容易放大误报

建议修复顺序也按这个优先级推进：

1. 先把 IRQ 执行状态从“上下文标签 + 局部规则”升级成显式状态
2. 再补强跨函数/跨 crate/间接调用的上下文与状态传播
3. 再把 IRQ 入口识别和 helper 封装接成更声明式的机制
4. 再增强 `DisabledLocalIrqGuard` / IRQ flag 的数据流精度
5. 然后继续补更复杂的 IRQ 约束与报告解释
6. 最后用 nested/subclass 机制控制同类锁误报

### 4.1 还没有完整的 Linux lockdep 风格 IRQ 状态机

这一点是当前实现和目标设计之间最大的差距。

现在的原型已经为每个 lock mode 维护了一组 usage bits，并基于它们做单锁与依赖规则检查；但它仍然没有把 Linux lockdep 那种“执行状态机”和“锁状态派生”完整拆开。

也就是说，它还没有在 MIR 数据流里显式维护类似下面这样的结构化状态：

- 是否在 hardirq 中获取过
- 是否在 IRQ 开启的 task 上下文获取过
- 是否只在 IRQ 关闭时获取过

所以它目前做不到：

- 基于派生属性做更系统的 `hardirq-safe` / `irq-unsafe` 冲突判定
- 从“某条锁序边发生在可被 IRQ 打断路径上”进一步推导更复杂的 IRQ 环

目前实现更像：

- 先给事件贴上下文标签
- 再结合 guard behavior 推导 usage bits
- 最后按 usage bits 做单锁与依赖规则检查

### 4.2 上下文分类仍然比较硬编码

当前 IRQ 入口识别是硬编码的，只支持前面那三个入口 API。
虽然 `lockdep.toml` 已经能解析：

- `irq_entries`
- `ordered_helpers`

其中：

- `irq_entries` 已经影响 IRQ 入口识别；
- `ordered_helpers` 仍然只是解析、不影响分析。

另外，guard-returning helper 的锁返回现在已经通过 return-with-lock 建模接入分析，
不再依赖单独的 `wrappers` 配置。

这意味着：

- 新的 IRQ 注册封装还不会自动进入上下文分析
- 项目里如果再加一层 wrapper，当前原型大概率看不见

### 4.3 上下文传播范围仍然有限

当前传播虽然已经不再局限于最原始的“直接 `DefId` 调用边”，并且可以覆盖：

- local function alias
- callback wrapper
- nested callback wrapper
- returned guard through local wrapper

但总体上仍然只在 crate 内做本地摘要传播。
因此下面这些情况精度都有限：

- 跨 crate 调用链
- 更一般的间接调用
- 更复杂的回调保存和再调用
- 递归/SCC 上更丰富的上下文合流

这也是为什么现在的报告更适合被理解成“已观察到的最小证据”，而不是完整覆盖的证明。

### 4.4 `DisabledLocalIrqGuard` 跟踪仍然偏浅层

L1 bottom half 的 IRQ-disabled 状态目前是通过 `irq_guard_locals` 这个局部集合来表示的。
它已经能覆盖入口参数本身和简单的 local move，但还没有做更完整的 guard 流转建模。

因此如果未来代码里出现更复杂的 guard 搬运方式，例如：

- guard 被更复杂地 move 到其他 local
- 通过更复杂的别名链继续存活

当前 `BottomHalfL1IrqDisabled` 的识别可能会丢精度。

另外，CFG join 的 IRQ enablement 合流现在已经做成了对称 merge，不再依赖前驱访问顺序。

### 4.5 当前 IRQ 检查已经不只覆盖“同锁重入”，但 AA 兼容输出仍然偏向这一路径

当前主规则已经覆盖：

- 同一锁的 safe/unsafe 状态冲突
- safe -> unsafe 的依赖边违规

所以它已经不只是经典的：

- 任务持有 `A`
- 中断再次获取 `A`

但对于更复杂的 IRQ 参与型死锁，例如：

- 任务持有 `A` 再等 `B`
- 中断持有 `B` 再等 `A`

当前仍然主要依赖：

- usage bits 规则
- 普通锁图里的 `A -> B -> A` 环检测

而不是一个更完整的 IRQ 约束求解器。

### 4.6 报告粒度仍然偏“摘要”

当前 `irq conflict` 和 `irq_reentry` 都只提供少量 site。
它们能回答“为什么会报”，但还不能稳定回答：

- 完整调用链是什么
- 这个上下文是怎么一步步传播到目标函数的
- 是否存在多个不同来源共同造成同一冲突

## 5. 可以怎样评价当前阶段

如果把目标定成“当前 lockdep 是否已经开始分析中断相关死锁”，答案是肯定的，而且不是表面支持：

- 它已经有 IRQ 入口识别
- 已经有 crate 内上下文传播
- 已经有函数级/事件级/边级上下文标签
- 已经有 lock usage state
- 已经有 single-lock 和 dependency 两类 IRQ 规则
- 也保留了兼容层 `irq conflict` 和 `irq_reentry` 报告

但如果把目标定成“当前 lockdep 是否已经实现 lockdep 风格的中断安全分析”，答案是否定的。

更准确地说，当前状态应描述为：

- 已经完成了 IRQ 分析的骨架
- 已经能覆盖 Asterinas 中几类最关键的 IRQ 入口，以及 `SpinLock::disable_irq()` / `LocalIrqDisabled` 这类 guard behavior 语义
- 已经能覆盖 crate 内一部分 callback/wrapper 传播路径
- 但核心仍是启发式上下文分类和最小冲突检查
- 距离完整的 IRQ 安全属性推导、配置驱动入口扩展、以及更强的跨函数/跨 crate 精度，还有明显距离

## 6. 下一步增强方向：把 IRQ 检测继续做得更接近 Linux lockdep

注意：从这一节开始，内容主要是“后续增强方向”而不是“当前尚未开始”。
其中 usage state、single-lock rule、dependency rule、`irq_entries` 基础接线等内容已经有了第一版实现；下面保留的是它们继续演进时的建议方向。

这一节给出一个建议的落地顺序，目标不是一次性把所有能力做完，而是先把分析内核改造成可以承载 Linux lockdep 风格规则的形态，再逐步提高精度。

整体原则应当参考 Linux lockdep 的两层模型：

- 一层是 lock-class usage state
- 一层是 lock dependency rule

Linux 文档里明确把 IRQ 相关验证拆成这两部分：先记录“某把锁是否曾在 hardirq/softirq 中使用过，是否曾在这些状态开启时获取过”，再检查单锁状态冲突和多锁依赖冲突。当前 Asterinas 原型这两层都已经有了第一版，但执行状态跟踪、usage state 派生和报告解释性仍然比较粗糙，下一步应继续把它们拆得更清楚。来源：

- https://www.kernel.org/doc/html/latest/locking/lockdep-design.html
- https://www.kernel.org/doc/html/v5.14/translations/zh_CN/core-api/irq/irqflags-tracing.html

### 阶段 0：先重构现有实现的数据模型

目标：不要继续把 IRQ 分析逻辑散落在“事件标签 + 汇总时字符串分类”里，而是显式引入单独的 IRQ state 表示。

建议修改：

- 继续扩展 `tools/lockdep/analysis/src/model.rs` 中现有的 usage state artifact
- 在 `tools/lockdep/analysis/src/collect.rs` 中把“上下文/IRQ 标志事件”从隐式规则进一步提升成更显式的状态
- 在 `tools/lockdep/src/main.rs` 中继续弱化 legacy `irq_conflicts` 兼容层，把主路径彻底收敛到基于 state 的规则检查

建议先引入一个内部结构，例如：

- `LockUsageState`
- `ContextFrame`
- `IrqStateBits`

其中 `LockUsageState` 至少记录：

- ever in hardirq
- ever in softirq
- ever acquired with hardirq enabled
- ever acquired with softirq enabled
- 以上各项的 read/write 区分

在 Asterinas 语义里可以先把它映射成：

- `HardIrqTopHalf` / `BottomHalfL2` -> interrupt context
- `BottomHalfL1` -> softirq-like context
- `Task` -> task context
- `TaskIrqDisabled` / `BottomHalfL1IrqDisabled` -> 对应某些 “irq disabled” 观测位

这一阶段不追求立即改变用户可见报告，重点是先把后续规则需要的数据沉淀下来。

### 阶段 1：从“上下文标签”升级为“IRQ 标志状态跟踪”

目标：把“当前是否处于 IRQ context”和“当前 IRQ 是否开启”分开表示。

这是当前原型与 Linux 最核心的差距之一。现在的 `Task` / `TaskIrqDisabled` 等标签是把两个维度压扁了，而 Linux lockdep 实际上更接近同时跟踪：

- 当前上下文层级
- 当前 irq flags 是否开启

建议在 MIR 数据流状态里显式加入：

- `hardirq_context_depth`
- `softirq_context_depth`
- `hardirq_enabled`
- `softirq_enabled`

在 Asterinas 里的第一版近似可以是：

- `HardIrqTopHalf` 进入时：`hardirq_context_depth += 1`
- `BottomHalfL2` 进入时：按 hardirq-like 还是独立 L2 语义建模，需要先在设计里定死
- `BottomHalfL1` 进入时：`softirq_context_depth += 1`
- `DisabledLocalIrqGuard` 或 `LocalIrqDisabled`/`WriteIrqDisabled(write)`：把 `hardirq_enabled` 置为 false

这一步的关键收益是：后续不需要再靠 `is_interrupt_context()` / `is_interruptible_context()` 这种硬编码分类函数去猜，而是可以直接根据状态位推导：

- hardirq-safe
- hardirq-unsafe
- softirq-safe
- softirq-unsafe

### 阶段 2：实现单锁状态规则

目标：先把 Linux lockdep 的 single-lock state rules 落下来。

建议先实现这些检查：

1. 同一 lock class 既被标记为 hardirq-safe，又被标记为 hardirq-unsafe
2. 同一 lock class 既被标记为 softirq-safe，又被标记为 softirq-unsafe
3. 同一 lock class 出现同锁重入

这里的定义建议直接对齐 Linux 文档里的术语：

- hardirq-safe: 曾在 hardirq context 获取过
- hardirq-unsafe: 曾在 hardirq enabled 的上下文获取过
- softirq-safe: 曾在 softirq context 获取过
- softirq-unsafe: 曾在 softirq enabled 的上下文获取过

但要做两点 Asterinas 特化：

1. `RwLock<WriteIrqDisabled>` 仍然要保留读写区分
2. `BottomHalfL1` 与 `BottomHalfL2` 是否分别映射到 softirq/hardirq，需要先在设计文档里明确，不要一边实现一边临时决定

建议把当前 `irq conflict` 报告保留一段时间，但在内部把它降级成兼容层；新的主报告应改成：

- `single_lock_irq_violation`
- violation kind: `hardirq_safe_vs_unsafe` / `softirq_safe_vs_unsafe`

这样后续不会把“同锁状态冲突”和“多锁依赖冲突”混成一个概念。

### 阶段 3：实现基于 state 的多锁依赖规则

目标：补上比当前 `A` 同时出现在 interrupt/interruptionable context 更接近 Linux 的多锁规则。

建议新增这些检查：

- `<hardirq-safe> -> <hardirq-unsafe>`
- `<softirq-safe> -> <softirq-unsafe>`

也就是：

- 如果某个 `L1` 在 hardirq 中可获取，那么任何 `L1 -> L2` 都不允许把 `L2` 证明成 hardirq-unsafe
- softirq 也同理

这一步和当前实现最大的不同在于：

- 现在的原型只会检查“同一把锁是否跨上下文重复出现”
- 新规则会检查“锁序边本身是否把 safe 锁导向 unsafe 锁”

这样才能更接近 Linux 文档中的 dependency rule，而不只是 `irq_reentry(A)` 这一种形态。

建议报告结构新增：

- `irq_dependency_violations`
- witness edge
- source lock state
- target lock state
- origin function/context/location

### 阶段 4：把 IRQ 入口识别从硬编码升级为声明式机制

目标：让中断语义不再只依赖 `collect_entry_contexts()` 里的几个 API 名字。

建议分两步：

1. 先让 `lockdep.toml` 中的 `irq_entries` 真正生效
2. 再补 Rust attribute 版本，例如：
   - `#[lockdep::entry(context = "...")]`

建议配置格式不要只写一个字符串列表，而是写成带 context kind 的结构，例如：

- `function`
- `context`
- `irq_flags`
- `arg_roles`

这样才能描述：

- 哪个参数是回调
- 进入后默认属于 hardirq 还是 softirq-like 上下文
- 进入时 irq flags 是开还是关

如果只加“函数名列表”，很快又会回到当前这类硬编码限制。

### 阶段 5：补强 guard/IRQ flags 的数据流精度

目标：减少 `TaskIrqDisabled` / `BottomHalfL1IrqDisabled` 这类状态丢失。

建议增强这些点：

- `DisabledLocalIrqGuard` 的 move / reborrow / return 传播
- `disable_irq()` 形成的 guard 链条传播
- 显式 `drop()` 和作用域结束恢复 flags 的建模
- wrapper 函数中的 irq-disable / irq-enable 语义摘要

当前 `irq_guard_locals` 只是局部集合，精度不够。建议把“IRQ flags 来源”提升成与 lock guard 对称的一等对象，例如：

- `FlagGuardArtifact`
- `FlagEffect::DisableHardIrq`
- `FlagEffect::RestoreHardIrq`

这样才能在 MIR join 时更稳定地做合流，也更容易解释为什么某次 acquire 被视为 `irq enabled` 或 `irq disabled`。

### 阶段 6：补足跨函数摘要，但优先做上下文敏感摘要

目标：让 IRQ state 能跨调用边传播，而不是只传播“这个函数属于哪些 context”。

建议摘要从当前的“entry locks”升级到至少包含：

- entry state requirements
- exit state effects
- may acquire lock classes
- may return with locks held
- may disable/restore irq flags

这里最重要的是 context-sensitive summary，至少区分：

- 在 hardirq 下调用此函数
- 在 softirq/L1 下调用此函数
- 在 task 且 irq enabled 下调用此函数
- 在 task 且 irq disabled 下调用此函数

如果没有这层，后面所有基于 state 的规则都会被调用边打断，最后又退回到“只看函数本地”的水平。

### 阶段 7：引入 Linux 风格的 subclass / nested locking 机制

目标：减少“同类锁多实例”在 IRQ 与 AA 分析中的误报。

Linux lockdep 对 nested locking 的处理是把同一 lock-class 的不同嵌套层级视为子类。Asterinas 后续也应引入类似机制，否则同类对象的排序加锁会持续污染：

- self-lock 检测
- irq-safe/unsafe 状态
- safe -> unsafe 依赖边

建议落地顺序：

1. 先让 `ordered_helpers` 生效
2. 再引入显式 subclass 概念
3. 最后为“同类不同实例”提供可验证的排序键注解

否则 IRQ 规则一旦做强，误报也会同步放大。

### 阶段 8：改造报告，把“为什么判成 unsafe”解释清楚

目标：让输出不只是结论，而是可审计的推导链。

建议每条 IRQ 相关告警至少输出：

- 哪个 usage bit 首次被置位
- 哪个 usage bit 与之冲突
- 对应的 witness site
- 如果是依赖违规，再输出触发违规的 witness edge

也就是说，报告要能回答：

- 这把锁为什么被视为 hardirq-safe
- 它又为什么被视为 hardirq-unsafe
- 是哪条 `L1 -> L2` 使得 safe -> unsafe 规则被破坏

否则规则做得越像 Linux，用户越难复核。

### 阶段 9：补测试矩阵，按规则而不是按示例函数命名

目标：避免后续重构时把 IRQ 规则悄悄做坏。

建议把测试拆成三层：

1. 单锁状态测试
   - hardirq-safe vs hardirq-unsafe
   - softirq-safe vs softirq-unsafe
   - `RwLock<WriteIrqDisabled>` 读写差异

2. 多锁依赖测试
   - safe -> unsafe 违规
   - 普通 `A -> B -> A`
   - IRQ 参与的 `A -> B -> A`

3. 配置/注解测试
   - `irq_entries`
   - wrapper
   - ordered helper
   - nested subclass

测试断言也建议从“函数名里有没有某个上下文字符串”，升级成“某个 lock class 的 usage state bits 是否符合预期”。

## 7. 推荐实施顺序

如果要控制风险，建议按下面顺序分批落地：

1. 数据模型重构
   - 先引入 lock usage state 和 irq flag state
2. 单锁状态规则
   - 先做 hardirq/softirq safe vs unsafe
3. 多锁依赖规则
   - 再做 safe -> unsafe dependency
4. 配置驱动入口
   - 让 `irq_entries` 和 wrapper 生效
5. 上下文敏感摘要
   - 解决跨函数传播精度
6. nested/subclass
   - 控制同类锁误报
7. 报告与测试
   - 最后稳定用户可见输出

这个顺序的原因很直接：

- 没有 state model，就无法真正接近 Linux
- 没有单锁规则，多锁规则会缺解释基础
- 没有摘要精度，规则一加强就会误报爆炸
- 没有 nested/subclass，同类锁场景会持续污染结果

## 8. 一版可执行的近期里程碑

如果只看最近几轮提交，建议拆成下面 4 个里程碑：

### 里程碑 1：引入 usage state，但先不改输出

- 新增 lock-class usage state 数据结构
- 收集 hardirq/softirq context 与 enabled bits
- 保持现有 `irq conflict` 输出不变

这样可以先把底层事实收集稳定下来。

### 里程碑 2：把 `irq conflict` 替换成 single-lock rule 报告

- 新增 hardirq-safe/unsafe 与 softirq-safe/unsafe 检查
- 保留 `irq_reentry` 作为兼容 alias，一段时间后再移除

这样能先完成“从启发式冲突到规则化 state 检查”的迁移。

### 里程碑 3：实现 safe -> unsafe dependency 规则

- 在全局图构建后，基于 state 检查边违规
- 输出 witness edge 与相关 state

这一步完成后，IRQ 检测才算真正开始接近 Linux lockdep 的 dependency rule。

### 里程碑 4：启用 `irq_entries` 和 wrapper

- 让新入口和封装 API 可以声明式接入
- 补 fixture 和回归测试

这一步完成后，分析器才具备在 Asterinas 代码库中持续演进的条件，而不是每次改 API 都要改分析器源码。

## 9. 更细的实现拆解

这一节把上面的阶段进一步细化到“改哪些文件、加什么结构、先后顺序是什么”。

### 9.1 第一批改动：先把数据模型搭起来

这一批的目标不是立刻改变诊断结果，而是先把 IRQ 相关事实收集完整。

建议修改文件：

- `tools/lockdep/analysis/src/model.rs`
- `tools/lockdep/analysis/src/collect.rs`
- `tools/lockdep/src/main.rs`

建议在 `model.rs` 新增这些结构：

- `LockUsageStateArtifact`
- `LockUsageSiteArtifact`
- `IrqStateArtifact`

其中 `LockUsageStateArtifact` 建议至少包含这些字段：

- `class`
- `primitive`
- `read_usage`
- `write_usage`
- `sites`

其中 `read_usage` / `write_usage` 可以先共用同一个状态结构，例如：

- `used_in_hardirq`
- `used_in_softirq`
- `used_with_hardirq_enabled`
- `used_with_softirq_enabled`
- `used_with_hardirq_disabled`
- `used_with_softirq_disabled`

`sites` 用来保存 witness，建议不是只保留一个，而是每个 bit 保留第一个观测点：

- `first_hardirq_use`
- `first_softirq_use`
- `first_hardirq_enabled_use`
- `first_softirq_enabled_use`

在 `FunctionArtifact` 中建议新增：

- `irq_states: Vec<IrqStateArtifact>`
- `lock_usage_states: Vec<LockUsageStateArtifact>`

注意这里即使有些信息最终要全局聚合，也建议先按函数 artifact 输出，因为这样更容易调试和写测试。

### 9.2 第二批改动：把 MIR 状态从“标签”改成“结构化上下文”

当前 `collect.rs` 里的 `ContextKind` 仍然有价值，但它不应继续承担全部 IRQ 语义。

建议在 `collect.rs` 里新增一个内部状态结构，例如：

```rust
struct IrqExecutionState {
    base_context: ContextKind,
    hardirq_depth: u8,
    softirq_depth: u8,
    hardirq_enabled: bool,
    softirq_enabled: bool,
}
```

同时把当前 `BlockState` 扩成：

```rust
struct BlockState<'tcx> {
    aliases: BTreeMap<Local, Place<'tcx>>,
    guards: BTreeMap<Local, LockInfoArtifact>,
    irq_guard_locals: BTreeSet<Local>,
    irq_state: IrqExecutionState,
}
```

这里的迁移要分两步：

1. 保留 `current_context()`，但让它从 `irq_state` 推导字符串
2. 后续新规则全部只读 `irq_state`，不再依赖上下文字符串判断

这样做的原因是：

- 现有 JSON/测试不会立刻大改
- 新旧逻辑可以并存一段时间

### 9.3 第三批改动：给每次 acquire 生成 usage bit

建议在 `emit_effect()` 处理 `Acquire` 时，同时生成 usage state 更新。

可以新增一个帮助函数：

```rust
fn usage_bits_for_acquire(
    irq_state: &IrqExecutionState,
    lock: &LockInfoArtifact,
) -> LockUsageBits
```

这个函数的职责是：

- 判断这次获取是读还是写
- 判断当前是否处于 hardirq / softirq context
- 判断当前 hardirq / softirq 是否 enabled
- 产出应该被置位的 usage bits

然后新增一个聚合器：

```rust
fn record_lock_usage(
    usage_map: &mut BTreeMap<(String, String, String), LockUsageStateArtifact>,
    lock: &LockInfoArtifact,
    bits: LockUsageBits,
    site: LockUsageSiteArtifact,
)
```

这里的 key 建议至少包含：

- `class`
- `primitive`
- `acquire`

这样 `RwLock.read` 和 `RwLock.write` 可以先分开建模，后面再决定报告层是否合并显示。

### 9.4 第四批改动：把 IRQ 入口识别改造成“入口描述”

当前 `collect_entry_contexts()` 直接返回 `HashMap<DefId, Vec<ContextKind>>`，这对后续扩展不够。

建议改成先构建一个入口描述结构：

```rust
struct IrqEntryDescriptor {
    context: ContextKind,
    hardirq_enabled_on_entry: bool,
    softirq_enabled_on_entry: bool,
    callback_arg_index: usize,
}
```

然后把识别逻辑拆成两层：

1. `builtin_irq_entry_descriptor(callee_def_id) -> Option<IrqEntryDescriptor>`
2. `configured_irq_entry_descriptor(...) -> Option<IrqEntryDescriptor>`

最后在：

- 内建识别
- `lockdep.toml`

两路合并后，统一生成 entry state。

这样后续启用 `irq_entries` 配置时，不需要重写核心传播逻辑。

### 9.5 第五批改动：新增全局 state 聚合层

当前 `main.rs` 只聚合：

- lock nodes
- graph edges
- cycles
- irq_conflicts
- aa_deadlocks

建议新增一个中间层：

```rust
struct GlobalLockUsageState {
    by_lock_mode: BTreeMap<(String, String, String), AggregatedLockUsageState>,
}
```

聚合函数可以放在 `main.rs`，也可以单独拆到一个新模块，例如：

- `tools/lockdep/src/irq.rs`

如果准备持续增强 IRQ 规则，我更建议直接拆文件，否则 `main.rs` 会继续膨胀。

建议新增函数：

- `build_global_lock_usage_states(crates: &[AnalysisArtifact]) -> GlobalLockUsageState`
- `find_single_lock_irq_violations(...)`
- `find_irq_dependency_violations(...)`

然后让旧的：

- `find_irq_conflicts`

先保留一段时间，只作为兼容输出。

### 9.6 第六批改动：先落 single-lock rule

建议第一个真正替换现有逻辑的提交，只做 single-lock rule，不碰 dependency rule。

具体实现步骤：

1. 读取聚合后的 `AggregatedLockUsageState`
2. 对每个 lock mode 判断：
   - `used_in_hardirq && used_with_hardirq_enabled`
   - `used_in_softirq && used_with_softirq_enabled`
3. 生成新的 violation report

建议新增报告结构：

```rust
struct SingleLockIrqViolationReport {
    class: String,
    primitive: String,
    acquire: String,
    kind: String,
    safe_site: Option<LockUsageSiteArtifact>,
    unsafe_site: Option<LockUsageSiteArtifact>,
}
```

`kind` 初版可以固定为：

- `hardirq_safe_vs_unsafe`
- `softirq_safe_vs_unsafe`

这一批完成后，终端输出和 JSON 输出都应新增这一类报告，但先不要删除旧的 `irq_conflicts`，避免回归时难以对比。

### 9.7 第七批改动：再落 dependency rule

在 single-lock rule 稳定后，再开始依赖图规则。

建议做法是：

1. 先按 lock node 找到它对应的 aggregated usage state
2. 遍历 global graph 中每条 deadlock-relevant edge
3. 检查：
   - from lock 是否 `hardirq-safe`
   - to lock 是否 `hardirq-unsafe`
   - softirq 同理

新增报告结构建议如下：

```rust
struct IrqDependencyViolationReport {
    kind: String,
    from: LockInfoArtifact,
    to: LockInfoArtifact,
    from_safe_site: Option<LockUsageSiteArtifact>,
    to_unsafe_site: Option<LockUsageSiteArtifact>,
    witness_edge_origin: EdgeOrigin,
}
```

这里最关键的是 `witness_edge_origin`，因为 dependency violation 不像 single-lock violation 那样天然只有一把锁，必须明确是由哪条锁序边触发的。

### 9.8 第八批改动：让配置真正接入分析

当前 `LockdepConfig` 已经解析出：

- `irq_entries`
- `ordered_helpers`

其中 `irq_entries` 已经生效；`ordered_helpers` 仍然 inert。

后续建议聚焦两件事：

1. `irq_entries`
   - 继续扩展可配置的 IRQ 注册入口覆盖面

2. `ordered_helpers`
   - 接到 edge normalization / subclass 逻辑

不要把它们和 IRQ 规则重构混在一个提交里，否则出问题时不容易定位。

### 9.9 第九批改动：重做测试结构

现在 fixture 测试偏向“这个函数有没有某个上下文标签”。
后续如果要引入 usage state，建议把测试分成两层：

1. artifact 层测试
   - 某个函数是否产生正确的 `lock_usage_states`
   - 某个 acquire 是否带正确的 usage bits

2. global report 层测试
   - 是否产生正确的 `single_lock_irq_violations`
   - 是否产生正确的 `irq_dependency_violations`

建议新增至少这些 fixture 用例：

- task 中 `irq enabled` 获取，hardirq 中再次获取同锁
- L1 中获取某锁，top half/L2 中获取同锁
- `RwLock<WriteIrqDisabled>` 的 read/read 不报，read/write 报
- `A -> B`，其中 `A` hardirq-safe，`B` hardirq-unsafe
- helper 返回持锁 guard，再由调用者继续获取下一把锁

### 9.10 第十批改动：最后再删兼容层

当下面三件事都稳定之后，再考虑删除旧逻辑：

- `single_lock_irq_violations` 稳定
- `irq_dependency_violations` 稳定
- 新测试矩阵稳定

届时可以删除或降级：

- `find_irq_conflicts`
- `irq_reentry` 里基于旧 heuristic 的那部分逻辑
- `is_interrupt_context()` / `is_interruptible_context()` 这类字符串分类辅助函数

否则过早删除兼容层，会让中间迁移阶段很难验证“新规则到底比旧规则更准了，还是只是换了一套说法”。

## 10. 建议的提交切分

如果要开始实现，我建议按下面顺序提交，尽量保持每个提交都可测试、可回滚：

1. `Refactor lockdep IRQ analysis state model`
   - 引入 `IrqExecutionState`
   - 保持原输出不变

2. `Add lock usage state artifacts for IRQ analysis`
   - 输出每函数的 usage state
   - 新增基础 fixture

3. `Implement single-lock IRQ safety violations`
   - 新增 `single_lock_irq_violations`
   - 暂时保留 `irq_conflicts`

4. `Implement IRQ dependency violation checks`
   - 基于 global graph 检查 safe -> unsafe

5. `Expand configured IRQ entries`
   - 继续扩展 `lockdep.toml` 中 IRQ 入口点的覆盖面

6. `Add nested locking support for IRQ precision`
   - 处理 subclass / ordered helpers

7. `Remove legacy heuristic IRQ conflict checks`
   - 删除旧兼容逻辑

这个切分的好处是：

- 每一步都能用 fixture 回归
- 新旧报告能并存对比
- 一旦某一步误报明显增加，可以只回退那一步
