//! 索引式工作空间服务（PRD 02）：真实目录列举 + .anikit 缩略图缓存 + 磁盘监听热刷新
//! + 步骤 2 基础整理写操作（新建文件夹 / 重命名 / 回收站删除 / 移动 / 复制粘贴，
//! 纯磁盘语义，越权校验与 .anikit 保护同读路径）。
//! 缩略图缓存键 = 相对路径 + mtime + 大小（无资产 ID 可挂，PRD 技术口径），缓存放
//! workspace/.anikit/thumbs/，可整体删除重建；新鲜度 = 文件名内嵌的 mtime+size 与当前文件
//! 一致（无 sidecar 索引）；.anikit 自身变更不触发刷新（防缓存写回自激）。

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::adapters::inbox_watch;
use crate::commands::library::AppState;
use crate::core::space::WORKSPACE_DIR;
use crate::services::thumbnail;
use crate::services::views::ThumbKind;

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

/// 单条目：文件补 大小 / mtime / 扩展名 / 缩略图（内容缩略图 → 格式图标回退）；目录恒无缩略图
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
  // 图片探测宽高（魔数不认扩展名，壁纸站扩展名撒谎场景同 thumbnail::probe 口径）；
  // 只读文件头，探测失败静默 null——布局回退方盒（useGridLayout 既定链）
  let (w, h) = if is_dir { (None, None) } else { crate::services::thumbnail::probe(path).map(|(w, h)| (Some(w), Some(h))).unwrap_or((None, None)) };
  // 缩略图来源（2026-09-17 实机定：非图片/音视频类型必须有系统格式图标，不许空盒）：
  // ① 内容缩略图缓存新鲜 = asset；② 缩略管线解不出的类型（w/h 探测失败且非音视频）=
  //   现场 shell 提取系统格式图标，按扩展名缓存 .anikit/thumbs/{ext}.webp（库 cache/thumbs 同口径换锚）；
  // ③ 可解码但尚未生成 = (null, null)，后台生成后 thumbs-ready 回填
  let (thumb_path, thumb_kind) = if is_dir {
    (None, None)
  } else if let Some(t) = fresh_thumb_path(root, &rel, mtime_ms, size) {
    (Some(t), Some(ThumbKind::Asset))
  } else {
    let decodable = w.is_some()
      || crate::services::media::VIDEO_EXTS.contains(&ext.as_str())
      || crate::services::media::AUDIO_EXTS.contains(&ext.as_str());
    if !decodable {
      let dir = thumbs_dir(root);
      if crate::services::thumbnail::ensure_format_icon_in(&dir, path, &ext) {
        (Some(crate::services::thumbnail::format_icon_path_in(&dir, &ext).to_string_lossy().into_owned()), Some(ThumbKind::FormatIcon))
      } else {
        (None, None)
      }
    } else {
      (None, None)
    }
  };
  Some(WsEntry { name: name.to_string(), rel_path: rel, is_dir, size, mtime_ms, w, h, ext, thumb_path, thumb_kind })
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

/// 重名自动规避：目标已存在时在词干后追加 " (n)"（n 从 2 递增），不覆盖已有条目
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
    let candidate = dir.join(format!("{stem} ({n}){ext}"));
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
/// 目录禁移入自身子树；重名自动 " (n)" 规避，不覆盖
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

  /// 非图片/音视频类型回退系统格式图标（2026-09-17 实机定）：现场 shell 提取按扩展名缓存
  /// .anikit/thumbs/{ext}.webp，thumbPath + thumbKind=FormatIcon；可解码类型不回退（thumbPath=null）
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

    let png = list.iter().find(|e| e.name == "图.png").unwrap();
    assert_eq!(png.thumb_path, None, "可解码类型走内容缩略图管线，不回退图标");
    assert_eq!(png.thumb_kind, None);

    // 二次列举：图标已缓存直接命中（不再现场提取）
    let list2 = list_dir(&root, "").unwrap();
    let txt2 = list2.iter().find(|e| e.name == "说明.txt").unwrap();
    assert_eq!(txt2.thumb_kind, Some(ThumbKind::FormatIcon));
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
    // fla 不可解码，list_dir 会回退 FormatIcon 图标；其余条目（外.png/子目录）无 thumb_path
    assert!(list_dir(&root, "").unwrap().iter().all(|e| {
      e.thumb_path.is_none() || e.name == "内.png" || e.thumb_kind == Some(ThumbKind::FormatIcon)
    }));
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

  /// 移动：基本移入 + 重名自动 " (n)" 规避；目录禁移入自身子树
  #[test]
  fn move_entries_relocates_and_avoids_clobber() {
    let (_dir, root) = make_ws();
    fs::create_dir_all(root.join("目标")).unwrap();
    fs::create_dir_all(root.join("夹/子")).unwrap();
    fs::write(root.join("夹/子/内.txt"), b"x").unwrap();
    make_png(&root.join("甲.png"), 10, 10);
    make_png(&root.join("目标/甲.png"), 10, 10);

    move_entries(&root, &["甲.png".to_string()], "目标").unwrap();
    assert!(root.join("目标/甲 (2).png").is_file(), "重名应自动加后缀，不覆盖");
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
    assert!(root.join("目标/源夹 (2)/子/内.txt").is_file(), "重名目录应自动加后缀");
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
    assert!(!is_echo_within(&[touched.clone()], Duration::from_secs(3)));
    note_known_op(vec![touched.clone()]);
    // 窗口期内：判回声
    assert!(is_echo_within(&[touched.clone()], Duration::from_secs(3)));
    // 混合事件（含未登记路径）不判——真实外部改动不被误吃
    assert!(!is_echo_within(&[touched, root.join("外部.txt")], Duration::from_secs(3)));
    // 极小窗口（等价过期）：不再判回声
    std::thread::sleep(Duration::from_millis(10));
    assert!(!is_echo_within(&[root.join("甲.txt")], Duration::from_millis(1)));
  }
}
