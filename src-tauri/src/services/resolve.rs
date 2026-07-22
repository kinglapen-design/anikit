//! 冲突决策服务（PRD 10_重复添加提示）：导入挂起的 hash 冲突，前端逐项弹窗收集
//! 用户决策后一次性提交执行。单项失败不阻塞（中文原因进 failed），
//! 全部处理完重建 Placement 索引 + folders.json 一次落盘。

use std::collections::HashSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::library::{self, Library};
use crate::services::import as ops;
use crate::services::import::{FailedItem, Source, ROOT_FOLDER_ID};

/// 用户决策（serde 小驼峰："useExisting" / "copy" / "link" / "skip"）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResolveAction {
  /// 用已存入文件导入：不导入新文件，库内已有资源的 origin Placement 移到目标文件夹
  UseExisting,
  /// 保留两个：源文件走标准建档（新 ID 新实体），继承原导入的 source（复制 / 移动）
  Copy,
  /// 关联文件夹：在目标文件夹为已有资源建链接 Placement
  Link,
  /// 跳过：什么都不做，源文件保留（即使原导入选了剪切）
  Skip,
}

/// 单项决策（前端弹窗结果；linkName 仅 action = "link" 时有意义）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictResolution {
  pub source_path: PathBuf,
  pub existing_asset_id: String,
  pub target_folder: String,
  pub action: ResolveAction,
  #[serde(default)]
  pub link_name: Option<String>,
}

/// 决策执行汇总（skip 也计入 resolved——用户已决策、无需动作）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveReport {
  pub resolved: u32,
  pub failed: Vec<FailedItem>,
}

/// 逐项执行决策：skip 直接过；copy 复用建档原语（继承 source）；useExisting / link 为轻操作；
/// 全部处理完重建 Placement 索引 + folders.json 一次落盘（有改动才写）。
pub fn resolve_conflicts(lib: &mut Library, items: &[ConflictResolution], source: Source) -> ResolveReport {
  let mut report = ResolveReport { resolved: 0, failed: Vec::new() };
  let mut dirty = false;
  for item in items {
    match apply_one(lib, item, source) {
      Ok(changed) => {
        dirty |= changed;
        report.resolved += 1;
      }
      Err(reason) => {
        report.failed.push(FailedItem { path: item.source_path.to_string_lossy().into_owned(), reason });
      }
    }
  }
  if dirty {
    lib.index.rebuild_placements(&lib.folders.items);
    if let Err(e) = library::save_folders(lib) {
      log::error!("folders.json 落盘失败：{e}");
    }
  }
  log::info!("冲突决策完成：成功 {}、失败 {}", report.resolved, report.failed.len());
  report
}

/// 执行单项决策；返回 Ok(true) 表示 folders.json 有改动
fn apply_one(lib: &mut Library, item: &ConflictResolution, source: Source) -> Result<bool, String> {
  match item.action {
    ResolveAction::Skip => Ok(false),
    ResolveAction::Link => link(lib, item).map(|_| true),
    ResolveAction::UseExisting => use_existing(lib, item).map(|_| true),
    ResolveAction::Copy => copy_new(lib, item, source).map(|_| true),
  }
}

/// 目标文件夹校验（root 保留 ID 放行）
fn check_target(lib: &Library, target_folder: &str) -> Result<(), String> {
  if target_folder != ROOT_FOLDER_ID && !lib.index.folders.contains_key(target_folder) {
    return Err("目标文件夹不存在，可能已被删除".to_string());
  }
  Ok(())
}

/// 库内资源校验
fn check_asset(lib: &Library, asset_id: &str) -> Result<(), String> {
  if !lib.index.assets.contains_key(asset_id) {
    return Err("库内资源不存在，可能已被删除".to_string());
  }
  Ok(())
}

/// link：目标文件夹建链接 Placement（显示名 = 用户输入名或源文件原名，unique_name），源文件不动
fn link(lib: &mut Library, item: &ConflictResolution) -> Result<(), String> {
  check_target(lib, &item.target_folder)?;
  check_asset(lib, &item.existing_asset_id)?;
  let name = item
    .link_name
    .clone()
    .filter(|n| !n.trim().is_empty())
    .unwrap_or_else(|| ops::file_name(&item.source_path));
  ops::add_placement(lib, item.existing_asset_id.clone(), item.target_folder.clone(), name, false);
  Ok(())
}

/// useExisting：已有资源的 origin Placement 移到目标文件夹（原位置不再保留）；
/// 目标文件夹已有该资源链接 Placement 则去重（删链接，保留移过来的 origin）；源文件不动
fn use_existing(lib: &mut Library, item: &ConflictResolution) -> Result<(), String> {
  check_target(lib, &item.target_folder)?;
  check_asset(lib, &item.existing_asset_id)?;
  // 去重：删掉目标文件夹里该资源的链接 Placement（先删再定位 origin，避免索引位移）
  lib.folders.items.retain(|p| {
    !(p.asset == item.existing_asset_id && p.folder == item.target_folder && !p.origin)
  });
  let origin_pos = lib
    .folders
    .items
    .iter()
    .position(|p| p.asset == item.existing_asset_id && p.origin)
    .ok_or_else(|| "资源数据异常：缺少原始位置记录".to_string())?;
  // 显示名唯一化（排除 origin 自身），order 取目标文件夹末尾
  let existing: HashSet<String> = lib
    .folders
    .items
    .iter()
    .enumerate()
    .filter(|(i, p)| *i != origin_pos && p.folder == item.target_folder)
    .map(|(_, p)| p.name.clone())
    .collect();
  let origin = &mut lib.folders.items[origin_pos];
  origin.name = library::unique_name(&existing, &origin.name.clone());
  origin.folder = item.target_folder.clone();
  origin.order = existing.len() as i32;
  Ok(())
}

/// copy：源文件走标准建档（新 ID 新实体 + origin Placement），继承原导入的 source
fn copy_new(lib: &mut Library, item: &ConflictResolution, source: Source) -> Result<(), String> {
  check_target(lib, &item.target_folder)?;
  let pending = ops::PendingFile { path: item.source_path.clone(), chain: Vec::new() };
  let hash = ops::sha256_file(&pending.path).map_err(|e| format!("文件读取失败：{e}"))?;
  let root = lib.root.clone();
  let asset = ops::build_asset(&root, &pending, &hash, source)?;
  let name = ops::file_name(&pending.path);
  ops::add_placement(lib, asset.id.clone(), item.target_folder.clone(), name, true);
  lib.index.assets.insert(asset.id.clone(), asset);
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library::ASSTES_DIR;
  use crate::core::models::{Folder, Placement};
  use crate::services::import::{ImportOptions, Structure};
  use crate::services::import_task::run_import;
  use std::sync::atomic::AtomicBool;
  use std::sync::Mutex;

  struct Harness {
    _dir: tempfile::TempDir,
    lib: Mutex<Option<Library>>,
    root: PathBuf,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("测试库");
    let lib = library::create(&root).unwrap();
    Harness { _dir: dir, lib: Mutex::new(Some(lib)), root }
  }

  fn with_lib<R>(h: &Harness, f: impl FnOnce(&mut Library) -> R) -> R {
    f(h.lib.lock().unwrap().as_mut().unwrap())
  }

  /// 导入 paths 制造冲突：先入库 seed（内容按 seed_name 唯一，避免撞上前用例的资源），
  /// 再导入同内容文件 → 返回冲突项
  fn make_conflict(h: &Harness, seed_name: &str, dup_name: &str, target: Option<String>) -> ops::ConflictItem {
    let content = format!("内容:{seed_name}");
    let seed = h._dir.path().join(seed_name);
    std::fs::write(&seed, &content).unwrap();
    let seed_out = import(h, vec![seed], None);
    assert_eq!(seed_out.imported, 1, "种子应正常入库：{:?}", seed_out.failed);
    let dup = h._dir.path().join(dup_name);
    std::fs::write(&dup, &content).unwrap();
    let outcome = import(h, vec![dup], target);
    assert_eq!(outcome.conflicts.len(), 1, "应产生一个冲突");
    outcome.conflicts[0].clone()
  }

  fn import(h: &Harness, paths: Vec<PathBuf>, target: Option<String>) -> ops::ImportOutcome {
    let opts = ImportOptions { structure: Structure::Flatten, source: Source::Copy, target_folder: target, paths, imported_from: None };
    run_import(&h.lib, h.root.clone(), opts, |_| {}, &AtomicBool::new(false))
  }

  /// 手动加文件夹（返回 ID）
  fn add_folder(h: &Harness, id: &str, name: &str) -> String {
    with_lib(h, |l| {
      let folder = Folder { id: id.into(), name: name.into(), parent: None, order: 0 };
      l.folders.folders.push(folder.clone());
      l.index.folders.insert(id.into(), folder);
      id.into()
    })
  }

  fn resolve(h: &Harness, items: Vec<ConflictResolution>, source: Source) -> ResolveReport {
    with_lib(h, |l| resolve_conflicts(l, &items, source))
  }

  fn to_resolution(c: &ops::ConflictItem, action: ResolveAction, link_name: Option<&str>) -> ConflictResolution {
    ConflictResolution {
      source_path: c.source_path.clone(),
      existing_asset_id: c.existing_asset_id.clone(),
      target_folder: c.target_folder.clone(),
      action,
      link_name: link_name.map(|s| s.into()),
    }
  }

  fn placements_of(h: &Harness, asset_id: &str) -> Vec<Placement> {
    with_lib(h, |l| l.folders.items.iter().filter(|p| p.asset == asset_id).cloned().collect())
  }

  #[test]
  fn use_existing_moves_origin_and_dedups_link() {
    let h = make_lib();
    let folder_id = add_folder(&h, "F01", "人物");
    let conflict = make_conflict(&h, "原图.png", "副本.png", Some(folder_id.clone()));
    let asset_id = conflict.existing_asset_id.clone();
    // 预置：目标文件夹已有一条该资源的链接 Placement（应被去重删掉）
    with_lib(&h, |l| {
      ops::add_placement(l, asset_id.clone(), folder_id.clone(), "快捷.png".into(), false);
    });

    let report = resolve(&h, vec![to_resolution(&conflict, ResolveAction::UseExisting, None)], Source::Copy);
    assert_eq!((report.resolved, report.failed.len()), (1, 0));

    let placements = placements_of(&h, &asset_id);
    // origin 移到目标文件夹且唯一（链接已去重），根目录不再保留
    assert_eq!(placements.len(), 1);
    assert!(placements[0].origin);
    assert_eq!(placements[0].folder, folder_id);
    // 源文件保留不动
    assert!(conflict.source_path.exists());
    // 索引反查同步
    assert_eq!(with_lib(&h, |l| l.index.placements_by_folder["F01"].len()), 1);
  }

  #[test]
  fn copy_action_imports_new_entity_with_inherited_source() {
    let h = make_lib();
    let folder_id = add_folder(&h, "F01", "人物");
    let conflict = make_conflict(&h, "原图.png", "副本.png", Some(folder_id.clone()));
    let old_asset = conflict.existing_asset_id.clone();

    // 继承 copy：源文件保留，实体两份
    let report = resolve(&h, vec![to_resolution(&conflict, ResolveAction::Copy, None)], Source::Copy);
    assert_eq!((report.resolved, report.failed.len()), (1, 0));
    assert!(conflict.source_path.exists(), "copy 模式源文件应保留");
    assert_eq!(with_lib(&h, |l| l.index.assets.len()), 2);
    assert_eq!(std::fs::read_dir(h.root.join(ASSTES_DIR)).unwrap().count(), 2);

    // 继承 move：源文件消失（再造一个冲突）
    let conflict2 = make_conflict(&h, "基准2.png", "副本2.png", None);
    let report2 = resolve(&h, vec![to_resolution(&conflict2, ResolveAction::Copy, None)], Source::Move);
    assert_eq!(report2.resolved, 1);
    assert!(!conflict2.source_path.exists(), "move 模式源文件应移入库");
    // 旧资源的 origin Placement 不受新实体影响
    assert_eq!(placements_of(&h, &old_asset).len(), 1);
  }

  #[test]
  fn link_action_honors_custom_and_default_names() {
    let h = make_lib();
    let c1 = make_conflict(&h, "原图.png", "副本.png", None);
    let c2 = make_conflict(&h, "基准2.png", "走路.fla", None);

    let report = resolve(
      &h,
      vec![
        to_resolution(&c1, ResolveAction::Link, Some("自定义名.png")),
        to_resolution(&c2, ResolveAction::Link, None),
      ],
      Source::Copy,
    );
    assert_eq!((report.resolved, report.failed.len()), (2, 0));

    let names: Vec<String> =
      placements_of(&h, &c1.existing_asset_id).iter().map(|p| p.name.clone()).collect();
    assert!(names.contains(&"自定义名.png".to_string()), "link 用用户输入名：{names:?}");
    let names2: Vec<String> =
      placements_of(&h, &c2.existing_asset_id).iter().map(|p| p.name.clone()).collect();
    assert!(names2.contains(&"走路.fla".to_string()), "link 默认源文件原名：{names2:?}");
    // 链接 Placement 非 origin，源文件保留
    assert!(placements_of(&h, &c1.existing_asset_id).iter().filter(|p| p.origin).count() == 1);
    assert!(c1.source_path.exists());
  }

  #[test]
  fn skip_action_changes_nothing_even_with_move_source() {
    let h = make_lib();
    let conflict = make_conflict(&h, "原图.png", "副本.png", None);

    let report = resolve(&h, vec![to_resolution(&conflict, ResolveAction::Skip, None)], Source::Move);
    assert_eq!((report.resolved, report.failed.len()), (1, 0));
    // 什么都不做：Placement 仍只有原资源一条，源文件保留（即使 source 是剪切）
    assert_eq!(placements_of(&h, &conflict.existing_asset_id).len(), 1);
    assert!(conflict.source_path.exists(), "跳过即不导入，源文件保留");
  }

  #[test]
  fn failed_item_does_not_block_others() {
    let h = make_lib();
    let c1 = make_conflict(&h, "原图.png", "副本.png", None);
    let c2 = make_conflict(&h, "基准2.png", "副本2.png", None);
    // c1 的源文件删掉 → copy 必失败；c2 正常
    std::fs::remove_file(&c1.source_path).unwrap();

    let report = resolve(
      &h,
      vec![to_resolution(&c1, ResolveAction::Copy, None), to_resolution(&c2, ResolveAction::Link, None)],
      Source::Copy,
    );
    assert_eq!((report.resolved, report.failed.len()), (1, 1));
    assert!(report.failed[0].reason.contains("文件读取失败"), "中文原因：{}", report.failed[0].reason);
    assert_eq!(placements_of(&h, &c2.existing_asset_id).len(), 2);
  }

  #[test]
  fn rejects_missing_target_or_asset() {
    let h = make_lib();
    let c = make_conflict(&h, "原图.png", "副本.png", None);
    let mut bad_target = to_resolution(&c, ResolveAction::Link, None);
    bad_target.target_folder = "F-不存在".into();
    let mut bad_asset = to_resolution(&c, ResolveAction::UseExisting, None);
    bad_asset.existing_asset_id = "AN-不存在".into();

    let report = resolve(&h, vec![bad_target, bad_asset], Source::Copy);
    assert_eq!(report.failed.len(), 2);
    assert!(report.failed[0].reason.contains("目标文件夹不存在"));
    assert!(report.failed[1].reason.contains("库内资源不存在"));
  }

  #[test]
  fn resolves_into_keep_chain_leaf_folder() {
    let h = make_lib();
    // keep 模式冲突项的 target_folder 应为目录链末级
    let seed = h._dir.path().join("种子.png");
    std::fs::write(&seed, "相同内容").unwrap();
    import(&h, vec![seed], None);
    let tree = h._dir.path().join("参考");
    std::fs::create_dir_all(tree.join("人物")).unwrap();
    std::fs::write(tree.join("人物/副本.png"), "相同内容").unwrap();
    let opts = ImportOptions {
      structure: Structure::Keep,
      source: Source::Copy,
      target_folder: None,
      paths: vec![tree],
      imported_from: None,
    };
    let outcome = run_import(&h.lib, h.root.clone(), opts, |_| {}, &AtomicBool::new(false));
    assert_eq!(outcome.conflicts.len(), 1);
    let leaf = &outcome.conflicts[0].target_folder;
    with_lib(&h, |l| {
      let folder = &l.index.folders[leaf];
      assert_eq!(folder.name, "人物", "冲突落点应为目录链末级");
    });
    // link 决策落在末级文件夹
    let report = resolve(&h, vec![to_resolution(&outcome.conflicts[0], ResolveAction::Link, None)], Source::Copy);
    assert_eq!(report.failed.len(), 0);
    assert_eq!(with_lib(&h, |l| l.index.placements_by_folder[leaf.as_str()].len()), 1);
  }
}
