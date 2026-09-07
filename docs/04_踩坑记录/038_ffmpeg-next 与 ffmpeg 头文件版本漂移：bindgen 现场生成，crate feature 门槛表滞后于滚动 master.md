## 038 ffmpeg-next 与 ffmpeg 头文件版本漂移：bindgen 现场生成，crate feature 门槛表滞后于滚动 master

- **现象：** 2026-08-20 阶段四 spike，ffmpeg-next 8.1.0 配 BtbN 最新 master dev 库（avcodec 63 = 9.0-dev）编译 15 错（`AV_PKT_DATA_DYNAMIC_HDR_SMPTE_2094_APP5` 等枚举未覆盖）；换 autobuild-2025-08-31 库（avcodec 62.13，ffmpeg 8.0 时期）后仍 1 错：`AV_FRAME_DATA_EXIF not covered`。
- **原因：** ffmpeg-sys-next 是 **bindgen 按本机头文件现场生成绑定**——头文件里的新枚举值会出现在生成代码里，而 ffmpeg-next 高层代码的 match 臂由 `ffmpeg_X_Y` feature 门控，feature 又按**版本号门槛表**（如 `("ffmpeg_8_1", 62, 28)` = avcodec major 62 且 minor ≥ 28）探测。EXIF frame side data 实际在 avcodec 62.13 已合入，crate 作者按 62.28 写的门槛 → bindgen 产物含枚举、高层 match 缺臂，E0004。
- **解法：** 两步——① 按 crate 同期挑 dev 库（BtbN 无版本化 tag，只有 master 滚动 + 月末 autobuild：autobuild-2025-08-31 即 ffmpeg 8.0 发布后 9 天）；② 残余个别枚举用 `[patch.crates-io]` 本地副本（`patches/ffmpeg-next`），仅把 `src/util/frame/side_data.rs` 三处 cfg `ffmpeg_8_1`→`ffmpeg_8_0`（其余 8_1 门控项在本头文件不存在，保持原样）；patch 副本须删 `.cargo-checksum.json`。
- **预防：** 用 bindgen 系 crate（ffmpeg-next、*-sys）时，dev 库版本不是越新越好——与 crate 发布同期对齐；`AVFrameSideDataType`/`AVPacketSideDataType` 这类增长型枚举是漂移高发区。铺量进主线时该补丁随依赖一起固化，升 crate 先全量编译看漂移面。
