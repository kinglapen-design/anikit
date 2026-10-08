# 3. anikit:// 协议与媒体服务

**双形态分工（2026-10-08 定案并落地，PRD 02-09）：**

- `anikit://item/{ID}` / `anikit://folder/{ID}` —— **内部标识符**：复制链接 / 详情 links 字段 / 内部 MD 引用语法 / MCP 工具参数。ID 寻址改名/移动不死链。**系统协议已注册**（2026-10-08 deep-link 复活：启动时幂等覆写 HKCU\Software\Classes\anikit，免提权 portable 绿色、exe 漂移自愈）——外部软件点击 = 系统拉起 exe 带 URL 参数 → 单实例回调 → 运行中实例 `media-open` 跳转 + 窗口前置，**零浏览器标签页**；冷启动链接参数存 pending，前端就绪 + 库打开后消费（`take_pending_deep_link` 命令）。
- `http://127.0.0.1:{port}` —— **对外可点击形态**：本地媒体服务端点（`server/media.rs`，与 MCP 同 axum 实例 / 同端口，17701 优先、被占回退动态；复制 URL 时实际端口 ≠ 17701 提醒「端口被占，链接重启后可能失效」）。保留场景 = AI 客户端（渲染器只认 http 链接）+ deep-link 未注册环境兜底。
  - `GET /item/{ID}` —— 图片字节流（按 ID 实时解析磁盘路径，重命名/移动不死）。图片扩展名（png/jpg/jpeg/webp/gif/bmp/avif）→ origin 原图；其余 → webp 缩略图（含 thumbAssetId 伴生代理）；都没有 / 库未打开 / ID 不存在 = 404 纯文本。`Cache-Control: no-cache`（origin 外部编辑自愈口径）。外部 MD 编辑器 `![]()` 引用渲染用。
  - `GET /open/{kind}/{ID}`（kind = item | folder）—— 点击跳转：校验存在（404 纯文本兜底）→ `emit("media-open", { kind, id })` 前端**新开标签页定位**（`src/lib/anikitLink.ts`——2026-10-08 用户定：跳转不打断当前视口，已有同 viewKey 标签页激活复用不堆积）+ 主窗口前置（托盘同口径 `show_main_window`），回极简页「已在 AniKit 中定位，可关闭本页」。AI 回复 / 未注册 deep-link 环境回链用。

**归一化解析**：`core/anikit_ref.rs`（Rust：MCP 工具参数 / open_uri 生成）与 `src/lib/anikitLink.ts`（前端：渲染 / 粘贴 / 跳转）同逻辑——anikit:// 与 http(s)://127.0.0.1:{port}/(open/)item|folder/{id} 双形态 → 纯 id。

**外部编辑器兼容性（2026-10-08 实机走查补）**：

- **Obsidian**：http 图片正常渲染；anikit:// 链接首次点击弹「Open external link?」安全确认（其对一切外部协议的标准行为，Eagle eagle:// 同款）——勾「Always open anikit:// links」一次永久免弹。
- **VS Code**：预览安全级别 Strict（默认）拦截 http 本地图片（右上角横幅「已禁用此文档中的部分内容」，图片裂图）——点横幅切「允许不安全内容」即显示（每文档一次，或设置 `markdown.preview.security`）。参见 [vscode#32251](https://github.com/microsoft/vscode/issues/32251)。
- **Typora / AniKit 内部编辑器**：正常渲染（内部走 convertFileSrc 不经 http）。

**MD 打包导出**（`services/md_bundle.rs` + `commands/md_bundle.rs`，S6）：扫描文档双形态 item 引用（open/ 回链保留不改写）→ 图源同 /item/{id} 口径导出 {文档名}-打包/assets/（重名 _n）+ 引用改写相对路径。命令：`export_md_bundle(assetId, destDir)`（库内 md，目录选择器）/ `ws_export_md_bundle(absPath)`（ws md，落点文档旁），回包 `BundleReport { outDir, images, docFile, skipped[] }`；不改库数据不进撤销。
