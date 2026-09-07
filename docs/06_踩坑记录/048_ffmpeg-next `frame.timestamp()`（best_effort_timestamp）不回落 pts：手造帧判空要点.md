## 048 ffmpeg-next `frame.timestamp()`（best_effort_timestamp）不回落 pts：手造帧判空要点

- **现象：** 2026-08-22 重采样单测手造 Audio 帧 `set_pts(Some(...))` 后 `timestamp()` 恒 None，push_audio 早退、0 块产出（测试全灭但无报错）。
- **原因：** 新版 FFmpeg 的 `best_effort_timestamp` 是 AVFrame 独立字段（解码器写），`av_frame_get_best_effort_timestamp` 只读它、**不再回落 pts**；手造帧 set_pts 写的是 pts 字段，两字段互不通。
- **解法：** 读时刻统一 `frame.timestamp().or_else(|| frame.pts())`（生产解码器恒有 best_effort，回落只为测试与怪容器兜底）。
- **预防：** 凡是"读帧时刻"的代码都按双字段兜底写；测试手造帧用 set_pts 即可覆盖同一条路径。
