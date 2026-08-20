//! 动图帧流命令（阶段四·自研解码器）：动图卡播放源。
//! 前端 AssetCard 动图播放 → start_anim_stream 拿会话 → WS 收帧渲染 canvas；
//! 停止（滚出视口 / 悬停结束 / 关开关）→ stop_anim_stream；
//! 卡片尺寸变化（缩放布局）→ set_anim_stream_size 热更新出帧尺寸（不重启会话）。

use tauri::State;

use crate::commands::library::AppState;
use crate::services::frame_stream::{self, StreamStart};

/// 开启动图帧流会话：返回 { session, port }（WS 连接 ws://127.0.0.1:{port}/{session}）。
/// short_edge = 目标短边（卡片显示尺寸，后端 clamp；真短边等比、不足不放大）。
/// 非动图 / 会话上限 / 文件缺失返回 Err（前端回退静态缩略图，不报错打扰）。
/// async：命令线程池执行，metadata IO 不堵主线程（同 get_asset_file_path 口径）
#[tauri::command]
pub async fn start_anim_stream(
  state: State<'_, AppState>,
  asset_id: String,
  short_edge: u32,
) -> Result<StreamStart, String> {
  let assets_dir = {
    let guard = state.library.lock().map_err(|e| e.to_string())?;
    let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
    lib.assets_dir.clone()
  };
  frame_stream::start(&assets_dir, &asset_id, short_edge)
}

/// 停止会话（幂等：已停止/不存在静默成功——滚出视口与卸载可能重复调）
#[tauri::command]
pub async fn stop_anim_stream(session: u64) -> Result<(), String> {
  frame_stream::stop(session);
  Ok(())
}

/// 热更新出帧目标短边（卡片尺寸跟随）：会话不存在静默成功（与 stop 同幂等口径）
#[tauri::command]
pub async fn set_anim_stream_size(session: u64, short_edge: u32) -> Result<(), String> {
  frame_stream::set_size(session, short_edge);
  Ok(())
}

/// 播放 / 暂停（预览窗口）：暂停 = 解码线程推完当前帧阻塞，恢复无缝续播
#[tauri::command]
pub async fn set_anim_stream_playing(session: u64, playing: bool) -> Result<(), String> {
  frame_stream::set_playing(session, playing);
  Ok(())
}

/// 倍速（预览窗口播放条 RateControl）：delay = delay_ms / rate
#[tauri::command]
pub async fn set_anim_stream_rate(session: u64, rate: f32) -> Result<(), String> {
  frame_stream::set_rate(session, rate);
  Ok(())
}

/// 定位到目标帧（预览播放条拖动 / 逐帧步进）：快进 / 回退重开由服务层处理
#[tauri::command]
pub async fn seek_anim_stream(session: u64, frame_index: u32) -> Result<(), String> {
  frame_stream::seek(session, frame_index);
  Ok(())
}
