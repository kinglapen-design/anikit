# 2026-10-12_01 MCP 链接清单工具（list_links）

## 背景与定案

用户定：MCP 工具里加入所有链接文件的列表，AI 需要查看——「资产留大池、链接进场次」组织习惯的可见性配套：AI 经 `link_placements` 建链接后，此前没有任何工具能回查哪些文件是链接、链在哪、原件在哪（只能全量 `list_placements` 自行过滤 `origin=false`，破省 token 口径）。PRD 08-06 立为**只读补批（第九个只读工具）**，工具面 21 → 22（只读 9 + 写 13）。

## 实现

新工具 `list_links`（`tools/links.rs` 一类一文件 + 注册处一行，工具注册表模式）：无参全库列举链接 Placement（`folders.items` 里 `origin = false` 者；脏数据跳过同 placement_views 口径），逐条回 `LinkView { assetId / ext / link / origin / uri }`——**link 与 origin 均复用详情链接区块 `PlacementLink` 结构**（folderId 为 null = 未分类 root / blockName / path 路径链最多三段 / name），`commands::archive::placement_link` 提 pub(crate) 同一份实现（与 APP 详情面板「链接」区块同口径）；origin 经 `index.placements_of` 反查原始 Placement，无存活原始（进回收站 / 彻底删除）= null；uri = http open 点击跳转形态（2026-10-08 S5 同口径）。

## 牵连面同步

- `server/mcp/mod.rs` instructions 串：计数「一期只读 8 + 写工具 12」→「只读 9 + 写工具 13」、典型流程加 list_links、**顺手销 10-11 待办**（rename_folders 入枚举、「其余十个进撤销」→ 十一个）；顶部批注补五批 / 只读补批两段。
- 插件面板 MCP 详情窗功能列表：只读 8 → 9 + 名单加「链接清单」。
- PRD 08-06：一期白名单行 + 只读补批定案段 + 阶段表 ⑨（余量批顺移 ⑩）。
- 接口 MCP通道 / 使用指南：计数 21 → 22（只读 9）、工具表加 list_links 行、URI 增强段补 list_links 逐项 uri、典型流程两处补 list_links。

## 验证

cargo check / clippy 净 + cargo test 全绿 + typecheck 0 错。**过程插曲**：从仓库根 `cargo check --manifest-path` 撞 ffmpeg-sys-next pkg-config 报错——`src-tauri/.cargo/config.toml` 的 `[env]`（FFMPEG_DIR / LIBCLANG_PATH）按 cwd 向上查找，根目录跑吃不到；须 `cd src-tauri` 内跑（历史会话一贯如此）。

**实机走查（2026-10-12_02 免安装包重打轮已销）**：真机握手 tools/list 22 在列 + 实调 list_links 真库回 274 条零错误零 null origin。
