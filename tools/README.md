# tools —— 开发辅助工具

放开发期的辅助脚本：假数据生成（10 万条 metadata 性能测试）、打包脚本、数据迁移工具等。

与 spike/ 的区别：tools/ 是长期使用的开发工具，spike/ 是一次性的技术验证。

## 清单

- `fetch-ffmpeg.py` —— 下载 ffmpeg/ffprobe sidecar（BtbN LGPL 静态构建）到 `src-tauri/binaries/`；首次开发 / 换机必跑（音视频解码依赖，二进制不入 git）
- `gen-anim-samples.py` —— 生成动图测试样本（anim3.gif / anim3.webp / still.webp → `test-assets/`）；需 Pillow（装 `tools/.venv`）
- `waveform-match.py` —— 波形渲染逐像素比对（候选算法 vs 参考 webp，2026-07-26 波形定稿用）
- `gen-fake-data.py` —— 10 万条假数据（性能测试）
- `test-assets/` —— 测试样本（动图 gif/webp、sample.mp4、sample.mp3 带 TBPM=128、dynamic*.wav 波形目检样本）
- `style-preview/` —— 样式预览小 HTML（用户维护）
