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

**工具注册表模式**：一类一文件 + 一行注册（前端 barSpecs 为范式）。MCP 工具注册表与未来的插件 API 注册表是同一份资产，一次投入两处兑现；8.2 批量生成缩略图共享同一套通道基础。

一期只读白名单（薄壳转发既有 command）：`search_assets` / `list_folder_tree` / `list_placements` / `get_asset_detail` / `get_asset_origin_path`（源图与缩略图路径）/ `read_text_asset` / `list_archives` / `get_current_library`。**看图零集成**：返回磁盘绝对路径，Agent 有文件权限即直读 webp 缩略图与源图，不经协议传图。

纪律：禁止 assets/ 全目录枚举（以 folders.json 的 Placement 为入口按 ID 直达）；originName 随重命名同步改磁盘名，外部不得缓存路径、每次现推；有副作用的准只读命令（触发转码 / 后台生成队列类）不进只读白名单；库未打开时返回明确错误而非静默空表。

### 写入闸（二期）

**确认闸形态定案 A（2026-09-19 用户定；2026-09-20 开关位置修订）**：全局写入开关 + 撤销兜底——**插件面板 MCP 详情窗**「允许写入」开关（默认关，持久化于 config.json **settings.mcp 子树** `writeEnabled`，MCP 写调用逐次检查、关态回工具级错误），开态下写操作直接执行，用户靠撤销兜底。不做逐笔弹窗：全是可逆操作，批量场景（一次写几十条提示词）弹窗不可行。（初版落点 = 偏好设置弹窗 + settings.ui 子树 `mcpWriteEnabled`，2026-09-20 随 MCP 条目化进插件面板挪入详情窗，旧键由后端一次性迁移；偏好设置弹窗空壳退役。）

**服务总开关（2026-09-20 用户定，随 MCP 条目化落地）**：MCP server 初版随 APP 启动恒起 17701；条目化后详情窗加「MCP 服务」启停开关（settings.mcp 子树 `serverEnabled`，缺省开保持旧行为）——不用 Agent 的用户可整个关掉、17701 不白听。动机 = 入口统一 + 暴露面管理（server 内嵌进程、启动成本毫秒级，省的从来不是启动速度）。停止即删端口文件，Agent 发现机制同步下线。

**首批写工具（2026-09-19 定并落地，三个）**：`create_document`（在指定文件夹新建 md / txt 文本资产入库）、`write_text_asset`（写 / 改文本内容，metadata.size / modifiedAt 同步）、`rename_placements`（改名，批量数组、单条 = 1 项）。撤销口径：create_document / rename_placements 进撤销（单写点 + 单撤销步，复用既有 UndoManager）；**write_text_asset 不进撤销**——与 APP 内文本编辑同口径（2026-08-23 定：库操作撤销体系不覆盖内容编辑），此点写进工具描述让 Agent 知晓。

**二批写工具（工作流驱动，2026-09-19 用户定，五个）**：`create_folder`（新建文件夹）/ `move_placements`（资产移入文件夹——属 03 节 L3 结构性操作，仍走形态 A 全局开关 + 撤销兜底，可逆，不另立弹窗闸）/ `link_placements`（链接汇总——资产留大池、链接进场次文件夹的用户习惯配套）/ `move_folders`（移动文件夹重设父级——同日用户补：「万一改一下结构就改不动了」，仅改数据不动磁盘）/ `import_paths`（导入磁盘文件入库——Agent 外部生成图片后入库的入口，缩略图随导入管线自动生成、导入即可看图）。

**import_paths 冲突策略 = 方案一（2026-09-19 用户定，不阻断，已落地）**：hash 完全相同 → 跳过并把已有资产 id 回给 Agent（查重白送，Agent 直接拿去用）；同名不同内容 → 自动加 `-序号` 后缀入库（复用 core::library::unique_name 既有单写点口径）；回包逐文件报告处理结果（imported / skippedDuplicate + 已有 id / renamed + finalName / failed + reason）。自动化场景不中断优先，全程可追溯。

撤销口径：create_folder / move_placements / link_placements / move_folders 进撤销（同首批 _op 复用范式）；**import_paths 不进撤销**（落地时定：APP 导入管线本就不接 UndoManager——导入 = 资产池新增，撤销体系覆盖库结构操作不覆盖导入，MCP 同口径不硬接，工具描述如实告知）。

**提示词载体说明**：提示词专属功能（[[03_数据链接]] 的提示词链接）尚未落地，当前载体 = md / txt 文本资产——写工具写的就是文本资产形态的提示词；专属功能落地后工具面再加，不冲突。

**三批写工具（2026-09-20 用户定，一个）**：`trash_placements`（丢到库内回收站——Placement 从 folders.json 移入 recycle.json 按资产聚合，磁盘文件不动，进撤销同结构批 `_op` 范式，APP 内 Ctrl+Z 可撤、回收站可还原）。定案：**允许 AI 丢回收站，不许彻底删除**——彻底删除永不经 MCP（L4 永远人工确认：回收站清空 / 彻底删除 / 磁盘物理删除只在 APP 内人工）。

二期余量（随后批）：`set_asset_props`（打属性词）/ `set_asset_descriptions`（写描述）。AI 写入内容统一打标记、可一键清除（对齐 03 节 L2）随批量业务再评。

### 阶段表

① 本 PRD 立项（2026-09-19）→ ② **只读 MCP**（2026-09-19 落地，Codex 真库验证跑通）→ ③ **写入闸首批**（2026-09-19 落地，形态 A + 三写工具 + 偏好设置弹窗）→ ④ **二批结构性五件套**（create_folder / move_placements / link_placements / move_folders / import_paths 方案一冲突策略，2026-09-19 定案并落地）→ ⑤ **三批 trash_placements**（库内回收站可逆删除：允许 AI 丢回收站、彻底删除永不经 MCP，2026-09-20 定案并落地）→ ⑥ **MCP 条目化进插件面板**（2026-09-20 落地：桥接类「AI 外接通道（MCP）」条目 + 详情窗四组——服务总开关 / 允许写入挪入 / 使用教程含 Agent 接入配置复制 / 功能列表 / 路径；偏好设置弹窗退役）→ ⑦ 余量批（属性词 / 描述 / 打标记；一键写 Agent 配置、已配置工具检测随该批再评）。可选起步垫（库索引导出小命令）未做、随需要再评。接口文档 `docs/05_接口文档/MCP通道.md` + `MCP使用指南.md` 随开发同步。
