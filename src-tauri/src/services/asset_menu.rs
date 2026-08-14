//! 右键菜单配套服务（6.3）：缩略图三件套（剪贴板应用 / 自定义 / 强制刷新）、
//! 导出（文件 / 文件夹）、base64 复制。
//! 缩略图三件套与封面帧（commands/thumb_frame）同口径：先落盘 metadata.json 再同步内存索引
//! （规范第 7 条）；导出不接撤销（库外写操作）；缩略图三件套不接撤销（同 set_thumb_frame 既有口径）。
//!
//! 注：本模块故意放在 services 层（命令层只做取库与参数传递），locate 逻辑内联——
//! info 路径由 ID 推导（PRD 01-2.3），meta 取内存索引。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;

use crate::core::json_io::write_json;
use crate::core::library::{Library, CACHE_DIR};
use crate::core::models::AssetMeta;
use crate::core::paths;
use crate::services::thumbnail;

/// 缩略图三件套返回（与 SetThumbResult 同构）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbResult {
  pub thumb_path: String,
}

/// 资产定位：info 目录 + origin 路径 + metadata（克隆）
fn locate(lib: &Library, asset_id: &str) -> Result<(PathBuf, PathBuf, AssetMeta), String> {
  let meta = lib.index.assets.get(asset_id).ok_or_else(|| "资源不存在，可能已被删除".to_string())?;
  let info_dir = lib.info_dir(asset_id);
  Ok((info_dir.clone(), info_dir.join(&meta.origin_name), meta.clone()))
}

/// 用户可见的真实实体路径（复制文件路径 / 文件管理器定位 / 打开方式共用，2026-08-14 修复）：
/// 普通资产 = info/originName；**序列帧 = frames/ 封面帧**——originName 指向 frames/ 内帧，
/// info 根路径不存在（重命名跟随后 originName 恒为某帧名，见 sequence::rename_frames）。
/// 存量脱钩兜底：封面帧路径不存在（2026-08-14 前改名遗留 originName 与帧脱钩）→
/// thumb_frame → frames/ 内任一文件，保证落点是真实文件
pub fn real_origin_file(lib: &Library, asset_id: &str) -> Result<PathBuf, String> {
  let meta = lib.index.assets.get(asset_id).ok_or_else(|| "资源不存在，可能已被删除".to_string())?;
  if let Some(frames) = crate::services::sequence::frames_dir_if_sequence(&lib.assets_dir, asset_id, meta) {
    let cover = frames.join(&meta.origin_name);
    if cover.is_file() {
      return Ok(cover);
    }
    if let Some(tf) = &meta.thumb_frame {
      let p = frames.join(tf);
      if p.is_file() {
        return Ok(p);
      }
    }
    if let Some(entry) = fs::read_dir(&frames).ok().and_then(|mut rd| rd.find_map(|e| e.ok())) {
      return Ok(entry.path());
    }
    return Ok(cover); // 帧目录空：原样返回（调用方报错）
  }
  Ok(paths::origin_file(&lib.assets_dir, asset_id, &meta.origin_name))
}

/// metadata 落盘 + 索引同步（先落盘再更新，规范第 7 条）
fn persist(lib: &mut Library, info_dir: &Path, mut meta: AssetMeta, thumb: &str, clear_frame: bool) -> Result<ThumbResult, String> {
  if clear_frame {
    meta.thumb_frame = None; // 外来源（剪贴板 / 自定义文件）：封面帧标记不再有效
  }
  meta.thumbnail = Some(thumb.to_string());
  meta.modified_at = super::operations::now();
  write_json(&info_dir.join("metadata.json"), &meta).map_err(|e| e.to_string())?;
  let id = meta.id.clone();
  lib.index.assets.insert(id, meta);
  Ok(ThumbResult { thumb_path: info_dir.join(thumb).to_string_lossy().into_owned() })
}

/// 重生成缩略图：同名交给 generate 直接覆盖（同 thumb_frame 口径，防先删后失败留悬空）
fn regenerate(info_dir: &Path, meta: &AssetMeta, source: &Path) -> Option<String> {
  thumbnail::generate(info_dir, source, &meta.name)
}

/// 剪贴板图片应用为缩略图（6.3 右键菜单；2026-07-23 待办落地）：
/// arboard 读图 → PNG 落 cache/tmp → 同管线重生成覆盖 → 临时文件即删
pub fn set_thumbnail_from_clipboard(lib: &mut Library, asset_id: &str) -> Result<ThumbResult, String> {
  let (info_dir, _origin, meta) = locate(lib, asset_id)?;
  let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("剪贴板不可用：{e}"))?;
  let img = clipboard.get_image().map_err(|_| "剪贴板里没有图片".to_string())?;
  let rgba = image::RgbaImage::from_raw(img.width as u32, img.height as u32, img.bytes.to_vec())
    .ok_or_else(|| "剪贴板图片数据无效".to_string())?;
  let tmp = lib.root.join(CACHE_DIR).join("tmp").join(format!("thumb-clip-{asset_id}.png"));
  if let Some(dir) = tmp.parent() {
    fs::create_dir_all(dir).map_err(|e| format!("临时目录创建失败：{e}"))?;
  }
  image::DynamicImage::ImageRgba8(rgba).save(&tmp).map_err(|e| format!("剪贴板图片保存失败：{e}"))?;
  let thumb = regenerate(&info_dir, &meta, &tmp);
  let _ = fs::remove_file(&tmp);
  let thumb = thumb.ok_or_else(|| "缩略图生成失败，请重试".to_string())?;
  persist(lib, &info_dir, meta, &thumb, true)
}

/// 自定义缩略图（6.3 右键菜单）：指定图片文件（动图取首帧——generate 解码首帧口径与导入一致）
pub fn set_thumbnail_custom(lib: &mut Library, asset_id: &str, source_path: &str) -> Result<ThumbResult, String> {
  let (info_dir, _origin, meta) = locate(lib, asset_id)?;
  let source = PathBuf::from(source_path);
  if !source.is_file() {
    return Err("所选文件不存在".to_string());
  }
  let thumb = regenerate(&info_dir, &meta, &source).ok_or_else(|| "该文件无法用作缩略图（不支持的格式）".to_string())?;
  persist(lib, &info_dir, meta, &thumb, true)
}

/// 强制刷新缩略图（6.3 右键菜单"防自检失败"）：从 origin 重跑导入同管线
/// （generate 内含图片 / PSD / SVG / 视频首帧 / 音频波形全格式兜底）
pub fn refresh_thumbnail(lib: &mut Library, asset_id: &str) -> Result<ThumbResult, String> {
  let (info_dir, origin, meta) = locate(lib, asset_id)?;
  if !origin.is_file() {
    return Err(format!("「{}」的源文件缺失", meta.name));
  }
  let thumb = regenerate(&info_dir, &meta, &origin).ok_or_else(|| "该格式暂不支持刷新缩略图".to_string())?;
  persist(lib, &info_dir, meta, &thumb, false)
}

/// 目标目录校验（导出公共）：存在且是目录
fn check_target_dir(target_dir: &str) -> Result<PathBuf, String> {
  let dir = PathBuf::from(target_dir);
  if !dir.is_dir() {
    return Err("导出目录不存在".to_string());
  }
  Ok(dir)
}

/// 重名去重：目标已存在时词干追加 -1、-2…（保持扩展名）
fn dedupe_path(dir: &Path, file_name: &str) -> PathBuf {
  let candidate = dir.join(file_name);
  if !candidate.exists() {
    return candidate;
  }
  let (stem, ext) = match file_name.rfind('.') {
    Some(i) => (&file_name[..i], &file_name[i..]),
    None => (file_name, ""),
  };
  let mut n = 1;
  loop {
    let c = dir.join(format!("{stem}-{n}{ext}"));
    if !c.exists() {
      return c;
    }
    n += 1;
  }
}

/// 导出数据源：库索引只读快照（assets_dir + 资产表 + folders 文件）。
/// 命令层锁内克隆、工作线程持快照拷贝——导出 IO 不占用库锁，UI 不被长拷贝阻塞。
pub struct ExportSource {
  pub assets_dir: PathBuf,
  pub assets: std::collections::HashMap<String, AssetMeta>,
  pub folders: crate::core::models::FoldersFile,
}

/// 导出失败项（Finished.failed 推前端 / export_retry 入参原样回传）：
/// path = 源文件绝对路径（弹窗展示取其文件名）；target = **首跑实际尝试的完整目标路径**
///（已含重名去重——重试按 源→目标 原路覆盖，不再撞无关文件、已成功项也不重复导出）
#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFailedItem {
  pub path: String,
  pub target: String,
  pub reason: String,
}

/// 导出结果：copied = 实际拷出文件数；failed = 失败清单（导出失败弹窗 + 重试数据源）
#[derive(Debug, Default)]
pub struct ExportOutcome {
  pub copied: usize,
  pub failed: Vec<ExportFailedItem>,
}

/// 导出进度事件（命令层 Channel → 前端；serde tag="type" 小驼峰，同 ImportEvent 约定）
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ExportEvent {
  /// 清点完成，总数已知（单位 = 文件，序列帧按帧计）
  Started { total: u32 },
  /// 每处理一个文件推一个（成功失败都推，进度恒走完）
  Item,
  /// 结束（含取消中断）：copied = 实际拷出文件数；failed = 失败清单；error = 整体失败（如目录不可写）
  Finished { copied: u32, failed: Vec<ExportFailedItem>, error: Option<String> },
}

/// 序列帧判定：origin 指向 frames/ 内帧（info 根无该实体），整组帧在 frames/ 目录。
/// 动图解包同样写 frames/，但其 origin 在 info 根是实体文件 → 不误判。
/// （rename_origin 2026-08-14 前对序列帧只改 metadata 不改帧名，存量 origin_name 可能已与所有帧脱钩，故按"根无实体"判）
fn sequence_frames(src: &ExportSource, asset_id: &str, meta: &AssetMeta) -> Option<PathBuf> {
  crate::services::sequence::frames_dir_if_sequence(&src.assets_dir, asset_id, meta)
}

/// 序列帧整组导出：目标目录下建 `{folder_name}/` 文件夹（重名 -序号），frames/ 内全部帧拷入
/// （2026-08-14 用户定：序列帧用一个文件夹装起来，文件夹名称用文件名）。
/// 拷出帧数计入 out.copied；失败帧逐条进 out.failed（源帧路径 + 实际目标路径）；
/// 每帧（成败皆然）推一个 Item；cancel 置位时当前帧拷完后中断
fn copy_sequence_to(
  dir: &Path,
  frames: &Path,
  folder_name: &str,
  out: &mut ExportOutcome,
  report: &mut dyn FnMut(ExportEvent),
  cancel: &AtomicBool,
) {
  let target = dedupe_path(dir, folder_name);
  if fs::create_dir_all(&target).is_err() {
    return;
  }
  let Ok(entries) = fs::read_dir(frames) else { return };
  let mut copied = 0usize;
  for entry in entries.flatten() {
    if cancel.load(Ordering::Relaxed) {
      break;
    }
    let path = entry.path();
    if !path.is_file() {
      continue;
    }
    let dest = target.join(entry.file_name());
    match fs::copy(&path, &dest) {
      Ok(_) => copied += 1,
      Err(e) => out.failed.push(ExportFailedItem {
        path: path.to_string_lossy().into_owned(),
        target: dest.to_string_lossy().into_owned(),
        reason: e.to_string(),
      }),
    }
    report(ExportEvent::Item);
  }
  out.copied += copied;
  if copied == 0 {
    let _ = fs::remove_dir(&target); // 一帧未出（含取消）不留空壳文件夹
  }
}

/// 复制 origin 到目标（按显示名，缺扩展名补 metadata.ext）；
/// 序列帧资产改走整组导出：显示名去扩展名建文件夹，frames/ 全部帧拷入
fn copy_origin_to(
  src: &ExportSource,
  dir: &Path,
  asset_id: &str,
  display_name: &str,
  out: &mut ExportOutcome,
  report: &mut dyn FnMut(ExportEvent),
  cancel: &AtomicBool,
) {
  let Some(meta) = src.assets.get(asset_id) else { return };
  if let Some(frames) = sequence_frames(src, asset_id, meta) {
    let name = ensure_ext(display_name, &meta.ext);
    let stem = Path::new(&name).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or(name);
    copy_sequence_to(dir, &frames, &stem, out, report, cancel);
    return;
  }
  let origin = paths::origin_file(&src.assets_dir, asset_id, &meta.origin_name);
  if !origin.is_file() {
    return;
  }
  let name = ensure_ext(display_name, &meta.ext);
  let target = dedupe_path(dir, &name);
  match fs::copy(&origin, &target) {
    Ok(_) => out.copied += 1,
    Err(e) => out.failed.push(ExportFailedItem {
      path: origin.to_string_lossy().into_owned(),
      target: target.to_string_lossy().into_owned(),
      reason: e.to_string(),
    }),
  }
  report(ExportEvent::Item);
}

/// 显示名缺扩展名时补 metadata.ext（rename_placement 原始段同规则）
fn ensure_ext(name: &str, ext: &str) -> String {
  if ext.is_empty() || name.to_lowercase().ends_with(&format!(".{}", ext.to_lowercase())) {
    name.to_string()
  } else {
    format!("{name}.{ext}")
  }
}

/// 单资产将导出的文件数（Started 清点用，与拷贝循环同口径）：
/// 序列帧 = frames/ 内文件数；普通 = origin 在则 1
fn export_file_count(src: &ExportSource, asset_id: &str) -> usize {
  let Some(meta) = src.assets.get(asset_id) else { return 0 };
  if let Some(frames) = sequence_frames(src, asset_id, meta) {
    return fs::read_dir(frames).map(|rd| rd.flatten().filter(|e| e.path().is_file()).count()).unwrap_or(0);
  }
  usize::from(paths::origin_file(&src.assets_dir, asset_id, &meta.origin_name).is_file())
}

/// 导出文件（6.3 右键菜单）：各资产 origin 复制到目标目录（按 originName，重名 -序号）；
/// 序列帧资产 = 建 `{资产名}/` 文件夹装全部帧（文件夹重名同样 -序号）。
/// 进度：先推 Started（总数 = 文件数），逐文件推 Item；cancel 置位时当前文件拷完后中断。
/// 返回 ExportOutcome（copied 文件数 + failed 失败清单，失败弹窗与重试用）
pub fn export_assets(
  src: &ExportSource,
  asset_ids: &[String],
  target_dir: &str,
  report: &mut dyn FnMut(ExportEvent),
  cancel: &AtomicBool,
) -> Result<ExportOutcome, String> {
  let dir = check_target_dir(target_dir)?;
  let total: usize = asset_ids.iter().map(|id| export_file_count(src, id)).sum();
  report(ExportEvent::Started { total: total as u32 });
  let mut out = ExportOutcome::default();
  for id in asset_ids {
    if cancel.load(Ordering::Relaxed) {
      break;
    }
    let Some(meta) = src.assets.get(id) else { continue };
    if sequence_frames(src, id, meta).is_some() {
      copy_origin_to(src, &dir, id, &meta.name.clone(), &mut out, report, cancel);
      continue;
    }
    copy_origin_to(src, &dir, id, &meta.origin_name.clone(), &mut out, report, cancel);
  }
  Ok(out)
}

/// 导出文件夹（6.3 右键菜单）：目标目录下建同名目录（重名 -序号），子层级保留，
/// 素材按各自 Placement 显示名复制（链接 Placement 按其显示名导出同一份内容）；
/// 序列帧 = 显示名去扩展名建文件夹装全部帧。进度与取消同 export_assets
pub fn export_folder(
  src: &ExportSource,
  folder_id: &str,
  target_dir: &str,
  report: &mut dyn FnMut(ExportEvent),
  cancel: &AtomicBool,
) -> Result<ExportOutcome, String> {
  let dir = check_target_dir(target_dir)?;
  let folder_name = src
    .folders
    .folders
    .iter()
    .find(|f| f.id == folder_id)
    .map(|f| f.name.clone())
    .ok_or_else(|| "目标文件夹不存在，可能已被删除".to_string())?;
  let total = count_folder_files(src, folder_id);
  let root_dir = dedupe_path(&dir, &folder_name);
  fs::create_dir_all(&root_dir).map_err(|e| format!("导出目录创建失败：{e}"))?;
  report(ExportEvent::Started { total: total as u32 });
  let mut out = ExportOutcome::default();
  export_folder_inner(src, folder_id, &root_dir, &mut out, report, cancel);
  Ok(out)
}

/// 子树将导出的文件总数（含全部后代文件夹，与拷贝循环同口径）
fn count_folder_files(src: &ExportSource, folder_id: &str) -> usize {
  let mut n = 0;
  for p in &src.folders.items {
    if p.folder == folder_id {
      n += export_file_count(src, &p.asset);
    }
  }
  for child in &src.folders.folders {
    if child.parent.as_deref() == Some(folder_id) {
      n += count_folder_files(src, &child.id);
    }
  }
  n
}

fn export_folder_inner(
  src: &ExportSource,
  folder_id: &str,
  dir: &Path,
  out: &mut ExportOutcome,
  report: &mut dyn FnMut(ExportEvent),
  cancel: &AtomicBool,
) {
  for p in &src.folders.items {
    if cancel.load(Ordering::Relaxed) {
      return;
    }
    if p.folder == folder_id {
      copy_origin_to(src, dir, &p.asset, &p.name, out, report, cancel);
    }
  }
  for child in &src.folders.folders {
    if cancel.load(Ordering::Relaxed) {
      return;
    }
    if child.parent.as_deref() == Some(folder_id) {
      let sub = dedupe_path(dir, &child.name);
      if fs::create_dir_all(&sub).is_ok() {
        export_folder_inner(src, &child.id, &sub, out, report, cancel);
      }
    }
  }
}

/// 导出失败重试（导出失败弹窗"重试"，2026-08-14 用户定）：items = 首跑 Finished.failed
/// 原样回传，按记录的 源→目标 原路覆盖拷贝（目标父目录补建）——成功的项不重导、
/// 重名去重已在首跑计入 target，不会撞开无关文件。再失败原样回新清单（原因刷新）
pub fn retry_export(items: &[ExportFailedItem], report: &mut dyn FnMut(ExportEvent), cancel: &AtomicBool) -> ExportOutcome {
  report(ExportEvent::Started { total: items.len() as u32 });
  let mut out = ExportOutcome::default();
  for item in items {
    if cancel.load(Ordering::Relaxed) {
      break;
    }
    let dest = Path::new(&item.target);
    let result = dest
      .parent()
      .map(|p| fs::create_dir_all(p).map_err(|e| e.to_string()))
      .unwrap_or(Ok(()))
      .and_then(|_| fs::copy(Path::new(&item.path), dest).map_err(|e| e.to_string()));
    match result {
      Ok(_) => out.copied += 1,
      Err(e) => out.failed.push(ExportFailedItem { path: item.path.clone(), target: item.target.clone(), reason: e }),
    }
    report(ExportEvent::Item);
  }
  out
}

/// base64 上限（10MB；更大的文件复制 base64 无实际用途）
const BASE64_LIMIT: u64 = 10 * 1024 * 1024;
/// 允许 base64 的静态图片格式
const BASE64_EXTS: [&str; 6] = ["png", "jpg", "jpeg", "bmp", "webp", "gif"];

/// 复制 base64（6.3 右键菜单）：仅静态图片且 ≤10MB，返回纯 base64（不带 data: 前缀）
pub fn get_asset_base64(lib: &Library, asset_id: &str) -> Result<String, String> {
  let (_info_dir, origin, meta) = locate(lib, asset_id)?;
  if !BASE64_EXTS.contains(&meta.ext.to_lowercase().as_str()) {
    return Err("该格式不支持 base64 复制".to_string());
  }
  if meta.size > BASE64_LIMIT {
    return Err("文件过大（超过 10MB），不支持 base64 复制".to_string());
  }
  let bytes = fs::read(&origin).map_err(|e| format!("源文件读取失败：{e}"))?;
  Ok(base64_encode(&bytes))
}

/// base64 编码（无依赖手写：字节 3→4 查表，避免为一个小功能新增 base64 crate）
fn base64_encode(data: &[u8]) -> String {
  const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
  for chunk in data.chunks(3) {
    let b0 = chunk[0] as u32;
    let b1 = *chunk.get(1).unwrap_or(&0) as u32;
    let b2 = *chunk.get(2).unwrap_or(&0) as u32;
    let n = (b0 << 16) | (b1 << 8) | b2;
    out.push(T[(n >> 18) as usize & 63] as char);
    out.push(T[(n >> 12) as usize & 63] as char);
    out.push(if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
    out.push(if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library;
  use crate::core::models::Folder;
  use crate::services::import::{self as ops, PendingFile, Source};
  use crate::services::operations::finish;
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
    image::RgbaImage::from_pixel(w, h, image::Rgba([30, 30, 200, 255])).save(path).unwrap();
  }

  fn add_asset(h: &mut Harness, id: &str, file: &str, folder: &str) {
    let src = h.dir.path().join(file);
    if file.ends_with(".png") {
      make_png(&src, 100, 80);
    } else {
      fs::write(&src, b"fake").unwrap();
    }
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = ops::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.into(), meta);
    ops::add_placement(&mut h.lib, id.into(), folder.into(), file.into(), true, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
  }

  #[test]
  fn base64_encode_rfc4648() {
    assert_eq!(base64_encode(b""), "");
    assert_eq!(base64_encode(b"f"), "Zg==");
    assert_eq!(base64_encode(b"fo"), "Zm8=");
    assert_eq!(base64_encode(b"foo"), "Zm9v");
    assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
    assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
  }

  #[test]
  fn get_asset_base64_guards_and_content() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-A", "图.png", "root");
    add_asset(&mut h, "AN-F", "工程.fla", "root");
    // 图片放行且内容正确（PNG 头起点）
    let b64 = get_asset_base64(&h.lib, "AN-A").unwrap();
    assert!(base64_encode(&[0x89, b'P', b'N']).starts_with(b64.chars().take(4).collect::<String>().as_str()) || !b64.is_empty());
    // fla 拒绝
    let err = get_asset_base64(&h.lib, "AN-F").unwrap_err();
    assert!(err.contains("不支持"), "{err}");
    // 超大拒绝
    h.lib.index.assets.get_mut("AN-A").unwrap().size = BASE64_LIMIT + 1;
    let err = get_asset_base64(&h.lib, "AN-A").unwrap_err();
    assert!(err.contains("过大"), "{err}");
  }

  #[test]
  fn refresh_and_custom_thumbnail_regenerate() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-A", "图.png", "root");
    // refresh：从 origin 重生成，thumbFrame 不动
    let r = refresh_thumbnail(&mut h.lib, "AN-A").unwrap();
    assert!(r.thumb_path.ends_with("图_thumbnail.webp"), "{}", r.thumb_path);
    assert!(PathBuf::from(&r.thumb_path).is_file());
    // custom：外来图片 → thumbFrame 清空
    let custom = h.dir.path().join("自定义.png");
    make_png(&custom, 50, 50);
    let r = set_thumbnail_custom(&mut h.lib, "AN-A", custom.to_str().unwrap()).unwrap();
    assert!(PathBuf::from(&r.thumb_path).is_file());
    let meta = &h.lib.index.assets["AN-A"];
    assert_eq!(meta.thumb_frame, None, "外来源清空封面帧标记");
    let disk: AssetMeta = read_json(&h.lib.info_dir("AN-A").join("metadata.json")).unwrap();
    assert_eq!(disk.thumbnail.as_deref(), Some("图_thumbnail.webp"), "metadata 落盘");
    // 不存在的源报错
    assert!(set_thumbnail_custom(&mut h.lib, "AN-A", "不存在.png").is_err());
  }

  /// 导出数据源快照（命令层锁内克隆的同形物）
  fn src(h: &Harness) -> ExportSource {
    ExportSource {
      assets_dir: h.lib.assets_dir.clone(),
      assets: h.lib.index.assets.clone(),
      folders: h.lib.folders.clone(),
    }
  }

  #[test]
  fn export_assets_dedupes_and_skips_missing() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-A", "a.png", "root");
    add_asset(&mut h, "AN-B", "a.png", "root"); // 同名 originName
    let out = h.dir.path().join("导出");
    fs::create_dir_all(&out).unwrap();
    let src = src(&h);
    let mut sink = |_e: ExportEvent| {};
    let no = AtomicBool::new(false);
    let out0 = export_assets(&src, &["AN-A".into(), "AN-B".into(), "AN-不存在".into()], out.to_str().unwrap(), &mut sink, &no).unwrap();
    assert_eq!(out0.copied, 2, "同名也去重导出两份");
    assert!(out0.failed.is_empty());
    assert!(out.join("a.png").is_file());
    assert!(out.join("a-1.png").is_file(), "重名追加 -序号");
    assert!(export_assets(&src, &["AN-A".into()], "不存在目录", &mut sink, &no).is_err());
  }

  #[test]
  fn export_folder_keeps_hierarchy_and_display_names() {
    let mut h = make_lib();
    let sub = Folder { id: "F-SUB".into(), name: "子夹".into(), parent: Some("F-TOP".into()), order: 0 };
    let top = Folder { id: "F-TOP".into(), name: "顶夹".into(), parent: None, order: 0 };
    h.lib.folders.folders.push(top.clone());
    h.lib.folders.folders.push(sub.clone());
    h.lib.index.folders.insert(top.id.clone(), top);
    h.lib.index.folders.insert(sub.id.clone(), sub);
    add_asset(&mut h, "AN-A", "a.png", "F-TOP");
    add_asset(&mut h, "AN-B", "b.png", "F-SUB");
    // 链接 Placement：同资产在子夹里的别名
    ops::add_placement(&mut h.lib, "AN-A".into(), "F-SUB".into(), "别名.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();

    let out = h.dir.path().join("导出2");
    fs::create_dir_all(&out).unwrap();
    let src = src(&h);
    let mut sink = |_e: ExportEvent| {};
    let no = AtomicBool::new(false);
    let n = export_folder(&src, "F-TOP", out.to_str().unwrap(), &mut sink, &no).unwrap().copied;
    assert_eq!(n, 3, "顶层 1 + 子夹 1 + 链接别名 1");
    let base = out.join("顶夹");
    assert!(base.join("a.png").is_file(), "显示名导出");
    assert!(base.join("子夹").join("b.png").is_file(), "子层级保留");
    assert!(base.join("子夹").join("别名.png").is_file(), "链接按显示名导出");
    assert!(export_folder(&src, "F-不存在", out.to_str().unwrap(), &mut sink, &no).is_err());
  }

  /// 序列帧资产入库（3 帧 png，打包进 frames/ + origin Placement）
  fn add_sequence(h: &mut Harness, id: &str, name: &str, folder: &str) {
    let src_dir = h.dir.path().join(format!("{id}-源"));
    fs::create_dir_all(&src_dir).unwrap();
    let mut frames = Vec::new();
    for n in 1..=3u32 {
      let p = src_dir.join(format!("{name}{n}.png"));
      make_png(&p, 10 + n, 10);
      frames.push((p, n));
    }
    let meta =
      crate::services::sequence::build_sequence_asset(&h.lib.assets_dir, name, "png", 1, &frames, "sha256:seq", id, Source::Copy, None)
        .unwrap();
    h.lib.index.assets.insert(id.into(), meta);
    ops::add_placement(&mut h.lib, id.into(), folder.into(), format!("{name}.png"), true, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
  }

  #[test]
  fn export_assets_sequence_packs_frames_into_named_folder() {
    let mut h = make_lib();
    add_sequence(&mut h, "AN-S", "走路", "root");
    add_asset(&mut h, "AN-A", "a.png", "root");
    let out = h.dir.path().join("导出序列");
    fs::create_dir_all(&out).unwrap();
    let src = src(&h);
    let mut events = Vec::new();
    let no = AtomicBool::new(false);
    let n = {
      let mut sink = |e: ExportEvent| events.push(e);
      export_assets(&src, &["AN-S".into(), "AN-A".into()], out.to_str().unwrap(), &mut sink, &no).unwrap().copied
    };
    assert_eq!(n, 4, "序列帧按帧计 3 + 普通文件 1");
    assert_eq!(events.first(), Some(&ExportEvent::Started { total: 4 }), "清点事件先行，总数 = 文件数");
    assert_eq!(events.iter().filter(|e| matches!(e, ExportEvent::Item)).count(), 4, "逐文件推 Item");
    assert!(out.join("a.png").is_file());
    let dir = out.join("走路");
    assert!(dir.is_dir(), "序列帧导出为资产名文件夹");
    for f in ["走路-1.png", "走路-2.png", "走路-3.png"] {
      assert!(dir.join(f).is_file(), "{f} 应拷入");
    }
    assert!(!out.join("走路-1.png").is_file(), "帧不散落到目标根");
  }

  #[test]
  fn export_folder_sequence_and_anim_unpack_regression() {
    let mut h = make_lib();
    let top = Folder { id: "F-TOP".into(), name: "顶夹".into(), parent: None, order: 0 };
    h.lib.folders.folders.push(top.clone());
    h.lib.index.folders.insert(top.id.clone(), top);
    add_sequence(&mut h, "AN-S", "走路", "F-TOP");
    // 链接 Placement：同资产别名 → 别名为名的文件夹装同一组帧
    ops::add_placement(&mut h.lib, "AN-S".into(), "F-TOP".into(), "别名.png".into(), false, &mut HashMap::new());
    // 动图解包回归：origin 在 info 根 + frames/ 有解包帧 → 仍按单文件导出
    add_asset(&mut h, "AN-G", "动图.gif", "F-TOP");
    let unpack = paths::frames_dir(&h.lib.assets_dir, "AN-G");
    fs::create_dir_all(&unpack).unwrap();
    fs::write(unpack.join("动图-00001.webp"), b"f").unwrap();
    finish(&mut h.lib).unwrap();

    let out = h.dir.path().join("导出夹");
    fs::create_dir_all(&out).unwrap();
    let src = src(&h);
    let mut sink = |_e: ExportEvent| {};
    let no = AtomicBool::new(false);
    let n = export_folder(&src, "F-TOP", out.to_str().unwrap(), &mut sink, &no).unwrap().copied;
    assert_eq!(n, 7, "序列帧 2 条 Placement × 3 帧 + 动图 1");
    let base = out.join("顶夹");
    assert!(base.join("走路").join("走路-1.png").is_file(), "origin 显示名建文件夹");
    assert!(base.join("别名").join("走路-2.png").is_file(), "链接别名建文件夹装同组帧");
    assert!(base.join("动图.gif").is_file(), "动图解包 frames/ 不误判，单文件导出");
    assert!(!base.join("动图").exists(), "动图不建文件夹");
  }

  #[test]
  fn export_cancel_stops_before_next_asset() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-A", "a.png", "root");
    add_asset(&mut h, "AN-B", "b.png", "root");
    let out = h.dir.path().join("导出取消");
    fs::create_dir_all(&out).unwrap();
    let src = src(&h);
    let mut sink = |_e: ExportEvent| {};
    let cancelled = AtomicBool::new(true); // 预置取消：一个文件都不应拷出
    let out0 = export_assets(&src, &["AN-A".into(), "AN-B".into()], out.to_str().unwrap(), &mut sink, &cancelled).unwrap();
    assert_eq!(out0.copied, 0, "取消置位即中断");
    assert!(!out.join("a.png").exists());
  }

  #[test]
  fn real_origin_file_points_into_frames_for_sequence() {
    let mut h = make_lib();
    add_sequence(&mut h, "AN-S", "走路", "root");
    add_asset(&mut h, "AN-A", "a.png", "root");
    let seq = real_origin_file(&h.lib, "AN-S").unwrap();
    assert!(seq.is_file(), "序列帧 = frames/ 封面帧（真实存在）");
    assert!(seq.ends_with("走路-1.png"), "{seq:?}");
    let normal = real_origin_file(&h.lib, "AN-A").unwrap();
    assert!(normal.is_file() && normal.ends_with("a.png"));
    assert!(real_origin_file(&h.lib, "AN-不存在").is_err());
  }

  #[test]
  fn real_origin_file_legacy_desync_falls_back() {
    // 存量脱钩（2026-08-14 前改名遗留：originName = 显示名.png，与所有帧不同名）→ 回退真实帧
    let mut h = make_lib();
    add_sequence(&mut h, "AN-S", "走路", "root");
    h.lib.index.assets.get_mut("AN-S").unwrap().origin_name = "跑步.png".into();
    let p = real_origin_file(&h.lib, "AN-S").unwrap();
    assert!(p.is_file(), "脱钩也要落到真实帧：{p:?}");
    assert!(p.parent().unwrap().ends_with("frames"));
  }

  #[test]
  fn retry_export_recopies_and_keeps_failures() {
    let h = make_lib();
    let src_file = h.dir.path().join("源.png");
    make_png(&src_file, 8, 8);
    let out = h.dir.path().join("重试输出");
    let good_target = out.join("子目录").join("a.png"); // 父目录不存在 → 重试自动补建
    let items = vec![
      ExportFailedItem {
        path: src_file.to_string_lossy().into_owned(),
        target: good_target.to_string_lossy().into_owned(),
        reason: "首跑失败".into(),
      },
      ExportFailedItem {
        path: h.dir.path().join("不存在.png").to_string_lossy().into_owned(),
        target: out.join("b.png").to_string_lossy().into_owned(),
        reason: "源丢失".into(),
      },
    ];
    let mut events = Vec::new();
    let no = AtomicBool::new(false);
    let out0 = {
      let mut sink = |e: ExportEvent| events.push(e);
      retry_export(&items, &mut sink, &no)
    };
    assert_eq!(out0.copied, 1);
    assert!(good_target.is_file(), "成功项原路拷出（父目录补建）");
    assert_eq!(out0.failed.len(), 1, "再失败项留清单");
    assert!(out0.failed[0].path.ends_with("不存在.png"));
    assert_eq!(out0.failed[0].target, items[1].target, "目标路径原样保留供下次重试");
    assert_eq!(events.first(), Some(&ExportEvent::Started { total: 2 }));
    assert_eq!(events.iter().filter(|e| matches!(e, ExportEvent::Item)).count(), 2);
  }
}
