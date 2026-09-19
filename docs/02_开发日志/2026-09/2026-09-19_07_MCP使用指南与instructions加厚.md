# MCP Agent 使用指南 + 握手 instructions 加厚（2026-09-19，用户定）

一期落地后用户问「换 Agent 怎么办 / 有没有 README 给 Agent 看」，两件配套落地。

**新建 `docs/05_接口文档/MCP使用指南.md`**——Agent 与接入者视角：连接方式（固定地址 `http://127.0.0.1:17701/mcp` + Kimi Code / Codex / Claude Code 三家配置示例）、8 工具速览表、「批量看资产写提示词」典型流程五步、纪律红线（只读 / 路径不缓存 / 禁全目录枚举）、故障排查（406 是规范行为等）。05 Index 协议节同步登记。

**握手 instructions 加厚**（`server/mcp/mod.rs` 的 `#[tool_handler]` 宏参数）——MCP 的设计里这就是「给 Agent 的 README」：每次连接 initialize 必读，无需用户手动喂文档。从一句话简介扩为典型流程（探库状态 → 搜索/浏览 → 详情 → 取路径直读看图 → 读文本资产）+ 纪律（只读 / 路径现取不缓存 / 库未开回报错），与使用指南同口径（代码注释注明两处同步义务）。

cargo check 过；dev watcher 重编后 curl initialize 验证新 instructions 随握手下发（见会话记录）。
