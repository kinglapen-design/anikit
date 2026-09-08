## 072 flex-basis 定值项不进父容器 max-content——width 定值才有贡献，FloatBar 探针实测丢稿值宽（2026-09-09）

- **现象：** 预览悬浮栏接入轮，FloatBarPlay 进度条区设 `flex: 1 1 327px`（327 = 播放条稿值），FloatBar 壳的 inline `max-content` 探针实测宽度把 327 整个丢掉——视频播放条壳实测量 357（只含固定件），进度条被挤成 0 宽、拖钮贴死时间码。改 `flex: 0 1 327px`（shrink 1 不变、只去 grow）依旧丢宽。
- **原因：** Blink 下 flex 容器求 max-content 时，**`flex-basis` 为确定值且 `flex-shrink > 0` 的项，其 max-content 贡献只报内容固有尺寸**（basis 不进贡献——可压项的贡献按内容算）；`flex-grow > 0` 同样只报内容固有尺寸。BarSlider 轨道宽 100%、拖钮 absolute，内容固有宽 = 0 → 贡献 0。一次性实验 `tools/cdp-flex-contrib-exp.py` 实测定案：`flex:0 1 327px` → 贡献 0；`flex:0 0 327px` / `width:327px` / `min-width:327px` / 内层定宽子 → 贡献 327。
- **解法：** 进度条区改 **`width: 327px` + 默认 `flex: 0 1 auto` + `min-width: 0`**（FloatBarPlay `.fbp-progress`）——width 定值有 max-content 贡献（探针拿到 327 稿值链），shrink 1 + min-width 0 保窄壳可压缩。同类排查信号：FloatBar 探针（或任何 max-content 实测）读出的宽比预期小一整块 = 某 flex 项 basis 定值被丢。
- **预防：** **凡是参与 max-content 实测链的 flex 项，宽度用 `width` 定值，不用 `flex-basis` 定值**（grow>0 或 shrink>0 都会丢）；要 grow 吃余量的场景另外靠父容器定宽（壳宽 js 钉死时 grow 本无意义）。实验脚本留档 `tools/cdp-flex-contrib-exp.py` 可复验。
