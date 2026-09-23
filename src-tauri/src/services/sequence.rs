//! 序列帧检测与打包入库（PRD 01-2.5，阶段 4.1）。
//! 检测（宽容）：仅同目录兄弟文件成组，按"基础名 + 扩展名 + 序号位宽"分组，
//! 支持 `标题001` / `标题 (1)` / `标题_001` / `标题-001`（基础名可含其他文字，序号在末尾）；
//! 序号连续（允许少量缺帧：缺帧 ≤ max(2, 跨度/10)；同组同号不判）且 ≥ 3 才判定为序列帧；
//! 仅对缩略图管线支持的图片格式判定。
//! 入库归一：`{ID}.info/frames/` 存放全部帧，统一重命名为 `标题_序号.扩展名`（序号补齐位宽，
//! 库内只维护这一种格式）；metadata 记 fps（默认 24）/ frameCount / thumbFrame（默认首帧文件名）。
//! 查重：组 hash = 各帧 sha256 按序号升序串联后再 sha256，撞 hash 走既有冲突挂起流程。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::core::json_io::write_json;
use crate::core::models::{AssetMeta, DATA_VERSION};
use crate::core::paths;
use crate::services::import::{self, PendingFile, Source};
use crate::services::thumbnail;

/// 判定为序列帧的最小帧数（PRD 01-2.5）
pub const MIN_FRAMES: usize = 3;

/// metadata.fps 默认值（用户 2026-07-22 定案）
pub const DEFAULT_FPS: f64 = 24.0;

/// 帧引用：PendingFile 下标 + 序号
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameRef {
  pub file_idx: usize,
  pub number: u32,
}

/// 一组序列帧（同目录 / 同基础名 / 同扩展名 / 同序号位宽）
#[derive(Debug, Clone, PartialEq)]
pub struct SequenceGroup {
  /// 基础名（资产标题）
  pub name: String,
  /// 扩展名（小写，无点）
  pub ext: String,
  /// 序号位宽（入库补齐用）
  pub num_width: usize,
  /// 帧清单（按序号升序）
  pub frames: Vec<FrameRef>,
}

/// 解析帧文件名词干（去扩展名）→ （基础名， 序号， 位宽）。
/// 支持四种形态：标题001 / 标题 (1) / 标题_001 / 标题-001；基础名为空或序号溢出 u32 不判。
pub fn parse_frame_name(stem: &str) -> Option<(String, u32, usize)> {
  // 形态 标题 (1)（Windows 复制命名；无空格 标题(1) 同样宽容命中）
  if let Some((head, digits)) = stem.strip_suffix(')').and_then(|s| s.rsplit_once('(')) {
    if let Some(number) = parse_digits(digits) {
      let base = head.trim_end();
      if !base.is_empty() {
        return Some((base.to_string(), number, digits.len()));
      }
    }
  }
  // 其余形态：词干末尾连续数字，剥离尾部一个分隔符（_ / - / 空格）
  let digit_len = stem.bytes().rev().take_while(|b| b.is_ascii_digit()).count();
  if digit_len == 0 {
    return None;
  }
  let number = parse_digits(&stem[stem.len() - digit_len..])?;
  let base = stem[..stem.len() - digit_len].trim_end_matches(['_', '-', ' ']);
  if base.is_empty() {
    return None;
  }
  Some((base.to_string(), number, digit_len))
}

/// 纯数字串 → u32（空串 / 非全数字 / 溢出 None）
fn parse_digits(digits: &str) -> Option<u32> {
  if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
    return None;
  }
  digits.parse::<u64>().ok().and_then(|n| u32::try_from(n).ok())
}

/// 缩略图管线支持的图片扩展名（与 Cargo image 特性一致：png / jpeg / gif / webp / bmp）
fn is_image_ext(ext: &str) -> bool {
  matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp")
}

/// 检测序列帧分组：输入展开后的文件清单，返回判定的组（组内帧按序号升序）。
/// 分组键 = （所在目录， 基础名， 扩展名， 位宽）——仅同目录兄弟文件成组。纯函数不碰磁盘。
pub fn detect_groups(files: &[PendingFile]) -> Vec<SequenceGroup> {
  let mut map: HashMap<(PathBuf, String, String, usize), Vec<FrameRef>> = HashMap::new();
  for (idx, f) in files.iter().enumerate() {
    let Some(ext) = f.path.extension().map(|s| s.to_string_lossy().to_lowercase()) else {
      continue;
    };
    if !is_image_ext(&ext) {
      continue;
    }
    let Some(stem) = f.path.file_stem().map(|s| s.to_string_lossy().into_owned()) else {
      continue;
    };
    let Some((base, number, num_width)) = parse_frame_name(&stem) else {
      continue;
    };
    let parent = f.path.parent().map(Path::to_path_buf).unwrap_or_default();
    map.entry((parent, base, ext, num_width)).or_default().push(FrameRef { file_idx: idx, number });
  }
  map
    .into_iter()
    .filter_map(|((_, name, ext, num_width), frames)| {
      continuous(frames).map(|frames| SequenceGroup { name, ext, num_width, frames })
    })
    .collect()
}

/// 连续判定（PRD：序号连续，允许少量缺帧）：≥ MIN_FRAMES、序号唯一（同号 = 命名冲突不判）、
/// 缺帧 ≤ max(2, 跨度/10)（容忍阈值 PRD 未量化，2026-07-22 定案取值，04_开发日志有记）
fn continuous(mut frames: Vec<FrameRef>) -> Option<Vec<FrameRef>> {
  if frames.len() < MIN_FRAMES {
    return None;
  }
  frames.sort_by_key(|f| f.number);
  if frames.windows(2).any(|w| w[0].number == w[1].number) {
    return None;
  }
  let span = (frames.last().unwrap().number - frames.first().unwrap().number + 1) as usize;
  if span - frames.len() > (span / 10).max(2) {
    return None;
  }
  Some(frames)
}

/// 组 hash：各帧 sha256 按序号升序串联后再 sha256（输出与单文件同格式 "sha256:{hex}"）
pub fn group_hash(frame_hashes: &[&str]) -> String {
  let mut hasher = Sha256::new();
  for h in frame_hashes {
    hasher.update(h.as_bytes());
  }
  format!("sha256:{:x}", hasher.finalize())
}

/// 库内归一帧文件名：标题_序号.扩展名（序号补齐位宽；PRD：库内只维护这一种格式；
/// 2026-09-23 序号格式自 - 改 _——存量 - 帧名 parse_frame_name 兼容，重排序号时自然迁入）
pub fn frame_name(name: &str, number: u32, num_width: usize, ext: &str) -> String {
  format!("{name}_{number:0>num_width$}.{ext}")
}

/// 序列帧资产判定（导出 / 重命名共用）：origin 不在 info 根（指向 frames/ 内帧），frames/ 目录存在。
/// 动图解包同写 frames/，但其 origin 在 info 根是实体文件 → 不误判
pub(crate) fn frames_dir_if_sequence(assets_dir: &Path, asset_id: &str, meta: &AssetMeta) -> Option<PathBuf> {
  if paths::info_dir(assets_dir, asset_id).join(&meta.origin_name).is_file() {
    return None;
  }
  let dir = paths::frames_dir(assets_dir, asset_id);
  if dir.is_dir() { Some(dir) } else { None }
}

/// 序列帧整组改词干（rename_origin 序列帧分支，2026-08-14 用户定：序列帧重命名帧文件跟着改）：
/// frames/ 内帧按"新词干_序号"改名（保留序号尾与扩展名），thumb_frame / origin_name / name 同步；
/// 磁盘改名成对入 renames（撤销登记用）。重取词干靠"末尾 _序号"解析，不依赖旧词干匹配——
/// 历史遗留（此功能前改名只改了 metadata）的脱钩状态也能归位，originName 无帧匹配时回退指到封面帧
pub(crate) fn rename_frames(
  frames_dir: &Path,
  meta: &mut AssetMeta,
  new_name: &str,
  renames: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<(), String> {
  let new_stem = Path::new(new_name).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
  if new_stem.is_empty() {
    return Ok(());
  }
  let old_origin = meta.origin_name.clone();
  let old_thumb_frame = meta.thumb_frame.clone();
  let mut names: Vec<String> = fs::read_dir(frames_dir)
    .map_err(|e| format!("帧目录读取失败：{e}"))?
    .flatten()
    .filter(|e| e.path().is_file())
    .map(|e| e.file_name().to_string_lossy().into_owned())
    .collect();
  names.sort(); // 确定性顺序（撤销按序成对反转）
  let mut origin_matched = false;
  for fname in names {
    let Some(new_fname) = restem_frame(&fname, &new_stem) else { continue };
    if new_fname == fname {
      origin_matched = origin_matched || fname == old_origin;
      continue;
    }
    let old = frames_dir.join(&fname);
    let new = frames_dir.join(&new_fname);
    fs::rename(&old, &new).map_err(|e| format!("帧文件改名失败：{e}"))?;
    renames.push((old, new));
    if fname == old_origin {
      meta.origin_name = new_fname.clone();
      origin_matched = true;
    }
    if old_thumb_frame.as_deref() == Some(fname.as_str()) {
      meta.thumb_frame = Some(new_fname);
    }
  }
  if !origin_matched {
    // 旧 originName 已与帧脱钩（2026-08-14 前改名遗留）：回退指到封面帧
    if let Some(tf) = meta.thumb_frame.clone() {
      meta.origin_name = tf;
    }
  }
  meta.name = new_stem;
  Ok(())
}

/// 帧文件改词干：`{词干}_{序号}.{扩展名}` → 保留序号尾与扩展名换词干；不合帧格式返回 None
///（2026-09-23 归一名自 `-` 改 `_`：存量 `-` 帧名兼容解析，改名一律产出 `_` 新格式）
fn restem_frame(file_name: &str, new_stem: &str) -> Option<String> {
  let (stem, ext) = file_name.rsplit_once('.')?;
  let dash = stem.rfind(['-', '_'])?;
  let digits = &stem[dash + 1..];
  if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
    return None;
  }
  Some(format!("{new_stem}_{digits}.{ext}"))
}

/// 序列帧打包建档（锁外重活，与 build_asset_with_id 同纪律）：建 `{ID}.info/frames/` →
/// 逐帧入库并归一重命名 → 首帧探测宽高 → 首帧生成缩略图（复用图片管线）→ 写 metadata.json。
/// 路径全由新 ID 推导，不依赖共享状态。
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_sequence_asset(
  assets_dir: &Path,
  name: &str,
  ext: &str,
  num_width: usize,
  frames: &[(PathBuf, u32)],
  hash: &str,
  asset_id: &str,
  source: Source,
  imported_from: Option<&str>,
) -> Result<AssetMeta, String> {
  let info_dir = paths::info_dir(assets_dir, asset_id);
  let frames_dir = paths::frames_dir(assets_dir, asset_id);
  fs::create_dir_all(&frames_dir).map_err(|e| format!("info 文件夹创建失败：{e}"))?;

  let mut size = 0u64;
  let mut thumb_frame = String::new();
  // 创建时间 = 首帧源文件磁盘创建时间（store_file 的 Move 分支会删源，须在循环前读）
  let created_at = frames.first().and_then(|(src, _)| import::source_created_at(src));
  for (src, number) in frames {
    let dest_name = frame_name(name, *number, num_width, ext);
    size += fs::metadata(src).map(|m| m.len()).unwrap_or(0);
    import::store_file(src, &frames_dir.join(&dest_name), source)?;
    if thumb_frame.is_empty() {
      thumb_frame = dest_name;
    }
  }

  let cover = frames_dir.join(&thumb_frame);
  let (width, height) = thumbnail::probe(&cover).map(|(w, h)| (Some(w), Some(h))).unwrap_or((None, None));
  let thumb = thumbnail::generate(&info_dir, &cover, name);
  let now = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
  let meta = AssetMeta {
    version: DATA_VERSION,
    id: asset_id.to_string(),
    name: name.to_string(),
    origin_name: thumb_frame.clone(),
    ext: ext.to_string(),
    size,
    hash: hash.to_string(),
    width,
    height,
    duration: None,
    bpm: None,
    rating: 0,
    links: Vec::new(),
    imported_at: now.clone(),
    imported_from: imported_from.map(|s| s.to_string()),
    created_at,
    modified_at: now,
    tags: Vec::new(),
    note: String::new(),
    descriptions: Vec::new(),
    note_count: None,
    thumbnail: thumb.as_ref().map(|t| t.name.clone()),
    thumb_width: thumb.as_ref().map(|t| t.width),
    thumb_height: thumb.as_ref().map(|t| t.height),
    colors: thumb.map(|t| t.colors).unwrap_or_default(),
    fps: Some(DEFAULT_FPS),
    frame_count: Some(frames.len() as u32),
    thumb_frame: Some(thumb_frame),
    hidden: None,
    thumb_asset_id: None,
    r#type: None,
    source: None,
    start: None,
    end: None,
  };
  write_json(&info_dir.join("metadata.json"), &meta).map_err(|e| e.to_string())?;
  Ok(meta)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library;

  fn pf(path: &str) -> PendingFile {
    PendingFile { path: PathBuf::from(path), chain: Vec::new() }
  }

  /// 造一组假路径（不同目录用前缀区分）
  fn files(paths: &[&str]) -> Vec<PendingFile> {
    paths.iter().map(|p| pf(p)).collect()
  }

  #[test]
  fn parse_frame_name_accepts_four_forms() {
    assert_eq!(parse_frame_name("标题001"), Some(("标题".to_string(), 1, 3)));
    assert_eq!(parse_frame_name("标题 (1)"), Some(("标题".to_string(), 1, 1)));
    assert_eq!(parse_frame_name("标题_007"), Some(("标题".to_string(), 7, 3)));
    assert_eq!(parse_frame_name("标题-42"), Some(("标题".to_string(), 42, 2)));
    // 基础名可含其他文字（含数字与分隔符），序号在末尾
    assert_eq!(parse_frame_name("第2话 走路-009"), Some(("第2话 走路".to_string(), 9, 3)));
    assert_eq!(parse_frame_name("走路(3)"), Some(("走路".to_string(), 3, 1)));
  }

  #[test]
  fn parse_frame_name_rejects_non_frames() {
    assert_eq!(parse_frame_name("封面"), None); // 无序号
    assert_eq!(parse_frame_name("001"), None); // 基础名为空
    assert_eq!(parse_frame_name("(1)"), None); // 基础名为空
    assert_eq!(parse_frame_name("2024报告"), None); // 序号不在末尾
    assert_eq!(parse_frame_name("标题99999999999"), None); // 序号溢出 u32
  }

  #[test]
  fn detect_groups_requires_three_and_continuity() {
    // ≥3 且连续 → 成组（帧按序号升序）
    let fs = files(&["/a/标题003.png", "/a/标题001.png", "/a/标题002.png"]);
    let groups = detect_groups(&fs);
    assert_eq!(groups.len(), 1);
    assert_eq!((groups[0].name.as_str(), groups[0].num_width), ("标题", 3));
    assert_eq!(groups[0].frames.iter().map(|f| f.number).collect::<Vec<_>>(), vec![1, 2, 3]);

    // <3 不判
    assert!(detect_groups(&files(&["/a/标题001.png", "/a/标题002.png"])).is_empty());
  }

  #[test]
  fn detect_groups_allows_small_gaps_rejects_sparse_and_dup() {
    // 缺 1 帧（跨度 5 缺 1 ≤ 阈值）→ 成组
    let fs = files(&["/a/标题001.png", "/a/标题002.png", "/a/标题004.png", "/a/标题005.png"]);
    assert_eq!(detect_groups(&fs).len(), 1);

    // 过稀（跨度 9 缺 6）→ 不判
    let fs = files(&["/a/标题01.png", "/a/标题05.png", "/a/标题09.png"]);
    assert!(detect_groups(&fs).is_empty());

    // 同组同号（标题 (1) 与 标题1 撞号）→ 不判
    let fs = files(&["/a/标题 (1).png", "/a/标题1.png", "/a/标题2.png"]);
    assert!(detect_groups(&fs).is_empty());
  }

  #[test]
  fn detect_groups_scopes_by_dir_ext_and_width() {
    let fs = files(&[
      // 同目录不同扩展名 → 两组
      "/a/跑001.png", "/a/跑002.png", "/a/跑003.png",
      "/a/跑001.jpg", "/a/跑002.jpg", "/a/跑003.jpg",
      // 不同目录不成组
      "/b/跳001.png", "/b/跳002.png", "/c/跳003.png",
      // 不同位宽各自成组
      "/a/走1.png", "/a/走2.png", "/a/走3.png",
      // 非图片格式不判
      "/a/工程001.fla", "/a/工程002.fla", "/a/工程003.fla",
    ]);
    let groups = detect_groups(&fs);
    assert_eq!(groups.len(), 3, "应按 目录+基础名+扩展名+位宽 分组：{groups:?}");
    assert!(groups.iter().all(|g| g.name != "跳" && g.name != "工程"));
  }

  #[test]
  fn group_hash_is_order_sensitive_and_stable() {
    let a = group_hash(&["sha256:aa", "sha256:bb"]);
    let b = group_hash(&["sha256:aa", "sha256:bb"]);
    let swapped = group_hash(&["sha256:bb", "sha256:aa"]);
    assert_eq!(a, b);
    assert_ne!(a, swapped);
    assert!(a.starts_with("sha256:"));
  }

  fn make_png(path: &Path, w: u32, h: u32) {
    image::RgbaImage::from_pixel(w, h, image::Rgba([30, 200, 30, 255])).save(path).unwrap();
  }

  #[test]
  fn build_sequence_asset_normalizes_frames_and_meta() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    let assets_dir = lib.assets_dir.clone();
    drop(lib);

    // 源：标题_1 / 标题_2 / 标题_3（位宽 3 入库补齐）
    let src_dir = dir.path().join("源");
    fs::create_dir_all(&src_dir).unwrap();
    let frames: Vec<(PathBuf, u32)> = (1..=3)
      .map(|n| {
        let p = src_dir.join(format!("标题_{n}.png"));
        make_png(&p, 100, 80);
        (p, n)
      })
      .collect();
    let hash = group_hash(&["sha256:x"]);
    let meta =
      build_sequence_asset(&assets_dir, "标题", "png", 3, &frames, &hash, "AN-TEST", Source::Move, None).unwrap();

    // frames/ 归一重命名（序号补齐位宽），剪切整组移走
    let info = paths::info_dir(&assets_dir, "AN-TEST");
    for n in 1..=3 {
      let name = format!("标题_00{n}.png");
      assert!(info.join("frames").join(&name).is_file(), "缺帧 {name}");
      assert!(!frames[n - 1].0.exists(), "剪切后源帧应消失");
    }
    // metadata：fps / frameCount / thumbFrame（首帧文件名）/ 宽高 / 缩略图
    assert_eq!(meta.fps, Some(24.0));
    assert_eq!(meta.frame_count, Some(3));
    assert_eq!(meta.thumb_frame.as_deref(), Some("标题_001.png"));
    assert_eq!(meta.origin_name, "标题_001.png");
    assert_eq!((meta.width, meta.height), (Some(100), Some(80)));
    assert_eq!(meta.thumbnail.as_deref(), Some("标题_thumbnail.webp"));
    assert!(info.join("标题_thumbnail.webp").is_file());
    assert!(meta.size > 0);
    // metadata.json 可回读
    let back: AssetMeta =
      crate::core::json_io::read_json(&info.join("metadata.json")).unwrap();
    assert_eq!(back, meta);
  }

  #[test]
  fn build_sequence_asset_copy_keeps_sources() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    let p1 = dir.path().join("走 (1).png");
    make_png(&p1, 10, 10);
    let frames = vec![(p1.clone(), 1), (dir.path().join("走 (2).png"), 2)];
    make_png(&frames[1].0, 10, 10);
    build_sequence_asset(&lib.assets_dir, "走", "png", 1, &frames, "sha256:y", "AN-COPY", Source::Copy, None).unwrap();
    assert!(p1.exists(), "复制后源帧应保留");
  }
}
