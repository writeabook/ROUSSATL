# OSAL 后端行为契约

## 1. 目的

本文档定义了每个 OSAL 后端必须满足的行为契约。它描述的是**什么是正确行为**，而非**如何实现**。

任何实现 `osal-api` trait 的 crate 都必须通过契约测试。这确保了应用代码在不同后端上表现一致。

## 2. 后端要求

每个后端必须：

1. 实现 `osal-api` 中的所有 trait，在合法输入下不 panic
2. 通过 `osal-testkit` 中的契约测试套件
3. 将平台特定错误映射为 `osal_api::Error` 变体
4. 记录任何有意的行为偏差
5. 不通过公共 API 暴露平台特定类型或错误码

## 3. Trait 契约

### 3.1 Mutex（互斥锁）

保护类型 `T` 的值的互斥锁。

**要求行为：**

- `lock(timeout)` 获取锁，阻塞时间不超过 `timeout`
- 成功时返回一个 guard，提供 `&mut T` 访问
- guard 被 drop 时释放锁
- 可重入：拥有者可以多次锁定同一个 mutex（每次 `lock()` 必须匹配一次 guard drop）
- 非拥有者在尝试锁定已被锁定的 mutex 时必须阻塞或失败
- `Timeout::NoWait` 必须立即返回；`Timeout::Forever` 必须阻塞直到获取成功

**错误条件：**
- `Error::LockFailed` — 无法获取锁
- `Error::Timeout` — 超时后仍未获取到锁

**ISR 行为：**
- `isr_lock()` 非阻塞；若锁被其他上下文持有则返回 `Error::LockFailed`

### 3.2 Semaphore（信号量）

用于资源管理和信号传递的计数信号量。

**要求行为：**

- `acquire(timeout)` 若 count > 0 则减 1，否则阻塞
- `release()` 将 count 增加到 `max_count` 并唤醒一个等待者
- `count()` 返回当前计数（非阻塞）
- `max_count()` 返回最大计数
- 二进制信号量等价于 `max = 1` 的计数信号量

**错误条件：**
- `Error::Timeout` — 超时后仍未获取到
- `Error::InvalidParameter` — 初始计数 > 最大计数

**ISR 行为：**
- `isr_acquire()` 非阻塞
- `isr_release()` 可在中断上下文中调用

### 3.3 Queue（队列）

有界 FIFO 消息队列，用于任务间通信。

**要求行为：**

- `send(msg, timeout)` 入队消息；队列满时阻塞
- `recv(msg, timeout)` 出队消息；队列空时阻塞
- FIFO 顺序：消息按发送顺序被接收
- 固定消息大小：所有消息具有相同的字节长度
- 容量在创建时固定；`len()` / `capacity()` 报告状态

**错误条件：**
- `Error::Timeout` — 发送/接收超时
- `Error::QueueFull` — 队列满时非阻塞发送失败
- `Error::QueueEmpty` — 队列空时非阻塞接收失败
- `Error::InvalidMessageSize` — 消息大小不匹配

**唤醒规则：**
- 一次 `send()` 最多唤醒一个被阻塞的接收者
- 一次 `recv()` 最多唤醒一个被阻塞的发送者

### 3.4 Task（任务/线程）

独立的执行上下文。

**要求行为：**

- 任务有名称、栈大小、优先级和入口函数
- `spawn()` 启动任务；入口函数在新上下文中执行
- `join(timeout)` 等待任务完成并返回结果
- 任务由不透明的 `Handle` 标识
- `current()` 返回调用者任务的 handle
- 优先级决定调度顺序（数值越大越紧急）

**生命周期状态：**
- `Created`（已创建）→ `Ready`（就绪）→ `Running`（运行中）→ `Finished`（已完成）
- 中间状态：`Blocked`（阻塞中）、`Suspended`（已挂起）

**错误条件：**
- `Error::InvalidParameter` — 名称过长或栈过小
- `Error::AlreadyInitialized` — 任务已经启动
- `Error::NotInitialized` — 尝试 join 未启动的任务

### 3.5 Timer（定时器）

用于延迟和周期性回调的软件定时器。

**要求行为：**

- `start()` 开始倒计时；回调在周期结束后执行
- `stop()` 阻止未来的回调（正在执行的回调不会被中断）
- `reset()` 从当前时间重新开始倒计时
- `change_period(new_period)` 更新后续触发的周期
- 单次：触发一次后停止
- 周期：每次到期后自动重新加载
- 回调在定时器管理锁之外执行

**精度：**
- 定时器不得在周期之前触发
- 实际触发可能因调度延迟而推后
- 实时精度取决于后端

**错误条件：**
- `Error::InvalidParameter` — 周期为零

### 3.6 Event Flags（事件标志）

多比特同步：任务等待特定比特位被设置。

**要求行为：**

- `set(bits)` 设置指定位；唤醒匹配的等待者
- `clear(bits)` 清除指定位
- `get()` 返回当前位掩码（非阻塞）
- `wait(mask, timeout)` 阻塞直到 `mask` 中**任意**位被设置
- 从 `wait` 返回时位**不会**被自动清除
- 调用者检查 `returned & mask != 0` 判断成功

**等待语义：**
- OR 语义（任意位）：默认行为。任意请求位被设置时 wait 返回。

**错误条件：**
- `Error::Timeout` — 超时且无匹配位被设置

### 3.7 Clock（时钟）

时间测量和延迟原语。

**要求行为：**

- `now()` 返回单调递增的时间戳
- `elapsed(since)` 返回自时间戳以来的时长
- `delay(duration)` 将调用任务阻塞至少 `duration` 时间
- 时钟必须单调；不能倒退
- Tick 周期由后端定义，但必须记录文档

**精度：**
- `delay(0)` 必须立即返回
- `delay(d)` 必须阻塞至少 `d` 时间；可能因调度而更长

### 3.8 System（系统控制）

全局系统操作。

**要求行为：**

- `critical_section_enter()` / `critical_section_exit()` 为短临界区提供互斥
- `heap_free()` 返回可用堆字节数（在虚拟内存系统上可返回 `usize::MAX`）
- `task_count()` 返回已注册任务数

**临界区规则：**
- 临界区可嵌套
- 在实时后端上可禁用中断
- 在主机系统上，进程级 mutex 即可满足需求

## 4. ISR 安全性

某些后端（FreeRTOS）区分任务上下文和中断服务例程（ISR）上下文。在没有真正 ISR 的后端（POSIX, Mock）上，`isr_*` 方法必须：

- 非阻塞
- 不等待条件变量
- 在有界时间内完成
- 如果操作无法安全执行，返回 `Error::Unsupported`

## 5. 错误映射

每个后端将其原生错误映射为 `osal_api::Error`：

| 条件 | OSAL 错误 |
|------|-----------|
| 内存分配失败 | `Error::OutOfMemory` |
| 超时/截止时间到 | `Error::Timeout` |
| 队列已满 | `Error::QueueFull` |
| 队列为空 | `Error::QueueEmpty` |
| 锁竞争 | `Error::LockFailed` |
| 无效参数 | `Error::InvalidParameter` |
| 功能不可用 | `Error::Unsupported` |
| 意外的原生错误 | `Error::Internal("描述")` |

原始平台错误码（`errno`、FreeRTOS `pdFAIL` 等）不得通过 OSAL API 泄漏。

## 6. 并发保证

所有公共 OSAL 类型在适用时必须为 `Send + Sync`。

- `Mutex<T>`：当 `T: Send` 时为 `Send + Sync`
- `Queue`：`Send + Sync`
- `Semaphore`：`Send + Sync`
- `EventFlags`：`Send + Sync`
- `Task`：`Send + Sync`
- `Timer`：`Send + Sync`

这些类型的操作默认是线程安全的。`isr_*` 变体为中断上下文提供额外的保证。

## 7. 契约测试检查清单

每个后端在被接受前必须通过以下测试类别：

**Mutex（互斥锁）：**
- [ ] 创建、锁定、解锁
- [ ] guard drop 释放锁
- [ ] 拥有者可重入锁定
- [ ] 跨任务互斥
- [ ] 非阻塞 try-lock

**Semaphore（信号量）：**
- [ ] 以初始计数创建
- [ ] acquire 递减，release 递增
- [ ] 为空时超时
- [ ] signal 唤醒等待任务
- [ ] 达到最大计数时 release 返回错误

**Queue（队列）：**
- [ ] FIFO 顺序
- [ ] 未满时 send 成功
- [ ] 未空时 recv 成功
- [ ] 满/空时超时
- [ ] 被阻塞的发送者在 recv 后唤醒

**Task（任务）：**
- [ ] 创建、启动、join
- [ ] 向入口函数传递参数
- [ ] 多个并发任务
- [ ] 任务元数据查询

**Timer（定时器）：**
- [ ] 单次定时器触发一次
- [ ] 周期定时器重复触发
- [ ] stop 阻止回调
- [ ] reset 重新开始倒计时

**Event Flags（事件标志）：**
- [ ] set、get、clear 操作
- [ ] 任意位设置时 wait 返回
- [ ] 未设置位时 wait 超时
- [ ] 位不被自动清除

**Clock / System（时钟/系统）：**
- [ ] 时钟单调递增
- [ ] delay 阻塞至少请求的时间
- [ ] 临界区互斥
