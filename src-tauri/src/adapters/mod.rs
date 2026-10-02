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
//! - window_fx：窗口背景效果（2026-10-01 终局转向：弃云母恒 solid 实底，换回最大化过渡动画）
//! - clipboard：系统剪贴板私有格式写入（8.4 发送至 AN：Flash 11 Picture，arboard 不管私有格式）
//! - shell_menu：Windows 原生右键菜单（IContextMenu 经典菜单 + 消息转发窗口；工作空间「系统菜单」项）
//! - junction：NTFS 目录链接（工作空间「外部文件夹挂进 WS」方向 b：创建 / 读目标 / 删除 reparse point）
//! - space_icon：储存空间文件夹品牌图标（desktop.ini IconResource + 文件夹只读位，Eagle .library 同款）
//! - everything：Everything 搜索 IPC（工作空间搜索 2026-10-03：手写 1.4 WM_COPYDATA QUERY2，
//!   能力三态探测 + 回包接收窗，不引 DLL / es.exe 零分发授权问题）

pub mod clipboard;
pub mod decoder;
pub mod everything;
pub mod ffmpeg;
pub mod inbox_drive;
pub mod inbox_watch;
pub mod junction;
pub mod recycle_bin;
pub mod shell_menu;
pub mod shell_open;
pub mod shell_path;
pub mod shell_thumb;
pub mod space_icon;
pub mod window_fx;
