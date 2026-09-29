//! 业务服务层（01_开发规范 第 1 条）：核心业务逻辑，UI 与命令层不直接实现功能。
//!
//! - import：导入管线原语与公开类型（扫描 / 哈希 / 建档 / 登记）
//! - import_task：导入任务编排（分段短锁、事件推送、取消、落盘节流）
//! - inbox：收件箱（盘符挂载 + 文件监听 + 就绪批次串行复用导入管线）
//! - resolve：hash 冲突决策（PRD 10_重复添加提示，导入挂起 → 用户逐项决策）
//! - sequence：序列帧检测与打包入库（PRD 01-2.5，4.1）
//! - psb：PSB 合成图解码（psd crate 不支持 PSB，自研轻量解析只取内嵌合成图，4.2 补）
//! - preview_queue：预览解析队列（4.3 预览窗口：单 worker + 代际作废，切换素材不堆积）
//! - thumb_formats：图片格式解码注册表（新格式 = 目录内新增一个适配器文件 + ADAPTERS 一行；4.2）
//! - thumbnail：缩略图与图片宽高探测（缓存，可重建）
//! - anim：动图（GIF / WEBP 动图）探测与逐帧解码（5.1，纯 Rust；probe 为字节级扫描 2026-07-26）
//! - anim_preview：动图小动画播放缓存（2.3；2026-09-02 恢复：info/{stem}_anim.webp 短边 320，卡片播放源）
//! - anim_preview_queue：小动画生成队列（单 worker 无代际作废，导入挂钩 + 懒生成；事件 preview-anim-ready）
//! - frame_stream：动图帧流（阶段四·自研解码器：解码线程会话 + WS 推帧，预览窗口播放源；卡片层 2026-09-02 回滚原生 img）
//! - media：音视频元数据与预览图（5.1，ffprobe/ffmpeg sidecar；视频首帧 / 音频波形）
//! - operations：素材基础操作（6.1 选择态：新建文件夹 / 移动 / 排序 / 关联 / 副本 / 重命名 / 删除）
//! - archive：档案文件夹（3.2 一轮骨架：新建档案 / 文件夹 ⇄ 档案转换 / 档案数据清单）
//! - recent：最近使用（6.1 第二轮，recent.json LRU 500；唯一启用入口 = 默认程序打开成功）
//! - recycle：回收站（6.1 第二轮：视图 / 还原 / 彻底删除 / 清空）
//! - scan：重复文件扫描（只读，按存量 hash 分组）
//! - search：分词搜索（只读，3.6 一轮：空格分词 AND / 子串包含 / 链接 URL decode / 路径链）
//! - custom_data：自定义数据（6.1：文件与档案的属性 / 描述编辑、属性池聚合与全局删除）
//! - relations：关系（6.2 选择模式一轮：文件与档案间的关系建立 / 编辑 / 删除，空间级 relations.json）
//! - color：颜色提取（6.1 颜色解析：缩略图主色存 metadata.colors，详情颜色 bar 数据源；存量懒回填）
//! - font：字体元数据与样张渲染（5.6，ttf-parser / fontdue 纯 Rust）+ 系统级安装卸载（用户级注册表）
//! - text：文本资产读写（5.6，txt/md；读 UTF-8 优先 GBK 回退，写恒 UTF-8，2MB 上限）
//! - prompt_scan：内嵌提示词扫描（2026-09-26 提示词轮，PRD 03_数据链接 02_文件「分析提示词」：
//!   PNG tEXt/iTXt/zTXt + JPEG/WEBP EXIF + ComfyUI 节点图抽取，本地解析无 AI）
//! - prompt：提示词业务（2026-09-26 提示词轮：指定 / 清除 / 同步快照 / 分析 / 新建提示词文档）
//! - docx：docx 预览文本提取（PRD 04-04 条 6，R6 首例：zip + quick-xml 段落结构，非版式渲染只读）
//! - folder_tree：文件夹树视图组装（命令层 / plugin_ws / MCP 三处共用，2026-09-25 自 commands 下迁）
//! - views：Placement 列表视图（中间面板与导入事件共用）
//! - workspace：索引式工作空间只读底座（PRD 02 步骤 1：目录列举 + .anikit 缩略图缓存 + 监听热刷新）
//! - ws_thumb_queue：工作空间缩略图生成队列（2026-09-26 性能优化：常驻 worker 池固定并发 +
//!   跨请求去重 + 失败记忆 + 视口优先插队，取代每 invoke 裸 spawn 串行线程）

pub mod anim;
pub mod anim_preview;
pub mod anim_preview_queue;
pub mod archive;
pub mod asset_menu;
pub mod cep_export;
pub mod cep_install;
pub mod color;
pub mod custom_data;
pub mod docx;
pub mod folder_tree;
pub mod font;
pub mod frame_stream;
pub mod import;
pub mod import_task;
pub mod inbox;
pub mod media;
pub mod operations;
pub mod preview_queue;
pub mod prompt;
pub mod prompt_scan;
pub mod psb;
pub mod purge;
pub mod recent;
pub mod recycle;
pub mod relations;
pub mod resolve;
pub mod scan;
pub mod search;
pub mod send_stage;
pub mod sequence;
pub mod text;
pub mod thumb_formats;
pub mod thumbnail;
pub mod views;
pub mod workspace;
pub mod ws_thumb_queue;
