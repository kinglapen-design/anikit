## 06. AI 外接通道（MCP）

工作重心转向漫剧生产后，AI 的核心诉求是「大量看资产写提示词、批量生成镜头版本」：资源管理器预览不行，导入 APP 后外部 AI 又读不到库。内置 Agent（03 节）太遥远，先用外部 Agent 工具（Kimi Code / Claude Code / Codex 等）经 MCP 协议控制 APP——本节即 03 节 L1~L4 分级授权的外接落地形态：**只读先行、写入闸后置**（立项备忘 2026-09-17，PRD 落地 2026-09-19）。

**目标工作流（2026-09-19 用户描述，工具面设计标尺）**：Agent **看图 → 看剧本 → 拆剧本 → 判断所需资产 → 生成图片 → 导入库 → 命名 → 放进对应文件夹 → 建目录 → 写 MD 文档**。读、写、结构三类能力都在这条链上，缺一即断。

**工作目录架构（同日定）**：Agent 的 cwd = 漫剧项目文件夹（落在 AniKit 工作空间内，如 `workspace/01_projects/…/漫剧_穿越异世界/`：`.agents/` 放工作流说明书、`06_AI_temp/` 当生成图候车区），AniKit 库只当资产归宿、不经 cwd 直接进出——库的一切进出走 MCP 工具。用户库内组织习惯（同日确认）：资产入大池文件夹，每场戏用**链接**汇总进场次文件夹，场次内用**分组**整理（提示词 / P1 / P2……）——故工具面须含 `link_placements`；分组经实证全是**规则组**（「提示词」= ext md、「Pn」= ext mp4 + 名含 Pn，2026-09-19 查库内 folders.json），命名规范 + 链接即自动归组，**不需要 assign_group 工具**。

### 传输选型

原立项备忘推荐「localhost HTTP+SSE 直连、stdio sidecar 次之」；落地前修正：**MCP 规范的独立 HTTP+SSE 传输已废弃**，现行标准传输为 stdio 与 Streamable HTTP（单端点 POST，流式回推内部仍走 SSE 机制），选型按现行规范走。

**定案：AniKit 进程内嵌 Streamable HTTP server**——127.0.0.1 回环 + **固定端口 17701 优先**（被占回退动态；Agent 工具只读静态 config，动态端口每重启失效不匹配此场景，2026-09-19 用户定），端口文件兜底写实际端口，复用 Tauri 的 tokio 运行时不另起进程，Agent 工具侧配 URL 直连。stdio sidecar 列为二期备选：遇到仅支持 stdio 的 Agent 工具时，出一个瘦 sidecar 转发到本通道。

**实现选型（2026-09-19 用户定 A 案，已落地）**：官方 Rust SDK `rmcp` 3.4（≥1.4.0 内置 Host 头校验防 DNS rebinding，CVE-2026-42559）+ axum 0.8 挂载其 Streamable HTTP 服务，协议层零手写；备选「零新增依赖手写 JSON-RPC over HTTP」弃选（协议细节成本高、易错）。

安全口径：回环直连、无鉴权（本机 Agent 场景）；Host 头校验防 DNS rebinding（rmcp 已内置）；token 鉴权列为余量（结构性写工具放开后再评）。

### 工具面

**工具注册表模式**：一类一文件 + 一行注册（前端 barSpecs 为范式）。MCP 工具注册表与未来的插件 API 注册表共享设计资产，一次投入两处兑现。

一期只读白名单（薄壳转发既有 command）：`search_assets` / `list_folder_tree` / `list_placements` / `get_asset_detail` / `get_asset_origin_path`（源图与缩略图路径）/ `read_text_asset` / `list_archives` / `get_current_library`（2026-10-12 补第九个 `list_links`，见下「只读补批」）。**看图零集成**：返回磁盘绝对路径，Agent 有文件权限即直读 webp 缩略图与源图，不经协议传图。

纪律：禁止 assets/ 全目录枚举（以 folders.json 的 Placement 为入口按 ID 直达）；originName 随重命名同步改磁盘名，外部不得缓存路径、每次现推；有副作用的准只读命令（触发转码 / 后台生成队列类）不进只读白名单；库未打开时返回明确错误而非静默空表。

### 写入闸（二期）

**确认闸形态定案 A（2026-09-19 用户定；2026-09-20 开关位置修订）**：全局写入开关 + 撤销兜底——**插件面板 MCP 详情窗**「允许写入」开关（默认关，持久化于 config.json **settings.mcp 子树** `writeEnabled`，MCP 写调用逐次检查、关态回工具级错误），开态下写操作直接执行，用户靠撤销兜底。不做逐笔弹窗：全是可逆操作，批量场景（一次写几十条提示词）弹窗不可行。（初版落点 = 偏好设置弹窗 + settings.ui 子树 `mcpWriteEnabled`，2026-09-20 随 MCP 条目化进插件面板挪入详情窗，旧键由后端一次性迁移；偏好设置弹窗空壳退役。）

**服务总开关（2026-09-20 用户定，随 MCP 条目化落地）**：MCP server 初版随 APP 启动恒起 17701；条目化后详情窗加「MCP 服务」启停开关（settings.mcp 子树 `serverEnabled`，缺省开保持旧行为）——不用 Agent 的用户可整个关掉、17701 不白听。动机 = 入口统一 + 暴露面管理（server 内嵌进程、启动成本毫秒级，省的从来不是启动速度）。停止即删端口文件，Agent 发现机制同步下线。

**首批写工具（2026-09-19 定并落地，三个）**：`create_document`（在指定文件夹新建 md / txt 文本资产入库）、`write_text_asset`（写 / 改文本内容，metadata.size / modifiedAt 同步）、`rename_placements`（改名，批量数组、单条 = 1 项）。撤销口径：create_document / rename_placements 进撤销（单写点 + 单撤销步，复用既有 UndoManager）；**write_text_asset 不进撤销**——与 APP 内文本编辑同口径（2026-08-23 定：库操作撤销体系不覆盖内容编辑），此点写进工具描述让 Agent 知晓。

**二批写工具（工作流驱动，2026-09-19 用户定，五个）**：`create_folder`（新建文件夹）/ `move_placements`（资产移入文件夹——属 03 节 L3 结构性操作，仍走形态 A 全局开关 + 撤销兜底，可逆，不另立弹窗闸）/ `link_placements`（链接汇总——资产留大池、链接进场次文件夹的用户习惯配套）/ `move_folders`（移动文件夹重设父级——同日用户补：「万一改一下结构就改不动了」，仅改数据不动磁盘）/ `import_paths`（导入磁盘文件入库——Agent 外部生成图片后入库的入口，缩略图随导入管线自动生成、导入即可看图）。

**import_paths 冲突策略 = 方案一（2026-09-19 用户定，不阻断，已落地）**：hash 完全相同 → 跳过并把已有资产 id 回给 Agent（查重白送，Agent 直接拿去用）；同名不同内容 → 自动加 `_序号` 后缀入库（复用 core::library::unique_name 既有单写点口径）；回包逐文件报告处理结果（imported / skippedDuplicate + 已有 id / renamed + finalName / failed + reason）。自动化场景不中断优先，全程可追溯。

撤销口径：create_folder / move_placements / link_placements / move_folders 进撤销（同首批 _op 复用范式）；**import_paths 不进撤销**（落地时定：APP 导入管线本就不接 UndoManager——导入 = 资产池新增，撤销体系覆盖库结构操作不覆盖导入，MCP 同口径不硬接，工具描述如实告知）。

**提示词载体说明**：提示词专属功能（[[03_数据链接]] 的提示词链接）**已落地（2026-09-26 提示词轮）**——绑定 = 对 md/txt 文本资产的引用 + 内容快照（metadata.prompt / archives.json 同构字段），APP 侧三态菜单与详情提示词 tab 全接线；MCP 工具面同步加 `assign_prompt`（指定文本资产为提示词）/ `clear_prompt`（断开，快照随解绑清除）两写工具（走写入闸、进撤销，工具面 17 → 19）。Agent 写提示词链路 = `create_document` + `write_text_asset` 写文本资产 → `assign_prompt` 绑定到目标资产/档案。

**三批写工具（2026-09-20 用户定，一个）**：`trash_placements`（丢到库内回收站——Placement 从 folders.json 移入 recycle.json 按资产聚合，磁盘文件不动，进撤销同结构批 `_op` 范式，APP 内 Ctrl+Z 可撤、回收站可还原）。定案：**允许 AI 丢回收站，不许彻底删除**——彻底删除永不经 MCP（L4 永远人工确认：回收站清空 / 彻底删除 / 磁盘物理删除只在 APP 内人工）。

**四批写工具（2026-10-08 用户定，一个）**：`set_folder_groups`（设置文件夹分组定义——场次文件夹「提示词 / P1 / P2」规则组的 Agent 侧入口，此前只能 APP 内人工配）。语义同 APP 分组管理窗口：全量草稿式提交 = 该文件夹分组数组终态（空数组 = 取消分组；漏交 = 删除该组），新组 id 缺省后端生成（`G` 系，对齐 F / B id 规则），改既有组照 `list_folder_tree` 回包 id 抄；进撤销同 `_op` 范式。2026-09-19「不需要 assign_group 工具」结论维持——自由组成员归组不设独立工具，需要时经本工具 members 全量管理。

**五批写工具（2026-10-11 用户定，一个）**：`rename_folders`（文件夹改名 / 批量改名——APP 侧文件夹批量重命名（[[14_批量编辑]]）同日落地后补齐 Agent 侧入口；单条改名 = 1 项数组，与 `rename_placements` 同形）。语义同 APP：仅改数据不动磁盘（Folder 是数据层分组无磁盘实体）、同名不拦截（文件夹名允许重复）、失败项（非法名 / 陈旧 id）跳过进 failed 汇总不整体回滚；进撤销同 `_op` 范式（`commands::operations::rename_folders_op` 整段复用，APP 命令同一实现）。工具面 20 → 21（只读 8 + 写 13）。

二期余量（随后批）：`set_asset_props`（打属性词）/ `set_asset_descriptions`（写描述）。AI 写入内容统一打标记、可一键清除（对齐 03 节 L2）随批量业务再评。

**只读补批 `list_links`（2026-10-12 用户定，第九个只读工具）**：全库链接 Placement 清单——「资产留大池、链接进场次」组织习惯的可见性配套：AI 建链接（`link_placements`）后需要能回查哪些文件是链接、链在哪个文件夹、原件在哪。无参全库列举（链接体量小，不破省 token 口径；替代「全量 `list_placements` 自行过滤 origin」的重路径），逐条回 `assetId / ext / link / origin / uri`——link 与 origin 均复用详情链接区块 `PlacementLink` 结构（folderId 为 null = 未分类 root / blockName / path 路径链 / name），origin = null 表示原始 Placement 已不在库（进回收站或彻底删除）；uri = http open 点击跳转形态（2026-10-08 S5 同口径）。工具面 21 → 22（只读 9 + 写 13）。

### 语义检索与任务工具（2026-10-25）

语义分析接入同一命令服务层，不另造检索或任务管线。只读工具为 `semantic_search`、`get_semantic_analysis_task`、`list_semantic_analysis_tasks`。语义检索支持中文查询、文件夹范围与递归选项，返回素材的最佳片段、其他命中段和真实时间戳 / 帧索引；只查询已有派生索引，不因查询自动分析素材。首次查询可能加载本地文本模型，但不调用外部 provider。

任务工具为 `start_semantic_analysis`、`start_detailed_semantic_analysis`、`pause_semantic_analysis`、`resume_semantic_analysis`、`cancel_semantic_analysis`、`retry_semantic_analysis`。它们属于受控写操作，逐次经过现有 MCP 写入闸，不进素材结构撤销栈；索引进度经任务检查点恢复。启动分析必须提供非空 `assetIds`，**不向 Agent 开放缺省全库或目录批扫**。详细分析只访问用户已配置的 provider，不在 MCP 返回或配置工具中暴露 API key。暂停、恢复、取消和重试仅作用于当前素材库的任务，恢复与重试保持原任务的基础 / 详细模式。

命中片段新增 `create_semantic_segment`（2026-10-27）：输入 `assetId + segmentId + targetFolderId? + physical?`，虚拟片段引用源范围，物理截取为独立普通资产；必须同模型 / 分析版本和源指纹，禁止嵌套。创建进入普通撤销，成功后广播库变更。工具面为 **32 个 = 只读 12 + 写 20**，其中创建片段等 12 个写工具进撤销，文本覆写 / 导入及六个语义任务工具共 8 个不进撤销。

### 档案管理工具（2026-10-31）

档案管理接入同一命令服务层。新增只读 `get_archive_data`，返回与 APP 档案详情一致的属性、描述、链接、封面、提示词及统计；新增写工具 `create_archive`、`convert_to_archive`、`convert_to_folder`、`set_archive_props`、`set_archive_descriptions`、`set_archive_links`、`delete_archive`。全部写工具逐次经过 MCP 写入闸并进入普通撤销，成功后广播库变更。

`convert_to_archive` 保留 APP 的两段式语义：有子文件夹且未确认时返回 `need_confirm`，确认后才平移子树 Placement 并删除子文件夹节点。`delete_archive` 仅接受档案，删除档案节点并把其中 Placement 移到未分类，不删除素材实体；同时清除对应 `archives.json` 记录。物理删除不进入 MCP。档案改名与移动继续复用 `rename_folders` / `move_folders`，提示词继续复用 `assign_prompt` / `clear_prompt`。

本批共 8 个工具，工具面调整为 **40 个 = 只读 13 + 写 27**；其中 20 个写工具进撤销，文本覆写、导入及六个语义任务工具共 8 个不进撤销。

### CLI 外接入口（1.0 收尾）

面向具备终端能力的 Agent 与脚本自动化，增加 CLI 调用入口。**CLI 与 MCP 并存，不替换 MCP**。CLI 以瘦客户端连接运行中的 AniKit，复用同一业务操作、写入闸、撤销、回收站红线与热刷新，不直接读写库数据文件、不另造业务管线。输出采用适合 Agent 与脚本解析的紧凑 JSON；先落地高频只读命令，再按真实工作流补齐写命令。

### 阶段表

① 本 PRD 立项（2026-09-19）→ ② **只读 MCP**（2026-09-19 落地，Codex 真库验证跑通）→ ③ **写入闸首批**（2026-09-19 落地，形态 A + 三写工具 + 偏好设置弹窗）→ ④ **二批结构性五件套**（create_folder / move_placements / link_placements / move_folders / import_paths 方案一冲突策略，2026-09-19 定案并落地）→ ⑤ **三批 trash_placements**（库内回收站可逆删除：允许 AI 丢回收站、彻底删除永不经 MCP，2026-09-20 定案并落地）→ ⑥ **MCP 条目化进插件面板**（2026-09-20 落地：桥接类「AI 外接通道（MCP）」条目 + 详情窗四组——服务总开关 / 允许写入挪入 / 使用教程含 Agent 接入配置复制 / 功能列表 / 路径；偏好设置弹窗退役）→ ⑦ **四批 set_folder_groups**（分组定义写工具：场次规则组的 Agent 侧入口，全量草稿式提交 + 新组 id 后端生成，2026-10-08 定案）→ ⑧ **五批 rename_folders**（文件夹改名 / 批量改名：APP 文件夹批量重命名的 Agent 侧入口，失败跳过汇总、进撤销，2026-10-11 定案并落地）→ ⑨ **只读补批 list_links**（全库链接 Placement 清单：链接位置 + 原始位置（PlacementLink 结构）+ uri，「链接进场次」习惯的可见性配套，2026-10-12 定案并落地）→ ⑩ **档案管理批**（档案详情 + 新建 / 转换 / 数据编辑 / 可撤销删除，2026-10-31 定案并落地）→ ⑪ **CLI 外接入口**（1.0 收尾新增，见上节）→ ⑫ 余量批（属性词 / 描述 / 打标记；一键写 Agent 配置、已配置工具检测随该批再评）。可选起步垫（库索引导出小命令）未做、随需要再评。接口文档 `docs/05_接口文档/MCP通道.md` + `MCP使用指南.md` 随开发同步。
