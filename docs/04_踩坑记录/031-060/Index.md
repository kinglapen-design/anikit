# 踩坑记录 031-060

> 四段式：现象 / 原因 / 解法 / 预防。共 30 条。返回 [Index](../Index.md)。

| 编号 | 标题 | 日期 | 链接 |
| --- | --- | --- | --- |
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
