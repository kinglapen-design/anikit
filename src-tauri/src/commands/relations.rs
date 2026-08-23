//! 命令 API：关系（6.2 选择模式一轮，PRD 03_数据链接 02/06）——文件与档案之间关系的
//! 查询 / 建立 / 编辑 / 删除。数据落空间根 relations.json（core::relations 单一写点）；
//! 写命令走撤销三步（record_file 快照任意路径，空间级文件天然支持）。
//! 锁序约定：library → space → undo（本组命令为三者同持的唯一处，固定同序防死锁）；
//! 服务失败 = 未落盘（校验全前置、原子写最后一步），undo.cancel() 收尾即可，无需库重载。

use tauri::State;

use crate::commands::library::AppState;
use crate::core::models::Relation;
use crate::core::relations::{load_relations, relations_file};
use crate::services::relations;

/// 关系清单（只读）：空间级全量返回（关系可跨库），前端按当前库过滤显示
#[tauri::command]
pub async fn list_relations(state: State<'_, AppState>) -> Result<Vec<Relation>, String> {
  let lib_guard = state.library.lock().map_err(|e| e.to_string())?;
  if lib_guard.is_none() {
    return Err("请先打开素材库".to_string());
  }
  let space_guard = state.space.lock().map_err(|e| e.to_string())?;
  let space = space_guard.as_ref().ok_or_else(|| "请先打开储存空间".to_string())?;
  Ok(load_relations(&space.root).map_err(|e| e.to_string())?.relations)
}

/// 建立关系（选择模式备选区确认）：from = 发起文件/档案，to = 选中文件/档案（kind = asset/archive）；
/// label = to 对 from 的称谓（必填），reverse_label = from 对 to 的称谓（可空，手动填写）。
/// 返回写入的完整记录（含生成 id 与显示名快照，前端就地插入免重拉）
#[tauri::command]
pub async fn add_relation(
  state: State<'_, AppState>,
  from_kind: String,
  from_id: String,
  to_kind: String,
  to_id: String,
  label: String,
  reverse_label: Option<String>,
) -> Result<Relation, String> {
  let lib_guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = lib_guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  let space_guard = state.space.lock().map_err(|e| e.to_string())?;
  let space = space_guard.as_ref().ok_or_else(|| "请先打开储存空间".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("添加关系");
  undo.record_file(&relations_file(&space.root)).map_err(|e| e.to_string())?;
  match relations::add(&space.root, lib, (&from_kind, &from_id), (&to_kind, &to_id), &label, reverse_label.as_deref()) {
    Ok(rel) => {
      undo.commit().map_err(|e| e.to_string())?;
      log::info!("已添加关系：{} → {}", rel.from.name, rel.to.name);
      Ok(rel)
    }
    Err(e) => {
      undo.cancel();
      Err(e)
    }
  }
}

/// 编辑关系（详情关系 item 二次修改）：改称谓（label / reverse_label）+ 可选换绑端点
///（new_to_* / new_from_* 二选一，成对给才生效——被换端 = 当前详情目标的对方）。
/// 无改动 = 取消事务不进撤销栈（同 custom_data 口径）
#[tauri::command]
pub async fn update_relation(
  state: State<'_, AppState>,
  id: String,
  label: String,
  reverse_label: Option<String>,
  new_to_kind: Option<String>,
  new_to_id: Option<String>,
  new_from_kind: Option<String>,
  new_from_id: Option<String>,
) -> Result<(), String> {
  let lib_guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = lib_guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  let space_guard = state.space.lock().map_err(|e| e.to_string())?;
  let space = space_guard.as_ref().ok_or_else(|| "请先打开储存空间".to_string())?;
  let new_to = match (new_to_kind, new_to_id) {
    (Some(k), Some(i)) => Some((k, i)),
    _ => None,
  };
  let new_from = match (new_from_kind, new_from_id) {
    (Some(k), Some(i)) => Some((k, i)),
    _ => None,
  };
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("编辑关系");
  undo.record_file(&relations_file(&space.root)).map_err(|e| e.to_string())?;
  match relations::update(
    &space.root,
    lib,
    &id,
    &label,
    reverse_label.as_deref(),
    new_to.as_ref().map(|(k, i)| (k.as_str(), i.as_str())),
    new_from.as_ref().map(|(k, i)| (k.as_str(), i.as_str())),
  ) {
    Ok(true) => undo.commit().map_err(|e| e.to_string()),
    Ok(false) => {
      undo.cancel();
      Ok(())
    }
    Err(e) => {
      undo.cancel();
      Err(e)
    }
  }
}

/// 删除关系（详情关系 item ×）
#[tauri::command]
pub async fn remove_relation(state: State<'_, AppState>, id: String) -> Result<(), String> {
  let lib_guard = state.library.lock().map_err(|e| e.to_string())?;
  if lib_guard.is_none() {
    return Err("请先打开素材库".to_string());
  }
  let space_guard = state.space.lock().map_err(|e| e.to_string())?;
  let space = space_guard.as_ref().ok_or_else(|| "请先打开储存空间".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("删除关系");
  undo.record_file(&relations_file(&space.root)).map_err(|e| e.to_string())?;
  match relations::remove(&space.root, &id) {
    Ok(()) => undo.commit().map_err(|e| e.to_string()),
    Err(e) => {
      undo.cancel();
      Err(e)
    }
  }
}
