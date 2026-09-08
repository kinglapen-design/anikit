# 阶段七 · 通用确认弹窗换代 ConfirmModal（AppConfirm 渲染层迁移，2026-09-08）

起因：移动面板换代后，用户实机截图发现落点确认窗仍是旧 AppConfirm（AppModal alert 壳 + 旧品牌青绿钮 + 48 黄徽章），指示「顺便换成最新的」。

## 落地

AppConfirm（selection.confirmPrompt 的唯一渲染宿主，App.vue 挂载）渲染层整体换 modals/ConfirmModal——组件/模态窗口/弹窗稿口径（520 宽 / 标题 18/26 Bold 一级 / 正文 13/18 三级 / 钮 120×38 次左主右 / 遮罩黑色-50% 点击不关闭）。confirmPrompt 的 9 个生产方（menuOps 6 / inboxOps / PropsDialog / MovePanel）字段零改，全量随宿主迁移：

- kind 映射：alert warning → warning（状态/警告钮）；alert error 或 danger → danger（状态/禁止钮）；无 alert 非危险 → brand（稿：不危险的提示换品牌色）。
- alert 提示音保留：AppConfirm 内 watch confirmPrompt 播 playAlert（旧 AppModal alert onMounted 口径）。
- bodySegments 高亮段保留 = --n-brand（新稿正文未画高亮，功能必需就近定档；旧 29 稿品牌色口径沿，色号随新品牌蓝）。
- ConfirmModal .body 补 word-break: break-all（沿用旧 AppConfirm .text 折行口径，长路径不顶出定宽）。
- 旧口径同废：AppConfirm min-width 88 组件化、.text max-width 372、AppModal alert 48 徽章（新稿无徽章）。

## 验证与波及

vue-tsc 0 错。cdp-move-panel-probe S3/S4/S6 改新壳断言（.kit-modal / .actions .btn / 次左主右「取消, 移动文件夹」/ warning 档 / 去徽章断言），S1-S6 全 PASS + 确认窗截图目验贴稿。复跑首轮曾拿旧模块空跑——vite watcher 漏变更既坑（速查 §12 探针桩纪律），批量 touch 后绿。

存量探针引用旧壳选择器（.app-modal.alert / .app-confirm）的一批标过期、随对应面板轮适配：cdp-archive / cdp-block-manage / cdp-custom-data / cdp-group(-stress) / cdp-sequence-pack / cdp-library-menu（多枚本在 2026-08-30 待办② 过渡态）。FailedListDialog 等 .app-modal.alert 直用方不走 confirmPrompt，不在此列。

速查 §24 末条新立（映射 + 探针波及清单）、§1 两条旧口径作废标注。
