## 047 ffmpeg-next `frame.plane::<f32>()` 长度 = samples() 不含声道数：立体声交错只取一半（5.3 音频解码实测）

- **现象：** 2026-08-22 5.3 视频 PCM 音频链联调：重采样输出立体声 1098 样本/声道，按 `plane::<f32>(0)[..n*2]` 取交错数据直接越界 panic（range end index 2196 out of range for slice of length 1098）。
- **原因：** ffmpeg-next 的 `frame::Audio::plane<T>()` 切片长度恒 = `samples()`（每声道样本数），**不乘声道数**——packed 立体声的实际数据是 n×2 个 f32 交错，plane 只返回前一半。语义陷阱：名字像"整平面"，实现是"按样本计"。
- **解法：** 绕开 plane()：`frame.data(0)`（linesize 字节切片）→ `from_raw_parts(ptr as *const f32, n * 2)` 手工切（decoder.rs `f32_plane` 助手钉注释）；planar 格式另行按声道取 data(i)。
- **预防：** ffmpeg-next 音频帧读数据一律走 data()/字节层，plane()/plane_mut() 只在"每声道独立平面且逐声道处理"时按样本数理解；写 AV 解码新代码时先跑带立体声的单测（本坑由 audio_resample_chunks / decode_video_with_audio 两测试当场暴露）。
