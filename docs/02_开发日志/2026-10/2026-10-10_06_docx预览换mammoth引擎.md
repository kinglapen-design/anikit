# 2026-10-10_06 docx 预览换 mammoth 引擎（表格 + 图片内嵌）

用户实机报 docx 预览「看不到表格、图片」——「公司反馈文档都是视频截图配文字」，初代 quick-xml 段落提取（表格平铺无框线、无图片）不达标。先答五条路线（结构提取 / docx→HTML / docx→PDF / 栅格化图片 / 系统预览处理器），用户拍板 mammoth。

## 落地

- **PRD 04-04 条 6 docx 条翻转**：预览 = mammoth.js 前端 docx→HTML（表格带框线 / 图片内嵌 / 标题 / 列表 / 粗斜体；浮动布局图片与文本框降级顺序内嵌、形状批注丢失为已知口径，真撞上再升 docx-preview——docxOps 封装层留换引擎位）；上限改文件 64MB（原 document.xml 32MB / 文本 2MB 口径随提取链路只留卡片用）。
- **后端**：新命令 `read_docx_binary`（原始二进制 IPC 响应，不经 JSON；is_docx 格式门 + 64MB 闸 + origin 自愈触点同 read_text_asset 口径）；`read_docx_asset` 存活——消费方收窄为库内文本卡正文懒加载（cardText.ts，卡片要轻文本不吃 mammoth）；`read_docx_by_path`（ws 文本卡）不动。
- **前端**：PreviewDocx.vue 重写 = readDocxBinary → docxToHtml → v-html 进 `.md-body`（mammoth 裸标签被 markdown.css 全局排版天然命中，只补图片限宽与块距）；`src/mammoth-shim.d.ts` 喂 mammoth 自带类型的 Buffer 引用（不引 @types/node）。
- **验证**：cargo test docx 模块 10 绿（新增 read_docx_bytes 字节一致 + 格式门）+ typecheck 0 错 + build 绿；探针 cdp-docx-preview-probe.py 重写（桩手搓真 docx：zip + 表格 + 内嵌 1px PNG）——标题 / 粗体 / 表格四格 / 图片 data URI / showImg 白名单 / 打开按钮 / png 回归全 PASS，截图目验贴稿。

## 待实机走查

真实反馈文档（截图配文字）观感 / 大文档（图多 10MB+）转换耗时 / 浮动图片文档的降级形态。图片若嫌占内存另轮改缓存目录 + convertFileSrc 引用（data URI 为 v1 口径）。
