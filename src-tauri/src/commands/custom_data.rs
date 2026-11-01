//! 命令 API：自定义数据（6.1，PRD 03_数据链接 02/06）——文件与档案的属性 / 描述编辑、
//! 属性池（32 稿「已有属性标签」）、全局删除属性（32 稿池内 ×，前端警告二次确认后调用）。
//! 写命令走撤销三步（锁序 library → undo，与 commands::operations 同约）；
//! 服务返回无改动时取消事务（不进撤销栈，与 convert_to_archive 询问段同口径）。

use tauri::State;

use crate::commands::library::AppState;
use crate::commands::operations::fail;
use crate::core::library::ARCHIVES_FILE;
use crate::core::models::DescItem;
use crate::core::paths;
use crate::services::custom_data;

/// 编辑文件属性（tags 语义 = 属性词）
#[tauri::command]
pub async fn set_asset_props(state: State<'_, AppState>, asset_id: String, props: Vec<String>) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("编辑属性");
  undo.record_file(&paths::info_dir(&lib.assets_dir, &asset_id).join("metadata.json")).map_err(|e| e.to_string())?;
  match custom_data::set_asset_props(lib, &asset_id, &props) {
    Ok(true) => undo.commit().map_err(|e| e.to_string()),
    Ok(false) => {
      undo.cancel();
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 编辑文件描述（标题 + 内容逐条）
#[tauri::command]
pub async fn set_asset_descriptions(state: State<'_, AppState>, asset_id: String, descriptions: Vec<DescItem>) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("编辑描述");
  undo.record_file(&paths::info_dir(&lib.assets_dir, &asset_id).join("metadata.json")).map_err(|e| e.to_string())?;
  match custom_data::set_asset_descriptions(lib, &asset_id, &descriptions) {
    Ok(true) => undo.commit().map_err(|e| e.to_string()),
    Ok(false) => {
      undo.cancel();
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 编辑文件链接（metadata.links URL 列表，2026-10-08 S4 anikit 协议消费端：详情链接输入框可编辑化）
#[tauri::command]
pub async fn set_asset_links(state: State<'_, AppState>, asset_id: String, links: Vec<String>) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("编辑链接");
  undo.record_file(&paths::info_dir(&lib.assets_dir, &asset_id).join("metadata.json")).map_err(|e| e.to_string())?;
  match custom_data::set_asset_links(lib, &asset_id, &links) {
    Ok(true) => undo.commit().map_err(|e| e.to_string()),
    Ok(false) => {
      undo.cancel();
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 编辑档案链接（archives.json links，同上）
#[tauri::command]
pub async fn set_archive_links(state: State<'_, AppState>, folder_id: String, links: Vec<String>) -> Result<(), String> {
  super::archive_ops::set_archive_links_op(&state, &folder_id, &links)
}

/// 属性池（只读）：全库聚合去重（资产 tags + 档案 props），按小写字典序
#[tauri::command]
pub fn list_prop_pool(state: State<AppState>) -> Result<Vec<String>, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  Ok(custom_data::list_prop_pool(lib))
}

/// 全局删除属性（二次确认后）：全部命中资产 tags / 档案 props 移除该批词，单撤销步。
/// 返回影响处数（命中资产数 + 命中档案数，toast 用）
#[tauri::command]
pub async fn delete_props_globally(state: State<'_, AppState>, words: Vec<String>) -> Result<u32, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  // 先汇总影响面（只读扫描，资产 / 档案均按 id 去重）→ 按影响面 record 快照 → 再逐词执行
  let mut asset_ids: Vec<String> = Vec::new();
  let mut archive_ids: Vec<String> = Vec::new();
  for w in &words {
    let usage = custom_data::prop_usage(lib, w);
    for id in usage.asset_ids {
      if !asset_ids.contains(&id) {
        asset_ids.push(id);
      }
    }
    for id in usage.archive_ids {
      if !archive_ids.contains(&id) {
        archive_ids.push(id);
      }
    }
  }
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("删除属性");
  for id in &asset_ids {
    undo.record_file(&paths::info_dir(&lib.assets_dir, id).join("metadata.json")).map_err(|e| e.to_string())?;
  }
  if !archive_ids.is_empty() {
    undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  }
  for w in &words {
    if let Err(e) = custom_data::delete_prop_globally(lib, w) {
      return Err(fail(lib, &mut undo, e));
    }
  }
  undo.commit().map_err(|e| e.to_string())?;
  Ok(asset_ids.len() as u32 + archive_ids.len() as u32)
}
