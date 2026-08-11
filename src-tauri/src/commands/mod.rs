//! 命令 API 层（Tauri commands，01_开发规范 第 1 条业务层）：
//! UI / AI Agent / 插件统一经命令接口操作，不直接读写数据文件。

pub mod asset_menu;
pub mod doc_apps;
pub mod import;
pub mod inbox;
pub mod library;
pub mod media;
pub mod notes;
pub mod operations;
pub mod preview;
pub mod recent;
pub mod recycle;
pub mod settings;
pub mod thumb_frame;
pub mod undo;
