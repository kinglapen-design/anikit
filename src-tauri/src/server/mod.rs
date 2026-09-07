//! server/ —— 动画软件通信通道（PRD 07-01「统一通信通道」）：
//! AniKit 内置本地 WS 服务，各软件插件（适配器）连接后双向通信——AniKit 下发指令
//! （打开文件 / 导出 / 发送至舞台），插件回传进度与结果。8.1 首客户端 = Animate CEP 插件。

mod plugin_ws;

pub use plugin_ws::{broadcast_settings, request_paste, try_acquire_send};
#[cfg(not(test))] // 测试构建不起 server（踩坑 014）
pub use plugin_ws::init;
