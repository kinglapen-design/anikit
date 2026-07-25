//! 业务服务层（01_开发规范 第 1 条）：核心业务逻辑，UI 与命令层不直接实现功能。
//!
//! - import：导入管线原语与公开类型（扫描 / 哈希 / 建档 / 登记）
//! - import_task：导入任务编排（分段短锁、事件推送、取消、落盘节流）
//! - inbox：收件箱（盘符挂载 + 文件监听 + 就绪批次串行复用导入管线）
//! - resolve：hash 冲突决策（PRD 10_重复添加提示，导入挂起 → 用户逐项决策）
//! - sequence：序列帧检测与打包入库（PRD 01-2.5，4.1）
//! - psb：PSB 合成图解码（psd crate 不支持 PSB，自研轻量解析只取内嵌合成图，4.2 补）
//! - preview_queue：预览解析队列（4.3 预览窗口：单 worker + 代际作废，切换素材不堆积）
//! - thumb_formats：PSD / PSB / SVG 解码补充（psd / resvg crate + psb.rs，无 Adobe 依赖，4.2）
//! - thumbnail：缩略图与图片宽高探测（缓存，可重建）
//! - views：Placement 列表视图（中间面板与导入事件共用）

pub mod import;
pub mod import_task;
pub mod inbox;
pub mod preview_queue;
pub mod psb;
pub mod resolve;
pub mod sequence;
pub mod thumb_formats;
pub mod thumbnail;
pub mod views;
