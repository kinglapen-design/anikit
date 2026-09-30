# 当前状态 · 2026-10

> 本月 3 条状态记录。返回 [当前状态 Index](../Index.md)。

| 文件 | 日期 | 标题 |
| --- | --- | --- |
| [2026-10-01_03_工具链混链三次复发_关闭dev增量编译.md](2026-10-01_03_工具链混链三次复发_关闭dev增量编译.md) | 2026-10-01 | 工具链混链三次复发：dev 增量编译关闭 | ：踩坑 091 三次复发（诱因 = rust 1.98.0→1.98.1 patch 升级改 anon 符号哈希，dev 链接 LNK2019×35）；清 incremental + `[profile.dev] incremental = false` 终局关闭（091 预防节定案兑现）；全量重编 1m37s 链接绿 + test/clippy 绿；代价 = 改 Rust 全量重编本 crate |
| [2026-10-01_02_空白处系统菜单_目录背景版.md](2026-10-01_02_空白处系统菜单_目录背景版.md) | 2026-10-01 | 空白处系统菜单：目录背景版 | ：「排序/新建」本属目录背景菜单不属条目菜单；shell_menu.rs 共享管线重构 + 背景对象 = 目录 PIDL → BindToObject → CreateViewObject（Explorer 空白右键同径）；新命令 ws_shell_menu_background + 空白菜单挂「系统菜单」；含 查看▸/排序方式▸/刷新/新建▸/属性，查看/排序托管菜单天然无效（先天边界）；554 测绿 + clippy 净 + typecheck 绿；待实机走查 |
| [2026-10-01_01_工作空间右键菜单移植与系统菜单.md](2026-10-01_01_工作空间右键菜单移植与系统菜单.md) | 2026-10-01 | 工作空间右键菜单移植与系统菜单 | ：ws 四 spec 重组对齐库菜单分组/分割线/快捷键（打开外发组/剪贴板组/复制数据▸/创建副本/系统菜单；「打开」冗余项不列；新建 = 先建后命名移植）；快捷键新绑 Ctrl+D / Shift+Enter / Ctrl+E；新命令 6 个 + cep 抽 send_fla；**Windows 原生右键菜单** = IContextMenu 经典菜单（全 PIDL + 消息转发窗口 + spawn_blocking STA），压缩解压/TortoiseGit 等经典扩展全量出，Win11 IExplorerCommand 现代扩展不在其中；553 测绿 + clippy 净 + typecheck 绿；待实机走查 |
