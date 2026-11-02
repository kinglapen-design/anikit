# CLI 外接入口（anikit-cli）

> 2026-11-02 定案并落地（1.0 收尾第 8 项，PRD `08_AI 功能/06_AI外接通道MCP.md`「CLI 外接入口」）。实现 = `src-tauri/src/bin/anikit-cli.rs`（src/bin 第三 bin）；工具参数与回包结构以 [MCP通道](MCP通道.md) 与各命令文档为准，本文只记 CLI 层协议与命令映射。

## 定位与红线

`anikit-cli.exe` 是面向终端 Agent 与脚本自动化的**瘦客户端**，与 MCP 并存、不替换 MCP。它连接**运行中的 AniKit**（MCP 服务），复用同一业务操作、写入闸、撤销、回收站红线与热刷新广播；不直接读写库数据文件、不另造业务管线。后端零改动、零新端点——AniKit 侧没有任何 CLI 专属业务代码，CLI 只做参数映射与 MCP 协议解析。

首版只提供**只读命令**（只读 13 工具全量 + tools 列表）。写命令子命令后续按真实工作流补批，补批同样只映射 MCP 既有写工具（写入闸默认关，需在插件面板 MCP 详情窗开启）。

## 连接与端口发现

- 通信 = MCP JSON-RPC over Streamable HTTP：POST `http://127.0.0.1:{port}/mcp`，每次运行一次握手（`initialize` → `notifications/initialized` 通知 → `tools/call` / `tools/list`）。SSE（text/event-stream）与 application/json 两种响应形态都接。
- 端口：默认读全局配置目录 `AniKit/mcp-server.port`（与 anikit-paste 同一发现机制；停服即删该文件），缺文件 / 解析失败回退固定端口 **17701**；`--port N` 覆盖。
- AniKit 未启动、MCP 服务总开关关、端口被占回退动态但未读端口文件等情况 = 连接失败，退出码 2 并提示检查服务开关。
- 连接超时 3 秒、请求超时 60 秒。

## 命令表

```
anikit-cli <子命令> [参数] [--pretty] [--port N]
```

| 子命令 | 映射 MCP 工具 | 参数 |
| --- | --- | --- |
| `status` | get_current_library | — |
| `search <关键词>` | search_assets | `--folder ID` 限定文件夹；`--no-recursive` 不含子文件夹 |
| `tree` | list_folder_tree | — |
| `placements` | list_placements | `--folder ID`（缺省全库，量大慎用） |
| `detail <assetId>` | get_asset_detail | — |
| `path <assetId>` | get_asset_origin_path | 源文件 + 缩略图绝对路径 |
| `read <assetId>` | read_text_asset | txt / md 文本资产 |
| `archives` | list_archives | — |
| `archive <folderId>` | get_archive_data | — |
| `links` | list_links | 全库链接 Placement 清单 |
| `semantic <查询>` | semantic_search | `--folder ID` / `--limit N` / `--no-recursive`；只查既有索引，不触发分析 |
| `task <taskId>` | get_semantic_analysis_task | — |
| `tasks` | list_semantic_analysis_tasks | — |
| `tools` | tools/list | 瘦身输出 = 名称 + 描述 + readOnly 标记（不含 inputSchema） |
| `-h` / `--help` / `help` | — | 用法文本 |

全局选项：`--pretty`（缩进格式化输出，默认紧凑 JSON）、`--port N`。选项可出现在任意位置；`--port=17702` / `--folder=ID` / `--limit=5` 等号形态同吃。

id 类参数（assetId / folderId）与 MCP 同口径：可直接粘贴 `anikit://item|folder/{id}` 或 `http://127.0.0.1:{port}/(open/)item|folder/{id}` 链接整段，服务端自动归一化。

## 输出与退出码

- 成功：stdout = 工具回包的紧凑 JSON（与 MCP content 文本同源）；`--pretty` 重排缩进，解析失败保原文不丢数据。
- **退出码 0** = 成功；**1** = 工具级错误（isError，如「请先打开素材库」「资源不存在」「MCP 写入未开启」），stdout 输出 `{"error":"..."}`；**2** = 连接 / 协议 / 用法错误，stderr 中文提示。
- 脚本用法示例：`anikit-cli search 杰森 --folder F2610080228-XJZ | jq '.placements[].assetId'`。

## 分发

随免安装包带件：`tools/pack-portable.py` 组装 `anikit-cli.exe` 到包根（与 anikit.exe / anikit-paste.exe 并列）。构建 = `cargo build --release --bins`（tauri build --no-bundle 全 bins 同出）。开发自测 = `cargo run --bin anikit-cli -- status`；单元测试 = `cargo test --bin anikit-cli`（参数映射 / SSE 切分 / 回包提取与瘦身）。
