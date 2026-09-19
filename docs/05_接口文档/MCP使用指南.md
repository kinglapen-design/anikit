# MCP 使用指南（给 Agent 与接入者）

AniKit 进程内嵌的 MCP server（PRD 08-06：一期只读 8 工具 + 二期首批写工具 3 个，写入闸形态 A）。本文是接入与使用说明；协议层细节（握手 / 回包结构 / 错误口径）见 [MCP通道](MCP通道.md)。

## 连接

服务随 APP 启动，地址恒定 `http://127.0.0.1:17701/mcp`（固定端口优先；被占回退动态，实际端口以全局配置目录 `AniKit/mcp-server.port` 为准）。各 Agent 工具配置：

- Kimi Code：`mcp.json` 加 `{ "mcpServers": { "anikit": { "url": "http://127.0.0.1:17701/mcp" } } }`（用户级 `~/.kimi-code/mcp.json` 或项目级 `.kimi-code/mcp.json`；会话中途添加只对新会话生效）
- Codex：`config.toml` 加 `[mcp_servers.anikit]` + `url = "http://127.0.0.1:17701/mcp"`，重启会话
- Claude Code：`claude mcp add --transport http anikit http://127.0.0.1:17701/mcp`

## 工具速览（11 个 = 只读 8 + 写 3）

| 工具 | 用途 |
| --- | --- |
| `get_current_library` | 当前空间 / 素材库状态（先调它确认库已打开） |
| `search_assets` | 分词搜索（空格 AND）；`folderId` 可限定文件夹 |
| `list_folder_tree` | 文件夹树（区块 + 节点，拿文件夹 id） |
| `list_placements` | 文件夹资产清单，带 `thumbPath` 缩略图绝对路径 |
| `get_asset_detail` | 资产详情：属性词 / 描述 / 注释 / 链接 / 主色 / 归属 |
| `get_asset_origin_path` | 源文件与缩略图磁盘绝对路径（看图入口） |
| `read_text_asset` | 文本资产内容（txt / md，提示词、剧本等） |
| `list_archives` | 档案列表（封面、属性、描述等） |
| `create_document`（写） | 新建文档入库（md / txt / fla / psd / ai / docx / xlsx），回 PlacementView；**进撤销** |
| `write_text_asset`（写） | 整体覆写文本资产内容（提示词正文等），size / modifiedAt 同步；**不进撤销，先备份** |
| `rename_placements`（写） | 批量改名（单条 = 1 项数组），失败跳过不整体回滚，回 BatchRenameReport；**进撤销** |

## 写入开关（二期，默认关）

三个写工具受**全局写入开关**门禁：APP 菜单栏「偏好设置」→「允许 MCP 写入」（持久化于全局 config.json `settings.ui.mcpWriteEnabled`，默认关）。开关**逐次生效**——写调用每次都现查：关态一律回工具级错误「MCP 写入未开启（偏好设置里可开启）」，开态直接执行。写前建议流程：

1. `write_text_asset` 覆写内容**不进撤销**（与 APP 内编辑同口径）——先 `read_text_asset` 留底再覆写；
2. `create_document` / `rename_placements` 进撤销——用户在 APP 内 Ctrl+Z 可整步撤回，但仍请克制调用、批量改名一次给全数组；
3. 删除类操作不存在于本通道（L4 永远人工确认），不要尝试构造。

## 典型流程（批量看资产写提示词）

1. `get_current_library` 确认库已打开（未开时其余工具回「请先打开素材库」）；
2. `search_assets` 搜关键词，或 `list_folder_tree` → `list_placements` 按文件夹浏览；
3. `get_asset_origin_path` 拿 `thumbPath`（webp 缩略图）/ `originPath`（源图）**磁盘绝对路径，直接用文件读取能力看图**，不经协议传图；
4. `get_asset_detail` 看既有标签 / 描述 / 注释作参考；
5. `read_text_asset` 读提示词模板、剧本等文本资产；开关开启后可 `create_document` 建新提示词文档 → `write_text_asset` 写入 → `rename_placements` 改规范名。

## 纪律红线

- **写入先确认开关**：写工具默认关闸，回「MCP 写入未开启」时提示用户去偏好设置开启，不要重试刷屏。
- **路径不缓存**：资产磁盘名随库内重命名同步变化，路径必须每次现取现用。
- 浏览走文件夹树 / 搜索入口按 id 直达，不要全目录枚举 assets 池。

## 故障排查

- 裸浏览器 / 无 `Accept: text/event-stream` 头的请求访问 `/mcp` 回 406「Client must accept text/event-stream」——规范行为，不是故障。
- 连不上先确认 APP 在运行；端口异常时读 `mcp-server.port` 拿实际端口。
