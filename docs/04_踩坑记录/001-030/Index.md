# 踩坑记录 001-030

> 四段式：现象 / 原因 / 解法 / 预防。共 30 条。返回 [Index](../Index.md)。

| 编号 | 标题 | 日期 | 链接 |
| --- | --- | --- | --- |
| 001 | PySide6 快速滚动卡顿（历史项目） |  | [PySide6 快速滚动卡顿（历史项目）](001_PySide6 快速滚动卡顿（历史项目）.md) |
| 002 | `tauri dev` 一直 Waiting for frontend dev server |  | [`tauri dev` 一直 Waiting for frontend dev server](002_`tauri dev` 一直 Waiting for frontend dev server.md) |
| 003 | vite watcher 监视 cargo 产物导致 EBUSY 崩溃 |  | [vite watcher 监视 cargo 产物导致 EBUSY 崩溃](003_vite watcher 监视 cargo 产物导致 EBUSY 崩溃.md) |
| 004 | OLE 拖拽放线程池导致无声失败 |  | [OLE 拖拽放线程池导致无声失败](004_OLE 拖拽放线程池导致无声失败.md) |
| 005 | 超大列表原生滚动拖拽必然闪白（棋盘格） |  | [超大列表原生滚动拖拽必然闪白（棋盘格）](005_超大列表原生滚动拖拽必然闪白（棋盘格）.md) |
| 006 | Figma 导出图标把示例文字转曲烘进 SVG |  | [Figma 导出图标把示例文字转曲烘进 SVG](006_Figma 导出图标把示例文字转曲烘进 SVG.md) |
| 007 | 无边框窗口装饰四轮探索，终局为不透明 + 原生 DWM |  | [无边框窗口装饰四轮探索，终局为不透明 + 原生 DWM](007_无边框窗口装饰四轮探索，终局为不透明 + 原生 DWM.md) |
| 008 | Figma 描边 inside 不占空间，CSS border 外扩占位 |  | [Figma 描边 inside 不占空间，CSS border 外扩占位](008_Figma 描边 inside 不占空间，CSS border 外扩占位.md) |
| 009 | 长 dev 会话热更新状态损坏（界面渲染但交互全挂） |  | [长 dev 会话热更新状态损坏（界面渲染但交互全挂）](009_长 dev 会话热更新状态损坏（界面渲染但交互全挂）.md) |
| 010 | `mod core` 遮蔽内建 core crate |  | [`mod core` 遮蔽内建 core crate](010_`mod core` 遮蔽内建 core crate.md) |
| 011 | vue-tsc 与 TypeScript 7（native）包布局不兼容 |  | [vue-tsc 与 TypeScript 7（native）包布局不兼容](011_vue-tsc 与 TypeScript 7（native）包布局不兼容.md) |
| 012 | PSB 结构判定看签名（8BPB）而非版本号 |  | [PSB 结构判定看签名（8BPB）而非版本号](012_PSB 结构判定看签名（8BPB）而非版本号.md) |
| 013 | Tauri v2 非 async 命令在主线程执行——主线程内 build 窗口死锁 |  | [Tauri v2 非 async 命令在主线程执行——主线程内 build 窗口死锁](013_Tauri v2 非 async 命令在主线程执行——主线程内 build 窗口死锁.md) |
| 014 | 测试 exe 链接 tauri GUI 依赖链：0xc0000139 入口点未找到 |  | [测试 exe 链接 tauri GUI 依赖链：0xc0000139 入口点未找到](014_测试 exe 链接 tauri GUI 依赖链：0xc0000139 入口点未找到.md) |
| 015 | 视觉类目检用合成样本：口径缺陷被掩盖 |  | [视觉类目检用合成样本：口径缺陷被掩盖](015_视觉类目检用合成样本：口径缺陷被掩盖.md) |
| 016 | GIF 元数据探测用解码级遍历：导入大 GIF 卡数秒 |  | [GIF 元数据探测用解码级遍历：导入大 GIF 卡数秒](016_GIF 元数据探测用解码级遍历：导入大 GIF 卡数秒.md) |
| 017 | dev 构建性能误判：慢的是 O0 不是管线 |  | [dev 构建性能误判：慢的是 O0 不是管线](017_dev 构建性能误判：慢的是 O0 不是管线.md) |
| 018 | LGPL ffmpeg 构建没有 libx264：转码静默失败 |  | [LGPL ffmpeg 构建没有 libx264：转码静默失败](018_LGPL ffmpeg 构建没有 libx264：转码静默失败.md) |
| 019 | 增量链接缓存损坏：LNK2019 "无法解析的外部符号 anon…" 挡住 dev 启动 | 2026-07-28 | [增量链接缓存损坏：LNK2019 "无法解析的外部符号 anon…" 挡住 dev 启动](019_增量链接缓存损坏：LNK2019 -无法解析的外部符号 anon…- 挡住 dev 启动.md) |
| 020 | vite 起不来：listen EACCES——端口被 Windows NAT 动态保留段覆盖 |  | [vite 起不来：listen EACCES——端口被 Windows NAT 动态保留段覆盖](020_vite 起不来：listen EACCES——端口被 Windows NAT 动态保留段覆盖.md) |
| 021 | Vue SFC scoped 块内 `:global(x)` 带后代选择器被吞：拖拽时整窗黑屏 |  | [Vue SFC scoped 块内 `:global(x)` 带后代选择器被吞：拖拽时整窗黑屏](021_Vue SFC scoped 块内 `-global(x)` 带后代选择器被吞：拖拽时整窗黑屏.md) |
| 022 | 窗口几何持久化把"最小化态"写回：启动恢复成一小块 |  | [窗口几何持久化把"最小化态"写回：启动恢复成一小块](022_窗口几何持久化把-最小化态-写回：启动恢复成一小块.md) |
| 023 | dockview-vue 的 props 传递约定：整组包在一个 `params` prop 里 |  | [dockview-vue 的 props 传递约定：整组包在一个 `params` prop 里](023_dockview-vue 的 props 传递约定：整组包在一个 `params` prop 里.md) |
| 024 | dockview-vue 多根 Fragment + 父级 scoped 尺寸失效：dock 区域全黑且无报错 |  | [dockview-vue 多根 Fragment + 父级 scoped 尺寸失效：dock 区域全黑且无报错](024_dockview-vue 多根 Fragment + 父级 scoped 尺寸失效：dock 区域全黑且无报错.md) |
| 025 | dockview 自动布局回调持旧尺寸：同帧 forceLayout 被覆盖，网格渲染宽恒晚一帧 |  | [dockview 自动布局回调持旧尺寸：同帧 forceLayout 被覆盖，网格渲染宽恒晚一帧](025_dockview 自动布局回调持旧尺寸：同帧 forceLayout 被覆盖，网格渲染宽恒晚一帧.md) |
| 026 | 系统级指针抢夺：pointerup / pointercancel 双丢，手势视觉层永久残留 |  | [系统级指针抢夺：pointerup / pointercancel 双丢，手势视觉层永久残留](026_系统级指针抢夺：pointerup - pointercancel 双丢，手势视觉层永久残留.md) |
| 027 | 裸 tsc 不检查 .vue：SFC 自由变量 ReferenceError 全绿漏网 |  | [裸 tsc 不检查 .vue：SFC 自由变量 ReferenceError 全绿漏网](027_裸 tsc 不检查 .vue：SFC 自由变量 ReferenceError 全绿漏网.md) |
| 028 | `:hover:not()` 伪类计入特异性：悬停规则压过状态类，拖拽并入只见边框不见填充 |  | [`:hover:not()` 伪类计入特异性：悬停规则压过状态类，拖拽并入只见边框不见填充](028_`-hover-not()` 伪类计入特异性：悬停规则压过状态类，拖拽并入只见边框不见填充.md) |
| 029 | vite 热更新"假生效"：文件已改、页面跑旧码，重启 app 窗口也没用 | 2026-08-11 | [vite 热更新"假生效"：文件已改、页面跑旧码，重启 app 窗口也没用](029_vite 热更新-假生效-：文件已改、页面跑旧码，重启 app 窗口也没用.md) |
| 030 | 状态切换用 v-if 摘挂内容：空 flex 容器基线漂移，弹窗整体跳 1px |  | [状态切换用 v-if 摘挂内容：空 flex 容器基线漂移，弹窗整体跳 1px](030_状态切换用 v-if 摘挂内容：空 flex 容器基线漂移，弹窗整体跳 1px.md) |
