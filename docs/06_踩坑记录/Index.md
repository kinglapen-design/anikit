# 06_踩坑记录 Index

# 06_踩坑记录

> 四段式：现象 / 原因 / 解法 / 预防。踩到即记，写完把能沉淀的预防措施补进 `01_开发规范`。

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
| 031 | serde 内部 tag 枚举：`rename_all` 只改变体名，不改字段名 | 2026-08-13 | [serde 内部 tag 枚举：`rename_all` 只改变体名，不改字段名](031_serde 内部 tag 枚举：`rename_all` 只改变体名，不改字段名.md) |
| 032 | 拖拽僵尸会话：无 pointer capture 丢 pointerup，旧 sess 被静默复用吞 click | 2026-08-13 | [拖拽僵尸会话：无 pointer capture 丢 pointerup，旧 sess 被静默复用吞 click](032_拖拽僵尸会话：无 pointer capture 丢 pointerup，旧 sess 被静默复用吞 click.md) |
| 033 | cargo 增量缓存损坏 LNK2019（Windows MSVC，反复出现） | 2026-08-13 | [cargo 增量缓存损坏 LNK2019（Windows MSVC，反复出现）](033_cargo 增量缓存损坏 LNK2019（Windows MSVC，反复出现）.md) |
| 034 | std::process::Command 自动引号转义 vs explorer 原始命令行解析（/select 回退"文档"） | 2026-08-14 | [std::process::Command 自动引号转义 vs explorer 原始命令行解析（/select 回退"文档"）](034_std--process--Command 自动引号转义 vs explorer 原始命令行解析（-select 回退-文档-）.md) |
| 035 | pointer capture 挂太早：begin 即 capture 会把 dblclick 重定向，名称区双击改名被吞成"打开" | 2026-08-18 | [pointer capture 挂太早：begin 即 capture 会把 dblclick 重定向，名称区双击改名被吞成"打开"](035_pointer capture 挂太早：begin 即 capture 会把 dblclick 重定向，名称区双击改名被吞成-打开-.md) |
| 036 | 描边颜色过渡 = 重绘型动画：GPU 合成损伤区漏覆盖，取消状态后品牌色像素残留 |  | [描边颜色过渡 = 重绘型动画：GPU 合成损伤区漏覆盖，取消状态后品牌色像素残留](036_描边颜色过渡 = 重绘型动画：GPU 合成损伤区漏覆盖，取消状态后品牌色像素残留.md) |
| 037 | 异步 watch 窗口期旧布局越界：Vue 渲染管道被异常打"半死"——点了有反应（store 变）但 DOM 不更新 | 2026-08-19 | [异步 watch 窗口期旧布局越界：Vue 渲染管道被异常打"半死"——点了有反应（store 变）但 DOM 不更新](037_异步 watch 窗口期旧布局越界：Vue 渲染管道被异常打-半死-——点了有反应（store 变）但 DOM 不更新.md) |
| 038 | ffmpeg-next 与 ffmpeg 头文件版本漂移：bindgen 现场生成，crate feature 门槛表滞后于滚动 master | 2026-08-20 | [ffmpeg-next 与 ffmpeg 头文件版本漂移：bindgen 现场生成，crate feature 门槛表滞后于滚动 master](038_ffmpeg-next 与 ffmpeg 头文件版本漂移：bindgen 现场生成，crate feature 门槛表滞后于滚动 master.md) |
| 039 | BtbN lgpl 构建 libwebp 只开 encoder：动图 webp 在 ffmpeg 管线无解 | 2026-08-20 | [BtbN lgpl 构建 libwebp 只开 encoder：动图 webp 在 ffmpeg 管线无解](039_BtbN lgpl 构建 libwebp 只开 encoder：动图 webp 在 ffmpeg 管线无解.md) |
| 040 | Windows std::thread::sleep 粒度 15.6ms：30fps 帧节奏顶不满（实测 25.6） | 2026-08-20 | [Windows std::thread::sleep 粒度 15.6ms：30fps 帧节奏顶不满（实测 25.6）](040_Windows std--thread--sleep 粒度 15.6ms：30fps 帧节奏顶不满（实测 25.6）.md) |
| 041 | dev server 两坑连环：vite watcher 丢文件变更发旧版 SFC + HMR 整页刷新把 CDP 探针挂死 | 2026-08-20 | [dev server 两坑连环：vite watcher 丢文件变更发旧版 SFC + HMR 整页刷新把 CDP 探针挂死](041_dev server 两坑连环：vite watcher 丢文件变更发旧版 SFC + HMR 整页刷新把 CDP 探针挂死.md) |
| 042 | ffmpeg-next scaler.run 输出帧 buffer 尺寸不符即报错丢帧：热改输出尺寸必须连输出帧一起重建 | 2026-08-20 | [ffmpeg-next scaler.run 输出帧 buffer 尺寸不符即报错丢帧：热改输出尺寸必须连输出帧一起重建](042_ffmpeg-next scaler.run 输出帧 buffer 尺寸不符即报错丢帧：热改输出尺寸必须连输出帧一起重建.md) |
| 043 | ShellExecuteW "openas" 动词未注册返回 31（SE_ERR_NOASSOC）：「打开方式」对话框要备 rundll32 兜底 | 2026-08-20 | [ShellExecuteW "openas" 动词未注册返回 31（SE_ERR_NOASSOC）：「打开方式」对话框要备 rundll32 兜底](043_ShellExecuteW -openas- 动词未注册返回 31（SE_ERR_NOASSOC）：「打开方式」对话框要备 rundll32 兜底.md) |
| 044 | CDP 探针按 title 属性找不到按钮：IconButton 的 title prop 走 v-tip 指令，不落 title attribute | 2026-08-20 | [CDP 探针按 title 属性找不到按钮：IconButton 的 title prop 走 v-tip 指令，不落 title attribute](044_CDP 探针按 title 属性找不到按钮：IconButton 的 title prop 走 v-tip 指令，不落 title attribute.md) |
| 045 | tauri dev 失败退出后 vite 孤儿进程残留占 5373：重跑 dev 报 Port is already in use | 2026-08-20 | [tauri dev 失败退出后 vite 孤儿进程残留占 5373：重跑 dev 报 Port is already in use](045_tauri dev 失败退出后 vite 孤儿进程残留占 5373：重跑 dev 报 Port is already in use.md) |
| 046 | 回收站卡片按下交框选回调：pointerdown preventDefault 吞 click 合成，单击选中全灭（探针合成 click 绕过 pointerdown 链，掩盖真实缺陷） | 2026-08-21 | [回收站卡片按下交框选回调：pointerdown preventDefault 吞 click 合成，单击选中全灭（探针合成 click 绕过 pointerdown 链，掩盖真实缺陷）](046_回收站卡片按下交框选回调：pointerdown preventDefault 吞 click 合成，单击选中全灭（探针合成 click 绕过 pointerdown 链，掩盖真实缺陷）.md) |
| 047 | ffmpeg-next `frame.plane::<f32>()` 长度 = samples() 不含声道数：立体声交错只取一半（5.3 音频解码实测） | 2026-08-22 | [ffmpeg-next `frame.plane::<f32>()` 长度 = samples() 不含声道数：立体声交错只取一半（5.3 音频解码实测）](047_ffmpeg-next `frame.plane---f32-()` 长度 = samples() 不含声道数：立体声交错只取一半（5.3 音频解码实测）.md) |
| 048 | ffmpeg-next `frame.timestamp()`（best_effort_timestamp）不回落 pts：手造帧判空要点 | 2026-08-22 | [ffmpeg-next `frame.timestamp()`（best_effort_timestamp）不回落 pts：手造帧判空要点](048_ffmpeg-next `frame.timestamp()`（best_effort_timestamp）不回落 pts：手造帧判空要点.md) |
| 049 | 性能/体感结论必须真机数据先行——仿真探针全绿而实机卡死（5.3 裸帧流一日弯路的总根因） | 2026-08-22 | [性能/体感结论必须真机数据先行——仿真探针全绿而实机卡死（5.3 裸帧流一日弯路的总根因）](049_性能-体感结论必须真机数据先行——仿真探针全绿而实机卡死（5.3 裸帧流一日弯路的总根因）.md) |
| 050 | timeupdate 仅 ~250ms 粒度且拖动中回写覆盖目标值——scrub"连 UI 都卡"的体感与解码无关 | 2026-08-22 | [timeupdate 仅 ~250ms 粒度且拖动中回写覆盖目标值——scrub"连 UI 都卡"的体感与解码无关](050_timeupdate 仅 ~250ms 粒度且拖动中回写覆盖目标值——scrub-连 UI 都卡-的体感与解码无关.md) |
| 051 | WebView2 远程调试：WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS 被 wry 显式参数覆盖，CDP 端口须走 tauri.conf additionalBrowserArgs | 2026-08-22 | [WebView2 远程调试：WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS 被 wry 显式参数覆盖，CDP 端口须走 tauri.conf additionalBrowserArgs](051_WebView2 远程调试：WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS 被 wry 显式参数覆盖，CDP 端口须走 tauri.conf additionalBrowserArgs.md) |
| 052 | image crate 0.25 按扩展名预设解码格式（假扩展名文件 InvalidSignature） | 2026-08-23 | [image crate 0.25 按扩展名预设解码格式（假扩展名文件 InvalidSignature）](052_image crate 0.25 按扩展名预设解码格式（假扩展名文件 InvalidSignature）.md) |
| 053 | CDP 探针桩返回可变 DB 集合必须深拷贝——共享引用绕过 Vue 响应式（双 item / 编辑不刷新假象） | 2026-08-23 | [CDP 探针桩返回可变 DB 集合必须深拷贝——共享引用绕过 Vue 响应式（双 item / 编辑不刷新假象）](053_CDP 探针桩返回可变 DB 集合必须深拷贝——共享引用绕过 Vue 响应式（双 item - 编辑不刷新假象）.md) |
| 054 | 后台 `pnpm dev` 收工后 vite 子进程成孤儿占端口（TaskStop 只杀 pnpm 壳） | 2026-08-28 | [后台 `pnpm dev` 收工后 vite 子进程成孤儿占端口（TaskStop 只杀 pnpm 壳）](054_后台 `pnpm dev` 收工后 vite 子进程成孤儿占端口（TaskStop 只杀 pnpm 壳）.md) |
| 055 | grid `0fr→1fr` 宽度推挤动画只适用外部定宽容器——自适应容器里 0fr 轨道仍占 max-content 宽度 | 2026-08-29 | [grid `0fr→1fr` 宽度推挤动画只适用外部定宽容器——自适应容器里 0fr 轨道仍占 max-content 宽度](055_grid `0fr→1fr` 宽度推挤动画只适用外部定宽容器——自适应容器里 0fr 轨道仍占 max-content 宽度.md) |
| 056 | CDP 探针连上历史僵尸 headless Edge——固定调试端口 + terminate 只杀父进程（旧页面旧模块鬼状态） | 2026-08-29 | [CDP 探针连上历史僵尸 headless Edge——固定调试端口 + terminate 只杀父进程（旧页面旧模块鬼状态）](056_CDP 探针连上历史僵尸 headless Edge——固定调试端口 + terminate 只杀父进程（旧页面旧模块鬼状态）.md) |
| 057 | 云母 Mica 接入：`show()` 前 `set_effects` 吞掉 WebView2 全部内容（只剩云母渐变） | 2026-08-30 | [云母 Mica 接入：`show()` 前 `set_effects` 吞掉 WebView2 全部内容（只剩云母渐变）](057_云母 Mica 接入：`show()` 前 `set_effects` 吞掉 WebView2 全部内容（只剩云母渐变）.md) |
| 058 | flex/grid 容器的 trailing gap 被 Chromium 计入 scrollHeight——高度展开动画终值偏大，切回 auto 瞬间兄弟回弹 | 2026-08-30 | [flex/grid 容器的 trailing gap 被 Chromium 计入 scrollHeight——高度展开动画终值偏大，切回 auto 瞬间兄弟回弹](058_flex-grid 容器的 trailing gap 被 Chromium 计入 scrollHeight——高度展开动画终值偏大，切回 auto 瞬间兄弟回弹.md) |
| 059 | trailing gap 横向同款：dockview 自绘横向滚动条因 scrollWidth 假溢出 4px 横贯标签组底部 | 2026-08-30 | [trailing gap 横向同款：dockview 自绘横向滚动条因 scrollWidth 假溢出 4px 横贯标签组底部](059_trailing gap 横向同款：dockview 自绘横向滚动条因 scrollWidth 假溢出 4px 横贯标签组底部.md) |
| 060 | 容器 overflow 裁到内容盒——标签页反弧肩膀（外溢伪元素）首尾枚外侧恒被裁掉 | 2026-08-30 | [容器 overflow 裁到内容盒——标签页反弧肩膀（外溢伪元素）首尾枚外侧恒被裁掉](060_容器 overflow 裁到内容盒——标签页反弧肩膀（外溢伪元素）首尾枚外侧恒被裁掉.md) |
| 061 | dockview 标签溢出下拉（tabs overflow list）在 DPR 缩放环境下假溢出恒显，压占 + 钮右端 | 2026-08-30 | [dockview 标签溢出下拉（tabs overflow list）在 DPR 缩放环境下假溢出恒显，压占 + 钮右端](061_dockview 标签溢出下拉（tabs overflow list）在 DPR 缩放环境下假溢出恒显，压占 + 钮右端.md) |
| 062 | rAF 引擎的「进行中」布尔无兜底——窗口遮挡/headless 节流下动画链暂停，状态恒卡 true | 2026-08-30 | [rAF 引擎的「进行中」布尔无兜底——窗口遮挡/headless 节流下动画链暂停，状态恒卡 true](062_rAF 引擎的「进行中」布尔无兜底——窗口遮挡-headless 节流下动画链暂停，状态恒卡 true.md) |
| 063 | 主窗口云母边框五轮实录：DWM/CSS 双画恒错位双层、双裁切 AA 透缝、直角线被角弧吞断、纯 DWM 画边角弧 AA 咬内容留缺口——终态 = DWM 画 + 前端同色遮罩环淹没 AA 过渡带 | 2026-08-30 | [主窗口云母边框五轮实录：DWM/CSS 双画恒错位双层、双裁切 AA 透缝、直角线被角弧吞断、纯 DWM 画边角弧 AA 咬内容留缺口——终态 = DWM 画 + 前端同色遮罩环淹没 AA 过渡带](063_主窗口云母边框五轮实录：DWM-CSS 双画恒错位双层、双裁切 AA 透缝、直角线被角弧吞断、纯 DWM 画边角弧 AA 咬内容留缺口——终态 = DWM 画 + 前端同色遮罩环淹没 AA 过渡带.md) |
| 064 | vite dev 对 agent 写盘不感知——Edit 落盘但 HMR/探针拿旧代码，验证假 FAIL | 2026-08-30 | [vite dev 对 agent 写盘不感知——Edit 落盘但 HMR/探针拿旧代码，验证假 FAIL](064_vite dev 对 agent 写盘不感知——Edit 落盘但 HMR-探针拿旧代码，验证假 FAIL.md) |
| 065 | 伪元素当背景层必须沉底：absolute 无 z-index = 层叠第 6 层盖正常流内容（2026-08-31） | 2026-08-31 | [伪元素当背景层必须沉底：absolute 无 z-index = 层叠第 6 层盖正常流内容（2026-08-31）](065_伪元素当背景层必须沉底：absolute 无 z-index = 层叠第 6 层盖正常流内容（2026-08-31）.md) |
| 066 | 存在性判断误用 `iter().all(==)`——空表/全匹配两极端都成立，懒建变重复建档（2026-09-01） | 2026-09-01 | [存在性判断误用 `iter().all(==)`——空表/全匹配两极端都成立，懒建变重复建档（2026-09-01）](066_存在性判断误用 `iter().all(==)`——空表-全匹配两极端都成立，懒建变重复建档（2026-09-01）.md) |
| 067 | 前后端「同一约定」必须含长宽比——取景框方形化后烘培仍出 13:10 = 确认后二次裁剪（2026-09-01） | 2026-09-01 | [前后端「同一约定」必须含长宽比——取景框方形化后烘培仍出 13:10 = 确认后二次裁剪（2026-09-01）](067_前后端「同一约定」必须含长宽比——取景框方形化后烘培仍出 13-10 = 确认后二次裁剪（2026-09-01）.md) |
| 068 | ffmpeg libwebp 动画编码差分帧产物本身带残影——解码端缺陷定性只是冰山一角（2026-09-02） | 2026-09-02 | [ffmpeg libwebp 动画编码差分帧产物本身带残影——解码端缺陷定性只是冰山一角（2026-09-02）](068_ffmpeg libwebp 动画编码差分帧产物本身带残影——解码端缺陷定性只是冰山一角（2026-09-02）.md) |
| 069 | CEP evalScript 周期性执行 = 打断宿主输入态——面板轮询模型本身不能要（2026-09-02） | 2026-09-02 | [CEP evalScript 周期性执行 = 打断宿主输入态——面板轮询模型本身不能要（2026-09-02）](069_CEP evalScript 周期性执行 = 打断宿主输入态——面板轮询模型本身不能要（2026-09-02）.md) |
