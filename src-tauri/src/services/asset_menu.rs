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
use crate::services::origin_sync;
use crate::services::thumbnail;
use crate::services::thumb_formats;

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

/// metadata 落盘 + 索引同步（先落盘再更新，规范第 7 条）；
/// thumb_width/height 同步刷新（5.4：布局比例跟缩略图走——自定义后自适应，刷新缩略图回退）
fn persist(lib: &mut Library, info_dir: &Path, mut meta: AssetMeta, thumb: &thumbnail::ThumbOut, clear_frame: bool) -> Result<ThumbResult, String> {
  if clear_frame {
    meta.thumb_frame = None; // 外来源（剪贴板 / 自定义文件）：封面帧标记不再有效
  }
  meta.thumbnail = Some(thumb.name.clone());
  meta.thumb_width = Some(thumb.width);
  meta.thumb_height = Some(thumb.height);
  meta.colors = thumb.colors.clone(); // 主色随缩略图刷新（6.1 颜色解析）
  meta.modified_at = super::operations::now();
  write_json(&info_dir.join("metadata.json"), &meta).map_err(|e| e.to_string())?;
  let id = meta.id.clone();
  lib.index.assets.insert(id, meta);
  Ok(ThumbResult { thumb_path: info_dir.join(&thumb.name).to_string_lossy().into_owned() })
}

/// 重生成缩略图：同名交给 generate 直接覆盖（同 thumb_frame 口径，防先删后失败留悬空）
fn regenerate(info_dir: &Path, meta: &AssetMeta, source: &Path) -> Option<thumbnail::ThumbOut> {
  thumbnail::generate(info_dir, source, &meta.name)
}

/// 剪贴板图片应用为缩略图（6.3 右键菜单；2026-07-23 待办落地）：
/// arboard 读图（PNG/CF_DIBV5）→ CF_DIB 兜底（2026-09-17 实机走查：QQ/微信截图等只写
/// DIB 的软件读不到——arboard 不认 CF_DIB）→ PNG 落 cache/tmp → 同管线重生成覆盖 → 临时文件即删
pub fn set_thumbnail_from_clipboard(lib: &mut Library, asset_id: &str) -> Result<ThumbResult, String> {
  let (info_dir, _origin, meta) = locate(lib, asset_id)?;
  let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("剪贴板不可用：{e}"))?;
  let rgba = match clipboard.get_image() {
    Ok(img) => image::RgbaImage::from_raw(img.width as u32, img.height as u32, img.bytes.to_vec())
      .ok_or_else(|| "剪贴板图片数据无效".to_string())?,
    Err(_) => crate::adapters::clipboard::read_image_dib_fallback()?,
  };
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

/// 复制文件到系统剪贴板（2026-09-17 实机走查：复制图片要能粘贴到 APP 外——网页 AI 对话框等）：
/// 内部「复制文件」同步调用——CF_HDROP 文件列表（各软件按附件粘贴）+ 单张静态图且 ≤20MB
/// 附位图（PNG + CF_DIBV5，对话框直接贴图；Eagle 同款）。ws: 与解析失败项跳过
/// （real_origin_file 同 resolve_drag_paths 口径）；纯读操作不改库数据。
/// 恒 cut=false：库剪切不写系统剪贴板（移动语义 APP 内流转，PRD 02-13 2026-09-17 定案）
pub fn copy_files_to_system_clipboard(lib: &Library, asset_ids: &[String]) -> Result<(), String> {
  let mut seen = std::collections::HashSet::new();
  let paths: Vec<PathBuf> = asset_ids
    .iter()
    .filter_map(|id| real_origin_file(lib, id).ok())
    .filter(|p| seen.insert(p.clone()))
    .collect();
  if paths.is_empty() {
    return Err("没有可复制的文件".to_string());
  }
  let bitmap = clipboard_bitmap(&paths);
  let refs: Vec<&Path> = paths.iter().map(|p| p.as_path()).collect();
  crate::adapters::clipboard::write_files_clipboard(&refs, bitmap.as_ref(), false)
}

/// 系统剪贴板附位图（库 2026-09-17 与 ws 2026-10-01 同口径，供 write_files_clipboard 复用）：
/// 仅当文件清单 = 单张静态图（png/jpg/jpeg/bmp/webp/gif）且 ≤20MB（解码预算）时解码附位图——
/// 对话框 / IM 直接贴图；否则 None（仅 CF_HDROP 附件）。解码失败静默 None
pub fn clipboard_bitmap(paths: &[PathBuf]) -> Option<image::RgbaImage> {
  if paths.len() != 1 {
    return None;
  }
  let p = &paths[0];
  let ext = p
    .extension()
    .and_then(|e| e.to_str())
    .map(|e| e.to_ascii_lowercase())
    .unwrap_or_default();
  let small = fs::metadata(p).map(|m| m.len() <= 20 * 1024 * 1024).unwrap_or(false);
  if small && matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp" | "webp" | "gif") {
    image::open(p).ok().map(|i| i.to_rgba8())
  } else {
    None
  }
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

/// 刷新缩略图（6.3 右键菜单"防自检失败"；2026-10-08 扩展为全格式 + 文件数据托底）：
/// 无条件重算 size / hash / 宽高 + 从 origin 重跑导入同管线缩略图（含音视频首帧 / 波形）；
/// 文本等无内容缩略图格式不再报"不支持"——数据照刷、缩略图回退格式图标。
/// 序列帧 / 虚拟片段 / origin 缺失仍报「源文件缺失」（同旧口径）
pub fn refresh_thumbnail(lib: &mut Library, asset_id: &str) -> Result<origin_sync::RefreshResult, String> {
  let out = origin_sync::refresh_from_origin(lib, asset_id)?;
  Ok(origin_sync::RefreshResult { thumb_path: out.thumb_path, size: out.size, modified_at: out.modified_at })
}

/// 复制缩略图位图到剪贴板（5.4；2026-08-22 用户定：全类型统一复制各自缩略图——
/// 静态图 / 视频封面 / 序列帧封面 / 动图首帧 / 音频波形均可，shell 格式图标也放开不拦，
/// 用途 = 配「自定义缩略图 - 从剪贴板」把本资产缩略图设给其他资产）。纯读操作不改库数据。
pub fn copy_thumbnail_to_clipboard(lib: &Library, asset_id: &str) -> Result<(), String> {
  let src = thumbnail_source(lib, asset_id)?;
  let img = image::open(&src).map_err(|e| format!("缩略图解码失败：{e}"))?.to_rgba8();
  let (width, height) = (img.width() as usize, img.height() as usize);
  let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("剪贴板不可用：{e}"))?;
  clipboard
    .set_image(arboard::ImageData { width, height, bytes: img.into_raw().into() })
    .map_err(|e| format!("剪贴板写入失败：{e}"))
}

/// 缩略图源路径定位（拆出供单测——arboard 写入薄壳不测，防测试污染用户剪贴板）：
/// info 缩略图优先；无内容缩略图（meta.thumbnail = None）回退格式图标缓存
/// （cache/thumbs/{ext}.webp，ensure_format_icon 幂等——无缓存现场 shell 提取）
fn thumbnail_source(lib: &Library, asset_id: &str) -> Result<PathBuf, String> {
  let (info_dir, origin, meta) = locate(lib, asset_id)?;
  if let Some(name) = &meta.thumbnail {
    let p = info_dir.join(name);
    if p.is_file() {
      return Ok(p);
    }
    return Err("缩略图文件缺失，请刷新缩略图后重试".to_string());
  }
  if thumbnail::ensure_format_icon(&lib.root, &origin, &meta.ext) {
    return Ok(thumbnail::format_icon_path(&lib.root, &meta.ext));
  }
  Err("该格式的系统图标不可用".to_string())
}

// —— 伴生缩略图绑定（8.1，PRD 02-15 hidden 资产模型；CEP 导出与手动设置共用）——

/// 可作为伴生缩略图的资产格式：静态图 + 动图（hidden 绑定后工程卡片显示其缩略图）
fn is_thumb_capable(ext: &str) -> bool {
  matches!(ext, "png" | "jpg" | "jpeg" | "bmp" | "webp" | "gif")
}

/// 设置伴生缩略图（绑定 / 换绑）：asset = 工程侧（fla 等），thumb = 缩略图资产。
/// 一个 hidden 资产可绑多个工程（重复绑定幂等）；换绑不动旧 hidden 资产
/// （PRD 02-15：旧缩略图由用户手动「解除缩略图」或删除）。
/// 写序 = 先工程侧引用后缩略图侧 hidden：半途失败时引用目标未 hidden 不代理
/// （视图层 filter 兜住），不出现反向半态。不接撤销（同缩略图三件套既有口径）。
pub fn set_thumb_asset(lib: &mut Library, asset_id: &str, thumb_id: &str) -> Result<(), String> {
  if asset_id == thumb_id {
    return Err("不能把资产设为自身的缩略图".to_string());
  }
  let asset = lib.index.assets.get(asset_id).ok_or_else(|| "资源不存在，可能已被删除".to_string())?.clone();
  if asset.hidden == Some(true) {
    return Err("缩略图资产不能再绑定缩略图".to_string());
  }
  let thumb = lib.index.assets.get(thumb_id).ok_or_else(|| "缩略图资产不存在，可能已被删除".to_string())?.clone();
  if !is_thumb_capable(&thumb.ext) {
    return Err(format!("「.{}」格式不能用作缩略图", thumb.ext));
  }
  if asset.thumb_asset_id.as_deref() == Some(thumb_id) {
    return Ok(()); // 重复绑定幂等
  }
  let now = super::operations::now();
  let mut a = asset;
  a.thumb_asset_id = Some(thumb_id.to_string());
  a.modified_at = now.clone();
  write_json(&lib.info_dir(asset_id).join("metadata.json"), &a).map_err(|e| e.to_string())?;
  lib.index.assets.insert(asset_id.to_string(), a);
  let mut t = thumb;
  t.hidden = Some(true);
  t.modified_at = now;
  write_json(&lib.info_dir(thumb_id).join("metadata.json"), &t).map_err(|e| e.to_string())?;
  lib.index.assets.insert(thumb_id.to_string(), t);
  Ok(())
}

/// 还原为素材（PRD 02-15「还原为素材」；换绑后旧缩略图的「解除缩略图」同此）：
/// 取消 hidden 标记（链接位置原样恢复显示），并清除所有工程侧指向它的 thumb_asset_id 引用
/// （联动清除——否则工程卡片会继续显示一个已非 hidden 资产的图）。返回清除的引用数。
pub fn restore_thumb_asset(lib: &mut Library, asset_id: &str) -> Result<u32, String> {
  let asset = lib.index.assets.get(asset_id).ok_or_else(|| "资源不存在，可能已被删除".to_string())?.clone();
  if asset.hidden != Some(true) {
    return Err("该资产不是缩略图资产".to_string());
  }
  let refs: Vec<String> = lib
    .index
    .assets
    .values()
    .filter(|m| m.thumb_asset_id.as_deref() == Some(asset_id))
    .map(|m| m.id.clone())
    .collect();
  let now = super::operations::now();
  for id in &refs {
    let mut m = lib.index.assets.get(id).unwrap().clone();
    m.thumb_asset_id = None;
    m.modified_at = now.clone();
    write_json(&lib.info_dir(id).join("metadata.json"), &m).map_err(|e| e.to_string())?;
    lib.index.assets.insert(id.clone(), m);
  }
  let mut a = asset;
  a.hidden = None;
  a.modified_at = now;
  write_json(&lib.info_dir(asset_id).join("metadata.json"), &a).map_err(|e| e.to_string())?;
  lib.index.assets.insert(asset_id.to_string(), a);
  Ok(refs.len() as u32)
}

/// 解除伴生缩略图绑定（详情面板缩略图区块 ×，2026-09-23）：仅清工程侧 thumb_asset_id，
/// hidden 伴生资产不动（PRD 02-15：旧缩略图由用户手动「解除缩略图」或删除——还原走
/// restore_thumb_asset）。无绑定 = 幂等 Ok 不落盘；不接撤销（同缩略图三件套既有口径）
pub fn unbind_thumb_asset(lib: &mut Library, asset_id: &str) -> Result<(), String> {
  let asset = lib.index.assets.get(asset_id).ok_or_else(|| "资源不存在，可能已被删除".to_string())?.clone();
  if asset.thumb_asset_id.is_none() {
    return Ok(());
  }
  let mut a = asset;
  a.thumb_asset_id = None;
  a.modified_at = super::operations::now();
  write_json(&lib.info_dir(asset_id).join("metadata.json"), &a).map_err(|e| e.to_string())?;
  lib.index.assets.insert(asset_id.to_string(), a);
  Ok(())
}

/// 目标目录校验（导出公共）：存在且是目录
fn check_target_dir(target_dir: &str) -> Result<PathBuf, String> {
  let dir = PathBuf::from(target_dir);
  if !dir.is_dir() {
    return Err("导出目录不存在".to_string());
  }
  Ok(dir)
}

/// 重名去重：目标已存在时词干追加 _1、_2…（保持扩展名；2026-09-23 序号格式自 - 改 _）
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
    let c = dir.join(format!("{stem}_{n}{ext}"));
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

/// 序列帧整组导出：目标目录下建 `{folder_name}/` 文件夹（重名 _序号），frames/ 内全部帧拷入
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

/// 导出文件（6.3 右键菜单）：各资产 origin 复制到目标目录（按 originName，重名 _序号）；
/// 序列帧资产 = 建 `{资产名}/` 文件夹装全部帧（文件夹重名同样 _序号）。
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

/// 导出文件夹（6.3 右键菜单）：目标目录下建同名目录（重名 _序号），子层级保留，
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

/// base64 上限（10MB；更大的内容复制 base64 无实际用途）
const BASE64_LIMIT: u64 = 10 * 1024 * 1024;
/// 允许 base64 的静态图片格式（读原文件字节）
const BASE64_EXTS: [&str; 6] = ["png", "jpg", "jpeg", "bmp", "webp", "gif"];
/// 允许 base64 的解码格式（2026-08-14 用户定）：合成图解码 → 长边 ≤4096 PNG 编码，
/// 贴给 AI / 网页可用（原字节 PSD 对方不认；10MB 上限对编码后 PNG 判定）
const DECODE_BASE64_EXTS: [&str; 3] = ["psd", "psb", "svg"];

/// base64 复制定位（命令层锁内取数）：origin 路径 + 小写扩展名 + 文件大小；
/// 重活（PSD 解码 / PNG 编码）走 base64_from_origin 在锁外执行
pub fn locate_base64_source(lib: &Library, asset_id: &str) -> Result<(PathBuf, String, u64), String> {
  let (_info_dir, origin, meta) = locate(lib, asset_id)?;
  Ok((origin, meta.ext.to_lowercase(), meta.size))
}

/// 复制 base64（6.3 右键菜单）：静态图片读原字节（≤10MB）；psd / psb / svg 解码合成图
/// → 长边 ≤4096 PNG 编码（≤10MB）；返回纯 base64（不带 data: 前缀）
pub fn base64_from_origin(origin: &Path, ext: &str, size: u64) -> Result<String, String> {
  if BASE64_EXTS.contains(&ext) {
    if size > BASE64_LIMIT {
      return Err("文件过大（超过 10MB），不支持 base64 复制".to_string());
    }
    let bytes = fs::read(origin).map_err(|e| format!("源文件读取失败：{e}"))?;
    return Ok(base64_encode(&bytes));
  }
  if DECODE_BASE64_EXTS.contains(&ext) {
    let img = thumb_formats::decode(origin).ok_or_else(|| "解码失败（PSD 可能未存合成图）".to_string())?;
    let img = thumbnail::shrink_long_edge(image::DynamicImage::ImageRgba8(img), thumbnail::FULL_PREVIEW_LONG_EDGE);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).map_err(|e| format!("PNG 编码失败：{e}"))?;
    let bytes = buf.into_inner();
    if bytes.len() as u64 > BASE64_LIMIT {
      return Err("解码后图片过大（超过 10MB），不支持 base64 复制".to_string());
    }
    return Ok(base64_encode(&bytes));
  }
  Err("该格式不支持 base64 复制".to_string())
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
  fn set_thumb_asset_binds_and_marks_hidden() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-F", "工程.fla", "root");
    add_asset(&mut h, "AN-T", "封面.webp", "root");
    set_thumb_asset(&mut h.lib, "AN-F", "AN-T").unwrap();
    assert_eq!(h.lib.index.assets["AN-T"].hidden, Some(true), "缩略图资产打 hidden");
    assert_eq!(h.lib.index.assets["AN-F"].thumb_asset_id.as_deref(), Some("AN-T"), "工程侧记引用");
    // 先落盘后内存（规范第 7 条）：磁盘与索引一致
    let meta: AssetMeta = read_json(&h.lib.info_dir("AN-T").join("metadata.json")).unwrap();
    assert_eq!(meta.hidden, Some(true));
    let meta: AssetMeta = read_json(&h.lib.info_dir("AN-F").join("metadata.json")).unwrap();
    assert_eq!(meta.thumb_asset_id.as_deref(), Some("AN-T"));
    set_thumb_asset(&mut h.lib, "AN-F", "AN-T").unwrap(); // 重复绑定幂等
    assert_eq!(h.lib.index.assets["AN-F"].thumb_asset_id.as_deref(), Some("AN-T"));
  }

  #[test]
  fn set_thumb_asset_rebind_keeps_old_hidden_and_shares() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-F", "工程.fla", "root");
    add_asset(&mut h, "AN-F2", "工程2.fla", "root");
    add_asset(&mut h, "AN-T", "封面.webp", "root");
    add_asset(&mut h, "AN-T2", "封面2.webp", "root");
    set_thumb_asset(&mut h.lib, "AN-F", "AN-T").unwrap();
    // 多工程共绑同一 hidden 资产（PRD 02-15：一个资产可绑定多个工程）
    set_thumb_asset(&mut h.lib, "AN-F2", "AN-T").unwrap();
    assert_eq!(h.lib.index.assets["AN-F2"].thumb_asset_id.as_deref(), Some("AN-T"));
    // 换绑：旧 hidden 资产保持 hidden（用户手动处理）
    set_thumb_asset(&mut h.lib, "AN-F", "AN-T2").unwrap();
    assert_eq!(h.lib.index.assets["AN-F"].thumb_asset_id.as_deref(), Some("AN-T2"));
    assert_eq!(h.lib.index.assets["AN-T"].hidden, Some(true), "换绑不动旧 hidden");
  }

  #[test]
  fn set_thumb_asset_guards() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-F", "工程.fla", "root");
    add_asset(&mut h, "AN-T", "封面.webp", "root");
    add_asset(&mut h, "AN-V", "视频.mp4", "root");
    assert!(set_thumb_asset(&mut h.lib, "AN-F", "AN-F").unwrap_err().contains("自身"));
    assert!(set_thumb_asset(&mut h.lib, "AN-F", "AN-X").unwrap_err().contains("不存在"));
    assert!(set_thumb_asset(&mut h.lib, "AN-F", "AN-V").unwrap_err().contains("不能用作缩略图"), "非图格式拦截");
    set_thumb_asset(&mut h.lib, "AN-F", "AN-T").unwrap();
    // hidden 资产不能再绑定自己的缩略图（防套娃）
    assert!(set_thumb_asset(&mut h.lib, "AN-T", "AN-F").unwrap_err().contains("缩略图资产"));
  }

  /// 解除绑定（详情缩略图区块 ×，2026-09-23）：仅清工程侧 thumb_asset_id，hidden 侧不动；
  /// 幂等（无绑定 = Ok 不写盘）；不存在资产报中文错
  #[test]
  fn unbind_thumb_asset_clears_binding_only() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-F", "工程.fla", "root");
    add_asset(&mut h, "AN-T", "封面.webp", "root");
    set_thumb_asset(&mut h.lib, "AN-F", "AN-T").unwrap();
    unbind_thumb_asset(&mut h.lib, "AN-F").unwrap();
    assert_eq!(h.lib.index.assets["AN-F"].thumb_asset_id, None, "工程侧引用清除");
    assert_eq!(h.lib.index.assets["AN-T"].hidden, Some(true), "hidden 侧不动（还原走 restore）");
    let meta: AssetMeta = read_json(&h.lib.info_dir("AN-F").join("metadata.json")).unwrap();
    assert_eq!(meta.thumb_asset_id, None, "先落盘后内存（规范第 7 条）");
    unbind_thumb_asset(&mut h.lib, "AN-F").unwrap(); // 幂等：无绑定 = Ok
    assert!(unbind_thumb_asset(&mut h.lib, "AN-X").unwrap_err().contains("不存在"));
  }

  #[test]
  fn restore_thumb_asset_clears_refs_and_unhides() {    let mut h = make_lib();
    add_asset(&mut h, "AN-F", "工程.fla", "root");
    add_asset(&mut h, "AN-F2", "工程2.fla", "root");
    add_asset(&mut h, "AN-T", "封面.webp", "root");
    set_thumb_asset(&mut h.lib, "AN-F", "AN-T").unwrap();
    set_thumb_asset(&mut h.lib, "AN-F2", "AN-T").unwrap();
    let cleared = restore_thumb_asset(&mut h.lib, "AN-T").unwrap();
    assert_eq!(cleared, 2, "返回清除的引用数");
    assert_eq!(h.lib.index.assets["AN-T"].hidden, None, "hidden 解除");
    assert_eq!(h.lib.index.assets["AN-F"].thumb_asset_id, None);
    assert_eq!(h.lib.index.assets["AN-F2"].thumb_asset_id, None);
    // 落盘一致
    let meta: AssetMeta = read_json(&h.lib.info_dir("AN-F").join("metadata.json")).unwrap();
    assert_eq!(meta.thumb_asset_id, None);
    // 非 hidden 资产还原报错
    assert!(restore_thumb_asset(&mut h.lib, "AN-T").unwrap_err().contains("不是缩略图资产"));
  }

  #[test]
  fn base64_guards_and_content() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-A", "图.png", "root");
    add_asset(&mut h, "AN-F", "工程.fla", "root");
    // 图片放行且内容正确（PNG 头起点）
    let (origin, ext, size) = locate_base64_source(&h.lib, "AN-A").unwrap();
    let b64 = base64_from_origin(&origin, &ext, size).unwrap();
    assert!(base64_encode(&[0x89, b'P', b'N']).starts_with(b64.chars().take(4).collect::<String>().as_str()) || !b64.is_empty());
    // fla 拒绝
    let (origin, ext, size) = locate_base64_source(&h.lib, "AN-F").unwrap();
    let err = base64_from_origin(&origin, &ext, size).unwrap_err();
    assert!(err.contains("不支持"), "{err}");
    // 超大拒绝（静态图片分支按文件大小判定）
    let (origin, ext, _) = locate_base64_source(&h.lib, "AN-A").unwrap();
    let err = base64_from_origin(&origin, &ext, BASE64_LIMIT + 1).unwrap_err();
    assert!(err.contains("过大"), "{err}");
  }

  #[test]
  fn base64_svg_decodes_to_png() {
    // 解码格式分支：svg 走适配器解码 → PNG 编码（解码回字节应为 PNG 魔数）
    use base64::Engine;
    let dir = tempfile::tempdir().unwrap();
    let svg = dir.path().join("图.svg");
    fs::write(&svg, r#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8" fill="red"/></svg>"#).unwrap();
    let size = fs::metadata(&svg).unwrap().len();
    let b64 = base64_from_origin(&svg, "svg", size).unwrap();
    let bytes = base64::engine::general_purpose::STANDARD.decode(&b64).unwrap();
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"), "应为 PNG 编码输出");
  }

  #[test]
  fn base64_fake_psd_decode_failure() {
    // 假 PSD 解码失败 → 明确报错（未存合成图的 PSD 同路）
    let dir = tempfile::tempdir().unwrap();
    let psd = dir.path().join("假.psd");
    fs::write(&psd, b"not a psd").unwrap();
    let size = fs::metadata(&psd).unwrap().len();
    let err = base64_from_origin(&psd, "psd", size).unwrap_err();
    assert!(err.contains("解码失败"), "{err}");
  }

  #[test]
  fn refresh_and_custom_thumbnail_regenerate() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-A", "图.png", "root");
    // refresh：从 origin 重生成 + 数据重算（2026-10-08 托底扩展），thumbFrame 不动
    let r = refresh_thumbnail(&mut h.lib, "AN-A").unwrap();
    let thumb_path = r.thumb_path.expect("图片应重生成内容缩略图");
    assert!(thumb_path.ends_with("图_thumbnail.webp"), "{thumb_path}");
    assert!(PathBuf::from(&thumb_path).is_file());
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

  #[cfg(windows)]
  #[test]
  fn copy_thumbnail_source_locates_thumb_and_format_icon() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-A", "图.png", "root");
    // info 缩略图存在 → 源 = info 缩略图
    refresh_thumbnail(&mut h.lib, "AN-A").unwrap();
    let src = thumbnail_source(&h.lib, "AN-A").unwrap();
    assert!(src.ends_with("图_thumbnail.webp"), "{}", src.display());
    assert!(src.is_file());
    // 缩略图文件缺失 → 明确报错（提示刷新）
    fs::remove_file(&src).unwrap();
    let err = thumbnail_source(&h.lib, "AN-A").unwrap_err();
    assert!(err.contains("缩略图文件缺失"), "{err}");
    // 无内容缩略图（fla）→ 回退格式图标缓存（无缓存现场 shell 提取）
    add_asset(&mut h, "AN-B", "工程.fla", "root");
    let icon = thumbnail_source(&h.lib, "AN-B").unwrap();
    assert_eq!(icon, h.lib.root.join("cache/thumbs/fla.webp"));
    assert!(icon.is_file());
    // 资产不存在 → 报错
    assert!(thumbnail_source(&h.lib, "AN-不存在").is_err());
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
    assert!(out.join("a_1.png").is_file(), "重名追加 _序号");
    assert!(export_assets(&src, &["AN-A".into()], "不存在目录", &mut sink, &no).is_err());
  }

  #[test]
  fn export_folder_keeps_hierarchy_and_display_names() {
    let mut h = make_lib();
    let sub = Folder { id: "F-SUB".into(), name: "子夹".into(), parent: Some("F-TOP".into()), block: None, icon: None, kind: None, color: None, groups: None, order: 0 };
    let top = Folder { id: "F-TOP".into(), name: "顶夹".into(), parent: None, block: None, icon: None, kind: None, color: None, groups: None, order: 0 };
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
    for f in ["走路_1.png", "走路_2.png", "走路_3.png"] {
      assert!(dir.join(f).is_file(), "{f} 应拷入");
    }
    assert!(!out.join("走路_1.png").is_file(), "帧不散落到目标根");
  }

  #[test]
  fn export_folder_sequence_and_anim_unpack_regression() {
    let mut h = make_lib();
    let top = Folder { id: "F-TOP".into(), name: "顶夹".into(), parent: None, block: None, icon: None, kind: None, color: None, groups: None, order: 0 };
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
    assert!(base.join("走路").join("走路_1.png").is_file(), "origin 显示名建文件夹");
    assert!(base.join("别名").join("走路_2.png").is_file(), "链接别名建文件夹装同组帧");
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
    assert!(seq.ends_with("走路_1.png"), "{seq:?}");
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
