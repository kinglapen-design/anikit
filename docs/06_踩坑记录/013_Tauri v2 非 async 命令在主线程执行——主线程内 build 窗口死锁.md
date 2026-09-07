## 013 Tauri v2 非 async 命令在主线程执行——主线程内 build 窗口死锁

- **现象：** 预览"新窗口打开"点击后整个进程卡死——主窗口无法拖动/关闭、托盘菜单无响应；新窗口出现但不渲染内容、压在主窗口下方。
- **原因：** Tauri v2 中**未声明 async 的命令在主线程执行**。`open_preview_window` 是 sync 命令，在主线程内 `WebviewWindowBuilder::build()`，窗口创建流程与主线程事件循环互等，主线程死锁。
- **解法：** 命令改 `pub async fn`（线程池执行，build 内部 dispatch 回主线程不阻塞——官方 multiwindow 示例路径）；preview 模块 5 条命令一并 async 化（list_sequence_frames 帧条小图生成等 IO 重活也不该占主线程）。
- **预防：** Tauri v2 命令默认主线程执行——**凡涉及窗口创建 / IO 重活 / COM 调用的命令一律 async**；纯内存快操作（取锁读索引）才用 sync。新增命令时对照本条自查。
