# MCP 补开 move_folders（第 16 个工具，2026-09-19 用户补定）

二批四件套落地后用户补一句「移动文件夹要开一下，万一改一下结构就改不动了」——`move_folders`（文件夹重设父级，仅改数据不动磁盘）同日补开，工具面 15 → 16（只读 8 + 写 8）。

实现零新范式：commands/operations.rs 抽 `move_folders_op` 整段（锁 library → 锁 undo → 单撤销步），`tools/structure.rs` 加第四个工具。面向 Agent 的简化：入参只给 `folderIds` + `targetParentId`（缺省 / "root" = 顶层文件夹区块，落点 = 同级末尾）——命令层的 `before_folder_id`（排序插入位）与 `target_block`（跨区块）参数不收，MCP 侧固定 None / 顶层时补 `"dir"`（命令层口径：落根级给值才写 Folder.block，「文件夹」区块 id 恒 dir）。循环护栏（移入自身或子孙整条报错）、同父 / 陈旧 id 跳过均随 `_op` 整段生效。进撤销。

验证插曲值得记：当时用户跑的是**当天早些时候打的便携版**（单实例插件下 dev 实例已关），移动验证戳过去「tool not found」——便携版是静态二进制、没有 watcher 重编，这一课和踩坑 076/080 同族（验证对象必须是新二进制，别对旧进程验证新功能）。cargo check / test 471 绿后增量重编 release（59s）备好新 exe；便携版运行中 exe 被 Windows 锁定无法覆写（Device or resource busy），**刷新便携包与 move_folders 的真机 curl 验证待用户关便携版后进行**——文档与 instructions 已先按 16 工具口径同步。
