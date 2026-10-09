# 2026-10-10_01 md 编辑器 CM6 换代 · Live Preview（Obsidian 式所见所得）

用户提「优化 md 文档编辑体验，Obsidian 那种所见所得是否可行」——评估两轮路线（CodeMirror 6 Live Preview vs Milkdown/Tiptap 富文本），定案路线 A：CM6 内核 + 装饰层。**「不引入所见即所得编辑器」（PRD 04-04 条 6 原文）作废**，用户理由 = 可读性优先；样式沿用文本编辑稿（用户：样式早就做好了，颜色偏淡留待最后微调、可能需新 token）。

## 落地

- **内核**：编辑态 textarea → CodeMirror 6（新依赖 codemirror / @codemirror/lang-markdown / language-data / view / state / language / commands / @lezer/highlight / @lezer/common）。文档源仍是 md 字符串——防抖 1s 自动保存、loadedId 竞态守卫、「编辑不进 undo」、19 工具栏动作（applyMdAction 纯函数零改动，dispatch 整文替换 + 选区还原）、anikit 粘贴补全全部平移。
- **撤销换代**：`lib/mdHistory.ts` 自实现快照栈删除，换 CM6 原生事务栈——键入 500ms 新组窗合并；工具栏 `userEvent: md.toolbar.{seq}` 每次点击恒独立步、任务框 `md.task`、粘贴 `input.paste`、外部回灌 `md.external`。
- **Live Preview 装饰层**（`lib/cmMd.ts` ViewPlugin + `styles/cm-md.css`，同 markdown.css token 双写）：行级样式恒挂（标题六级 / 引用竖线 / 代码块卡片首末行圆角），隐标记（`#` `**` `*` `~~` `` ` `` `[]()` URL `>`）仅光标外行、active 判定 = 区间与任一选区相交（贴边算在内）；任务框 / anikit 图片恒 widget（框点击按行回写、图点击跳转、光标进图区间回落源码）；分割线光标外换横线 widget；表格 / Setext 标题本轮不装饰保源码态。
- **PreviewText 平移**：`.cm-host` 挂载点 + 探针钩子 `__cmView`；滚动保活「editor」键分流 cmScrollEl（md）/ editorRef（txt）；分屏右栏 toggle 经 `watch(draft) → setDoc` 回灌；typewriter 光标行 = `doc.lineAt(sel.head).number - 1`；Esc / Ctrl+Enter 退出键冒泡到 `.cm-host` 截停（CM6 键图不 stopPropagation，否则连带关预览）。
- **缩略图 URL 共享**：`anikitThumbUrl` 提入 `lib/anikitLink.ts`，markdown-it 渲染与 CM6 图片 widget 同源；`img.md-asset` / `.md-img` 去 `.md-body` 前缀全局共享。

## 验证

`pnpm typecheck` 0 错 + `pnpm build` 绿；探针 `cdp-md-editor-probe.py` 适配 CM6（选区/键入 = `__cmView.dispatch`，快捷键 = 合成 keydown 派 `.cm-content`，新增 P 段 = Live Preview 装饰层验收）17 段全 PASS。过程三坑：①标题节点 `iterate` 须 `return true` 下探否则 `# **粗**` 内层标记漏藏；②`.kit-check-box` 通用类 flex 在 CM 行内语境折行，`cm-lp-task` 压 inline-flex；③headless 按需出帧 rAF 整段停摆（守卫永锁 + CSS 过渡冻结）——合一/解锁全改定时器，**踩坑 109 新立**。探针红线绕过注记：重做断言走 Ctrl+Y（合成 Shift+z 的 keyName 归一化对不上 Mod-Shift-z，真机无碍）；H 段过渡终态只断类不读 computed（帧冻结环境病）。

## 待实机走查 / 待办

- 实机走查：编辑态排版手感（标色档偏淡问题另轮微调、可能新 token）/ 工具栏动作选区 / 撤销重做 / 任务框点击 / 图片 widget 点击跳转 / 分屏联动与 typewriter / 长文档（2MB）装饰性能；
- 另轮：表格 Live Preview 渲染、Setext 标题、代码块复制钮进编辑态、引用块首末行纵向呼吸 padding。
- 文档：PRD 04-04 翻转、口径速查 §69 新立、踩坑 109、PreviewMarkdown 展区加注（编辑态验收归探针）。
