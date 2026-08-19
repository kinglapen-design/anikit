//! 动图帧流命令（阶段四·自研解码器）：动图卡播放源。
//! 前端 AssetCard 动图播放 → start_anim_stream 拿会话 → WS 收帧渲染 canvas；
//! 停止（滚出视口 / 悬停结束 / 关开关）→ stop_anim_stream。

use tauri::State;

use crate::commands::library::AppState;
use crate::services::frame_stream::{self, StreamStart};

/// 开启动图帧流会话：返回 { session, port }（WS 连接 ws://127.0.0.1:{port}/{session}）。
/// 非动图 / 会话上限 / 文件缺失返回 Err（前端回退静态缩略图，不报错打扰）。
/// async：命令线程池执行，metadata IO 不堵主线程（同 get_asset_file_path 口径）
#[tauri::command]
pub async fn start_anim_stream(state: State<'_, AppState>, asset_id: String) -> Result<StreamStart, String> {
  let assets_dir = {
    let guard = state.library.lock().map_err(|e| e.to_string())?;
    let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
    lib.assets_dir.clone()
  };
  frame_stream::start(&assets_dir, &asset_id)
}

/// 停止会话（幂等：已停止/不存在静默成功——滚出视口与卸载可能重复调）
#[tauri::command]
pub async fn stop_anim_stream(session: u64) -> Result<(), String> {
  frame_stream::stop(session);
  Ok(())
}
