# MCP 四批：set_folder_groups 分组工具（2026-10-08，PRD 08-06 ⑦）

用户提「给 MCP 加上分组的工具」。缺口核实：分组定义此前**可读不可写**——`list_folder_tree` 节点回包已带 groups（规则组 / 自由组全量），但 Agent 无法给新建场次文件夹配「提示词 / P1 / P2」规则组，只能回 APP 内人工开分组管理窗口。漫剧生产链路（PRD 08-06 目标工作流）里每开一场戏就要配一组规则，这正是该自动化的点。

范围照 PRD 既有结论收窄：2026-09-19 定「命名规范 + 链接即自动归组，**不需要 assign_group 工具**」维持不变——本轮只加 `set_folder_groups` 一个（分组定义管理）；自由组成员归组不设独立工具，需要时经本工具 members 全量管理（全量提交天然覆盖增删）。PRD 08-06 先行改定案（四批段 + 阶段表 ⑦），代码随后。

## 语义 = APP 分组管理窗口同款草稿式

工具语义与 33 稿分组管理窗口完全同构：**全量草稿式提交**——`groups` 数组 = 该文件夹分组终态（空数组 = 取消分组、漏交 = 删组），一次提交单撤销步。Agent 侧典型用法 = `list_folder_tree` 拿既有场次的 groups 抄改（改既有组照回包抄 id）。新建组 id 缺省 / 空串时后端生成——`core::models::new_group_id()`（`G` + yyMMddHHmm + 3 位随机字符，对齐 F / B 系 id 规则；前端自造的 `g{ts36}-{rand36}` 式不影响，两式并存都是合法 id），生成结果随回包终态数组返回，Agent 凭它做后续编辑定位。

实现照三批 `_op` 范式零新链路：`commands/operations.rs` 的 `set_folder_groups` 命令整段抽 `set_folder_groups_op(state, folder_id, groups)`（锁 library → 锁 undo → 单写点 + 单撤销步），命令变薄壳一行委托，APP 行为零变化。MCP 侧新 `tools/groups.rs`（一类一文件 + tools/mod.rs 一行 + mod.rs 注册一行）：`read_only_hint = false` + write_gate 门禁，入参 GroupDefParams / GroupRuleParams 本地参数件（schemars 需要 + id 可选语义，模型直挂不可行），转换后调 `_op`，成功 `emit_library_changed` + 回终态 `GroupDef[]`。校验全走既有服务层（values 空数组拒绝 / 规则集完全相同拒绝 / 文件夹不存在报错），工具零新增校验逻辑。instructions 同步：写工具 11 → 12、进撤销 9 → 10。

## 验证

`cargo check` 零警告（非 test profile——mcp 模块 cfg(not(test))，cargo test 不编译 groups.rs，check 必需）；`new_group_id` 补 `group_id_format` 单测（顺带消化 test profile 下唯一消费者缺席的 dead_code）；clippy 全目标净；**589 测绿（+1）**。

真机端到端（dev 实例热重建后 curl 17701 握手）：tools/list **20 个工具**，set_folder_groups 在列；实调 `{folderId:"F-NOPE", groups:[{name:"提示词", rules:[{kind:"ext",values:["md"]}]}]}` → 过写入闸、过参数反序列化（中文组名 + 嵌套 rules），回工具级错误「目标文件夹不存在，可能已被删除」（isError=true）——全链通且对库零副作用。前端只动文案，`pnpm typecheck` 0 错。**待实机走查：真实场次文件夹配组 → APP 侧分组即时显示 / Ctrl+Z 撤销**。

## 文档

PRD 08-06（四批段 + 阶段表 ⑦、余量批顺延 ⑧）、MCP通道.md（工具面 20 分五表、写入热刷新与撤销口径计数、**顺手补提示词批两工具缺表**——09-26 轮只更新了使用指南）、MCP使用指南.md（速览表 + 写入开关节 + 漫剧链路第 1 步补 set_folder_groups 用法 + 计数）、插件面板 MCP 详情窗功能列表（McpPluginWindow + PreviewPlugins 陈列件，写 9 → 12——**顺手补提示词轮漏更**，该窗自 09-20 起停在写 9）、当前状态 Index。
