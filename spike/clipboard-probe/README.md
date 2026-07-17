# Animate 剪贴板格式探针（阶段 0 · 任务 1）

目的：确认 Animate 复制元件时写入系统剪贴板的私有格式是否为 XFL 类 XML——这决定"发送舞台"能否跳过打开 fla（见 PRD `07_动画软件通信` 04）。

## 使用步骤

1. 打开 Animate，在舞台上**复制一个元件**（Ctrl+C），不要清空剪贴板。
2. 双击 `run.bat`（或执行 `python probe.py`）。
3. 把生成的 `out/report.txt` 交给开发 / AI 分析。

## 我们在找什么

- 注册格式（ID ≥ 0xC000，名称含 Flash / Animate / Adobe 字样）；
- 数据内容以 `<` 开头或可解码为 XML 文本的格式；
- 若能确认是 XFL 类 XML：尝试从 fla（XFL 压缩包）的库元件 XML 生成该格式数据 → 写回剪贴板 → `clipboardPaste(true)` 粘贴验证，即可实现免打开文件的发送。

零依赖：仅使用 Python 标准库 ctypes 调用 Win32，无需安装任何东西。输出在 `out/`（已被 .gitignore 排除）。
