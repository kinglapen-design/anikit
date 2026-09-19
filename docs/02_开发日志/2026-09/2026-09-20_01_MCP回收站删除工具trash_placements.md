# MCP 三批：trash_placements 回收站删除工具（2026-09-20，PRD 08-06 ⑤）

用户两条反馈催生本轮：① 让 agent 重命名文件，agent 说没有重命名工具；② 允许 AI 把文件丢回收站，但不许彻底删除。① 查证是虚惊——`rename_placements` 随首批（2026-09-19）已落地，agent 侧会话建于工具上线前（tools/list 会话缓存）或连着旧版 APP，重连即可，无需改代码。② 是 PRD 级需求变更：原定案「删除永不经 MCP（L4 永远人工确认）」改写为**「允许 AI 丢库内回收站，彻底删除永不经 MCP」**——PRD 08-06 先行改定案（三批段 + 阶段表 ⑤），代码随后。

## 落点 = 库内回收站，技术债为零

库内回收站（recycle.json 按资产聚合快照、info 目录磁盘文件不动）现成且可逆：`services::operations::delete::delete_placements` 的 `"recycle"` 分支，命令层 `delete_placements` 带完整撤销（undo record folders.json + recycle.json 单撤销步）。彻底删除路径（recycle.rs 彻底清除 / purge.rs / 系统回收站适配器）**一律不注册为工具**，「不许彻底删除」天然满足。

实现照结构批 `_op` 范式：`commands/operations.rs` 的 `delete_placements` 命令整段抽 `delete_placements_op(state, items, last_action)`（锁 library → 锁 undo → 单写点 + 单撤销步），命令变薄壳一行委托，行为零变化（uncategorized 分流标签原样）。MCP 侧新 `tools/trash.rs`（一类一文件 + tools/mod.rs 一行 + mod.rs 注册一行）：`trash_placements`，`read_only_hint = false` + write_gate 门禁，入参复用 structure.rs 的 PlacementRefParams 三元组，恒传 `Some("recycle")`（uncategorized 只留 APP 内），成功 `emit_library_changed` + 回 `{ requested: N }`（service 无计数返回，陈旧 / 重复三元组自动跳过不计，doc 注明）。instructions 同步：写工具 8 → 9、进撤销 6 → 7、纪律改「彻底删除永不经 MCP」。

## 验证

`cargo test` 471 过 0 挂；`cargo check` 零警告（非 test profile——mcp 模块 cfg(not(test))，cargo test 不编译 trash.rs，check 必需）。真机端到端（自起 tauri dev，库「动画资产」自动打开，用户闸本开着）：

- tools/list 17 个工具，trash_placements 在列（readOnlyHint=false）；
- 真调丢 `blank-test-2.png`（AN2609192128-YE3CE，09-19 MCP测试文件夹产物）→ 回 `{ requested: 1 }`；
- 复核 list_placements：文件夹只剩 95KBR；recycle.json 新增 YE3CE 聚合条目（origin 标记随记录）；info 目录三件套（png / webp / metadata.json）原样未动——物理零接触；
- 空数组回「items 至少 1 项」；陈旧三元组（同项再丢一次）跳过不报错；
- 撤销步 = 命令层同一实现（09-19 首批已验 APP 内 Ctrl+Z 链路），本步用户可在 APP 回收站一键还原或 Ctrl+Z。

产物留库：YE3CE 在用户回收站（本就是 09-19 验证残留，用户会处理）。测毕关 dev。

## 文档

PRD 08-06（三批段 + 阶段表 ⑤，余量批顺延 ⑥）、MCP通道.md（工具面 17 分四表、写入闸 / 纪律段口径）、MCP使用指南.md（速览 + 写入开关节 + 顺手修 15 → 17 计数残留）、当前状态 Index、待办 2026-09-19（工具面膨胀计数 16 → 17）。
