//! 数据层（01_开发规范 第 1 条：数据层 → 业务层 → UI 层）
//!
//! - models：数据模型，与 PRD 01 / 06 的 JSON 结构一一对应
//! - json_io：JSON 统一读写（原子写、version 门禁）
//! - paths：assets 池路径推导单点（ID → info 路径）
//! - space：储存空间创建 / 校验 / 打开（PRD 01_储存空间）
//! - library：素材库创建 / 校验 / 打开
//! - index：内存索引（Asset ↔ Placement 反查）
//! - backup：滚动备份（10 份）
//! - migrate：版本迁移骨架
//! - config：全局配置（%APPDATA%\AniKit\config.json）

pub mod backup;
pub mod config;
pub mod index;
pub mod json_io;
pub mod library;
pub mod migrate;
pub mod models;
pub mod notes;
pub mod paths;
pub mod relations;
pub mod space;
pub mod undo;
