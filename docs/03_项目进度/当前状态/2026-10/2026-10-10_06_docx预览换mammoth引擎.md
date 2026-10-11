# 当前状态：docx 预览换 mammoth 引擎（2026-10-10）

用户实机报 docx 预览看不到表格 / 图片（反馈文档 = 视频截图配文字场景不达标）→ 拍板 mammoth：预览 = `read_docx_binary`（原始字节 IPC，64MB 闸 + 自愈触点）→ 前端 mammoth.js 转 HTML 进 `.md-body`（表格带框线 / 图片内嵌 / 标题列表粗斜体；浮动图片与文本框降级顺序内嵌为已知口径，docx-preview 留升级位）；`read_docx_asset` 收窄为库内文本卡正文提取。cargo 10 绿 + typecheck / build 绿 + 探针重写全 PASS（桩手搓真 docx 含表格 + 内嵌 PNG）。**待实机走查：真实反馈文档观感 / 大图文档转换耗时**。详见 [开发日志 2026-10-10_06](../../02_开发日志/2026-10/2026-10-10_06_docx预览换mammoth引擎.md)。
