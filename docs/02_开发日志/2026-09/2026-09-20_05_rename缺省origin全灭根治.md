# rename_placements 缺省 origin 全灭根治：自动消歧 + 诊断自愈（2026-09-20）

用户转来豆包 agent 的排查单：批量重命名 48 项全报「项目不存在，可能已被移动或删除」，list / detail 全正常，假 id 报错一模一样。根因当晚坐实——**Placement 定位是三元组（assetId + folderId + origin），豆包漏传 origin**（缺省 false），而记录全是 origin=true，精确匹配落空。数据层不经过三元组匹配所以全绿。锅的一半在工具描述：write.rs 把 origin 写成「true = 连磁盘一起改 / false = 仅改显示名」，读起来是改名力度开关，实际是**定位键**——agent 以为「只改显示名不用传」，必然踩中。

## 根治（service 层锁内消歧，写点内聚）

- `RenameItem.origin: bool → Option<bool>`（serde 兼容：APP 前端传布尔 = Some）；新增 `resolve_origin` helper（services/operations/mod.rs，pub(crate)）：None = 按 (asset, folder) 找——**唯一记录自动命中**（origin / 链接皆可），多条（origin + 链接同文件夹）进 failed 要求指定，零条走原「项目不存在」路径。
- **命令层撤销快照走同一 helper**：rename_placements_op 的 metadata 快照判断改 `resolve_origin(...) == Some(true)`——漏判会让 origin 项漏 record（撤销后 originName 不回滚），这是消歧最容易埋的暗坑，单写点必须落在同一函数。
- **诊断自愈**：定位落空但 (asset, folder) 有记录时，failed reason 带实际值「该文件夹内此资产的记录 origin=true，请核对 origin 字段」——agent 看了自己就会改，不用再回来排查。
- 描述口径改写：write.rs origin doc（定位三元组 + 缺省自动定位 + 力度由命中记录自然决定）、structure/trash 的 PlacementRefParams（缺省 false = 只命中链接件，漏传会落空被跳过）、instructions / MCP通道 / 使用指南同步。

## 验证

`cargo test` 472 过 0 挂（新增 `rename_placements_origin_omitted_auto_resolve`：单条 origin 消歧 + 单条链接消歧 + 多条拒判 + 传错诊断文案；`cargo check` 零警告）。免安装包重打（旧 trash 初版仅数小时寿命直接替换，不另留档）。真机冒烟：tools/list 17；**缺省 origin 改临时文档名 → renamed 1 且磁盘同步改名**（消歧取 true 无误）；trash 清理。插曲：首轮拿 95KBR 测报「项目不存在」虚惊——用户已把 09-19 的 MCP测试图清掉了，失败恰是正确行为（0 条记录 = 原文案）。产物：回收站两条临时文档（用户一键清）。

## 边界与待办

move / trash / link 的 PlacementRef 是共享结构（delete / move / link 三族 + 命令层多处分流），origin 缺省消歧推广面大，本轮只修 rename（用户实际痛点）——PlacementRef 系记待办下迭代评（move 有 moved 计数露馅、trash 静默跳过最该补实际数回包）。
