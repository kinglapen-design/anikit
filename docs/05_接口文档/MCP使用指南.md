# MCP 使用指南（给 Agent 与接入者）

AniKit 进程内嵌的 MCP server（PRD 08-06：只读 13 工具 + 写工具 27 个，写入闸形态 A）。本文是接入与使用说明；协议层细节（握手 / 回包结构 / 错误口径）见 [MCP通道](MCP通道.md)。

## 连接

服务默认随 APP 启动（插件面板「AI 外接通道（MCP）」详情窗可启停——停服即删端口文件），地址恒定 `http://127.0.0.1:17701/mcp`（固定端口优先；被占回退动态，实际端口以全局配置目录 `AniKit/mcp-server.port` 为准）。各 Agent 工具配置：

- Kimi Code：`mcp.json` 加 `{ "mcpServers": { "anikit": { "url": "http://127.0.0.1:17701/mcp" } } }`（用户级 `~/.kimi-code/mcp.json` 或项目级 `.kimi-code/mcp.json`；会话中途添加只对新会话生效）
- Codex：`config.toml` 加 `[mcp_servers.anikit]` + `url = "http://127.0.0.1:17701/mcp"`，重启会话
- Claude Code：`claude mcp add --transport http anikit http://127.0.0.1:17701/mcp`

## 工具速览（40 个 = 只读 13 + 写 27）

| 工具 | 用途 |
| --- | --- |
| `get_current_library` | 当前空间 / 素材库状态（先调它确认库已打开） |
| `search_assets` | 分词搜索（空格 AND）；`folderId` 可限定文件夹 |
| `list_folder_tree` | 文件夹树（区块 + 节点，拿文件夹 id；节点带 groups 分组配置） |
| `list_placements` | 文件夹资产清单，带 `thumbPath` 缩略图绝对路径 |
| `list_links` | 全库链接清单（2026-10-12）：链接位置 + 原始位置（`PlacementLink` 结构，原始不在库 = null）+ `uri` |
| `get_asset_detail` | 资产详情：属性词 / 描述 / 注释 / 链接 / 主色 / 归属 |
| `get_asset_origin_path` | 源文件与缩略图磁盘绝对路径（看图入口） |
| `read_text_asset` | 文本资产内容（txt / md，提示词、剧本等） |
| `list_archives` | 档案列表（封面、属性、描述等） |
| `get_archive_data` | 单档详情：属性 / 描述 / 链接 / 封面 / 提示词与统计 |
| `create_archive`（写） | 新建档案，回 Folder；`parentId` 缺省 = 顶层；**进撤销** |
| `convert_to_archive`（写） | 文件夹转档案；有子级先回 `need_confirm`，`confirmed=true` 才剔除层级；**进撤销** |
| `convert_to_folder`（写） | 档案解散为普通文件夹并清档案数据记录，Placement 不动；**进撤销** |
| `set_archive_props`（写） | 整体设置档案属性词，trim / 去空 / 保序去重；**进撤销** |
| `set_archive_descriptions`（写） | 整体设置档案描述（标题 + 内容数组）；**进撤销** |
| `set_archive_links`（写） | 整体设置档案链接，trim / 去空 / 保序去重；**进撤销** |
| `delete_archive`（写） | 删除档案节点与数据记录，Placement 移未分类、素材实体不动；**进撤销**，不开放物理删除 |
| `semantic_search` | 中文语义检索已有索引，回最佳片段、其他命中段、毫秒 / 原始帧索引与 uri；不因查询自动分析，也不调用外部 provider |
| `get_semantic_analysis_task` | 当前库单个分析任务快照（含 basic / detailed 模式、进度、真实后端与错误） |
| `list_semantic_analysis_tasks` | 当前库分析任务清单，其他库不可见 |
| `create_document`（写） | 新建文档入库（md / txt / fla / psd / ai / docx / xlsx），回 PlacementView；**进撤销** |
| `write_text_asset`（写） | 整体覆写文本资产内容（提示词正文等），size / modifiedAt 同步；**不进撤销，先备份** |
| `rename_placements`（写） | 批量改名（单条 = 1 项数组），失败跳过不整体回滚，回 BatchRenameReport；`origin` 缺省自动定位（文件夹内唯一记录即命中，多条须照回包指定）；**进撤销** |
| `create_folder`（写） | 新建文件夹（`parentId` 缺省 = 顶层），回 Folder 拿新 folderId；**进撤销** |
| `move_placements`（写） | 资产移入文件夹（Placement 级，去重 / 原地自动跳过），回 `{ moved }`；**进撤销** |
| `move_folders`（写） | 移动文件夹重设父级（仅改数据不动磁盘；`targetParentId` 缺省 = 顶层），回 `{ moved }`；**进撤销** |
| `link_placements`（写） | 链接汇总：资产留原地、目标文件夹建链接 Placement，回 `{ linked }`；**进撤销** |
| `import_paths`（写） | 导入磁盘文件入库（缩略图自动生成、导入即可看图）；冲突不阻断——hash 相同跳过回已有 id / 同名不同内容加 `_序号` 后缀；逐文件报告；**不进撤销** |
| `trash_placements`（写） | 丢到库内回收站（Placement 进 recycle.json 快照，磁盘文件不动），回 `{ requested }`；**进撤销**（Ctrl+Z 可撤、回收站可还原）。彻底删除 / 清空回收站不存在于本通道，须 APP 内人工 |
| `assign_prompt`（写） | 指定提示词（2026-09-26 提示词轮）：把库内 md/txt 文本资产设为目标的提示词——目标 `assetId` / `folderId`（档案）二选一 + `promptAssetId`；引用 + 内容快照（源被彻底删除不断链，快照保留）；**进撤销** |
| `clear_prompt`（写） | 断开提示词：移除目标（`assetId` / `folderId` 二选一）的提示词绑定，快照随解绑清除；**进撤销** |
| `set_folder_groups`（写） | 设置文件夹分组定义（2026-10-08 四批）：全量草稿式提交——`groups` = 该文件夹分组终态数组（空数组 = 取消分组、漏交 = 删组）；改既有组照 `list_folder_tree` 回包抄 id，新组 id 缺省后端生成随回包返回；规则组 = `rules` 非空自动归组，自由组 = 空 `rules` + `members` 手动归组；**进撤销** |
| `rename_folders`（写） | 文件夹改名 / 批量改名（2026-10-11 五批）：`items: [{ folderId, newName }]`，单个文件夹改名 = 1 项数组；仅改数据不动磁盘（文件夹无磁盘实体）、同名不拦截（文件夹名允许重复）、失败项（非法名 / 已删 id）跳过进 failed 汇总，回 BatchRenameReport；**进撤销** |
| `start_semantic_analysis`（写） | 启动基础画面分析，assetIds 必填非空，不支持缺省全库；回 taskId；**不进撤销** |
| `start_detailed_semantic_analysis`（写） | 启动外部详细分析，使用已配置 provider；不配置或回显密钥；**不进撤销** |
| `pause_semantic_analysis`（写） | 暂停指定任务，保持原分析模式；**不进撤销** |
| `resume_semantic_analysis`（写） | 恢复指定任务，返回原 taskId 或中断恢复的新 taskId；**不进撤销** |
| `cancel_semantic_analysis`（写） | 取消指定任务，未完成项可重试；**不进撤销** |
| `retry_semantic_analysis`（写） | 重试失败和未完成素材，返回新 taskId 并保持原模式；**不进撤销** |
| `create_semantic_segment`（写） | 按 `assetId + segmentId` 创建片段；`physical=false` 为引用源范围的虚拟片段，`true` 为 ffmpeg 物理截取；可选 `targetFolderId`；**进撤销** |

## 写入开关（二期，默认关）

二十七个写工具受**全局写入开关**门禁：插件面板「AI 外接通道（MCP）」详情窗 →「允许写入」（持久化于全局 config.json `settings.mcp.writeEnabled`，默认关；2026-09-20 从偏好设置弹窗挪入，旧 settings.ui.mcpWriteEnabled 自动迁移）。开关**逐次生效**——写调用每次都现查：关态一律回工具级错误「MCP 写入未开启（插件面板 MCP 详情窗里可开启）」，开态直接执行。六个语义任务工具不进撤销，恢复靠任务检查点；启动分析必须明确给非空 assetIds，不允许 Agent 默认全库分析。写前建议流程：

1. `write_text_asset` 覆写内容**不进撤销**（与 APP 内编辑同口径）——先 `read_text_asset` 留底再覆写；
2. `import_paths` **不进撤销**（导入 = 资产池新增，撤销体系不覆盖）——但冲突策略不阻断：重复导入同文件只会 `skippedDuplicate` 回已有 id，放心重试；
3. 其余二十个（create_document / rename_placements / rename_folders / create_folder / move_placements / move_folders / link_placements / trash_placements / assign_prompt / clear_prompt / set_folder_groups / create_semantic_segment + 七个档案写工具）进撤销——用户在 APP 内 Ctrl+Z 可整步撤回，但仍请克制调用、批量改名一次给全数组；
4. 彻底删除类操作不存在于本通道（L4 永远人工确认），不要尝试构造；丢回收站用 `trash_placements`（可逆）。`delete_archive` 只删除档案节点与数据记录，素材实体保留，不是物理删除。

## 典型流程

### 批量看资产写提示词（一期只读链路）

1. `get_current_library` 确认库已打开（未开时其余工具回「请先打开素材库」）；
2. `search_assets` 搜关键词，或 `list_folder_tree` → `list_placements` 按文件夹浏览；`list_links` 看全库链接清单（链在哪个文件夹、原件在哪）；
3. `get_asset_origin_path` 拿 `thumbPath`（webp 缩略图）/ `originPath`（源图）**磁盘绝对路径，直接用文件读取能力看图**，不经协议传图；
4. `get_asset_detail` 看既有标签 / 描述 / 注释作参考；
5. `read_text_asset` 读提示词模板、剧本等文本资产。

### 漫剧生产链路（二期写入，开关开启后）

外部生成图 → 入库 → 写提示词 → 归档到场次文件夹的完整闭环：

1. `list_folder_tree` 拿目标场次文件夹 id（如 `…/04_分镜/第六集/场_01`）；场次文件夹的 groups 是**规则组**（按扩展名 / 名称关键字自动归组，如「提示词」组收 md、「P1」组收名含 P1 的 mp4）——入库 / 改名即自动归组，无需手动归组调用；**新建场次文件夹的分组定义用 `set_folder_groups` 一次配好**（照既有场次回包 groups 抄改即可）；
2. `import_paths` 把外部生成的图导入目标文件夹（`folderId`）；逐文件报告拿回 assetId——重复导入同文件 = `skippedDuplicate` 直接拿已有 id 用，同名不同内容 = `renamed` 自动加 `_序号` 后缀；
3. `create_document(kind="md", folderId=场次 id)` 建提示词文档 → `write_text_asset` 写入提示词正文（先建后写；覆写前记得 read 留底）→ `rename_placements` 改成规范名（规则组按名字 / 扩展名自动归进「提示词」组）→ `assign_prompt` 把文档绑定为对应资产 / 档案的提示词（引用 + 快照，APP 详情提示词 tab 与右键「提示词」菜单即时可见）；
4. 整理归档：`move_placements` 把资产移入目标文件夹；`link_placements` 把大池里的资产以链接放进场次文件夹（本体留原地，不占双份磁盘）——「资产留大池、链接进场次」的用户习惯配套；`list_links` 随时回查全库链接及其原始位置；
5. 每步回包里的 assetId / folderId 是后续调用的定位凭据；路径一律现取（`get_asset_origin_path`），禁止缓存。

### 档案管理链路（2026-10-31）

`create_archive` 返回新 folderId 后，用 `set_archive_props` / `set_archive_descriptions` / `set_archive_links` 写档案数据，用 `get_archive_data` 回读，用 `move_placements` 或 `link_placements` 汇总素材。已有文件夹可经 `convert_to_archive` 转换；返回 `need_confirm` 时先确认是否允许剔除子文件夹层级，再以 `confirmed=true` 重调。

`delete_archive` 删除档案节点和数据记录，档案内素材移入未分类，不删除素材实体，APP 内可撤销。只需要去掉档案形态时改用 `convert_to_folder`；改名和移动仍用 `rename_folders` / `move_folders`。

### 语义检索与任务（2026-10-27）

先在 APP 选中素材做基础分析，或经 `start_semantic_analysis` 显式给非空 assetIds；检索不会自动替你分析全库。`semantic_search` 返回每个资产的最佳片段、其他命中段与真实毫秒 / 原始帧索引，可交给 APP 预览定位。任务用 `get_semantic_analysis_task` / `list_semantic_analysis_tasks` 查询，按 taskId 暂停、恢复、取消或重试，恢复保持原 basic / detailed 模式。

详细动作和候选因果经 `start_detailed_semantic_analysis` 使用用户已经配置的 OpenAI 兼容服务或 Ollama。AniKit 不安装或管理 Ollama，也不内置 LLM；MCP 不配置或回显 API key。外部描述与事件仍由本地同一 CLIP 编码，详细分析完成后日常检索不需要再调用 provider。observed 只表示可观察连续事件，hypothesis 是不确定的候选因果，不能把先后关系当成确定因果。

## 纪律红线

- **写入先确认开关**：写工具默认关闸，回「MCP 写入未开启」时提示用户去插件面板 MCP 详情窗开启，不要重试刷屏。
- **路径不缓存**：资产磁盘名随库内重命名同步变化，路径必须每次现取现用。
- 浏览走文件夹树 / 搜索入口按 id 直达，不要全目录枚举 assets 池。
- **id 参数可传链接**（2026-10-08 S5）：assetId / folderId 参数直接吃 `anikit://item|folder/{id}` 与 `http://127.0.0.1:{port}/(open/)item|folder/{id}` 链接形态——用户粘贴的链接整段透传即可，无需自行字符串处理；`search_assets` placements 与 `get_asset_detail` 回包附 `uri` 字段（http open 形态），回复用户时可直接引用（点击回 AniKit 定位）。

## 省 token（2026-09-19 用户实测后补）

MCP 进 Agent 上下文的开销分三块，对应三招：

- **工具 schema 固定开销**（每会话一次）：40 个工具的说明 + 参数 schema。Kimi Code 侧可在 mcp.json 给 anikit 加 `"deferred": true`（按需加载，见 Kimi Code MCP 文档），不用工具时不占上下文。
- **回包体积**（每次调用）：本通道已做两刀——回包仅 `content` 紧凑 JSON、不带 structuredContent 重复副本。Agent 侧配合：先 `search_assets` 缩小范围再按需 `get_asset_detail`，避免不带 folderId 的全库 `list_placements`（几百条 Placement 全字段很占额度）。
- **图片读取**：看图读 `thumbPath`（webp 缩略图，几百 token 级），非必要不读源图（大图千 token 级）。

## 故障排查

- 裸浏览器 / 无 `Accept: text/event-stream` 头的请求访问 `/mcp` 回 406「Client must accept text/event-stream」——规范行为，不是故障。
- 连不上先确认 APP 在运行；端口异常时读 `mcp-server.port` 拿实际端口。
