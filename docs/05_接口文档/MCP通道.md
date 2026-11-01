# 3. AI 外接通道（MCP，一期只读 + 二期写工具）

- 服务：`server/mcp/`（PRD 08-06），`http://127.0.0.1:17701/mcp`——**固定端口 17701 优先**（Agent 工具只读静态 config，动态端口每重启失效不匹配；被占回退动态），lib.rs setup 拉起（`server::init_mcp`，读服务总开关决定是否启动）；端口文件 = 全局配置目录 `AniKit/mcp-server.port`（每次启动覆写实际端口，兜底发现；停服即删）。显式 `ANIKIT_CONFIG_DIR` 的隔离验收改为写该目录下的同名文件，不覆盖用户实例的端口文件。
- **服务总开关与面板命令（2026-09-20 插件面板 MCP 详情窗配套）**：全局句柄 `MCP_HANDLE`（stop 信号 + 实际端口 + serve 任务）支持运行期启停（graceful shutdown）；配置 `settings.mcp` 子树（`serverEnabled` 缺省 true / `writeEnabled` 缺省 false，core::config 单写点，旧 `settings.ui.mcpWriteEnabled` 一次性迁移）。命令：`get_mcp_status`（running / port / url / serverEnabled / writeEnabled）/ `set_mcp_server_enabled`（先启停后持久化）/ `set_mcp_write_enabled`（持久化即生效）——回包同构 McpStatus，前端插件面板详情窗（plugins/McpPluginWindow）直调，不经 ui store 管道。
- 实现：官方 Rust SDK `rmcp` 3.4（Streamable HTTP server 为 Tower service，axum `nest_service("/mcp", …)` 挂载；复用 Tauri tokio 运行时，不另起进程）。协议握手 / 会话管理 / 工具 schema 协商全由 SDK 承担。
- 安全口径：仅回环监听、无鉴权（本机 Agent 场景）；Host 头校验防 DNS rebinding 由 rmcp 内置（≥1.4.0，默认 allowedHosts = localhost / 127.0.0.1 / ::1）。**测试构建不起 server**（cfg(not(test))，踩坑 014 同 plugin_ws）。
- **写入闸（二期，2026-09-19 用户定形态 A；2026-09-20 开关挪插件面板 MCP 详情窗）**：全局写入开关 + 撤销兜底——开关持久化 config.json `settings.mcp.writeEnabled`（默认 false，插件面板「AI 外接通道（MCP）」详情窗「允许写入」开启；初版落点 settings.ui.mcpWriteEnabled 由后端一次性迁移）；MCP 写调用**逐次检查**（`write_gate` 直读配置，core::config 无缓存、低频调用直读可接受），关态 / 读取失败一律回工具级错误「MCP 写入未开启（插件面板 MCP 详情窗里可开启）」（isError=true，与「请先打开素材库」同口径）。不做逐笔弹窗（首批全是 L2 可逆操作，二批结构性操作可逆仍走形态 A）；**彻底删除永不经 MCP**（L4 永远人工确认；库内回收站可逆删除经 trash_placements，2026-09-20 用户定）。
- **写入热刷新（2026-09-19 补）**：库数据文件无监听（命令层写由前端自刷新），MCP 绕开前端——素材结构写工具成功路径广播 `mcp-library-changed`（`emit_library_changed`；import_paths 有真实入库才发），前端 `initMcpListener` 去抖 300ms 全量对齐（同 cep-imported 的 refresh 口径，不带成功提示音）。语义任务另发 `semantic-analysis-progress`，不进素材结构撤销栈。

## 握手流程（Streamable HTTP，单端点 POST /mcp）

1. POST `initialize`（头：`Accept: application/json, text/event-stream`、`Content-Type: application/json`；协议版本 2025-03-26 或更新）→ 回包 SSE 帧（`data:` 行 JSON-RPC），响应头带 `Mcp-Session-Id`；`serverInfo = { name: "anikit", version: <app 版本> }`，capabilities 仅 `tools`。
2. POST `notifications/initialized`（带 `Mcp-Session-Id` 头）→ 202 无体。
3. 之后 `tools/list` / `tools/call` 均 POST 同端点、带 session 头；回包同为 SSE `data:` 帧。
4. 会话内存态（LocalSessionManager），APP 重启即失效——Agent 重连重新 initialize 即可。

## 工具面（40 个 = 只读 13 + 写 27）

加工具 = `server/mcp/tools/` 加一个文件 + `mod.rs` 注册处一行（工具注册表模式，范式 = 前端 barSpecs；未来复用为插件 API 注册表）。回包统一：**仅 `content` 文本（紧凑 JSON），不带 structuredContent**——同一份数据双带会被不查重的客户端双倍计入上下文（2026-09-19 用户实测额度消耗后定的省 token 口径；未声明 outputSchema 时 structuredContent 本就可选）；库未打开回工具级错误（`isError: true`，文本「请先打开素材库」），不静默空表；域错误（资产不存在等）同口径回中文原因。

**URI 增强（2026-10-08 S5，PRD 02-09）**：①所有收 assetId / folderId 的工具参数经 `core::anikit_ref::normalize_ref_id` 归一化（Params struct 字段挂 serde deserialize_with）——`anikit://item|folder/{id}` 与 `http(s)://127.0.0.1:{port}/(open/)item|folder/{id}` 链接形态直接吃，AI 整段透传用户粘贴的链接即可；②`search_assets` placements 每项、`get_asset_detail` 顶层与 `list_links` 逐项附 `uri` 字段（http open 形态 `http://127.0.0.1:{port}/open/item/{id}`，server/media.rs 端点——AI 回复用户时可点击回 AniKit；新增字段不破坏既有解析）。

只读 13 个（全部 `annotations.readOnlyHint = true`）：

| 工具 | 入参 | 回包（同命令层结构） |
| --- | --- | --- |
| `get_current_library` | 无 | `{ space: SpaceInfo \| null, library: LibraryInfo \| null }`——探状态用，未开库不算错误 |
| `search_assets` | `{ query: string, folderId?: string, recursive?: boolean }` | `SearchHits { placements: [{ assetId, folderId, uri }], archives: [folderId] }`（uri = http open 点击跳转链接，2026-10-08 S5）。范围口径：不传 folderId = 全库（回收站除外）；传 = 限定该文件夹，recursive 缺省 true 含子级 |
| `list_folder_tree` | 无 | `FolderTreeView { blocks: [{ id, name, icon }], nodes: [{ id, name, count, totalSize, icon, kind, color, groups?, children }] }` |
| `list_placements` | `{ folderId?: string }`（"root" = 未分类；缺省 = 全部） | `PlacementView[]`（assetId / folderId / name / 宽高 / ext / **thumbPath 缩略图绝对路径** / size / duration / tags 等，字段见 `services/views.rs`） |
| `list_links` | 无 | `LinkView[]`（assetId / ext / link / origin / uri）——全库链接 Placement 清单（2026-10-12 只读补批，PRD 08-06 ⑨）；link 与 origin 均 = 详情链接区块 `PlacementLink` 结构（folderId 为 null = 未分类 root / blockName / path 路径链最多三段 / name），origin = null 表示原始 Placement 已不在库（回收站或彻底删除）；uri = http open 点击跳转链接（S5 同口径） |
| `get_asset_detail` | `{ assetId }` | `AssetDetail`（originName / ext / size / 宽高 / duration / tags / descriptions / colors / links / note / placements 反查 / archives 归属 / thumbAsset 调用侧伴生缩略图 / thumbUsedBy 被引用清单——后两项 2026-09-23 缩略图区块接业务补输出）+ 顶层 `uri`（http open 点击跳转链接，2026-10-08 S5）。**纯读**——不走 APP 侧 ensure_asset_colors 懒回填，存量资产 colors 可能为空 |
| `get_asset_origin_path` | `{ assetId }` | `{ originPath, thumbPath: string \| null }`——源文件真实路径（序列帧 = frames/ 封面帧，同右键「复制文件路径」口径）+ 缩略图路径（含 thumbAssetId 伴生代理：fla 等工程资产取伴生 hidden 资产的图） |
| `read_text_asset` | `{ assetId }` | `{ content }`（txt / md；UTF-8 优先 GBK 回退，2MB 上限；非文本资产报错） |
| `list_archives` | 无 | `ArchiveListItem[]`（folderId / badge / avatar / props / descriptions / links / createdAt / totalSize / coverPath / coverVer）；folderId 可交给 list_placements 列档案内容 |
| `get_archive_data` | `{ folderId }` | `ArchiveDetailView`（ArchiveData 全字段 + fileCount / totalSize / coverPath / coverVer / prompt）；无数据记录时回默认空记录 + 统计 |
| `semantic_search` | `{ query, folderId?, recursive?, limit? }` | 已有语义索引的最佳片段、其他命中段、真实毫秒 / 原始帧索引与 uri；只查询，不自动分析，不调用外部 provider；limit 为资产数，缺省 30、上限 100 |
| `get_semantic_analysis_task` | `{ taskId }` | 当前库单个任务快照（含 mode / status / completed / errors）；不存在或不属于当前库 = null |
| `list_semantic_analysis_tasks` | 无 | 当前库任务清单，最多 50 个，其他库不可见 |

写 27 个（全部 `annotations.readOnlyHint = false`）：公共门禁 = **write_gate 逐次检查** `settings.mcp.writeEnabled`（关态回「MCP 写入未开启（插件面板 MCP 详情窗里可开启）」工具级错误）。撤销口径分流——进撤销 20 个（create_document / rename_placements / rename_folders / create_folder / move_placements / move_folders / link_placements / trash_placements / assign_prompt / clear_prompt / set_folder_groups / create_semantic_segment + 档案批 create_archive / convert_to_archive / convert_to_folder / set_archive_props / set_archive_descriptions / set_archive_links / delete_archive）复用 `commands::operations` / `commands::archive_ops` / `commands::prompt` 抽出的 `_op` 整段（锁 library → 锁 undo → 单写点 + 单撤销步，与 APP 命令同一实现）；不进撤销 8 个 = write_text_asset（与 APP 内文本编辑同口径，2026-08-23 定）+ import_paths（APP 导入管线本就不接 UndoManager，不硬接）+ 六个语义任务工具（检查点恢复，不修改素材权威 JSON）。

首批 3 个（2026-09-19，`tools/write.rs`）：

| 工具 | 入参 | 回包（同命令层结构） |
| --- | --- | --- |
| `create_document` | `{ kind: string, folderId?: string }`（kind = md / txt / fla / psd / ai / docx / xlsx；folderId 缺省 = 根目录） | `PlacementView`。**进撤销**（撤销 = info 目录移入 cache/tmp 备份，可重做） |
| `write_text_asset` | `{ assetId, content }`（整体覆写，恒 UTF-8 无 BOM，2MB 上限） | `{ size, modifiedAt }`（metadata 同步落盘）。**不进撤销**——与 APP 内文本编辑同口径（2026-08-23 定：库操作撤销体系不覆盖内容编辑），工具描述注明提醒 Agent 先备份 |
| `rename_placements` | `{ items: [{ assetId, folderId, origin?: boolean, newName, oldName?: string }] }`（单条改名 = 1 项数组；origin = 定位三元组之一，true 原始 / false 链接，照 list_placements 回包抄——**缺省 = 自动定位**：该文件夹内仅此资产一条记录即命中，多条（origin + 链接同文件夹）进 failed 要求指定；**oldName（2026-10-11）= 同 origin 重复链接按旧显示名精确到行**，origin 缺省时 oldName 唯一命中亦参与消歧；命中 origin 记录连磁盘 originName + 词干缩略图同步改；定位落空但文件夹内有记录时 reason 带实际 origin 值。空数组回「items 至少 1 项」） | `BatchRenameReport { renamed, failed: [{ name, reason }] }`——失败项跳过不整体回滚。**进撤销**（整体单撤销步） |

二批结构性 5 个（2026-09-19，`tools/structure.rs` + `tools/import.rs`）：

| 工具 | 入参 | 回包（同命令层结构） |
| --- | --- | --- |
| `create_folder` | `{ name: string, parentId?: string }`（parentId 缺省 / "root" = 顶层；空名 / Windows 非法字符拒绝） | `Folder`（含新 folderId）。**进撤销** |
| `move_placements` | `{ items: [{ assetId, folderId, origin? }], targetFolderId }`（"root" = 未分类；去重 / 原地 / 陈旧项跳过不计） | `{ moved: number }`（实际移动条数）。**进撤销**（整体单撤销步；root → 非 root 自动摘 importedFrom 标，metadata 快照随撤销步） |
| `move_folders` | `{ folderIds: string[], targetParentId?: string }`（targetParentId 缺省 / "root" = 顶层文件夹区块；落点 = 同级末尾，不给排序插入位；移入自身或子孙整条报错，同父 / 陈旧 id 跳过；空数组回「folderIds 至少 1 项」） | `{ moved: number }`。**进撤销**（整体单撤销步；仅改数据不动磁盘，2026-09-19 同日用户补开：结构重组留一手） |
| `link_placements` | `{ items: [{ assetId, folderId, origin? }], targetFolderId, names?: (string \| null)[] }`（names 与 items 等长对齐，null 项 = 用源 Placement 名；缺省 = 全部源名） | `{ linked: number }`。资产本体留原地、目标文件夹建链接 Placement（不占双份磁盘）。**进撤销**（整体单撤销步） |
| `import_paths` | `{ paths: string[], folderId?: string }`（仅文件绝对路径，文件夹请展开；复制语义源文件不动；folderId 缺省 / "root" = 未分类） | `ImportReport { items: [{ path, status, assetId?, finalName?, reason? }], imported, skippedDuplicate, renamed, failed }`——**冲突策略 = 方案一**（2026-09-19 用户定，不阻断）：hash 完全相同 → `skippedDuplicate` + 回已有资产 id；同名不同内容 → 加 `_序号` 后缀入库 → `renamed` + finalName；缩略图随管线自动生成。**不进撤销**；同步执行（回包即最终态，无进度事件、不支持取消） |

三批回收站删除 1 个（2026-09-20，`tools/trash.rs`）：

| 工具 | 入参 | 回包（同命令层结构） |
| --- | --- | --- |
| `trash_placements` | `{ items: [{ assetId, folderId, origin? }] }`（单条 = 1 项数组；陈旧 / 重复三元组自动跳过不计；空数组回「items 至少 1 项」） | `{ requested: number }`（请求条数）。Placement 从 folders.json 移入 recycle.json 按资产聚合，**磁盘文件不动**。**进撤销**（整体单撤销步；APP 内 Ctrl+Z 可撤、回收站可还原）。红线：彻底删除 / 清空回收站永不经 MCP，须 APP 内人工 |

提示词批 2 个（2026-09-26，`tools/prompt.rs`）：

| 工具 | 入参 | 回包（同命令层结构） |
| --- | --- | --- |
| `assign_prompt` | `{ assetId? / folderId?, promptAssetId }`（目标二选一——assetId 资产 / folderId 档案；promptAssetId = 库内 md/txt 文本资产） | 成功空 content。提示词 = 引用 + 内容快照（源被彻底删除不断链，快照保留）。**进撤销** |
| `clear_prompt` | `{ assetId? / folderId? }`（目标二选一） | 成功空 content。断开提示词绑定，快照随解绑清除。**进撤销** |

四批分组 1 个（2026-10-08，`tools/groups.rs`）：

| 工具 | 入参 | 回包（同命令层结构） |
| --- | --- | --- |
| `set_folder_groups` | `{ folderId, groups: [{ id?, name, rules?: [{ kind, values }], members? }] }`——**全量草稿式提交** = 该文件夹分组终态数组（空数组 = 取消分组、漏交 = 删组）；id 缺省 / 空 = 新建组（后端 `G` 系生成），改既有组照 `list_folder_tree` 回包抄。rules 空 = 自由组（members = assetId 手动归组）；rules 非空 = 规则组（组内 AND：ext 扩展名 / type 八类 / name 名称词 / importDate / modifyDate 日期档 / tags 标签词；values 空数组拒绝；规则集完全相同的组拒绝） | 终态 `GroupDef[]`（新建组含生成 id）。**进撤销**（整体单撤销步）。assign_group 不设工具（PRD 2026-09-19 结论维持，自由组成员经 members 全量管理） |

五批文件夹改名 1 个（2026-10-11，`tools/write.rs`——与 rename_placements 同文件）：

| 工具 | 入参 | 回包（同命令层结构） |
| --- | --- | --- |
| `rename_folders` | `{ items: [{ folderId, newName }] }`（单个文件夹改名 = 1 项数组，与 rename_placements 同形；空数组回「items 至少 1 项」） | `BatchRenameReport { renamed, failed: [{ name, reason }] }`——仅改数据不动磁盘（Folder 无磁盘实体）；同名不拦截（文件夹名允许重复）、同名幂等仍计 renamed；失败项（非法名 / 陈旧 id）跳过进 failed 不整体回滚。**进撤销**（整体单撤销步） |

语义任务批 6 个（2026-10-27，`tools/semantic.rs`）：全部受写入闸、只作用当前库、不进素材结构撤销栈，进度经任务检查点恢复。

| 工具 | 入参 | 回包与边界 |
| --- | --- | --- |
| `start_semantic_analysis` | `{ assetIds: string[] }` | 返回 taskId；清单必须非空，不支持 folderId 或缺省全库。启动本地基础画面分析 |
| `start_detailed_semantic_analysis` | `{ assetIds: string[] }` | 返回 taskId；先有同模型基础分析，使用用户已配置的 OpenAI 兼容 / Ollama provider，不配置或回显 API key |
| `pause_semantic_analysis` | `{ taskId }` | 暂停指定任务，保留原 basic / detailed 模式 |
| `resume_semantic_analysis` | `{ taskId }` | 返回原 taskId 或中断恢复的新 taskId，保持原模式 |
| `cancel_semantic_analysis` | `{ taskId }` | 取消指定任务；已取消的未完成项可重试 |
| `retry_semantic_analysis` | `{ taskId }` | 返回新 taskId，重试失败和未完成素材，保持原模式 |

片段创建 1 个（2026-10-27，`tools/semantic.rs`）：

| 工具 | 入参 | 回包与边界 |
| --- | --- | --- |
| `create_semantic_segment` | `{ assetId, segmentId, targetFolderId?, physical? }` | 返回新 `PlacementView`。`physical=false` 为引用源范围的虚拟片段，`true` 为独立物理资产；缺省放到未分类。必须有同模型版本、同分析版本与源指纹一致的命中，不允许嵌套；视频按真实 PTS，动图 / 序列帧按原始帧索引。**进撤销**，成功后广播 `mcp-library-changed` |

档案管理批 7 个写工具（2026-10-31，`tools/archives.rs`；详情只读见上表）：全部复用 `commands/archive_ops.rs`，进撤销并广播 `mcp-library-changed`。

| 工具 | 入参 | 回包与边界 |
| --- | --- | --- |
| `create_archive` | `{ name, parentId?, afterFolderId?, blockId? }` | 返回新 `Folder`。parentId 缺省 / "root" = 顶层；afterFolderId 为可选同级插入锚点，blockId 为可选顶层区块 |
| `convert_to_archive` | `{ folderId, confirmed? }` | 返回 `done` / `need_confirm`。有子文件夹且未确认时不改数据；confirmed=true 后平移子树 Placement、删除子文件夹节点 |
| `convert_to_folder` | `{ folderId }` | 解散为普通文件夹并清 archives.json 记录，Placement 不动；成功回 `{}` |
| `set_archive_props` | `{ folderId, props: string[] }` | 属性词终态整体替换，trim / 去空 / 保序去重；空数组 = 清空；成功回 `{}` |
| `set_archive_descriptions` | `{ folderId, descriptions: [{ title, content }] }` | 描述终态整体替换，标题 / 内容 trim，双空项丢弃；成功回 `{}` |
| `set_archive_links` | `{ folderId, links: string[] }` | 链接终态整体替换，trim / 去空 / 保序去重；成功回 `{}` |
| `delete_archive` | `{ folderId }` | 仅接受档案；返回 `DeleteFolderReport`。删除档案节点与 archives.json 记录，全部 Placement 移未分类，素材实体不动；可撤销，物理删除不开放 |

## 纪律（PRD 08-06 定）

- **只读白名单**：有副作用的准只读命令（get_media_source / list_sequence_frames / prepare_anim_strips / get_asset_file_path 等触发转码 / 后台生成队列类）**不进**只读白名单。
- **看图零集成**：工具只回磁盘绝对路径，Agent 有文件权限即直读 webp 缩略图与源图，不经协议传图。
- **路径不缓存**：originName 随重命名同步改磁盘名，外部每次现推现取（get_asset_origin_path），不得缓存复用。
- 禁止 assets/ 全目录枚举：以文件夹树 / Placement 为入口按 id 直达。
- **写入闸 = 形态 A（2026-09-19 用户定）**：全局开关 + 撤销兜底（见头部「写入闸」段）；结构性操作（move_placements 移动）仍走形态 A 不另立逐笔弹窗（可逆，撤销兜底）；**彻底删除永不经 MCP**（L4 永远人工确认）——库内回收站可逆删除经 trash_placements（2026-09-20 用户定，进撤销）。
- **import_paths 冲突策略 = 方案一（2026-09-19 用户定，不阻断）**：hash 完全相同 → 跳过并把已有资产 id 回给 Agent；同名不同内容 → 自动加 `_序号` 后缀入库（沿用库内 unique_name 口径）；逐文件报告可追溯。实现 = 逐文件同步直调既有 `run_import` 四阶段管线（folders.json 节流落盘单写点不破），不另造导入管线。
- 二期余量（随后批）：set_asset_props / set_asset_descriptions。

## 客户端配置示例

```json
{ "mcpServers": { "anikit": { "url": "http://127.0.0.1:17701/mcp" } } }
```
