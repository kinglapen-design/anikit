//! 业务服务层（01_开发规范 第 1 条）：核心业务逻辑，UI 与命令层不直接实现功能。
//!
//! - import：导入管线（文件 / 文件夹 / 剪贴板 → 封闭式素材库）
//! - thumbnail：缩略图与图片宽高探测（缓存，可重建）

pub mod import;
pub mod thumbnail;
