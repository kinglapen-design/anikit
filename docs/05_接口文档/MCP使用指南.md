# MCP 使用指南（给 Agent 与接入者）

AniKit 进程内嵌的 MCP server（PRD 08-06：一期只读 8 工具 + 二期写工具 9 个，写入闸形态 A）。本文是接入与使用说明；协议层细节（握手 / 回包结构 / 错误口径）见 [MCP通道](MCP通道.md)。

## 连接

服务随 APP 启动，地址恒定 `http://127.0.0.1:17701/mcp`（固定端口优先；被占回退动态，实际端口以全局配置目录 `AniKit/mcp-server.port` 为准）。各 Agent 工具配置：

- Kimi Code：`mcp.json` 加 `{ "mcpServers": { "anikit": { "url": "http://127.0.0.1:17701/mcp" } } }`（用户级 `~/.kimi-code/mcp.json` 或项目级 `.kimi-code/mcp.json`；会话中途添加只对新会话生效）
- Codex：`config.toml` 加 `[mcp_servers.anikit]` + `url = "http://127.0.0.1:17701/mcp"`，重启会话
- Claude Code：`claude mcp add --transport http anikit http://127.0.0.1:17701/mcp`

## 工具速览（17 个 = 只读 8 + 写 9）

| 工具 | 用途 |
| --- | --- |
| `get_current_library` | 当前空间 / 素材库状态（先调它确认库已打开） |
| `search_assets` | 分词搜索（空格 AND）；`folderId` 可限定文件夹 |
| `list_folder_tree` | 文件夹树（区块 + 节点，拿文件夹 id；节点带 groups 分组配置） |
| `list_placements` | 文件夹资产清单，带 `thumbPath` 缩略图绝对路径 |
| `get_asset_detail` | 资产详情：属性词 / 描述 / 注释 / 链接 / 主色 / 归属 |
| `get_asset_origin_path` | 源文件与缩略图磁盘绝对路径（看图入口） |
| `read_text_asset` | 文本资产内容（txt / md，提示词、剧本等） |
| `list_archives` | 档案列表（封面、属性、描述等） |
| `create_document`（写） | 新建文档入库（md / txt / fla / psd / ai / docx / xlsx），回 PlacementView；**进撤销** |
| `write_text_asset`（写） | 整体覆写文本资产内容（提示词正文等），size / modifiedAt 同步；**不进撤销，先备份** |
| `rename_placements`（写） | 批量改名（单条 = 1 项数组），失败跳过不整体回滚，回 BatchRenameReport；**进撤销** |
| `create_folder`（写） | 新建文件夹（`parentId` 缺省 = 顶层），回 Folder 拿新 folderId；**进撤销** |
| `move_placements`（写） | 资产移入文件夹（Placement 级，去重 / 原地自动跳过），回 `{ moved }`；**进撤销** |
| `move_folders`（写） | 移动文件夹重设父级（仅改数据不动磁盘；`targetParentId` 缺省 = 顶层），回 `{ moved }`；**进撤销** |
| `link_placements`（写） | 链接汇总：资产留原地、目标文件夹建链接 Placement，回 `{ linked }`；**进撤销** |
| `import_paths`（写） | 导入磁盘文件入库（缩略图自动生成、导入即可看图）；冲突不阻断——hash 相同跳过回已有 id / 同名不同内容加 `-序号` 后缀；逐文件报告；**不进撤销** |
| `trash_placements`（写） | 丢到库内回收站（Placement 进 recycle.json 快照，磁盘文件不动），回 `{ requested }`；**进撤销**（Ctrl+Z 可撤、回收站可还原）。彻底删除 / 清空回收站不存在于本通道，须 APP 内人工 |

## 写入开关（二期，默认关）

九个写工具受**全局写入开关**门禁：APP 菜单栏「偏好设置」→「允许 MCP 写入」（持久化于全局 config.json `settings.ui.mcpWriteEnabled`，默认关）。开关**逐次生效**——写调用每次都现查：关态一律回工具级错误「MCP 写入未开启（偏好设置里可开启）」，开态直接执行。写前建议流程：

1. `write_text_asset` 覆写内容**不进撤销**（与 APP 内编辑同口径）——先 `read_text_asset` 留底再覆写；
2. `import_paths` **不进撤销**（导入 = 资产池新增，撤销体系不覆盖）——但冲突策略不阻断：重复导入同文件只会 `skippedDuplicate` 回已有 id，放心重试；
3. 其余七个（create_document / rename_placements / create_folder / move_placements / move_folders / link_placements / trash_placements）进撤销——用户在 APP 内 Ctrl+Z 可整步撤回，但仍请克制调用、批量改名一次给全数组；
4. 彻底删除类操作不存在于本通道（L4 永远人工确认），不要尝试构造；丢回收站用 `trash_placements`（可逆）。

## 典型流程

### 批量看资产写提示词（一期只读链路）

1. `get_current_library` 确认库已打开（未开时其余工具回「请先打开素材库」）；
2. `search_assets` 搜关键词，或 `list_folder_tree` → `list_placements` 按文件夹浏览；
3. `get_asset_origin_path` 拿 `thumbPath`（webp 缩略图）/ `originPath`（源图）**磁盘绝对路径，直接用文件读取能力看图**，不经协议传图；
4. `get_asset_detail` 看既有标签 / 描述 / 注释作参考；
5. `read_text_asset` 读提示词模板、剧本等文本资产。

### 漫剧生产链路（二期写入，开关开启后）

外部生成图 → 入库 → 写提示词 → 归档到场次文件夹的完整闭环：

1. `list_folder_tree` 拿目标场次文件夹 id（如 `…/04_分镜/第六集/场_01`）；场次文件夹的 groups 是**规则组**（按扩展名 / 名称关键字自动归组，如「提示词」组收 md、「P1」组收名含 P1 的 mp4）——入库 / 改名即自动归组，无需手动归组调用；
2. `import_paths` 把外部生成的图导入目标文件夹（`folderId`）；逐文件报告拿回 assetId——重复导入同文件 = `skippedDuplicate` 直接拿已有 id 用，同名不同内容 = `renamed` 自动加 `-序号` 后缀；
3. `create_document(kind="md", folderId=场次 id)` 建提示词文档 → `write_text_asset` 写入提示词正文（先建后写；覆写前记得 read 留底）→ `rename_placements` 改成规范名（规则组按名字 / 扩展名自动归进「提示词」组）；
4. 整理归档：`move_placements` 把资产移入目标文件夹；`link_placements` 把大池里的资产以链接放进场次文件夹（本体留原地，不占双份磁盘）——「资产留大池、链接进场次」的用户习惯配套；
5. 每步回包里的 assetId / folderId 是后续调用的定位凭据；路径一律现取（`get_asset_origin_path`），禁止缓存。

## 纪律红线

- **写入先确认开关**：写工具默认关闸，回「MCP 写入未开启」时提示用户去偏好设置开启，不要重试刷屏。
- **路径不缓存**：资产磁盘名随库内重命名同步变化，路径必须每次现取现用。
- 浏览走文件夹树 / 搜索入口按 id 直达，不要全目录枚举 assets 池。

## 省 token（2026-09-19 用户实测后补）

MCP 进 Agent 上下文的开销分三块，对应三招：

- **工具 schema 固定开销**（每会话一次）：17 个工具的说明 + 参数 schema。Kimi Code 侧可在 mcp.json 给 anikit 加 `"deferred": true`（按需加载，见 Kimi Code MCP 文档），不用工具时不占上下文。
- **回包体积**（每次调用）：本通道已做两刀——回包仅 `content` 紧凑 JSON、不带 structuredContent 重复副本。Agent 侧配合：先 `search_assets` 缩小范围再按需 `get_asset_detail`，避免不带 folderId 的全库 `list_placements`（几百条 Placement 全字段很占额度）。
- **图片读取**：看图读 `thumbPath`（webp 缩略图，几百 token 级），非必要不读源图（大图千 token 级）。

## 故障排查

- 裸浏览器 / 无 `Accept: text/event-stream` 头的请求访问 `/mcp` 回 406「Client must accept text/event-stream」——规范行为，不是故障。
- 连不上先确认 APP 在运行；端口异常时读 `mcp-server.port` 拿实际端口。
