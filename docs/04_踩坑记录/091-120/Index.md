# 踩坑记录 091-120

> 四段式：现象 / 原因 / 解法 / 预防。共 6 条。返回 [Index](../Index.md)。

| 编号 | 标题 | 日期 | 链接 |
| --- | --- | --- | --- |
| 096 | cargo build --release ≠ tauri build——custom-protocol feature 决定 dev/prod（不看 PROFILE），直编出 dev 模式包连 devUrl 白屏；冒烟只验后端端口是盲点，release 验证必须含真 UI 截图（2026-09-29） | 2026-09-29 | [cargo 直编出 dev 模式包（2026-09-29）](096_cargo_release直编≠tauri_build——custom-protocol注入缺失出dev模式包连devUrl白屏（2026-09-29）.md) |
| 091 | 工具链升级后增量缓存混链——anon.*.llvm 符号 LNK2019（cargo clean -p 不够，清 target/debug/incremental 才断根）（2026-09-26） | 2026-09-26 | [工具链升级后增量缓存混链（2026-09-26）](091_工具链升级后增量缓存混链——anon.llvm符号LNK2019，清incremental重链（2026-09-26）.md) |
| 092 | URL 编码路径做文件名超 NTFS 255 组件上限——中文每字符 9 倍膨胀（截断前缀 + FNV-1a 散列有界命名，purge 改按 hash 段匹配，cache_v2 标志一次性清旧缓存）（2026-09-26） | 2026-09-26 | [URL 编码路径做文件名超 NTFS 255 组件上限（2026-09-26）](092_URL编码路径做文件名超NTFS255组件上限——中文9倍膨胀，截断前缀+FNV散列有界命名（2026-09-26）.md) |
| 093 | 仅改图标 tauri build 不重嵌 exe 图标——build.rs rerun-if-changed 13 条不含 icons/（pefile 帧覆盖率对账实锤；touch tauri.conf.json 逼构建脚本重跑）（2026-09-26） | 2026-09-26 | [仅改图标不重嵌 exe 图标（2026-09-26）](093_仅改图标tauri_build不重嵌exe图标——rerun-if-changed不含icons，touch_conf触发重跑（2026-09-26）.md) |
| 094 | Win11 25H2 窗口双杀：DWM 默认边框（COLOR_NONE）停画 + 浅色主题云母变白（窗口主题跟随系统，Mica 浅色变体 × 无浅色 UI）——显式染 #333 边框 + theme:"Dark" 锁窗口深色（2026-09-29） | 2026-09-29 | [25H2 边框停画 + 浅色云母变白（2026-09-29）](094_Win11_25H2窗口双杀：DWM默认边框停画+浅色主题云母变白——显式染边框+锁窗口深色主题（2026-09-29）.md) |
| 095 | 配置写盘「先删后换」窗口期丢文件——MCP 写入闸等无快照子树静默回默认（目标丢失后 ui/几何有前端快照自动重建、唯无快照子树可见）；MoveFileExW 原子替换断根 + load_from tmp 残留兜底恢复（2026-09-29） | 2026-09-29 | [先删后换窗口期丢文件（2026-09-29）](095_配置写盘先删后换窗口期丢文件——MCP写入闸等无快照子树静默回默认，MoveFileExW原子替换断根（2026-09-29）.md) |
