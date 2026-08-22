//! 视频帧流命令（5.3 全面接入自研解码器）：视频卡 / 预览播放源。
//! 前端 → start_video_stream 拿会话 → WS 收帧（视频帧 + 音轨 PCM 块 + flush/ended 控制消息）；
//! 播放 / 暂停 / 倍速 / 出帧尺寸 / 停止复用 anim_stream 命令组（按 session id 操作，无类型门槛）。
//! 定位语义 = 秒（seek_video_stream）：任何方向后端真 seek（ffmpeg 关键帧落点 + 前向丢帧，
//! 与动图帧索引顺序快进不同）；逐帧步进 = 暂停 + seek((idx±1)/fps)。
//! 帧率自愈：存量视频 metadata 无 fps（5.3 前导入）——启动时先补探测写回（heal_video_fps，
//! 与 get_media_source 共用）；仍不可得 = Err（前端回退静态缩略图，帧率语义缺失不猜默认值）。

use tauri::State;

use crate::commands::library::AppState;
use crate::commands::media::heal_video_fps;
use crate::commands::preview::locate_asset;
use crate::services::frame_stream::{self, StreamStart};

/// 开启视频帧流会话：返回 { session, port, hasAudio, fps, duration }
/// （WS 连接 ws://127.0.0.1:{port}/{session}；协议 v2 见 services/frame_stream 模块头）。
/// short_edge = 目标短边（卡片 / 预览显示尺寸，后端 clamp；真短边等比、不足不放大）。
/// start_secs = 首趟起播点（卡片自动播放 = 封面帧 t:{秒}；预览 = 0）。
/// loop_enabled = 循环（卡片恒 true；预览跟随播放条循环钮，set_stream_loop 热改）。
/// 非视频 / fps 不可得 / 会话上限 / 文件缺失返回 Err（前端回退静态缩略图，不报错打扰）。
/// async：命令线程池执行，probe / metadata IO 不堵主线程（同 get_asset_file_path 口径）
#[tauri::command]
pub async fn start_video_stream(
  app: tauri::AppHandle,
  state: State<'_, AppState>,
  asset_id: String,
  short_edge: u32,
  start_secs: f64,
  loop_enabled: bool,
) -> Result<StreamStart, String> {
  let assets_dir = {
    let mut guard = state.library.lock().map_err(|e| e.to_string())?;
    let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
    let (info_dir, origin, mut meta) = locate_asset(lib, &asset_id)?;
    if !crate::services::media::is_video(&origin) {
      return Err(format!("「{}」不是视频资源", meta.name));
    }
    heal_video_fps(lib, &asset_id, &info_dir, &origin, &mut meta, Some(&app));
    lib.assets_dir.clone()
  };
  frame_stream::start_video(&assets_dir, &asset_id, short_edge, start_secs, loop_enabled)
}

/// 定位到目标秒（预览播放条拖动 / 逐帧步进 / 卡片 scrub / 自动播放范围循环回起点）：
/// 任何方向后端真 seek；到位推帧后按播放态继续或停住（暂停态 = 逐帧定位语义）。
/// 幂等，失败静默（会话可能已消亡——同 stop_anim_stream 口径）
#[tauri::command]
pub async fn seek_video_stream(session: u64, secs: f64) -> Result<(), String> {
  frame_stream::seek_video(session, secs);
  Ok(())
}

/// 循环开关（预览播放条循环钮）：false = 自然到底发 ended 停末帧（播放键 = seek(0) 重开）。
/// 幂等，失败静默
#[tauri::command]
pub async fn set_stream_loop(session: u64, loop_enabled: bool) -> Result<(), String> {
  frame_stream::set_loop(session, loop_enabled);
  Ok(())
}
