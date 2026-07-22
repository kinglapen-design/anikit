//! 业务服务层（01_开发规范 第 1 条）：核心业务逻辑，UI 与命令层不直接实现功能。
//!
//! - import：导入管线原语与公开类型（扫描 / 哈希 / 建档 / 登记）
//! - import_task：导入任务编排（分段短锁、事件推送、取消、落盘节流）
//! - inbox：收件箱（盘符挂载 + 文件监听 + 就绪批次串行复用导入管线）
//! - resolve：hash 冲突决策（PRD 10_重复添加提示，导入挂起 → 用户逐项决策）
//! - thumbnail：缩略图与图片宽高探测（缓存，可重建）
//! - views：Placement 列表视图（中间面板与导入事件共用）

pub mod import;
pub mod import_task;
pub mod inbox;
pub mod resolve;
pub mod thumbnail;
pub mod views;
