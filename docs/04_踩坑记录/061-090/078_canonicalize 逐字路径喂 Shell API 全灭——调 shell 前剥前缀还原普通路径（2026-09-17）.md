# 078. canonicalize 的逐字路径（verbatim path）喂 Shell API 全军覆没——调 shell 前剥前缀还原普通路径

日期：2026-09-17。环境：Windows 11 + rust-msvc，工作空间实机走查。

## 现象

工作空间步骤 1 实机走查：非图片/音视频条目（txt/exe/dll/ico……）卡片只有扩展名徽标，缩略图区空盒等多久都不出。调试测试里 `ensure_format_icon_in` 稳定返回 false，打印的路径形如 `\\?\C:\Users\...\说明.txt`；同一文件用普通路径直调却成功拿到 256×256 图标。

## 原因

工作空间 `list_dir` 走 `resolve_under` 做越权校验，内部 `canonicalize()` 产出 **`\\?\` 逐字前缀路径**（verbatim path）。该前缀是 NT 对象管理器路径，常规 Win32 API 大多兼容，但 **`SHCreateItemFromParsingName` 解析不了**——静默失败（返回错误码，无 panic、无日志爆点），表现就是「图标提取整体无声失败」。此前封闭式素材库不经这条规范化路径，所以同一适配器在库侧一直正常，只有新接的工作空间全灭，极具迷惑性。

## 解法

`adapters/shell_thumb.rs` 的 `imp::format_icon` 调 shell 前剥前缀：普通形 `\\?\C:\...` → `C:\...`；UNC 形 `\\?\UNC\s\sh` → `\\s\sh`。剥前缀在适配器内部做，调用方（库侧 / 工作空间侧）路径口径均不变。

## 预防

1. **凡喂给 Shell/Explorer 系 API（`SHCreateItemFromParsingName`、`SHGetFileInfo`、拖放 IDataObject 等）的路径，先确认不是 `\\?\` 逐字形**；canonicalize 的产物流到这些 API 前一律过一遍剥前缀。
2. 诊断「某类调用整体静默失败」时，第一步打印实参路径看前缀，比怀疑 COM 初始化、格式支持快得多。
3. 同一适配器跨两条数据路径（库 / 工作空间）表现不一致时，先 diff 两条路径的入参形态，而不是怀疑适配器本身。
