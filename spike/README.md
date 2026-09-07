# spike —— 技术验证代码

阶段 0 的验证性代码都放这里，每个任务一个子目录。验证完成后结论记入 `docs/02_开发日志/Index.md` 与 `docs/03_项目进度/Index.md`，代码保留归档，**不进主线**（不进 src/、src-tauri/）。

当前任务：

- `clipboard-probe/` —— Animate 剪贴板格式探针（任务 1，第一优先）
- `send-to-an-ref/` —— 旧 electron 版 JSFL 参考（2026-09-02 用户提供：导出库 + 批量缩略图两份；8.3 发送至 AN 兜底护栏模式蓝本，8.2 批量导出参考素材）
- `decode-proto/` / `icon-migrate/` / `scroll-proto/` —— 各阶段验证存档
- `video-seek/` —— 视频播放通道真机实测（2026-08-22，5.3 方向终版依据）：`probe.py` = 真 WebView2 里 `<video>` 通道基线（asset protocol vs localhost Range 的 seek/scrub/播放数据）；`ui-smoke.py` = 真实 UI 冒烟（悬停 scrub / 预览播放 / 逐帧真机数据）。前置：tauri.conf 窗口加 `additionalBrowserArgs: --remote-debugging-port=9223`（踩坑 051，用完删）。播放性能回归可复用
