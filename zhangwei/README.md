# 基于 OpenGL 基线推进 Vulkan 使能：Asterinas virtio-gpu 对 Mesa 3D 的兼容性验证

## 作品概述

本项目是一个面向 Asterinas 图形子系统兼容性完善的内核研发选题，聚焦 virtio-gpu 前端与 Mesa 3D 用户态之间的契约验证。比赛开始前，Asterinas 已经具备一定的 OpenGL 可用性；本次工作的目标是在这一基线上继续推进 Vulkan 用户态路径，并把探索过程整理为一份可解释、可验证、可复用的技术成果。

这项工作的重点不是“让某个图形 demo 简单跑起来”，而是围绕 guest 内核为 Mesa 图形栈提供的关键接口契约，系统检查哪些能力已经具备、哪些兼容性缺口仍然存在、以及如何通过小步修复和证据驱动的方式逐步把 Vulkan bring-up 推进下去。

## 解决的问题

在比赛开始前，Asterinas 的 virtio-gpu 实现已经能够支撑一部分 OpenGL 场景，例如：

- `kmscube`
- `xfce`
- `glxgears`

但这些结果还不能说明 virtio-gpu 前端已经具备较完整的 Mesa 3D 兼容性。相比 OpenGL 路径，Vulkan 对 guest 内核侧的要求更复杂，尤其涉及：

- `GETPARAM` / `GET_CAPS` 等能力协商接口；
- blob 资源创建与 host-visible 映射；
- `CONTEXT_INIT` 及其相关上下文初始化流程；
- `EXECBUFFER`、`syncobj`、fence 等同步与命令提交流程。

因此，本项目要解决的核心问题是：在不破坏既有 OpenGL 可用性的前提下，推进 Asterinas virtio-gpu 对 Vulkan 用户态的兼容性，并对 Mesa VirGL 与 Mesa Venus 依赖的共享 guest-kernel 契约做一次更系统的验证。

## 项目目标

本项目的目标可以概括为四点：

1. 保持现有 OpenGL 路径可用，避免 Vulkan 相关修改引入回归。
2. 推进 Vulkan 用户态初始化，使 `vulkaninfo`、`vkcube` 成为明确的验证目标。
3. 检查并补齐 Mesa VirGL 与 Mesa Venus 共用的 virtio-gpu / DRM 接口契约。
4. 总结一套在内核图形栈 bring-up 与调试中使用 Coding Agents 的可复用方法。

## 架构设计

本项目涉及的整体链路可以概括为四层：

1. 应用层：`kmscube`、`glxgears`、`vulkaninfo`、`vkcube`
2. Mesa 用户态层：VirGL 路径服务 OpenGL，Venus 路径服务 Vulkan
3. Asterinas guest 内核层：DRM core、virtio-gpu frontend，以及相关 ioctl / mmap / sync 支持
4. virtio-gpu 设备与宿主机图形栈

虽然 OpenGL 和 Vulkan 是两条不同的 API 路径，但它们在 guest 内核侧依赖了大量共享能力。因此，这个项目的本质并不是单点功能开发，而是一次围绕 virtio-gpu guest 契约的兼容性梳理、修复与验证。

## 主要工作

### 1. 推进 Vulkan bring-up

围绕 Vulkan 用户态路径，比赛期间重点推进了以下工作：

- 以 `vulkaninfo` 和 `vkcube` 作为阶段性验证目标；
- 检查并修正 `DrmSyncObject` 相关 ioctl 的实现问题；
- 补充 blob 资源与 host-visible mapping 相关处理；
- 结合日志、代码阅读与用户态调用路径，排查同步与命令提交流程中的兼容性问题。

其中，一个关键问题是此前加入的 12 个 `DrmSyncObject` ioctl 命令存在实现不正确的情况。为此，先围绕 `DrmSyncObject`、`dma_fence` 以及 Vulkan 所依赖的同步语义做了一轮系统梳理，再逐项筛查 ioctl 行为，并结合低层测试和调试信息逐步修复问题。

### 2. 做 OpenGL 非回归验证

由于 Vulkan 使能建立在既有 OpenGL 基线上，因此本项目并没有把目标定义为“从零支持图形”，而是强调双 API 路径的兼容性验证。比赛期间持续把以下程序作为 OpenGL 侧的非回归证据：

- `kmscube`
- `xfce`
- `glxgears`

这部分工作的作用，是证明 Vulkan 相关修改没有破坏原有 VirGL 路径。

### 3. 整理 patch 集并降低评审成本

除功能修复之外，本项目还包含一轮面向评审可读性的 patch 整理工作。相关代码在比赛前后累计已经形成较大的 patch 集，原始提交数量超过 50 个，总修改规模超过 7000 行。比赛期间借助 Agent 协助整理，在不改变主要功能边界的前提下，将 patch 集压缩为 5 个更便于 review 的提交。

## 当前结果与交付物

本项目当前形成的结果主要包括：

- 一条已经存在的 OpenGL 可运行基线；
- 一组围绕 Vulkan bring-up 的兼容性修复与验证工作的代码；
- 一份记录 Agent 使用过程的 `experiment.md`；
- 一份总结可迁移经验的 `lessons.md`；
- 一个demo 视频

就当前 README 对应的结论而言，本项目应理解为一次“面向 Mesa 3D 的 virtio-gpu 兼容性验证与推进”，而不是对单个 demo 成败的孤立描述。即使某些 Vulkan 目标仍在持续收敛，这项工作的技术价值依然体现在：

- 明确了 guest 内核侧需要满足的关键接口契约；
- 找出了若干与 Linux / Mesa 预期不一致的兼容性缺口；
- 通过低层检查与端到端验证逐步缩小问题范围；
- 沉淀了更适合星绽内核研发场景的 Agent 协作方式。

## 验证方式

本项目采用两类验证方式：

- 底层验证：围绕 ioctl、blob、context、同步语义等关键路径做针对性检查；
- 端到端验证：使用 `kmscube`、`xfce`、`glxgears`、`vulkaninfo`、`vkcube` 观察用户可见行为。

这样的验证方式有两个目的：一方面避免只看 demo 结果而忽略底层语义错误，另一方面也避免只修低层接口却缺少用户态可见证据。

## 如何利用 Coding Agents

根据比赛要求，README 需要说明作品是如何利用 Agents 完成的。本项目中，Coding Agents 主要用于以下几类工作：

### 1. 补齐底层知识并收敛问题模型

在处理 `DrmSyncObject`、`dma_fence` 等底层机制时，先让 Agent 协助阅读和整理背景知识，帮助开发者建立更清晰的问题模型，再由人工确认关键假设、边界和后续验证方式。

### 2. 加速拆小后的接口问题修复

当问题已经被收敛到单个 ioctl、单条同步路径或单类 capability 行为后，再让 Agent 逐项分析实现、生成候选修复、补测试草稿或补调试代码。这种“小步迭代”的协作方式比直接要求 Agent 解决整个 Vulkan bring-up 更有效，也更容易人工审核。

### 3. 支持“补日志 -> 复现 -> 回传 -> 继续修复”的调试闭环

对于可复现的图形栈问题，先让 Agent 帮助在内核或用户态关键路径上补日志，再将日志返回给 Agent 继续缩小范围、生成下一轮修复建议。实践中，这种方法明显优于无证据的直接猜测。

### 4. 协助整理 patch 和文档

除代码修复外，Agent 还被用于辅助整理 patch 集、压缩提交组织、重写说明文档，从而降低人工整理成本，提高最终提交材料的可读性。

## 经验与边界

本项目的一个直接经验是：在图形栈与内核同步语义这类复杂问题上，Agent 的最大价值通常不是“直接给出最终答案”，而是帮助开发者更快建立知识框架、补齐证据、拆小问题并推进迭代。

相应地，这个项目也坚持一个明确边界：任何由 Agent 参与生成的代码、结论或解释，都必须经过人工验证。尤其是 ioctl 语义、同步行为和端到端图形结果，不能用模型输出直接替代真实验证。这一点也与比赛要求一致，即“自我验证、自我负责”。
