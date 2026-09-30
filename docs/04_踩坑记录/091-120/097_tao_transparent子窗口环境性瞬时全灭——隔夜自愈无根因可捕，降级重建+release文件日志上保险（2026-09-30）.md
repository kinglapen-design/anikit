# 097 tao transparent 子窗口环境性瞬时全灭——隔夜自愈无根因可捕，降级重建 + release 文件日志上保险（2026-09-30）

## 现象

朋友实机（实体 Win11 25H2 build 26200，非虚拟机，与开发者机同系统版本）测试打包版：主窗口正常，**所有子窗口全灭**——预览「新窗口打开」、插件详情窗（Animate / MCP）点击零反应：无窗口（任务栏 / Alt+Tab 均无）、无闪退、无任何提示。开发者机同打包版一切正常。次日朋友再试**未改任何代码自愈**，云母子窗口全正常。

## 原因

tao `transparent(true)` 子窗口（layered 窗口 hack）环境敏感，在朋友机上出现**环境性瞬时创建失败**，与机器配置、系统版本无关（隔夜自愈 + 次日新版日志实证 transparent 创建成功、云母 set_effects 成功）。具体失败环境因子已不可考（故障窗口期无日志——这正是本坑最痛的一点）。

「零提示零线索」是三重缺陷叠加：①预览路径菜单 `void invoke("open_preview_window")` 五处静默吞错；②release 构建日志插件未启用（`cfg!(debug_assertions)` 闸），发版排障零日志；③release/ 目录命名混乱（无版本号、新旧包并存）致用户一度以为发错版本，排障方向反复。

## 解法（2026-09-29 兼容轮，止血 + 保险，d0c8f876）

- preview.rs 抽 `open_child_window` 共用开窗（plugin.rs 同用）：transparent 优先，build 失败**降级 transparent(false) 重建**（视觉退实色方角，功能可用），原始错误全程进日志；降级窗 URL 追加 `&fx=solid` 锁实色底（非透明无云母载体，防 mica 主模式切透明底露白底）。
- 前端 `lib/previewOps.ts` 封装 `openPreviewWindow`（失败 toast），替换 assetCard×3 / recycle / PreviewTitleBar 五处 `void invoke`。
- lib.rs 日志插件 release 同样启用（Stdout + LogDir，5MB 轮转留 2 份，**`%LOCALAPPDATA%\com.anikit.dev\logs\anikit.log`**——Tauri v2 Windows app_log_dir 在 Local 非 Roaming，一度按 %APPDATA% 让朋友白找）。
- release/ 治理：发包 zip 命名规则立为 `anikit-v{版本}-{日期}.zip`，旧包归 `_archive/`。

## 预防

- **发版排障第一原则 = 文件日志**：此后 release 恒有 `logs/anikit.log`，故障窗口期不再裸奔（本次靠「隔夜自愈 + 朋友恰好发回日志」运气收场，根因永远灭失）。
- invoke 一律走封装 toast，`void invoke` 吞错只许用于纯读且失败无害场景。
- 发包命名 `anikit-v{版本}-{日期}.zip`，release/ 只留「现行 zip + 现行目录 + _archive」。
- 治本（子窗口对齐主窗口 transparent:false + 框架延伸方案）挂起观察：降级 + 日志保险已就位，日后再有同类报案**且有 error 日志**时启动。
