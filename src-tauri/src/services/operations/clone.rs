//! 克隆文件夹（3.4，PRD 02-13 复制克隆；2026-08-18 用户定：没有链接克隆——文件夹没有链接语义，
//! 档案的链接走文件同款链接管理、不是克隆）：递归复制子树——
//! - 文件夹节点新 id（图标 / 颜色 / kind / 区块归属保留；根克隆名 "xx 副本" 同级唯一化、紧随源之后）
//! - 子树 origin 资产物理复制（info 目录整体拷贝 + 新 ID，duplicate 同管线；同资产只复制一次）
//! - Placement 结构忠实重建：origin 挂对应新文件夹（origin=true）；链接 Placement 指向子树内新副本，
//!   指向子树外资产的链接原样保留（不越界复制别夹本体）
//! - 档案节点 archives.json 记录深拷贝（badge / avatar / props / descriptions / links 随迁）
//! 任一根失败整体 Err：已复制目录删除 + 内存数据回滚（撤销事务由命令层 cancel）。

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::core::json_io::{read_json, write_json};
use crate::core::library::{self, Library};
use crate::core::models::{new_asset_id, ArchiveData, AssetMeta, Folder, Placement, ARCHIVE_KIND};
use crate::services::import as ops;

use super::{finish, now, sync_folder_index};

/// 克隆结果：新根文件夹 id 清单 + 节点/资产计数 + （源 info, 新 info）目录对（命令层登记 DiskOp 用）
#[derive(Debug, Default)]
pub struct CloneOutcome {
  pub folder_ids: Vec<String>, // 新根（顺序 = 传入序）
  pub folder_count: usize,     // 全部新节点数（含子级）
  pub asset_count: usize,      // 物理复制的资产数
  pub copies: Vec<(PathBuf, PathBuf)>,
  // —— 回滚账本（失败时按账清理）——
  new_node_ids: Vec<String>,   // 全部新文件夹节点
  new_asset_ids: Vec<String>,  // 全部新资产
  new_archive_ids: Vec<String>, // 新建档案记录的 folderId
}

/// 批量克隆：每个传入 id 独立克隆一棵（父子同选 = 两棵都建，不去重——克隆无副作用，用户意图照单执行）；
/// 陈旧 id（选中后已被删）跳过
pub fn clone_folders(lib: &mut Library, folder_ids: &[String]) -> Result<CloneOutcome, String> {
  let mut out = CloneOutcome::default();
  let mut seen = HashSet::new();
  for id in folder_ids {
    if !seen.insert(id) {
      continue;
    }
    if !lib.index.folders.contains_key(id) {
      continue; // 陈旧 id 跳过
    }
    if let Err(e) = clone_one_root(lib, id, &mut out) {
      rollback(lib, &out);
      return Err(e);
    }
  }
  if out.folder_count > 0 {
    sync_folder_index(lib);
    finish(lib)?;
    if !out.new_archive_ids.is_empty() {
      library::save_archives(lib).map_err(|e| e.to_string())?;
    }
    log::info!("克隆 {} 棵文件夹（{} 个节点，{} 个资产物理副本）", out.folder_ids.len(), out.folder_count, out.asset_count);
  }
  Ok(out)
}

/// 单棵克隆：根名 "xx 副本" 同级唯一化并插到源之后 → 递归建节点 → 子树 Placement 重建
fn clone_one_root(lib: &mut Library, src_id: &str, out: &mut CloneOutcome) -> Result<(), String> {
  let src = lib.index.folders.get(src_id).cloned().ok_or_else(|| "目标文件夹不存在，可能已被删除".to_string())?;
  let sibling_names: HashSet<String> =
    lib.folders.folders.iter().filter(|f| f.parent == src.parent).map(|f| f.name.clone()).collect();
  let root_name = library::unique_name(&sibling_names, &format!("{} 副本", src.name));
  // 紧随源之后：同级后续兄弟 order 顺移 +1（create_folder 同款插入）
  let insert_order = src.order + 1;
  for f in &mut lib.folders.folders {
    if f.parent == src.parent && f.order >= insert_order {
      f.order += 1;
    }
  }
  let mut id_map: HashMap<String, String> = HashMap::new();
  let root_id = clone_node(lib, &src, src.parent.clone(), insert_order, &root_name, &mut id_map, out)?;
  out.folder_ids.push(root_id);

  // 子树 Placement（按全局 items 序 = 各段内相对序保持）
  let subtree: Vec<Placement> = lib.folders.items.iter().filter(|p| id_map.contains_key(&p.folder)).cloned().collect();
  // 第一趟：origin 资产物理复制（一资产仅一条 origin，天然去重；克隆资产名 / hash 保留）
  let mut cloned_assets: HashMap<String, String> = HashMap::new();
  for p in subtree.iter().filter(|p| p.origin) {
    let new_asset = clone_asset(lib, &p.asset, out)?;
    cloned_assets.insert(p.asset.clone(), new_asset);
  }
  // 第二趟：Placement 重建——origin 指向新副本；链接指向子树内新副本，子树外资产原样指向
  let mut anchor = HashMap::new();
  for p in &subtree {
    let target_asset = if p.origin {
      cloned_assets[&p.asset].clone()
    } else {
      cloned_assets.get(&p.asset).cloned().unwrap_or_else(|| p.asset.clone())
    };
    let new_folder = id_map[&p.folder].clone();
    ops::add_placement(lib, target_asset, new_folder, p.name.clone(), p.origin, &mut anchor);
  }
  Ok(())
}

/// 递归克隆节点（字段全保留，parent 重挂；子级按 order 顺克、order 重写连续）；
/// 档案节点顺带深拷贝 archives.json 记录
fn clone_node(
  lib: &mut Library,
  src: &Folder,
  new_parent: Option<String>,
  order: i32,
  name: &str,
  id_map: &mut HashMap<String, String>,
  out: &mut CloneOutcome,
) -> Result<String, String> {
  let id = ops::new_folder_id();
  let folder = Folder {
    id: id.clone(),
    name: name.to_string(),
    parent: new_parent,
    block: src.block.clone(),
    icon: src.icon.clone(),
    kind: src.kind.clone(),
    color: src.color.clone(),
    groups: src.groups.clone(), // 分组规则随迁（3.5：分组是文件夹整理配置，克隆保留）
    order,
  };
  lib.folders.folders.push(folder.clone());
  lib.index.folders.insert(id.clone(), folder);
  out.new_node_ids.push(id.clone());
  out.folder_count += 1;
  if src.kind.as_deref() == Some(ARCHIVE_KIND) {
    if let Some(rec) = lib.archives.archives.iter().find(|a| a.folder_id == src.id).cloned() {
      lib.archives.archives.push(ArchiveData { folder_id: id.clone(), ..rec });
      out.new_archive_ids.push(id.clone());
    }
  }
  id_map.insert(src.id.clone(), id.clone());
  let mut children: Vec<Folder> =
    lib.folders.folders.iter().filter(|f| f.parent.as_deref() == Some(src.id.as_str())).cloned().collect();
  children.sort_by_key(|f| f.order);
  for (i, c) in children.iter().enumerate() {
    clone_node(lib, c, Some(id.clone()), i as i32, &c.name, id_map, out)?;
  }
  Ok(id)
}

/// 物理复制资产：info 目录整体拷贝 → metadata 改 id / 时间刷新写盘 → 索引 insert（duplicate 同管线，
/// 差异 = 克隆保留源资产名 metadata.name 与显示名，不做唯一化）
fn clone_asset(lib: &mut Library, src_id: &str, out: &mut CloneOutcome) -> Result<String, String> {
  if !lib.index.assets.contains_key(src_id) {
    return Err("资源数据异常：缺少元数据".to_string());
  }
  let new_id = new_asset_id();
  let old_info = lib.info_dir(src_id);
  let new_info = lib.info_dir(&new_id);
  super::duplicate::copy_dir(&old_info, &new_info)?;
  let meta_path = new_info.join("metadata.json");
  let mut meta: AssetMeta = read_json(&meta_path).map_err(|e| e.to_string())?;
  meta.id = new_id.clone();
  let stamp = now();
  meta.imported_at = stamp.clone();
  meta.modified_at = stamp;
  write_json(&meta_path, &meta).map_err(|e| e.to_string())?;
  lib.index.assets.insert(new_id.clone(), meta); // 先落盘再同步内存（规范第 7 条）
  out.copies.push((old_info, new_info));
  out.new_asset_ids.push(new_id.clone());
  out.asset_count += 1;
  Ok(new_id)
}

/// 失败回滚：清索引 / Placement / 节点 / 档案记录 + 删已复制目录（撤销事务由命令层 cancel）
fn rollback(lib: &mut Library, out: &CloneOutcome) {
  for id in &out.new_asset_ids {
    lib.index.assets.remove(id);
  }
  let node_set: HashSet<&str> = out.new_node_ids.iter().map(String::as_str).collect();
  lib.folders.items.retain(|p| !node_set.contains(p.folder.as_str()));
  lib.folders.folders.retain(|f| !node_set.contains(f.id.as_str()));
  lib.archives.archives.retain(|a| !out.new_archive_ids.contains(&a.folder_id));
  sync_folder_index(lib);
  lib.index.rebuild_placements(&lib.folders.items);
  for (_, new_dir) in &out.copies {
    if let Err(e) = std::fs::remove_dir_all(new_dir) {
      log::error!("克隆回滚清理失败（{}）：{e}", new_dir.display());
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library::{ARCHIVES_FILE, FOLDERS_FILE};
  use crate::core::models::{ArchivesFile, FoldersFile};
  use crate::services::import::{PendingFile, Source};
  use std::fs;

  struct Harness {
    dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { dir, lib }
  }

  fn add_folder(lib: &mut Library, id: &str, parent: Option<&str>, order: i32) {
    let folder = Folder { id: id.into(), name: id.into(), parent: parent.map(|p| p.to_string()), block: None, icon: None, kind: None, color: None, groups: None, order };
    lib.folders.folders.push(folder.clone());
    lib.index.folders.insert(id.into(), folder);
  }

  fn add_asset(h: &mut Harness, id: &str, file: &str, folder: &str) {
    let src = h.dir.path().join(file);
    fs::write(&src, b"fake").unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = ops::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.into(), meta);
    ops::add_placement(&mut h.lib, id.into(), folder.into(), file.into(), true, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
  }

  /// 结构克隆：子树节点 / 资产去重复制 / 链接指向新副本 / 根名"xx 副本"紧随源后 / 落盘一致
  #[test]
  fn clone_folder_deep_copies_structure_and_assets() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-TOP", None, 0);
    add_folder(&mut h.lib, "F-SUB", Some("F-TOP"), 0);
    add_folder(&mut h.lib, "F-NEIGHBOR", None, 1);
    add_asset(&mut h, "AN-A", "a.png", "F-SUB");
    // 同资产在 F-TOP 内一条链接 → 克隆后指向子树内新副本
    ops::add_placement(&mut h.lib, "AN-A".into(), "F-TOP".into(), "a.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();

    let out = clone_folders(&mut h.lib, &["F-TOP".into()]).unwrap();
    assert_eq!((out.folder_count, out.asset_count), (2, 1), "2 节点 + 1 资产副本");
    let new_root = &out.folder_ids[0];
    let root = &h.lib.index.folders[new_root];
    assert_eq!(root.name, "F-TOP 副本");
    assert_eq!(root.order, 1, "紧随源之后");
    assert_eq!(h.lib.index.folders["F-NEIGHBOR"].order, 2, "后续兄弟顺移");
    let new_sub = h.lib.folders.folders.iter().find(|f| f.parent.as_deref() == Some(new_root.as_str())).unwrap();
    assert_eq!(new_sub.name, "F-SUB", "子节点同名");
    // 资产去重：origin 在新子夹、链接指向新资产
    let new_asset = h
      .lib
      .folders
      .items
      .iter()
      .find(|p| p.folder == new_sub.id && p.origin)
      .map(|p| p.asset.clone())
      .unwrap();
    assert_ne!(new_asset, "AN-A");
    assert!(h.lib.info_dir(&new_asset).join("a.png").is_file(), "info 物理复制");
    assert_eq!(h.lib.index.assets[&new_asset].hash, "sha256:t", "hash 保留");
    let link = h.lib.folders.items.iter().find(|p| p.folder == *new_root && !p.origin).unwrap();
    assert_eq!(link.asset, new_asset, "子树内链接指向新副本");
    // 落盘一致
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert!(disk.folders.iter().any(|f| f.id == *new_root), "folders.json 落盘");
  }

  /// 链接指向子树外资产：克隆保留原指向（不越界复制别夹本体）；根名二次克隆唯一化
  #[test]
  fn clone_folder_keeps_outside_links_and_unique_names() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-TOP", None, 0);
    add_folder(&mut h.lib, "F-OUT", None, 1);
    add_asset(&mut h, "AN-B", "b.png", "F-OUT"); // origin 在子树外
    ops::add_placement(&mut h.lib, "AN-B".into(), "F-TOP".into(), "b.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();

    let out = clone_folders(&mut h.lib, &["F-TOP".into()]).unwrap();
    assert_eq!(out.asset_count, 0, "子树外资产不复制");
    let link = h.lib.folders.items.iter().find(|p| p.folder == out.folder_ids[0] && !p.origin).unwrap();
    assert_eq!(link.asset, "AN-B", "链接原样指向子树外资产");

    let out2 = clone_folders(&mut h.lib, &["F-TOP".into()]).unwrap();
    let name2 = &h.lib.index.folders[&out2.folder_ids[0]].name;
    assert_eq!(name2, "F-TOP 副本-1", "同级唯一化");
    // 陈旧 id 跳过
    let out3 = clone_folders(&mut h.lib, &["F-幽灵".into()]).unwrap();
    assert!(out3.folder_ids.is_empty());
  }

  /// 档案子节点：kind 保留 + archives.json 记录深拷贝（badge 随迁、folderId 换新）并落盘
  #[test]
  fn clone_folder_copies_archive_record() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-TOP", None, 0);
    add_folder(&mut h.lib, "F-ARC", Some("F-TOP"), 0);
    h.lib.folders.folders.iter_mut().find(|f| f.id == "F-ARC").unwrap().kind = Some(ARCHIVE_KIND.into());
    h.lib.index.folders.get_mut("F-ARC").unwrap().kind = Some(ARCHIVE_KIND.into());
    let mut rec = ArchiveData::new("F-ARC");
    rec.badge = Some(crate::core::models::Badge { text: "主角".into(), color: "red".into() });
    h.lib.archives.archives.push(rec);

    let out = clone_folders(&mut h.lib, &["F-TOP".into()]).unwrap();
    let new_arc = h.lib.folders.folders.iter().find(|f| f.parent.as_deref() == Some(out.folder_ids[0].as_str())).unwrap();
    assert_eq!(new_arc.kind.as_deref(), Some(ARCHIVE_KIND), "kind 保留");
    let new_rec = h.lib.archives.archives.iter().find(|a| a.folder_id == new_arc.id).unwrap();
    assert_eq!(new_rec.badge.as_ref().unwrap().text, "主角", "档案记录深拷贝");
    let adisk: ArchivesFile = read_json(&h.lib.root.join(ARCHIVES_FILE)).unwrap();
    assert!(adisk.archives.iter().any(|a| a.folder_id == new_arc.id), "archives.json 落盘");
    assert!(adisk.archives.iter().any(|a| a.folder_id == "F-ARC"), "原记录不动");
  }
}
