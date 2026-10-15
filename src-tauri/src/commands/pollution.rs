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

/// 全库污染扫描（2026-10-16 立项，PRD 03_查找文件-05）：两段短锁 + spawn_blocking——
/// 锁 1 快照（assets 池目录 + 有活 Placement 的资产 meta 集，回收站 / 无位置记录不扫）；
/// 锁外逐资产 read_dir（万级资产秒级内）；锁 2 给命中资产装卡片视图（纯内存）。
/// 无主 info（孤儿 / 他库资产）不在此列——开库自检与无主资产弹窗另管
#[tauri::command]
pub async fn scan_pollution(state: State<'_, AppState>) -> Result<Vec<pollution::PollutionAssetView>, String> {
  let (assets_dir, metas) = {
    let guard = state.library.lock().map_err(|e| e.to_string())?;
    let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
    let metas: Vec<AssetMeta> = lib
      .index
      .assets
      .values()
      .filter(|m| !lib.index.placements_of(&m.id).is_empty())
      .cloned()
      .collect();
    (lib.assets_dir.clone(), metas)
  };
  let hits = tauri::async_runtime::spawn_blocking(move || pollution::scan_pollution(&assets_dir, &metas))
    .await
    .map_err(|e| e.to_string())?;
  if hits.is_empty() {
    return Ok(Vec::new());
  }
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut views: Vec<pollution::PollutionAssetView> = hits
    .into_iter()
    .filter_map(|(id, items)| {
      let meta = lib.index.assets.get(&id)?;
      Some(pollution::PollutionAssetView {
        asset: crate::services::scan::dup_asset_view(lib, meta, &lib.format_icons),
        info_path: crate::core::paths::info_dir(&lib.assets_dir, &id).to_string_lossy().into_owned(),
        items,
      })
    })
    .collect();
  // 污染条目数降序（清理视角重灾区在前），平手按显示名再按 id（确定性输出）
  views.sort_by(|a, b| {
    b.items
      .len()
      .cmp(&a.items.len())
      .then_with(|| a.asset.name.cmp(&b.asset.name))
      .then_with(|| a.asset.asset_id.cmp(&b.asset.asset_id))
  });
  Ok(views)
}
