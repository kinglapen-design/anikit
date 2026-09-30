# 当前状态 · 2026-10

> 本月 8 条状态记录。返回 [当前状态 Index](../Index.md)。

| 文件 | 日期 | 标题 |
| --- | --- | --- |
| [2026-10-01_08_工作空间拖入高亮三连修.md](2026-10-01_08_工作空间拖入高亮三连修.md) | 2026-10-01 | 工作空间拖入高亮三连修 | ：_07 走查两连报——pane 命中层错位（卡片层→视口根，下半视口恒亮）/ ws 同目录禁高亮（库同款守卫）/ OS 拖入悬停高亮（enter/over 驱动 osFileDragHover，与内部拖拽共视觉槽；虚拟视图/回收站不高亮）；typecheck 绿；待实机走查 |
| [2026-10-01_07_工作空间与资源管理器拖拽剪贴板互通.md](2026-10-01_07_工作空间与资源管理器拖拽剪贴板互通.md) | 2026-10-01 | 工作空间 ↔ 资源管理器拖拽与剪贴板互通 | ：待办立项 A 节兑现，先改 PRD 02-13 再动工——拖入一律复制（onDrop ws 落点三分支 + ws_import_paths / import_external_entries 护栏）；拖出复制语义（ws_drag_paths 共用 start_files_drag）；剪贴板双向（ws_write_clipboard 剪切附 DropEffect=MOVE，粘贴序号仲裁 ws_paste_system）；菜单「粘贴」条件扩 + spec async 化；剪切半透明三处接通；558 测绿 + clippy 净 + typecheck 绿；待实机走查 |
| [2026-10-01_06_工作空间性能优化二轮.md](2026-10-01_06_工作空间性能优化二轮.md) | 2026-10-01 | 工作空间性能优化二轮 | ：待办三点 + 观察项落地——①fs-changed 负载带变更目录 rel 集（父+祖父，null 兜底全量），前端 rescanDirs 精准重扫；②DirScan thumbs 快照集合 + probe.json 魔数探测缓存（含负缓存）；③ensureDir 挂载跳过 + 切空间 ws store $reset；④worker 池上限 4→8；556 测绿 + clippy 净 + typecheck 绿；待实机走查 |
| [2026-10-01_05_回收站行归位与快速入口总容器.md](2026-10-01_05_回收站行归位与快速入口总容器.md) | 2026-10-01 | 回收站行归位与快速入口总容器 | ：用户走查修订——回收站行挪根层最上方 + 套库快速访问同口径无标题总容器（行 gap 2，后续入口行直接加不动嵌套）；零新组件零新 token；typecheck 绿 |
| [2026-10-01_04_工作空间侧栏回收站行.md](2026-10-01_04_工作空间侧栏回收站行.md) | 2026-10-01 | 工作空间侧栏回收站行 | ：ws 删除走系统回收站（PRD 06 既有）——根层底部固定「回收站」行（库快速访问同款 TreeRowItem quick + trash，无数量），点击 = 新命令 open_system_recycle_bin（explorer shell:RecycleBinFolder）；554 测绿 + clippy 净 + typecheck 绿；待实机走查 |
| [2026-10-01_03_工具链混链三次复发_关闭dev增量编译.md](2026-10-01_03_工具链混链三次复发_关闭dev增量编译.md) | 2026-10-01 | 工具链混链三次复发：dev 增量编译关闭 | ：踩坑 091 三次复发（诱因 = rust 1.98.0→1.98.1 patch 升级改 anon 符号哈希，dev 链接 LNK2019×35）；清 incremental + `[profile.dev] incremental = false` 终局关闭（091 预防节定案兑现）；全量重编 1m37s 链接绿 + test/clippy 绿；代价 = 改 Rust 全量重编本 crate |
| [2026-10-01_02_空白处系统菜单_目录背景版.md](2026-10-01_02_空白处系统菜单_目录背景版.md) | 2026-10-01 | 空白处系统菜单：目录背景版 | ：「排序/新建」本属目录背景菜单不属条目菜单；shell_menu.rs 共享管线重构 + 背景对象 = 目录 PIDL → BindToObject → CreateViewObject（Explorer 空白右键同径）；新命令 ws_shell_menu_background + 空白菜单挂「系统菜单」；含 查看▸/排序方式▸/刷新/新建▸/属性，查看/排序托管菜单天然无效（先天边界）；554 测绿 + clippy 净 + typecheck 绿；待实机走查 |
| [2026-10-01_01_工作空间右键菜单移植与系统菜单.md](2026-10-01_01_工作空间右键菜单移植与系统菜单.md) | 2026-10-01 | 工作空间右键菜单移植与系统菜单 | ：ws 四 spec 重组对齐库菜单分组/分割线/快捷键（打开外发组/剪贴板组/复制数据▸/创建副本/系统菜单；「打开」冗余项不列；新建 = 先建后命名移植）；快捷键新绑 Ctrl+D / Shift+Enter / Ctrl+E；新命令 6 个 + cep 抽 send_fla；**Windows 原生右键菜单** = IContextMenu 经典菜单（全 PIDL + 消息转发窗口 + spawn_blocking STA），压缩解压/TortoiseGit 等经典扩展全量出，Win11 IExplorerCommand 现代扩展不在其中；553 测绿 + clippy 净 + typecheck 绿；待实机走查 |
