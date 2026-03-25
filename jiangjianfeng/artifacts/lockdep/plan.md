# Asterinas 静态 Lockdep 死锁检测工具计划

状态说明：

- 本文主要保留为设计/路线图文档，不再精确描述当前实现进度。
- 当前已经落地的能力包括：
  - 独立工具与 `cargo osdk lockdep`
  - 结构化 lock class
  - single-lock / dependency IRQ 报告
  - local callable alias、callback wrapper、nested wrapper、returned guard 的 crate 内传播
  - review 修复后的 release driver 构建、branch-dependent cycle 保留、CFG join IRQ merge
- 仍未落地的重要项包括：
  - cross-crate summary propagation
  - `ordered_helpers` 生效
  - Linux lockdep 风格完整 IRQ 状态机

## 1. 目标

目标是在 Asterinas 中实现一个基于 lockdep 思想的静态死锁检测工具，并把它做成 Cargo 插件形态，优先提供 `cargo osdk lockdep` 这一用户入口。

该工具的核心能力应包括：

- 静态提取锁获取顺序，构建 lock dependency graph。
- 通过图上的环检测潜在死锁。
- 显式分析中断相关死锁，而不只是普通线程间锁序反转。
- 贴合 Asterinas/OSTD 的同步原语和上下文语义，而不是做一个泛化但精度很差的 Rust 扫描器。
- 输出可读的诊断结果，包括环路路径、涉及的锁、涉及的函数、调用链和源码位置。

## 2. 为什么这件事在 Asterinas 里值得单独设计

Asterinas 不是普通应用层 Rust 工程，至少有以下特征会直接影响设计：

- `ostd` 的锁并不只是“锁”，它们还编码了上下文语义。`SpinLock<T, PreemptDisabled>`、`SpinLock<T, LocalIrqDisabled>`、`RwLock<T, WriteIrqDisabled>` 分别对应不同的原子上下文。
- `disable_irq().lock()` 是显式 API，而不是隐藏在某个运行时里。这对静态分析是好事，因为中断禁用语义可从类型和调用上直接恢复。
- OSTD 将 IRQ 分成 top half 和 bottom half，并区分 L1/L2 中断层级。`register_bottom_half_handler_l1` 与 `register_bottom_half_handler_l2` 的可重入性不同。
- 项目已有明确的锁序编码规范，且代码中已经存在“lock order”注释、“both task and interrupt contexts”注释和大量 `disable_irq().lock()` 调用，说明该工具能直接服务现有开发模式。
- 仓库里已经有 `cargo-component` 这类基于 `rustc_driver` 的编译期分析工具先例，因此不需要从零摸索 Cargo + rustc 集成路线。

## 3. 需求拆解

这个任务本质上分成四层：

1. 用户入口层  
   提供 Cargo 子命令，接受目标 crate、输出格式、忽略规则、严格级别等参数。

2. 编译器接入层  
   复用 rustc 的类型信息、方法解析、MIR 和 span，而不是只做文本级扫描。

3. 分析引擎层  
   静态提取“在什么上下文下，持有哪些锁，再去获取哪个锁”的事实，并做跨函数汇总。

4. 报告层  
   负责将 SCC/环路结果、IRQ 安全性冲突、来源路径、近似假设明确地展示出来。

## 4. 推荐总体架构

推荐采用“两层入口、一层分析内核 + 一层结果汇总”的结构：

- 用户入口优先做成 `cargo osdk lockdep`。
- 分析内核作为单独工具 crate 存放，避免把 `rustc_private` 直接耦合进当前 `osdk` 主 crate 的普通构建路径。
- `cargo osdk lockdep` 负责参数解析、准备环境、调用分析驱动。
- 分析驱动参考 `kernel/libs/comp-sys/cargo-component` 的模式，通过 `RUSTC_WORKSPACE_WRAPPER` 接入 rustc。
- 每次 rustc 调用只负责产出当前 crate 的静态事实文件；最终的全局 lock dependency graph 由前端在 Cargo 结束后统一汇总构建。

建议的目录布局：

```text
tools/lockdep/
├── Cargo.toml
├── build.rs
├── src/
│   ├── main.rs          # 前端，负责编排 cargo、收集工件、汇总结果
│   └── driver.rs        # rustc_driver 入口，只分析单个 crate
├── analysis/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── artifact.rs
│       ├── context.rs
│       ├── graph.rs
│       ├── summary.rs
│       ├── collect/
│       ├── report/
│       └── config.rs
└── tests/
```

推荐分两步落地：

- 第一步实现独立分析工具 `tools/lockdep`，先把静态分析闭环打通。
- 第二步在 `osdk/src/cli.rs` 增加 `lockdep` 子命令，把它包装成 `cargo osdk lockdep`。

这样做的原因是：

- `rustc_driver` 依赖会明显增加复杂度，最好先隔离。
- `cargo-component` 已经证明这种分层模式在本仓库内是可行的。
- 后续如果希望把分析器单独发布或单独测试，也更方便。
- `rustc_driver` 回调天然以“单次 crate 编译”为单位运行，必须显式加入工件汇总层，才能稳定支持跨 crate 的全局图分析。

## 5. 为什么必须基于 rustc/MIR，而不是简单 AST 或 grep

如果只做源码文本扫描，会很快撞上下面这些问题：

- `lock()`、`write()`、`read()` 是方法调用，必须先解析 receiver 的真实类型。
- `disable_irq().lock()` 的语义不是两个独立关键词，而是 guard 行为升级。
- 锁 guard 的生命周期由作用域、显式 `drop()`、临时值和 MIR drop 点共同决定。
- 中断入口点包含函数注册、闭包、函数指针和框架回调，不做编译期解析很难精确恢复。
- 跨 crate 的方法内联、泛型单态化和 trait 调用会严重影响锁序恢复。

因此，推荐分析层最少使用：

- HIR/类型系统：恢复方法调用和函数绑定关系。
- MIR：恢复 guard 生命周期、调用点、drop 点、控制流和基本块级锁状态。
- Span：输出精确诊断位置。

## 6. 锁依赖图的建模

### 6.1 节点类型

图中至少需要两类节点：

1. `LockClass`
2. `ContextClass`

`LockClass` 不是“某一次具体加锁实例”，而是 lockdep 语义上的“锁类”。推荐定义如下：

- 全局/静态锁：以 `DefId` 唯一标识。
- 结构体字段锁：以“接收者抽象路径 + 字段名 + 锁类型”标识。
- 局部分配锁：以“函数 `DefId` + MIR local/分配点 + 锁类型”标识。
- 泛型实例锁：必要时以单态化后的实例路径标识；如果成本过高，MVP 阶段可先退化到定义点级别。

`ContextClass` 用于编码中断相关约束，建议至少包含：

- `TaskIrqEnabled`
- `TaskIrqDisabled`
- `HardIrqTopHalfL1`
- `BottomHalfL1IrqEnabled`
- `BottomHalfL1IrqDisabled`
- `HardIrqTopHalfL2`
- `BottomHalfL2`

这里不要求最终图中都作为“普通锁节点”呈现给用户，但分析内核里必须有等价表示。

### 6.2 边类型

图中边至少需要区分三类：

1. 普通锁序边  
   若代码在持有 `A` 时获取 `B`，则加入 `A -> B`。

2. 上下文约束边  
   若某锁只能在 IRQ 关闭后安全获取，或只在 hardirq context 获取，则需要将“上下文”投影到图中。

3. 派生约束边  
   例如一个锁在 top half 中被获取，则该锁自动具有 hardirq-safe 属性；若同一个锁又在 task context 且 IRQ 开启时被获取，则这不是普通顺序环，而是 IRQ 自死锁风险。

### 6.3 节点附加属性

每个 `LockClass` 应至少记录：

- 锁原语种类：`SpinLock` / `RwLock` / `Mutex` / `RwMutex` / 其他包装。
- 获取模式：`lock` / `read` / `write` / `try_lock` / `upgrade`。
- guard 语义：`PreemptDisabled` / `LocalIrqDisabled` / `WriteIrqDisabled`。
- 是否出现在 top half。
- 是否出现在 L1 bottom half。
- 是否出现在 L2。
- 是否出现在 task context 且 IRQ 可能开启。
- 所有观测到的源码位置。

## 7. 中断死锁的建模原则

这是整个项目里最关键的一部分。只找普通锁环是不够的。

### 7.1 需要覆盖的 IRQ 相关死锁

至少要覆盖以下场景：

1. 同一把锁在中断上下文可获取，同时在 task context 且本地 IRQ 开启时也可获取。  
   这是经典“任务持锁后被中断，中断回调再次拿同一把锁”的自死锁。

2. 任务上下文持有 `A`，中断上下文获取 `B`，另一路径在持有 `B` 时获取 `A`。  
   这是中断参与的锁序环，不一定是同一把锁重入。

3. L1 bottom half 在本地 IRQ 重新开启后持锁，被 top half/L2 打断，形成嵌套死锁。

4. `WriteIrqDisabled` 这类“读不关 IRQ、写关 IRQ”的读写锁语义。  
   这要求检测器按读路径和写路径分别建模，不能把 `read()` 和 `write()` 混成一个“锁获取”。

### 7.2 推荐采用的 lockdep 风格规则

建议同时实现两套检查：

1. 图环检查  
   用于捕捉普通锁序反转和跨上下文锁序环。

2. IRQ 安全性分类检查  
   参考 Linux lockdep 的思想，为每个 `LockClass` 维护类似以下状态：
   - 是否在 hardirq context 获取过。
   - 是否在 task context 且 IRQ 开启时获取过。
   - 是否只在 IRQ 关闭状态获取过。

然后报告如下冲突：

- 一个锁既是 hardirq-safe，又是 irq-unsafe。
- 某条锁序边从“IRQ 可打断路径”通向“hardirq 可获取锁”，推导出潜在 IRQ 环。

这样做的原因是：

- 不是所有 IRQ 死锁都会在朴素 `A -> B -> A` 图里显式出现。
- 有些死锁本质上是“同锁在可中断和中断上下文双重出现”的上下文冲突。

## 8. 静态分析范围与精度策略

这类工具不可能一开始就完美。必须先定义 MVP 的精度边界。

### 8.1 MVP 应优先支持的模式

- 直接调用 `SpinLock::lock`、`RwLock::read`、`RwLock::write`。
- `SpinLock<..., PreemptDisabled>::disable_irq().lock()`。
- 明确的局部 guard 绑定：
  - `let guard = lock.lock();`
  - `let mut guard = self.foo.disable_irq().lock();`
  - `drop(guard);`
- 基于 MIR drop 的作用域结束释放。
- 直接函数调用和单态化后的普通泛型调用。
- IRQ 注册入口：
  - `IrqLine::on_active`
  - `register_bottom_half_handler_l1`
  - `register_bottom_half_handler_l2`

### 8.2 MVP 暂时降级处理的模式

- 复杂 trait object 动态分发。
- 通过多层包装函数间接获取锁，但没有清晰内联摘要。
- 宏大规模生成且 span 不稳定的锁包装。
- 非标准同步原语或第三方自定义包装。
- 依赖精确别名分析才能区分的锁实例。

这些场景不要“假装支持”，而应明确输出：

- `unknown`
- `conservative`
- `suppressed by config`

### 8.3 建议加入的配置/注解机制

为了让工具在内核代码里长期可用，建议一开始就预留以下机制：

- `lockdep.toml`
  - 忽略某些函数/模块
  - 注册自定义锁包装 API
  - 标注已知中断入口
  - 标注“多次获取同类锁”的排序键与规范顺序
  - 允许某些保守假阳性被静默

- Rust attribute
  - `#[lockdep::entry(context = "hardirq_l1")]`
  - `#[lockdep::ignore(reason = "...")]`
  - `#[lockdep::wrapper(acquire = "...", release = "...")]`
  - `#[lockdep::ordered_by(key = "...")]`
  - `#[lockdep::acquire_many(kind = "...", order = "...")]`

MVP 不一定要全部实现，但设计上要预留。

### 8.4 建议加入的“多次获取锁”注解

这是后续消除假阳性的关键机制，但当前阶段先写入计划，不立刻实现。

适用场景至少包括：

- 一个 helper 会根据某个全序键决定两把锁的获取顺序；
- 一个 helper 会对一组同类锁排序后批量加锁；
- 一个 helper 在 MIR 上看起来会产生 `A -> B` 和 `B -> A`，但实际上两条边来自互斥分支，且运行时顺序被排序逻辑规范化；
- 一个 helper 会多次获取“同一 lock class 的不同实例”，需要显式告诉分析器“这些实例按某个 key 排序”。

建议的 attribute 方向：

- `#[lockdep::ordered_by(key = "ino")]`
  - 标在 helper 或类型上，表示多次获取该类锁时应按 `ino` 递增顺序规范化。
- `#[lockdep::acquire_many(kind = "inode", order = "ascending_by(this.ino, other.ino)")]`
  - 标在 helper 上，表示函数内部的多锁获取是一个“规范顺序”的批量加锁操作。
- `#[lockdep::acquire_many(kind = "futex_bucket", order = "ascending_by(index_1, index_2)")]`
  - 适用于 `lock_bucket_pairs` 这类按 bucket index 排序的 helper。

建议的 `lockdep.toml` 方向：

- 当 Rust attribute 不方便加在目标代码上时，允许在配置文件中登记：
  - 某个函数是“ordered multi-lock helper”；
  - 它的锁实例来自哪些参数或局部路径；
  - 它的排序键是什么；
  - 分析器应如何把该函数内产生的双向边规范化为单向边。

设计目标：

- 不是“静默忽略”这些 helper；
- 而是让分析器在知道排序规则后，仍然保留锁序信息，只是把它规范化为运行时真实可能发生的顺序。

预期收益：

- 消除 `lock_bucket_pairs`、`read_lock_two_inodes`、`write_lock_two_inodes` 这类典型假阳性；
- 为后续批量加锁 API 提供稳定扩展点；
- 让“通过 annotation 提高精度”成为正式工作流，而不是 ad-hoc 特判。

## 9. 分析流水线设计

建议的分析流水线如下。

### 阶段 A：工作区与 crate 准备

- 解析 `cargo metadata` 或复用 OSDK 当前 crate 发现逻辑。
- 确定需要分析的目标 crate，默认至少覆盖 `ostd` 和 `kernel`。
- 读取 `rust-toolchain.toml`，确保驱动使用与仓库一致的 nightly。
- 为本次分析创建临时工件目录，约定每个 crate 在 `after_analysis` 结束后写入一个 `CrateFacts` JSON 文件。
- 区分两类编译入口：
  - 可直接用 `cargo check` 的普通 Rust crate；
  - 必须带目标三元组、feature 与 OSDK 环境的内核 crate（如 `ostd`、`kernel`）。
  第一版独立工具可以先跑通前者；后续要分析 `ostd/kernel`，前端必须升级为 target-aware / OSDK-aware。

### 阶段 B：编译器驱动接入

- 新建 `driver.rs`，参考 `cargo-component`：
  - 初始化 `rustc_driver`
  - 设置 `SYSROOT`
  - 在 `after_analysis` 中进入自定义分析
- 通过环境变量传递分析模式、输出路径和配置文件路径。
- 明确区分两层职责：
  - 前端只负责运行 `cargo check`、设置 `RUSTC_WORKSPACE_WRAPPER`、汇总工件；
  - 驱动只负责分析当前 crate 并落盘，不直接尝试构建全局图。

### 阶段 C：入口点收集

收集所有潜在执行上下文入口：

- 普通函数入口
- IRQ top half 回调
- L1 bottom half 回调
- L2 bottom half 回调
- 可能的工作队列/软中断桥接点

这一步输出的是“函数 -> 初始上下文状态”的映射。

### 阶段 D：函数内锁事实提取

对每个 MIR body 提取：

- 获取了哪些锁
- 获取时当前持有哪些锁
- 获取时的上下文状态
- guard 的创建点和释放点
- 调用了哪些可分析的函数

建议维护一个前向数据流状态：

- `held_locks: OrderedSet<LockClass>`
- `context: ContextState`
- `unknown_effect: bool`

每遇到加锁操作时：

- 记录当前所有 `held_locks -> new_lock` 的边
- 记录 `new_lock` 的上下文属性
- 将 `new_lock` 入栈

每遇到 drop 时：

- 将对应 lock class 出栈

### 阶段 E：跨函数摘要

为每个函数计算 `FunctionSummary`，至少包括：

- `requires_context`
- `acquire_events`
- `internal_edges`
- `may_call`
- `returns_with_locks` 是否允许
- `unknown_behavior`

然后做迭代汇总，直到摘要稳定。

MVP 阶段可以先只做“调用点内联式摘要传播”：

- 同 crate、可解析、非递归函数直接展开摘要
- 对递归 SCC 使用保守合并
- 对“ordered multi-lock helper”暂时不要直接折叠为普通双向边；
  在后续实现注解后，应在摘要层先做规范化，再进入全局图。

### 阶段 E.5：跨 crate 工件汇总

- 前端扫描工件目录，读取所有 `CrateFacts`。
- 先做 crate 内汇总，再做跨 crate 连接：
  - 同名 `DefPath` 不直接合并，必须携带 crate 身份。
  - 跨 crate 调用边在 MVP 阶段可以先只保留“调用到外部函数”的占位事实；
    等到后续支持更强的摘要传播后，再按 `DefId`/实例路径连接。
- 全局 lock dependency graph 的构建应发生在这一阶段之后，而不是 `after_analysis` 内部。

### 阶段 F：全局图构建与检查

构建全局图后做两类分析：

- Tarjan/Kosaraju 求 SCC，并从非平凡 SCC 中提取最短见证环。
- 基于 IRQ 上下文属性做冲突检查。

### 阶段 G：报告输出

推荐支持以下输出：

- 终端人类可读报告
- `--json`
- `--dot`

人类可读报告至少应包含：

- 环路摘要
- 环上的每条边来自哪个函数/调用点
- 锁类型和上下文类型
- 是否为 IRQ 自死锁、普通锁序环或不确定问题
- 近似来源说明

## 10. 关键技术点细化

### 10.1 如何识别“加锁”

MVP 先硬编码识别以下 API：

- `ostd::sync::SpinLock::lock`
- `ostd::sync::SpinLock::try_lock`
- `ostd::sync::RwLock::read`
- `ostd::sync::RwLock::write`
- `ostd::sync::Mutex::lock`
- `ostd::sync::RwMutex::{read,write}`
- `SpinLock<PreemptDisabled>::disable_irq`

注意：

- `disable_irq()` 本身不是“拿锁”，而是把后续锁获取的 guard 语义升级到 `LocalIrqDisabled`。
- `RwLock<WriteIrqDisabled>::read()` 与 `write()` 需要分开建模。

### 10.2 如何识别“锁释放”

推荐基于 MIR，而不是语法结构猜测：

- guard local 的 `StorageDead`
- 显式 `drop(guard)`
- 基本块结束的析构路径

必要时可以把“无法证明释放”的路径标记为保守持锁。

### 10.3 如何恢复锁类身份

优先级建议如下：

1. 静态项/全局项
2. `self.field` / `arg.field` 这类投影路径
3. 局部变量分配点
4. 退化到类型级锁类

目标不是一开始做到完美实例敏感，而是让“同一路径上的同一类锁”能稳定汇聚。

额外要求：

- 对局部 MIR place 不能直接使用裸 `(*_4)`、`_7` 这类字符串作为跨函数 lock class identity。
- 后续实现 annotation 时，需要能把“同类锁的不同实例”与“同一个局部临时值”区分开来；
  否则 `ordered_by(...)` 无法稳定作用于真正的锁实例集合。

### 10.4 如何识别 IRQ 上下文

应优先利用框架 API 识别，而不是靠注释：

- `IrqLine::on_active` 注册的闭包/函数，标为 `HardIrqTopHalfL1`。
- `register_bottom_half_handler_l1` 注册函数，标为 `BottomHalfL1IrqEnabled` 起步，但函数体中需显式跟踪 `DisabledLocalIrqGuard` 的持有、转移、`drop` 与重建。
- `register_bottom_half_handler_l2` 注册函数，标为 `BottomHalfL2`。
- 若函数参数包含 `DisabledLocalIrqGuard` 并在入口就有效，可将初始状态设为 IRQ 关闭。

### 10.5 如何处理 bottom half 里的 IRQ 重新开启

这是 Asterinas 特有重点：

- L1 bottom half 开始执行时，代码先 `disable_preempt()`，然后显式 `enable_local()`。
- 之后回调拿到一个 `DisabledLocalIrqGuard`，它可以选择暂时持有或转移 guard。
- 因此 L1 bottom half 不能简单粗暴地标成“全程 IRQ disabled”。

推荐做法：

- 初始上下文设为 `BottomHalfL1IrqEnabled`。
- 当 `DisabledLocalIrqGuard` 值处于 live 状态时，将当前路径标记为 IRQ 关闭。
- 当该 guard 被 `drop`、move 或超出作用域后，回退到 `BottomHalfL1IrqEnabled`。
- `disable_irq().lock()` 只是锁 guard 内含一个新的 IRQ-disabled 能力；不能把它和 L1 bottom half 入口参数混成同一个布尔标记。

这一步做对了，才能分辨“bottom half 中安全加锁”和“bottom half 打开 IRQ 后被 top half 重入”的差别。

## 11. MVP 建议范围

第一版不要试图覆盖所有同步原语。建议把 MVP 定义为：

- 只分析 `ostd::sync` 中现有锁原语。
- 只分析 `kernel/` 与 `ostd/`。
- 先支持直接方法调用，不做复杂动态分发。
- 能检测：
  - 普通锁序环
  - 同锁 IRQ 自死锁
  - 由 top half / L1 bottom half / L2 引起的典型嵌套环
- 提供文本报告和 JSON 报告。

如果能把这一步做好，已经有很高实用价值。

## 12. 分阶段实现步骤

### Phase 0：设计与样例固化

- 列出当前仓库中的真实锁模式样本：
  - `disable_irq().lock()`
  - `RwLock<WriteIrqDisabled>`
  - IRQ callback 注册
  - work queue / timer / device 路径
- 用 5 到 10 个真实样本定义预期检测结果。
- 同时准备 5 到 10 个最小化合成测试 crate。

交付物：

- 分析规则文档
- 测试样例表
- 误报/漏报预期边界

### Phase 1：工具骨架

- 新建 `tools/lockdep`。
- 建立 `main.rs + driver.rs + analysis/` 结构。
- 参考 `cargo-component` 跑通 `rustc_driver` 最小骨架。
- 能对指定 crate 遍历 `mir_keys` 并导出 `CrateFacts`。
- 前端能汇总多个 crate 的工件，并输出最基本的“分析了哪些 crate / 函数”摘要。
- 这一阶段不强求立刻支持 `ostd/kernel` 的真实构建参数；重点是把独立工具链路跑通。

交付物：

- `cargo run --manifest-path tools/lockdep/Cargo.toml -- ...` 可启动独立分析工具
- 基本 CLI 可用
- 工件落盘与汇总链路跑通

### Phase 1.5：目标感知的编译入口

- 为独立工具补充：
  - `--target`
  - `--features` / `--no-default-features`
  - 透传必要的 `RUSTFLAGS`
- 评估是直接调用 `cargo check --target ...`，还是在独立工具阶段就复用 `cargo osdk build/check` 的环境准备逻辑。
- 至少要做到能对 `ostd` 在一个受支持架构上完成编译并产出 `CrateFacts`。

### Phase 2：函数内锁采集

- 识别标准锁 API。
- 在单函数内恢复：
  - 锁获取
  - 锁释放
  - 当前持锁集合
  - 当前上下文状态
- 生成函数内依赖边。

交付物：

- 可输出“某函数内发现的锁序边”
- 单函数测试通过
- 至少有一个基于 `ostd` API 的独立测试 crate 覆盖：
  - 普通锁序边
  - `disable_irq().lock()`
  - `RwLock<WriteIrqDisabled>`

### Phase 3：IRQ 入口和上下文传播

- 识别 top half / L1 bottom half / L2 bottom half。
- 引入 `ContextState`。
- 支持 `disable_irq` 语义和 IRQ 开关状态传播。
- 加入 IRQ 安全性属性统计。

交付物：

- 可输出“某锁在何种上下文下被获取”
- 能报告最基础 IRQ 自死锁
- 至少有一个测试 crate 覆盖：
  - top half
  - L1 bottom half
  - L2 bottom half

### Phase 4：跨函数摘要与全局图

- 实现 `FunctionSummary`。
- 在可分析调用点上传播摘要。
- 构建全局 lock dependency graph。
- 实现 SCC 和见证环提取。

交付物：

- 能跨函数报告真实环路
- JSON/DOT 导出可用

### Phase 5：报告质量和配置能力

- 提升错误信息可读性。
- 加入 `lockdep.toml` 原型。
- 支持忽略规则、额外入口点和包装函数配置。
- 加入“ordered multi-lock helper”注解原型，但可以先只解析配置，不改变分析结果。
- 对“不确定”路径给出保守说明。
- 增加测试，验证配置文件至少能被读取并影响部分汇总结果。

当前状态备注：

- `ignore`、`irq_entries` 已经在当前实现中生效。
- guard-returning helper 的返回锁已经可以通过 return-with-lock 建模传播给调用者。
- `ordered_helpers` 仍然只解析、不影响分析结果。
- 配置格式说明文档已经单独整理为 `tools/lockdep/lockdep-toml.md`。

交付物：

- 可供开发者日常使用的终端输出
- 可供 CI 消费的 JSON 输出
- 配套的使用文档与代码文档，覆盖：
  - CLI 用法
  - 输出格式
  - 模块职责
  - 当前限制

### Phase 5.5：多次获取锁注解落地

- 实现 `#[lockdep::ordered_by(...)]` / `#[lockdep::acquire_many(...)]` 的最小子集，或等价的 `lockdep.toml` 配置。
- 在函数内和摘要层对被标注 helper 做顺序规范化。
- 将“有序双锁/批量锁”从“假阳性来源”转化为“有根据的单向锁序事实”。

交付物：

- 能消除 `lock_bucket_pairs`、`read_lock_two_inodes`、`write_lock_two_inodes` 这类典型假阳性；
- 文档中明确写出 annotation 语义、限制与示例。

### Phase 5.8：AA 型死锁检测

- 显式支持 `A -> A` 自环检测，而不是只检测 `A -> B -> A`。
- 至少覆盖两类 AA 场景：
  - 普通上下文里的同锁重复获取；
  - 中断参与的“task 持有 A，被中断后再次获取 A”。
- 在全局报告里区分：
  - 普通锁序环；
  - AA/self-loop；
  - IRQ 安全性冲突。

交付物：

- 能报告普通 self-loop；
- 能报告由中断上下文与可中断上下文共同触发的 AA 风险；
- 有基于 `ostd` API 的测试 fixture 覆盖这些场景。
- fixture 测试至少断言：
  - `cycle_count`
  - `irq_conflict_count`
  - `aa_deadlock_count`
  - `self_lock` 与 `irq_reentry` 两类 AA 报告都存在。

### Phase 6：接入 OSDK

- 在 `osdk/src/cli.rs` 增加 `Lockdep` 子命令。
- 统一参数风格，支持：
  - `cargo osdk lockdep`
  - `cargo osdk lockdep --json`
  - `cargo osdk lockdep --dot out.dot`
  - `cargo osdk lockdep --crate kernel`
- 让 `cargo osdk lockdep` 调用 `tools/lockdep` 分析驱动。

当前状态备注：

- 这一阶段已经有最小落地版本。
- 当前 `cargo osdk lockdep` 通过 shell-out 调用 `tools/lockdep`。
- 后续工作重点不再是“有没有入口”，而是“是否需要更深的 OSDK 原生集成”。

交付物：

- 面向仓库用户的正式入口

### Phase 7：CI 与回归集成

- 增加最小回归测试。
- 选择是否把 `cargo osdk lockdep` 加入 `make check`。
- 如果初期误报较多，可先作为非阻塞检查。

交付物：

- CI job
- 回归基线

## 13. 测试策略

测试必须分三层。

### 13.1 单元测试

用于验证：

- 锁 API 识别
- MIR 中 guard 生命周期恢复
- 上下文状态转移
- SCC 提取

### 13.2 合成 crate 测试

构造最小案例，覆盖：

- `A -> B` 与 `B -> A`
- 同锁 IRQ 重入
- `disable_irq().lock()` 安全路径
- `RwLock<WriteIrqDisabled>` 读写差异
- L1 bottom half 重新开启 IRQ 后被打断

### 13.3 仓库内回归测试

从 Asterinas 真实代码中抽取：

- 应报问题的样例
- 不应报问题的样例
- 以前出现过锁序问题的路径

建议优先选这些子系统做基线：

- `kernel/src/device/evdev`
- `kernel/src/thread/work_queue`
- `kernel/src/time/core/timer.rs`
- `ostd/src/irq`
- `ostd/src/sync`

## 14. 验收标准

第一阶段验收标准建议如下：

- 能通过一个命令分析 `kernel` 与 `ostd`。
- 能输出全局 lock dependency graph。
- 能对至少一组普通锁序环给出见证路径。
- 能对至少一组 IRQ 自死锁给出见证路径。
- 对 `disable_irq().lock()` 与 top/bottom half 的区别有明确处理。
- 对不支持模式不会静默漏掉，而会标记不确定性。

## 15. 主要风险与缓解方案

### 风险 1：锁实例识别不准，误把不同实例合并成同一把锁

缓解：

- 先做 lock class，而不是实例级唯一化。
- 对字段路径尽量保留接收者投影信息。
- 报告里标注“按锁类合并”。

### 风险 2：跨函数传播导致误报爆炸

缓解：

- MVP 只对可解析、非动态分发路径做摘要传播。
- 不可分析调用点直接标记 `unknown_behavior`。
- 先支持 `--focus crate/module`。

### 风险 3：IRQ 语义建模错误，导致误判

缓解：

- 先以 OSTD 框架 API 为准建模，而不是试图猜所有中断入口。
- 为 top half、L1 bottom half、L2 分别写专门测试。
- 把 `WriteIrqDisabled` 单独作为一类规则验证。

### 风险 4：和 nightly/rustc_private 强耦合，维护成本高

缓解：

- 将分析驱动隔离在单独 crate。
- 参考现有 `cargo-component` 维护模式。
- 尽量把项目专有逻辑放在独立 `analysis` crate，减少对 rustc API 细节的扩散。

## 16. 我建议的实际开发顺序

如果现在开始做，我建议按下面顺序推进，而不是一上来就追求“完整静态 lockdep”：

1. 先做工具骨架和 MIR 遍历。
2. 再做单函数内锁采集。
3. 然后把 IRQ 上下文建模做对。
4. 再做跨函数摘要和全局环检测。
5. 最后接入 `cargo osdk lockdep` 和配置系统。

原因很简单：

- 这件事真正的技术难点不是“找环”，而是“正确提取锁事实和 IRQ 语义”。
- 如果前两步不稳定，后面的全局图只会放大误差。

## 17. 结论

这个工具是可做的，而且与 Asterinas 当前代码形态高度匹配。最合适的路线不是写一个文本扫描器，而是基于 `rustc_driver + MIR + lockdep 风格上下文分类` 做一个静态分析器。

最推荐的落地方式是：

- 先在 `tools/lockdep` 中实现独立分析工具；
- 再把它包装为 `cargo osdk lockdep`；
- 第一版先覆盖 `ostd::sync`、IRQ top/bottom half 和典型锁序环；
- 在此基础上逐步加入配置、注解和更多同步原语支持。
