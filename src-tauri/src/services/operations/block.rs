//! 区块操作（2026-08-14 区块子系统，PRD 02 §01）：区块管理弹窗草稿式一次提交（对齐 30 稿
//! 确认/取消）与侧栏拖拽排序即时提交。命令层（commands::operations）持锁并做撤销三步接入。
//! 删除非空区块口径（2026-08-14 用户定）：区块内顶级文件夹带子树移入「目录」区块根级
//! （Folder.block 置 None），不摧毁用户层级。

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::core::library::{self, Library};
use crate::core::models::{Block, DEFAULT_BLOCK_ID};

use super::sync_folder_index;

/// 区块草稿行（区块管理弹窗确认时一次提交）：id = None 为新建（后端发 id），Some = 既有区块改名/换图标
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockInput {
  #[serde(default)]
  pub id: Option<String>,
  pub name: String,
  #[serde(default)]
  pub icon: Option<String>,
}

/// 删除非空区块的确认条目（needConfirm 时随报告返回，前端弹 26 稿删除区块警告）
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockDeleteConflict {
  pub name: String,
  pub folder_count: usize,
}

/// 提交结果：needConfirm = true 时未动任何数据，blocks 为涉及的非空待删区块（对齐 DeleteReport 两段式）
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyBlockReport {
  pub need_confirm: bool,
  #[serde(default)]
  pub blocks: Vec<BlockDeleteConflict>,
}

/// 生成区块 ID：`B` + 时间戳（yyMMddHHmm，时间有序）+ 3 位随机字符（对齐 new_folder_id 风格）
fn new_block_id() -> String {
  const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
  use rand::Rng;
  let mut rng = rand::thread_rng();
  let suffix: String = (0..3).map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char).collect();
  format!("B{}-{suffix}", chrono::Local::now().format("%y%m%d%H%M"))
}

/// 草稿式一次提交：blocks = 最终保留的全部区块（数组序 = 显示顺序，order 连续重写 0..n，
/// 对齐 reorder_folders 做法）；deleted = 待删区块 id。
/// deleted 含非空区块（存在 Folder.block 指向它的顶级文件夹）且 confirmed = false 时返回
/// needConfirm 报告、不执行任何写入；confirmed = true 时这些区块的顶级文件夹 block 置 None
/// （移入「目录」）再删。默认区块「目录」禁删可改名；新建区块名不做唯一化（库允许同名，
/// 2026-08-14 用户定同类口径）
pub fn apply_block_changes(
  lib: &mut Library,
  blocks: Vec<BlockInput>,
  deleted: Vec<String>,
  confirmed: bool,
) -> Result<ApplyBlockReport, String> {
  if deleted.iter().any(|id| id == DEFAULT_BLOCK_ID) {
    return Err("默认区块「目录」不可删除".to_string());
  }
  // 草稿自相矛盾护栏：同一 id 既在保留列表又在删除列表
  for input in &blocks {
    if let Some(id) = &input.id {
      if deleted.contains(id) {
        return Err(format!("区块 {id} 既在保留列表又在删除列表（草稿状态矛盾）"));
      }
    }
  }
  // 保留列表校验：名称非空（trim 后）；既有区块 id 不重复且必须存在；
  // 现有区块缺一不可（既不在保留列表也不在删除列表 = 草稿缺行，防静默丢区块）
  let mut seen = HashSet::new();
  for input in &blocks {
    if input.name.trim().is_empty() {
      return Err("区块名称不能留空".to_string());
    }
    if let Some(id) = &input.id {
      if !seen.insert(id.clone()) {
        return Err("保留列表存在重复区块".to_string());
      }
      if !lib.folders.blocks.iter().any(|b| &b.id == id) {
        return Err(format!("区块 {id} 不存在，可能已被删除"));
      }
    }
  }
  for b in &lib.folders.blocks {
    let kept = blocks.iter().any(|i| i.id.as_deref() == Some(b.id.as_str()));
    if !kept && !deleted.contains(&b.id) {
      return Err(format!("区块「{}」既不在保留列表也不在删除列表", b.name));
    }
  }
  // 非空待删区块（有顶级文件夹引用）
  let mut conflicts: Vec<BlockDeleteConflict> = Vec::new();
  for id in &deleted {
    let folder_count =
      lib.folders.folders.iter().filter(|f| f.parent.is_none() && f.block.as_deref() == Some(id.as_str())).count();
    if folder_count > 0 {
      let name = lib.folders.blocks.iter().find(|b| &b.id == id).map(|b| b.name.clone()).unwrap_or_default();
      conflicts.push(BlockDeleteConflict { name, folder_count });
    }
  }
  if !confirmed && !conflicts.is_empty() {
    return Ok(ApplyBlockReport { need_confirm: true, blocks: conflicts });
  }
  // 确认删除：引用待删区块的文件夹移入「目录」（Folder.block 置 None；只动引用，子树随父级不动）
  if !deleted.is_empty() {
    for f in &mut lib.folders.folders {
      let clear = matches!(&f.block, Some(b) if deleted.contains(b));
      if clear {
        f.block = None;
      }
    }
  }
  // 重写区块表：数组序 = 最终顺序；既有区块改名/换图标，新建后端发 id
  let mut next: Vec<Block> = Vec::with_capacity(blocks.len());
  for (i, input) in blocks.into_iter().enumerate() {
    let id = input.id.unwrap_or_else(new_block_id);
    next.push(Block { id, name: input.name.trim().to_string(), icon: input.icon, order: i as i32 });
  }
  lib.folders.blocks = next;
  sync_folder_index(lib); // Folder.block 改动同步进索引
  library::save_folders(lib).map_err(|e| e.to_string())?;
  log::info!("区块管理提交：保留 {} 个，删除 {} 个", lib.folders.blocks.len(), deleted.len());
  Ok(ApplyBlockReport { need_confirm: false, blocks: Vec::new() })
}

/// 侧栏拖拽排序即时提交：ids 必须恰好覆盖现有全部区块（缺 / 多 / 重复 / 不存在均报错），
/// 按给定顺序重写 order 为连续 0..n
pub fn reorder_blocks(lib: &mut Library, ids: Vec<String>) -> Result<(), String> {
  let mut seen = HashSet::new();
  for id in &ids {
    if !seen.insert(id) {
      return Err("排序列表存在重复区块".to_string());
    }
    if !lib.folders.blocks.iter().any(|b| &b.id == id) {
      return Err(format!("区块 {id} 不存在，可能已被删除"));
    }
  }
  if ids.len() != lib.folders.blocks.len() {
    return Err("排序列表须恰好覆盖现有全部区块".to_string());
  }
  for (i, id) in ids.iter().enumerate() {
    if let Some(b) = lib.folders.blocks.iter_mut().find(|b| &b.id == id) {
      b.order = i as i32;
    }
  }
  library::save_folders(lib).map_err(|e| e.to_string())?;
  log::info!("排序区块：{ids:?}");
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library::FOLDERS_FILE;
  use crate::core::models::{Folder, FoldersFile};

  struct Harness {
    _dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { _dir: dir, lib }
  }

  fn add_folder(lib: &mut Library, id: &str, block: Option<&str>) {
    let folder = Folder {
      id: id.into(),
      name: id.into(),
      parent: None,
      block: block.map(|s| s.into()),
      icon: None,
      order: lib.folders.folders.len() as i32,
    };
    lib.folders.folders.push(folder.clone());
    lib.index.folders.insert(id.into(), folder);
  }

  fn input(id: Option<&str>, name: &str, icon: Option<&str>) -> BlockInput {
    BlockInput { id: id.map(|s| s.into()), name: name.into(), icon: icon.map(|s| s.into()) }
  }

  /// 测试库初始即带默认区块「目录」（FoldersFile::new 口径），保留列表首行恒为它
  fn dir_input() -> BlockInput {
    input(Some(DEFAULT_BLOCK_ID), "目录", None)
  }

  /// 全新建两个区块：后端发 id（B 前缀），数组序 = order 连续重写
  #[test]
  fn apply_creates_new_blocks_in_array_order() {
    let mut h = make_lib();
    let rep = apply_block_changes(
      &mut h.lib,
      vec![dir_input(), input(None, "角色", Some("icon-role")), input(None, "场景", None)],
      vec![],
      false,
    )
    .unwrap();
    assert!(!rep.need_confirm);
    let blocks = &h.lib.folders.blocks;
    assert_eq!(blocks.len(), 3);
    assert!(blocks[1].id.starts_with('B'), "新建区块后端发 id");
    assert_eq!(
      blocks.iter().map(|b| (b.name.as_str(), b.order)).collect::<Vec<_>>(),
      vec![("目录", 0), ("角色", 1), ("场景", 2)],
      "数组序 = order"
    );
    assert_eq!(blocks[1].icon.as_deref(), Some("icon-role"));
    assert_eq!(blocks[2].icon, None);
    // 落盘验证
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.blocks.len(), 3);
  }

  /// 改名 / 换图标 / 排序（数组序即最终 order）；默认区块可改名
  #[test]
  fn apply_renames_reicons_and_reorders_existing() {
    let mut h = make_lib();
    apply_block_changes(&mut h.lib, vec![dir_input(), input(None, "角色", None)], vec![], false).unwrap();
    let new_id = h.lib.folders.blocks[1].id.clone();

    let rep = apply_block_changes(
      &mut h.lib,
      vec![input(Some(&new_id), "角色动画", Some("icon-role")), input(Some(DEFAULT_BLOCK_ID), "素材目录", None)],
      vec![],
      false,
    )
    .unwrap();
    assert!(!rep.need_confirm);
    let blocks = &h.lib.folders.blocks;
    assert_eq!(blocks.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(), vec![new_id.as_str(), DEFAULT_BLOCK_ID], "排序生效");
    assert_eq!((blocks[0].name.as_str(), blocks[0].order), ("角色动画", 0));
    assert_eq!(blocks[0].icon.as_deref(), Some("icon-role"), "换图标");
    assert_eq!((blocks[1].name.as_str(), blocks[1].order), ("素材目录", 1), "默认区块可改名");
  }

  /// 删空区块直接执行；删非空区块 confirmed = false → needConfirm 不动任何数据
  #[test]
  fn apply_delete_empty_ok_and_non_empty_needs_confirm() {
    let mut h = make_lib();
    apply_block_changes(&mut h.lib, vec![dir_input(), input(None, "角色", None), input(None, "场景", None)], vec![], false).unwrap();
    let role = h.lib.folders.blocks[1].id.clone();
    let scene = h.lib.folders.blocks[2].id.clone();
    add_folder(&mut h.lib, "F01", Some(&role)); // 角色区块非空；场景区块空

    let rep = apply_block_changes(&mut h.lib, vec![dir_input()], vec![role.clone(), scene.clone()], false).unwrap();
    assert!(rep.need_confirm);
    assert_eq!(rep.blocks, vec![BlockDeleteConflict { name: "角色".into(), folder_count: 1 }], "只报非空区块");
    // 未执行任何写入
    assert_eq!(h.lib.folders.blocks.len(), 3);
    assert_eq!(h.lib.index.folders["F01"].block.as_deref(), Some(role.as_str()));
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.blocks.len(), 3, "盘上同样未动");
  }

  /// confirmed = true：非空区块的顶级文件夹 block 置 None（移入「目录」，子树不动）后再删
  #[test]
  fn apply_confirmed_delete_moves_folders_back_to_dir() {
    let mut h = make_lib();
    apply_block_changes(&mut h.lib, vec![dir_input(), input(None, "角色", None)], vec![], false).unwrap();
    let role = h.lib.folders.blocks[1].id.clone();
    add_folder(&mut h.lib, "F01", Some(&role));
    // 子文件夹不持区块引用，确认删除不受影响
    let child = Folder { id: "F01-1".into(), name: "子".into(), parent: Some("F01".into()), block: None, icon: None, order: 0 };
    h.lib.folders.folders.push(child.clone());
    h.lib.index.folders.insert(child.id.clone(), child);

    let rep = apply_block_changes(&mut h.lib, vec![dir_input()], vec![role], true).unwrap();
    assert!(!rep.need_confirm);
    assert_eq!(h.lib.folders.blocks.len(), 1, "区块已删");
    assert_eq!(h.lib.index.folders["F01"].block, None, "文件夹移入「目录」（索引同步）");
    assert_eq!(h.lib.index.folders["F01-1"].parent.as_deref(), Some("F01"), "子树层级不动");
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.folders.iter().find(|f| f.id == "F01").unwrap().block, None, "落盘验证");
  }

  /// 护栏：删默认区块「目录」报错；deleted 与 blocks 撞 id 报错；现有区块缺行报错；不存在区块报错
  #[test]
  fn apply_rejects_invalid_drafts() {
    let mut h = make_lib();
    apply_block_changes(&mut h.lib, vec![dir_input(), input(None, "角色", None)], vec![], false).unwrap();
    let role = h.lib.folders.blocks[1].id.clone();

    assert!(apply_block_changes(&mut h.lib, vec![], vec![DEFAULT_BLOCK_ID.into()], false).is_err(), "默认区块禁删");
    assert!(
      apply_block_changes(&mut h.lib, vec![dir_input(), input(Some(&role), "角色", None)], vec![role.clone()], false).is_err(),
      "deleted 与 blocks 撞 id"
    );
    assert!(apply_block_changes(&mut h.lib, vec![dir_input()], vec![], false).is_err(), "现有区块缺行（既不留也不删）");
    assert!(
      apply_block_changes(&mut h.lib, vec![dir_input(), input(Some("B-不存在"), "x", None)], vec![role.clone()], false).is_err(),
      "保留列表引用不存在区块"
    );
    assert!(apply_block_changes(&mut h.lib, vec![dir_input(), input(None, "  ", None)], vec![role], false).is_err(), "空名拒绝");
    assert_eq!(h.lib.folders.blocks.len(), 2, "报错均不动数据");
  }

  /// 同名区块允许（2026-08-14 用户定同类口径：不做唯一化）
  #[test]
  fn apply_allows_duplicate_names() {
    let mut h = make_lib();
    apply_block_changes(&mut h.lib, vec![dir_input(), input(None, "角色", None), input(None, "角色", None)], vec![], false)
      .unwrap();
    assert_eq!(h.lib.folders.blocks.len(), 3);
    assert_eq!(h.lib.folders.blocks[1].name, h.lib.folders.blocks[2].name);
  }

  /// 排序：ids 覆盖全集按给定顺序重写 order；不覆盖全集 / 重复 / 不存在报错
  #[test]
  fn reorder_blocks_rewrites_orders() {
    let mut h = make_lib();
    apply_block_changes(&mut h.lib, vec![dir_input(), input(None, "角色", None), input(None, "场景", None)], vec![], false)
      .unwrap();
    let role = h.lib.folders.blocks[1].id.clone();
    let scene = h.lib.folders.blocks[2].id.clone();

    reorder_blocks(&mut h.lib, vec![scene.clone(), DEFAULT_BLOCK_ID.into(), role.clone()]).unwrap();
    fn order_of(lib: &Library, id: &str) -> i32 {
      lib.folders.blocks.iter().find(|b| b.id == id).unwrap().order
    }
    assert_eq!((order_of(&h.lib, &scene), order_of(&h.lib, DEFAULT_BLOCK_ID), order_of(&h.lib, &role)), (0, 1, 2));
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.blocks.iter().find(|b| b.id == scene).unwrap().order, 0, "落盘验证");

    // 缺行 / 多 id / 重复 / 不存在 → 报错且不动数据
    assert!(reorder_blocks(&mut h.lib, vec![DEFAULT_BLOCK_ID.into(), role.clone()]).is_err());
    assert!(reorder_blocks(&mut h.lib, vec![DEFAULT_BLOCK_ID.into(), role.clone(), scene.clone(), "B-多".into()]).is_err());
    assert!(reorder_blocks(&mut h.lib, vec![DEFAULT_BLOCK_ID.into(), role.clone(), role.clone()]).is_err());
    assert!(reorder_blocks(&mut h.lib, vec![DEFAULT_BLOCK_ID.into(), role.clone(), "B-不存在".into()]).is_err());
    assert_eq!(order_of(&h.lib, &scene), 0, "报错不动数据");
  }
}
