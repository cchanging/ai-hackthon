# Lock Class 标识设计方案

日期：2026-03-24

状态说明：

- 文中“为什么需要改造”的问题已经大体在当前实现中落地为结构化 `LockClassKey`。
- 但本文仍然有设计文档性质，尤其是更强的实例敏感、对象来源链和 annotation 配合部分，
  依旧是后续工作，而不是“已经全部完成”。

本文档给出 `tools/lockdep` 中锁类标识（`LockClass`）的改造方案，目标是替换当前过于粗糙的字符串键，减少不同锁类被错误合并导致的误报。

## 1. 背景

当前 `lockdep` 在识别一次加锁调用后，会为该锁生成一个 `class: String`。

关键路径：

- [`tools/lockdep/analysis/src/collect.rs:535`](/root/asterinas-codex/tools/lockdep/analysis/src/collect.rs#L535)
- [`tools/lockdep/analysis/src/collect.rs:883`](/root/asterinas-codex/tools/lockdep/analysis/src/collect.rs#L883)
- [`tools/lockdep/analysis/src/collect.rs:919`](/root/asterinas-codex/tools/lockdep/analysis/src/collect.rs#L919)
- [`tools/lockdep/analysis/src/model.rs:18`](/root/asterinas-codex/tools/lockdep/analysis/src/model.rs#L18)

当前规则本质上是把 MIR `Place` 编码成一个简化字符串：

- root
  - 返回值：`<fn_path>::return`
  - 参数：`arg1`、`arg2`
  - 局部：`<fn_path>::local12`
- projection
  - `Deref` => `.*`
  - `Field` => `.fieldN`
  - 其余投影类似处理

所以最后会出现这类键：

- `arg1.*.field1`
- `arg1.*.field0`
- `some::func::local12.*.field0`

## 2. 当前问题

这种标识方案的核心问题是：

- 它描述的是“当前 MIR 里如何走到这个 receiver”
- 它没有稳定描述“这把锁在语义上是谁”

因此会出现以下误合并：

1. 不同类型的方法参数碰撞

- 两个不同类型的方法都可能把 `self` 编码为 `arg1`
- 如果它们都在“解一次引用后访问第 1 个字段”，就都会落成 `arg1.*.field1`

2. 字段序号不具备语义

- `.field1` 只说明“第 1 个字段”
- 不说明该字段属于哪个类型，也不说明字段名是什么

3. `Deref` 丢失类型信息

- `.*` 只说明发生了解引用
- 不说明解引用后的目标类型

4. 局部变量身份不稳定

- `local12` 是 MIR local 编号
- 编号只在当前函数 MIR 内有意义，不适合做长期稳定语义键

这正是诸如以下误报的来源：

- `UartConsole.callbacks` 和 `RamInode.metadata` 都被归成 `arg1.*.field1`
- `OverlayInode.upper` 在不同对象实例上被当成同一把锁

## 3. 设计目标

新设计需要满足：

1. 稳定

- 不依赖易漂移的 `arg1`、`local12`
- 尽量依赖类型、定义点、字段定义等稳定信息

2. 可区分

- 不同类型的锁不能再因为“字段序号相同”而合并
- 不同来源的局部值要尽量恢复其真实来源

3. 可序列化

- 仍然要能导出 JSON
- 也要方便在终端/报告里显示

4. 渐进落地

- 第一阶段先解决最主要的跨类型误合并
- 第二阶段再做更细粒度的实例敏感

## 4. 总体方案

把当前的 `class: String` 改成“结构化锁类键 + 可读字符串表示”。

建议新增：

```rust
pub struct LockClassKey {
    pub root: LockRootKey,
    pub projections: Vec<ProjectionKey>,
}

pub enum LockRootKey {
    Global {
        def_path: String,
    },
    ReceiverArg {
        method_def_path: String,
        self_ty: String,
    },
    FnArg {
        fn_def_path: String,
        index: usize,
        ty: String,
    },
    Local {
        fn_def_path: String,
        index: usize,
        ty: String,
        origin: LocalOriginKey,
    },
    ReturnValue {
        fn_def_path: String,
        ty: String,
    },
}

pub enum LocalOriginKey {
    Unknown,
    AliasOf(Box<LockRootKey>),
    AggregateTemp {
        ty: String,
    },
    RefOfPlace {
        base: Box<LockRootKey>,
    },
}

pub enum ProjectionKey {
    Deref {
        pointee_ty: String,
    },
    Field {
        owner_ty: String,
        field_name: String,
        field_index: usize,
    },
    Downcast {
        enum_ty: String,
        variant_name: String,
    },
    Index,
    ConstantIndex,
    Subslice,
    OpaqueCast {
        ty: String,
    },
}
```

然后：

- 图算法和比较逻辑使用结构化键
- 输出 JSON 时序列化结构化键
- 终端摘要和报告里再格式化成字符串

## 5. Root 设计

### 5.1 `self` 参数

当前问题：

- `self` 被编码成 `arg1`
- 不同类型的方法会碰撞

新规则：

- 如果当前函数是 method，且 `local == 1` 对应 receiver
- 则 root 编码为：

```rust
ReceiverArg {
    method_def_path,
    self_ty,
}
```

效果：

- `UartConsole::trigger_input_callbacks` 的 `self`
- 和 `RamInode::atime` 的 `self`
- 不会再共用同一个 root

### 5.2 普通函数参数

当前问题：

- `arg2` 这种表示没有函数和类型信息

新规则：

```rust
FnArg {
    fn_def_path,
    index,
    ty,
}
```

效果：

- 不同函数里的第一个参数不会再天然碰撞

### 5.3 局部变量

当前问题：

- `fn_path::local12` 只是 MIR 临时编号

新规则：

```rust
Local {
    fn_def_path,
    index,
    ty,
    origin,
}
```

其中 `origin` 尽量恢复来源。

建议在 [`apply_statement()`](/root/asterinas-codex/tools/lockdep/analysis/src/collect.rs#L400) 中维护 `local_origins`：

- `Rvalue::Ref(_, _, source)`：
  - 记录为 `RefOfPlace`
- `Rvalue::Use(operand)`：
  - 若来自另一个 local，记录为 `AliasOf`
- `Rvalue::Cast(_, operand, _)`：
  - 若来自另一个 local，也记录为 `AliasOf`
- 其余：
  - 暂时记录为 `Unknown`

这能让很多局部 receiver 不再只剩下 `_12` 这种无语义编号。

### 5.4 返回值

保持单独 root：

```rust
ReturnValue {
    fn_def_path,
    ty,
}
```

## 6. Projection 设计

### 6.1 `Deref`

当前问题：

- 只编码为 `.*`
- 丢失 pointee 类型

新规则：

```rust
Deref {
    pointee_ty,
}
```

这样即使路径形状相同：

- `&UartConsole -> field1`
- `&RamInode -> field1`

也不会再碰撞。

### 6.2 `Field`

当前问题：

- 只编码 `.fieldN`
- 缺少字段所属类型和字段名

新规则：

```rust
Field {
    owner_ty,
    field_name,
    field_index,
}
```

构造时应沿着 `Place` 逐步推进当前类型：

- 先拿 root 对应的类型
- 遇到 `Deref` 就切到 pointee type
- 遇到 `Field` 时，从当前 ADT 上解析字段定义
- 写入字段名和 owner type

效果：

- `UartConsole.callbacks`
- `RamInode.metadata`

即使都在“第 1 个字段”，也不会再相等。

### 6.3 `Downcast`

当前规则只记 `variantN`。

建议改成：

```rust
Downcast {
    enum_ty,
    variant_name,
}
```

这样更稳定，也更可读。

### 6.4 其他投影

以下几类先保留较粗表示即可：

- `Index`
- `ConstantIndex`
- `Subslice`
- `OpaqueCast`

这些不是当前误报主因。

## 7. 显示格式

内部使用结构化键后，仍需要一个稳定、可读的显示格式。

建议统一格式化为：

```text
root=<...>;proj=deref(<ty>)->field(<owner>::<name>#<index>)->...
```

例如：

```text
root=self(method=kernel::...::UartConsole::trigger_input_callbacks,self_ty=kernel::...::UartConsole<...>);proj=deref(kernel::...::UartConsole<...>)->field(kernel::...::UartConsole<...>::callbacks#1)
```

简化显示时可以再折叠成：

```text
UartConsole::self.callbacks
```

但比较逻辑应始终使用结构化键，而不是格式化字符串。

## 8. 分阶段落地方案

### 阶段 1：先修掉主要误合并

目标：

- 解决 `arg1.*.fieldN` 这类跨类型碰撞

建议最小改动：

1. `LockInfoArtifact.class` 改成结构化键，或增加 `class_key`
2. `self` root 特殊化，带上 `self_ty`
3. 普通参数 root 带上 `fn_def_path + index + ty`
4. `Field` 带上 `owner_ty + field_name + field_index`
5. `Deref` 带上 `pointee_ty`

这一阶段就足以明显减少：

- `UartConsole.callbacks` vs `RamInode.metadata`
- 大量跨模块拼接出的假 cycle

### 阶段 2：补充局部来源恢复

目标：

- 降低 `local12.*.field0` 这类局部路径误差

建议改动：

1. `BlockState` 新增 `local_origins`
2. 在 `apply_statement()` 中维护来源
3. `lock_class_key()` 在遇到 local root 时优先使用 `origin`

### 阶段 3：做实例敏感

目标：

- 区分同一类型、同一字段、不同对象实例

典型场景：

- `self.upper`
- `self.parent.upper`

这一层不能只靠“类型 + 字段名”解决。

可选方向：

1. 为 root 引入“来源链”

- `self`
- `self.parent`
- `arg2`
- `local derived from self.parent`

2. 对局部 alias 图做更强的 place 归约

- 让 `parent.upper` 不再和 `self.upper` 折叠成同一节点

这部分可以放在第一阶段之后，不必一开始做满。

## 9. 需要修改的代码位置

核心改动点：

- [`tools/lockdep/analysis/src/model.rs`](/root/asterinas-codex/tools/lockdep/analysis/src/model.rs)
  - 定义新的 `LockClassKey`
  - 更新 `LockInfoArtifact`

- [`tools/lockdep/analysis/src/collect.rs:513`](/root/asterinas-codex/tools/lockdep/analysis/src/collect.rs#L513)
  - `classify_lock_call()` 构造新的 class key

- [`tools/lockdep/analysis/src/collect.rs:883`](/root/asterinas-codex/tools/lockdep/analysis/src/collect.rs#L883)
  - 重写 `lock_class_key()`

- [`tools/lockdep/analysis/src/collect.rs:919`](/root/asterinas-codex/tools/lockdep/analysis/src/collect.rs#L919)
  - 替换 `place_root_key()`

- [`tools/lockdep/analysis/src/collect.rs:400`](/root/asterinas-codex/tools/lockdep/analysis/src/collect.rs#L400)
  - 为 local 增加来源跟踪

- [`tools/lockdep/src/main.rs`](/root/asterinas-codex/tools/lockdep/src/main.rs)
  - 更新所有依赖 `class: String` 的比较和展示逻辑

## 10. 预期收益

完成阶段 1 后，应当直接减少以下误报：

1. 不同类型上同字段序号的误合并

- 例如 `UartConsole.callbacks`
- 和 `RamInode.metadata`

2. 跨模块拼接出的假 cycle

- witness 边不再仅凭 `arg1.*.fieldN` 相连

3. IRQ 冲突误报

- 不同类型的 `SpinLock` 不会再因为字段序号相同而被视为同一锁类

## 11. 非目标

本设计的第一阶段不试图完全解决：

- ordered double-lock helper 的路径敏感问题
- `RwLock` / `RwMutex` 的 mode compatibility 问题
- 所有实例级别 alias 归约问题

这些是独立精度问题，应在锁类标识稳定后继续处理。

## 12. 为什么不能只用定义位置作为 Key

一个直觉上的方案是：

- 对每个锁类，直接使用“该锁在源码中的定义位置”
- 例如 `file:line:column`

这个信息可以作为辅助信息，但不能单独作为锁类唯一 key。

原因如下。

### 12.1 它无法区分同一字段的不同实例

定义位置只能回答：

- “这个字段/静态锁定义在哪里”

但 lockdep 还需要回答：

- “当前路径拿到的是哪个对象上的这把锁”

典型例子：

- [`kernel/src/fs/fs_impls/overlayfs/fs.rs`](/root/asterinas-codex/kernel/src/fs/fs_impls/overlayfs/fs.rs) 中的 `OverlayInode.upper`

这里：

- `self.upper`
- `self.parent.upper`

字段定义位置完全一样，但运行时是两把不同的锁。

如果 key 只用字段定义位置，这两把锁仍然会被合并，`self.upper` 和 `parent.upper` 还是无法区分。

### 12.2 它无法表达“参数上传入的锁”

很多函数加锁的对象不是在当前函数里直接定义出来的，而是从参数传入的。

例如：

```rust
fn foo(lock: &SpinLock<T>) {
    let _guard = lock.lock();
}
```

在 `foo` 内部并不知道：

- 这个 `lock` 最终对应哪个具体字段
- 它来自哪个对象
- 它在调用方源码中的定义位置是什么

对这种函数，摘要里必须先保留“相对来源”，例如：

- 第 1 个参数上的某个锁路径

否则就没法把 `foo` 的行为传播回不同调用点。

### 12.3 它会把批量实例过度合并

即使拿到了一个锁字段或构造点的定义位置，也仍然可能有很多不同实例共享同一位置，例如：

- `Vec<SpinLock<_>>` 的多个元素
- hash bucket 中的一组锁
- 同一 struct 的多个实例
- 同一循环构造出的多个对象

如果 key 只靠定义位置，这些锁都会被并成一个类，精度仍然不够。

### 12.4 它是文本坐标，不是语义身份

`file:line:column` 这种坐标在工程里并不稳定：

- 注释或空行变化会导致行号变化
- 重排代码会让 key 改变，但语义未变
- 宏展开和生成代码的 span 也未必稳定

而锁类更适合用“语义身份”描述，例如：

- 定义点 `DefId`
- receiver 类型
- 字段所属类型与字段名
- 参数/局部来源链

### 12.5 结论

定义位置不是没用，而是不足以单独承担锁类身份。

更合适的用法是：

- 对 `static` / 全局锁，把定义点作为 root 身份的一部分
- 对字段锁，把字段定义信息作为 field 身份的一部分
- 对报告展示，输出人类可读的 `file:line:column`

但 lock class 的核心 key 仍然需要包含：

- 来源
- 类型
- 投影语义
- 必要时的实例来源链

## 13. 关于“传参时代入锁来源”

另一个自然的想法是：

- callee 里先把锁记成 `arg1...`
- 在调用点再把实参来源代入进去

这个方向是正确的，而且应当实现，但它不能替代锁类本身的稳定设计。

### 13.1 它能解决什么

它主要解决的是跨函数绑定问题。

例如 callee 内部拿的是：

- `arg1.lock`

而 caller 调用时传入的是：

- `self.foo`

那么在做函数摘要传播时，可以把 callee 中相对于 `arg1` 的锁路径重绑定到 caller 的实际来源上。

这一步很重要，因为很多辅助函数并不知道实参最终来自哪里。

### 13.2 为什么它单独不够

即使做了调用点代入，也仍然有三类问题解决不了。

#### 13.2.1 很多误报发生在函数内部

有些误报不是跨函数传播时才产生的，而是在函数内部建边、建事件、建 IRQ 报告时就已经发生了。

例如不同类型的方法内部都把 `self` 编成：

- `arg1.*.field1`

那在函数内部分析阶段，它们就已经可能被误并，不需要等到 callsite substitution 才出问题。

#### 13.2.2 不是所有来源都来自参数

很多锁路径来源于：

- `self.field`
- `self.parent.field`
- 局部变量
- 临时值
- 返回值
- 闭包捕获

这些来源不都能靠“把 `arg1` 替换成实参”恢复。

尤其像：

- `self.upper`
- `self.parent.upper`

这类问题需要保留对象来源链，而不只是参数替换。

#### 13.2.3 前提是 callee 的占位符本身足够有语义

如果 callee 的摘要只是：

- `arg1.*.field1`

那就算 caller 想代入，信息也还是太少。

因为这里看不出来：

- owner type 是谁
- `field1` 对应的字段名是什么
- `Deref` 后的类型是什么

所以要先让 callee 内部使用“结构化、可代入”的锁类表示，再做参数代入。

### 13.3 正确的位置

“传参时代入来源”应当是第二层机制，不是第一层。

建议顺序：

1. 先把 lock class 改成结构化键
   - root 带来源和类型
   - projection 带字段语义和 pointee type
2. 再在 interprocedural summary 传播时，对 `FnArg` / `ReceiverArg` 做实参替换

也就是说：

- 函数内部分析先得到“相对来源”的结构化 key
- 跨函数传播时，再把相对来源绑定到调用点上的实际来源

### 13.4 推荐的代入方式

建议把参数相关 root 设计成可重绑定的占位符：

```rust
ReceiverArg {
    method_def_path,
    self_ty,
}

FnArg {
    fn_def_path,
    index,
    ty,
}
```

然后在 callsite 传播时：

- 把 callee 的 `ReceiverArg` / `FnArg`
- 替换成 caller 当前实参对应的来源树

这样 callee 摘要才真正可复用。

### 13.5 结论

“传参时代入来源”是需要做的，但它解决的是：

- 跨函数的来源绑定

它不能单独解决：

- 函数内部的错误合并
- 同一类型不同实例的区分
- 占位符本身过于粗糙的问题

因此它应当建立在“结构化 lock class key”之上，而不是替代它。

## 14. 最终建议

建议按下面顺序推进：

1. 先把 `class` 从字符串改成结构化键
2. 立即补上：
   - `self_ty`
   - `fn_def_path`
   - `owner_ty`
   - `field_name`
   - `pointee_ty`
3. 再补 local 来源追踪
4. 最后再考虑实例敏感

这是当前减少误报、又不至于把实现复杂度拉得过高的最短路径。
