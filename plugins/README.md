# plugins —— 动画软件插件

各动画软件的插件（适配器）按软件分子目录，通过本地 WebSocket 与 AniKit 主程序通信（见 PRD `07_动画软件通信`）。

现状：

- `animate-cep/` —— Animate CEP 插件（8.1 已落地）：选中元件导出（对齐舞台中央）→ WebP/fla 经 WS 入库（hidden 绑定）；开发期安装 = 目录内 `install.bat`（正式入口 = APP 工具栏插件钮 → CEP 导出设置弹窗一键安装）
- 面板样式 token 自 `src/styles/tokens.css` 拷贝（`css/tokens.css`）——token 变更时同步
- `obsidian-anikit-paste/` —— Obsidian 插件「AniKit 贴图直链」（零构建零依赖，纯 CommonJS）：编辑器伴侣插件，不经 WS，纯文件系统对接（见下节）

## obsidian-anikit-paste —— AniKit 贴图直链

Obsidian 里粘贴/拖入图片时不存 vault 附件夹，直接拷进 AniKit 自动导入收件箱（随后被 AniKit 移走入库），并当场在光标处插入链接：

``` markdown
[![{文件名}](http://127.0.0.1:{port}/item/by-name/{percent编码文件名})](anikit://item/by-name/{percent编码文件名})
```

内层 http 渲染图片（AniKit 媒体服务），外层 `anikit://` 点击拉起 AniKit 定位；设置里可切换为仅 http 图片形态（Typora 等环境用）。链接写入时资产尚未入库——by-name 由 AniKit 侧解析最新一笔，唯一名（`贴图-yyyyMMdd-HHmmss-fff.扩展名`，同秒撞名追加 `-2`）保确定性。

- 收件箱推导：`%APPDATA%\AniKit\config.json` → `spaces[0].path\自动导入`；端口：`%APPDATA%\AniKit\mcp-server.port`，缺失回退 17701；两处均可在插件设置覆盖收件箱目录
- 支持图片扩展名：png / jpg / jpeg / gif / webp / bmp / svg；无白名单图片时完全不干预 Obsidian 默认行为
- 安装：整个目录拷到 vault 的 `.obsidian/plugins/anikit-paste/` 下，Obsidian 设置 → 第三方插件中启用（桌面端限定）
