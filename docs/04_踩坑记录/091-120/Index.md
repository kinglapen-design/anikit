# 踩坑记录 091-120

> 四段式：现象 / 原因 / 解法 / 预防。共 9 条。返回 [Index](../Index.md)。

| 编号 | 标题 | 日期 | 链接 |
| --- | --- | --- | --- |
| 099 | RDC 递归监听不穿透 junction——链接子树事件零上报（notify Windows 后端无标志静默漏报，RDC 机制本身行为）；每链接目标单独补 target 监听 + 事件路径翻译回词法门路径走同一 changed_dirs 管道，拓扑变化恒整体重建（2026-10-02） | 2026-10-02 | [RDC 递归监听不穿透 junction（2026-10-02）](099_RDC递归监听不穿透junction——链接子树事件零上报，每链接目标单独补监听+路径翻译（2026-10-02）.md) |
| 098 | dockview 对已激活 panel 重复 openPanel 无守卫——pane 聚焦 / 切标签即 renderPanel（内容元素 detach→reattach），内嵌滚动位置归零（Vue 无感：组件不重建、元素同一；MO 轨迹 childList -1/+1 实锤）；应用层滚动保活 = 位置记组件闭包 + 盯宿主 content 容器 childList 恢复（2026-10-02） | 2026-10-02 | [dockview 重复 openPanel 无守卫（2026-10-02）](098_dockview重复openPanel无守卫——pane激活即detach重挂内容滚动归零（2026-10-02）.md) |
| 097 | tao transparent 子窗口环境性瞬时全灭——实体 Win11 同版系统隔夜自愈无根因可捕（故障窗口期无日志）；open_child_window 失败降级非透明重建 + release 文件日志 + invoke 封装 toast 上保险；发包命名 v{版本}-{日期}（2026-09-30） | 2026-09-30 | [tao transparent 子窗口环境性瞬时全灭（2026-09-30）](097_tao_transparent子窗口环境性瞬时全灭——隔夜自愈无根因可捕，降级重建+release文件日志上保险（2026-09-30）.md) |
| 096 | cargo build --release ≠ tauri build——custom-protocol feature 决定 dev/prod（不看 PROFILE），直编出 dev 模式包连 devUrl 白屏；冒烟只验后端端口是盲点，release 验证必须含真 UI 截图（2026-09-29） | 2026-09-29 | [cargo 直编出 dev 模式包（2026-09-29）](096_cargo_release直编≠tauri_build——custom-protocol注入缺失出dev模式包连devUrl白屏（2026-09-29）.md) |
| 091 | 工具链升级后增量缓存混链——anon.*.llvm 符号 LNK2019（cargo clean -p 不够，清 target/debug/incremental 才断根；2026-10-01 三次复发（patch 升级同触发）→ [profile.dev] incremental=false 终局关闭）（2026-09-26） | 2026-09-26 | [工具链升级后增量缓存混链（2026-09-26）](091_工具链升级后增量缓存混链——anon.llvm符号LNK2019，清incremental重链（2026-09-26）.md) |
| 092 | URL 编码路径做文件名超 NTFS 255 组件上限——中文每字符 9 倍膨胀（截断前缀 + FNV-1a 散列有界命名，purge 改按 hash 段匹配，cache_v2 标志一次性清旧缓存）（2026-09-26） | 2026-09-26 | [URL 编码路径做文件名超 NTFS 255 组件上限（2026-09-26）](092_URL编码路径做文件名超NTFS255组件上限——中文9倍膨胀，截断前缀+FNV散列有界命名（2026-09-26）.md) |
| 093 | 仅改图标 tauri build 不重嵌 exe 图标——build.rs rerun-if-changed 13 条不含 icons/（pefile 帧覆盖率对账实锤；touch tauri.conf.json 逼构建脚本重跑）（2026-09-26） | 2026-09-26 | [仅改图标不重嵌 exe 图标（2026-09-26）](093_仅改图标tauri_build不重嵌exe图标——rerun-if-changed不含icons，touch_conf触发重跑（2026-09-26）.md) |
| 094 | Win11 25H2 窗口双杀：DWM 默认边框（COLOR_NONE）停画 + 浅色主题云母变白（窗口主题跟随系统，Mica 浅色变体 × 无浅色 UI）——显式染 #333 边框 + theme:"Dark" 锁窗口深色（2026-09-29） | 2026-09-29 | [25H2 边框停画 + 浅色云母变白（2026-09-29）](094_Win11_25H2窗口双杀：DWM默认边框停画+浅色主题云母变白——显式染边框+锁窗口深色主题（2026-09-29）.md) |
| 095 | 配置写盘「先删后换」窗口期丢文件——MCP 写入闸等无快照子树静默回默认（目标丢失后 ui/几何有前端快照自动重建、唯无快照子树可见）；MoveFileExW 原子替换断根 + load_from tmp 残留兜底恢复（2026-09-29） | 2026-09-29 | [先删后换窗口期丢文件（2026-09-29）](095_配置写盘先删后换窗口期丢文件——MCP写入闸等无快照子树静默回默认，MoveFileExW原子替换断根（2026-09-29）.md) |
| 096 | Tauri 拖拽事件坐标是物理像素（PhysicalPosition）——elementFromPoint / getBoundingClientRect 是 CSS 逻辑像素，125% 缩放下混用 = 落点命中区整体偏移（侧边栏右侧误触发 / 视口右侧不触发；库导入树行落点同隐患同修）；onDragDropEvent 入口统一除 devicePixelRatio（2026-10-01） | 2026-10-01 | [拖拽事件物理像素坐标（2026-10-01）](096_Tauri拖拽事件坐标是物理像素——125缩放下落点命中区整体偏移（2026-10-01）.md) |
