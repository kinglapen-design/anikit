//! 档案文件夹服务（2026-08-15 档案子系统一轮骨架，PRD 02_文件整理 03 / 03_数据链接 06）：
//! 档案 = 文件夹 + 详细数据——folders.json 里 Folder.kind = "archive" 标记形态，
//! 头像 / 徽标 / 属性 / 描述存库级 archives.json（core::library::save_archives 单一写点）。
//! 层级约束：档案内不允许子级文件夹——护栏在 operations::folder（create_folder / move_folders），
//! 本模块负责新建档案与两种转换（文件夹 ⇄ 档案）。

use crate::core::library::{self, Library};
use crate::core::models::{ArchiveData, Folder, ARCHIVE_KIND};
use crate::services::operations;

/// 转换结果（两段式：有子级须用户确认剔除层级，PRD 02_文件整理 03）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertOutcome {
  /// 有子文件夹且未确认：不动任何数据，待用户确认后以 confirmed = true 重调
  NeedConfirm,
  /// 已转换（含幂等：已是档案直接返回）
  Done,
}

/// 新建档案：复用新建文件夹全链路（命名校验 / 插入定位 / 区块继承，operations::create_folder
/// 内部已含档案父级护栏），落盘后补 kind 标记 + 建 archives.json 记录（created_at = 当前时间）
pub fn create_archive(
  lib: &mut Library,
  parent: Option<&str>,
  name: &str,
  after_folder_id: Option<&str>,
  block_id: Option<&str>,
) -> Result<Folder, String> {
  let folder = operations::create_folder(lib, parent, name, after_folder_id, block_id)?;
  set_kind(lib, &folder.id, Some(ARCHIVE_KIND));
  ensure_archive_record(lib, &folder.id);
  library::save_folders(lib).map_err(|e| e.to_string())?;
  library::save_archives(lib).map_err(|e| e.to_string())?;
  log::info!("新建档案：{}（{:?} 下）", folder.name, folder.parent);
  Ok(lib.index.folders[&folder.id].clone())
}

/// 档案记录存在性保障（created_at 只在新建 / 转换当次写入；已有记录不动——
/// 解散重建场景下"创建时间"以最近一次转换为准）
fn ensure_archive_record(lib: &mut Library, folder_id: &str) {
  if lib.archives.archives.iter().any(|a| a.folder_id == folder_id) {
    return;
  }
  let mut rec = ArchiveData::new(folder_id);
  rec.created_at = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
  lib.archives.archives.push(rec);
}

/// 文件夹转换为档案：已是档案幂等返回 Done；有子文件夹且未确认返回 NeedConfirm（不动任何数据）；
/// 确认后剔除层级——全部后代子文件夹的 Placement 平移到该档案（order 追加在后重排）、
/// 子文件夹节点删除、子级本身是档案的 archives 记录一并清除——然后置 kind 标记
pub fn convert_to_archive(lib: &mut Library, folder_id: &str, confirmed: bool) -> Result<ConvertOutcome, String> {
  if !lib.index.folders.contains_key(folder_id) {
    return Err("目标文件夹不存在，可能已被删除".to_string());
  }
  if lib.index.folders[folder_id].kind.as_deref() == Some(ARCHIVE_KIND) {
    return Ok(ConvertOutcome::Done); // 幂等：已是档案
  }
  let descendants = descendant_ids(lib, folder_id);
  if !descendants.is_empty() && !confirmed {
    return Ok(ConvertOutcome::NeedConfirm);
  }
  let archives_changed = if descendants.is_empty() { false } else { flatten(lib, folder_id, &descendants) };
  set_kind(lib, folder_id, Some(ARCHIVE_KIND));
  // 转换当次建档记录（created_at = 当前时间；幂等已是档案已在上方提前返回，不会覆盖）
  let record_created = !lib.archives.archives.iter().any(|a| a.folder_id == folder_id);
  ensure_archive_record(lib, folder_id);
  operations::sync_folder_index(lib);
  lib.index.rebuild_placements(&lib.folders.items);
  library::save_folders(lib).map_err(|e| e.to_string())?;
  if archives_changed || record_created {
    library::save_archives(lib).map_err(|e| e.to_string())?;
  }
  log::info!("文件夹转换为档案：{folder_id}（解散 {} 个子文件夹）", descendants.len());
  Ok(ConvertOutcome::Done)
}

/// 档案解散为普通文件夹：摘 kind 标记 + 清 archives.json 中该档案的记录（无记录 = 无操作）
pub fn convert_to_folder(lib: &mut Library, folder_id: &str) -> Result<(), String> {
  if !lib.index.folders.contains_key(folder_id) {
    return Err("目标文件夹不存在，可能已被删除".to_string());
  }
  set_kind(lib, folder_id, None);
  let before = lib.archives.archives.len();
  lib.archives.archives.retain(|a| a.folder_id != folder_id);
  library::save_folders(lib).map_err(|e| e.to_string())?;
  if lib.archives.archives.len() != before {
    library::save_archives(lib).map_err(|e| e.to_string())?;
  }
  log::info!("档案解散为文件夹：{folder_id}");
  Ok(())
}

/// 全部档案数据（克隆全量，量小）
pub fn list_archives(lib: &Library) -> Vec<ArchiveData> {
  lib.archives.archives.clone()
}

/// 档案封面图绝对路径（avatar.file → 库根 covers/；文件缺失视为无封面，回退子项首图）
pub fn cover_file_path(lib: &Library, folder_id: &str) -> Option<String> {
  let avatar = lib.archives.archives.iter().find(|a| a.folder_id == folder_id)?.avatar.as_ref()?;
  let p = lib.root.join(library::COVERS_DIR).join(&avatar.file);
  p.is_file().then(|| p.to_string_lossy().into_owned())
}

/// 封面设置入参（43 稿「更换封面 - 从本地磁盘中选择」）：本地图片路径 + 取景框参数
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverInput {
  pub source_path: String,
  pub crop: crate::core::models::Crop,
}

/// 封面烘培输出规格（43 稿封面框 218:168 = 13:10，短边取缩略图同档 320 → 416×320）
pub const COVER_OUT_W: u32 = 416;
pub const COVER_OUT_H: u32 = 320;

/// 封面设置（43 稿，6.1）：徽标三角语义（None = 不动 / Some(None) = 清除 / Some(Some) = 设置）+
/// 封面（None = 不动 / Some = 选图按取景参数烘培新封面）。记录懒建；返回是否有改动。
/// 封面文件带时间戳命名不覆盖写：undo 回滚 archives.json 后旧封面文件仍在即恢复显示；
/// 旧文件留盘（孤儿清理记 09 待办，不随换封面即删——即删会破坏 undo 恢复）
pub fn set_archive_cover_settings(
  lib: &mut Library,
  folder_id: &str,
  badge: Option<Option<crate::core::models::Badge>>,
  cover: Option<CoverInput>,
) -> Result<bool, String> {
  if !lib.index.folders.contains_key(folder_id) {
    return Err("目标档案不存在，可能已被删除".to_string());
  }
  let mut changed = false;
  let mut new_avatar: Option<crate::core::models::Avatar> = None;
  if let Some(input) = cover {
    let file = bake_cover(lib, folder_id, &input.source_path, input.crop)?;
    new_avatar = Some(crate::core::models::Avatar { file, crop: Some(input.crop) });
  }
  if lib.archives.archives.iter().all(|a| a.folder_id == folder_id) {
    lib.archives.archives.push(ArchiveData::new(folder_id));
  }
  let rec = lib.archives.archives.iter_mut().find(|a| a.folder_id == folder_id).expect("懒建后必有");
  if let Some(b) = badge {
    if rec.badge != b {
      rec.badge = b;
      changed = true;
    }
  }
  if let Some(avatar) = new_avatar {
    rec.avatar = Some(avatar);
    changed = true;
  }
  if changed {
    library::save_archives(lib).map_err(|e| e.to_string())?;
    log::info!("档案封面设置：{folder_id}");
  }
  Ok(changed)
}

/// 封面烘培：源图按内容魔数解码（假扩展名防护，踩坑 052）→ 按取景参数裁切缩放 →
/// webp 存库根 covers/（编码失败退 PNG）。返回封面文件名
fn bake_cover(lib: &Library, folder_id: &str, source_path: &str, crop: crate::core::models::Crop) -> Result<String, String> {
  use crate::services::thumbnail;
  let source = std::path::PathBuf::from(source_path);
  if !source.is_file() {
    return Err("所选文件不存在".to_string());
  }
  let img = thumbnail::decode_by_signature(&source).ok_or_else(|| "该文件无法用作封面（不支持的格式）".to_string())?;
  let (iw, ih) = (img.width() as f64, img.height() as f64);
  if iw < 1.0 || ih < 1.0 {
    return Err("该文件无法用作封面（图片尺寸无效）".to_string());
  }
  // 取景模型（与前端预览同一约定）：scale ≥ 1，1 = 图片 cover 铺满框；
  // x / y = 图片中心相对框中心的偏移（框宽 / 框高的分数，右下为正）——
  // 源图可见窗口中心 = 图中心 − 偏移/缩放（视图右偏 = 看到图的左侧），窗口 clamp 在图内（不露底）
  let scale = crop.scale.max(1.0);
  let z = (COVER_OUT_W as f64 / iw).max(COVER_OUT_H as f64 / ih) * scale; // 输出像素 / 源像素
  let (sw, sh) = (COVER_OUT_W as f64 / z, COVER_OUT_H as f64 / z); // 源图可见窗口尺寸
  let cx = iw / 2.0 - (crop.x * COVER_OUT_W as f64) / z;
  let cy = ih / 2.0 - (crop.y * COVER_OUT_H as f64) / z;
  let left = (cx - sw / 2.0).clamp(0.0, (iw - sw).max(0.0)).round() as u32;
  let top = (cy - sh / 2.0).clamp(0.0, (ih - sh).max(0.0)).round() as u32;
  let (swu, shu) = ((sw.round() as u32).max(1).min(img.width()), (sh.round() as u32).max(1).min(img.height()));
  let out = img
    .crop_imm(left, top, swu, shu)
    .resize_exact(COVER_OUT_W, COVER_OUT_H, image::imageops::FilterType::Triangle);
  let dir = lib.root.join(library::COVERS_DIR);
  std::fs::create_dir_all(&dir).map_err(|e| format!("封面目录创建失败：{e}"))?;
  let stamp = chrono::Local::now().format("%Y%m%d%H%M%S%3f");
  let webp_name = format!("{folder_id}-{stamp}.webp");
  if thumbnail::save_webp(&out, &dir.join(&webp_name)).is_ok() {
    return Ok(webp_name);
  }
  let png_name = format!("{folder_id}-{stamp}.png");
  out.save(dir.join(&png_name)).map_err(|e| format!("封面保存失败：{e}"))?;
  Ok(png_name)
}


/// 写 Folder.kind（内存：folders.folders 与索引同步；落盘由调用方负责）
fn set_kind(lib: &mut Library, folder_id: &str, kind: Option<&str>) {
  if let Some(f) = lib.folders.folders.iter_mut().find(|f| f.id == folder_id) {
    f.kind = kind.map(|s| s.to_string());
  }
  if let Some(f) = lib.index.folders.get_mut(folder_id) {
    f.kind = kind.map(|s| s.to_string());
  }
}

/// 全部后代文件夹 ID：BFS 逐层展开、每层按 order 排序（Placement 平移顺序 = 展示顺序，确定性）
fn descendant_ids(lib: &Library, folder_id: &str) -> Vec<String> {
  let mut out = Vec::new();
  let mut frontier: Vec<String> = vec![folder_id.to_string()];
  while !frontier.is_empty() {
    let mut children: Vec<&Folder> =
      lib.folders.folders.iter().filter(|f| f.parent.as_ref().is_some_and(|p| frontier.contains(p))).collect();
    children.sort_by_key(|f| f.order);
    frontier = children.iter().map(|f| f.id.clone()).collect();
    out.extend(frontier.iter().cloned());
  }
  out
}

/// 剔除层级：子孙文件夹的 Placement 逐个平移到档案（order 追加在后，最后整体重排为连续 0..n），
/// 子孙节点删除；子孙本身是档案的 archives 记录一并清除。返回 archives.json 是否有改动
fn flatten(lib: &mut Library, folder_id: &str, descendants: &[String]) -> bool {
  let before = lib.archives.archives.len();
  lib.archives.archives.retain(|a| !descendants.iter().any(|d| d == &a.folder_id));
  let archives_changed = lib.archives.archives.len() != before;

  let mut next = lib.folders.items.iter().filter(|p| p.folder == folder_id).map(|p| p.order).max().unwrap_or(-1) + 1;
  for d in descendants {
    let mut idx: Vec<usize> =
      lib.folders.items.iter().enumerate().filter(|(_, p)| &p.folder == d).map(|(i, _)| i).collect();
    idx.sort_by_key(|&i| lib.folders.items[i].order);
    for i in idx {
      lib.folders.items[i].folder = folder_id.to_string();
      lib.folders.items[i].order = next;
      next += 1;
    }
  }
  lib.folders.folders.retain(|f| !descendants.iter().any(|d| d == &f.id));

  // 档案内 order 重排为连续 0..n（稳定排序：原有项保持在前，平移项按追加序在后）
  let mut idx: Vec<usize> =
    lib.folders.items.iter().enumerate().filter(|(_, p)| p.folder == folder_id).map(|(i, _)| i).collect();
  idx.sort_by_key(|&i| lib.folders.items[i].order);
  for (n, i) in idx.into_iter().enumerate() {
    lib.folders.items[i].order = n as i32;
  }
  archives_changed
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library::{ARCHIVES_FILE, FOLDERS_FILE};
  use crate::core::models::{ArchiveData, ArchivesFile, FoldersFile, Placement};
  use crate::core::undo::UndoManager;

  struct Harness {
    _dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { _dir: dir, lib }
  }

  fn add_folder(lib: &mut Library, id: &str, name: &str, parent: Option<&str>, order: i32) {
    let folder =
      Folder { id: id.into(), name: name.into(), parent: parent.map(|s| s.into()), block: None, icon: None, kind: None, color: None, groups: None, order };
    lib.folders.folders.push(folder.clone());
    lib.index.folders.insert(id.into(), folder);
  }

  fn mark_archive(lib: &mut Library, id: &str) {
    lib.folders.folders.iter_mut().find(|f| f.id == id).unwrap().kind = Some(ARCHIVE_KIND.into());
    lib.index.folders.get_mut(id).unwrap().kind = Some(ARCHIVE_KIND.into());
  }

  fn add_placement(lib: &mut Library, asset: &str, folder: &str, order: i32) {
    lib.folders.items.push(Placement { asset: asset.into(), folder: folder.into(), name: format!("{asset}.fla"), order, origin: true });
    lib.index.rebuild_placements(&lib.folders.items);
  }

  fn folder_orders<'a>(lib: &'a Library, folder_id: &str) -> Vec<(&'a str, i32)> {
    let mut v: Vec<(&str, i32)> = lib
      .folders
      .items
      .iter()
      .filter(|p| p.folder == folder_id)
      .map(|p| (p.asset.as_str(), p.order))
      .collect();
    v.sort_by_key(|(_, o)| *o);
    v
  }

  #[test]
  fn create_archive_marks_kind_and_persists() {
    let mut h = make_lib();
    let f = create_archive(&mut h.lib, None, "雪牧", None, None).unwrap();
    assert_eq!(f.kind.as_deref(), Some(ARCHIVE_KIND));
    assert_eq!(h.lib.index.folders[&f.id].kind.as_deref(), Some(ARCHIVE_KIND), "索引同步");
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.folders.iter().find(|x| x.id == f.id).unwrap().kind.as_deref(), Some(ARCHIVE_KIND), "落盘");
    // 建档记录：created_at 当次写入（RFC3339 非空）且落盘
    let rec = h.lib.archives.archives.iter().find(|a| a.folder_id == f.id).expect("新建即建档");
    assert!(!rec.created_at.is_empty(), "created_at 当次写入");
    // 档案下新建文件夹被护栏拒绝（operations::create_folder 全链路复用）
    assert!(create_archive(&mut h.lib, Some(f.id.as_str()), "子", None, None).is_err());
  }

  /// 转换为档案：当次建档记录 created_at；幂等再转不覆盖既有记录
  #[test]
  fn convert_to_archive_creates_record_with_created_at() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-CVT", "待转", None, 0);
    convert_to_archive(&mut h.lib, "F-CVT", false).unwrap();
    let rec = h.lib.archives.archives.iter().find(|a| a.folder_id == "F-CVT").expect("转换即建档");
    assert!(!rec.created_at.is_empty(), "created_at 当次写入");
    let first = rec.created_at.clone();
    // 幂等再转：记录不重建、created_at 不覆盖
    convert_to_archive(&mut h.lib, "F-CVT", false).unwrap();
    let rec2 = h.lib.archives.archives.iter().find(|a| a.folder_id == "F-CVT").unwrap();
    assert_eq!(rec2.created_at, first, "幂等不覆盖 created_at");
    assert_eq!(h.lib.archives.archives.iter().filter(|a| a.folder_id == "F-CVT").count(), 1, "不重复建档");
  }

  /// undo 还原新建档案（begin → record_file → 业务 → commit 三步口径）
  #[test]
  fn undo_rolls_back_create_archive() {
    let mut h = make_lib();
    library::save_folders(&h.lib).unwrap();
    let path = h.lib.root.join(FOLDERS_FILE);

    let mut undo = UndoManager::default();
    undo.begin("新建档案");
    undo.record_file(&path).unwrap();
    let f = create_archive(&mut h.lib, None, "雪牧", None, None).unwrap();
    undo.commit().unwrap();
    assert_eq!(undo.state().undo_label.as_deref(), Some("新建档案"));

    undo.undo().unwrap();
    library::reload(&mut h.lib).unwrap();
    assert!(!h.lib.index.folders.contains_key(&f.id), "undo 后档案节点消失");
  }

  #[test]
  fn convert_to_archive_without_children_direct() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "雪牧", None, 0);
    add_placement(&mut h.lib, "AN-A", "F01", 0);

    let out = convert_to_archive(&mut h.lib, "F01", false).unwrap();
    assert_eq!(out, ConvertOutcome::Done, "无子级不需要确认");
    assert_eq!(h.lib.index.folders["F01"].kind.as_deref(), Some(ARCHIVE_KIND));
    assert_eq!(folder_orders(&h.lib, "F01"), vec![("AN-A", 0)], "Placement 不动");
    // 幂等：再转一次直接 Done，数据不变
    let out = convert_to_archive(&mut h.lib, "F01", true).unwrap();
    assert_eq!(out, ConvertOutcome::Done);
    // 不存在报错
    assert!(convert_to_archive(&mut h.lib, "F-不存在", true).is_err());
  }

  #[test]
  fn convert_to_archive_need_confirm_keeps_data_untouched() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "父", None, 0);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);
    add_placement(&mut h.lib, "AN-A", "F01-1", 0);
    library::save_folders(&h.lib).unwrap();

    let out = convert_to_archive(&mut h.lib, "F01", false).unwrap();
    assert_eq!(out, ConvertOutcome::NeedConfirm);
    assert_eq!(h.lib.index.folders.len(), 2, "未动数据：子文件夹还在");
    assert_eq!(h.lib.index.folders["F01"].kind, None, "未动数据：kind 未置");
    assert_eq!(folder_orders(&h.lib, "F01-1"), vec![("AN-A", 0)], "未动数据：Placement 未平移");
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.folders.len(), 2, "未动数据：盘上原样");
  }

  /// confirmed 剔除层级：子文件夹消失、Placement 平移到档案（追加在后重排）、子级档案记录清除
  #[test]
  fn convert_to_archive_confirmed_flattens_hierarchy() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "父", None, 0);
    add_folder(&mut h.lib, "F01-2", "子乙", Some("F01"), 1);
    add_folder(&mut h.lib, "F01-1", "子甲", Some("F01"), 0);
    add_folder(&mut h.lib, "F01-1-1", "孙", Some("F01-1"), 0);
    mark_archive(&mut h.lib, "F01-1"); // 子级本身是档案：同样解散并清记录
    h.lib.archives.archives.push(ArchiveData::new("F01-1"));
    add_placement(&mut h.lib, "AN-A", "F01", 0);
    add_placement(&mut h.lib, "AN-B", "F01", 1);
    add_placement(&mut h.lib, "AN-C", "F01-1", 0);
    add_placement(&mut h.lib, "AN-D", "F01-1-1", 0);
    add_placement(&mut h.lib, "AN-E", "F01-2", 0);

    let out = convert_to_archive(&mut h.lib, "F01", true).unwrap();
    assert_eq!(out, ConvertOutcome::Done);
    assert_eq!(h.lib.index.folders["F01"].kind.as_deref(), Some(ARCHIVE_KIND));
    assert_eq!(h.lib.folders.folders.len(), 1, "子孙节点删除");
    // 平移顺序 = BFS 每层按 order：子甲 → 子乙 → 孙；追加在原有两项之后，整体重排连续
    assert_eq!(
      folder_orders(&h.lib, "F01"),
      vec![("AN-A", 0), ("AN-B", 1), ("AN-C", 2), ("AN-E", 3), ("AN-D", 4)],
      "Placement 平移追加在后重排"
    );
    assert!(
      h.lib.archives.archives.iter().all(|a| a.folder_id != "F01-1"),
      "子级档案记录清除"
    );
    assert_eq!(
      h.lib.archives.archives.iter().filter(|a| a.folder_id == "F01").count(),
      1,
      "转换当次为 F01 建档（created_at）"
    );
    assert_eq!(h.lib.index.placements_of("AN-D").len(), 1, "索引重建");
    assert_eq!(h.lib.index.placements_of("AN-D")[0].folder, "F01");
    // 落盘验证
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.folders.len(), 1);
    assert_eq!(disk.items.iter().filter(|p| p.folder == "F01").count(), 5);
    let adisk: ArchivesFile = read_json(&h.lib.root.join(ARCHIVES_FILE)).unwrap();
    assert!(adisk.archives.iter().all(|a| a.folder_id != "F01-1"), "archives.json 落盘：子级记录清除");
  }

  #[test]
  fn convert_to_folder_unmarks_and_clears_record() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "雪牧", None, 0);
    mark_archive(&mut h.lib, "F01");
    h.lib.archives.archives.push(ArchiveData::new("F01"));
    h.lib.archives.archives.push(ArchiveData::new("F-其他"));

    convert_to_folder(&mut h.lib, "F01").unwrap();
    assert_eq!(h.lib.index.folders["F01"].kind, None, "摘 kind");
    assert_eq!(h.lib.folders.folders[0].kind, None);
    assert_eq!(h.lib.archives.archives.len(), 1, "清该档案记录");
    assert_eq!(h.lib.archives.archives[0].folder_id, "F-其他", "其他档案记录不动");
    let adisk: ArchivesFile = read_json(&h.lib.root.join(ARCHIVES_FILE)).unwrap();
    assert_eq!(adisk.archives.len(), 1, "archives.json 落盘");
    // 不存在报错
    assert!(convert_to_folder(&mut h.lib, "F-不存在").is_err());
  }

  /// undo 还原解散档案（folders.json + archives.json 双快照）
  #[test]
  fn undo_rolls_back_convert_to_folder() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "雪牧", None, 0);
    mark_archive(&mut h.lib, "F01");
    h.lib.archives.archives.push(ArchiveData::new("F01"));
    library::save_folders(&h.lib).unwrap();
    library::save_archives(&h.lib).unwrap();

    let mut undo = UndoManager::default();
    undo.begin("解散档案为文件夹");
    undo.record_file(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    undo.record_file(&h.lib.root.join(ARCHIVES_FILE)).unwrap();
    convert_to_folder(&mut h.lib, "F01").unwrap();
    undo.commit().unwrap();

    undo.undo().unwrap();
    library::reload(&mut h.lib).unwrap();
    assert_eq!(h.lib.index.folders["F01"].kind.as_deref(), Some(ARCHIVE_KIND), "undo 后恢复档案形态");
    assert_eq!(h.lib.archives.archives.len(), 1, "undo 后记录恢复");
  }

  #[test]
  fn list_archives_clones_all() {
    let mut h = make_lib();
    h.lib.archives.archives.push(ArchiveData::new("F01"));
    h.lib.archives.archives.push(ArchiveData::new("F02"));
    let all = list_archives(&h.lib);
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].folder_id, "F01");
  }

  /// 封面设置：徽标三角语义（不动 / 设置 / 清除）+ 落盘；无记录档案懒建
  #[test]
  fn cover_settings_badge_tri_state() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-A", "雪牧", None, 0);
    mark_archive(&mut h.lib, "F-A");
    let badge = crate::core::models::Badge { text: "主角".into(), color: "#402122".into() };
    // 设置徽标（无记录懒建）
    assert!(set_archive_cover_settings(&mut h.lib, "F-A", Some(Some(badge.clone())), None).unwrap());
    assert_eq!(h.lib.archives.archives.iter().find(|a| a.folder_id == "F-A").unwrap().badge, Some(badge));
    // 不动徽标（None）= 无改动
    assert!(!set_archive_cover_settings(&mut h.lib, "F-A", None, None).unwrap());
    // 清除徽标
    assert!(set_archive_cover_settings(&mut h.lib, "F-A", Some(None), None).unwrap());
    assert_eq!(h.lib.archives.archives.iter().find(|a| a.folder_id == "F-A").unwrap().badge, None);
    let disk: crate::core::models::ArchivesFile = read_json(&h.lib.root.join(ARCHIVES_FILE)).unwrap();
    assert_eq!(disk.archives.iter().find(|a| a.folder_id == "F-A").unwrap().badge, None, "落盘");
    // 不存在报错
    assert!(set_archive_cover_settings(&mut h.lib, "F-不存在", None, None).is_err());
  }

  /// 封面烘培：左红右蓝测试图按取景偏移裁切——x>0 视图右偏看到图左侧（红），x<0 看到右侧（蓝）；
  /// 输出恒 416×320，avatar.file 记录且 covers/ 内文件存在；时间戳命名不覆盖
  #[test]
  fn cover_bake_crops_by_crop_params() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-A", "雪牧", None, 0);
    // 400×200 左半红右半蓝
    let src = h.lib.root.join("cover-src.png");
    let mut img = image::RgbaImage::new(400, 200);
    for (x, _y, p) in img.enumerate_pixels_mut() {
      *p = if x < 200 { image::Rgba([220, 30, 30, 255]) } else { image::Rgba([30, 30, 220, 255]) };
    }
    img.save(&src).unwrap();

    let crop = crate::core::models::Crop { x: 0.2, y: 0.0, scale: 1.0 };
    assert!(set_archive_cover_settings(
      &mut h.lib,
      "F-A",
      None,
      Some(CoverInput { source_path: src.to_string_lossy().into_owned(), crop }),
    )
    .unwrap());
    let avatar = h.lib.archives.archives.iter().find(|a| a.folder_id == "F-A").unwrap().avatar.clone().unwrap();
    let out_path = h.lib.root.join(crate::core::library::COVERS_DIR).join(&avatar.file);
    assert!(out_path.is_file(), "封面文件落盘");
    assert_eq!(crate::services::thumbnail::probe(&out_path), Some((416, 320)), "输出恒 416×320");
    let baked = crate::services::thumbnail::decode_by_signature(&out_path).unwrap().to_rgba8();
    let center = baked.get_pixel(208, 160);
    assert!(center[0] > center[2], "x>0 看到图左侧（红）：{center:?}");
    // cover_file_path 输出绝对路径；换一张不覆盖旧文件
    assert_eq!(cover_file_path(&h.lib, "F-A").as_deref(), Some(out_path.to_string_lossy().as_ref()));
    let crop2 = crate::core::models::Crop { x: -0.2, y: 0.0, scale: 1.0 };
    set_archive_cover_settings(
      &mut h.lib,
      "F-A",
      None,
      Some(CoverInput { source_path: src.to_string_lossy().into_owned(), crop: crop2 }),
    )
    .unwrap();
    let avatar2 = h.lib.archives.archives.iter().find(|a| a.folder_id == "F-A").unwrap().avatar.clone().unwrap();
    assert_ne!(avatar.file, avatar2.file, "时间戳命名不覆盖（undo 回滚可恢复旧封面）");
    let baked2 = crate::services::thumbnail::decode_by_signature(&h.lib.root.join(crate::core::library::COVERS_DIR).join(&avatar2.file))
      .unwrap()
      .to_rgba8();
    let center2 = baked2.get_pixel(208, 160);
    assert!(center2[2] > center2[0], "x<0 看到图右侧（蓝）：{center2:?}");
    // 不存在的源文件报错
    assert!(set_archive_cover_settings(
      &mut h.lib,
      "F-A",
      None,
      Some(CoverInput { source_path: "不存在.png".into(), crop: crate::core::models::Crop { x: 0.0, y: 0.0, scale: 1.0 } }),
    )
    .is_err());
  }
}
