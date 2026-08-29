//! 适配器层（01_开发规范 第 1 条）：系统能力封装，业务语义在 services。
//!
//! - inbox_drive：收件箱盘符挂载 / 卸载（Windows DefineDosDeviceW，同 subst 机制）
//! - inbox_watch：收件箱目录文件监听（notify 增量事件 + 写入完成检测）
//! - shell_open：系统默认程序打开文件（Windows ShellExecuteW "open"，预览窗口用）
//! - shell_thumb：Shell 格式图标（IShellItemImageFactory，4.2 保底，产物仅存 cache/thumbs 不进 info）
//! - ffmpeg：ffmpeg / ffprobe sidecar（5.1 音视频元数据 / 抽帧 / PCM，二进制缺失返回 None 保底）
//! - decoder：ffmpeg-next 库级解码（阶段四自研解码器：GIF/视频逐帧 RGBA 帧流；webp 归 services/anim）
//! - window_fx：窗口背景效果（7.1 十七轮：Win11 22H2+ 云母 Mica，其余实色回退）

pub mod decoder;
pub mod ffmpeg;
pub mod inbox_drive;
pub mod inbox_watch;
pub mod shell_open;
pub mod shell_thumb;
pub mod window_fx;
