# 095 配置写盘「先删后换」窗口期丢文件——MCP 写入闸等无快照子树静默回默认，MoveFileExW 原子替换断根（2026-09-29）

- **现象：** 用户实机长期间歇报「MCP 允许写入开关自己关掉了」——不是每次，重启电脑后、或某次重启应用后发现回关；其余设置（窗口位置、UI 偏好）从无异常。2026-09-29 排障会话中强杀进程换包后当场复现。
- **根因链：** `json_io::write_text_atomic` 原实现 = 写 `.tmp` → `remove_file(目标)` → `rename(tmp, 目标)`（Windows 的 `fs::rename` 不覆盖已存在文件，项目早期实证后采用先删后换）。**删与换之间存在窗口期——进程恰在此间隙被杀（强杀 / 关电源 / 关机）= 目标文件永久丢失**，下次启动 `load_from` 按默认重建。窗口仅微秒级，故表现为「不知道哪次就关了」的偶发。config.json 丢失后：ui 子树（前端 store 防抖写回）与窗口几何（useWindowStatePersistence）由**前端内存态启动后随即写回**，用户无感知；**唯 `settings.mcp`（写入闸/服务开关）、`settings.pluginServer` 等无前端快照的子树无人重建** → 静默回默认关。「只丢开关、不丢别的」的谜底即此。
- **解法：**
  - 写点断根（全局数据文件共用）：`write_text_atomic` 的删-换改 `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)`——同卷内原子替换无窗口期，WRITE_THROUGH 保证落盘再返回；非 Windows 保持 `fs::rename`（POSIX 原子覆盖）。
  - 读侧兜底（config.json）：`load_from` 目标缺失但 `.tmp` 存在且可解析 → 恢复当次写入成果并顺手转正（半截 tmp 弃用回默认）——兜住旧代码已造成 / 「tmp 写完未替换」被杀的残余现场。
- **预防：** ①Windows 上「先删后换」不是原子写，覆盖替换必须 `MoveFileExW(REPLACE_EXISTING)`（Rust `fs::rename` 在 Windows 不覆盖已存在文件）；②评估「丢配置」类 BUG 时先分清哪些子树有前端快照会自动重建、哪些没有——无快照子树才是用户唯一可见的伤口；③排查路径上强杀进程（Stop-Process -Force）本身就会触发此类窗口期事故，属既成事实干扰项，注意与原始症状区分。
