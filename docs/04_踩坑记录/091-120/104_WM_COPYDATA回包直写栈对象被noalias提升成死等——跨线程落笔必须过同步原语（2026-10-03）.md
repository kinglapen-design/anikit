# WM_COPYDATA 回包直写栈对象被 noalias 提升成死等——跨线程落笔必须过同步原语（2026-10-03）

## 现象

Everything IPC 适配器（`adapters/everything.rs`）真机冒烟：查询发出成功（Everything 的 WM_COPYDATA 回包确证到达本线程 WndProc，dwData / cbData 全对），回包字节也拷贝进了栈上 `ReplySlot.buf`——但等包循环的 `slot.buf.is_some()` **永远读到 false**，10 秒超时。同形态 Python 探针（同协议同窗口同泵法）完全正常。

## 原因

回包的落笔时序：调用线程阻塞在 `SendMessageTimeoutW(SMTO_BLOCK)` 内——Windows 为防死锁，阻塞期间**入站的 sent 消息会在本线程派发**（Everything 进程线程 SendMessage 我方接收窗 → 我方 WndProc 在此窗口期执行，写栈对象）。问题出在 Rust 优化器：等包循环持有 `&mut ReplySlot`，LLVM noalias 规则认定「该引用独占、循环内 FFI 调用（PeekMessage）不可能写入」→ **把 `buf.is_some()` 的读提升出循环**，只读一次 = 死等。Python 无此优化所以正常。这是「别名 UB 在实践中的真实咬人」教科书案例——代码按 C 直觉写完全成立，优化后才炸。

## 解法

回包通道改 `static REPLIES: Mutex<Vec<(u32 reply_id, Vec<u8>)>>`：WndProc 锁内 push，等包循环锁内按 id 取——Mutex 锁 / 解锁是编译器与用户双重可见的同步点，读不可能被提升。附带收益：天然支持多查询并发对号（不再需要 GWLP_USERDATA 挂栈指针 + 清零时序那一套）。Windhawk 参照实现用的是专用接收线程 + 事件 + atomic，同属「过同步原语」路线。

## 预防

- **跨线程 / 跨进程经裸指针落笔的共享状态，读取端必须过同步原语（Mutex / Atomic / 事件）**，禁止「栈对象 + 裸指针 + 优化器视野外写入」——`&mut` 的 noalias 契约在 FFI 边界上特别容易被静默违反。
- 信号 =「日志证明写发生了，读却永远看不到」+ 只在优化构建 / 特定语言出现：先查别名与可见性，再查协议。
- 本项目新例入档：凡 Win32 回调（WndProc / hook）向调用方返还数据，一律走静态同步容器或 Atomic，不走 GWLP_USERDATA 挂栈对象回写（shell_menu 的 Forwarder 只是「读」既有对象，无此问题）。
