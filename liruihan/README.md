# virtio-vsock

## 概述

Asterinas 之前已有 virtio-vsock 的实现，但其实现长久无人维护，且本身有很多正确性、性能、以及代码质量问题导致事实上也难以维护，此前我断断续续用两周时间想过一般需要完全重写的全新设计，这里想探索一下有没有可能利用 AI Agent 生成满足要求的高质量代码。

生成一个能跑的 PoC 或者 demo **不是** 目标，毕竟已经有的 virtio-vsock 也能 demo。

一个正确设计其实需要考虑很多内容，而且牵扯到很多细节：

- socket 状态机、连接状态机
- backend 连接管理（如后台关闭与超时销毁）
- virtio 硬件队列、socket 软件队列
- 基于 credit 的流量控制
- 发送与接受天然的反向性与锁顺序的矛盾
- 合理的系统设计表达 connection 的拥有者
- 软/硬中断与进程上下文的约束、任务分配的理性
- ……

所以这里有两个问题：

- 怎么才能告诉 Agent 一个已经在脑海中想好的设计？
- 怎么才能让 Agent 把这个设计转化成代码？

## Agent 使用

 - Day 1：迭代 plan，我提供 review
   - `00-guide.md`, `01-plan.md`, `02-review.md`, `03-plan.md`, `04-review.md`, ...
   - 结论：这个过程没有指出我已经成型的 design 中的任何问题，且效率远没有我直接把 design 写出来效率高

 - Day 2：迭代 code，我回答它不确定的问题，并通过另开 session 让它自己 review
   - “如果 plan 存在矛盾或其他实质问题导致实现不合理或困难，请及时终止并讨论可能的解决方案。”
   - 结论：效率很低，而且最终写出来的代码存在很多或大或小的问题，还得自己手动改

```
> find kernel/comps/virtio/src/device/vsock-codex | xargs cat | wc -l
863
> git show e6ff00b4a439d08be7230c56a73b2331ea6cb66e --stat
commit e6ff00b4a439d08be7230c56a73b2331ea6cb66e
Author: Ruihan Li <lrh2000@pku.edu.cn>
Date:   Wed Mar 25 16:41:15 2026 +0800

    Manually revise `virtio/src/device/vsock`

 kernel/comps/virtio/src/device/vsock/buffer.rs |  28 +------
 kernel/comps/virtio/src/device/vsock/config.rs |   4 +-
 kernel/comps/virtio/src/device/vsock/device.rs | 443 +++++++++++++++------------------------------------------------------------------------------------------
 kernel/comps/virtio/src/device/vsock/mod.rs    | 152 ++++++++++--------------------------
 kernel/comps/virtio/src/device/vsock/packet.rs |  67 ++++++++++++++++
 kernel/comps/virtio/src/device/vsock/queue.rs  | 209 ++++++++++++++++++++++++++++++++++++++++++++++++++
 6 files changed, 385 insertions(+), 518 deletions(-)
```
