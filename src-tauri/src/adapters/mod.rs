//! 适配器层（01_开发规范 第 1 条）：系统能力封装，业务语义在 services。
//!
//! - inbox_drive：收件箱盘符挂载 / 卸载（Windows DefineDosDeviceW，同 subst 机制）
//! - inbox_watch：收件箱目录文件监听（notify 增量事件 + 写入完成检测）
//! - shell_open：系统默认程序打开文件（Windows ShellExecuteW "open"，预览窗口用）
//! - shell_path：Shell API 路径规范化（剥 canonicalize 的 \\?\ 逐字前缀，踩坑 078）
//! - shell_thumb：Shell 格式图标（IShellItemImageFactory，4.2 保底，产物仅存 cache/thumbs 不进 info）
//! - recycle_bin：系统回收站删除（SHFileOperationW FOF_ALLOWUNDO，工作空间步骤 2，PRD 06 不进库内回收站）
//! - ffmpeg：ffmpeg / ffprobe sidecar（5.1 音视频元数据 / 抽帧 / PCM，二进制缺失返回 None 保底）
//! - decoder：ffmpeg-next 库级解码（阶段四自研解码器：GIF/视频逐帧 RGBA 帧流；webp 归 services/anim）
//! - window_fx：窗口背景效果（7.1 十七轮：Win11 22H2+ 云母 Mica，其余实色回退）
//! - clipboard：系统剪贴板私有格式写入（8.4 发送至 AN：Flash 11 Picture，arboard 不管私有格式）

pub mod clipboard;
pub mod decoder;
pub mod ffmpeg;
pub mod inbox_drive;
pub mod inbox_watch;
pub mod recycle_bin;
pub mod shell_open;
pub mod shell_path;
pub mod shell_thumb;
pub mod window_fx;
