//! 冲突决策服务（PRD 10_重复添加提示）：导入挂起的 hash 冲突，前端逐项弹窗收集
//! 用户决策后一次性提交执行。单项失败不阻塞（中文原因进 failed），
//! 全部处理完重建 Placement 索引 + folders.json 一次落盘。

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::library::{self, Library};
use crate::core::models::new_asset_id;
use crate::core::paths;
use crate::services::import as ops;
use crate::services::import::{FailedItem, Source, ROOT_FOLDER_ID};
use crate::services::sequence;
use crate::services::thumbnail;

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

/// 单项决策（前端弹窗结果）。2026-08-13 用户定：弹窗内嵌批量重命名属过度设计废除
///（newName 字段同步删除）——"导入就是导入，只选择怎么导入；要命名导进来再说"
///（批量重命名归独立弹窗，阶段三，37 稿）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictResolution {
  pub source_path: PathBuf,
  pub existing_asset_id: String,
  pub target_folder: String,
  pub action: ResolveAction,
  /// 序列帧组信息（冲突项原样回传；action = "copy" 时按此重新打包建档，4.1）
  #[serde(default)]
  pub sequence: Option<ops::SequenceSource>,
  /// 预览缓存路径（冲突项原样回传；决策完成后删除——cache/tmp 会话级临时，4.2）
  #[serde(default)]
  pub source_thumb_path: Option<String>,
  /// 本批次导入渠道（冲突项原样回传，2026-08-13）：copy 建档的新资产继承该标记
  ///（自动导入冲突"保留两个"的新资产带 inbox 标 → 进"已导入"视图，否则用户以为文件丢失）
  #[serde(default)]
  pub imported_from: Option<String>,
}

/// 决策执行汇总（skip 也计入 resolved——用户已决策、无需动作）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveReport {
  pub resolved: u32,
  pub failed: Vec<FailedItem>,
}

/// 逐项执行决策：skip 直接过；copy 复用建档原语（继承 source）；useExisting / link 为轻操作；
/// 决策成功（含 skip）即删除该项预览缓存（cache/tmp 会话级临时，4.2）；
/// 全部处理完重建 Placement 索引 + folders.json 一次落盘（有改动才写）。
pub fn resolve_conflicts(lib: &mut Library, items: &[ConflictResolution], source: Source) -> ResolveReport {
  let mut report = ResolveReport { resolved: 0, failed: Vec::new() };
  let mut dirty = false;
  for item in items {
    match apply_one(lib, item, source) {
      Ok(changed) => {
        dirty |= changed;
        report.resolved += 1;
        cleanup_preview(item);
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

/// 决策完成（含 skip）后删除该项预览缓存（cache/tmp 会话级临时）；删除失败仅记日志——
/// 启动时 open 还会整目录兜底清空，不阻塞决策
fn cleanup_preview(item: &ConflictResolution) {
  if let Some(p) = &item.source_thumb_path {
    if let Err(e) = std::fs::remove_file(p) {
      log::warn!("预览缓存删除失败（{p}）：{e}");
    }
  }
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

/// link：目标文件夹建链接 Placement（显示名 = 源文件原名，unique_name），源文件不动；
/// 落点不设限（2026-08-13 用户定案：允许落未分类，原 check_not_root 候车区护栏废除）
fn link(lib: &mut Library, item: &ConflictResolution) -> Result<(), String> {
  check_target(lib, &item.target_folder)?;
  check_asset(lib, &item.existing_asset_id)?;
  let name = ops::file_name(&item.source_path);
  ops::add_placement(lib, item.existing_asset_id.clone(), item.target_folder.clone(), name, false, &mut HashMap::new());
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

/// copy：源走标准建档（新 ID 新实体 + origin Placement），继承原导入的 source；
/// 序列帧冲突按回传的组信息重新打包（frames/ 归一，组 hash 重算）
fn copy_new(lib: &mut Library, item: &ConflictResolution, source: Source) -> Result<(), String> {
  check_target(lib, &item.target_folder)?;
  let root = lib.root.clone();
  let assets_dir = lib.assets_dir.clone();
  match &item.sequence {
    Some(seq) => {
      let stem = seq.name.clone();
      let frames: Vec<(PathBuf, u32)> = seq.frames.iter().map(|f| (f.path.clone(), f.number)).collect();
      let mut hashes = Vec::with_capacity(frames.len());
      for (path, _) in &frames {
        hashes.push(ops::sha256_file(path).map_err(|e| format!("文件读取失败：{e}"))?);
      }
      let hash = sequence::group_hash(&hashes.iter().map(String::as_str).collect::<Vec<_>>());
      let asset = sequence::build_sequence_asset(
        &assets_dir,
        &stem,
        &seq.ext,
        seq.width as usize,
        &frames,
        &hash,
        &new_asset_id(),
        source,
        item.imported_from.as_deref(), // 继承批次渠道标记（2026-08-13）
      )?;
      ops::add_placement_keep_name(lib, asset.id.clone(), item.target_folder.clone(), format!("{stem}.{}", seq.ext), true, &mut HashMap::new());
      lib.index.assets.insert(asset.id.clone(), asset);
      Ok(())
    }
    None => {
      let pending = ops::PendingFile { path: item.source_path.clone(), chain: Vec::new() };
      let hash = ops::sha256_file(&pending.path).map_err(|e| format!("文件读取失败：{e}"))?;
      // build_asset_with_id：携带批次渠道标记（新资产进"已导入"视图，2026-08-13）
      let asset = ops::build_asset_with_id(&assets_dir, &pending, &hash, &new_asset_id(), source, item.imported_from.as_deref())?;
      // 显示名 = 源文件原名；hash 判定不受影响
      let name = ops::file_name(&pending.path);
      // 4.2：无内容缩略图补系统格式图标缓存（cache/thumbs，不进 info）并登记内存集
      if asset.thumbnail.is_none() {
        let origin = paths::origin_file(&assets_dir, &asset.id, &asset.origin_name);
        if thumbnail::ensure_format_icon(&root, &origin, &asset.ext) {
          lib.format_icons.insert(asset.ext.clone());
        }
      }
      // "保留两个"允许同名（2026-08-14 用户定：不自动加序号，要改名用户自己改）
      ops::add_placement_keep_name(lib, asset.id.clone(), item.target_folder.clone(), name, true, &mut HashMap::new());
      lib.index.assets.insert(asset.id.clone(), asset);
      Ok(())
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library;
  use crate::core::models::{Folder, Placement};
  use crate::services::import::{ImportOptions, SequenceMode, Structure};
  use crate::services::import_task::run_import;
  use std::sync::atomic::AtomicBool;
  use std::sync::Mutex;

  struct Harness {
    _dir: tempfile::TempDir,
    lib: Mutex<Option<Library>>,
    root: PathBuf,
    assets_dir: PathBuf,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    let root = lib.root.clone();
    let assets_dir = lib.assets_dir.clone();
    Harness { _dir: dir, lib: Mutex::new(Some(lib)), root, assets_dir }
  }

  fn with_lib<R>(h: &Harness, f: impl FnOnce(&mut Library) -> R) -> R {
    f(h.lib.lock().unwrap().as_mut().unwrap())
  }

  #[test]
  fn resolved_decision_removes_preview_cache() {
    let h = make_lib();
    // 预览缓存文件（cache/tmp）；决策完成（含 skip）即删
    let cache = h.root.join("cache/tmp/abc.webp");
    std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
    std::fs::write(&cache, b"x").unwrap();

    let report = with_lib(&h, |l| {
      resolve_conflicts(
        l,
        &[ConflictResolution {
          source_path: PathBuf::from("源.svg"),
          existing_asset_id: "AN-X".into(),
          target_folder: ROOT_FOLDER_ID.into(),
          action: ResolveAction::Skip,
          sequence: None,
          source_thumb_path: Some(cache.to_string_lossy().into_owned()),
          imported_from: None,
        }],
        Source::Copy,
      )
    });
    assert_eq!(report.resolved, 1);
    assert!(!cache.exists(), "决策完成后预览缓存应删除");
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
    let opts = ImportOptions {
      structure: Structure::Flatten,
      source: Source::Copy,
      target_folder: target,
      paths,
      imported_from: None,
      sequence_mode: SequenceMode::Scatter,
    };
    run_import(&h.lib, h.root.clone(), h.assets_dir.clone(), opts, |_| {}, &AtomicBool::new(false))
  }

  /// 手动加文件夹（返回 ID）
  fn add_folder(h: &Harness, id: &str, name: &str) -> String {
    with_lib(h, |l| {
      let folder = Folder { id: id.into(), name: name.into(), parent: None, block: None, icon: None, kind: None, color: None, groups: None, order: 0 };
      l.folders.folders.push(folder.clone());
      l.index.folders.insert(id.into(), folder);
      id.into()
    })
  }

  fn resolve(h: &Harness, items: Vec<ConflictResolution>, source: Source) -> ResolveReport {
    with_lib(h, |l| resolve_conflicts(l, &items, source))
  }

  fn to_resolution(c: &ops::ConflictItem, action: ResolveAction) -> ConflictResolution {
    ConflictResolution {
      source_path: c.source_path.clone(),
      existing_asset_id: c.existing_asset_id.clone(),
      target_folder: c.target_folder.clone(),
      action,
      sequence: c.sequence.clone(),
      source_thumb_path: c.source_thumb_path.clone(),
      imported_from: c.imported_from.clone(),
    }
  }

  fn placements_of(h: &Harness, asset_id: &str) -> Vec<Placement> {
    with_lib(h, |l| l.folders.items.iter().filter(|p| p.asset == asset_id).cloned().collect())
  }

  /// 批次渠道继承（2026-08-13 用户实测：自动导入冲突"保留两个"的新资产丢标，
  /// 不进"已导入"视图以为文件丢失）：copy 决策的新资产 metadata.imported_from = 批次渠道
  #[test]
  fn copy_inherits_batch_imported_from() {
    let h = make_lib();
    let mut c = make_conflict(&h, "原图.png", "副本.png", None);
    c.imported_from = Some("inbox".into());
    let old_id = c.existing_asset_id.clone();

    let report = resolve(&h, vec![to_resolution(&c, ResolveAction::Copy)], Source::Copy);
    assert_eq!((report.resolved, report.failed.len()), (1, 0));

    with_lib(&h, |l| {
      let new = l.index.assets.values().find(|a| a.id != old_id).expect("新资产已建档");
      assert_eq!(new.imported_from.as_deref(), Some("inbox"), "新资产继承批次渠道标记");
    });
  }

  #[test]
  fn use_existing_moves_origin_and_dedups_link() {
    let h = make_lib();
    let folder_id = add_folder(&h, "F01", "人物");
    let conflict = make_conflict(&h, "原图.png", "副本.png", Some(folder_id.clone()));
    let asset_id = conflict.existing_asset_id.clone();
    // 预置：目标文件夹已有一条该资源的链接 Placement（应被去重删掉）
    with_lib(&h, |l| {
      ops::add_placement(l, asset_id.clone(), folder_id.clone(), "快捷.png".into(), false, &mut HashMap::new());
    });

    let report = resolve(&h, vec![to_resolution(&conflict, ResolveAction::UseExisting)], Source::Copy);
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
  fn copy_action_keeps_same_name_without_counter() {
    let h = make_lib();
    // 同名冲突（不同目录、同名同内容）：库内已有"场景.png"（root），再导入同名文件 → 冲突
    let d1 = h._dir.path().join("d1");
    let d2 = h._dir.path().join("d2");
    std::fs::create_dir_all(&d1).unwrap();
    std::fs::create_dir_all(&d2).unwrap();
    std::fs::write(d1.join("场景.png"), "同内容").unwrap();
    std::fs::write(d2.join("场景.png"), "同内容").unwrap();
    assert_eq!(import(&h, vec![d1.join("场景.png")], None).imported, 1);
    let outcome = import(&h, vec![d2.join("场景.png")], None);
    assert_eq!(outcome.conflicts.len(), 1);
    let c = outcome.conflicts[0].clone();

    let report = resolve(&h, vec![to_resolution(&c, ResolveAction::Copy)], Source::Copy);
    assert_eq!((report.resolved, report.failed.len()), (1, 0));
    let names: Vec<String> = with_lib(&h, |l| {
      l.folders.items.iter().filter(|p| p.folder == ROOT_FOLDER_ID).map(|p| p.name.clone()).collect()
    });
    assert_eq!(names.iter().filter(|n| n.as_str() == "场景.png").count(), 2, "保留两个不自动加序号（2026-08-14 用户定）");
  }

  #[test]
  fn copy_action_imports_new_entity_with_inherited_source() {
    let h = make_lib();
    let folder_id = add_folder(&h, "F01", "人物");
    let conflict = make_conflict(&h, "原图.png", "副本.png", Some(folder_id.clone()));
    let old_asset = conflict.existing_asset_id.clone();

    // 继承 copy：源文件保留，实体两份
    let report = resolve(&h, vec![to_resolution(&conflict, ResolveAction::Copy)], Source::Copy);
    assert_eq!((report.resolved, report.failed.len()), (1, 0));
    assert!(conflict.source_path.exists(), "copy 模式源文件应保留");
    assert_eq!(with_lib(&h, |l| l.index.assets.len()), 2);
    assert_eq!(std::fs::read_dir(&h.assets_dir).unwrap().count(), 2);

    // 继承 move：源文件消失（再造一个冲突）
    let conflict2 = make_conflict(&h, "基准2.png", "副本2.png", None);
    let report2 = resolve(&h, vec![to_resolution(&conflict2, ResolveAction::Copy)], Source::Move);
    assert_eq!(report2.resolved, 1);
    assert!(!conflict2.source_path.exists(), "move 模式源文件应移入库");
    // 旧资源的 origin Placement 不受新实体影响
    assert_eq!(placements_of(&h, &old_asset).len(), 1);
  }

  #[test]
  fn link_action_uses_source_file_names() {
    let h = make_lib();
    let folder_id = add_folder(&h, "F01", "人物");
    let c1 = make_conflict(&h, "原图.png", "副本.png", Some(folder_id.clone()));
    let c2 = make_conflict(&h, "基准2.png", "走路.fla", Some(folder_id));

    let report = resolve(
      &h,
      vec![
        to_resolution(&c1, ResolveAction::Link),
        to_resolution(&c2, ResolveAction::Link),
      ],
      Source::Copy,
    );
    assert_eq!((report.resolved, report.failed.len()), (2, 0));

    let names: Vec<String> =
      placements_of(&h, &c1.existing_asset_id).iter().map(|p| p.name.clone()).collect();
    assert!(names.contains(&"副本.png".to_string()), "link 用源文件原名：{names:?}");
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

    let report = resolve(&h, vec![to_resolution(&conflict, ResolveAction::Skip)], Source::Move);
    assert_eq!((report.resolved, report.failed.len()), (1, 0));
    // 什么都不做：Placement 仍只有原资源一条，源文件保留（即使 source 是剪切）
    assert_eq!(placements_of(&h, &conflict.existing_asset_id).len(), 1);
    assert!(conflict.source_path.exists(), "跳过即不导入，源文件保留");
  }

  #[test]
  fn failed_item_does_not_block_others() {
    let h = make_lib();
    let folder_id = add_folder(&h, "F01", "人物");
    let c1 = make_conflict(&h, "原图.png", "副本.png", None);
    let c2 = make_conflict(&h, "基准2.png", "副本2.png", Some(folder_id));
    // c1 的源文件删掉 → copy 必失败；c2 正常
    std::fs::remove_file(&c1.source_path).unwrap();

    let report = resolve(
      &h,
      vec![to_resolution(&c1, ResolveAction::Copy), to_resolution(&c2, ResolveAction::Link)],
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
    let mut bad_target = to_resolution(&c, ResolveAction::Link);
    bad_target.target_folder = "F-不存在".into();
    let mut bad_asset = to_resolution(&c, ResolveAction::UseExisting);
    bad_asset.existing_asset_id = "AN-不存在".into();

    let report = resolve(&h, vec![bad_target, bad_asset], Source::Copy);
    assert_eq!(report.failed.len(), 2);
    assert!(report.failed[0].reason.contains("目标文件夹不存在"));
    assert!(report.failed[1].reason.contains("库内资源不存在"));
  }

  /// 决策落点不设限（2026-08-13 用户定案）：link / useExisting 落点为 root（未分类）照常执行
  #[test]
  fn link_and_use_existing_allow_root_target() {
    let h = make_lib();
    // 导入目标为 None → 冲突落点即未分类（root）
    let c1 = make_conflict(&h, "原图.png", "副本.png", None);
    let c2 = make_conflict(&h, "基准2.png", "副本2.png", None);

    let report = resolve(
      &h,
      vec![to_resolution(&c1, ResolveAction::Link), to_resolution(&c2, ResolveAction::UseExisting)],
      Source::Copy,
    );
    assert_eq!((report.resolved, report.failed.len()), (2, 0), "落点 root 应照常执行：{:?}", report.failed);
    // c1 在 root 建出链接；c2 的 origin 仍在 root（本就落在 root，幂等）
    let p1 = placements_of(&h, &c1.existing_asset_id);
    assert_eq!(p1.len(), 2);
    assert!(p1.iter().any(|p| !p.origin && p.folder == ROOT_FOLDER_ID), "root 应多出链接");
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
      sequence_mode: SequenceMode::Scatter,
    };
    let outcome = run_import(&h.lib, h.root.clone(), h.assets_dir.clone(), opts, |_| {}, &AtomicBool::new(false));
    assert_eq!(outcome.conflicts.len(), 1);
    let leaf = &outcome.conflicts[0].target_folder;
    with_lib(&h, |l| {
      let folder = &l.index.folders[leaf];
      assert_eq!(folder.name, "人物", "冲突落点应为目录链末级");
    });
    // link 决策落在末级文件夹
    let report = resolve(&h, vec![to_resolution(&outcome.conflicts[0], ResolveAction::Link)], Source::Copy);
    assert_eq!(report.failed.len(), 0);
    assert_eq!(with_lib(&h, |l| l.index.placements_by_folder[leaf.as_str()].len()), 1);
  }

  /// 打包模式导入（4.1）
  fn import_pack(h: &Harness, paths: Vec<PathBuf>) -> ops::ImportOutcome {
    let opts = ImportOptions {
      structure: Structure::Flatten,
      source: Source::Copy,
      target_folder: None,
      paths,
      imported_from: None,
      sequence_mode: SequenceMode::Pack,
    };
    run_import(&h.lib, h.root.clone(), h.assets_dir.clone(), opts, |_| {}, &AtomicBool::new(false))
  }

  #[test]
  fn copy_action_repacks_sequence_from_source() {
    let h = make_lib();
    // 造 3 帧 PNG 序列（内容固定，两次导入组 hash 相同）
    let src = h._dir.path().join("序列");
    std::fs::create_dir_all(&src).unwrap();
    let mut paths = Vec::new();
    for n in 1..=3u8 {
      let p = src.join(format!("走路_{n}.png"));
      image::RgbaImage::from_pixel(8, 8, image::Rgba([n, 0, 0, 255])).save(&p).unwrap();
      paths.push(p);
    }

    // 首次打包入库（copy 源保留）→ 再次打包导入撞组 hash 挂起冲突
    assert_eq!(import_pack(&h, paths.clone()).imported, 1);
    let out = import_pack(&h, paths.clone());
    assert_eq!(out.conflicts.len(), 1, "序列帧应整组挂起一个冲突");
    let c = out.conflicts[0].clone();
    assert_eq!(c.source_name, "走路.png", "显示名用序列帧标题");
    assert!(c.sequence.is_some(), "冲突项应携带组信息");

    // "保留两个"：按回传组信息重新打包（新 ID、frames/ 归一、fps/frameCount 元数据）
    let report = resolve(&h, vec![to_resolution(&c, ResolveAction::Copy)], Source::Copy);
    assert_eq!((report.resolved, report.failed.len()), (1, 0));
    assert_eq!(with_lib(&h, |l| l.index.assets.len()), 2);
    let new_meta = with_lib(&h, |l| {
      l.index.assets.values().find(|a| a.id != c.existing_asset_id).unwrap().clone()
    });
    assert_eq!((new_meta.fps, new_meta.frame_count), (Some(24.0), Some(3)));
    let frames_dir = crate::core::paths::frames_dir(&h.assets_dir, &new_meta.id);
    for n in 1..=3 {
      assert!(frames_dir.join(format!("走路-{n}.png")).is_file());
    }
    assert_eq!(placements_of(&h, &new_meta.id)[0].name, "走路.png", "保留两个不自动加序号——同名允许（2026-08-14 用户定）");
    // 源帧保留（继承 copy）
    assert!(paths.iter().all(|p| p.exists()));
  }
}
