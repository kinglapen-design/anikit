# 3. AI 外接通道（MCP，一期只读 + 二期写工具）

- 服务：`server/mcp/`（PRD 08-06），`http://127.0.0.1:17701/mcp`——**固定端口 17701 优先**（Agent 工具只读静态 config，动态端口每重启失效不匹配；被占回退动态），lib.rs setup 拉起（`server::init_mcp`）；端口文件 = 全局配置目录 `AniKit/mcp-server.port`（每次启动覆写实际端口，兜底发现）。
- 实现：官方 Rust SDK `rmcp` 3.4（Streamable HTTP server 为 Tower service，axum `nest_service("/mcp", …)` 挂载；复用 Tauri tokio 运行时，不另起进程）。协议握手 / 会话管理 / 工具 schema 协商全由 SDK 承担。
- 安全口径：仅回环监听、无鉴权（本机 Agent 场景）；Host 头校验防 DNS rebinding 由 rmcp 内置（≥1.4.0，默认 allowedHosts = localhost / 127.0.0.1 / ::1）。**测试构建不起 server**（cfg(not(test))，踩坑 014 同 plugin_ws）。
- **写入闸（二期，2026-09-19 用户定形态 A）**：全局写入开关 + 撤销兜底——开关持久化 config.json `settings.ui.mcpWriteEnabled`（默认 false，偏好设置弹窗「允许 MCP 写入」开启）；MCP 写调用**逐次检查**（`write_gate` 直读配置，core::config 无缓存、低频调用直读可接受），关态 / 读取失败一律回工具级错误「MCP 写入未开启（偏好设置里可开启）」（isError=true，与「请先打开素材库」同口径）。不做逐笔弹窗（首批全是 L2 可逆操作，二批结构性操作可逆仍走形态 A）；删除永不经 MCP（L4 永远人工确认）。

## 握手流程（Streamable HTTP，单端点 POST /mcp）

1. POST `initialize`（头：`Accept: application/json, text/event-stream`、`Content-Type: application/json`；协议版本 2025-03-26 或更新）→ 回包 SSE 帧（`data:` 行 JSON-RPC），响应头带 `Mcp-Session-Id`；`serverInfo = { name: "anikit", version: <app 版本> }`，capabilities 仅 `tools`。
2. POST `notifications/initialized`（带 `Mcp-Session-Id` 头）→ 202 无体。
3. 之后 `tools/list` / `tools/call` 均 POST 同端点、带 session 头；回包同为 SSE `data:` 帧。
4. 会话内存态（LocalSessionManager），APP 重启即失效——Agent 重连重新 initialize 即可。

## 工具面（15 个 = 一期只读 8 + 二期写 7）

加工具 = `server/mcp/tools/` 加一个文件 + `mod.rs` 注册处一行（工具注册表模式，范式 = 前端 barSpecs；未来复用为插件 API 注册表）。回包统一：`content` 文本（pretty JSON）+ `structuredContent` 双带；库未打开回工具级错误（`isError: true`，文本「请先打开素材库」），不静默空表；域错误（资产不存在等）同口径回中文原因。

只读 8 个（一期，全部 `annotations.readOnlyHint = true`）：

| 工具 | 入参 | 回包（同命令层结构） |
| --- | --- | --- |
| `get_current_library` | 无 | `{ space: SpaceInfo \| null, library: LibraryInfo \| null }`——探状态用，未开库不算错误 |
| `search_assets` | `{ query: string, folderId?: string, recursive?: boolean }` | `SearchHits { placements: [{ assetId, folderId }], archives: [folderId] }`。范围口径：不传 folderId = 全库（回收站除外）；传 = 限定该文件夹，recursive 缺省 true 含子级 |
| `list_folder_tree` | 无 | `FolderTreeView { blocks: [{ id, name, icon }], nodes: [{ id, name, count, totalSize, icon, kind, color, groups?, children }] }` |
| `list_placements` | `{ folderId?: string }`（"root" = 未分类；缺省 = 全部） | `PlacementView[]`（assetId / folderId / name / 宽高 / ext / **thumbPath 缩略图绝对路径** / size / duration / tags 等，字段见 `services/views.rs`） |
| `get_asset_detail` | `{ assetId }` | `AssetDetail`（originName / ext / size / 宽高 / duration / tags / descriptions / colors / links / note / placements 反查 / archives 归属）。**纯读**——不走 APP 侧 ensure_asset_colors 懒回填，存量资产 colors 可能为空 |
| `get_asset_origin_path` | `{ assetId }` | `{ originPath, thumbPath: string \| null }`——源文件真实路径（序列帧 = frames/ 封面帧，同右键「复制文件路径」口径）+ 缩略图路径（含 thumbAssetId 伴生代理：fla 等工程资产取伴生 hidden 资产的图） |
| `read_text_asset` | `{ assetId }` | `{ content }`（txt / md；UTF-8 优先 GBK 回退，2MB 上限；非文本资产报错） |
| `list_archives` | 无 | `ArchiveListItem[]`（folderId / badge / avatar / props / descriptions / links / createdAt / totalSize / coverPath / coverVer）；folderId 可交给 list_placements 列档案内容 |

写 7 个（二期，全部 `annotations.readOnlyHint = false`）：公共门禁 = **write_gate 逐次检查** `settings.ui.mcpWriteEnabled`（关态回「MCP 写入未开启（偏好设置里可开启）」工具级错误）。撤销口径分流——进撤销 5 个（create_document / rename_placements / create_folder / move_placements / link_placements）复用 `commands::operations` 抽出的 `_op` 整段（锁 library → 锁 undo → 单写点 + 单撤销步，与 APP 命令同一实现）；不进撤销 2 个 = write_text_asset（与 APP 内文本编辑同口径，2026-08-23 定）+ import_paths（APP 导入管线本就不接 UndoManager，不硬接）。

首批 3 个（2026-09-19，`tools/write.rs`）：

| 工具 | 入参 | 回包（同命令层结构） |
| --- | --- | --- |
| `create_document` | `{ kind: string, folderId?: string }`（kind = md / txt / fla / psd / ai / docx / xlsx；folderId 缺省 = 根目录） | `PlacementView`。**进撤销**（撤销 = info 目录移入 cache/tmp 备份，可重做） |
| `write_text_asset` | `{ assetId, content }`（整体覆写，恒 UTF-8 无 BOM，2MB 上限） | `{ size, modifiedAt }`（metadata 同步落盘）。**不进撤销**——与 APP 内文本编辑同口径（2026-08-23 定：库操作撤销体系不覆盖内容编辑），工具描述注明提醒 Agent 先备份 |
| `rename_placements` | `{ items: [{ assetId, folderId, origin?: boolean, newName }] }`（单条改名 = 1 项数组；origin = true 连磁盘 originName + 词干缩略图同步改；空数组回「items 至少 1 项」） | `BatchRenameReport { renamed, failed: [{ name, reason }] }`——失败项跳过不整体回滚。**进撤销**（整体单撤销步） |

二批结构性 4 个（2026-09-19，`tools/structure.rs` + `tools/import.rs`）：

| 工具 | 入参 | 回包（同命令层结构） |
| --- | --- | --- |
| `create_folder` | `{ name: string, parentId?: string }`（parentId 缺省 / "root" = 顶层；空名 / Windows 非法字符拒绝） | `Folder`（含新 folderId）。**进撤销** |
| `move_placements` | `{ items: [{ assetId, folderId, origin? }], targetFolderId }`（"root" = 未分类；去重 / 原地 / 陈旧项跳过不计） | `{ moved: number }`（实际移动条数）。**进撤销**（整体单撤销步；root → 非 root 自动摘 importedFrom 标，metadata 快照随撤销步） |
| `link_placements` | `{ items: [{ assetId, folderId, origin? }], targetFolderId, names?: (string \| null)[] }`（names 与 items 等长对齐，null 项 = 用源 Placement 名；缺省 = 全部源名） | `{ linked: number }`。资产本体留原地、目标文件夹建链接 Placement（不占双份磁盘）。**进撤销**（整体单撤销步） |
| `import_paths` | `{ paths: string[], folderId?: string }`（仅文件绝对路径，文件夹请展开；复制语义源文件不动；folderId 缺省 / "root" = 未分类） | `ImportReport { items: [{ path, status, assetId?, finalName?, reason? }], imported, skippedDuplicate, renamed, failed }`——**冲突策略 = 方案一**（2026-09-19 用户定，不阻断）：hash 完全相同 → `skippedDuplicate` + 回已有资产 id；同名不同内容 → 加 `-序号` 后缀入库 → `renamed` + finalName；缩略图随管线自动生成。**不进撤销**；同步执行（回包即最终态，无进度事件、不支持取消） |

## 纪律（PRD 08-06 定）

- **只读白名单**：有副作用的准只读命令（get_media_source / get_waveform / list_sequence_frames / prepare_anim_strips / get_asset_file_path 等触发转码 / 后台生成队列类）**不进**只读白名单。
- **看图零集成**：工具只回磁盘绝对路径，Agent 有文件权限即直读 webp 缩略图与源图，不经协议传图。
- **路径不缓存**：originName 随重命名同步改磁盘名，外部每次现推现取（get_asset_origin_path），不得缓存复用。
- 禁止 assets/ 全目录枚举：以文件夹树 / Placement 为入口按 id 直达。
- **写入闸 = 形态 A（2026-09-19 用户定）**：全局开关 + 撤销兜底（见头部「写入闸」段）；结构性操作（move_placements 移动）仍走形态 A 不另立逐笔弹窗（可逆，撤销兜底）；**删除永不经 MCP**（L4 永远人工确认）。
- **import_paths 冲突策略 = 方案一（2026-09-19 用户定，不阻断）**：hash 完全相同 → 跳过并把已有资产 id 回给 Agent；同名不同内容 → 自动加 `-序号` 后缀入库（沿用库内 unique_name 口径）；逐文件报告可追溯。实现 = 逐文件同步直调既有 `run_import` 四阶段管线（folders.json 节流落盘单写点不破），不另造导入管线。
- 二期余量（随后批）：set_asset_props / set_asset_descriptions。

## 客户端配置示例

```json
{ "mcpServers": { "anikit": { "url": "http://127.0.0.1:17701/mcp" } } }
```
