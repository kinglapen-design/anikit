# 121 WebView2 Runtime 156 同 UDF 多 WebView 创建全灭——子窗口独立 UDF 隔离 + 先隐后显超时兜底（2026-11-02）

## 现象

用户报「多实例窗口打不开了，新建窗口、插件窗口都打不开」：主窗口正常，一切子窗口（插件详情三类 / 预览独立窗）点了没反应。日志（`%LOCALAPPDATA%/com.anikit.dev/logs/anikit.log`）每次开窗都有：

```
[plugin] 创建插件详情子窗口 plugin-mcp（transparent）
[tauri_runtime_wry][ERROR] failed to create webview: WebView2 error: HRESULT(0x8007139F)「组或资源的状态不是执行请求操作的正确状态。」
```

## 原因

**根因 = WebView2 Runtime 156.0.4314.58（2026-11-01 10:18 自动更新）起，与主窗口共享 user-data-dir 时第二个 WebView 创建报 0x8007139F**（同 UDF 多 CoreWebView2Environment 冲突，[Microsoft Q&A 同症](https://learn.microsoft.com/en-us/answers/questions/5986053/webview2-stops-working-after-2-3-days-with-hresult)）。同日 14:20 首条报错 = 更新后数小时，早于当日任何代码改动——不是项目回归，是系统级外部变量。

排查走过一段弯路先记：**透明与否无关**——跳过 transparent(true) 改 build(false) 后同样 0x8007139F（dev 日志实证），「透明创建全灭」的初判被证伪后才锁定 UDF 共享。

两个应用侧机制把「创建失败」放大成「窗口打不开」的隐形故障：

1. **透明降级链不触发**：wry 报 ERROR 后 **build(true) 仍返回 Ok**——「失败降级非透明重建」分支从未进入（全日志零条「降级」记录），「失败会返回 Err」的设计假设不成立。
2. **先隐后显无兜底**：`visible(false)` + `on_page_load(Finished → show)` 下 webview 没创建成功时 Finished 永不触发，窗口对象存在但永不显示，用户只能看到「点了没反应」。

**附带发现（独立小 bug）**：子窗口 `PreviewWindow onMounted` 即 `content.refresh()`，**启动早期 AppState 库未就绪时吃「请先打开素材库」**（用户实机截图坐实）——主窗口有库就绪等待（deep-link pending 同口径），子窗口没有。

## 解法

`commands/preview.rs open_child_window`：

- **每 label 独立 user-data-dir**（`child_data_dir` = `app_local_data_dir/EBWebView-children/{label}`），主窗口与子窗口、各子窗口彼此 UDF 全隔离，Runtime 156 下同 UDF 冲突根除；透明首试 + 非透明降级链保留（视觉圆角不变）。
- **先隐后显加 3 秒超时强制 show 兜底**（已 show 幂等无感）——此后任何加载失败类故障窗口都可见可关，不再隐形。

`PreviewWindow.vue`：**`refreshWhenLibraryReady()`**——仅拦「请先打开素材库」错 200ms×50 重试（其余错误原样抛），子窗口启动竞态自愈。

验证（dev 实例，CDP 9333）：插件窗真实渲染 MCP 全内容（含 CLI 组）、预览窗正确 assetKey 后杰森大图正常显示，两窗日志再无 wry ERROR。

## 预防

- **发版排障第一步 = 看日志文件**（设置面板 → 关于 → 查看软件日志）——wry 层 ERROR 只有这里有，主窗口 UI 不会呈现。
- **「先隐后显 / 先备后显」类模式必须配超时或失败兜底**，否则任何加载失败都会把故障变成「什么都没发生」。
- **WebView2 Runtime 自动更新是不可控外部变量**：版本目录安装时间（`C:\Program Files (x86)\Microsoft\EdgeWebView\Application\`）是首选对照证据；先区分「代码回归」与「Runtime 更新」再决定改不改代码。
- **UDF 隔离的副作用要知道**：子窗口变独立 browser process——主窗口 CDP `/json` 看不到子窗口 target（探针断言不能查 target，要查 Windows 窗口标题 / 截屏）；`document.title` 不改变原生窗口标题（builder 定），探针状态要写 DOM 截屏读；`open_preview_window` 的 assetKey = **`${assetId}|${folderId}`**（placementKey，非纯 assetId——传纯 id 会 current null 黑屏，属于探针参数错不是产品 bug）。
- Runtime 修复共享 UDF 创建后，可考虑回滚为共享 UDF（隔离目录 `EBWebView-children/{label}` 会逐窗累积，若长期保留需评估清理策略——本轮先记待办）。
