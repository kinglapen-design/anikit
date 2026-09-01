# cep-proto —— Animate CEP 插件链路探针（阶段八 8.1 前置）

目的：正式开发前验证三件事——① CEP 面板在本机 Animate 2024 加载显示；② 面板 JS 经端口文件发现 + 原生 WebSocket 连通本地 WS server；③ jsx 导出链路（选中元件 → 复制到新文档 → 缩放对齐舞台中央 90% → PNG / PNG 序列 → fla 另存）。

## 结构

- `extension/` —— CEP 扩展本体（manifest / index.html / js / jsx），安装即复制到 `%APPDATA%\Adobe\CEP\extensions\com.anikit.cep`
- `ws_server.py` —— 最小 WS server（tools/.venv 的 websockets），模拟 AniKit 侧：动态端口 + 写 `%APPDATA%\AniKit\cep-server.port` 端口文件 + echo
- `install.bat` —— 一键安装（PlayerDebugMode 本机已开，CSXS.9/10/11 均 = 1）

## 验证步骤

1. `install.bat` 安装扩展；
2. 启动 WS server：`tools/.venv/Scripts/python.exe spike/cep-proto/ws_server.py`；
3. 打开 Animate 2024 → 菜单 窗口 > 扩展 > AniKit；
4. 面板应显示「已连接 127.0.0.1:端口」；点「读取选中元件」返回选中信息 JSON；
5. 舞台上放一个多帧 movieclip，选中后分别点「导出静图」「导出动图」，检查 `%TEMP%\anikit-cep-out\` 产物（fla / png / f_XXXX.png 序列）。

## 结论（2026-09-01 实机验证全过）

- [x] 面板加载显示（Animate 2024，PlayerDebugMode 已开免配置）
- [x] WS 连通（端口文件发现机制：server 写 `%APPDATA%\AniKit\cep-server.port`，面板 node fs 读 + 原生 WebSocket 直连）
- [x] jsx 读取选中元件信息（中文元件名经 evalScript 返回无乱码）
- [x] 静图导出 + fla 另存 + 90% 居中对齐（视觉留边 = 元件内容自身边距，非对齐误差）
- [x] 动态导出：~~`exportPNGSeqToFile`~~ **Animate 2024 已移除该 API**（TypeError: not a function）；正确姿势 = `tl.insertFrames(n)` 扩展主时间轴 + 循环 `tl.currentFrame = i` + `exportPNG(file, true, true)` 逐帧导；**movieclip 必须先转 `symbolType='graphic'` + `loop='loop'` 才随主时间轴逐帧推进内部动画**（用户旧项目 ref/cep-extension 踩坑结论），导完还原。
- [x] 坐标口径：缩放居中只作用于 PNG 画面，导出后还原坐标再保存 fla（fla 内元件 = 原舞台原位坐标，实机确认）。
- [x] ffmpeg 合成 WebP 动画实测（17 帧 @25fps）：1080p/q80 = 628KB、1024/q80 = 281KB、1024/q60 = 230KB → 10 秒档线性估算 ~4MB，**2M 软上限 + 降级链定案成立**。

**已知余项（正式版处理）**：① Animate 默认 PNG 导出不带透明通道（白底），正式版需发布设置/profile 让导出带 alpha；② CEP 面板 HTML 输入框打不了中文（CEF IME 顽疾），命名输入需 jsx ScriptUI 原生对话框辅助回填；③ jsx 报错消息中文经 evalScript 传出乱码（GBK↔UTF-8），正式版错误码化。
