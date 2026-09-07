## 014 测试 exe 链接 tauri GUI 依赖链：0xc0000139 入口点未找到

- **现象：** preview_queue 测试持有 `AppHandle` 后，Windows 上测试 exe 加载即失败（STATUS_ENTRYPOINT_NOT_FOUND），全部测试无法启动。
- **原因：** 测试代码经 `AppHandle::emit` / 析构把 tauri GUI 依赖链（rfd 的 `comctl32!TaskDialogIndirect` 等）拉进测试 exe，而测试 exe 无 comctl32 v6 清单（tauri-build 只给主程序内嵌），System32 的 v5.82 无此导出。build.rs `/MANIFESTINPUT` 与 tauri-build 清单资源冲突、`#[link(+whole-archive)]` LNK1356，均不可行。
- **解法：** AppHandle 字段与 emit 实现 `#[cfg(not(test))]`，测试构建起无事件实例（`start(None)`，句柄 mem::forget 丢弃）——生产路径行为不变。
- **预防：** services 层需要 AppHandle（事件 / 窗口）时，一律走"Option 句柄 + cfg(test) 空实现"模式，测试不碰 tauri GUI 类型；命令层测试调私有逻辑函数，不直接构造 AppHandle。
