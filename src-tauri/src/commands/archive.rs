//! 命令 API：档案文件夹（2026-08-15 档案子系统一轮骨架，PRD 02_文件整理 03 / 03_数据链接）。
//! 写命令走撤销三步（锁序 library → undo，与 commands::operations 同约）；只读命令同步直出。

use tauri::State;

use crate::commands::library::AppState;
use crate::commands::operations::fail;
use crate::core::library::{Library, ARCHIVES_FILE, FOLDERS_FILE};
use crate::core::models::{ArchiveData, Folder, Placement, ARCHIVE_KIND, DEFAULT_BLOCK_ID, ROOT_FOLDER_ID};
use crate::services::archive::{self, ConvertOutcome};

/// 新建档案：parentId = null / "root" 为顶层；afterFolderId / blockId 口径同 create_folder
#[tauri::command]
pub async fn create_archive(
  state: State<'_, AppState>,
  parent_id: Option<String>,
  name: String,
  after_folder_id: Option<String>,
  block_id: Option<String>,
) -> Result<Folder, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("新建档案");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match archive::create_archive(lib, parent_id.as_deref(), &name, after_folder_id.as_deref(), block_id.as_deref()) {
    Ok(folder) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(folder)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 文件夹转换为档案（两段式）：有子文件夹且 confirmed = false 时返回 "need_confirm"
/// 不动数据（询问段丢弃事务，不进撤销栈）；否则剔除层级并置 kind，返回 "done"；已是档案幂等 "done"
#[tauri::command]
pub async fn convert_to_archive(
  state: State<'_, AppState>,
  folder_id: String,
  confirmed: bool,
) -> Result<String, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("转换为档案");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  match archive::convert_to_archive(lib, &folder_id, confirmed) {
    // 询问段未动数据：丢弃事务，不进撤销栈
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

/// 档案解散为普通文件夹：摘 kind 标记 + 清 archives.json 中该档案的记录
#[tauri::command]
pub async fn convert_to_folder(state: State<'_, AppState>, folder_id: String) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("解散档案为文件夹");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  undo.record_file(&lib.root.join(ARCHIVES_FILE)).map_err(|e| e.to_string())?;
  match archive::convert_to_folder(lib, &folder_id) {
    Ok(()) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 全部档案数据（克隆全量，量小）
#[tauri::command]
pub fn list_archives(state: State<AppState>) -> Result<Vec<ArchiveData>, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  Ok(archive::list_archives(lib))
}

/// 单个档案的详细数据；无记录返回默认空记录（档案刚转换、尚未配置数据）
#[tauri::command]
pub fn get_archive_data(state: State<AppState>, folder_id: String) -> Result<ArchiveData, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  Ok(lib.archives.archives.iter().find(|a| a.folder_id == folder_id).cloned().unwrap_or_else(|| ArchiveData::new(&folder_id)))
}

/// 资产详情（PRD 03_数据链接 02）：基础信息 + 属性 / 链接 / 注释 + 链接区块（Placement 反查）+ 归属档案
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDetail {
  pub asset_id: String,
  pub origin_name: String,
  pub ext: String,
  pub size: u64,
  pub width: Option<u32>,
  pub height: Option<u32>,
  pub duration: Option<f64>,
  pub imported_at: String,
  pub modified_at: String,
  /// 属性词（复用原 tags 字段，2026-08-04 标签体系取消后语义转为属性，PRD 03_数据链接 06）
  pub tags: Vec<String>,
  pub links: Vec<String>,
  pub note: String,
  /// 链接区块：该资产全部 Placement 反查
  pub placements: Vec<PlacementLink>,
  /// 归属档案：Placement 所在 folder 链上 kind = archive 者去重
  pub archives: Vec<ArchiveRef>,
}

/// 链接区块一条：哪个区块的哪个文件夹、显示什么名称
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementLink {
  /// 所在文件夹 ID；未分类（root Placement）= null
  pub folder_id: Option<String>,
  /// 所在区块名（未分类 / 无区块归属 = 默认区块「目录」）
  pub block_name: String,
  /// 路径段，最多三段：根目录 / 父级 / 所在目录（超出三段取首段 + 末两段）；未分类 = ["未分类"]
  pub path: Vec<String>,
  pub name: String,
  pub origin: bool,
}

/// 归属档案一条
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveRef {
  pub folder_id: String,
  pub name: String,
}

/// 资产详情（只读）；资产不存在报中文错
#[tauri::command]
pub fn get_asset_detail(state: State<AppState>, asset_id: String) -> Result<AssetDetail, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  asset_detail(lib, &asset_id)
}

/// 组装资产详情（命令层薄壳，便于单测）
fn asset_detail(lib: &Library, asset_id: &str) -> Result<AssetDetail, String> {
  let meta = lib.index.assets.get(asset_id).ok_or_else(|| "资源不存在，可能已被删除".to_string())?;
  let mut placements = Vec::new();
  let mut archives: Vec<ArchiveRef> = Vec::new();
  for p in lib.index.placements_of(asset_id) {
    let Some(link) = placement_link(lib, p) else { continue };
    // 归属档案反查：沿 folder 父链上溯，kind = archive 者去重收集
    let mut cur = link.folder_id.clone();
    while let Some(id) = cur {
      let Some(f) = lib.index.folders.get(&id) else { break };
      if f.kind.as_deref() == Some(ARCHIVE_KIND) && !archives.iter().any(|a| a.folder_id == id) {
        archives.push(ArchiveRef { folder_id: id.clone(), name: f.name.clone() });
      }
      cur = f.parent.clone();
    }
    placements.push(link);
  }
  Ok(AssetDetail {
    asset_id: meta.id.clone(),
    origin_name: meta.origin_name.clone(),
    ext: meta.ext.clone(),
    size: meta.size,
    width: meta.width,
    height: meta.height,
    duration: meta.duration,
    imported_at: meta.imported_at.clone(),
    modified_at: meta.modified_at.clone(),
    tags: meta.tags.clone(),
    links: meta.links.clone(),
    note: meta.note.clone(),
    placements,
    archives,
  })
}

/// Placement → 链接区块一条；folder 链断裂（脏数据）返回 None 整条跳过
fn placement_link(lib: &Library, p: &Placement) -> Option<PlacementLink> {
  if p.folder == ROOT_FOLDER_ID {
    return Some(PlacementLink {
      folder_id: None,
      block_name: block_name(lib, None),
      path: vec!["未分类".to_string()],
      name: p.name.clone(),
      origin: p.origin,
    });
  }
  // folder 链（所在目录 → 根目录逐级上溯）
  let mut chain: Vec<&Folder> = Vec::new();
  let mut cur = Some(p.folder.clone());
  while let Some(id) = cur {
    let f = lib.index.folders.get(&id)?;
    chain.push(f);
    cur = f.parent.clone();
  }
  chain.reverse();
  let top_block = chain[0].block.clone();
  let mut path: Vec<String> = chain.iter().map(|f| f.name.clone()).collect();
  // 最多三段（PRD 03_数据链接 02：区块 / 根目录父级 / 所在目录；区块由 blockName 单列）
  if path.len() > 3 {
    path = vec![path[0].clone(), path[path.len() - 2].clone(), path[path.len() - 1].clone()];
  }
  Some(PlacementLink {
    folder_id: Some(p.folder.clone()),
    block_name: block_name(lib, top_block.as_deref()),
    path,
    name: p.name.clone(),
    origin: p.origin,
  })
}

/// 区块名：None / 失效引用 = 默认区块「目录」
fn block_name(lib: &Library, block: Option<&str>) -> String {
  let id = block.unwrap_or(DEFAULT_BLOCK_ID);
  lib.folders.blocks.iter().find(|b| b.id == id).map(|b| b.name.clone()).unwrap_or_else(|| "目录".to_string())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library;
  use crate::core::models::{AssetMeta, Block};

  struct Harness {
    _dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { _dir: dir, lib }
  }

  fn add_folder(lib: &mut Library, id: &str, name: &str, parent: Option<&str>, block: Option<&str>, order: i32) {
    let folder = Folder {
      id: id.into(),
      name: name.into(),
      parent: parent.map(|s| s.into()),
      block: block.map(|s| s.into()),
      icon: None,
      kind: None,
      order,
    };
    lib.folders.folders.push(folder.clone());
    lib.index.folders.insert(id.into(), folder);
  }

  fn add_asset(lib: &mut Library, id: &str) {
    let meta: AssetMeta = serde_json::from_value(serde_json::json!({
      "version": 1, "id": id, "name": "人物", "originName": "人物.fla", "ext": "fla", "size": 100,
      "width": 1920, "height": 1080, "importedAt": "2026-08-15T00:00:00+08:00",
      "modifiedAt": "2026-08-15T00:00:00+08:00", "tags": ["主角"], "links": ["https://x"], "note": "注释"
    }))
    .unwrap();
    lib.index.assets.insert(id.into(), meta);
  }

  fn add_placement(lib: &mut Library, asset: &str, folder: &str, name: &str, origin: bool) {
    lib.folders.items.push(Placement { asset: asset.into(), folder: folder.into(), name: name.into(), order: 0, origin });
    lib.index.rebuild_placements(&lib.folders.items);
  }

  /// 链接区块反查：普通文件夹链（区块名 + 三段路径）与未分类（目录 / ["未分类"]）
  #[test]
  fn asset_detail_placement_links() {
    let mut h = make_lib();
    h.lib.folders.blocks.push(Block { id: "B01".into(), name: "角色".into(), icon: None, order: 1 });
    add_folder(&mut h.lib, "F-TOP", "顶级", None, Some("B01"), 0);
    add_folder(&mut h.lib, "F-MID", "父级", Some("F-TOP"), None, 0);
    add_folder(&mut h.lib, "F-LEAF", "所在", Some("F-MID"), None, 0);
    add_asset(&mut h.lib, "AN-A");
    add_placement(&mut h.lib, "AN-A", "F-LEAF", "男人.fla", true);
    add_placement(&mut h.lib, "AN-A", "root", "人物.fla", false);

    let d = asset_detail(&h.lib, "AN-A").unwrap();
    assert_eq!(d.asset_id, "AN-A");
    assert_eq!((d.width, d.height), (Some(1920), Some(1080)));
    assert_eq!(d.tags, vec!["主角"]);
    assert_eq!(d.placements.len(), 2);
    let dir_link = &d.placements[0];
    assert_eq!(dir_link.folder_id.as_deref(), Some("F-LEAF"));
    assert_eq!(dir_link.block_name, "角色", "顶级祖先的区块");
    assert_eq!(dir_link.path, vec!["顶级", "父级", "所在"], "三段：根目录 / 父级 / 所在目录");
    assert!(dir_link.origin);
    let root_link = &d.placements[1];
    assert_eq!(root_link.folder_id, None, "未分类 folderId = null");
    assert_eq!(root_link.block_name, "目录");
    assert_eq!(root_link.path, vec!["未分类"]);
    assert!(!root_link.origin);
    // 资产不存在报中文错
    assert!(asset_detail(&h.lib, "AN-不存在").is_err());
  }

  /// 超过三段的链：取首段 + 末两段
  #[test]
  fn asset_detail_path_truncates_to_three_segments() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F1", "一", None, None, 0);
    add_folder(&mut h.lib, "F2", "二", Some("F1"), None, 0);
    add_folder(&mut h.lib, "F3", "三", Some("F2"), None, 0);
    add_folder(&mut h.lib, "F4", "四", Some("F3"), None, 0);
    add_asset(&mut h.lib, "AN-A");
    add_placement(&mut h.lib, "AN-A", "F4", "x.fla", true);

    let d = asset_detail(&h.lib, "AN-A").unwrap();
    assert_eq!(d.placements[0].path, vec!["一", "三", "四"], "首段（根目录）+ 末两段（父级 / 所在目录）");
    assert_eq!(d.placements[0].block_name, "目录", "无区块归属 = 默认区块");
  }

  /// 归属档案反查：folder 链上 kind = archive 者去重（含嵌套档案、跨 Placement 去重）
  #[test]
  fn asset_detail_archive_refs_deduped() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-A", "档案甲", None, None, 0);
    add_folder(&mut h.lib, "F-B", "档案乙", None, None, 1);
    add_folder(&mut h.lib, "F-C", "普通", Some("F-A"), None, 0);
    for id in ["F-A", "F-B"] {
      h.lib.folders.folders.iter_mut().find(|f| f.id == id).unwrap().kind = Some(ARCHIVE_KIND.into());
      h.lib.index.folders.get_mut(id).unwrap().kind = Some(ARCHIVE_KIND.into());
    }
    add_asset(&mut h.lib, "AN-A");
    add_placement(&mut h.lib, "AN-A", "F-C", "a.fla", true); // 链上：F-A
    add_placement(&mut h.lib, "AN-A", "F-A", "b.fla", false); // 档案内直接：F-A（去重）
    add_placement(&mut h.lib, "AN-A", "F-B", "c.fla", false); // F-B

    let d = asset_detail(&h.lib, "AN-A").unwrap();
    assert_eq!(
      d.archives.iter().map(|a| (a.folder_id.as_str(), a.name.as_str())).collect::<Vec<_>>(),
      vec![("F-A", "档案甲"), ("F-B", "档案乙")],
      "去重且按首次出现序"
    );
  }
}
