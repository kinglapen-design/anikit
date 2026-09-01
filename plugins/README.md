# plugins —— 动画软件插件

各动画软件的插件（适配器）按软件分子目录，通过本地 WebSocket 与 AniKit 主程序通信（见 PRD `07_动画软件通信`）。

现状：

- `animate-cep/` —— Animate CEP 插件（8.1 已落地）：选中元件导出（对齐舞台中央）→ WebP/fla 经 WS 入库（hidden 绑定）；开发期安装 = 目录内 `install.bat`（正式入口 = APP 工具栏插件钮 → CEP 导出设置弹窗一键安装）
- 面板样式 token 自 `src/styles/tokens.css` 拷贝（`css/tokens.css`）——token 变更时同步
