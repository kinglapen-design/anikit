## 039 BtbN lgpl 构建 libwebp 只开 encoder：动图 webp 在 ffmpeg 管线无解

- **现象：** 2026-08-20 spike 实测 animated webp 解码失败：`webp_pipe` demuxer probe 报 `skipping unsupported chunk: ANIM/ANMF`、`Could not find codec parameters`；`find_by_name("libwebp_anim")` / `("libwebp")` 均 None。
- **原因：** 两层。① BtbN lgpl 构建 `--enable-libwebp` 只注册了 **encoder**（主线 sidecar 转 animated webp 用的就是它），libwebp / libwebp_anim **decoder 均缺席**；② ffmpeg 原生 webp decoder 本身不支持动画（ANIM/ANMF chunk 直接跳过，找不到静态图像数据即败），demuxer 层面就过不去——与 decoder 选型无关。
- **解法：** 动图 webp 不进 ffmpeg 管线，归主线既有 **image-webp** 纯 Rust 解码（`services/anim.rs` 生产在跑，逐帧 + delay 全量具备）；ffmpeg 管线管 GIF / 视频 / 其余格式。不为此换 GPL 构建（gyan full 含 libwebp decoder 但 GPL 传染）。
- **预防：** 评估"某格式能否进 ffmpeg 管线"要查三项：demuxer 支持、decoder 注册（`--list` 探针先跑）、构建 config（BtbN lgpl ≠ gyan full 的编码器清单）；sidecar 能**编码**不代表能**解码**。
