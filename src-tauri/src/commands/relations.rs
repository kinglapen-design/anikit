//! 命令 API：关系（6.2 选择模式，PRD 03_数据链接 02/06）——文件与档案之间关系的
//! 查询 / 批量建立 / 改称谓 / 删除。数据落空间根 relations.json（core::relations 单一写点）；
//! 写命令走撤销三步（record_file 快照任意路径，空间级文件天然支持）。
//! 锁序约定：library → space → undo（本组命令为三者同持的唯一处，固定同序防死锁）；
//! 服务失败 = 未落盘（校验全前置、原子写最后一步），undo.cancel() 收尾即可，无需库重载。

use serde::Deserialize;
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

/// 批量建立入参一条（选择模式备选区一组：选中文件 + 双框称谓）
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewRelationInput {
  pub to_kind: String,
  pub to_id: String,
  pub label: String,
  pub reverse_label: Option<String>,
}

/// 批量建立关系（选择模式确认，2026-08-24 用户定多选累积：一次为发起方添加多条）：
/// from = 发起文件/档案（kind = asset/archive，archive 的 id = folderId）；
/// 任一校验失败全不写（批量原子）；返回完整记录组（前端就地插入免重拉）。
/// kind 缺省 free（自由关系）；voice = 链接音频（6.2 三轮：无称谓，to 必须音频 asset，
/// from 不得音频——恒单向 使用者→音频，2026-08-24 用户定方案 1）。
/// 撤销标签：free 1 条 = "添加关系"，多条 = "添加 N 条关系"；voice = "链接音频" / "链接 N 条音频"
#[tauri::command]
pub async fn add_relations(
  state: State<'_, AppState>,
  from_kind: String,
  from_id: String,
  kind: Option<String>,
  items: Vec<NewRelationInput>,
) -> Result<Vec<Relation>, String> {
  let lib_guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = lib_guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  let space_guard = state.space.lock().map_err(|e| e.to_string())?;
  let space = space_guard.as_ref().ok_or_else(|| "请先打开储存空间".to_string())?;
  let kind = kind.as_deref().unwrap_or(crate::core::models::RELATION_KIND_FREE);
  let label = if kind == crate::core::models::RELATION_KIND_VOICE {
    if items.len() > 1 { format!("链接 {} 条音频", items.len()) } else { "链接音频".to_string() }
  } else if items.len() > 1 {
    format!("添加 {} 条关系", items.len())
  } else {
    "添加关系".to_string()
  };
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(&label);
  undo.record_file(&relations_file(&space.root)).map_err(|e| e.to_string())?;
  let items: Vec<(&str, &str, &str, Option<&str>)> =
    items.iter().map(|i| (i.to_kind.as_str(), i.to_id.as_str(), i.label.as_str(), i.reverse_label.as_deref())).collect();
  match relations::add_batch(&space.root, lib, (&from_kind, &from_id), kind, items) {
    Ok(rels) => {
      undo.commit().map_err(|e| e.to_string())?;
      log::info!("{label}：{} → {} 个目标", rels[0].from.name, rels.len());
      Ok(rels)
    }
    Err(e) => {
      undo.cancel();
      Err(e)
    }
  }
}

/// 编辑关系称谓（详情关系 item 点称谓框二次修改；换绑 2026-08-24 用户定废除——
/// "要换就重新再添加"）。无改动 = 取消事务不进撤销栈（同 custom_data 口径）
#[tauri::command]
pub async fn update_relation(
  state: State<'_, AppState>,
  id: String,
  label: String,
  reverse_label: Option<String>,
) -> Result<(), String> {
  let lib_guard = state.library.lock().map_err(|e| e.to_string())?;
  if lib_guard.is_none() {
    return Err("请先打开素材库".to_string());
  }
  let space_guard = state.space.lock().map_err(|e| e.to_string())?;
  let space = space_guard.as_ref().ok_or_else(|| "请先打开储存空间".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("编辑关系");
  undo.record_file(&relations_file(&space.root)).map_err(|e| e.to_string())?;
  match relations::update(&space.root, &id, &label, reverse_label.as_deref()) {
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
