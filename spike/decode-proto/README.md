# decode-proto —— 阶段四自研解码器 spike

目的：验证「ffmpeg-next 解码线程 + 帧队列推前端」链路（阶段四开工口径，2026-08-19）。

## 结论（2026-08-20，验证通过）

- **链路全通**：ffmpeg-next 解码 → 帧槽 → WS 推 → canvas putImageData，50 卡同播实测成立。
- **通道定案 = 本地 WS 推**（16 路全速：WS 1686fps / invoke 876 / custom protocol 924；50 路 2400fps）。
- **生产语义（pace=1 原速，50 路 1080p→320p，32 核机）**：GIF CPU 17%（每路 9.1/10fps）、H264 CPU 29%（每路 25.6/30fps）。
- **边界**：lgpl 构建无 libwebp decoder → 动图 webp 归主线 image-webp；Windows sleep 15.6ms 粒度坑（铺量 timeBeginPeriod 或消费端节奏）。
- 详细数据与铺量草案：`docs/02_开发日志/2026-08/Index.md`；踩坑 038/039/040。

## 组成

- `src/main.rs` —— CLI 探针：单路解码摸底 / `--list` 探 decoder / `--bench <文件> <路数> <秒数>` 并发吞吐。
- `shell/` —— 最小 Tauri 壳：N 路解码线程产 320p RGBA → WS 推 / invoke 拉 / custom protocol 拉三通道，`web/index.html?lanes=N&ch=ws|invoke|proto` canvas 实测。
- `cdp-probe.py` —— CDP 实测探针（起壳 + 逐通道采样 fps/CPU + 截图）：`tools/.venv/Scripts/python.exe spike/decode-proto/cdp-probe.py [lanes] [媒体] [ch列表] [pace]`。
- `patches/ffmpeg-next` —— EXIF cfg 门槛补丁（见 Cargo.toml 注释与踩坑 038）。
- `vendor/ffmpeg` —— BtbN lgpl-shared autobuild-2025-08-31 dev 库（不入 git）。
- `testdata/` —— 生成素材（不入 git，生成命令见下）。

## 环境搭建（换机 / 重装系统后跑一次）

```bash
# 1. libclang（ffmpeg-sys-next 的 bindgen 依赖；pip 包落项目内，不碰系统）
tools/.venv/Scripts/python.exe -m pip install --target tools/libclang libclang

# 2. ffmpeg lgpl-shared dev 库（BtbN autobuild-2025-08-31，avcodec 62.13 对齐 ffmpeg-next 8.1.0）
#    https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2025-08-31-13-00/ffmpeg-N-120856-g9893d66add-win64-lgpl-shared.zip
#    解压重命名为 spike/decode-proto/vendor/ffmpeg/（bin/ include/ lib/ 三级齐全）

# 3. 测试素材（用主线 sidecar 生成）
src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe -y -loglevel error -f lavfi -i "testsrc2=size=1920x1080:rate=10:duration=5" -pix_fmt rgb24 spike/decode-proto/testdata/big.gif
src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe -y -loglevel error -f lavfi -i "testsrc2=size=1920x1080:rate=30:duration=10" -c:v libopenh264 -b:v 3M spike/decode-proto/testdata/big.mp4
```

环境变量已由两处 `.cargo/config.toml` 固化（LIBCLANG_PATH / FFMPEG_DIR，相对路径）。
vendor/ 与 target/ 与 testdata/ 不入 git。

## 用法

```bash
cd spike/decode-proto
cargo run -- testdata/big.gif              # 单路摸底
cargo run -- --bench testdata/big.mp4 50 5 # 50 路并发 5 秒吞吐
cd shell && cargo run -- --lanes=50 --media=../testdata/big.gif --pace=1  # 壳（原速播放）
```

