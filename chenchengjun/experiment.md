# Experiment Record

## 工具组织方式

### 1. 主 Agent 负责编排

按照 `README.md` 和 `SKILL.md` 的设计，主 Agent 的职责不是读完所有细节后亲自完成一切，而是负责：

- 选择 profile
- 初始化报告
- 规划验证路径
- 调用脚本
- 汇总 findings
- 决定哪些问题是 `bug`、`unsupported`、`contract-risk`、`regression-risk` 或 `open-question`

这是一种“编排者”角色，而不是“唯一执行者”角色。

### 2. skill 脚本负责固定 workflow

这个项目很明显地把稳定、重复、容易形式化的步骤固化到脚本里，例如：

- `prepare_review.py`
  负责把输入转成 manifest。
- `classify_review_unit.py`
  负责把大任务拆成 review units。
- `select_test_family.py`
  负责尽早判断该任务应走哪类验证路径。
- `init_report.py`
  负责报告骨架初始化。
- `render_findings.py`
  负责把结构化 findings 渲染为报告内容。
- `run_targets.py`
  负责 Linux / Asterinas 的统一测试执行。

这样做的核心作用，是把 workflow 从“靠模型记住流程”改成“靠脚本固定流程”。模型只需要做判断，不需要每次都从头设计流程。

### 3. 子 Agent 负责具体具体任务执行

从 `asterinas-verify/SKILL.md` 和 `README.md` 可以看出，整个 skill 的理想组织方式是：

- 一个主 Agent 负责调度。
- 多个子 Agent 按 review unit 并行工作。
- Research Agent 负责 Linux 语义或 contract 调研。
- Evidence Agent 负责行号、证据 ID、finding 结构化。
- Test Agent 负责补回归测试。
- Report Agent 负责整理最终报告。

## 遇到的问题

### 1. context 压力非常大

`session-example.jsonl` 中的 token 统计显示，这类验证任务很容易快速膨胀。样例会话结束时累计 token 使用量已经接近两百万级别。这说明：

- 即使任务只针对一个 syscall，读代码、写报告、补测试、跑验证也会持续消耗上下文。
- 如果不做任务拆分和结果落盘，模型性能会明显下降。
- `lessons.md` 里提到的“复杂任务一定要严格控制 context 用量”是非常真实的经验，而不是抽象原则。

### 2. 单次任务中容易混合多种工作

一个验证任务天然会混合以下内容：

- 语义查证
- 代码阅读
- 差异判断
- 报告撰写
- 测试编写
- 测试执行

如果不给 Agent 非常明确的边界，它会倾向于一边分析一边扩范围，甚至顺手做额外工作。这一点在 `lessons.md` 里也有总结，因此 skill 明确规定：

- 不要修 bug，只验证。
- 不要一发现一个问题就立刻跑一次测试。
- 先收敛完整 bug 集，再统一执行验证。

这些约束本质上是在防止 Agent 把任务做散。

### 3. 输出质量不稳定

大模型如果没有给他非常具体的限制，其输出的质量会很有随机性，所以该 skill 将很多固定的workflow封装在了脚本里，保证执行的逻辑正确，也把输出的格式固定下来，例如 Findings 的结构、证据 ID、文件锚点、分类和置信度等字段，这样就能让结果更一致，也更容易被后续流程消费。