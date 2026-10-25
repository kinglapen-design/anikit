# spike —— 技术验证代码

阶段 0 的验证性代码都放这里，每个任务一个子目录。验证完成后结论记入 `docs/02_开发日志/Index.md` 与 `docs/03_项目进度/Index.md`，代码保留归档，**不进主线**（不进 src/、src-tauri/）。

当前任务：

- `clipboard-probe/` —— Animate 剪贴板格式探针（任务 1，第一优先）
- `send-to-an-ref/` —— 旧 electron 版 JSFL 参考（2026-09-02 用户提供：导出库 + 批量缩略图两份；8.3 发送至 AN 兜底护栏模式蓝本，8.2 批量导出参考素材）
- `decode-proto/` / `icon-migrate/` / `scroll-proto/` —— 各阶段验证存档
- `video-seek/` —— 视频播放通道真机实测（2026-08-22，5.3 方向终版依据）：`probe.py` = 真 WebView2 里 `<video>` 通道基线（asset protocol vs localhost Range 的 seek/scrub/播放数据）；`ui-smoke.py` = 真实 UI 冒烟（悬停 scrub / 预览播放 / 逐帧真机数据）。前置：tauri.conf 窗口加 `additionalBrowserArgs: --remote-debugging-port=9223`（踩坑 051，用完删）。播放性能回归可复用
- `semantic-clip/` —— 1.0 语义分析前置 spike（2026-10-24）：中文 CLIP 双塔导出、Rust `ort` CPU / DirectML、私有运行时与中文检索验收；模型、缓存、虚拟环境均本地隔离，不进主线
- `semantic-eval/` —— 真实动作语义验收集与模型共用评测：BridgeData V2（CC BY 4.0）轨迹抽帧、hard negatives、TopK / MRR 与错误分析
- `semantic-multilingual/` —— 许可清晰的多语言 CLIP 备选 spike：指定文本 / 图像双塔、CPU / DirectML 与同一质量集对照，不修改主线
- `semantic-store/` —— SQLite 语义数据层 spike：模型 revision 隔离、分段向量幂等替换、确定性 topK、任务检查点与重开续跑；3 测 + clippy 净
- `semantic-siglip/` —— SigLIP 2 base patch16-512 对照：同 150 帧 / 17+1 查询实测质量下降且成本更高，不选作 1.0 基础模型，保留复现证据
