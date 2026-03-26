## Lessons Learned

### 1. Bug 修复方面的挑战

**问题：架构破坏性修复**

在修复过程中，容易仅关注"让测试通过"，而忽视代码架构的完整性。典型问题：
- 在 VFS 层直接实现 Ext2 特定逻辑
- 使用 `downcast_ref` 在通用层访问具体文件系统
- 为单个测试添加特殊分支

**解决方案：架构审查自动化**

设置专门的 `kernel-architecture-audit` skill 检查合规性：
- 修复完成后自动 dispatch subagent 做架构校验
- 检查抽象边界、所有权边界、Linux 对齐性
- 拒绝补丁式修复，要求通过 trait/接口重构解决

**效果：**
- 避免了多次架构返工
- 保证了代码的长期可维护性
- 为未来集成其他文件系统（ext4、btrfs）奠定基础

### 2. 人与 Agent 的边界

**核心发现：并非所有任务都适合 Agent**

**人更高效的场景：**
- 架构设计决策（如选择 trait 设计方案）
- 复杂状态转换逻辑（如 partial block zeroing）
- 需要深度领域知识的判断

**Agent 更高效的场景：**
- 结构化分析（日志分析、根因诊断）
- 代码生成（基于明确的 spec）
- 验证和检查（架构审查、代码风格）

**平衡方案：Spec-Driven 开发**
- 人负责：设计决策、编写 spec（高层抽象）
- Agent 负责：基于 spec 生成代码、验证正确性
- 避免：让 Agent 直接实现复杂逻辑（容易跑偏）

**实践经验：**
- 直接让 Agent 实现 → 容易跑偏，需要多轮修正
- 纯 spec 驱动 → 效率较低，spec 编写耗时
- 最佳实践：人先设计框架 + Agent 填充细节

### 3. Context 管理策略

**问题：长任务中的 Context 退化**

在长时间对话中，自动 context compaction 会导致：
- 关键信息丢失
- Agent 重复询问已回答的问题
- 决策不一致

**解决方案：主动 Context 管理**

**策略 1：保存关键信息**
- 使用 auto-memory 持久化重要决策
- 记录已修复的 bug 模式
- 保存架构设计原则

**策略 2：新开 Session**
- 长任务完成一个阶段后，保存状态并新开 session
- 在新 session 中加载必要的 context
- 准确度明显优于持续 compaction

**实践数据：**
- 持续对话 50+ 轮：准确率下降至 ~70%
- 新开 session + 加载 context：准确率保持 ~85%

### 4. AI 擅长与不擅长的任务

**擅长的任务 (Good):**

**Deterministic I/O & Pure Logic**
- `get_blocks`: 块分配逻辑，规则明确
- `truncate_blocks`: 块截断逻辑，边界条件清晰
- 纯函数式逻辑，输入输出关系确定

**Verification Tasks**
- **Deadlock Checker**: 运行时分析，检测锁顺序问题
- **Bug Checker**: 将 xfstests 结果映射到根因
- **Spec Validator**: 验证实现是否符合规约

**不擅长的任务 (Bad):**

**Complex State Transitions**
- Partial block zeroing logic: 涉及多个状态、边界条件复杂
- 需要深度理解 PageCache 和块设备交互

**Data Race Detection**
- 受限于人类能力和测试用例
- 难以发现所有并发场景下的 race condition
- 需要形式化验证工具辅助

**Iterative Issues**
- 强行将补丁推入系统
- 倾向于"让测试通过"而非"正确设计"
- 需要架构审查约束

**Multi-Function Calls**
- 例如 `Ext2::sync()`: 需要同步 superblock、block group descriptors、inodes table、inode table desc、page cache
- 涉及多个子系统协调，容易遗漏步骤

**Code Style Issues**
- 逻辑正确但过于复杂
- 生成过多辅助函数
- 需要 `simplify` skill 后处理

**总结：**
- AI 适合确定性、可验证的任务
- 复杂状态转换、并发问题需要人工介入
- 通过 spec 和架构审查约束 AI 行为





