# AGENTS.md

## 项目简介

AniKit —— 面向动画师的素材管理软件：Eagle 类封闭式资源库 + 索引式工作空间 + 动画软件（Animate 等）双向通信。技术栈：Tauri + Rust 后端 + TypeScript（严格模式）前端。目标平台：Windows 优先。

## 动手前必读（按顺序）

**新会话入口：** 先读本文件 → `docs/05_项目进度/00_当前状态.md` → 从该处继续。

1. PRD 索引：`docs/03_项目-PRD/00_功能总览.md`（PRD 共 9 份，同目录 01-08）
2. 开发规范：`docs/01_开发规范.md` —— **必须遵守**
3. **UI 任务必读：`docs/02_样式与布局/03_新面板口径速查.md`** —— 任何新面板/弹窗/卡片/列表/菜单动手前通读（弹窗/滚轮/列表 item/重命名/透明度的已定口径与错误信号，2026-08-13 用户要求设立：防止跨会话重复犯同类样式错误；新定案即时回补）
   - **阶段七 UI 重构（2026-08-28 起，样式定稿）**：统一工作规范见技能 `ui-refactor`（`.kimi-code/skills/ui-refactor/SKILL.md`），设计稿入口 `docs/UI 重构/`；UI 任务自动触发，亦可 `/skill:ui-refactor` 手动调用
4. 进度与任务：`docs/05_项目进度/`（按阶段拆分，索引 `docs/05_项目进度.md`）
5. 接口文档：`docs/07_接口文档.md`（随开发更新）

其他：`docs/04_开发日志/`（按日期一天一文件，做了什么记什么；索引 `docs/04_开发日志.md`）、`docs/06_踩坑记录.md`、`docs/02_样式与布局.md`（设计 token，用户维护）。

## 链接约定

PRD 内使用 Obsidian 双链 `[[文件名]]`，按文件名在 `docs/03_项目-PRD/` 内解析（如 `[[01_资源与数据]]` → `docs/03_项目-PRD/01_资源与数据.md`）。

## 目录结构

``` text
├── docs/                 全部文档（01_开发规范、02_样式与布局、03_项目-PRD/、04-07 开发文档）
├── src/                  前端（TS 严格模式 + Vue 3 + pinia）
├── src-tauri/            Rust 后端
│   └── src/
│       ├── commands/     命令 API 层（Tauri commands，规范第 1 条业务层）
│       ├── core/         数据层（JSON 读写、迁移、备份；每个数据文件单一写点）
│       ├── services/     业务服务（导入、缩略图、搜索、帧流 WS（frame_stream，阶段四）……）
│       └── adapters/     适配器（剪贴板、Shell、全局热键、文件监听、ffmpeg 解码……）
│       （动画软件通信通道的 WS 服务随阶段七插件落地时再建，现无 server/ 目录）
├── plugins/              动画软件插件（CEP 等，如 plugins/animate-cep/）
├── spike/                阶段 0 技术验证代码（验证完归档，不进主线）
├── tools/                开发辅助工具（假数据生成、打包脚本等）
└── assets/               项目资源（图标等）
```

## 硬性边界

- `docs/00_修改与需求/` 由用户维护，**只读，禁止修改**。
- 改需求先改 PRD，再写代码；代码改动后同步更新 04 / 05 / 07 等相关文档。
- UI 文件禁止业务逻辑；一切功能抽象为命令接口（规范第 1 条）。
- 每个数据文件单一写点（规范第 1 条）。
- Git 写操作：完成任务后、大任务开始前可自行提交（2026-07-18 用户授权）；push / reset / rebase / clean 等破坏性操作仍须当场确认。
- UI 开发遇到没有 token 或设计参照的部分，必须先询问用户，禁止凭感觉自创样式（2026-07-18 用户要求）。
- 改样式时同步更新 `docs/02_样式与布局/` 对应文档；发现文档与项目实际不匹配的部分顺手修正（2026-07-21 用户要求）。
- 向用户提问一次只问一个：多问会导致只答一个就结束（2026-07-20 用户要求）。

## 常用命令

- `pnpm dev` / `pnpm tauri dev` —— 启动开发
- `cargo test` —— 后端单测
- `pnpm typecheck` / `pnpm build` —— 前端检查（vue-tsc，覆盖 .vue）/ 构建
- `python tools/fetch-ffmpeg.py` —— 下载 ffmpeg/ffprobe sidecar（首次开发 / 换机必跑；音视频解码依赖，二进制不入 git）
- `python tools/fetch-ffmpeg.py --dev-lib` —— 下载 ffmpeg dev 库到 `src-tauri/vendor/ffmpeg`（阶段四自研解码器编译前置；另需 libclang：`tools/.venv/Scripts/python.exe -m pip install --target tools/libclang libclang`，ffmpeg-sys-next 的 bindgen 依赖）

## 术语

Asset（资源实体）/ Placement（放置位置）/ originName（磁盘文件名）/ info 文件夹 / 素材库（封闭式）/ 工作空间（索引式）—— 以 PRD `01_资源与数据.md` 为准，不造同义词。
