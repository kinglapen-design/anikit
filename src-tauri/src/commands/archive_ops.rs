//! 档案命令共享操作：APP 命令与 MCP 写工具同走锁、撤销和业务服务。

use crate::commands::library::AppState;
use crate::commands::operations::fail;
use crate::core::library::{Library, ARCHIVES_FILE, FOLDERS_FILE};
use crate::core::models::{DescItem, Folder, ARCHIVE_KIND};
use crate::services::archive::{self, ConvertOutcome};
use crate::services::purge::{self, DeleteFolderReport};
use crate::services::custom_data;

pub(crate) fn create_archive_op(
  state: &AppState,
  parent_id: Option<&str>,
  name: &str,
  after_folder_id: Option<&str>,
  block_id: Option<&str>,
) -> Result<Folder, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("新建档案");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  match archive::create_archive(lib, parent_id, name, after_folder_id, block_id) {
    Ok(folder) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(folder)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

pub(crate) fn convert_to_archive_op(state: &AppState, folder_id: &str, confirmed: bool) -> Result<String, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("转换为档案");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  match archive::convert_to_archive(lib, folder_id, confirmed) {
    Ok(ConvertOutcome::NeedConfirm) => {
      undo.cancel();
      Ok("need_confirm".to_string())
    }
    Ok(ConvertOutcome::Done) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok("done".to_string())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

pub(crate) fn convert_to_folder_op(state: &AppState, folder_id: &str) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("解散档案为文件夹");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  match archive::convert_to_folder(lib, folder_id) {
    Ok(()) => undo.commit().map_err(|e| e.to_string()),
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

fn edit_archive_data_op(
  state: &AppState,
  label: &str,
  edit: impl FnOnce(&mut Library) -> Result<bool, String>,
) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(label);
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  match edit(lib) {
    Ok(true) => undo.commit().map_err(|e| e.to_string()),
    Ok(false) => {
      undo.cancel();
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

pub(crate) fn set_archive_props_op(state: &AppState, folder_id: &str, props: &[String]) -> Result<(), String> {
  edit_archive_data_op(state, "编辑属性", |lib| custom_data::set_archive_props(lib, folder_id, props))
}

pub(crate) fn set_archive_descriptions_op(state: &AppState, folder_id: &str, descriptions: &[DescItem]) -> Result<(), String> {
  edit_archive_data_op(state, "编辑描述", |lib| custom_data::set_archive_descriptions(lib, folder_id, descriptions))
}

pub(crate) fn set_archive_links_op(state: &AppState, folder_id: &str, links: &[String]) -> Result<(), String> {
  edit_archive_data_op(state, "编辑链接", |lib| custom_data::set_archive_links(lib, folder_id, links))
}

pub(crate) fn delete_archive_op(state: &AppState, folder_id: &str) -> Result<DeleteFolderReport, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let folder = lib.index.folders.get(folder_id).ok_or_else(|| "目标档案不存在，可能已被删除".to_string())?;
  if folder.kind.as_deref() != Some(ARCHIVE_KIND) {
    return Err("目标不是档案".to_string());
  }
  let name = folder.name.clone();
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(format!("删除文件夹 {name}"));
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  match purge::delete_folders(lib, std::slice::from_ref(&folder_id.to_string())) {
    Ok(report) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(report)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library;

  fn make_state() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    let state = AppState {
      space: std::sync::Mutex::new(None),
      library: std::sync::Mutex::new(Some(lib)),
      imports: std::sync::Mutex::new(std::collections::HashMap::new()),
      inbox: std::sync::Mutex::new(None),
      workspace: std::sync::Mutex::new(Vec::new()),
      undo: std::sync::Mutex::new(crate::core::undo::UndoManager::default()),
      ws_searches: std::sync::Mutex::new(std::collections::HashMap::new()),
      semantic_tasks: std::sync::Mutex::new(std::collections::HashMap::new()),
    };
    (dir, state)
  }

  #[test]
  fn create_edit_delete_and_undo_share_command_path() {
    let (_dir, state) = make_state();
    let folder = create_archive_op(&state, None, "雪牧", None, None).unwrap();
    set_archive_props_op(&state, &folder.id, &["主角".into(), " 主角 ".into()]).unwrap();
    let descriptions = vec![DescItem { title: "身份".into(), content: "漫剧主角".into() }];
    set_archive_descriptions_op(&state, &folder.id, &descriptions).unwrap();
    set_archive_links_op(&state, &folder.id, &["https://ref".into()]).unwrap();
    {
      let guard = state.library.lock().unwrap();
      let detail = crate::commands::archive::archive_detail(guard.as_ref().unwrap(), &folder.id);
      assert_eq!(detail.props, vec!["主角"]);
      assert_eq!(detail.descriptions, descriptions);
      assert_eq!(detail.links, vec!["https://ref"]);
    }

    let report = delete_archive_op(&state, &folder.id).unwrap();
    assert_eq!(report.deleted_folders, 1);
    {
      let guard = state.library.lock().unwrap();
      let lib = guard.as_ref().unwrap();
      assert!(!lib.index.folders.contains_key(&folder.id));
      assert!(lib.archives.archives.is_empty(), "删除当场清档案记录");
    }
    state.undo.lock().unwrap().undo().unwrap();
    let mut guard = state.library.lock().unwrap();
    let lib = guard.as_mut().unwrap();
    library::reload(lib).unwrap();
    assert_eq!(lib.index.folders[&folder.id].kind.as_deref(), Some(ARCHIVE_KIND));
    assert_eq!(lib.archives.archives[0].descriptions, descriptions, "撤销恢复档案数据");
  }

  #[test]
  fn conversion_and_delete_keep_archive_guards() {
    let (_dir, state) = make_state();
    let (parent, child) = {
      let mut guard = state.library.lock().unwrap();
      let lib = guard.as_mut().unwrap();
      let parent = crate::services::operations::create_folder(lib, None, "父", None, None).unwrap();
      let child = crate::services::operations::create_folder(lib, Some(&parent.id), "子", None, None).unwrap();
      (parent, child)
    };
    assert_eq!(convert_to_archive_op(&state, &parent.id, false).unwrap(), "need_confirm");
    assert!(state.library.lock().unwrap().as_ref().unwrap().index.folders.contains_key(&child.id));
    assert_eq!(convert_to_archive_op(&state, &parent.id, true).unwrap(), "done");
    assert!(!state.library.lock().unwrap().as_ref().unwrap().index.folders.contains_key(&child.id));
    convert_to_folder_op(&state, &parent.id).unwrap();
    assert!(state.library.lock().unwrap().as_ref().unwrap().archives.archives.is_empty());
    assert!(delete_archive_op(&state, &parent.id).is_err(), "普通文件夹不能走档案删除");
  }
}
