# 2026-09-25_09_MarkDown 编辑器三轮走查——滚轮合并联动 / 工具栏展开根治 / typewriter 跟滚居中

## 任务

用户实机三轮反馈（附截图）：1) 分屏两个滚轮都在右边还重叠，很奇怪——随后追问「两个滚动条合并成一个直接联动可以么」；2) 工具栏没展开（编辑态悬浮栏只露出两三个图标，被壳裁剪）；3) 分屏输入时右侧预览不跟滚、最下面一行被工具栏挡住——「书写的内容往往在最下面，能不能输入的时候预览的最后一行固定在中间」。

## 落地

**滚轮合并联动**。根因初判：ScrollThumb 用法铁律（必须与滚动容器同包裹层）被破坏——两个 thumb 同甩 `.text-preview` 根下，`absolute right: 0` 全贴到面板最右缘重叠。按用户「合并成一个」的定案直接做更好解：分屏改 `.pane` 包裹层后**只留单 ScrollThumb**（右栏渲染体主体，贴右缘），左 pane 无钮；双向**按比例联动**（`@scroll.passive` 互写 scrollTop，±1 阈值防往返震荡）。滚哪边另一边都跟随，一个滚动条管两栏。

**工具栏没展开根治**。根因：FloatBar 壳宽只在 mode / 显式 width / 搜索镜像变化时重测——editing 双态组数增减（预览 1 组 ↔ 编辑 5 组）内容宽变而壳宽不跟，5 组被 2 钮宽的壳裁剪（overflow hidden）。第一版 ResizeObserver 方案**已证无效并撤**：模式根盒宽恒 100% 随壳（`.fbm-root width:100%`），RO 观测不到内容变化。终案 = FloatBarMarkdown `watch(editing)` nextTick 后上报 `expanded-change`（复用搜索栏催宽通道，FloatBar 零改动）——内容自知组件上报才是正解。

**typewriter 跟滚 + 底部 66**。`lib/markdown.ts` 加 kitLineMap core rule：nesting=1 的块级 open token 一律挂 `data-line-start`（源码起始行；fence 包装 div 同挂）。输入 / keyup / click 时取光标行 → 右栏 `data-line-start <= 行` 的最近块 → 滚到**视口中间**（用户「预览最后一行固定中间」兑现；rAF 节流合一；精调期 linkLock 锁反向联动不拽左栏）。文档头尾块受滚动钳制物理居不了中属正常（探针改用中部行验真）。底部遮挡：预览态单栏渲染体与分屏右栏一律 66 补偿（§18 同档；全局 markdown.css 保持稿值底 0，补偿在 PreviewText scoped——docx 无工具栏语境不吃）。

## 验证

- `pnpm typecheck` 绿。
- 探针 `tools/cdp-md-editor-probe.py` 扩至 16 段全绿：D 段壳宽展开断言（预览 76 → 编辑全宽 + 内容不裁剪）、O 段单滚轮（thumb 恒 1）/ 正反向联动（左 0.5 → 右 0.5004 比例一致；右回 0 → 左跟回）/ typewriter（光标行 49 → 右栏 1106→771，命中块 centerOff 0 完美居中）；填充 30 独立段落撑滚动空间（单换行会被 md 折叠一段——测试数据坑一记）。A-O 全量无回归。
- 截图目验 `md-split.png`：工具栏 21 钮完整展开、双列实时同步。

## 文档

速查 §60 追加「三轮走查改版」条目（单滚轮联动口径 / ScrollThumb 包裹铁律复验 / RO 无效撤换组件上报 / data-line-start 与 typewriter 居中 / 底 66 新口径）。
