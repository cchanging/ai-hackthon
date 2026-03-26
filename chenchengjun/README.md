# Asterinas Verify Skill

### 项目概述

`asterinas-verify` 是一个面向 Asterinas 内核仓库的 Agentic Verification skill。它把“读 diff、拆问题、查语义、补证据、写报告、补测试、跑验证”这一整套高成本人工流程，整理成一条可复用、可并行、可追踪的验证流水线。

该项目的目标不是单纯帮用户“看代码”，而是让 Agent 能以更接近内核工程师的方式完成验证工作：先判断变更属于什么类型，再把复杂任务拆成多个 review unit，随后分别做语义分析、证据收集、回归测试设计和结果汇总，最后输出带有 Findings 的结构化结论。

### 解决的问题

`asterinas-verify`设想的一个使用场景是开发者在完成一个开发任务后，想要验证自己的改动是否存在 correctness 问题，或者想要评估一个即将合入的变更是否会引入 correctness 风险。传统的做法可能是自己读 diff、查文档、写&跑测试，还是需要挺多的effort的，而这个skill就是希望能够让AI来帮忙做这个验证的工作，降低人工成本，同时还能提供更结构化、更有证据链的结论。

实践

### 设计与实现

`asterinas-verify`的整个workflow如下:

- 用户指定需要验证的代码范围目标，例如一个 git range、一个 patch 文件、一个 syscall 或 module。
- Agent会先拆单元，再并行：将复杂变更拆成 review units，交给多个子Agents并发处理。
- 对于每一个验证的unit，Agent会先搜集Linux的相关对应实现，整理关于用户可见行为的spec，并对照相应代码改动，分析潜在问题。
- 对于找到的问题，Agent会先建立证据链，再下结论：通过 evidence ladder、behavioral spec pattern 和报告 schema 约束输出质量。
- Agent会先确认完整 bug 集，再集中验证：统一生成回归测试，最后通过 `asterinas-test` 这一个测试skill批量执行。

`asterinas-verify`不尝试解决代码风格问题且不对发现的bug进行尝试修复。

#### 1. 入口层

skill 支持以下入口形式：

- `verify change <git-range>`
- `verify patch <patch-file>`
- `verify files <path...>`
- `verify syscall <name-or-path>`
- `verify module <path...>`

这一层解决的是“用户怎么描述问题”的差异，把不同输入统一映射到验证流程。

#### 2. 预处理与拆分层

这一层由脚本负责将输入标准化并切分 review unit，核心包括：

- `prepare_review.py`
- `classify_review_unit.py`

它负责识别当前任务更适合走 syscall 验证、module 验证，还是 change 聚合验证，并在 change 模式下尽量把大任务拆成更小、更单一职责的 review units。

#### 3. 语义与证据层

这部分是 skill 的“判断核心”。它并不只看代码差异，还强调：

- 行为语义是否与 Linux 或既有 contract 保持一致；
- 结论是否有充足证据支撑；
- 用户态输入经过哪些边界条件后会触发不同路径；
- 不同类型问题应落到哪种 finding class。

这里依赖的核心参考资料包括：

- `references/evidence-ladder.md`
- `references/behavioral-spec-patterns.md`
- `references/report-schema.md`

它们共同定义了从“怀疑”到“确认”的证据路径，以及最后如何生成结构化 Findings。

这里额外强调一个实践原则：在检查正确性问题时，Agent 不应只围绕“正常输入”做推理，而应主动枚举用户可控输入的 corner case，例如：

- 空路径、空 buffer、零长度输入；
- 超长输入、边界长度、截断场景；
- 非法、部分有效或未对齐的用户指针；
- flag 组合、保留位和相互矛盾的参数；
- 部分成功、重试、重复调用与错误恢复路径。

这些 case 即使最后没有形成 finding，也应被记录到报告中，说明 reviewer 实际检查过哪些边界。

#### 4. 报告生成层

报告层的目标不是堆砌笔记，而是把验证结论整理成 reviewer 可消费的结果。核心脚本包括：

- `init_report.py`
- `render_findings.py`

生成结果强调：

- 顶层就出现 `Findings`；
- 按严重程度排序；
- 每条 finding 带文件锚点、证据 ID、分类和置信度；
- 详细证据与验证日志放在附录或后续章节。

#### 5. 验证执行层

验证执行层负责决定“测不测、测什么、怎么批量测”，核心包括：

- `select_test_family.py`
- `asterinas-test` skill

这使得 `asterinas-verify` 不会一发现疑点就立刻陷入碎片化执行，而是先收敛 bug 集，再统一安排 general tests 或 Linux-vs-Asterinas verification。

默认策略仍然是“先完成一轮审阅，再集中执行”。但对某些靠静态阅读难以确认的边界行为，skill 现在明确允许一种更强的闭环：

- 如果某个怀疑点集中在用户态输入 corner case；
- 如果可以写出一个有清晰 oracle 的 targeted test；
- 那么可以先生成该测试，用测试结果辅助确认该怀疑是否应该升级为 bug、unsupported 或 open-question。

这种测试属于“confirmation test”，它的作用是降低不确定性，而不是替代 evidence ladder 中的语义依据。

### Agents的分工合作

这个项目最核心的设计点，就是把 Agents 从“会聊天的助手”变成“分工明确的验证流水线参与者”。

#### 主 Agent 的职责

主 Agent 只负责高杠杆决策：

- 选择 verification profile；
- 初始化报告；
- 规划验证路径；
- 汇总子 Agent 的 findings；
- 做最终分类与结论输出。

这样主 Agent 不会被大段 diff、逐行找证据、手工拼表格这些高 token 成本工作拖垮。

#### 子 Agents 的职责

子 Agents 按 review unit 或任务类型并行工作，例如：

- Research Agent：负责 syscall 语义、模块 contract、上游行为证据的调研；
- Evidence Agent：负责提取精确的文件/行号引用，生成结构化 findings；
- Test Agent：为确认的 bug 编写回归测试；
- Report Agent：把结构化证据转换为报告中的 Findings 和附录内容。

这种模式的价值在于：

- 可以并行处理多个 review units；
- 每个 Agent 的上下文更小，输出更聚焦；
- 主 Agent 保持“编排者”角色，最终判断更稳定；
- 很适合复杂内核仓库中的大变更验证任务。

### 如何使用

如果你在 Codex / Agent 环境中使用这个 skill，可以直接以验证目标作为输入，例如：

```text
verify change HEAD~3..HEAD
verify syscall readlink
verify module kernel/comps/network
```

一个典型执行流程如下：

1. 用户给出待验证对象。
2. 主 Agent 选择 `change`、`syscall` 或 `module` profile。
3. 如果是复杂变更，先切分为多个 review units。
4. 多个子 Agents 并行收集证据、比对语义、提出 candidate regression tests。
5. 对仍不确定但可测的 corner case，先生成 targeted confirmation tests，并把结果作为补充证据。
6. 主 Agent 汇总后确认最终 bug 集。
7. 通过 `asterinas-test` 对目标集进行一次性批量验证。
8. 输出包含 `Findings`、用户态输入 corner cases、证据附录和验证日志的最终报告。
