# 当前状态：preview 可视化 token 调参面板

2026-11-02 已落地 dev 调参面板：`#design-system` 页右下「调参」钮 → 340px 抽屉，tokens.css 全变量（新 + legacy）可视化编辑；px / ms 滑杆 + 数字、颜色色块 + 取色器、其余文本。改动实时写 `:root`（主 App 同文档同吃）+ localStorage 持久化（main.ts dev 启动恢复，生产零进入）；改动高亮 / 原值 / 单项与全部重置 / 搜索 / 分组 / 仅已改；导出覆盖 CSS（含原值注释）与 JSON，贴回 tokens.css 或发 AI 定稿。

内置浏览器目验通过（改值 / 高亮 / 持久化恢复 / 过滤 / 重置），typecheck 0 错，测试覆盖已清零。**复制在纯浏览器受 clipboardWrite 权限弹窗拦截**，已加导出原文 textarea 手动复制兜底；WebView2 剪贴板行为与主 App 实机调参观感待 `pnpm tauri dev` 走查。面板只产覆盖层，不自动改 tokens.css 源文件。

开发日志：[2026-11-02_02_preview_token调参面板](../../../02_开发日志/2026-11/2026-11-02_02_preview_token调参面板.md)。
