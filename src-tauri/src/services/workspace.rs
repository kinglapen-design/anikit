//! 索引式工作空间服务（PRD 02，步骤 1 只读底座）：真实目录列举 + .anikit 缩略图缓存 +
//! 磁盘监听热刷新。写操作（新建 / 重命名 / 删除 / 拖拽移动）是步骤 2，本模块一概不做。
//! 缩略图缓存键 = 相对路径 + mtime + 大小（无资产 ID 可挂，PRD 技术口径），缓存放
//! workspace/.anikit/thumbs/，可整体删除重建；新鲜度 = 文件名内嵌的 mtime+size 与当前文件
//! 一致（无 sidecar 索引）；.anikit 自身变更不触发刷新（防缓存写回自激）。

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::adapters::inbox_watch;
use crate::commands::library::AppState;
use crate::core::space::WORKSPACE_DIR;
use crate::services::thumbnail;

/// 磁盘实况热刷新事件名（负载 = ()；.anikit 缓存写回自激已滤，前端去抖重扫）
pub const FS_CHANGED_EVENT: &str = "workspace-fs-changed";
/// 缩略图批次完成事件名（负载 = Vec<ThumbReady>，仅本趟实际生成成功的条目）
pub const THUMBS_EVENT: &str = "workspace-thumbs-ready";

/// 工作空间缓存目录名（workspace/.anikit/；缩略图 + 转码产物，可随时整体删除重建）
pub const CACHE_DIR: &str = ".anikit";

/// 目录单条（磁盘实况语义：无标签 / 评分等附加数据，PRD 02）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WsEntry {
  /// 条目名（文件 / 文件夹名）
  pub name: String,
  /// 相对 workspace 根的路径（/ 分隔）
  pub rel_path: String,
  pub is_dir: bool,
  /// 文件字节数；目录恒 0
  pub size: u64,
  /// 文件修改时间（Unix 毫秒）；目录恒 0
  pub mtime_ms: i64,
  /// 图片像素宽高（内容魔数探测，PRD 02：卡片布局按真实比例自适应，无填充裁切）；
  /// 非图片 / 探测失败 / 目录 = null
  pub w: Option<u32>,
  pub h: Option<u32>,
  /// 小写无点扩展名；目录 / 无扩展名 = ""
  pub ext: String,
  /// 缩略图缓存绝对路径：仅缓存新鲜（文件名内嵌 mtime+size 与当前文件一致）时给，否则 null
  pub thumb_path: Option<String>,
}

/// 缩略图生成结果（workspace-thumbs-ready 负载条目）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbReady {
  /// 源文件绝对路径
  pub abs_path: String,
  /// 缩略图缓存绝对路径
  pub thumb_path: String,
}

// ---------- 路径解析（防穿越） ----------

/// rel 解析到 workspace 根内的绝对路径：拒绝 .. 与绝对路径（组件级过滤，防穿越），
/// 存在则规范化（消符号链接 / 短名越界）；不存在按词法拼接结果用（列举瞬间被删不报错成越界）
fn resolve_under(root: &Path, rel: &str) -> Result<PathBuf, String> {
  let mut out = root.to_path_buf();
  for comp in Path::new(rel).components() {
    match comp {
      Component::Normal(c) => out.push(c),
      Component::CurDir => {}
      _ => return Err("路径不合法：只能访问工作空间内的目录".to_string()),
    }
  }
  let canon_root = root.canonicalize().map_err(|e| format!("工作空间不可用：{e}"))?;
  match out.canonicalize() {
    Ok(c) if c.starts_with(&canon_root) => Ok(c),
    Ok(_) => Err("路径越界：不允许访问工作空间外的目录".to_string()),
    Err(_) => Ok(out),
  }
}

/// 绝对路径是否位于 root 内（规范化后前缀判定；不存在 / 越界返回 false）
/// pub(crate)：commands::media 按路径取播放源时复用同一越界口径
pub(crate) fn is_under(root: &Path, abs: &Path) -> bool {
  match (root.canonicalize(), abs.canonicalize()) {
    (Ok(canon_root), Ok(canon)) => canon.starts_with(canon_root),
    _ => false,
  }
}

/// root 内绝对路径 → / 分隔相对路径（canonicalize 后剥前缀，保证与 list_dir 的 relPath 同源）
fn rel_of(root: &Path, abs: &Path) -> Option<String> {
  let canon_root = root.canonicalize().ok()?;
  let canon = abs.canonicalize().ok()?;
  let rel = canon.strip_prefix(canon_root).ok()?;
  Some(rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"))
}

// ---------- 目录列举 ----------

/// 列举 workspace 根下 rel 目录：目录在前、同名不区分大小写排序；.anikit 缓存目录隐藏；
/// thumbPath 仅缓存新鲜时给。rel 空串 = 根。越界 / 目录不存在给中文错（命令层降级为空列表）
pub fn list_dir(root: &Path, rel: &str) -> Result<Vec<WsEntry>, String> {
  let abs = resolve_under(root, rel)?;
  let rd = fs::read_dir(&abs).map_err(|e| format!("目录读取失败：{e}"))?;
  let mut entries = Vec::new();
  for e in rd.flatten() {
    let path = e.path();
    let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else { continue };
    if name == CACHE_DIR {
      continue; // .anikit 缓存目录不出现在磁盘实况里
    }
    if let Some(en) = entry(root, rel, &path, &name, path.is_dir()) {
      entries.push(en);
    }
  }
  entries.sort_by(|a, b| {
    b.is_dir.cmp(&a.is_dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())).then_with(|| a.name.cmp(&b.name))
  });
  Ok(entries)
}

/// 单条目：文件补 大小 / mtime / 扩展名 / 新鲜缩略图；目录 size/mtime 恒 0、无缩略图
fn entry(root: &Path, rel_base: &str, path: &Path, name: &str, is_dir: bool) -> Option<WsEntry> {
  let rel = if rel_base.is_empty() { name.to_string() } else { format!("{rel_base}/{name}") };
  let meta = fs::metadata(path).ok();
  let size = if is_dir { 0 } else { meta.as_ref().map(|m| m.len()).unwrap_or(0) };
  let mtime_ms = if is_dir {
    0
  } else {
    meta.and_then(|m| m.modified().ok())
      .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
      .map(|d| d.as_millis() as i64)
      .unwrap_or(0)
  };
  let ext = if is_dir {
    String::new()
  } else {
    path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
  };
  let thumb_path = if is_dir { None } else { fresh_thumb_path(root, &rel, mtime_ms, size) };
  // 图片探测宽高（魔数不认扩展名，壁纸站扩展名撒谎场景同 thumbnail::probe 口径）；
  // 只读文件头，探测失败静默 null——布局回退方盒（useGridLayout 既定链）
  let (w, h) = if is_dir { (None, None) } else { crate::services::thumbnail::probe(path).map(|(w, h)| (Some(w), Some(h))).unwrap_or((None, None)) };
  Some(WsEntry { name: name.to_string(), rel_path: rel, is_dir, size, mtime_ms, w, h, ext, thumb_path })
}

// ---------- 缩略图缓存 ----------

fn thumbs_dir(ws_root: &Path) -> PathBuf {
  ws_root.join(CACHE_DIR).join("thumbs")
}

/// rel 的 url-safe 编码：保留 [A-Za-z0-9._-]，其余字节（含 / 与非 ASCII）%XX 大写——
/// 单射 ⇒ 文件名唯一，且 Windows 文件名合法
fn encode_rel(rel: &str) -> String {
  let mut out = String::with_capacity(rel.len());
  for &b in rel.as_bytes() {
    if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-') {
      out.push(b as char);
    } else {
      out.push_str(&format!("%{b:02X}"));
    }
  }
  out
}

/// 缓存文件名：{url-safe 编码的 relPath}-{mtime_ms}-{size}.{webp|png}
fn cache_name(rel: &str, mtime_ms: i64, size: u64, ext: &str) -> String {
  format!("{}-{}-{}.{ext}", encode_rel(rel), mtime_ms, size)
}

/// 缓存命中（新鲜）判定：按当前 mtime+size 重算期望文件名，存在即新鲜
/// （webp / png 两种编码产物都认——编码失败退 PNG 的旧产物同样新鲜）
fn fresh_thumb_path(ws_root: &Path, rel: &str, mtime_ms: i64, size: u64) -> Option<String> {
  for ext in ["webp", "png"] {
    let dest = thumbs_dir(ws_root).join(cache_name(rel, mtime_ms, size, ext));
    if dest.is_file() {
      return Some(dest.to_string_lossy().into_owned());
    }
  }
  None
}

/// 单条缩略图生成：解码链复用 thumbnail（图片魔数 / PSD / SVG / 视频首帧 / 音频波形，
/// gif/webp 动图取首帧）；成功落盘 .anikit/thumbs/ 并顺手清理同 relPath 旧缓存（源已变化），
/// 解码 / 编码 / 落盘失败返回 None（调用方跳过该条目）
fn generate_thumb(ws_root: &Path, abs: &Path, rel: &str, mtime_ms: i64, size: u64) -> Option<PathBuf> {
  let img = thumbnail::load_origin(abs)?;
  let thumb = thumbnail::shrink_to_thumb(img);
  let (bytes, ext) = thumbnail::encode_thumb_bytes(&thumb)?;
  let dir = thumbs_dir(ws_root);
  fs::create_dir_all(&dir).ok()?;
  let dest = dir.join(cache_name(rel, mtime_ms, size, ext));
  fs::write(&dest, bytes).ok()?;
  purge_stale(&dir, rel, &dest);
  Some(dest)
}

/// 同 relPath 前缀的旧缓存清理：源文件变化后旧名文件永远不再新鲜，留着只膨胀缓存目录
fn purge_stale(dir: &Path, rel: &str, keep: &Path) {
  let prefix = format!("{}-", encode_rel(rel));
  let Ok(rd) = fs::read_dir(dir) else { return };
  for e in rd.flatten() {
    let p = e.path();
    if e.file_name().to_string_lossy().starts_with(&prefix) && p != keep {
      let _ = fs::remove_file(&p);
    }
  }
}

/// 逐一生成缺失 / 过期的缩略图（过滤 workspace 外路径与目录；已新鲜跳过；失败条目跳过），
/// 返回本趟实际生成成功的清单——调用方据此 emit workspace-thumbs-ready（可能为空数组）
pub fn request_thumbs(root: &Path, abs_paths: Vec<PathBuf>) -> Vec<ThumbReady> {
  let mut out = Vec::new();
  for abs in abs_paths {
    if !abs.is_file() || !is_under(root, &abs) {
      continue;
    }
    let Some(rel) = rel_of(root, &abs) else { continue };
    let Some(meta) = fs::metadata(&abs).ok() else { continue };
    let size = meta.len();
    let mtime_ms =
      meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64).unwrap_or(0);
    if fresh_thumb_path(root, &rel, mtime_ms, size).is_some() {
      continue;
    }
    match generate_thumb(root, &abs, &rel, mtime_ms, size) {
      Some(dest) => out.push(ThumbReady {
        abs_path: abs.to_string_lossy().into_owned(),
        thumb_path: dest.to_string_lossy().into_owned(),
      }),
      None => log::warn!("工作空间缩略图生成失败（已跳过）：{}", abs.display()),
    }
  }
  out
}

// ---------- 监听生命周期 ----------

/// 启动工作空间监听（空间打开唯一入口 activate_space 调用；workspace/ 目录缺失则不启）：
/// 任何路径事件（.anikit 缓存写回自激已滤）→ emit workspace-fs-changed，前端去抖重扫。
/// 先停旧监听（切空间防御性清理）；全部失败记日志降级，不影响空间打开
pub fn start_watch(app: &AppHandle, space_root: &Path) {
  stop_watch(app);
  let dir = space_root.join(WORKSPACE_DIR);
  if !dir.is_dir() {
    return;
  }
  let app_emit = app.clone();
  let handle = match inbox_watch::spawn(
    dir,
    |_| {}, // on_ready = 收件箱"写入完成"语义，工作空间不需要（步骤 2 写操作回声另定）
    move |paths| {
      if paths.iter().any(|p| !in_cache_dir(p)) {
        if let Err(e) = app_emit.emit(FS_CHANGED_EVENT, ()) {
          log::debug!("工作空间实况事件推送失败（前端可能已关闭）：{e}");
        }
      }
    },
  ) {
    Ok(h) => h,
    Err(e) => {
      log::error!("工作空间监听启动失败：{e}");
      return;
    }
  };
  match app.state::<AppState>().workspace.lock() {
    Ok(mut slot) => *slot = Some(handle),
    Err(e) => {
      log::error!("工作空间监听登记失败：{e}");
      drop(handle); // 登记失败即回收（Drop 自动 stop）
    }
  }
}

/// 停止工作空间监听（切空间 / 应用退出时调）；句柄 take 后 Drop 自动 stop（join 回收线程）
pub fn stop_watch(app: &AppHandle) {
  let handle = app.state::<AppState>().workspace.lock().ok().and_then(|mut slot| slot.take());
  drop(handle);
}

/// 路径是否位于 .anikit 缓存目录内（缩略图 / 转码产物写回会触发监听，滤掉防自激）
fn in_cache_dir(path: &Path) -> bool {
  path.components().any(|c| c.as_os_str() == CACHE_DIR)
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 造一个工作空间根 + 一张真 png（800×600）
  fn make_ws() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("workspace");
    fs::create_dir_all(&root).unwrap();
    (dir, root)
  }

  fn make_png(path: &Path, w: u32, h: u32) {
    image::RgbaImage::from_pixel(w, h, image::Rgba([200, 30, 30, 255])).save(path).unwrap();
  }

  /// 列举：目录在前、同名不区分大小写、.anikit 隐藏、字段齐全
  #[test]
  fn list_dir_sorts_hides_cache_and_fills_fields() {
    let (_dir, root) = make_ws();
    fs::create_dir_all(root.join("子目录")).unwrap();
    fs::create_dir_all(root.join(".anikit/thumbs")).unwrap();
    make_png(&root.join("Beta.png"), 100, 80);
    make_png(&root.join("alpha.png"), 100, 80);
    fs::write(root.join("无扩展名"), b"x").unwrap();

    let list = list_dir(&root, "").unwrap();
    let names: Vec<&str> = list.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["子目录", "alpha.png", "Beta.png", "无扩展名"], "目录在前、其余按名不区分大小写");
    assert!(!names.contains(&".anikit"), ".anikit 缓存目录应隐藏");

    let alpha = list.iter().find(|e| e.name == "alpha.png").unwrap();
    assert_eq!(alpha.rel_path, "alpha.png");
    assert!(!alpha.is_dir);
    assert!(alpha.size > 0);
    assert!(alpha.mtime_ms > 0);
    assert_eq!(alpha.ext, "png");
    assert_eq!(alpha.thumb_path, None, "未生成缓存时 thumbPath=null");

    let dir_en = list.iter().find(|e| e.is_dir).unwrap();
    assert_eq!((dir_en.size, dir_en.mtime_ms), (0, 0));
    assert_eq!(dir_en.ext, "");
    let noext = list.iter().find(|e| e.name == "无扩展名").unwrap();
    assert_eq!(noext.ext, "");
  }

  /// rel 越界拒绝：.. 、内嵌 .. 、绝对路径一律中文错；不存在的子目录按词法结果走（不越界）
  #[test]
  fn resolve_under_rejects_traversal() {
    let (_dir, root) = make_ws();
    for rel in ["..", "a/../../b", "../.."] {
      assert!(list_dir(&root, rel).is_err(), "rel={rel} 应被拒绝");
    }
    let abs_outside = tempfile::tempdir().unwrap().path().to_path_buf();
    assert!(list_dir(&root, &abs_outside.to_string_lossy()).is_err(), "绝对路径应被拒绝");
    assert!(list_dir(&root, "不存在/子").is_err(), "不存在的目录读取出错（非越界）");
    assert!(resolve_under(&root, "").is_ok(), "空 rel = 根");
  }

  /// url-safe 编码：非 ASCII 与 / 转 %XX，单射（不同 rel 不同名）
  #[test]
  fn encode_rel_is_url_safe_and_injective() {
    let enc = encode_rel("子目录/图a.png");
    assert!(enc.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '%')));
    assert!(enc.contains("%E5%AD%90") && enc.contains("%2F"));
    assert_ne!(encode_rel("a/b"), encode_rel("a%2Fb"), "编码须单射：a/b 与 a%2Fb 不同名");
    assert_ne!(encode_rel("图.png"), encode_rel("图png"));
    assert_eq!(cache_name("a.png", 1720000000000, 12345, "webp"), "a.png-1720000000000-12345.webp");
  }

  /// 缓存新鲜度：生成 → list 命中；源变化（大小变）→ 旧缓存失鲜、重新生成后再次命中
  #[test]
  fn thumb_freshness_tracks_mtime_and_size() {
    let (_dir, root) = make_ws();
    fs::create_dir_all(root.join("子")).unwrap();
    let f = root.join("子/图a.png");
    make_png(&f, 800, 600);

    assert_eq!(list_dir(&root, "子").unwrap()[0].thumb_path, None, "无缓存时 null");
    let ready = request_thumbs(&root, vec![f.clone()]);
    assert_eq!(ready.len(), 1, "首趟应生成 1 条");
    assert!(Path::new(&ready[0].thumb_path).is_file());
    // 产物是合法 webp、短边 320
    assert_eq!(image::image_dimensions(&ready[0].thumb_path).unwrap(), (426, 320));
    let hit = list_dir(&root, "子").unwrap()[0].thumb_path.clone();
    assert_eq!(hit.as_deref(), Some(ready[0].thumb_path.as_str()), "生成后 list 命中新鲜缓存");

    // 源变化（重写成不同尺寸的有效图、大小变）→ 失鲜；thumb_path 文件名嵌 rel+mtime+size
    make_png(&f, 500, 500);
    let stale = request_thumbs(&root, vec![f.clone()]);
    assert_eq!(stale.len(), 1, "过期后应重新生成");
    assert_ne!(stale[0].thumb_path, ready[0].thumb_path, "新缓存文件名应随 mtime/size 变化");
    assert_eq!(image::image_dimensions(&stale[0].thumb_path).unwrap(), (320, 320), "新缓存应对应新源尺寸");
    let dir = thumbs_dir(&root);
    let leftovers: Vec<_> = fs::read_dir(&dir).unwrap().flatten().collect();
    assert_eq!(leftovers.len(), 1, "旧缓存应被顺手清理");

    // 二次请求全新鲜 → 空结果（命中跳过）
    assert!(request_thumbs(&root, vec![f]).is_empty(), "全新鲜时应无事可做");
  }

  /// 请求过滤：workspace 外路径与目录跳过；解码失败条目跳过且不报错
  #[test]
  fn request_thumbs_filters_and_skips_failures() {
    let (dir, root) = make_ws();
    let inside = root.join("内.png");
    make_png(&inside, 64, 48);
    let outside = dir.path().join("外.png");
    make_png(&outside, 64, 48);
    let sub = root.join("子目录");
    fs::create_dir_all(&sub).unwrap();
    let fla = root.join("工程.fla");
    fs::write(&fla, b"fake fla").unwrap();

    let ready = request_thumbs(&root, vec![inside, outside, sub, fla]);
    assert_eq!(ready.len(), 1, "仅 workspace 内可解码的条目生成缩略图");
    assert!(ready[0].abs_path.ends_with("内.png"));
    assert!(list_dir(&root, "").unwrap().iter().all(|e| e.thumb_path.is_none() || e.name == "内.png"));
  }
}
