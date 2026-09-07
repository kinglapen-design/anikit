## 042 ffmpeg-next scaler.run 输出帧 buffer 尺寸不符即报错丢帧：热改输出尺寸必须连输出帧一起重建

- **现象：** 2026-08-20 帧流出帧尺寸热更新（卡片跟随显示尺寸）首版：运行期改目标尺寸后后续帧全部消失（`no_upscale_and_live_resize` 用例只收到改尺寸前的帧）。
- **原因：** 解码循环复用同一个 `ffmpeg::frame::Video` 作 scaler 输出——首次 `scaler.run` 时 ffmpeg-next 按当时输出尺寸分配 buffer；目标尺寸热更新后只重建了 scaler，输出帧还是旧尺寸 buffer，`sws_scale` 写不进去，`run` 返回 Err，被"单帧失败跳过"逻辑静默吞掉 → 之后每帧都丢。
- **解法：** 目标尺寸变化重建 scaler 时**一并 `*rgb = Video::empty()` 重置输出帧**，下次 run 重新分配。
- **预防：** ffmpeg-next 的 frame 复用优化（避免每帧分配）只在**格式/尺寸不变**时成立；任何运行期可变参数（尺寸、像素格式）进 scaler 链路，输出帧与 scaler 必须同代重建。"帧突然消失但无 panic"先查被跳过的单帧错误。
