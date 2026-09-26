//! 命令 API：提示词（2026-09-26 提示词轮，PRD 03_数据链接 02_文件「提示词」）。
//! 写命令走撤销三步（锁序 library → undo，与 commands::operations 同约）；服务返回无改动时
//! 取消事务（不进撤销栈，同 custom_data 口径）。assign / clear 两族抽 _op 整段（pub(crate)，
//! server/mcp 写工具 assign_prompt / clear_prompt 复用）。
//! 「删除提示词文档」= 命令层组合（delete_placements 源文档全部活位置丢回收站 + clear 绑定，
//! 单撤销事务；多文件 record 先例见 custom_data.rs delete_props_globally）。
//! 全部 async：涉及磁盘 IO 一律走线程池（规范第 2 条）。

use tauri::State;

use std::path::PathBuf;

use crate::commands::library::AppState;
use crate::commands::operations::fail;
use crate::core::library::{Library, ARCHIVES_FILE, FOLDERS_FILE, RECYCLE_FILE};
use crate::core::models::PromptBinding;
use crate::core::paths;
use crate::core::undo::DiskOp;
use crate::services::operations::{self, PlacementRef};
use crate::services::prompt;
use crate::services::views::PlacementView;

/// 分析提示词（解析图片内嵌元数据落已扫描态绑定，png/jpg/jpeg/webp 门禁在服务层）。
/// 撤销标签「分析提示词」；record 目标 metadata.json
#[tauri::command]
pub async fn analyze_asset_prompt(state: State<'_, AppState>, asset_id: String) -> Result<PromptBinding, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("分析提示词");
  undo.record_file(&paths::metadata_file(&lib.assets_dir, &asset_id)).map_err(|e| e.to_string())?;
  match prompt::analyze_asset(lib, &asset_id) {
    Ok(binding) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(binding)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 指定提示词（资产侧）。撤销标签「指定提示词」；record 目标 metadata.json
#[tauri::command]
pub async fn assign_asset_prompt(state: State<'_, AppState>, asset_id: String, prompt_asset_id: String) -> Result<(), String> {
  assign_asset_prompt_op(&state, &asset_id, &prompt_asset_id)
}

/// 指定提示词整段（锁 library → 锁 undo → 单撤销步）；pub(crate)：server/mcp
///（AI 外接通道写工具 assign_prompt）同用
pub(crate) fn assign_asset_prompt_op(state: &AppState, asset_id: &str, prompt_asset_id: &str) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("指定提示词");
  undo.record_file(&paths::metadata_file(&lib.assets_dir, asset_id)).map_err(|e| e.to_string())?;
  match prompt::assign_asset(lib, asset_id, prompt_asset_id) {
    Ok(true) => undo.commit().map_err(|e| e.to_string()),
    Ok(false) => {
      undo.cancel();
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 指定提示词（档案侧）。撤销标签「指定提示词」；record archives.json
#[tauri::command]
pub async fn assign_archive_prompt(state: State<'_, AppState>, folder_id: String, prompt_asset_id: String) -> Result<(), String> {
  assign_archive_prompt_op(&state, &folder_id, &prompt_asset_id)
}

/// 指定提示词（档案侧）整段；pub(crate)：server/mcp 同用
pub(crate) fn assign_archive_prompt_op(state: &AppState, folder_id: &str, prompt_asset_id: &str) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("指定提示词");
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  match prompt::assign_archive(lib, folder_id, prompt_asset_id) {
    Ok(true) => undo.commit().map_err(|e| e.to_string()),
    Ok(false) => {
      undo.cancel();
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 清除提示词（资产侧）：菜单「断开提示词文档链接」与「清空提示词」共用——整绑定移除
///（快照随解绑清除）。label = 撤销标签由调用方按前端语义传（「断开提示词」/「清空提示词」），
/// 缺省「移除提示词」（2026-09-26 定：命令加 label 参数分流，MCP 等外部调用走缺省）
#[tauri::command]
pub async fn clear_asset_prompt(state: State<'_, AppState>, asset_id: String, label: Option<String>) -> Result<(), String> {
  clear_asset_prompt_op(&state, &asset_id, label.as_deref())
}

/// 清除提示词（资产侧）整段；pub(crate)：server/mcp（写工具 clear_prompt）同用
pub(crate) fn clear_asset_prompt_op(state: &AppState, asset_id: &str, label: Option<&str>) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(label.unwrap_or("移除提示词"));
  undo.record_file(&paths::metadata_file(&lib.assets_dir, asset_id)).map_err(|e| e.to_string())?;
  match prompt::clear_asset(lib, asset_id) {
    Ok(true) => undo.commit().map_err(|e| e.to_string()),
    Ok(false) => {
      undo.cancel();
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 清除提示词（档案侧；label 口径同资产侧）
#[tauri::command]
pub async fn clear_archive_prompt(state: State<'_, AppState>, folder_id: String, label: Option<String>) -> Result<(), String> {
  clear_archive_prompt_op(&state, &folder_id, label.as_deref())
}

/// 清除提示词（档案侧）整段；pub(crate)：server/mcp 同用
pub(crate) fn clear_archive_prompt_op(state: &AppState, folder_id: &str, label: Option<&str>) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(label.unwrap_or("移除提示词"));
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  match prompt::clear_archive(lib, folder_id) {
    Ok(true) => undo.commit().map_err(|e| e.to_string()),
    Ok(false) => {
      undo.cancel();
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 同步提示词快照（资产侧；详情「保存提示词」钮 = 同步快照语义）。撤销标签「保存提示词」
#[tauri::command]
pub async fn refresh_asset_prompt_snapshot(state: State<'_, AppState>, asset_id: String) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("保存提示词");
  undo.record_file(&paths::metadata_file(&lib.assets_dir, &asset_id)).map_err(|e| e.to_string())?;
  match prompt::refresh_asset_snapshot(lib, &asset_id) {
    Ok(true) => undo.commit().map_err(|e| e.to_string()),
    Ok(false) => {
      undo.cancel();
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 同步提示词快照（档案侧）。撤销标签「保存提示词」；record archives.json
#[tauri::command]
pub async fn refresh_archive_prompt_snapshot(state: State<'_, AppState>, folder_id: String) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("保存提示词");
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  match prompt::refresh_archive_snapshot(lib, &folder_id) {
    Ok(true) => undo.commit().map_err(|e| e.to_string()),
    Ok(false) => {
      undo.cancel();
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 新建提示词文档（资产侧）：菜单「新建 .md/.txt 提示词文档」（未指定态）与
/// 「保存提示词为 .md/.txt 文档」（已扫描态）共用——名 = 资产显示名词干，已扫描态内容 = 快照，
/// folder_id 落点（None / "root" = 根目录）。流程 = 建文档（record folders.json +
/// record_disk_op info 备份，照 create_document_op 范式）+ 写内容 + assign 绑定
///（record 目标 metadata.json）——单撤销步「新建提示词文档」
#[tauri::command]
pub async fn new_asset_prompt_document(
  state: State<'_, AppState>,
  asset_id: String,
  folder_id: Option<String>,
  kind: String,
) -> Result<PlacementView, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let (doc_name, content) = match prompt::doc_seed(lib, &asset_id) {
    Ok(seed) => seed,
    Err(e) => return Err(e),
  };
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("新建提示词文档");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  undo.record_file(&paths::metadata_file(&lib.assets_dir, &asset_id)).map_err(|e| e.to_string())?;
  match new_prompt_document_with_bind(lib, Some(&asset_id), None, folder_id.as_deref(), &kind, &doc_name, &content) {
    Ok((view, info_dir, backup)) => {
      undo.record_disk_op(DiskOp::Rename { from: info_dir.clone(), to: backup.clone() }, DiskOp::Rename { from: backup, to: info_dir });
      undo.commit().map_err(|e| e.to_string())?;
      Ok(view)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 新建提示词文档（档案侧）：落点 = 档案自身 folderId，名 = 档案名，恒空内容
///（档案无「分析」产出路径，无已扫描态快照可带），其余同资产侧
#[tauri::command]
pub async fn new_archive_prompt_document(state: State<'_, AppState>, folder_id: String, kind: String) -> Result<PlacementView, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let doc_name = match prompt::archive_doc_seed(lib, &folder_id) {
    Ok(name) => name,
    Err(e) => return Err(e),
  };
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("新建提示词文档");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  match new_prompt_document_with_bind(lib, None, Some(&folder_id), Some(&folder_id), &kind, &doc_name, "") {
    Ok((view, info_dir, backup)) => {
      undo.record_disk_op(DiskOp::Rename { from: info_dir.clone(), to: backup.clone() }, DiskOp::Rename { from: backup, to: info_dir });
      undo.commit().map_err(|e| e.to_string())?;
      Ok(view)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 建文档 + 绑定（两命令共用）：target 二选一（asset / archive folder）
fn new_prompt_document_with_bind(
  lib: &mut Library,
  asset_id: Option<&str>,
  archive_folder_id: Option<&str>,
  doc_folder: Option<&str>,
  kind: &str,
  doc_name: &str,
  content: &str,
) -> Result<(PlacementView, PathBuf, PathBuf), String> {
  let (view, info_dir, backup) = prompt::new_prompt_document(lib, doc_folder, kind, doc_name, content)?;
  match (asset_id, archive_folder_id) {
    (Some(a), None) => {
      prompt::assign_asset(lib, a, &view.asset_id)?;
    }
    (None, Some(f)) => {
      prompt::assign_archive(lib, f, &view.asset_id)?;
    }
    _ => unreachable!("调用方保证二选一"),
  }
  Ok((view, info_dir, backup))
}

/// 删除提示词文档（资产侧）：prompt 源文档全部活 Placement 丢回收站 + 清除绑定，
/// 单撤销步「删除提示词文档」（record folders.json + recycle.json + 目标 metadata.json）。
/// 无绑定报错「未指定提示词」；已扫描态无文档可删报错
#[tauri::command]
pub async fn delete_asset_prompt_document(state: State<'_, AppState>, asset_id: String) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let src_id = {
    let meta = lib.index.assets.get(&asset_id).ok_or_else(|| "资源不存在，可能已被删除".to_string())?;
    match prompt_doc_source(meta.prompt.as_ref()) {
      Ok(id) => id,
      Err(e) => return Err(e),
    }
  };
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("删除提示词文档");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  undo.record_file(&lib.root.join(RECYCLE_FILE)).map_err(|e| e.to_string())?;
  undo.record_file(&paths::metadata_file(&lib.assets_dir, &asset_id)).map_err(|e| e.to_string())?;
  if let Err(e) = recycle_prompt_doc(lib, &src_id).and_then(|_| prompt::clear_asset(lib, &asset_id).map(|_| ())) {
    return Err(fail(lib, &mut undo, e));
  }
  undo.commit().map_err(|e| e.to_string())
}

/// 删除提示词文档（档案侧）：同资产侧口径（record folders.json + recycle.json + archives.json）
#[tauri::command]
pub async fn delete_archive_prompt_document(state: State<'_, AppState>, folder_id: String) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let src_id = {
    let bound = lib.archives.archives.iter().find(|a| a.folder_id == folder_id).and_then(|a| a.prompt.as_ref());
    match prompt_doc_source(bound) {
      Ok(id) => id,
      Err(e) => return Err(e),
    }
  };
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("删除提示词文档");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  undo.record_file(&lib.root.join(RECYCLE_FILE)).map_err(|e| e.to_string())?;
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  if let Err(e) = recycle_prompt_doc(lib, &src_id).and_then(|_| prompt::clear_archive(lib, &folder_id).map(|_| ())) {
    return Err(fail(lib, &mut undo, e));
  }
  undo.commit().map_err(|e| e.to_string())
}

/// 取提示词源文档 id：无绑定「未指定提示词」；已扫描态（无引用）「扫描结果无提示词文档可删除」
fn prompt_doc_source(prompt: Option<&PromptBinding>) -> Result<String, String> {
  match prompt {
    None => Err("未指定提示词".to_string()),
    Some(p) => p.asset_id.clone().ok_or_else(|| "扫描结果无提示词文档可删除".to_string()),
  }
}

/// 源文档全部活 Placement 丢回收站：同文件夹多链接三元组相同（PlacementRef 无 name 维度），
/// delete_placements 单轮去重只删首条——逐轮重收直至清空（2026-09-26 提示词轮）
fn recycle_prompt_doc(lib: &mut Library, src_id: &str) -> Result<(), String> {
  loop {
    let prefs: Vec<PlacementRef> = lib
      .index
      .placements_of(src_id)
      .iter()
      .map(|p| PlacementRef { asset_id: src_id.to_string(), folder_id: p.folder.clone(), origin: p.origin })
      .collect();
    if prefs.is_empty() {
      return Ok(());
    }
    operations::delete_placements(lib, &prefs, Some("recycle"))?;
  }
}
