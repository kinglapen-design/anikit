//! 污染文件命令（2026-10-16，1.0 收尾第 5 项）：详情「链接关系」tab 污染文件区块数据源与删除。
//! 「打开资源管理器」复用 commands::library::reveal_orphan_info（池内路径校验，文件/目录同适用）；
//! 「导入」零新命令——前端走既有 import_paths（move 语义 = 导入即清污）。

use tauri::State;

use super::library::AppState;
use crate::core::models::AssetMeta;
use crate::services::pollution::{self, PollutionItem};

/// 短锁取资产元数据快照与 assets 池目录（锁外 IO 纪律：read_dir / 回收站调用不持库锁）
fn pollution_context(state: &State<AppState>, asset_id: &str) -> Result<(std::path::PathBuf, AssetMeta), String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  let meta = lib.index.assets.get(asset_id).cloned().ok_or_else(|| "资产不存在".to_string())?;
  Ok((lib.assets_dir.clone(), meta))
}

/// 污染清单：info 文件夹内不属于已知文件集的条目（字段驱动 + 写死名，禁后缀盲扫）
#[tauri::command]
pub fn list_pollution_files(state: State<AppState>, asset_id: String) -> Result<Vec<PollutionItem>, String> {
  let (assets_dir, meta) = pollution_context(&state, &asset_id)?;
  pollution::list_pollution(&assets_dir, &meta)
}

/// 污染条目丢系统回收站（无确认弹窗，工作空间删除同口径）；
/// 服务层三层校验（单组件名 / 不在已知集 / 存在）保证永不误删库管文件
#[tauri::command]
pub fn trash_pollution_file(state: State<AppState>, asset_id: String, name: String) -> Result<(), String> {
  let (assets_dir, meta) = pollution_context(&state, &asset_id)?;
  pollution::trash_pollution(&assets_dir, &meta, &name)
}
