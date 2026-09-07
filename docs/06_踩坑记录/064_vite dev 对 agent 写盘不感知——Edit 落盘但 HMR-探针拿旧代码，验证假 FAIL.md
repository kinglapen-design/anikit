## 064 vite dev 对 agent 写盘不感知——Edit 落盘但 HMR/探针拿旧代码，验证假 FAIL

- **现象：** 2026-08-30（十八轮十三轮）。Edit 改 `src/App.vue`（mtime 已变 17:30:54）后，探针断言新 CSS 规则 FAIL；curl vite 的 style 子模块（加 `t=` 时间戳变体）仍返回旧内容。`touch` 文件后 vite 立即重转、新规则出现。
- **根因：** vite dev（v8.1.5，rolldown）的模块转换缓存失效依赖 fs watch 事件（chokidar）；agent 写盘方式（写临时文件 + 原子重命名）在 Windows 上 chokidar 漏 rename 事件——mtime 变了但无事件，vite 不重转（缓存被动失效，请求时不查 mtime）。
- **解法：** agent 改完前端文件后、跑探针或让用户看 HMR 前，先 `touch` 改动文件；存疑时用 curl 拉 vite 编译产物 grep 新标志确认（style 子模块 = `/src/Xxx.vue?vue&type=style&index=0&scoped=<hash>&lang.css`）。
- **预防：** 探针断言 FAIL 但代码明明已改时，先确认编译产物新旧再怀疑断言；探针复用用户 dev 实例（5373）时本坑生效，探针自起全新 vite 无此问题；实机验证让用户看效果前，确认 vite 终端已打 HMR 日志（或替用户 touch）。
