//! 副本与重命名（6.1）：duplicate = info 目录整体复制 + 新 ID + origin Placement
//! （副本 = 独立物理本体，落点即原始位置，与导入建档 / 克隆文件夹同口径——
//! 2026-09-08 修复：原误标链接语义 origin=false，回收站彻底清除按"快照含 origin = 本体在站"
//! 判定漏物理删，副本 info 永久残留；重命名副本随 origin 走磁盘段改名）；
//! rename = 显示名即时改，原始 Placement 追加磁盘段（originName + 词干缩略图同步改名）。

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::json_io::{read_json, write_json};
use crate::core::library::{self, Library};
use crate::core::models::{new_asset_id, AssetMeta, Placement};
use crate::services::import as ops;

use super::{check_folder, find_placement, finish, now, rewrite_orders, PlacementRef, RenameItem};

/// duplicate 结果：新资源 ID 清单 + 每条副本的（源 info, 新 info）目录对（命令层登记 DiskOp 用）
#[derive(Debug, Default)]
pub struct DuplicateOutcome {
  pub new_ids: Vec<String>,
  pub copies: Vec<(PathBuf, PathBuf)>,
}

/// 创建副本：每条 = copy_dir `{旧ID}.info` → `{新ID}.info`（保留原 hash，利于导入查重命中）
/// → 改 metadata（id / name / importedAt / modifiedAt）写盘 → 索引 insert
/// → 副本 Placement 挂 **origin = true**（2026-09-08 修复：副本 = 独立物理本体，落点即原始位置，
///   与导入建档 / 克隆文件夹同口径；原误标链接语义致回收站彻底清除漏物理删 info 残留；
///   连带口径：重命名副本走 origin 磁盘段改名、导入查重"用已存入文件导入"可命中副本）
/// → 副本落点（2026-08-20 用户定，替换原统一"段首前插"）：
///   源在目标文件夹内 = **紧随原件**（克隆后不用回顶部找——原段首前插会把副本顶到第一行第一列；
///   恒插源后第一位，连续副本时新副本贴原件、旧副本顺后移）；
///   跨文件夹副本粘贴（Ctrl+Shift+V 落他处）= 段首前插（新项顶部可见，导入同语义）。
/// 任一条失败整体 Err，已创建的副本全部回滚（目录删除 + 索引 / Placement 清理）
pub fn duplicate_placements(
  lib: &mut Library,
  items: &[PlacementRef],
  target_folder: &str,
  names: &[Option<String>],
) -> Result<DuplicateOutcome, String> {
  check_folder(lib, target_folder)?;
  let mut out = DuplicateOutcome::default();
  let mut front_anchor: HashMap<String, usize> = HashMap::new(); // 跨文件夹 add_placement 段首批次锚点
  for (i, item) in items.iter().enumerate() {
    let custom = names.get(i).and_then(|n| n.as_ref()).map(|n| n.trim()).filter(|n| !n.is_empty());
    match duplicate_one(lib, item, target_folder, custom, &mut front_anchor) {
      Ok(Some((new_id, pair))) => {
        out.new_ids.push(new_id);
        out.copies.push(pair);
      }
      Ok(None) => {} // 源 Placement 不存在（陈旧选择）跳过
      Err(e) => {
        rollback_copies(lib, &out);
        return Err(e);
      }
    }
  }
  if !out.new_ids.is_empty() {
    rewrite_orders(lib, target_folder); // 紧随插入后 order 连续化（finish 重建索引 + 落盘）
    finish(lib)?;
    log::info!("创建 {} 个副本至 {target_folder}", out.new_ids.len());
  }
  Ok(out)
}

/// 单条副本结果：新资源 ID + （源 info, 新 info）目录对
type OneCopy = (String, (PathBuf, PathBuf));

/// 单条副本；源 Placement 不存在返回 Ok(None)
fn duplicate_one(
  lib: &mut Library,
  item: &PlacementRef,
  target_folder: &str,
  custom: Option<&str>,
  front_anchor: &mut HashMap<String, usize>,
) -> Result<Option<OneCopy>, String> {
  let Some(pos) = find_placement(lib, item) else { return Ok(None) };
  let source_name = lib.folders.items[pos].name.clone();
  if !lib.index.assets.contains_key(&item.asset_id) {
    return Err("资源数据异常：缺少元数据".to_string());
  }
  let new_id = new_asset_id();
  let old_info = lib.info_dir(&item.asset_id);
  let new_info = lib.info_dir(&new_id);
  copy_dir(&old_info, &new_info)?;

  let meta_path = new_info.join("metadata.json");
  let mut meta: AssetMeta = read_json(&meta_path).map_err(|e| e.to_string())?;
  meta.id = new_id.clone();
  // 序列帧 originName 是帧名（"走路-1.png"），词干当资产名会带出 "-1"——副本沿用源资产名（2026-08-14）
  meta.name = custom.map(stem_of).unwrap_or_else(|| {
    if crate::services::sequence::frames_dir_if_sequence(&lib.assets_dir, &item.asset_id, &meta).is_some() {
      meta.name.clone()
    } else {
      stem_of(&meta.origin_name)
    }
  });
  let stamp = now();
  meta.imported_at = stamp.clone();
  meta.modified_at = stamp;
  write_json(&meta_path, &meta).map_err(|e| e.to_string())?;
  lib.index.assets.insert(new_id.clone(), meta); // 先落盘再同步内存（规范第 7 条）

  if item.folder_id == target_folder {
    // 同文件夹：紧随原件（2026-08-20 用户定）。恒插源后第一位——连续副本时新副本贴原件、
    // 旧副本顺后移；显示名唯一化读 items 实时数组（本批已插副本尚未入索引——finish 才 rebuild）；
    // order 由调用方 rewrite_orders 统一连续化
    let existing: HashSet<String> =
      lib.folders.items.iter().filter(|p| p.folder == target_folder).map(|p| p.name.clone()).collect();
    let display = library::unique_name(&existing, custom.unwrap_or(&source_name));
    let placement = Placement {
      asset: new_id.clone(),
      folder: target_folder.to_string(),
      name: display,
      order: 0,
      origin: true,
    };
    lib.folders.items.insert(pos + 1, placement);
  } else {
    // 跨文件夹副本粘贴：段首前插（导入同语义，新项顶部可见）；add_placement 内部唯一化 + 索引同步
    let display = custom.unwrap_or(&source_name).to_string();
    ops::add_placement(lib, new_id.clone(), target_folder.to_string(), display, true, front_anchor);
  }
  Ok(Some((new_id, (old_info, new_info))))
}

/// 副本失败回滚：清索引 / Placement + 删已复制目录（撤销事务由命令层 cancel）
fn rollback_copies(lib: &mut Library, out: &DuplicateOutcome) {
  for id in &out.new_ids {
    lib.index.assets.remove(id);
  }
  lib.folders.items.retain(|p| !out.new_ids.contains(&p.asset));
  lib.index.rebuild_placements(&lib.folders.items);
  for (_, new_dir) in &out.copies {
    if let Err(e) = fs::remove_dir_all(new_dir) {
      log::error!("副本回滚清理失败（{}）：{e}", new_dir.display());
    }
  }
}

/// 重命名 Placement：显示名即时改（rebuild + save_folders；**撞名不唯一化**——显示名放开，
/// PRD 01 边界规则 7，2026-08-18 用户定"护栏全拆"，单条与批量统一口径）；
/// 原始 Placement 追加磁盘段——rename_origin 原语改 originName + 词干缩略图 + meta 写盘 +
/// 索引同步，返回磁盘改名清单（旧路径, 新路径）（命令层登记 DiskOp::Rename；链接 Placement 为空）。
/// 原始 Placement 改名补扩展名（用户只改词干不丢 .fla，Windows 资源管理器同行为）；
/// 磁盘物理约束仍在：origin 撞名致改名失败中文报错整体回滚。
/// 磁盘段先行（更易失败：fla 被 Animate 占用）；folders.json 落盘失败时磁盘改名尽力改回
pub fn rename_placement(
  lib: &mut Library,
  asset_id: &str,
  folder_id: &str,
  origin: bool,
  new_name: &str,
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
  let trimmed = new_name.trim();
  library::validate_library_name(trimmed).map_err(|e| e.to_string())?;
  let item = PlacementRef { asset_id: asset_id.into(), folder_id: folder_id.into(), origin };
  let pos = find_placement(lib, &item).ok_or_else(|| "项目不存在，可能已被移动或删除".to_string())?;
  let effective = effective_name(lib, asset_id, origin, trimmed);

  // 磁盘段先行（originName + 词干缩略图 + meta 写盘）
  let old_meta = lib.index.assets.get(asset_id).cloned();
  let mut renames = Vec::new();
  if origin {
    let old_origin = old_meta.as_ref().map(|m| m.origin_name.clone()).unwrap_or_default();
    let result = ops::rename_origin(&lib.assets_dir, asset_id, &effective).map_err(|e| rename_fail(e, &old_origin))?;
    lib.index.assets.insert(asset_id.to_string(), result.meta);
    renames = result.renames;
  }

  // 显示名（撞名不拦截：显示名放开）
  let old_name = lib.folders.items[pos].name.clone();
  lib.folders.items[pos].name = effective.clone();
  lib.index.rebuild_placements(&lib.folders.items);
  if let Err(e) = library::save_folders(lib) {
    lib.folders.items[pos].name = old_name;
    lib.index.rebuild_placements(&lib.folders.items);
    if let Some(meta) = old_meta {
      if origin {
        // 磁盘改名尽力改回（含 metadata 复原）；
        // 序列帧 originName 是帧名（非改名输入口径），回滚名用 词干.扩展名（2026-08-14）
        let rollback_name = if crate::services::sequence::frames_dir_if_sequence(&lib.assets_dir, asset_id, &meta).is_some() {
          format!("{}.{}", meta.name, meta.ext)
        } else {
          meta.origin_name.clone()
        };
        if let Err(re) = ops::rename_origin(&lib.assets_dir, asset_id, &rollback_name) {
          log::error!("重命名落盘失败后磁盘回滚失败：{re}");
        }
      }
      lib.index.assets.insert(asset_id.to_string(), meta);
    }
    return Err(e.to_string());
  }
  log::info!("重命名为 {effective}");
  Ok(renames)
}

/// 批量重命名（3.4，37 稿 / PRD 02-14，2026-08-18）：逐条改名（语义同 rename_placement，
/// 撞名不拦截——显示名放开）；**失败项跳过不整体回滚**，汇总报告。
/// renames = 全部成功项的磁盘改名对汇总（命令层登记 DiskOp 用）
pub fn rename_placements(lib: &mut Library, items: &[RenameItem]) -> Result<BatchRenameOutcome, String> {
  let mut out = BatchRenameOutcome::default();
  for it in items {
    match rename_placement(lib, &it.asset_id, &it.folder_id, it.origin, &it.new_name) {
      Ok(rs) => {
        out.renames.extend(rs);
        out.renamed += 1;
      }
      Err(e) => out.failed.push(BatchRenameFailed { name: it.new_name.clone(), reason: e }),
    }
  }
  log::info!("批量重命名 {} 个（失败 {}）", out.renamed, out.failed.len());
  Ok(out)
}

/// 批量重命名结果：成功计数 + 失败清单 + 磁盘改名对汇总
#[derive(Debug, Default)]
pub struct BatchRenameOutcome {
  pub renamed: usize,
  pub failed: Vec<BatchRenameFailed>,
  pub renames: Vec<(PathBuf, PathBuf)>,
}

/// 批量重命名失败项（前端汇总提示用）
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRenameFailed {
  pub name: String,
  pub reason: String,
}

/// 磁盘段失败文案：提示可能被占用（fla 在 Animate 中打开是常见场景）
fn rename_fail(e: String, old_origin: &str) -> String {
  if e.contains("改名失败") {
    format!("{e}（{old_origin} 可能正被其他软件占用，请关闭后重试）")
  } else {
    e
  }
}

/// 实际生效名：原始 Placement 无扩展名时补 metadata.ext（链接 Placement 显示名自由，不补）
fn effective_name(lib: &Library, asset_id: &str, origin: bool, name: &str) -> String {
  if !origin || Path::new(name).extension().is_some() {
    return name.to_string();
  }
  match lib.index.assets.get(asset_id).map(|m| m.ext.as_str()) {
    Some(ext) if !ext.is_empty() => format!("{name}.{ext}"),
    _ => name.to_string(),
  }
}

/// 去扩展名（metadata.name 用语）
fn stem_of(name: &str) -> String {
  Path::new(name).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

/// 递归复制目录（duplicate 的 info 目录整体复制；pub(crate)：clone.rs 克隆资产同管线复用）
pub(crate) fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
  fs::create_dir_all(to).map_err(|e| format!("目录创建失败：{e}"))?;
  let entries = fs::read_dir(from).map_err(|e| format!("目录读取失败：{e}"))?;
  for entry in entries.filter_map(|e| e.ok()) {
    let path = entry.path();
    let dest = to.join(entry.file_name());
    if path.is_dir() {
      copy_dir(&path, &dest)?;
    } else {
      fs::copy(&path, &dest).map_err(|e| format!("文件复制失败：{e}"))?;
    }
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library::FOLDERS_FILE;
  use crate::core::models::FoldersFile;
  use crate::services::import::{PendingFile, Source};

  struct Harness {
    dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { dir, lib }
  }

  /// 造 PNG 资源（有内容缩略图 {stem}_thumbnail.webp）+ origin Placement 到 root
  fn add_png(h: &mut Harness, id: &str, file: &str) {
    let src = h.dir.path().join(file);
    image::RgbaImage::from_pixel(4, 4, image::Rgba([0, 0, 0, 255])).save(&src).unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = ops::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.into(), meta);
    ops::add_placement(&mut h.lib, id.into(), "root".into(), file.into(), true, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
  }

  fn pref(asset: &str, folder: &str, origin: bool) -> PlacementRef {
    PlacementRef { asset_id: asset.into(), folder_id: folder.into(), origin }
  }

  #[test]
  fn duplicate_copies_info_dir_with_new_id() {
    let mut h = make_lib();
    add_png(&mut h, "AN-A", "原图.png");
    let old_hash = h.lib.index.assets["AN-A"].hash.clone();
    let old_thumb = h.lib.index.assets["AN-A"].thumbnail.clone();

    let out = duplicate_placements(&mut h.lib, &[pref("AN-A", "root", true)], "root", &[None]).unwrap();
    assert_eq!(out.new_ids.len(), 1);
    let new_id = &out.new_ids[0];
    assert_ne!(new_id, "AN-A");

    // info 目录整体复制（源文件 + 缩略图 + metadata）
    let new_info = h.lib.info_dir(new_id);
    assert!(new_info.join("原图.png").is_file());
    assert_eq!(out.copies.len(), 1);
    // metadata：新 ID、时间刷新、hash 保留、缩略图登记保留
    let meta = &h.lib.index.assets[new_id];
    assert_eq!(meta.id, *new_id);
    assert_eq!(meta.hash, old_hash, "副本保留原 hash（导入查重命中）");
    assert_eq!(meta.thumbnail, old_thumb);
    let disk: AssetMeta = read_json(&new_info.join("metadata.json")).unwrap();
    assert_eq!(disk.id, *new_id, "metadata.json 落盘");
    // Placement：origin = true（副本 = 独立本体，落点即原始位置），显示名唯一化（同名追加 -1）
    let p = h.lib.folders.items.iter().find(|p| p.asset == *new_id).unwrap();
    assert!(p.origin);
    assert_eq!(p.name, "原图-1.png");
    // 陈旧三元组跳过不计
    let out2 = duplicate_placements(&mut h.lib, &[pref("AN-幽灵", "root", true)], "root", &[]).unwrap();
    assert!(out2.new_ids.is_empty());
  }

  /// 副本 = 独立本体（origin Placement）：删除进回收站 → 彻底清除物理删副本 info、源不动
  ///（2026-09-08 修复闭环：原 origin=false 误标，回收站按"纯位置记录"只删记录致 info 残留）
  #[test]
  fn duplicate_delete_purges_copy_info() {
    let mut h = make_lib();
    add_png(&mut h, "AN-A", "原图.png");
    let out = duplicate_placements(&mut h.lib, &[pref("AN-A", "root", true)], "root", &[None]).unwrap();
    let new_id = out.new_ids[0].clone();
    let new_info = h.lib.info_dir(&new_id);
    assert!(new_info.is_dir());

    crate::services::operations::delete_placements(&mut h.lib, &[pref(&new_id, "root", true)], Some("recycle")).unwrap();
    let rep = crate::services::recycle::delete_recycle(&mut h.lib, &[new_id.clone()], false).unwrap();
    assert_eq!(rep.deleted, 1, "无引用无活链接，第一段直接删不需确认");
    assert!(!new_info.exists(), "副本 info 物理删除");
    assert!(h.lib.info_dir("AN-A").is_dir(), "源 info 不动");
    assert!(h.lib.index.assets.contains_key("AN-A"));
    assert!(!h.lib.index.assets.contains_key(&new_id), "副本 index 移除");
    assert!(h.lib.recycle.items.is_empty(), "回收站条目清空");
  }

  #[test]
  fn duplicate_honors_custom_name() {
    let mut h = make_lib();
    add_png(&mut h, "AN-A", "原图.png");
    let out =
      duplicate_placements(&mut h.lib, &[pref("AN-A", "root", true)], "root", &[Some("副本.png".into())]).unwrap();
    let new_id = &out.new_ids[0];
    let p = h.lib.folders.items.iter().find(|p| p.asset == *new_id).unwrap();
    assert_eq!(p.name, "副本.png");
    assert_eq!(h.lib.index.assets[new_id].name, "副本", "metadata.name 用去扩展名形式");
  }

  #[test]
  fn rename_origin_placement_renames_disk_and_thumb() {
    let mut h = make_lib();
    add_png(&mut h, "AN-A", "原图.png");
    let info = h.lib.info_dir("AN-A");
    assert!(info.join("原图_thumbnail.webp").is_file());

    let renames = rename_placement(&mut h.lib, "AN-A", "root", true, "新图").unwrap();
    // 无扩展名自动补 .png；磁盘 originName + 词干缩略图同步改名
    assert!(info.join("新图.png").is_file());
    assert!(!info.join("原图.png").exists());
    assert!(info.join("新图_thumbnail.webp").is_file());
    assert_eq!(renames.len(), 2, "origin + 缩略图两对磁盘改名");
    let meta = &h.lib.index.assets["AN-A"];
    assert_eq!((meta.origin_name.as_str(), meta.name.as_str()), ("新图.png", "新图"));
    assert_eq!(meta.thumbnail.as_deref(), Some("新图_thumbnail.webp"));
    let disk: AssetMeta = read_json(&info.join("metadata.json")).unwrap();
    assert_eq!(disk.origin_name, "新图.png", "metadata.json 落盘");
    // 显示名同步改
    let p = h.lib.folders.items.iter().find(|p| p.asset == "AN-A").unwrap();
    assert_eq!(p.name, "新图.png");
    let folders: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(folders.items[0].name, "新图.png", "folders.json 落盘");
  }

  #[test]
  fn rename_link_placement_only_changes_display_name() {
    let mut h = make_lib();
    add_png(&mut h, "AN-A", "原图.png");
    ops::add_placement(&mut h.lib, "AN-A".into(), "root".into(), "快捷.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
    let info = h.lib.info_dir("AN-A");

    let renames = rename_placement(&mut h.lib, "AN-A", "root", false, "别名.png").unwrap();
    assert!(renames.is_empty(), "链接 Placement 无磁盘段");
    assert!(info.join("原图.png").is_file(), "磁盘不动");
    assert_eq!(h.lib.index.assets["AN-A"].origin_name, "原图.png");
    let link = h.lib.folders.items.iter().find(|p| p.asset == "AN-A" && !p.origin).unwrap();
    assert_eq!(link.name, "别名.png");
  }

  #[test]
  fn rename_rejects_invalid_names() {
    let mut h = make_lib();
    add_png(&mut h, "AN-A", "原图.png");
    // 首尾空白先 trim 再校验："名 " trim 后为合法名；"名." 与纯空白拒绝
    for bad in ["", "   ", "坏:名", "con", "名."] {
      assert!(rename_placement(&mut h.lib, "AN-A", "root", true, bad).is_err(), "应拒绝：{bad:?}");
    }
    assert!(rename_placement(&mut h.lib, "AN-A", "root", false, "幽灵").is_err(), "链接三元组不匹配");
    // 同文件夹撞名不追加序号（显示名放开，2026-08-18 用户定"护栏全拆"）
    add_png(&mut h, "AN-B", "另图.png");
    rename_placement(&mut h.lib, "AN-B", "root", true, "原图.png").unwrap();
    let b = h.lib.folders.items.iter().find(|p| p.asset == "AN-B").unwrap();
    assert_eq!(b.name, "原图.png", "显示名放开不唯一化");
  }

  /// 序列帧资产入库（3 帧 png 打包 frames/ + origin Placement 到 root）
  fn add_sequence(h: &mut Harness, id: &str, name: &str) {
    let src_dir = h.dir.path().join(format!("{id}-源"));
    fs::create_dir_all(&src_dir).unwrap();
    let mut frames = Vec::new();
    for n in 1..=3u32 {
      let p = src_dir.join(format!("{name}{n}.png"));
      image::RgbaImage::from_pixel(4, 4, image::Rgba([0, 0, 0, 255])).save(&p).unwrap();
      frames.push((p, n));
    }
    let meta =
      crate::services::sequence::build_sequence_asset(&h.lib.assets_dir, name, "png", 1, &frames, "sha256:seq", id, Source::Copy, None)
        .unwrap();
    h.lib.index.assets.insert(id.into(), meta);
    ops::add_placement(&mut h.lib, id.into(), "root".into(), format!("{name}.png"), true, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
  }

  #[test]
  fn rename_sequence_origin_renames_frames() {
    let mut h = make_lib();
    add_sequence(&mut h, "AN-S", "走路");
    let frames_dir = crate::core::paths::frames_dir(&h.lib.assets_dir, "AN-S");
    assert!(h.lib.info_dir("AN-S").join("走路_thumbnail.webp").is_file());

    let renames = rename_placement(&mut h.lib, "AN-S", "root", true, "跑步").unwrap();
    for f in ["跑步-1.png", "跑步-2.png", "跑步-3.png"] {
      assert!(frames_dir.join(f).is_file(), "{f} 应改词干");
    }
    assert!(!frames_dir.join("走路-1.png").exists(), "旧词干帧已改");
    let meta = &h.lib.index.assets["AN-S"];
    assert_eq!(meta.name, "跑步");
    assert_eq!(meta.origin_name, "跑步-1.png", "originName 跟随封面帧");
    assert_eq!(meta.thumb_frame.as_deref(), Some("跑步-1.png"));
    assert_eq!(meta.thumbnail.as_deref(), Some("跑步_thumbnail.webp"), "词干缩略图同步");
    assert!(h.lib.info_dir("AN-S").join("跑步_thumbnail.webp").is_file());
    assert_eq!(renames.len(), 4, "3 帧 + 缩略图四对磁盘改名");
    let disk: AssetMeta = read_json(&h.lib.info_dir("AN-S").join("metadata.json")).unwrap();
    assert_eq!(disk.origin_name, "跑步-1.png", "metadata.json 落盘");

    // 二次改名：重取词干靠序号尾解析，不依赖旧词干匹配
    rename_placement(&mut h.lib, "AN-S", "root", true, "跳跃").unwrap();
    assert!(frames_dir.join("跳跃-2.png").is_file());
    assert_eq!(h.lib.index.assets["AN-S"].thumb_frame.as_deref(), Some("跳跃-1.png"));
    assert_eq!(h.lib.index.assets["AN-S"].origin_name, "跳跃-1.png");
  }

  #[test]
  fn rename_anim_origin_keeps_unpacked_frames() {
    // 动图解包 frames/ 不是序列帧：origin 正常改名、解包帧目录不动
    let mut h = make_lib();
    let src = h.dir.path().join("动图.gif");
    fs::write(&src, b"fake").unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = ops::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:g", "AN-G", Source::Copy, None).unwrap();
    h.lib.index.assets.insert("AN-G".into(), meta);
    ops::add_placement(&mut h.lib, "AN-G".into(), "root".into(), "动图.gif".into(), true, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
    let unpack = crate::core::paths::frames_dir(&h.lib.assets_dir, "AN-G");
    fs::create_dir_all(&unpack).unwrap();
    fs::write(unpack.join("动图-00001.webp"), b"f").unwrap();

    rename_placement(&mut h.lib, "AN-G", "root", true, "新动图").unwrap();
    assert!(h.lib.info_dir("AN-G").join("新动图.gif").is_file(), "origin 正常改名");
    assert_eq!(h.lib.index.assets["AN-G"].origin_name, "新动图.gif");
    assert!(unpack.join("动图-00001.webp").is_file(), "解包帧目录不动");
  }

  #[test]
  fn duplicate_sequence_keeps_asset_name() {
    let mut h = make_lib();
    add_sequence(&mut h, "AN-S", "走路");
    let out = duplicate_placements(&mut h.lib, &[pref("AN-S", "root", true)], "root", &[None]).unwrap();
    let new_id = &out.new_ids[0];
    assert_eq!(h.lib.index.assets[new_id].name, "走路", "序列帧副本名不取 originName 词干");
    let new_frames = crate::core::paths::frames_dir(&h.lib.assets_dir, new_id);
    assert!(new_frames.join("走路-1.png").is_file(), "帧目录整体复制");
  }

  /// 副本落点（2026-08-20 用户定）：同文件夹紧随原件（新副本恒贴原件、旧副本顺后移），跨文件夹段首前插
  #[test]
  fn duplicate_inserts_after_source() {
    let mut h = make_lib();
    add_png(&mut h, "AN-A", "甲.png");
    add_png(&mut h, "AN-B", "乙.png");
    add_png(&mut h, "AN-C", "丙.png");
    // add_png 每条前插段首 → 初始 items 序：丙, 乙, 甲
    let out = duplicate_placements(&mut h.lib, &[pref("AN-B", "root", true)], "root", &[None]).unwrap();
    let names: Vec<&str> = h.lib.folders.items.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["丙.png", "乙.png", "乙-1.png", "甲.png"], "副本紧随原件");
    assert_eq!(h.lib.folders.items[2].asset, out.new_ids[0]);
    // 同原件再副本：新副本贴原件、旧副本后移
    duplicate_placements(&mut h.lib, &[pref("AN-B", "root", true)], "root", &[None]).unwrap();
    let names: Vec<&str> = h.lib.folders.items.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["丙.png", "乙.png", "乙-2.png", "乙-1.png", "甲.png"], "新副本恒贴原件");
    // 跨文件夹副本：段首前插（导入同语义）
    let fid = crate::services::import::new_folder_id();
    h.lib
      .folders
      .folders
      .push(crate::core::models::Folder { id: fid.clone(), name: "夹子".into(), parent: None, block: None, icon: None, kind: None, color: None, groups: None, order: 0 });
    h.lib.index.folders.insert(fid.clone(), h.lib.folders.folders.last().unwrap().clone());
    ops::add_placement(&mut h.lib, "AN-C".into(), fid.clone(), "丙.png".into(), true, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
    duplicate_placements(&mut h.lib, &[pref("AN-B", "root", true)], &fid, &[None]).unwrap();
    let seg: Vec<&str> = h.lib.folders.items.iter().filter(|p| p.folder == fid).map(|p| p.name.as_str()).collect();
    assert_eq!(seg, ["乙.png", "丙.png"], "跨文件夹副本段首前插");
  }

  /// 批量重命名（3.4）：逐条生效 + 撞名不唯一化（显示名放开）+ 失败项跳过不整体回滚
  #[test]
  fn rename_placements_batch_no_unique_and_partial_failure() {
    let mut h = make_lib();
    add_png(&mut h, "AN-A", "原图A.png");
    add_png(&mut h, "AN-B", "原图B.png");
    // 链接 Placement：批量改名仅改显示名
    ops::add_placement(&mut h.lib, "AN-A".into(), "root".into(), "快捷.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
    let item = |asset: &str, origin: bool, name: &str| RenameItem {
      asset_id: asset.into(),
      folder_id: "root".into(),
      origin,
      new_name: name.into(),
    };

    let out = rename_placements(
      &mut h.lib,
      &[
        item("AN-A", true, "同名.png"),   // origin：磁盘同步
        item("AN-A", false, "同名.png"),  // 链接：撞名不拦截（显示名放开）
        item("AN-幽灵", true, "幽灵.png"), // 陈旧项：失败收集继续
      ],
    )
    .unwrap();
    assert_eq!(out.renamed, 2);
    assert_eq!(out.failed.len(), 1);
    assert!(out.failed[0].reason.contains("不存在"), "失败原因人话");
    let names: Vec<&str> = h.lib.folders.items.iter().filter(|p| p.asset == "AN-A").map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["同名.png", "同名.png"], "撞名不追加序号");
    assert!(h.lib.info_dir("AN-A").join("同名.png").is_file(), "origin 磁盘同步改名");
    assert_eq!(out.renames.len(), 2, "origin + 词干缩略图两对磁盘改名（失败项无磁盘段）");
    assert_eq!(h.lib.folders.items.iter().find(|p| p.asset == "AN-B").unwrap().name, "原图B.png", "无关项不动");
  }
}
