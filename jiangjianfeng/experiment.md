# 实验记录

## 实验目标

- 作品名称：Asterinas 静态 Lockdep 分析原型
- 核心目标：为 Asterinas 实现一个基于 `rustc_driver + MIR` 的静态锁依赖分析工具，能检测普通锁序环、IRQ 相关死锁风险、AA/self-loop 风险，并逐步扩展到 `atomic mode` 违规检测
- 验收标准：
  - `tools/lockdep` 可以独立运行，并支持 `cargo osdk lockdep`
  - 能对真实的 `ostd` / `aster-kernel` 或依赖 `ostd` 的 crate 跑分析
  - 有 fixture 测试覆盖当前支持的核心场景
  - 关键误报能通过 review 迭代被解释、修复或明确标注为 limitation
  - 设计、进度和复盘文档能与实现现状对齐

## 交互记录来源

本次实验的真实 Agent 交互记录保存在 `artifacts/agent-sessions`，共 5 轮：

- `artifacts/agent-sessions/plan-and-impl/rollout-2026-03-24T03-40-03-019d1ded-96f8-7620-b9ab-12911a3244e1.jsonl`
- `artifacts/agent-sessions/review-round1/rollout-2026-03-24T08-33-19-019d1efa-1510-7b70-817b-5f42fcc7c595.jsonl`
- `artifacts/agent-sessions/review-round2/rollout-2026-03-24T08-33-25-019d1efa-2d93-7421-b572-9af6a1367ec1.jsonl`
- `artifacts/agent-sessions/review-round3/rollout-2026-03-25T02-11-54-019d22c3-3ec5-7cd3-b307-881c28c43bef.jsonl`
- `artifacts/agent-sessions/review-round4/rollout-2026-03-25T04-41-51-019d234c-8960-72e1-8b0e-fbaae84f9ea4.jsonl`

这些目录下同时保留了面向阅读的 `.md` / `.dialogue.md` 整理版，因此本实验记录优先基于这些整理结果，再用 JSONL 核对模型、工具和会话元信息。

提交仓库中同时附带了 `artifacts/lockdep` 源码快照，便于直接查看最终原型实现；但实验过程中的真实开发目录仍是 `/root/asterinas-codex/tools/lockdep`。

## 使用的模型和工具

- 模型提供方：`GPT 5.4`
- 主工作目录：`/root/asterinas-codex`
- 交互模式：单 Agent 持续开发，不是多 Agent 并行拆分

## Agent 交互方式

本次实验是“目标约束 + 实现 + 验证 + 复盘”循环。比较有效的交互方法有四类：

1. 先给设计文档，再要求 Agent 读代码后实施。  
   例如 `plan.md`、`lock_class.md`、`interrupts.md` 的需求都不是让 Agent 凭空发挥，而是要求它先对照文档和现有实现，确认哪些已经做了、哪些还没做，再继续补齐。

2. 让 Agent 按 phase 或 review 主题推进。  
   主实现阶段采用了非常明确的 phase 节奏：实现一阶段，提交一次，再继续下一阶段。这样能把每次对话限定在一个明确边界内，避免一次改太多。

3. 强制要求仓库级验证，而不是只看单元测试。  
   除了 fixture tests，用户多次要求 Agent 真正运行 `lockdep` 去分析 `ostd`、`aster-kernel` 以及所有依赖 `ostd` 的 crate，并要求它对每个告警给出源码级解释。

4. 明确限制 Agent 的越界空间。  
   典型例子是 review-round3 中用户明确要求“不要改动除了 lockdep 之外的代码”；这样 Agent 被迫把缺口收敛到分析器能力边界，而不是通过修改业务代码规避问题。

5. 实现完之后要求agent明确当前实现距离设计文档的gap。agent常常会在设计过程中偷懒，少实现一些feature,因此必须通过复盘的方式来明确实现的局限。

## 组织方式

- 组织模式：单 Agent 串行推进
- 人类职责：
  - 给出下一轮目标
  - 提供设计文档或 review 方向
  - 决定何时提交 commit
  - 判断哪些结论必须保守表述为 limitation
- Agent 职责：
  - 读取源码和文档，提炼当前状态
  - 在 `tools/lockdep` 内实施代码修改
  - 运行测试、真实扫描和格式化
  - 输出文档、报告和 commit

## 交互历史

### 1. 主实现阶段：plan-and-impl

- 会话特征：
  - 核心主题是按 phase 连续实现 `lockdep`

这一轮的交互方式很典型：先让 Agent 审查 `plan.md`，如果路线合理就直接开始实现。随后用户多次使用“先提交当前 phase，再进入下一个 phase”的方式驱动开发。

这一阶段完成了主线能力。

这一轮最重要的经验是：对编译器分析器来说，必须把“单 crate 采集”和“全局汇总”明确拆开，Agent 在第一轮就帮忙修正了这一点。另一个重要发现是：`ostd/kernel` 不能靠裸 `cargo check` 直接分析，必须显式处理目标架构和构建上下文。

### 2. review-round1：lock class 精度、全仓库扫描与误报修复

这一轮的起点不是“新增功能”，而是“继续 lock_class.md 中已经做了一半的修改”。用户的提问很有效，因为它把问题严格限定在一份设计文档和现有未完成实现之间。

这一轮的关键交互包括：

- 要求 Agent 补齐 `Global` / `ReceiverArg` / `FnArg` / `LocalOriginKey`
- 继续实现跨函数实参代入
- 增加 lock class 边界测试
- 对整个仓库中依赖 `ostd` 的 crate 跑分析
- 判断 `overlayfs` 告警是否为误报，并要求修复

其中最有价值的交互不是“写代码”，而是误报定位过程。用户先追问“是不是没有分析 `drop(upper_guard)`”，Agent 没有直接给拍脑袋答案，而是回到 `collect.rs` 和具体 MIR 形态，最终定位到“显式 `drop(guard)` 之前会有 guard move-transfer，旧状态机没有把 guard 从原 local 转移到临时 local”，这才是误报根因。

### 3. review-round2：IRQ 现状文档、实现计划与 `irq_entries` 接入

这一轮一开始用户没有直接要求写代码，而是先要求分析 IRQ 现状并写到 `interrupts.md`，并明确要求在新的 `git worktree` 中进行，避免碰撞主树未提交修改。

之后交互继续演化为：

- 先写 IRQ 现状文档
- 再基于文档生成实现计划
- 再细化成可执行步骤
- 最后才开始接入 `irq_entries`
- 接入后再补中断边界测试

这说明 Agent 在本实验中不仅被用于代码生成，也被用于“先写设计说明，再按设计回填实现”的文档驱动开发。

### 4. review-round3：全量依赖 `ostd` 扫描、`wrappers` 去除与重构

这一轮的任务密度最高，主要包含四件事：

1. 用 `tools/lockdep` 检测所有直接或间接依赖 `ostd` 的 crate。  
   Agent 通过 `cargo metadata` 计算反向依赖闭包，而不是手写包名单，避免漏掉间接依赖。

2. 检查 `aster-kernel` 注释里的 lock order 是否真的进入 lock graph。  
   在用户明确要求“不改 kernel 代码”的前提下，Agent 把缺口收敛到 `lockdep` 侧，并解释哪些属于当前分析器 limitation。

3. 解释 `lockdep.toml` 中 `wrappers` 的意义，并尝试移除它。  
   这里的交互非常有代表性：先通过实验对比“有 wrappers / 无 wrappers”的结果，证明配置不是理论必需，而是在补分析器缺口；之后用户要求把这个缺口直接做进分析器，于是 Agent 实现了 return-with-lock 传播，并删除了 `wrappers`。

4. 拆分过大的 `analysis/` 模块与测试文件。  
   用户最后要求做结构性重构，Agent 把 `collect.rs` 拆成了多个功能子模块，并把大测试拆成更清晰的独立测试。

其中最关键的交互模式是：用户不是问“能不能删 wrappers”，而是先逼 Agent 用真实结果说明它到底补了什么缺口，再要求把这个缺口做成分析器本身的能力。这种先证伪配置依赖、再推进实现的方式非常有效。

### 5. review-round4：新增 atomic mode 规则与仓库级 case review

这一轮聚焦新增规则：当持有 `SpinLock/RwLock` 时再获取 `Mutex/RwMutex`，应报告 `atomic mode` 违例。

这一轮的交互顺序非常清晰：

- 先让 Agent 判断规则放在收集期还是聚合期
- 再要求它补测试
- 再要求它对真实仓库扫描
- 最后要求它把所有依赖 `ostd` 的 crate 的 case 做源码级逐案分析，而不是只给统计数

这一轮关键成果是：

- 新增 `atomic_mode_violations`
- 对 `ostd + aster-kernel` 先做试跑
- 再扩展到 `22` 个依赖 `ostd` 的 crate
- 原始 `75` 条 case 被收敛成 `14` 组源码位置分析
- 报告写入 `tools/lockdep/atomic_mode_ostd_dependents_review.md`


## 遇到的问题

### 1. 误报定位不能只靠统计结果

- 原因分析：
  - 编译器分析工具的误报往往来自 MIR 细节、摘要传播或锁类合并策略
  - 只看 “cycle 数量减少了没有” 不足以定位根因
- 解决方式：
  - 强制 Agent 回到具体源码和 MIR 相关实现
  - 用定向 fixture 验证修复是否真正命中根因
  - `overlayfs` 的显式 `drop(guard)` 误报就是这样修掉的

### 2. 配置项容易长期停留在“已解析但未生效”

- 原因分析：
  - `lockdep.toml` 这类配置文件很容易先有格式，再拖很久才接进真实分析流程
- 解决方式：
  - 在 review 中专门要求 Agent 对照实现、文档和真实扫描结果，区分“已解析”“已统计”“已生效”
  - 然后针对 `irq_entries` 和 `wrappers` 分别推进：一个被真正接入实现，一个被实现替代后删除

## 对其他开发者有价值的经验

- 让 Agent 先读设计文档和当前实现，再动手改代码，效果明显好于“直接从一句目标开始生成”
- 对复杂静态分析器，最有效的驱动方式不是“大而全需求”，而是按 phase 和 review 轮次逐个压缩问题空间
- 让 Agent 在每个稳定节点提交 commit，很适合这种需要频繁回看回归和误报来源的项目
- 真实仓库扫描比 fixture 更重要，fixture 负责守回归，真实扫描负责暴露误报和适用边界
- 当用户明确要求“如果找不出来就承认 limitation”时，Agent 的输出质量会更高，因为它不能靠改业务代码绕开问题

