//! 适配器层（01_开发规范 第 1 条）：系统能力封装，业务语义在 services。
//!
//! - inbox_drive：收件箱盘符挂载 / 卸载（Windows DefineDosDeviceW，同 subst 机制）
//! - inbox_watch：收件箱目录文件监听（notify 增量事件 + 写入完成检测）

pub mod inbox_drive;
pub mod inbox_watch;
