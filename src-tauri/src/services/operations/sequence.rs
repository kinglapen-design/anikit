//! 序列帧包操作（5.2，PRD 04-序列帧 / 05-序列帧打包）：解散导入 / 删除帧 / 重排序号。
//! - 解散（dissolve）：每帧移动出库建档为独立素材，原包整体删除——危险操作**不进 undo**
//!   （命令层不调 undo.begin，警告弹窗明示不可撤销，与回收站彻底清除同级）；
//! - 删除帧（抽帧）：帧移 cache/seq-trash 暂存（会话级，开库清空 = 会话结束真删），
//!   磁盘改名对由命令层登记 DiskOp::Rename（undo 移回 / redo 移走，双向恒不撞名——
//!   暂存区与 frames/ 互斥），可撤销；
//! - 重排序号：剩余帧按当前文件名顺序重排为连续序号（词干-001..N，位宽只增不减），
//!   两段改名防撞（先临时名再目标名）；帧笔记 key 随映射迁移；磁盘改名对登记 DiskOp 可撤销。
//!
//! 播放口径：播放按 frames/ 文件名排序清单逐帧等长播放，序号不参与连续性推导——
//! 抽删帧序号断开不影响播放（PRD 04 包内视图文末口径）。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::json_io::write_json;
use crate::core::library::{Library, CACHE_DIR};
use crate::core::models::{new_asset_id, AssetMeta};
use crate::core::notes;
#[cfg(test)]
use crate::core::paths;
use crate::services::import::{self as ops, PendingFile, Source, ROOT_FOLDER_ID};
use crate::services::sequence::{frame_name, frames_dir_if_sequence};
use crate::services::thumbnail;

use super::{check_folder, finish, now};

/// 删帧暂存区子目录（cache/seq-trash/{assetId}/；会话级，开库清空清单已收录）
const SEQ_TRASH_DIR: &str = "seq-trash";

/// 序列帧校验 + 帧清单（按文件名排序，与播放口径同源）：返回 (info 目录, frames 目录, meta, 帧名列表)
fn sequence_context(lib: &Library, asset_id: &str) -> Result<(PathBuf, PathBuf, AssetMeta, Vec<String>), String> {
  let meta = lib
    .index
    .assets
    .get(asset_id)
    .cloned()
    .ok_or_else(|| format!("资源不存在：{asset_id}"))?;
  let info_dir = lib.info_dir(asset_id);
  let frames_dir = frames_dir_if_sequence(&lib.assets_dir, asset_id, &meta)
    .ok_or_else(|| format!("「{}」不是序列帧资源", meta.name))?;
  let mut names: Vec<String> = fs::read_dir(&frames_dir)
    .map_err(|e| format!("帧目录读取失败：{e}"))?
    .flatten()
    .filter(|e| e.path().is_file())
    .map(|e| e.file_name().to_string_lossy().into_owned())
    .collect();
  names.sort(); // 播放数据源同口径：按文件名排序，不推导序号连续性
  Ok((info_dir, frames_dir, meta, names))
}

/// 帧名解析词干 / 序号段位宽：`{词干}-{序号}.{扩展名}`（归一格式）；不合格式 None
fn parse_normalized(frame: &str) -> Option<(String, usize)> {
  let (stem, _ext) = frame.rsplit_once('.')?;
  let dash = stem.rfind('-')?;
  let digits = &stem[dash + 1..];
  if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
    return None;
  }
  Some((stem[..dash].to_string(), digits.len()))
}

/// 删帧暂存区路径（cache/seq-trash/{assetId}/，不存在则补建）
fn seq_trash_dir(lib: &Library, asset_id: &str) -> Result<PathBuf, String> {
  let dir = lib.root.join(CACHE_DIR).join(SEQ_TRASH_DIR).join(asset_id);
  fs::create_dir_all(&dir).map_err(|e| format!("暂存目录创建失败：{e}"))?;
  Ok(dir)
}

/// 资产的帧条缓存目录（cache/frames/{assetId}/）整体清除（重排 / 解散后旧帧名 key 全失效）
fn clear_strip_cache(lib: &Library, asset_id: &str) {
  let dir = lib.root.join(CACHE_DIR).join("frames").join(asset_id);
  if dir.is_dir() {
    let _ = fs::remove_dir_all(&dir); // 缓存清理失败不阻断业务（下趟预览按需重建）
  }
}

// ---------- 解散序列帧包导入 ----------

/// 解散结果：新素材 ID 清单 + 落点文件夹
#[derive(Debug, Default)]
pub struct DissolveOutcome {
  pub new_ids: Vec<String>,
  #[allow(dead_code)] // 命令层只回解散数量，落点文件夹暂仅测试断言（440 行钉落点语义）
  pub target_folder: String,
}

/// 解散序列帧包导入（危险操作，不进 undo）：每帧移动（Source::Move）出库建档为独立素材
/// （新 ID + 独立 info + 逐帧 sha256 + 缩略图，沿用导入单文件管线），落点 = 原包 origin
/// Placement 所在文件夹（无 origin 取第一条 Placement，无 Placement 兜底未分类），段首前插
/// 保持帧序、显示名唯一化；全部帧建档成功后删除原包全部 Placement、物理删除原包 info 目录
/// （逐帧笔记随之删除）。任一帧建档失败整体回滚（已移帧移回 frames/、已建 info 清除）。
pub fn dissolve_sequence(lib: &mut Library, asset_id: &str) -> Result<DissolveOutcome, String> {
  let (info_dir, frames_dir, meta, names) = sequence_context(lib, asset_id)?;
  if names.is_empty() {
    return Err(format!("「{}」的序列帧包为空，无帧可解散", meta.name));
  }
  let target = lib
    .folders
    .items
    .iter()
    .find(|p| p.asset == asset_id && p.origin)
    .or_else(|| lib.folders.items.iter().find(|p| p.asset == asset_id))
    .map(|p| p.folder.clone())
    .unwrap_or_else(|| ROOT_FOLDER_ID.to_string());
  check_folder(lib, &target)?;

  // 逐帧建档（帧移出库）；失败回滚已建项（帧移回 + 删新 info）
  let mut built: Vec<(String, String)> = Vec::new(); // (新 ID, 帧文件名)
  let mut metas: Vec<AssetMeta> = Vec::new();
  for name in &names {
    let frame_path = frames_dir.join(name);
    let result = (|| {
      let hash = ops::sha256_file(&frame_path).map_err(|e| format!("帧校验失败：{e}"))?;
      let new_id = new_asset_id();
      let pf = PendingFile { path: frame_path.clone(), chain: Vec::new() };
      let meta = ops::build_asset_with_id(&lib.assets_dir, &pf, &hash, &new_id, Source::Move, None)?;
      Ok::<(String, AssetMeta), String>((new_id, meta))
    })();
    match result {
      Ok((new_id, meta)) => {
        built.push((new_id, name.clone()));
        metas.push(meta);
      }
      Err(e) => {
        rollback_dissolve(lib, &frames_dir, &built);
        return Err(format!("帧「{name}」解散失败：{e}（已回滚，序列帧包未动）"));
      }
    }
  }

  // 登记：索引 insert + 段首前插 Placement（保帧序）+ 删除原包全部 Placement + 落盘
  let mut front_anchor: HashMap<String, usize> = HashMap::new();
  for ((new_id, frame), meta) in built.iter().zip(metas) {
    lib.index.assets.insert(new_id.clone(), meta);
    ops::add_placement(lib, new_id.clone(), target.clone(), frame.clone(), true, &mut front_anchor);
  }
  lib.folders.items.retain(|p| p.asset != asset_id);
  lib.index.assets.remove(asset_id);
  if let Err(e) = finish(lib) {
    rollback_dissolve(lib, &frames_dir, &built);
    return Err(format!("解散落盘失败：{e}（已回滚，序列帧包未动）"));
  }

  // 原包 info 目录物理删除（帧已移空；metadata / 缩略图 / notes.json 随之删除）+ 缓存清理
  if let Err(e) = fs::remove_dir_all(&info_dir) {
    log::error!("解散后原包 info 删除失败（{}）：{e}", info_dir.display());
  }
  clear_strip_cache(lib, asset_id);
  let trash = lib.root.join(CACHE_DIR).join(SEQ_TRASH_DIR).join(asset_id);
  if trash.is_dir() {
    let _ = fs::remove_dir_all(&trash); // 该包历史删帧暂存真删（防 undo 穿越命中已删资产）
  }
  log::info!("解散序列帧包「{}」：{} 帧 → 独立素材（落点 {target}）", meta.name, built.len());
  Ok(DissolveOutcome { new_ids: built.into_iter().map(|(id, _)| id).collect(), target_folder: target })
}

/// 解散回滚：已移出的帧移回 frames/、已建 info 目录删除、索引 / Placement 清理
fn rollback_dissolve(lib: &mut Library, frames_dir: &Path, built: &[(String, String)]) {
  for (new_id, frame) in built {
    let new_info = lib.info_dir(new_id);
    let moved = new_info.join(frame);
    if moved.is_file() {
      if let Err(e) = fs::rename(&moved, frames_dir.join(frame)) {
        log::error!("解散回滚帧移回失败（{frame}）：{e}");
      }
    }
    if let Err(e) = fs::remove_dir_all(&new_info) {
      log::error!("解散回滚清理失败（{}）：{e}", new_info.display());
    }
    lib.index.assets.remove(new_id);
  }
  let ids: Vec<&str> = built.iter().map(|(id, _)| id.as_str()).collect();
  lib.folders.items.retain(|p| !ids.contains(&p.asset.as_str()));
  lib.index.rebuild_placements(&lib.folders.items);
}

// ---------- 删除帧（抽帧） ----------

/// 删帧结果：磁盘移动对（帧原路径 → 暂存路径，命令层登记 DiskOp 用）+ 实际删除数
#[derive(Debug, Default)]
pub struct DeleteFramesOutcome {
  pub moves: Vec<(PathBuf, PathBuf)>,
  pub deleted: usize,
}

/// 删除序列帧包的帧（抽帧调节奏）：帧移 cache/seq-trash 暂存（命令层登记 DiskOp 可撤销，
/// 会话结束随开库清清真删）；封面帧 / originName 指向帧被删 → 改指剩余首帧并重建缩略图；
/// 帧笔记同步清除；frameCount / modifiedAt 更新落盘。**至少保留 1 帧**（删空报错）。
/// 陈旧帧名（不在清单内）跳过容错（同 delete_placements 陈旧三元组口径）。
pub fn delete_frames(lib: &mut Library, asset_id: &str, frame_names: &[String]) -> Result<DeleteFramesOutcome, String> {
  let (info_dir, frames_dir, mut meta, names) = sequence_context(lib, asset_id)?;
  let targets: Vec<&String> = frame_names.iter().filter(|n| names.contains(n)).collect();
  if targets.is_empty() {
    return Ok(DeleteFramesOutcome::default());
  }
  let remaining = names.len() - targets.len();
  if remaining < 1 {
    return Err(format!("序列帧包至少保留 1 帧（「{}」共 {} 帧，无法删除 {} 帧）", meta.name, names.len(), targets.len()));
  }

  let trash = seq_trash_dir(lib, asset_id)?;
  let mut out = DeleteFramesOutcome::default();
  let mut notes_remaining = 0usize;
  for name in &targets {
    let from = frames_dir.join(name);
    let to = trash.join(name);
    fs::rename(&from, &to).map_err(|e| format!("帧「{name}」移出失败：{e}"))?;
    out.moves.push((from, to));
    // 帧条小图缓存同步清（按帧名 keyed，已删帧不留陈旧）
    let strip = lib.root.join(CACHE_DIR).join("frames").join(asset_id).join(format!("{name}.webp"));
    if strip.is_file() {
      let _ = fs::remove_file(&strip);
    }
    // 帧笔记同步清除（key = 帧名；notes.json 由命令层 record_file 快照，undo 自动恢复）
    notes_remaining = notes::remove(&info_dir, name).map_err(|e| e.to_string())?.len();
  }
  out.deleted = targets.len();

  // metadata：frameCount / 封面帧与 originName 指向（被删则改指剩余首帧）/ noteCount 冗余 / modifiedAt
  let survivors: Vec<&String> = names.iter().filter(|n| !targets.contains(n)).collect();
  let first = survivors[0].clone();
  let cover_deleted = targets.iter().any(|n| Some(*n) == meta.thumb_frame.as_ref());
  if cover_deleted {
    meta.thumb_frame = Some(first.clone());
  }
  if targets.contains(&&meta.origin_name) {
    meta.origin_name = first.clone();
  }
  meta.frame_count = Some(remaining as u32);
  // noteCount 冗余同步（0 记 None 不写字段，同 commands/notes::apply_set 口径）——
  // 笔记文件不存在时 notes::remove 静默返回空，notes_remaining 保持 0 即 None
  meta.note_count = (notes_remaining > 0).then_some(notes_remaining as u32);
  meta.modified_at = now();
  if cover_deleted {
    // 封面帧被删：按新封面帧重建缩略图（同词干覆盖写）
    if let Some(thumb) = thumbnail::generate(&info_dir, &frames_dir.join(&first), &meta.name) {
      meta.thumbnail = Some(thumb.name);
      meta.thumb_width = Some(thumb.width);
      meta.thumb_height = Some(thumb.height);
      meta.colors = thumb.colors;
    }
  }
  write_json(&info_dir.join("metadata.json"), &meta).map_err(|e| e.to_string())?;
  lib.index.assets.insert(asset_id.to_string(), meta.clone());
  log::info!("删除序列帧「{}」{} 帧（剩 {}）", meta.name, out.deleted, remaining);
  Ok(out)
}

// ---------- 重排序号 ----------

/// 重排结果：undo / redo 磁盘操作序列（已按 record_disk_op 登记顺序折算——
/// apply_disk 逆序执行，故登记序 = 期望执行序的反序）+ 实际重排数（0 = 已连续无操作）
#[derive(Debug, Default)]
pub struct RenumberOutcome {
  /// 撤销序列：逆序执行 = [新名→临时名（升序）..., 临时名→旧名（升序）...] 两段还原
  pub undo_ops: Vec<(PathBuf, PathBuf)>,
  /// 重做序列：逆序执行 = [旧名→临时名（升序）..., 临时名→新名（升序）...] 业务重放
  pub redo_ops: Vec<(PathBuf, PathBuf)>,
  pub renamed: usize,
}

/// 重排序号：剩余帧按当前文件名顺序重排为连续序号 `词干-001..N`（词干 = meta.name，
/// 位宽 = max(现位宽, N 位数)，只增不减）。两段改名防撞（先 `~anikit-renumber` 临时名再目标名）；
/// 帧笔记 key 随旧 → 新映射迁移（保留原文本与时间）；封面帧 / originName 跟随；
/// 帧条缓存整清（旧帧名 key 全失效，预览按需重建）。已全部连续 = 无操作返回 0。
/// disk_ops 供命令层登记 DiskOp::Rename（undo = 逆序移回旧名 / redo = 逆序移到新名，
/// 两段式保证双向执行段内目标位恒腾空，不撞名）。
pub fn renumber_frames(lib: &mut Library, asset_id: &str) -> Result<RenumberOutcome, String> {
  let (info_dir, frames_dir, mut meta, names) = sequence_context(lib, asset_id)?;
  if names.len() < 2 {
    return Ok(RenumberOutcome::default());
  }
  // 词干 = meta.name（rename_frames 保证与帧名词干一致）；位宽 = max(各帧序号段宽, N 位数)
  let stem = if meta.name.is_empty() {
    names.first().and_then(|n| parse_normalized(n).map(|(s, _)| s)).unwrap_or_default()
  } else {
    meta.name.clone()
  };
  if stem.is_empty() {
    return Err(format!("「{}」帧名不合归一格式，无法重排", meta.origin_name));
  }
  let digits = (names.len() as u32).to_string().len();
  let width = names
    .iter()
    .filter_map(|n| parse_normalized(n).map(|(_, w)| w))
    .max()
    .unwrap_or(1)
    .max(digits);
  let ext = meta.ext.clone();

  // 旧 → 新映射（跳过同名无操作项）
  let mapping: Vec<(String, String)> = names
    .iter()
    .enumerate()
    .map(|(i, old)| (old.clone(), frame_name(&stem, (i + 1) as u32, width, &ext)))
    .filter(|(old, new)| old != new)
    .collect();
  if mapping.is_empty() {
    return Ok(RenumberOutcome::default());
  }

  // 两段改名：先全部改临时名（腾空旧名集合），再改目标名；中途失败尽力改回
  let tmp_of = |new: &str| format!("~anikit-renumber-{new}");
  let mut staged: Vec<(String, String)> = Vec::new(); // (旧名, 临时名)
  for (old, new) in &mapping {
    let tmp = tmp_of(new);
    if let Err(e) = fs::rename(frames_dir.join(old), frames_dir.join(&tmp)) {
      for (o, t) in staged.iter().rev() {
        let _ = fs::rename(frames_dir.join(t), frames_dir.join(o));
      }
      return Err(format!("重排改名失败（{old}）：{e}（已回滚）"));
    }
    staged.push((old.clone(), tmp));
  }
  // 业务执行序列 E = [旧名→临时名（升序）..., 临时名→新名（升序）...]（两段式段内目标位恒腾空）
  let mut exec: Vec<(PathBuf, PathBuf)> = Vec::new();
  for (old, tmp) in &staged {
    exec.push((frames_dir.join(old), frames_dir.join(tmp)));
  }
  let mut out = RenumberOutcome::default();
  for (new, tmp) in mapping.iter().map(|(_, n)| n).zip(staged.iter().map(|(_, t)| t)) {
    fs::rename(frames_dir.join(tmp), frames_dir.join(new)).map_err(|e| format!("重排落名失败（{new}）：{e}"))?;
    exec.push((frames_dir.join(tmp), frames_dir.join(new)));
  }
  out.renamed = mapping.len();
  // redo 序列 = E 的反序（逆序执行重放 E）；undo 序列 = U 的反序，
  // U = [新名→临时名（升序）..., 临时名→旧名（升序）...]（同两段式安全论证）
  out.redo_ops = exec.iter().rev().cloned().collect();
  let mut undo_exec: Vec<(PathBuf, PathBuf)> = Vec::new();
  for (new, tmp) in mapping.iter().map(|(_, n)| n).zip(staged.iter().map(|(_, t)| t)) {
    undo_exec.push((frames_dir.join(new), frames_dir.join(tmp)));
  }
  for (old, tmp) in &staged {
    undo_exec.push((frames_dir.join(tmp), frames_dir.join(old)));
  }
  out.undo_ops = undo_exec.iter().rev().cloned().collect();

  // 帧笔记 key 迁移（旧帧名 → 新帧名；replace_all 保原文本与时间）
  let map: HashMap<&str, &str> = mapping.iter().map(|(o, n)| (o.as_str(), n.as_str())).collect();
  let mut notes_list = notes::load(&info_dir).map_err(|e| e.to_string())?;
  let mut migrated = false;
  for note in notes_list.iter_mut() {
    if let Some(new) = map.get(note.key.as_str()) {
      note.key = new.to_string();
      migrated = true;
    }
  }
  if migrated {
    notes::replace_all(&info_dir, notes_list).map_err(|e| e.to_string())?;
  }

  // metadata：封面帧 / originName 跟随映射；modifiedAt 更新落盘
  if let Some(tf) = meta.thumb_frame.clone() {
    if let Some(new) = map.get(tf.as_str()) {
      meta.thumb_frame = Some(new.to_string());
    }
  }
  if let Some(new) = map.get(meta.origin_name.as_str()) {
    meta.origin_name = new.to_string();
  }
  meta.modified_at = now();
  write_json(&info_dir.join("metadata.json"), &meta).map_err(|e| e.to_string())?;
  lib.index.assets.insert(asset_id.to_string(), meta.clone());

  clear_strip_cache(lib, asset_id);
  log::info!("重排序号「{}」：{} 帧改名（共 {} 帧）", meta.name, out.renamed, names.len());
  Ok(out)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library;
  use std::collections::HashMap;

  struct Harness {
    dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { dir, lib }
  }

  fn make_png(path: &Path, w: u32, h: u32) {
    image::RgbaImage::from_pixel(w, h, image::Rgba([200, 30, 30, 255])).save(path).unwrap();
  }

  /// 造序列帧包（走路_1..=count.png 入库归一 走路-1..count，位宽 1）+ origin Placement 到 root
  fn add_sequence(h: &mut Harness, id: &str, count: u32) -> AssetMeta {
    add_sequence_wide(h, id, count, 1)
  }

  /// 位宽可控版（重排测试需要位宽 3 的归一名）
  fn add_sequence_wide(h: &mut Harness, id: &str, count: u32, width: usize) -> AssetMeta {
    let src = h.dir.path().join(format!("{id}-源"));
    fs::create_dir_all(&src).unwrap();
    let frames: Vec<(PathBuf, u32)> = (1..=count)
      .map(|n| {
        let p = src.join(format!("走路_{n}.png"));
        make_png(&p, 64, 48);
        (p, n)
      })
      .collect();
    let meta = crate::services::sequence::build_sequence_asset(
      &h.lib.assets_dir,
      "走路",
      "png",
      width,
      &frames,
      "sha256:seq",
      id,
      Source::Copy,
      None,
    )
    .unwrap();
    h.lib.index.assets.insert(id.into(), meta.clone());
    ops::add_placement(&mut h.lib, id.into(), ROOT_FOLDER_ID.into(), "走路.png".into(), true, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
    meta
  }

  /// 磁盘帧名清单（排序）
  fn frame_names(h: &Harness, id: &str) -> Vec<String> {
    let dir = paths::frames_dir(&h.lib.assets_dir, id);
    let mut names: Vec<String> =
      fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
  }

  #[test]
  fn dissolve_moves_frames_into_standalone_assets() {
    let mut h = make_lib();
    add_sequence(&mut h, "AN-S", 3);
    let out = dissolve_sequence(&mut h.lib, "AN-S").unwrap();
    assert_eq!(out.new_ids.len(), 3);
    assert_eq!(out.target_folder, ROOT_FOLDER_ID);

    // 原包清除：placements / 索引 / info 目录
    assert!(!h.lib.index.assets.contains_key("AN-S"));
    assert!(h.lib.folders.items.iter().all(|p| p.asset != "AN-S"));
    assert!(!h.lib.info_dir("AN-S").exists(), "原包 info 应物理删除");

    // 新素材：独立 info + 帧移入 + hash 实算 + placements 段首前插保帧序
    let names: Vec<&str> = h.lib.folders.items.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["走路-1.png", "走路-2.png", "走路-3.png"], "段首前插保持帧序");
    for (i, id) in out.new_ids.iter().enumerate() {
      let info = h.lib.info_dir(id);
      let frame = format!("走路-{}.png", i + 1);
      assert!(info.join(&frame).is_file(), "{id} 帧应移入独立 info");
      let meta = &h.lib.index.assets[id];
      assert_eq!(meta.origin_name, frame);
      assert!(meta.hash.starts_with("sha256:"));
      assert!(meta.frame_count.is_none(), "单帧素材无 frameCount");
      assert!(meta.thumbnail.is_some(), "独立素材应有自己的缩略图");
    }
    // 帧不残留在库内旧 frames 目录（随 info 整体删除）
  }

  #[test]
  fn dissolve_rejects_non_sequence() {
    let mut h = make_lib();
    let src = h.dir.path().join("单张.png");
    make_png(&src, 10, 10);
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = ops::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", "AN-P", Source::Copy, None).unwrap();
    h.lib.index.assets.insert("AN-P".into(), meta);
    let err = dissolve_sequence(&mut h.lib, "AN-P").unwrap_err();
    assert!(err.contains("不是序列帧"), "{err}");
  }

  #[test]
  fn delete_frames_moves_to_trash_and_updates_meta() {
    let mut h = make_lib();
    add_sequence(&mut h, "AN-S", 4);
    // 给 走路-2.png 写一条笔记
    let info = h.lib.info_dir("AN-S");
    notes::upsert(&info, "走路-2.png", "这一帧要修").unwrap();
    let mut meta = h.lib.index.assets["AN-S"].clone();
    meta.note_count = Some(1);
    write_json(&info.join("metadata.json"), &meta).unwrap();
    h.lib.index.assets.insert("AN-S".into(), meta);

    let out = delete_frames(&mut h.lib, "AN-S", &["走路-2.png".to_string()]).unwrap();
    assert_eq!(out.deleted, 1);
    assert_eq!(out.moves.len(), 1);
    // 帧移暂存区，frames/ 不再含
    assert!(!paths::frames_dir(&h.lib.assets_dir, "AN-S").join("走路-2.png").exists());
    assert!(out.moves[0].1.is_file(), "暂存区应有被删帧（撤销支撑）");
    assert_eq!(frame_names(&h, "AN-S"), ["走路-1.png", "走路-3.png", "走路-4.png"]);
    // metadata：frameCount / noteCount / 封面帧不动（删的不是封面）
    let meta = &h.lib.index.assets["AN-S"];
    assert_eq!(meta.frame_count, Some(3));
    assert_eq!(meta.note_count, None, "唯一笔记随帧清除，0 记 None");
    assert_eq!(meta.thumb_frame.as_deref(), Some("走路-1.png"));
    // notes.json 同步
    assert!(notes::load(&info).unwrap().is_empty());
  }

  #[test]
  fn delete_frames_cover_retargets_and_rebuilds_thumb() {
    let mut h = make_lib();
    add_sequence(&mut h, "AN-S", 3);
    let out = delete_frames(&mut h.lib, "AN-S", &["走路-1.png".to_string()]).unwrap();
    assert_eq!(out.deleted, 1);
    let meta = &h.lib.index.assets["AN-S"];
    assert_eq!(meta.thumb_frame.as_deref(), Some("走路-2.png"), "封面改指剩余首帧");
    assert_eq!(meta.origin_name, "走路-2.png", "originName 跟随");
    // metadata.json 落盘
    let disk: AssetMeta = read_json(&h.lib.info_dir("AN-S").join("metadata.json")).unwrap();
    assert_eq!(disk.frame_count, Some(2));
  }

  #[test]
  fn delete_frames_keeps_at_least_one() {
    let mut h = make_lib();
    add_sequence(&mut h, "AN-S", 2);
    // 位宽 1 的 2 帧：删 2 帧 = 删空报错
    let err = delete_frames(&mut h.lib, "AN-S", &["走路-1.png".to_string(), "走路-2.png".to_string()]).unwrap_err();
    assert!(err.contains("至少保留 1 帧"), "{err}");
    assert_eq!(frame_names(&h, "AN-S").len(), 2, "删空被拒，帧不动");
  }

  #[test]
  fn renumber_fills_gaps_and_migrates_notes() {
    let mut h = make_lib();
    add_sequence_wide(&mut h, "AN-S", 3, 3); // 走路-001..003
                                             // 造断号：002 → 005（清单 001/003/005）
    let dir = paths::frames_dir(&h.lib.assets_dir, "AN-S");
    fs::rename(dir.join("走路-002.png"), dir.join("走路-005.png")).unwrap();
    // 笔记挂在将改名的 003 上 + 封面帧指 003
    let info = h.lib.info_dir("AN-S");
    notes::upsert(&info, "走路-003.png", "蓄力帧").unwrap();
    let mut meta = h.lib.index.assets["AN-S"].clone();
    meta.thumb_frame = Some("走路-003.png".into());
    meta.origin_name = "走路-003.png".into();
    h.lib.index.assets.insert("AN-S".into(), meta.clone());
    write_json(&info.join("metadata.json"), &meta).unwrap();

    let out = renumber_frames(&mut h.lib, "AN-S").unwrap();
    assert_eq!(out.renamed, 2, "003→002 / 005→003 两项改名");
    assert_eq!(frame_names(&h, "AN-S"), ["走路-001.png", "走路-002.png", "走路-003.png"]);
    // 笔记 key 迁移（时间与文本保留）
    let list = notes::load(&info).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].key, "走路-002.png");
    assert_eq!(list[0].text, "蓄力帧");
    // 封面帧 / originName 跟随
    let meta = &h.lib.index.assets["AN-S"];
    assert_eq!(meta.thumb_frame.as_deref(), Some("走路-002.png"));
    assert_eq!(meta.origin_name, "走路-002.png");
  }

  #[test]
  fn renumber_already_continuous_is_noop() {
    let mut h = make_lib();
    add_sequence_wide(&mut h, "AN-S", 3, 3);
    let before = frame_names(&h, "AN-S");
    let out = renumber_frames(&mut h.lib, "AN-S").unwrap();
    assert_eq!(out.renamed, 0);
    assert!(out.undo_ops.is_empty() && out.redo_ops.is_empty());
    assert_eq!(frame_names(&h, "AN-S"), before, "已连续无磁盘变化");
  }

  /// undo / redo 磁盘序列安全性：apply_disk 逆序语义下，undo 还原断号、redo 重放连续，
  /// 全程不撞名（含 0 起序号场景——单向 rename 序列会撞名的坑，两段式登记免疫）
  #[test]
  fn renumber_undo_redo_sequences_are_two_phase_safe() {
    let mut h = make_lib();
    add_sequence_wide(&mut h, "AN-S", 3, 3);
    // 造 0 起断号：001→000、002→001（清单 000/001/003——old_i < new_i 的撞名高危形态）
    let dir = paths::frames_dir(&h.lib.assets_dir, "AN-S");
    fs::rename(dir.join("走路-002.png"), dir.join("走路-001.tmp")).unwrap();
    fs::rename(dir.join("走路-001.png"), dir.join("走路-000.png")).unwrap();
    fs::rename(dir.join("走路-001.tmp"), dir.join("走路-001.png")).unwrap();
    assert_eq!(frame_names(&h, "AN-S"), ["走路-000.png", "走路-001.png", "走路-003.png"]);

    let out = renumber_frames(&mut h.lib, "AN-S").unwrap();
    assert_eq!(frame_names(&h, "AN-S"), ["走路-001.png", "走路-002.png", "走路-003.png"], "业务正向连续");

    // 模拟 apply_disk：逆序执行 undo 序列 → 还原断号
    let apply = |ops: &[(PathBuf, PathBuf)]| {
      for (from, to) in ops.iter().rev() {
        if from.exists() {
          fs::rename(from, to).unwrap();
        }
      }
    };
    apply(&out.undo_ops);
    assert_eq!(frame_names(&h, "AN-S"), ["走路-000.png", "走路-001.png", "走路-003.png"], "undo 还原断号");
    apply(&out.redo_ops);
    assert_eq!(frame_names(&h, "AN-S"), ["走路-001.png", "走路-002.png", "走路-003.png"], "redo 重放连续");
  }
}
