## 050 timeupdate 仅 ~250ms 粒度且拖动中回写覆盖目标值——scrub"连 UI 都卡"的体感与解码无关

- **现象：** 2026-08-22 晚回滚 `<video>` 后用户复验："scrub 不跟手、跳动感明显、切换慢，连 UI 都卡住了"；实测 seek 本体 5-40ms 无病。
- **原因：** ① 悬停 scrub 的底部进度条 / 帧号胶囊只走 `timeupdate` 事件更新——Chromium 每 ~250ms 才发一次，UI 以 4fps 跳变，肉眼即"卡"；② 播放条拖动中 `seek()` 乐观写入目标值后，`timeupdate` 持续用元素实际位置回写 store——快速拖动时进度条被往回拽（弹回）。
- **解法：** Eagle 式"进度条剥离"（用户点破：Eagle 进度条是剥离出来的，滑动流畅因 UI 不等解码）——拖动/scrub 期间 UI 以鼠标位置为准立即渲染：`previewMedia.scrubbing` 标志（pointerdown/up 置位，pointercancel 兜底）拖动中 timeupdate 不回写、松手对齐真实位置；HoverVideoLayer.onScrub 设 `v.currentTime` 后同步写本地 currentTime。
- **预防：** ① 媒体元素状态与拖动中 UI 状态分层：拖动中 UI 跟手势、元素异步追、timeupdate 类低频事件只在非拖动期同步；② 记牢 timeupdate 是 ~250ms 粒度不是逐帧——任何"跟手"诉求都不能经它转发；③ 修"卡"先分清是数据慢还是 UI 慢，两者药方完全不同。
