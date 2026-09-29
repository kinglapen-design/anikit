//! 索引式工作空间服务（PRD 02）：真实目录列举 + .anikit 缩略图缓存 + 磁盘监听热刷新
//! 与步骤 2 基础整理写操作（新建文件夹 / 重命名 / 回收站删除 / 移动 / 复制粘贴，
//! 纯磁盘语义，越权校验与 .anikit 保护同读路径）。
//! 缩略图缓存键 = 截断的 url-safe 相对路径前缀 + rel 的 FNV-1a 散列 + mtime + 大小
//!（无资产 ID 可挂，PRD 技术口径；全长有界 ≤160 字符——2026-09-26 修复：旧版整条编码路径
//! 做文件名，中文每字符 9 倍膨胀，深中文 rel 超 NTFS 255 组件上限写盘必败，踩坑 092），
//! 缓存放 workspace/.anikit/thumbs/，可整体删除重建；新鲜度 = 文件名内嵌的 mtime+size 与当前
//! 文件一致（无 sidecar 索引）；.anikit 自身变更不触发刷新（防缓存写回自激）。
//! 缩略图生成的并发 / 去重 / 事件回吐归 services::ws_thumb_queue（2026-09-26 性能优化：
//! 原「每 invoke 裸 spawn 一条线程严格串行」退役）；本模块保留缓存键口径、单条生成原语
//! 与进程级失败记忆（解码失败键集合——命中即跳过不再重试，列举时落回系统格式图标）。

use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::adapters::inbox_watch;
use crate::adapters::shell_thumb;
use crate::commands::library::AppState;
use crate::core::space::WORKSPACE_DIR;
use crate::services::thumbnail;
use crate::services::views::ThumbKind;

/// 磁盘实况热刷新事件名（负载 = ()；.anikit 缓存写回自激已滤，前端去抖重扫）
pub const FS_CHANGED_EVENT: &str = "workspace-fs-changed";
/// 缩略图批次完成事件名（负载 = Vec<ThumbReady>：本趟实际生成成功的内容缩略图 +
/// 解码失败条目的格式图标落回（thumbKind 区分），经 ws_thumb_queue 批量回吐）
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
  /// 目录条目：是否含可见子目录（2026-09-26 侧栏树展开箭头真实化——列目录时顺带单次
  /// read_dir 提前退出扫描，发现任一非 .anikit 子目录即 true，不再前端乐观显示后消箭头）；
  /// 文件恒 false
  pub has_sub_dir: bool,
  /// 文件字节数；目录恒 0
  pub size: u64,
  /// 文件修改时间（Unix 毫秒）；目录恒 0
  pub mtime_ms: i64,
  /// 文件创建时间（Unix 毫秒，2026-09-26 详情面板基础数据补）：复用同一份 metadata 零额外开销；
  /// 目录 / 平台不支持（created() Err）恒 0
  pub ctime_ms: i64,
  /// 布局比例来源宽高（PRD 02：卡片布局按真实比例自适应，无填充裁切）：图片 = 源像素
  /// （内容魔数探测）；非图片（视频/音频/字体等）缓存缩略图新鲜时 = 缩略图尺寸（与源同比例，
  /// 布局只用比例，2026-09-26 视频卡方盒裁剪修复）；探测/读取失败 / 缩略图未生成 / 目录 = null
  pub w: Option<u32>,
  pub h: Option<u32>,
  /// 小写无点扩展名；目录 / 无扩展名 = ""
  pub ext: String,
  /// 缩略图缓存绝对路径：仅缓存新鲜（文件名内嵌 mtime+size 与当前文件一致）时给，否则 null
  pub thumb_path: Option<String>,
  /// 缩略图来源（同 services::views::ThumbKind 口径）：asset = 内容缩略图；
  /// formatIcon = 系统格式图标（缩略管线解不出的类型——fla/txt/zip 等，现场 shell 提取按扩展名缓存）；
  /// 可解码但缩略图未生成为 (null, null)（后台生成后 thumbs-ready 回填）
  pub thumb_kind: Option<crate::services::views::ThumbKind>,
}

/// 缩略图生成结果（workspace-thumbs-ready 负载条目）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbReady {
  /// 源文件绝对路径
  pub abs_path: String,
  /// 缩略图缓存绝对路径
  pub thumb_path: String,
  /// 缩略图实际像素宽/高（生成时 shrink 后尺寸已知；2026-09-26：前端随回填条目 w/h，
  /// 非图片条目卡片由 300 方盒恢复真实比例）；格式图标落回项 = 图标见方边长（前端忽略）
  pub w: u32,
  pub h: u32,
  /// 来源（2026-09-26 失败记忆补）：asset = 内容缩略图；formatIcon = 解码失败条目落回的
  /// 系统格式图标（前端据此回填 thumbKind 并不再请求——thumbPath 非 null 天然滤出补发清单）
  pub thumb_kind: ThumbKind,
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
/// pub(crate)：commands::media 按路径取卡片波形 peaks 时以 rel 拼缓存键（2026-09-26 填充卡对接）
pub(crate) fn rel_of(root: &Path, abs: &Path) -> Option<String> {
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

/// 单条目：文件补 大小 / mtime / 扩展名 / 缩略图（内容缩略图 → 格式图标回退）；目录恒无缩略图，
/// 补 has_sub_dir（树展开箭头显隐用）
fn entry(root: &Path, rel_base: &str, path: &Path, name: &str, is_dir: bool) -> Option<WsEntry> {
  let rel = if rel_base.is_empty() { name.to_string() } else { format!("{rel_base}/{name}") };
  let meta = fs::metadata(path).ok();
  let size = if is_dir { 0 } else { meta.as_ref().map(|m| m.len()).unwrap_or(0) };
  let mtime_ms = if is_dir {
    0
  } else {
    meta.as_ref()
      .and_then(|m| m.modified().ok())
      .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
      .map(|d| d.as_millis() as i64)
      .unwrap_or(0)
  };
  // 创建时间（2026-09-26 详情面板基础数据）：metadata 已在手，created() 零额外 syscall；
  // 平台不支持 / 获取失败静默 0（前端不显示该行）
  let ctime_ms = if is_dir {
    0
  } else {
    meta.as_ref()
      .and_then(|m| m.created().ok())
      .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
      .map(|d| d.as_millis() as i64)
      .unwrap_or(0)
  };
  let ext = if is_dir {
    String::new()
  } else {
    path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
  };
  // 图片探测宽高（魔数不认扩展名，壁纸站扩展名撒谎场景同 thumbnail::probe 口径）；
  // 只读文件头，探测失败静默 null——布局回退方盒（useGridLayout 既定链）
  let (mut w, mut h) = if is_dir { (None, None) } else { crate::services::thumbnail::probe(path).map(|(w, h)| (Some(w), Some(h))).unwrap_or((None, None)) };
  // 缩略图来源（2026-09-17 实机定：非图片/音视频类型必须有系统格式图标，不许空盒）：
  // ① 内容缩略图缓存新鲜 = asset；② 缩略管线解不出的类型（w/h 探测失败且非音视频）=
  //   现场 shell 提取系统格式图标——exe 走文件级缓存（.anikit/thumbs/{job_key}.icon.webp，
  //   同扩展名各程序图标不同），其余按扩展名缓存 {ext}.webp（库 cache/thumbs 同口径换锚）；
  // ③ 可解码但尚未生成 = (null, null)，后台生成后 thumbs-ready 回填；
  // ④ 失败记忆命中（2026-09-26：解码失败过的键 rel+mtime+size）视同不可解码落回 ② 的格式图标——
  //   失败条目不再空盒、不再随每次重扫重复请求重复失败
  let (thumb_path, thumb_kind) = if is_dir {
    (None, None)
  } else if let Some(t) = fresh_thumb_path(root, &rel, mtime_ms, size) {
    // 非图片（视频/音频/字体等）魔数探测不出尺寸，缩略图未就绪前只能回退 300 方盒裁切
    // （2026-09-26 实机报 ws 视频卡上下被裁）：缓存缩略图与源同比例，仅读缓存文件头回填
    // 宽高（列目录逐文件 spawn ffprobe 太慢，布局只用比例故缩略图尺寸即够；image crate 的
    // image_dimensions 只读文件头不全解码）；读失败静默 null 维持方盒回退
    if w.is_none() {
      if let Ok((tw, th)) = image::image_dimensions(&t) {
        w = Some(tw);
        h = Some(th);
      }
    }
    (Some(t), Some(ThumbKind::Asset))
  } else {
    let decodable = !is_thumb_failed(root, &rel, mtime_ms, size)
      && (w.is_some()
        || crate::services::media::VIDEO_EXTS.contains(&ext.as_str())
        || crate::services::media::AUDIO_EXTS.contains(&ext.as_str()));
    if !decodable {
      let dir = thumbs_dir(root);
      // exe 走文件级图标缓存（2026-09-17 用户定：按扩展名缓存会把首个 exe 的图标串给
      // 所有 exe）：命名同内容缩略图键（job_key = 截断前缀+hash+mtime+size，内嵌 .icon
      // 中缀防与内容缩略图撞名），改版本即失效重提并顺清旧缓存；thumbKind 仍 = FormatIcon
      if ext == "exe" {
        match ensure_exe_icon(root, path, &rel, mtime_ms, size) {
          Some(p) => (Some(p.to_string_lossy().into_owned()), Some(ThumbKind::FormatIcon)),
          None => (None, None),
        }
      } else if crate::services::thumbnail::ensure_format_icon_in(&dir, path, &ext) {
        (Some(crate::services::thumbnail::format_icon_path_in(&dir, &ext).to_string_lossy().into_owned()), Some(ThumbKind::FormatIcon))
      } else {
        (None, None)
      }
    } else {
      (None, None)
    }
  };
  // 目录条目：是否含子目录（树展开箭头显隐，2026-09-26）——列目录已持各条目路径，
  // 顺带单次扫描即得真实信息，免去前端「未加载乐观显示、展开后才消箭头」
  let has_sub_dir = is_dir && has_visible_sub_dir(path);
  Some(WsEntry { name: name.to_string(), rel_path: rel, is_dir, has_sub_dir, size, mtime_ms, ctime_ms, w, h, ext, thumb_path, thumb_kind })
}

/// 目录是否含可见子目录：单次 read_dir 提前退出——发现任一非 .anikit 子目录即 true，
/// 开销可忽略；可见口径与 list_dir 一致（只滤缓存目录，隐藏 / 系统目录照常计入）；
/// 读取失败（权限 / 列举瞬间被删）按 false —— 扫不出子级就不出箭头
fn has_visible_sub_dir(path: &Path) -> bool {
  let Ok(rd) = fs::read_dir(path) else { return false };
  for e in rd.flatten() {
    let p = e.path();
    if p.file_name().is_some_and(|n| n.to_string_lossy() == CACHE_DIR) {
      continue;
    }
    if p.is_dir() {
      return true;
    }
  }
  false
}

// ---------- 缩略图缓存 ----------

fn thumbs_dir(ws_root: &Path) -> PathBuf {
  ws_root.join(CACHE_DIR).join("thumbs")
}

/// rel 的 url-safe 编码：保留 [A-Za-z0-9._-]，其余字节（含 / 与非 ASCII）%XX 大写——
/// 单射 ⇒ 唯一，且 Windows 文件名合法。**只做编码原语**：落盘文件名一律走 key_prefix +
/// fnv64hex 的有界口径（整条编码中文每字符 9 倍膨胀无上限，踩坑 092）
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

/// FNV-1a 64bit 散列（手写常量算法十几行，不加新依赖；不用 DefaultHasher——跨工具链版本
/// 结果不稳定）：缓存键可读前缀截断后靠它保唯一（不同 rel 不撞名），输出定长 16 位小写 hex
fn fnv64hex(s: &str) -> String {
  let mut h: u64 = 0xcbf2_9ce4_8422_2325;
  for &b in s.as_bytes() {
    h ^= u64::from(b);
    h = h.wrapping_mul(0x0000_0100_0000_01b3);
  }
  format!("{h:016x}")
}

/// 缓存键可读前缀长度上限（字符数，纯 ASCII）：整名 ≤ 80+1+16+1+21+1+20+扩展名 ≈ 150，
/// 离 NTFS 255 组件上限留足余量
const KEY_PREFIX_MAX: usize = 80;

/// 缓存键可读前缀：encode_rel 截断到 ≤80 字符，只截在完整 %XX 转义边界上（编码串里 '%'
/// 只出现在转义起始，回退至多以它为尾的两类半截形态即可）；截断丢掉的区分度由 hash 段兜底
fn key_prefix(rel: &str) -> String {
  let enc = encode_rel(rel);
  if enc.len() <= KEY_PREFIX_MAX {
    return enc;
  }
  let b = enc.as_bytes();
  let mut cut = KEY_PREFIX_MAX;
  while cut > 0 && (b[cut - 1] == b'%' || (cut >= 2 && b[cut - 2] == b'%')) {
    cut -= 1;
  }
  enc[..cut].to_string()
}

/// 缓存键茎（无扩展名）：{截断≤80 的 url-safe rel 前缀}-{fnv64hex(rel)}-{mtime_ms}-{size}——
/// 内容缩略图 / exe 图标 / peaks / 失败记忆 / 队列去重共用同一身份口径；前缀只供人读，
/// 唯一性由 hash 段保证
/// pub(crate)：ws_thumb_queue 去重键、失败记忆键与 commands::media peaks 缓存名同源复用
pub(crate) fn job_key(rel: &str, mtime_ms: i64, size: u64) -> String {
  format!("{}-{}-{}-{}", key_prefix(rel), fnv64hex(rel), mtime_ms, size)
}

/// 缓存文件名：{job_key}.{webp|png}
fn cache_name(rel: &str, mtime_ms: i64, size: u64, ext: &str) -> String {
  format!("{}.{ext}", job_key(rel, mtime_ms, size))
}

/// 缓存命中（新鲜）判定：按当前 mtime+size 重算期望文件名，存在即新鲜
/// （webp / png 两种编码产物都认——编码失败退 PNG 的旧产物同样新鲜）
/// pub(crate)：ws_thumb_queue worker 执行前重查（排队期间可能已被另一趟生成）
pub(crate) fn fresh_thumb_path(ws_root: &Path, rel: &str, mtime_ms: i64, size: u64) -> Option<String> {
  for ext in ["webp", "png"] {
    let dest = thumbs_dir(ws_root).join(cache_name(rel, mtime_ms, size, ext));
    if dest.is_file() {
      return Some(dest.to_string_lossy().into_owned());
    }
  }
  None
}

// ---------- 失败记忆（2026-09-26 性能优化 A） ----------

/// 进程级失败键集合：（workspace 根, 缓存键茎 rel+mtime+size）。源文件变化（mtime/size 变）
/// 键自然失效会重试；进程级不落盘——重启后每失败条目至多再试一趟。空间切换无需清理
/// （键带根，跨空间不撞；失败条目量级小不膨胀）
fn failed_keys() -> &'static Mutex<HashSet<(PathBuf, String)>> {
  static FAILED: OnceLock<Mutex<HashSet<(PathBuf, String)>>> = OnceLock::new();
  FAILED.get_or_init(|| Mutex::new(HashSet::new()))
}

/// 失败记忆命中判定：entry() 据此把失败条目视同不可解码落回格式图标；ws_thumb_queue
/// 入队据此跳过（不再重复解码重复失败）
pub(crate) fn is_thumb_failed(root: &Path, rel: &str, mtime_ms: i64, size: u64) -> bool {
  failed_keys().lock().map(|s| s.contains(&(root.to_path_buf(), job_key(rel, mtime_ms, size)))).unwrap_or(false)
}

/// 记入失败记忆（ws_thumb_queue worker 解码 / 编码 / 落盘失败时）
pub(crate) fn note_thumb_failed(root: &Path, rel: &str, mtime_ms: i64, size: u64) {
  if let Ok(mut s) = failed_keys().lock() {
    s.insert((root.to_path_buf(), job_key(rel, mtime_ms, size)));
  }
}

/// 成功路径防御性清除（同键曾失败后又成功 = 瞬时故障自愈，防记忆误伤后续重扫）
pub(crate) fn clear_thumb_failed(root: &Path, rel: &str, mtime_ms: i64, size: u64) {
  if let Ok(mut s) = failed_keys().lock() {
    s.remove(&(root.to_path_buf(), job_key(rel, mtime_ms, size)));
  }
}

// ---------- 缩略图生成 ----------

/// 请求路径 → 生成任务要素（rel, mtime_ms, size）：过滤 workspace 外路径 / 目录 / 元数据
/// 读取失败 / 缓存已新鲜 / 失败记忆命中。pub(crate)：ws_thumb_queue 入队前解析复用同一口径
pub(crate) fn resolve_thumb_job(root: &Path, abs: &Path) -> Option<(String, i64, u64)> {
  if !abs.is_file() || !is_under(root, abs) {
    return None;
  }
  let rel = rel_of(root, abs)?;
  let meta = fs::metadata(abs).ok()?;
  let size = meta.len();
  let mtime_ms = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64).unwrap_or(0);
  if fresh_thumb_path(root, &rel, mtime_ms, size).is_some() || is_thumb_failed(root, &rel, mtime_ms, size) {
    return None;
  }
  Some((rel, mtime_ms, size))
}

/// 单条缩略图生成：解码链复用 thumbnail（图片魔数 / PSD / SVG / 视频首帧 / 音频波形，
/// gif/webp 动图取首帧）；成功落盘 .anikit/thumbs/ 并顺手清理同 relPath 旧缓存（源已变化），
/// 返回（缓存路径, 缩略图实际宽, 高）——尺寸随 thumbs-ready 事件带出供前端回填卡片比例；
/// 解码 / 编码 / 落盘失败返回 None（调用方记失败记忆）
/// pub(crate)：ws_thumb_queue worker 执行体复用
pub(crate) fn generate_thumb(ws_root: &Path, abs: &Path, rel: &str, mtime_ms: i64, size: u64) -> Option<(PathBuf, u32, u32)> {
  let img = thumbnail::load_origin(abs)?;
  let thumb = thumbnail::shrink_to_thumb(img);
  let (tw, th) = (thumb.width(), thumb.height());
  let (bytes, ext) = thumbnail::encode_thumb_bytes(&thumb)?;
  let dir = thumbs_dir(ws_root);
  fs::create_dir_all(&dir).ok()?;
  let dest = dir.join(cache_name(rel, mtime_ms, size, ext));
  fs::write(&dest, bytes).ok()?;
  purge_stale(&dir, rel, &dest);
  Some((dest, tw, th))
}

/// 同 rel 的旧缓存清理：源文件变化后旧名文件永远不再新鲜，留着只膨胀缓存目录。
/// 按 hash 段 `-{fnv64hex(rel)}-` 匹配（2026-09-26 有界命名起——截断前缀不再唯一，
/// hash 段才是身份）；按扩展名的格式图标（{ext}.webp）无此段天然不受影响
/// pub(crate)：commands::media 工作空间波形 peaks 缓存同口径顺清（2026-09-26 填充卡对接）
pub(crate) fn purge_stale(dir: &Path, rel: &str, keep: &Path) {
  let marker = format!("-{}-", fnv64hex(rel));
  let Ok(rd) = fs::read_dir(dir) else { return };
  for e in rd.flatten() {
    let p = e.path();
    if e.file_name().to_string_lossy().contains(&marker) && p != keep {
      let _ = fs::remove_file(&p);
    }
  }
}

/// exe 文件级图标缓存名：内容缩略图键 + `.icon` 中缀（同 job_key 前缀，purge_stale 同源
/// 顺清；中缀防与内容缩略图撞名——exe 不解码，理论上不撞，双保险）
fn icon_cache_name(rel: &str, mtime_ms: i64, size: u64) -> String {
  format!("{}.icon.webp", job_key(rel, mtime_ms, size))
}

/// exe 图标缓存命中（新鲜）判定：按当前 mtime+size 重算期望文件名，存在即新鲜
fn fresh_exe_icon_path(ws_root: &Path, rel: &str, mtime_ms: i64, size: u64) -> Option<PathBuf> {
  let dest = thumbs_dir(ws_root).join(icon_cache_name(rel, mtime_ms, size));
  dest.is_file().then_some(dest)
}

/// exe 文件级图标缓存：已新鲜直接命中；否则现场 shell 提取该文件自身的图标落盘
/// （不按扩展名——同扩展名各程序图标不同），顺手清理同 relPath 旧缓存；提取失败 None
fn ensure_exe_icon(ws_root: &Path, abs: &Path, rel: &str, mtime_ms: i64, size: u64) -> Option<PathBuf> {
  if let Some(p) = fresh_exe_icon_path(ws_root, rel, mtime_ms, size) {
    return Some(p);
  }
  let img = shell_thumb::format_icon(abs, shell_thumb::FORMAT_ICON_EDGE)?;
  let dir = thumbs_dir(ws_root);
  fs::create_dir_all(&dir).ok()?;
  let dest = dir.join(icon_cache_name(rel, mtime_ms, size));
  thumbnail::save_webp(&image::DynamicImage::ImageRgba8(img), &dest).ok()?;
  purge_stale(&dir, rel, &dest);
  Some(dest)
}

/// 解码失败条目的格式图标落回（ws_thumb_queue worker 专用）：与列举时非解码类型的回退
/// 同口径（按扩展名缓存 {ext}.webp），成功返回随 thumbs-ready 事件带给前端的条目——
/// 前端即时由空盒换图标并不再请求；图标提取本身失败（无关联程序等）返回 None，
/// 条目维持空盒，下次列举经 entry() 的失败记忆分支再落回
pub(crate) fn failed_icon_fallback(ws_root: &Path, abs: &Path) -> Option<ThumbReady> {
  let ext = abs.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
  let dir = thumbs_dir(ws_root);
  if !thumbnail::ensure_format_icon_in(&dir, abs, &ext) {
    return None;
  }
  Some(ThumbReady {
    abs_path: abs.to_string_lossy().into_owned(),
    thumb_path: thumbnail::format_icon_path_in(&dir, &ext).to_string_lossy().into_owned(),
    w: shell_thumb::FORMAT_ICON_EDGE,
    h: shell_thumb::FORMAT_ICON_EDGE,
    thumb_kind: ThumbKind::FormatIcon,
  })
}

/// 同步版逐一生成（测试专用——生产路径 = ws_thumb_queue worker 池；语义镜像 worker：
/// 同 resolve_thumb_job 过滤、成功清失败记忆、失败记失败记忆），
/// 返回本趟实际生成成功的清单
#[cfg(test)]
pub fn request_thumbs(root: &Path, abs_paths: Vec<PathBuf>) -> Vec<ThumbReady> {
  let mut out = Vec::new();
  for abs in abs_paths {
    let Some((rel, mtime_ms, size)) = resolve_thumb_job(root, &abs) else { continue };
    match generate_thumb(root, &abs, &rel, mtime_ms, size) {
      Some((dest, w, h)) => {
        clear_thumb_failed(root, &rel, mtime_ms, size);
        out.push(ThumbReady {
          abs_path: abs.to_string_lossy().into_owned(),
          thumb_path: dest.to_string_lossy().into_owned(),
          w,
          h,
          thumb_kind: ThumbKind::Asset,
        });
      }
      None => {
        log::warn!("工作空间缩略图生成失败（记入失败记忆）：{}", abs.display());
        note_thumb_failed(root, &rel, mtime_ms, size);
      }
    }
  }
  out
}

// ---------- 缓存版本迁移（2026-09-26 有界命名切换） ----------

/// 缓存命名版本标志（.anikit/cache_v2）：旧「整条编码路径」命名缓存随命名切换全部失效成
/// 孤儿，以标志文件记一次性清理已完成
const CACHE_VERSION_FLAG: &str = "cache_v2";

/// 开空间一次性缓存迁移（activate_space 唯一调用点，start_watch 之前）：标志不存在则清空
/// thumbs/ 与 peaks/ 后落标志——两者都是纯缓存可删可重建，下次列举按新命名重新生成；
/// .anikit 其余内容（workspace.json / media/ 转码产物，命名本就有界）不动；失败记忆是
/// 进程级无需迁移；清理 / 落标志失败仅记日志不阻断开空间
pub fn migrate_cache(space_root: &Path) {
  let ws = space_root.join(WORKSPACE_DIR);
  let cache = ws.join(CACHE_DIR);
  if !ws.is_dir() || cache.join(CACHE_VERSION_FLAG).is_file() {
    return;
  }
  for sub in ["thumbs", "peaks"] {
    let dir = cache.join(sub);
    if dir.is_dir() {
      if let Err(e) = fs::remove_dir_all(&dir) {
        log::warn!("工作空间旧命名缓存清理失败（{}）：{e}", dir.display());
      }
    }
  }
  // .anikit 可能尚不存在（新空间还没生成过缓存）：建目录只落标志，避免下趟开空间误清
  // 新命名缓存
  if let Err(e) = fs::create_dir_all(&cache).and_then(|_| fs::write(cache.join(CACHE_VERSION_FLAG), "2")) {
    log::warn!("缓存版本标志写入失败（{}）：{e}", cache.display());
  }
}

// ---------- 监听生命周期 ----------

/// 启动工作空间监听（空间打开唯一入口 activate_space 调用；workspace/ 目录缺失则不启）：
/// 任何路径事件（.anikit 缓存写回自激已滤）→ emit workspace-fs-changed，前端去抖重扫。
/// 用**纯转发轻量监听**（不就绪判定，零遍历零探测，stop 秒退——2026-09-19 用户实机
/// 「接入工作空间后托盘退出又卡又慢」：复用收件箱就绪判定版时对活动条目每 500ms 递归
/// 探测全树，退出 join 还要等当趟遍历跑完）。
/// 先停旧监听（切空间防御性清理）；全部失败记日志降级，不影响空间打开
pub fn start_watch(app: &AppHandle, space_root: &Path) {
  stop_watch(app);
  // 切空间清空缩略图队列 pending（旧空间的排队任务无意义；进行中的任务落盘幂等无害——
  // 缓存键内嵌 mtime+size，写进旧空间 .anikit 后重开仍新鲜可用）
  crate::services::ws_thumb_queue::clear_pending();
  let dir = space_root.join(WORKSPACE_DIR);
  if !dir.is_dir() {
    return;
  }
  let app_emit = app.clone();
  let handle = match inbox_watch::spawn_changes_only(
    dir,
    move |paths| {
      // .anikit 缓存写回自激滤掉 + 已知操作回声消重（PRD 步骤 2：命令返回即更新 UI，
      // 窗口期内本进程写操作的 watcher 事件不上报，窗口外 / 混合事件照常推送）
      if paths.iter().any(|p| !in_cache_dir(p)) && !is_echo(paths) {
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

/// 路径是否位于 .anikit 缓存目录内（缩略图 / 转码产物写回会触发监听，滤掉防自激；
/// 写操作同样禁入——缓存目录只归缓存管线管）
fn in_cache_dir(path: &Path) -> bool {
  path.components().any(|c| c.as_os_str() == CACHE_DIR)
}

// ---------- 写操作（PRD 02 步骤 2 基础整理：新建 / 重命名 / 回收站删除 / 移动 / 复制粘贴） ----------

/// 已知操作回声消重窗口（PRD 步骤 2 口径：命令返回即更新 UI，watcher 事件按已知
/// 操作消重，不用延时忽略——避免误吃窗口期外真实外部改动）。本进程写操作完成到
/// watcher 事件到达的量级远小于 3s，窗口外事件照常推送
const ECHO_WINDOW: Duration = Duration::from_secs(3);

/// 已知操作登记簿：（完成时刻, 受影响路径）。进程级单例（服务本身无状态）
fn known_ops() -> &'static Mutex<Vec<(Instant, PathBuf)>> {
  static OPS: OnceLock<Mutex<Vec<(Instant, PathBuf)>>> = OnceLock::new();
  OPS.get_or_init(|| Mutex::new(Vec::new()))
}

fn sweep_known_ops(ops: &mut Vec<(Instant, PathBuf)>) {
  ops.retain(|(t, _)| t.elapsed() < ECHO_WINDOW);
}

/// 写操作成功后登记受影响路径（旧 + 新），watcher 回声据此消重
fn note_known_op(paths: Vec<PathBuf>) {
  if paths.is_empty() {
    return;
  }
  if let Ok(mut ops) = known_ops().lock() {
    sweep_known_ops(&mut ops);
    ops.push((Instant::now(), paths[0].clone()));
    for p in paths.into_iter().skip(1) {
      ops.push((Instant::now(), p));
    }
  }
}

/// 事件路径是否全部为已知操作回声（每条都落在窗口期内某条登记路径之下）
fn is_echo(paths: &[PathBuf]) -> bool {
  is_echo_within(paths, ECHO_WINDOW)
}

/// 窗口抽参版（测试用极小窗口验证过期即不再消重）
fn is_echo_within(paths: &[PathBuf], window: Duration) -> bool {
  if paths.is_empty() {
    return false;
  }
  let Ok(mut ops) = known_ops().lock() else { return false };
  ops.retain(|(t, _)| t.elapsed() < window);
  paths.iter().all(|p| ops.iter().any(|(_, op)| p.starts_with(op)))
}

/// 新建 / 重命名的名字合法性：非空、非 . / ..、不含 Windows 非法字符与控制字符、
/// 不以点或空格结尾（结尾点/空格在资源管理器里会名实不符）
fn validate_name(name: &str) -> Result<(), String> {
  if name.is_empty() {
    return Err("名称不能为空".to_string());
  }
  if matches!(name, "." | "..") {
    return Err("名称不合法".to_string());
  }
  if name.chars().any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control()) {
    return Err(format!("名称含非法字符：{name}"));
  }
  if name.ends_with(['.', ' ']) {
    return Err(format!("名称不能以点或空格结尾：{name}"));
  }
  Ok(())
}

/// 重名自动规避：目标已存在时在词干后追加 _n（n 从 2 递增；2026-09-23 序号格式统一定案，
/// 原 "名 (n)" 括号式废除），不覆盖已有条目
fn unique_target(dir: &Path, name: &str) -> PathBuf {
  let target = dir.join(name);
  if !target.exists() {
    return target;
  }
  let (stem, ext) = match name.rfind('.') {
    Some(i) if i > 0 => (&name[..i], &name[i..]),
    _ => (name, ""),
  };
  for n in 2.. {
    let candidate = dir.join(format!("{stem}_{n}{ext}"));
    if !candidate.exists() {
      return candidate;
    }
  }
  unreachable!()
}

/// 目录递归复制（文件直拷）；任一步失败返回中文错
fn copy_recursive(src: &Path, dst: &Path) -> Result<(), String> {
  if src.is_dir() {
    fs::create_dir(dst).map_err(|e| format!("复制失败（无法创建 {}）：{e}", dst.display()))?;
    for e in fs::read_dir(src).map_err(|e| format!("复制失败（无法读取 {}）：{e}", src.display()))?.flatten() {
      copy_recursive(&e.path(), &dst.join(e.file_name()))?;
    }
    Ok(())
  } else {
    fs::copy(src, dst).map(|_| ()).map_err(|e| format!("复制失败（{}）：{e}", src.display()))
  }
}

/// 写操作源路径公共校验：解析 + .anikit 保护 + 存在性
fn resolve_existing(root: &Path, rel: &str) -> Result<PathBuf, String> {
  let p = resolve_under(root, rel)?;
  if in_cache_dir(&p) {
    return Err(".anikit 缓存目录不允许操作".to_string());
  }
  if !p.exists() {
    return Err(format!("条目已不存在：{rel}"));
  }
  Ok(p)
}

/// 新建文件夹：rel_dir 下创建 name；重名给中文错（新建须显式命名，不自动规避）
pub fn create_folder(root: &Path, rel_dir: &str, name: &str) -> Result<(), String> {
  validate_name(name)?;
  let dir = resolve_under(root, rel_dir)?;
  if !dir.is_dir() {
    return Err("目标目录不存在".to_string());
  }
  let target = dir.join(name);
  if target.exists() {
    return Err(format!("已存在同名条目：{name}"));
  }
  fs::create_dir(&target).map_err(|e| format!("新建文件夹失败：{e}"))?;
  note_known_op(vec![target]);
  Ok(())
}

/// 重命名：rel 最后一段改为 new_name（文件与文件夹同径）；目标重名报错，名字不变不动
pub fn rename_entry(root: &Path, rel: &str, new_name: &str) -> Result<(), String> {
  validate_name(new_name)?;
  let src = resolve_existing(root, rel)?;
  let Some(parent) = src.parent().map(|p| p.to_path_buf()) else {
    return Err("路径不合法".to_string());
  };
  let dest = parent.join(new_name);
  if dest == src {
    return Ok(());
  }
  if dest.exists() {
    return Err(format!("已存在同名条目：{new_name}"));
  }
  fs::rename(&src, &dest).map_err(|e| format!("重命名失败：{e}"))?;
  note_known_op(vec![src, dest]);
  Ok(())
}

/// 回收站删除：多路径一次调用（系统回收站，PRD 06 不进库内回收站）
pub fn delete_entries(root: &Path, rels: &[String]) -> Result<(), String> {
  let mut paths = Vec::new();
  for rel in rels {
    paths.push(resolve_existing(root, rel)?);
  }
  if paths.is_empty() {
    return Ok(());
  }
  crate::adapters::recycle_bin::delete_to_recycle(&paths)?;
  note_known_op(paths);
  Ok(())
}

/// 移动（拖拽 / 剪切粘贴）：src_rels 逐个移入 dest_rel_dir（同卷 rename）；
/// 目录禁移入自身子树；重名自动 _n 规避（n 从 2 递增），不覆盖
pub fn move_entries(root: &Path, src_rels: &[String], dest_rel_dir: &str) -> Result<(), String> {
  let dest = resolve_under(root, dest_rel_dir)?;
  if !dest.is_dir() {
    return Err("目标目录不存在".to_string());
  }
  for rel in src_rels {
    let src = resolve_existing(root, rel)?;
    if src == dest {
      return Err("不能移动到自身".to_string());
    }
    if dest.starts_with(&src) {
      return Err("不能移动到自己的子目录里".to_string());
    }
    let name = src.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or_else(|| "路径不合法".to_string())?;
    let target = unique_target(&dest, &name);
    fs::rename(&src, &target).map_err(|e| format!("移动失败（{name}）：{e}"))?;
    note_known_op(vec![src, target]);
  }
  Ok(())
}

/// 复制粘贴：src_rels 逐个复制进 dest_rel_dir（目录递归，重名自动规避）
pub fn copy_entries(root: &Path, src_rels: &[String], dest_rel_dir: &str) -> Result<(), String> {
  let dest = resolve_under(root, dest_rel_dir)?;
  if !dest.is_dir() {
    return Err("目标目录不存在".to_string());
  }
  for rel in src_rels {
    let src = resolve_existing(root, rel)?;
    if src == dest {
      return Err("不能复制到自身".to_string());
    }
    let name = src.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or_else(|| "路径不合法".to_string())?;
    let target = unique_target(&dest, &name);
    copy_recursive(&src, &target)?;
    note_known_op(vec![target]);
  }
  Ok(())
}

// ---------- 视图状态：常用固定（PRD 02 步骤 3，.anikit/workspace.json） ----------

/// 视图状态文件形状（字段后补；当前仅 pinned = 常用固定 rel 清单）
#[derive(Default, serde::Deserialize, serde::Serialize)]
struct ViewState {
  #[serde(default)]
  pinned: Vec<String>,
}

fn view_state_path(root: &Path) -> PathBuf {
  root.join(CACHE_DIR).join("workspace.json")
}

/// 读视图状态：缺文件 / 解析失败 / 形状不符 = 默认（空）——可删可重建口径
fn read_view_state(root: &Path) -> ViewState {
  fs::read_to_string(view_state_path(root)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn write_view_state(root: &Path, st: &ViewState) -> Result<(), String> {
  let path = view_state_path(root);
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|e| format!("视图状态写入失败：{e}"))?;
  }
  let text = serde_json::to_string_pretty(st).map_err(|e| format!("视图状态序列化失败：{e}"))?;
  fs::write(&path, text).map_err(|e| format!("视图状态写入失败：{e}"))
}

/// 常用固定清单（PRD 步骤 3）：读取时剔除已不存在 / 已改名 / 非目录条目（静默回写），
/// 去重保序；rel 全部经 resolve_under 校验，坏数据不炸
pub fn list_pinned(root: &Path) -> Vec<String> {
  let mut st = read_view_state(root);
  let before = st.pinned.len();
  st.pinned.retain(|rel| match resolve_under(root, rel) {
    Ok(p) => p.is_dir() && !in_cache_dir(&p),
    Err(_) => false,
  });
  let mut seen = std::collections::HashSet::new();
  st.pinned.retain(|rel| seen.insert(rel.clone()));
  if st.pinned.len() != before {
    if let Err(e) = write_view_state(root, &st) {
      log::warn!("常用固定清单清理回写失败：{e}");
    }
  }
  st.pinned
}

/// 固定文件夹到常用：仅目录可固定；重复固定幂等
pub fn pin(root: &Path, rel: &str) -> Result<Vec<String>, String> {
  let p = resolve_existing(root, rel)?;
  if !p.is_dir() {
    return Err("只能固定文件夹".to_string());
  }
  let mut st = read_view_state(root);
  if !st.pinned.iter().any(|r| r == rel) {
    st.pinned.push(rel.to_string());
    write_view_state(root, &st)?;
  }
  Ok(st.pinned)
}

/// 取消固定；返回最新清单（容忍清单里的失效条目——list_pinned 顺带清理）
pub fn unpin(root: &Path, rel: &str) -> Vec<String> {
  let mut st = read_view_state(root);
  st.pinned.retain(|r| r != rel);
  if let Err(e) = write_view_state(root, &st) {
    log::warn!("取消固定回写失败：{e}");
  }
  list_pinned(root)
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

  /// 非图片/音视频类型回退系统格式图标（2026-09-17 实机定）：现场 shell 提取——txt 按扩展名
  /// 缓存 .anikit/thumbs/{ext}.webp；exe 按文件缓存 {rel}-{mtime}-{size}.icon.webp（同扩展名
  /// 各程序图标不同，2026-09-17 用户定）；thumbPath + thumbKind=FormatIcon；可解码类型不回退
  #[cfg(windows)]
  #[test]
  fn list_dir_non_decodable_falls_back_to_format_icon() {
    let (_dir, root) = make_ws();
    fs::write(root.join("说明.txt"), b"hello").unwrap();
    fs::write(root.join("程序.exe"), b"MZ fake").unwrap();
    make_png(&root.join("图.png"), 100, 80);

    let list = list_dir(&root, "").unwrap();
    let txt = list.iter().find(|e| e.name == "说明.txt").unwrap();
    let t = txt.thumb_path.as_ref().expect("txt 应回退格式图标");
    assert!(t.replace('\\', "/").ends_with(".anikit/thumbs/txt.webp"), "图标缓存锚点：{t}");
    assert_eq!(txt.thumb_kind, Some(ThumbKind::FormatIcon));
    assert!(root.join(".anikit/thumbs/txt.webp").is_file());

    let exe = list.iter().find(|e| e.name == "程序.exe").unwrap();
    assert_eq!(exe.thumb_kind, Some(ThumbKind::FormatIcon));
    let exe_icon = exe.thumb_path.as_ref().expect("exe 应回退文件级图标");
    assert!(
      exe_icon.replace('\\', "/").contains(".anikit/thumbs/") && exe_icon.ends_with(".icon.webp") && !exe_icon.ends_with("exe.webp"),
      "exe 图标按文件缓存（非扩展名键）：{exe_icon}"
    );

    let png = list.iter().find(|e| e.name == "图.png").unwrap();
    assert_eq!(png.thumb_path, None, "可解码类型走内容缩略图管线，不回退图标");
    assert_eq!(png.thumb_kind, None);

    // 二次列举：图标已缓存直接命中（不再现场提取）
    let list2 = list_dir(&root, "").unwrap();
    let txt2 = list2.iter().find(|e| e.name == "说明.txt").unwrap();
    assert_eq!(txt2.thumb_kind, Some(ThumbKind::FormatIcon));
  }

  /// exe 文件级图标：两个 exe 各缓存各的（互不相同，根治按扩展名串味），mtime 变化旧缓存被清
  #[cfg(windows)]
  #[test]
  fn exe_icons_cache_per_file() {
    let (_dir, root) = make_ws();
    fs::write(root.join("甲.exe"), b"MZ fake one").unwrap();
    fs::write(root.join("乙.exe"), b"MZ fake two").unwrap();

    let list = list_dir(&root, "").unwrap();
    let a = list.iter().find(|e| e.name == "甲.exe").unwrap().thumb_path.clone().expect("甲 exe 图标");
    let b = list.iter().find(|e| e.name == "乙.exe").unwrap().thumb_path.clone().expect("乙 exe 图标");
    assert_ne!(a, b, "两个 exe 的图标缓存必须按文件区分");
    assert!(Path::new(&a).is_file() && Path::new(&b).is_file());

    // 甲内容变化（mtime+size 变）→ 新名新缓存，旧缓存被 purge_stale 清掉
    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(root.join("甲.exe"), b"MZ fake one renewed and larger").unwrap();
    let list2 = list_dir(&root, "").unwrap();
    let a2 = list2.iter().find(|e| e.name == "甲.exe").unwrap().thumb_path.clone().expect("甲 exe 新图标");
    assert_ne!(a2, a, "源变化后图标缓存键跟着变");
    assert!(!Path::new(&a).exists(), "旧 exe 图标缓存已清理");
    assert!(Path::new(&b).exists(), "乙的缓存不受影响");
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

  /// has_sub_dir（2026-09-26 树展开箭头真实化）：有直接子目录 true（不管子级是否为空——
  /// 箭头语义 = 有子目录行可展）；空目录 / 仅文件 / 仅 .anikit 子目录 false（.anikit 不计，
  /// 与 list_dir 隐藏口径一致）；文件条目恒 false；判定只看直接子级不递归（每层各自扫描）
  #[test]
  fn list_dir_fills_has_sub_dir() {
    let (_dir, root) = make_ws();
    fs::create_dir_all(root.join("有子/子")).unwrap();
    fs::create_dir_all(root.join("空夹")).unwrap();
    fs::create_dir_all(root.join("仅文件")).unwrap();
    fs::write(root.join("仅文件/图.txt"), b"x").unwrap();
    fs::create_dir_all(root.join("仅缓存/.anikit")).unwrap();
    fs::create_dir_all(root.join("祖辈/父辈/孙辈")).unwrap();
    fs::write(root.join("文件.txt"), b"x").unwrap();

    let list = list_dir(&root, "").unwrap();
    let get = |n: &str| list.iter().find(|e| e.name == n).unwrap();
    assert!(get("有子").has_sub_dir, "有直接子目录即 true（子级为空也算——箭头 = 有子目录行）");
    assert!(!get("空夹").has_sub_dir, "空目录无箭头");
    assert!(!get("仅文件").has_sub_dir, "只有文件不算");
    assert!(!get("仅缓存").has_sub_dir, ".anikit 子目录不计");
    assert!(get("祖辈").has_sub_dir);
    assert!(!get("文件.txt").has_sub_dir, "文件条目恒 false");

    // 每层按各自直接子级判定（不递归）：父辈见孙辈 true，孙辈自身 false
    let mid = list_dir(&root, "祖辈").unwrap();
    assert!(mid.iter().find(|e| e.name == "父辈").unwrap().has_sub_dir);
    let deep = list_dir(&root, "祖辈/父辈").unwrap();
    assert!(!deep.iter().find(|e| e.name == "孙辈").unwrap().has_sub_dir);
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
  }

  /// FNV-1a 64 已知向量（空串 / "a" 为公布标准向量）+ 定长 16 小写 hex
  #[test]
  fn fnv64hex_known_vectors() {
    assert_eq!(fnv64hex(""), "cbf29ce484222325");
    assert_eq!(fnv64hex("a"), "af63dc4c8601ec8c");
    assert_eq!(fnv64hex("图.png").len(), 16);
    assert!(fnv64hex("图.png").chars().all(|c| c.is_ascii_hexdigit()));
  }

  /// 缓存键长度有界（踩坑 092：旧「整条编码路径」做文件名，中文每字符 9 倍膨胀超 NTFS
  /// 255 组件上限写盘必败）：实锤案例 rel 旧命名 257 字符；新命名 ≤160，截断只落在完整
  /// %XX 转义边界（不切半），短 rel 不截断无 hash 前缀损失
  #[test]
  fn cache_name_bounded_for_deep_cjk_rel() {
    let rel = "02_创作/01_原创动画/01_计划中/224/01_灵感/05_音乐/不要去看天气预报.wav";
    let old = format!("{}-1720000000000-52819034.webp", encode_rel(rel));
    assert!(old.len() > 255, "旧命名复现实锤场景（{} 字符）", old.len());

    let name = cache_name(rel, 1720000000000, 52819034, "webp");
    assert!(name.len() <= 160, "新命名有界（{} 字符）：{name}", name.len());
    assert!(name.ends_with("-1720000000000-52819034.webp"), "mtime+size 仍内嵌文件名");

    let prefix = key_prefix(rel);
    assert!(prefix.len() <= KEY_PREFIX_MAX && encode_rel(rel).starts_with(&prefix));
    let b = prefix.as_bytes();
    assert_ne!(b[b.len() - 1], b'%', "不切在 % 后");
    assert_ne!(b[b.len() - 2], b'%', "不切在 %X 后");

    // 短 rel 不截断；不同 rel 键不同（hash 段兜底唯一性）
    assert_eq!(key_prefix("a.png"), "a.png");
    assert_eq!(cache_name("a.png", 1720000000000, 12345, "webp"), format!("a.png-{}-1720000000000-12345.webp", fnv64hex("a.png")));
    assert_ne!(job_key(rel, 1, 1), job_key("02_创作/01_原创动画/01_计划中/224/01_灵感/05_音乐/别的一首.wav", 1, 1));
  }

  /// 深中文 rel 生成-命中-purge 全链（踩坑 092 实锤场景防回归）：旧命名下 fs::write 必败
  /// 的 rel，现在生成落盘成功、list 命中；源变化重新生成后旧缓存被 purge_stale 顺清
  #[test]
  fn thumb_chain_works_for_deep_cjk_rel() {
    let (_dir, root) = make_ws();
    let rel_dir = "02_创作/01_原创动画/01_计划中/224/01_灵感/05_音乐";
    fs::create_dir_all(root.join(rel_dir)).unwrap();
    let f = root.join(format!("{rel_dir}/封面.png"));
    make_png(&f, 800, 600);

    let ready = request_thumbs(&root, vec![f.clone()]);
    assert_eq!(ready.len(), 1, "深中文路径缩略图应生成成功（旧命名此处写盘必败）");
    let file_name = Path::new(&ready[0].thumb_path).file_name().unwrap().to_string_lossy().into_owned();
    assert!(file_name.len() <= 160, "落盘缓存文件名有界：{file_name}");
    let hit = list_dir(&root, rel_dir).unwrap();
    assert_eq!(hit[0].thumb_path.as_deref(), Some(ready[0].thumb_path.as_str()), "list 命中新鲜缓存");

    std::thread::sleep(std::time::Duration::from_millis(20));
    make_png(&f, 500, 500);
    let again = request_thumbs(&root, vec![f]);
    assert_eq!(again.len(), 1, "源变化后重新生成");
    assert!(!Path::new(&ready[0].thumb_path).exists(), "旧缓存已顺清");
    assert!(Path::new(&again[0].thumb_path).is_file());
  }

  /// purge_stale 按 hash 段匹配：同截断前缀（80 字符后分岔）的两个 rel 互不误清；
  /// 同 rel 旧键（mtime/size 变）被清；按扩展名的格式图标无 hash 段不受影响
  #[test]
  fn purge_stale_matches_hash_segment_no_cross_purge() {
    let (_dir, root) = make_ws();
    let dir = thumbs_dir(&root);
    fs::create_dir_all(&dir).unwrap();
    let base = "02_创作/01_原创动画/01_计划中/224/01_灵感/05_音乐/07_音效库/09_采样";
    let rel_a = format!("{base}/甲.wav");
    let rel_b = format!("{base}/乙.wav");
    assert_eq!(key_prefix(&rel_a), key_prefix(&rel_b), "构造须同截断前缀");
    assert_ne!(fnv64hex(&rel_a), fnv64hex(&rel_b), "hash 段区分不同 rel");

    let keep_a = dir.join(cache_name(&rel_a, 2, 20, "webp"));
    let stale_a = dir.join(cache_name(&rel_a, 1, 10, "webp"));
    let other_b = dir.join(cache_name(&rel_b, 1, 10, "webp"));
    for p in [&keep_a, &stale_a, &other_b] {
      fs::write(p, b"x").unwrap();
    }
    fs::write(dir.join("wav.webp"), b"x").unwrap();

    purge_stale(&dir, &rel_a, &keep_a);
    assert!(keep_a.is_file() && other_b.is_file(), "keep 与他 rel 缓存不动");
    assert!(!stale_a.exists(), "同 rel 旧缓存被清");
    assert!(dir.join("wav.webp").is_file(), "格式图标不受影响");
  }

  /// 缓存版本迁移（2026-09-26 有界命名切换）：无标志 → thumbs/ 与 peaks/ 清空并落标志；
  /// .anikit 其余内容（workspace.json / media/）不动；有标志幂等不再清；workspace/ 缺失不动
  #[test]
  fn migrate_cache_clears_thumbs_and_peaks_once() {
    let dir = tempfile::tempdir().unwrap();
    let space = dir.path();
    let cache = space.join(WORKSPACE_DIR).join(CACHE_DIR);
    fs::create_dir_all(cache.join("thumbs")).unwrap();
    fs::create_dir_all(cache.join("peaks")).unwrap();
    fs::create_dir_all(cache.join("media")).unwrap();
    fs::write(cache.join("thumbs/旧命名缓存.webp"), b"x").unwrap();
    fs::write(cache.join("peaks/旧命名.json"), b"x").unwrap();
    fs::write(cache.join("media/media-1-2.mp4"), b"x").unwrap();
    fs::write(cache.join("workspace.json"), "{}").unwrap();

    migrate_cache(space);
    assert!(!cache.join("thumbs").exists() && !cache.join("peaks").exists(), "旧命名缓存目录清空");
    assert!(cache.join(CACHE_VERSION_FLAG).is_file(), "标志落盘");
    assert!(cache.join("media/media-1-2.mp4").is_file() && cache.join("workspace.json").is_file(), "其余内容不动");

    // 幂等：有标志后新命名缓存不再被清
    fs::create_dir_all(cache.join("thumbs")).unwrap();
    fs::write(cache.join("thumbs/新命名.webp"), b"x").unwrap();
    migrate_cache(space);
    assert!(cache.join("thumbs/新命名.webp").is_file());

    // workspace/ 缺失的空间：不建目录不动任何东西
    let bare = tempfile::tempdir().unwrap();
    migrate_cache(bare.path());
    assert!(!bare.path().join(WORKSPACE_DIR).exists());
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
    // 产物是合法 webp、短边 320；事件负载带缩略图实际尺寸（前端回填卡片比例用）
    assert_eq!(image::image_dimensions(&ready[0].thumb_path).unwrap(), (426, 320));
    assert_eq!((ready[0].w, ready[0].h), (426, 320));
    let hit = list_dir(&root, "子").unwrap();
    assert_eq!(hit[0].thumb_path.as_deref(), Some(ready[0].thumb_path.as_str()), "生成后 list 命中新鲜缓存");
    assert_eq!((hit[0].w, hit[0].h), (Some(800), Some(600)), "图片条目维持源魔数探测尺寸（不读缓存缩略图）");

    // 源变化（重写成不同尺寸的有效图、大小变）→ 失鲜；thumb_path 文件名嵌 rel+mtime+size
    make_png(&f, 500, 500);
    let stale = request_thumbs(&root, vec![f.clone()]);
    assert_eq!(stale.len(), 1, "过期后应重新生成");
    assert_ne!(stale[0].thumb_path, ready[0].thumb_path, "新缓存文件名应随 mtime/size 变化");
    assert_eq!(image::image_dimensions(&stale[0].thumb_path).unwrap(), (320, 320), "新缓存应对应新源尺寸");
    assert_eq!((stale[0].w, stale[0].h), (320, 320));
    let dir = thumbs_dir(&root);
    let leftovers: Vec<_> = fs::read_dir(&dir).unwrap().flatten().collect();
    assert_eq!(leftovers.len(), 1, "旧缓存应被顺手清理");

    // 二次请求全新鲜 → 空结果（命中跳过）
    assert!(request_thumbs(&root, vec![f]).is_empty(), "全新鲜时应无事可做");
  }

  /// 非图片条目（视频等魔数探测不出尺寸）：缓存缩略图新鲜时仅读其文件头回填 w/h——
  /// 卡片由 300 方盒恢复真实比例（2026-09-26 ws 视频卡 cover 裁切修复）；未生成时维持 null 回退
  #[test]
  fn list_dir_backfills_dims_from_cached_thumb() {
    let (_dir, root) = make_ws();
    let vid = root.join("片段.mp4");
    fs::write(&vid, b"fake mp4 undecodable").unwrap();

    // 无缓存：w/h = null、thumbPath = null（可解码类型等后台生成，不回退格式图标）
    let e0 = list_dir(&root, "").unwrap().into_iter().find(|e| e.name == "片段.mp4").unwrap();
    assert_eq!((e0.w, e0.h), (None, None));
    assert_eq!((e0.thumb_path, e0.thumb_kind), (None, None));

    // 模拟后台生成完成：把一张 640×360 png 落到期望缓存名（png 编码产物同样判新鲜）
    let meta = fs::metadata(&vid).unwrap();
    let mtime_ms = meta.modified().unwrap().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
    let dest = thumbs_dir(&root).join(cache_name("片段.mp4", mtime_ms, meta.len(), "png"));
    fs::create_dir_all(dest.parent().unwrap()).unwrap();
    make_png(&dest, 640, 360);

    let e1 = list_dir(&root, "").unwrap().into_iter().find(|e| e.name == "片段.mp4").unwrap();
    assert_eq!(e1.thumb_path.as_deref(), Some(dest.to_string_lossy().as_ref()));
    assert_eq!(e1.thumb_kind, Some(ThumbKind::Asset));
    assert_eq!((e1.w, e1.h), (Some(640), Some(360)), "缓存缩略图尺寸应回填 w/h（与源同比例）");
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

    let ready = request_thumbs(&root, vec![inside, outside, sub, fla.clone()]);
    assert_eq!(ready.len(), 1, "仅 workspace 内可解码的条目生成缩略图");
    assert!(ready[0].abs_path.ends_with("内.png"));
    // fla 不可解码，list_dir 会回退 FormatIcon 图标；其余条目（外.png/子目录）无 thumb_path
    assert!(list_dir(&root, "").unwrap().iter().all(|e| {
      e.thumb_path.is_none() || e.name == "内.png" || e.thumb_kind == Some(ThumbKind::FormatIcon)
    }));
    // 失败记忆（2026-09-26）：fla 已记入失败键，重复请求直接跳过不再重试
    assert!(request_thumbs(&root, vec![fla]).is_empty(), "失败记忆命中不再重试");
  }

  /// 可探测但解码必败的 png：逐块找到 IDAT 原地翻转数据字节——文件头（IHDR）完好
  ///（probe / into_dimensions 只读头部得尺寸），数据段 CRC 失配 decode 必败
  fn make_corrupt_png(path: &Path) {
    make_png(path, 64, 48);
    let mut bytes = fs::read(path).unwrap();
    let mut pos = 8; // 跳过 8 字节 PNG 签名
    while pos + 8 <= bytes.len() {
      let len = u32::from_be_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
      if &bytes[pos + 4..pos + 8] == b"IDAT" && len > 0 {
        bytes[pos + 8] ^= 0xFF;
        break;
      }
      pos += 12 + len; // len(4) + type(4) + data + crc(4)
    }
    fs::write(path, &bytes).unwrap();
  }

  /// 失败记忆（2026-09-26 性能优化 A）：解码失败记入后，entry 把失败条目视同不可解码落回
  /// 系统格式图标（thumbKind=FormatIcon、thumbPath=按扩展名缓存图标），不再以空盒待生成；
  /// 源文件变化（mtime/size 变）键自然失效——重新走内容缩略图管线待生成
  #[cfg(windows)]
  #[test]
  fn failed_entry_falls_back_to_format_icon() {
    let (_dir, root) = make_ws();
    let f = root.join("坏.png");
    make_corrupt_png(&f);
    assert!(crate::services::thumbnail::probe(&f).is_some(), "IDAT 翻转后文件头探测应仍有效");

    // 未失败前：可解码待生成 = (null, null)
    let e0 = list_dir(&root, "").unwrap().into_iter().find(|e| e.name == "坏.png").unwrap();
    assert_eq!((e0.thumb_path, e0.thumb_kind), (None, None));

    let meta = fs::metadata(&f).unwrap();
    let mtime_ms = meta.modified().unwrap().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
    note_thumb_failed(&root, "坏.png", mtime_ms, meta.len());
    let e1 = list_dir(&root, "").unwrap().into_iter().find(|e| e.name == "坏.png").unwrap();
    assert_eq!(e1.thumb_kind, Some(ThumbKind::FormatIcon), "失败条目应落回格式图标");
    let t = e1.thumb_path.as_deref().expect("失败条目应有格式图标路径");
    assert!(t.replace('\\', "/").ends_with(".anikit/thumbs/png.webp"), "按扩展名缓存图标：{t}");

    // 源变化（重写为真图，mtime/size 变）→ 旧失败键自然失效，回到待生成
    std::thread::sleep(std::time::Duration::from_millis(20));
    make_png(&f, 32, 32);
    let e2 = list_dir(&root, "").unwrap().into_iter().find(|e| e.name == "坏.png").unwrap();
    assert_eq!((e2.thumb_path, e2.thumb_kind), (None, None), "源变化后旧失败键失效，重新待生成");
  }

  // ---------- 步骤 2 写操作 ----------

  /// 新建：成功创建；重名 / 非法名（空、..、分隔符、结尾点）给中文错
  #[test]
  fn create_folder_validates_and_rejects_duplicates() {
    let (_dir, root) = make_ws();
    create_folder(&root, "", "新夹").unwrap();
    assert!(root.join("新夹").is_dir());
    assert!(create_folder(&root, "", "新夹").is_err(), "重名应拒绝");
    assert!(create_folder(&root, "", "新夹/嵌套").is_err(), "分隔符应拒绝");
    assert!(create_folder(&root, "", "..").is_err());
    assert!(create_folder(&root, "", "").is_err());
    assert!(create_folder(&root, "", "尾点.").is_err(), "结尾点应拒绝");
    assert!(create_folder(&root, "不存在的目录", "x").is_err());
    assert!(create_folder(&root, ".anikit", "x").is_err(), "缓存目录内新建应失败（目录不存在即败，不特判）");
  }

  /// 重命名：文件与文件夹同径生效；目标重名报错；名字不变不动；.anikit 拒绝
  #[test]
  fn rename_entry_moves_file_and_dir() {
    let (_dir, root) = make_ws();
    make_png(&root.join("旧.png"), 10, 10);
    fs::create_dir_all(root.join("旧夹")).unwrap();
    fs::create_dir_all(root.join(".anikit/thumbs")).unwrap();

    rename_entry(&root, "旧.png", "新.png").unwrap();
    assert!(root.join("新.png").is_file());
    assert!(!root.join("旧.png").exists());
    rename_entry(&root, "旧夹", "新夹").unwrap();
    assert!(root.join("新夹").is_dir());
    rename_entry(&root, "新.png", "新.png").unwrap(); // 名字不变不动

    make_png(&root.join("占位.png"), 10, 10);
    assert!(rename_entry(&root, "新.png", "占位.png").is_err(), "重名应拒绝");
    assert!(rename_entry(&root, "新.png", "a/b").is_err(), "分隔符应拒绝");
    assert!(rename_entry(&root, ".anikit/thumbs", "x").is_err(), ".anikit 拒绝");
  }

  /// 移动：基本移入 + 重名自动 _n 规避（n 从 2 递增）；目录禁移入自身子树
  #[test]
  fn move_entries_relocates_and_avoids_clobber() {
    let (_dir, root) = make_ws();
    fs::create_dir_all(root.join("目标")).unwrap();
    fs::create_dir_all(root.join("夹/子")).unwrap();
    fs::write(root.join("夹/子/内.txt"), b"x").unwrap();
    make_png(&root.join("甲.png"), 10, 10);
    make_png(&root.join("目标/甲.png"), 10, 10);

    move_entries(&root, &["甲.png".to_string()], "目标").unwrap();
    assert!(root.join("目标/甲_2.png").is_file(), "重名应自动加后缀，不覆盖");
    assert!(move_entries(&root, &["夹".to_string()], "夹/子").is_err(), "禁移入自身子树");
    move_entries(&root, &["夹".to_string()], "目标").unwrap();
    assert!(root.join("目标/夹/子/内.txt").is_file());
    assert!(move_entries(&root, &["目标/夹".to_string()], "目标/夹").is_err(), "移动到自身应拒绝");
  }

  /// 复制粘贴：文件直拷、目录递归；重名自动规避；.anikit 拒绝
  #[test]
  fn copy_entries_duplicates_recursively() {
    let (_dir, root) = make_ws();
    fs::create_dir_all(root.join("目标")).unwrap();
    fs::create_dir_all(root.join("源夹/子")).unwrap();
    fs::write(root.join("源夹/子/内.txt"), b"x").unwrap();
    fs::write(root.join("目标/源夹"), "占位同名文件").unwrap();

    copy_entries(&root, &["源夹".to_string()], "目标").unwrap();
    assert!(root.join("目标/源夹_2/子/内.txt").is_file(), "重名目录应自动加后缀");
    assert!(root.join("目标/源夹").is_file(), "原同名文件不被覆盖");
    assert!(copy_entries(&root, &[".anikit".to_string()], "目标").is_err(), ".anikit 拒绝");
  }

  /// 回声消重：窗口期内本进程操作的事件判回声；窗口外 / 无关路径不判
  #[test]
  fn known_op_echo_dedup_windowed() {
    let (_dir, root) = make_ws();
    let touched = root.join("甲.txt");
    fs::write(&touched, b"x").unwrap();

    // 登记前：同一事件不判回声
    assert!(!is_echo_within(std::slice::from_ref(&touched), Duration::from_secs(3)));
    note_known_op(vec![touched.clone()]);
    // 窗口期内：判回声
    assert!(is_echo_within(std::slice::from_ref(&touched), Duration::from_secs(3)));
    // 混合事件（含未登记路径）不判——真实外部改动不被误吃
    assert!(!is_echo_within(&[touched, root.join("外部.txt")], Duration::from_secs(3)));
    // 极小窗口（等价过期）：不再判回声
    std::thread::sleep(Duration::from_millis(10));
    assert!(!is_echo_within(&[root.join("甲.txt")], Duration::from_millis(1)));
  }

  /// 常用固定：固定 / 取消 / 幂等；读取时静默剔除失效（删除 / 改名）与非目录条目
  #[test]
  fn pinned_roundtrip_and_stale_pruned() {
    let (_dir, root) = make_ws();
    fs::create_dir_all(root.join("夹A/子")).unwrap();
    fs::create_dir_all(root.join("夹B")).unwrap();
    fs::write(root.join("文件.txt"), b"x").unwrap();
    fs::create_dir_all(root.join(".anikit/thumbs")).unwrap();

    assert!(pin(&root, "夹A").unwrap() == ["夹A"]);
    assert!(pin(&root, "夹B").unwrap() == ["夹A", "夹B"]);
    assert!(pin(&root, "夹A").unwrap() == ["夹A", "夹B"], "重复固定幂等");
    assert!(pin(&root, "文件.txt").is_err(), "文件不可固定");
    assert!(pin(&root, ".anikit/thumbs").is_err(), ".anikit 拒绝");
    assert!(pin(&root, "../外").is_err(), "越界拒绝");

    // 改名 → 旧 pin 失效剔除；再次读取也是空（已回写落盘）
    rename_entry(&root, "夹B", "夹B新").unwrap();
    rename_entry(&root, "夹A", "夹A新").unwrap();
    assert_eq!(list_pinned(&root), Vec::<String>::new(), "失效条目静默剔除并回写");
    assert_eq!(list_pinned(&root), Vec::<String>::new());

    // 取消固定
    pin(&root, "夹A新").unwrap();
    pin(&root, "夹B新").unwrap();
    assert_eq!(unpin(&root, "夹A新"), vec!["夹B新".to_string()], "取消后返回剩余清单");
    assert_eq!(unpin(&root, "夹B新"), Vec::<String>::new());
  }
}
