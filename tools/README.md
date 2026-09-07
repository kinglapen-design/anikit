# tools —— 开发辅助工具

放开发期的辅助脚本：假数据生成（10 万条 metadata 性能测试）、打包脚本、数据迁移工具等。

与 spike/ 的区别：tools/ 是长期使用的开发工具，spike/ 是一次性的技术验证。

## 清单

- `fetch-ffmpeg.py` —— 下载 ffmpeg/ffprobe sidecar（BtbN LGPL 静态构建）到 `src-tauri/binaries/`；首次开发 / 换机必跑（音视频解码依赖，二进制不入 git）
- `gen-anim-samples.py` —— 生成动图测试样本（anim3.gif / anim3.webp / still.webp → `test-assets/`）；需 Pillow（装 `tools/.venv`）
- `waveform-match.py` —— 波形渲染逐像素比对（候选算法 vs 参考 webp，2026-07-26 波形定稿用）
- `gen-fake-data.py` —— 10 万条假数据（性能测试）
- `cdp-recycle-probe.py` / `cdp-quick-recycle-probe.py` / `cdp-hamburger-probe.py` —— CDP 探针（1.3 菜单走查）：Edge 无头 `--remote-debugging-port` 打开 vite dev 页面 + 注入 `__TAURI_INTERNALS__` 桩（假库数据），Input.dispatchMouseEvent 真实派发右键/点击，验证菜单弹出与内容；依赖 `tools/.venv` 的 websockets； vite dev 需先在 5373 跑着
- `test-assets/` —— 测试样本（动图 gif/webp、sample.mp3 带 TBPM=128、dynamic*.wav 波形目检样本）
- `style-preview/` —— 样式预览小 HTML（用户维护 + 1.3 菜单探针：menu-probe / icon-grid（全图标总览）/ icon-probe / icon-compare / submenu-probe，链 dist CSS 复刻组件 DOM，Edge 无头截图对照设计图）
