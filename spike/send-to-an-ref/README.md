# send-to-an-ref —— 旧 electron 版 JSFL 参考（8.3 发送至 AN）

2026-09-02 用户提供的旧 electron 版 JSFL 两份（非本项目产物，参考归档，不进主线）：

- `jsfl/export_lib.jsfl` —— 旧「导出到素材库」（Animate → 库方向，选中元件 → 复制 → 新文档 → 原位粘贴 → 缩放居中 → 导 PNG/SWF → undo 还原 → 存 fla + meta.json 坐标）。**提炼进主线的模式**：`fl.openDocument` / `fl.closeDocument(doc, false)` / `fl.setActiveWindow(origDoc)` 现场还原链；`tempDoc.undo()` 还原缩放技巧。
- `jsfl/batch-export-thumbnails.jsfl` —— 旧「批量导出缩略图」jsfl 直启脚本（任务文件驱动 + 进度文件轮询，PRD 07-02 免插件兜底路线同构）——**8.2（APP 内无伴生图批量导出）的直接参考素材**。

注：用户本想提供「发送至舞台」（打开-复制-粘贴-关闭）方向的旧脚本，两次放置均为导出方向，发送方向旧脚本未找到；8.3 兜底函数 `pasteFlaFile`（plugins/animate-cep/jsx/host.jsx）为本次新写，采纳了上述已开护栏模式（`fl.documents` 预判——已开文档用完不 closeDocument）。

（嵌套的 `jsfl/jsfl/` 为压缩包双层解压残留，内容与上层一致。）
