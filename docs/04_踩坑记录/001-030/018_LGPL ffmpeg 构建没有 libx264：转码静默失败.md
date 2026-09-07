## 018 LGPL ffmpeg 构建没有 libx264：转码静默失败

- **现象：** 导入 mov 后预览报"该格式暂时无法播放"，转码兜底没生效；手动跑 sidecar 才发现 `libx264` 编码器不存在。
- **原因：** 5.1 选型特意用 BtbN **LGPL** 静态构建（避免 GPL 传染），而 x264 正是 GPL 许可——LGPL 构建不含 libx264。写转码参数时按惯性填了 `-c:v libx264`，ffmpeg 因编码器缺失非零退出。
- **解法：** 改用 LGPL 构建内可用的 h264 软编码器 `libopenh264`（Cisco，BSD 许可，无传染）——注意它不支持 x264 的 crf/preset，用 `-b:v` 码率控制；`-encoders` 列表核对过 libvpx-vp9 / 硬件编码器（h264_mf / nvenc / qsv）也在但前者太慢后者依赖机器，不作首选。
- **预防：** 给 sidecar 写编解码参数前，先用 `-encoders` / `-decoders` 核实该构建实际带了什么；许可选型（LGPL）与功能清单是一体的，改构建变体时必须重核。
