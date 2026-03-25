# Asterinas 静态 Lockdep 分析原型

## 作品概述

本作品围绕 Asterinas 的静态 `lockdep` 原型展开，目标是在编译期识别内核中的潜在死锁风险，而不是等到运行时再通过压力测试或故障复现去发现问题。

提交仓库中包含的源码快照位于：

- `artifacts/lockdep`

该原型已经实现为一个独立工具 crate。它面向 Asterinas/OSTD 的真实同步语义，重点分析：

- 普通锁序反转导致的死锁环(ABBA型死锁)
- 同一把锁的self-loop 风险（AA死锁）
- 中断相关的死锁：既包括“同一把锁同时出现在 IRQ 上下文和可中断的普通上下文中”导致的重入死锁，也包括“IRQ-safe 锁再依赖 IRQ-unsafe 锁”这类由锁序传播出来的中断参与型死锁

- 对 `asterinas`仓库 中全部 `24` 个直接或间接依赖 `ostd` 的 workspace crate 运行：

```bash
cargo run --manifest-path /root/asterinas-codex/tools/lockdep/Cargo.toml --bin cargo-lockdep -- \
  -p aster-bigtcp -p aster-block -p aster-cmdline -p aster-console -p aster-framebuffer -p aster-i8042 \
  -p aster-input -p aster-kernel -p aster-logger -p aster-mlsdisk -p aster-network -p aster-pci \
  -p aster-softirq -p aster-systree -p aster-time -p aster-uart -p aster-util -p aster-virtio \
  -p device-id -p osdk-frame-allocator -p osdk-heap-allocator -p osdk-test-kernel -p ostd -p xarray \
  --target x86_64-unknown-none -- --quiet
```

- 汇总结果：
  - 分析了 `24` 个 crate
  - 收集到 `25398` 个 MIR-backed function
  - 收集到 `3364` 个 lock event
  - 收集到 `1021` 个 lock edge
  - 检测到 `3` 个 potential lock cycle(2个假阳性)
  - 检测到 `75` 个 atomic-mode violation
  - 检测到 `0` 个 single-lock IRQ safety violation
  - 检测到 `0` 个 IRQ dependency violation
  - 检测到 `1` 个 potential AA/self-loop deadlock

## 解决的问题

Asterinas 不是普通用户态 Rust 项目，它的锁语义和上下文语义（中断/进程）高度耦合，导致通用静态扫描工具很难给出有价值的结果。这个作品主要解决以下问题：

1. 识别 Asterinas 特有的同步原语语义。  
   `SpinLock<T, PreemptDisabled>`、`SpinLock<T, LocalIrqDisabled>`、`RwLock<T, WriteIrqDisabled>` 不只是“锁类型不同”，还编码了不同的抢占和中断行为。

2. 在编译期恢复真实的持锁顺序。  
   仅做文本扫描无法正确处理 `lock()`、`read()`、`write()`、`disable_irq().lock()`、显式 `drop()`、作用域释放和 MIR drop 点。

3. 将中断上下文纳入死锁判断。  
   这个原型不仅找 `A -> B -> A` 这类普通环，也会追踪 top half、bottom half、task、IRQ disabled 等上下文，从而发现两类更接近内核真实问题的风险：一类是“同一把锁同时出现在 IRQ 上下文和 IRQ 仍开启的普通上下文里”的重入死锁风险；另一类是“IRQ-safe 锁再依赖 IRQ-unsafe 锁”这类由锁序传播出来的中断参与型死锁风险。

## 核心能力

当前原型已经具备以下能力：

- 基于 `rustc_driver` 和 MIR 提取每个函数内的 lock acquire/release 事件
- 构建函数内锁序边，并做 crate 内直接调用摘要传播
- 聚合 per-crate JSON 工件，构建全局 lock dependency graph
- 输出终端摘要、JSON 报告和 DOT 图
- 检测普通锁序环、AA/self-loop 风险、单锁 IRQ 冲突，以及 IRQ-safe 到 IRQ-unsafe 的依赖违规
- 支持 workspace 级 `lockdep.toml` 配置
- 提供 fixture 测试覆盖关键场景

这里的两类 IRQ 风险可以简单理解为：

- 单锁 IRQ 冲突：同一把锁既出现在中断上下文，又出现在仍可被中断打断的普通上下文；这对应当前报告里的 `single_lock_irq_violation`
- IRQ 依赖违规：某条锁序边的前驱锁已经被证明可在 IRQ 上下文获取，而后继锁却只在 IRQ-unsafe 条件下获取；这对应当前报告里的 `irq_dependency_violation`

当前重点支持的同步原语和语义包括：

- `SpinLock`
- `RwLock`
- `Mutex`
- `RwMutex`

当前已识别的执行上下文包括：

- `Task`
- `TaskIrqDisabled`
- `HardIrqTopHalf`
- `BottomHalfL1`
- `BottomHalfL1IrqDisabled`
- `BottomHalfL2`

## 架构设计

整体上，这是一个“两段式采集 + 一段式聚合”的静态分析工具：

```text
Cargo命令
        |
        v
src/main.rs
前端 CLI，拉起 cargo check，设置 RUSTC_WORKSPACE_WRAPPER
        |
        v
src/driver.rs
rustc_driver 包装器，在每个 crate 的 after_analysis 阶段采集 MIR 事实
        |
        v
analysis/
提取函数级 lock events / lock edges / contexts / usage states
        |
        v
JSON artifacts
每个 crate 一个分析工件
        |
        v
src/main.rs
汇总全局图，输出 cycle / IRQ / AA 报告，以及 JSON / DOT
```

核心模块如下：

1. 前端驱动：`artifacts/lockdep/src/main.rs`  
   负责解析 CLI 参数、调用 Cargo、收集各 crate 工件、聚合全局锁图，并生成终端摘要、JSON 和 DOT 输出。

2. 编译器包装器：`artifacts/lockdep/src/driver.rs`  
   通过 `RUSTC_WORKSPACE_WRAPPER` 接入 rustc，在 `after_analysis` 阶段触发分析逻辑，并将单 crate 结果写入工件目录。

3. 分析库入口：`artifacts/lockdep/analysis/src/lib.rs`  
   封装工件读写、统一分析模型导出，并连接采集逻辑与前端聚合逻辑。

4. MIR 事实提取：`artifacts/lockdep/analysis/src/collect.rs`  
   负责识别锁获取/释放、恢复函数内锁序、传播 crate 内直接调用上下文，并收集 IRQ 相关 usage bits。

5. 工件模型：`artifacts/lockdep/analysis/src/model.rs`  
   定义 `AnalysisArtifact`、`FunctionArtifact`、`LockInfoArtifact`、`LockUsageStateArtifact` 等可序列化结构，支撑后续汇总和报告输出。

6. 回归测试：`artifacts/lockdep/tests/fixture_cases.rs`  
   配合 `artifacts/lockdep/tests/fixtures/ostd-lockdep-cases/src/lib.rs` 验证锁序环、IRQ 场景、`disable_irq().lock()`、`RwLock<WriteIrqDisabled>`、AA 死锁等核心行为。

## 可复现方式

查看帮助：

```bash
cargo run --manifest-path /root/asterinas-codex/tools/lockdep/Cargo.toml -- --help
```

运行 fixture 测试：

```bash
cargo test --manifest-path /root/asterinas-codex/tools/lockdep/Cargo.toml --test fixture_cases
```

直接分析一个包：

```bash
cargo run --manifest-path /root/asterinas-codex/tools/lockdep/Cargo.toml -- \
  -p ostd --target x86_64-unknown-none -- --quiet
```

导出 JSON：

```bash
cargo run --manifest-path /root/asterinas-codex/tools/lockdep/Cargo.toml -- \
  -p aster-kernel --target x86_64-unknown-none \
  --emit-json lockdep.json -- --quiet
```

导出 DOT：

```bash
cargo run --manifest-path /root/asterinas-codex/tools/lockdep/Cargo.toml -- \
  -p aster-kernel --target x86_64-unknown-none \
  --emit-dot lockdep.dot -- --quiet
```

## 如何利用 Agents

这个项目里的 Agent 使用不是泛泛“让它帮忙写代码”，而是贯穿了整个 `lockdep` 原型的分阶段开发。真实交互记录保存在 `artifacts/agent-sessions/`，共 5 轮：

1. `plan-and-impl/`  
   先让 Agent 审查 `plan.md`，确认总体路线可行，再要求它按 phase 连续实现，并在每个稳定节点提交 commit。这个阶段完成了工具骨架、MIR 采集、跨函数摘要、全局环检测、DOT/JSON 输出、IRQ/AA 分析、配置与测试框架等主线能力。

2. `review-round1/`  
   这一轮不是重写功能，而是让 Agent 根据 `lock_class.md` 补齐 `lock class` 精度，随后要求它用真实仓库跑分析、判断告警真假，并继续修掉 `overlayfs` 的显式 `drop(guard)` 误报。

3. `review-round2/`  
   这一轮聚焦 IRQ 分析。用户先要求 Agent 在单独 `git worktree` 中写 `interrupts.md`，避免污染脏工作区；随后再让它按文档计划把 `irq_entries` 配置真正接入分析流程，并补充中断边界测试，最后再把变更 cherry-pick 回主树。

4. `review-round3/`  
   这一轮让 Agent 用 `tools/lockdep` 扫描所有依赖 `ostd` 的 crate，检查注释里声明的 lock order 是否真的进图，并在“不要改 kernel 代码”的约束下，把缺口收敛到 `lockdep` 自身能力边界。接着又继续要求 Agent 去掉 `wrappers` 配置，通过实现 return-with-lock 传播来补齐这部分能力，最后做代码重构和测试拆分。

5. `review-round4/`  
   这一轮新增了 `atomic mode` 违规检测：在持有 `SpinLock/RwLock` 时获取 `Mutex/RwMutex`。Agent 不仅实现了新规则和测试，还被要求对所有依赖 `ostd` 的 crate 做仓库级扫描，并把 75 条原始 case 收敛成 14 组源码级分析，写入 `artifacts/lockdep/atomic_mode_ostd_dependents_review.md`。

从这些交互可以看出，这个项目里 Agent 的典型用法是：

- 先读设计文档和现有代码，再动手实现，而不是直接生成大块补丁
- 按 phase 或 review 目标拆任务，每一轮只解决一个明确问题
- 每次实现后都立即跑 `cargo test`、`cargo-lockdep` 或真实仓库扫描做验证
- 当结果涉及误报/漏报判断时，要求 Agent 回到源码逐案解释，而不是只给统计数字
- 当工作区有并行修改时，用 `git worktree` 隔离文档或实验分支，避免相互污染

这个流程里，Agent 负责加速“读代码、改实现、跑验证、写报告”，而开发者负责给出下一轮目标、设定不允许跨越的边界，并对误报、真问题和后续取舍做最终判断。

## 当前边界与不足

该作品已经是POC，但还存在很多限制。当前主要限制包括：

- IRQ 安全性分析并不完备，以启发式的发现为主，并不是一个完备的的
- 上下文传播目前主要限于 crate 内 direct call
- `lockdep.toml` 只是已解析、未真正参与分析
- 复杂的间接调用、trait object、跨 crate 精确摘要传播仍然不足
- 锁类身份虽然已较早期版本稳定很多，但还不是完全实例敏感


## 项目结构

```text
.
├── README.md
├── artifacts/
│   ├── agent-sessions/
│   ├── hunted-bugs/
│   └── lockdep/
├── experiment.md
└── lessons.md
```

其中：

- `README.md`：作品概览、问题定义、架构与复现方式
- `artifacts/`：可复现材料目录；当前包含 `lockdep/` 源码快照、`agent-sessions/` 交互记录，以及 `hunted-bugs/` 仓库级扫描分析材料
- `experiment.md`：记录与 Agent 协作的方法、命令、模型和迭代过程
- `lessons.md`：沉淀最值得复用的经验
