# 2026-09-25_07_MarkDown 编辑器走查修订四条——编辑撤销栈 / 任务列表对齐 / 返回预览钮 / 工具栏悬停 tip

## 任务

用户实机走查 md 编辑器（当日 _06 轮）反馈四条：1) 没有接入撤销；2) 三个列表要对齐（有序无序是齐的，任务列表缩进太多）；3) 实时预览是不是比较难——没有实时预览怎么切换到预览；4) 工具悬停提示要给。

## 落地

**编辑撤销（`lib/mdHistory.ts` 自实现快照栈）**。根因：textarea 原生撤销只管连续键入，程序化修改（工具栏动作 / 复选框 toggle / v-model 同步）即断栈。组件会话级历史：编辑态 Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y 驱动（preventDefault 禁原生栈；全局 Ctrl+Z 本对输入框让位，不冲突）。记录三点：`@input` 键入（800ms 窗连续合并一步）、工具栏动作（独立步）、复选框 toggle（独立步——编辑态可回撤预览态的 toggle）。选区随快照恢复。与库撤销体系无关（「编辑不进 undo」口径不变）。

**任务列表对齐**。任务 li 撤掉额外缩进（与无序/有序同 content 左缘），复选框外挂 marker 列（`left: -20px` = 框 14 + 文距 6 组件 gap 档）——三列表文本列齐（探针实测三列同 363）、标记列齐（框 343 = 列 -20）。

**「返回预览」可见出口**。工具栏加第五组单钮：chevron-large 原样朝左（与预览工具栏返回钮同图标同语义），`action "preview"` 由 PreviewText 分流 exitEdit。实时预览已答用户：分屏（左码右渲）技术可行（rendered 本基于 draft 实时算）但 880 版心劈半太挤、分屏版式无稿；所见即所得 PRD 口径不做——先给可见出口，分屏若要另轮定版式（§60 待办）。

**悬停 tip**。19+1 钮全配 title（IconButton title prop 走 v-tip，内容容器轮同口径）。

**顺手两件**：chevron-large 烘焙 #808080 转 currentColor（命名对照十四批——2026-08-30 入库漏转，PreviewTitleBar 返回钮 / QuickNavButton 的 hover 升档一并未被发现，本轮治好）；vite dev watcher 静默丢变更坑实证（探针 FAIL 数据 = 旧样式，curl `/src/styles/markdown.css` 照发旧版——杀 dev 重启即愈，§60 探针坑补录）。

## 验证

- `pnpm typecheck` 绿。
- 探针 `tools/cdp-md-editor-probe.py` 扩至 12 段全绿：J 对齐（三文本列同 363 / 框 343 = marker 列）、I 撤销（撤工具栏步留键入 / 撤两步键入回初态 / 重做满）、K 返回预览钮（第五组 1 钮，点击回预览态 + 工具栏收）、L tip（hover bold 出「加粗」），A-H 原段无回归。
- 截图目验：三列表对齐与工具栏返回钮（md-preview / md-editing.png 重截）。

## 文档

速查 §60 追加「走修四条」条目（含撤销口径 / 对齐口径 / 返回预览钮 / tip 清单 / chevron-large 十四批 / vite watcher 坑）；待办同步：「退出仅靠惯例键」闭环（返回预览钮），分屏实时预览版式列待办。
