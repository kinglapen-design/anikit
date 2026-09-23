//! 命令 API：导入（任务化）与导入弹窗所需的树 / 列表查询。
//! 导入命令只做参数解析与短锁校验，随即 spawn 工作线程跑 services::import_task::run_import，
//! 立即返回 task_id；进度 / 结果经 on_event Channel 推送，cancel_import 可取消。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use crate::commands::library::AppState;
use crate::core::library::{Library, CACHE_DIR};
use crate::core::models::Placement;
use crate::services::import::{ImportEvent, ImportOptions, SequenceMode, Source, Structure, ROOT_FOLDER_ID};
use crate::services::import_task::run_import;
use crate::services::views::{placement_view, PlacementView};

/// 导入任务 ID 自增源
pub(crate) static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

// ---------- 导入前预估 ----------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifyResult {
  /// "files"（纯文件）/ "folder"（仅一个文件夹）/ "mixed"（多选含文件夹）
  pub kind: String,
  /// 展开后的文件总数（弹窗预估用）
  pub file_count: u32,
}

/// 分类所选路径并预估展开文件总数（导入弹窗第一步）
#[tauri::command]
pub fn classify_paths(paths: Vec<String>) -> ClassifyResult {
  let mut dir_count = 0usize;
  let mut file_count = 0u32;
  for p in &paths {
    let path = PathBuf::from(p);
    if path.is_dir() {
      dir_count += 1;
      file_count = file_count.saturating_add(count_files(&path));
    } else if path.is_file() {
      file_count = file_count.saturating_add(1);
    }
  }
  let kind = if dir_count == 0 {
    "files"
  } else if paths.len() == 1 {
    "folder"
  } else {
    "mixed"
  };
  ClassifyResult { kind: kind.to_string(), file_count }
}

/// 递归统计文件夹内文件数（仅弹窗预估，读不了的目录按 0 计）
fn count_files(dir: &Path) -> u32 {
  let mut count = 0u32;
  if let Ok(entries) = std::fs::read_dir(dir) {
    for entry in entries.filter_map(|e| e.ok()) {
      let path = entry.path();
      if path.is_dir() {
        count = count.saturating_add(count_files(&path));
      } else if path.is_file() {
        count = count.saturating_add(1);
      }
    }
  }
  count
}

// ---------- 序列帧检测（4.1，09 导入弹窗确认后调用） ----------

/// 检测到的序列帧组摘要（前端"监测到序列帧"弹窗依据；打包入库由 import_paths 重新检测执行）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceGroupInfo {
  /// 基础名（资产标题）
  pub name: String,
  pub ext: String,
  pub frame_count: u32,
}

/// 检测所选路径中的序列帧（后端为唯一事实来源：同目录兄弟文件按 基础名+扩展名+位宽 分组，
/// 序号连续允许少量缺帧且 ≥3，仅图片格式）；检测失败的路径静默跳过（导入阶段会进 failed）
#[tauri::command]
pub fn detect_sequences(paths: Vec<String>) -> Vec<SequenceGroupInfo> {
  let mut ignored = Vec::new();
  let raw: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
  let files = crate::services::import::collect(&raw, Structure::Flatten, &mut ignored);
  crate::services::sequence::detect_groups(&files)
    .into_iter()
    .map(|g| SequenceGroupInfo { name: g.name, ext: g.ext, frame_count: g.frames.len() as u32 })
    .collect()
}

// ---------- 导入（任务化） ----------

/// 导入文件 / 文件夹：立即返回 task_id，工作线程执行，事件经 on_event 推送；
/// sequence_mode "pack" 时检测到的序列帧整组打包为一个资源（4.1）
#[tauri::command]
pub fn import_paths(
  app: AppHandle,
  paths: Vec<String>,
  structure: String,
  source: String,
  target_folder: Option<String>,
  sequence_mode: String,
  on_event: Channel<ImportEvent>,
) -> Result<u64, String> {
  let structure = parse_structure(&structure)?;
  let source = parse_source(&source)?;
  let sequence_mode = parse_sequence_mode(&sequence_mode)?;
  let (root, assets_dir) = current_paths(&app)?;
  ensure_target(&app, &target_folder)?;
  let opts = ImportOptions {
    structure,
    source,
    target_folder,
    paths: paths.iter().map(PathBuf::from).collect(),
    imported_from: None,
    sequence_mode,
  };
  Ok(spawn_import(app, root, assets_dir, opts, on_event, None))
}

/// 剪贴板导入：先读图片（截图场景 → PNG 落 cache/tmp 临时文件），无图再读文件列表
/// （资源管理器 Ctrl+C 的文件，CF_HDROP），再读 base64 文本（2026-08-14 用户定：
/// data URI / 裸 base64 解码按魔数定扩展名落临时文件）；三者皆无报错。
/// 来源恒 Copy（2026-07-27 定：剪贴板复制的文件绝不能 Move 走用户原文件）；
/// 临时文件落 cache/tmp（会话级，开库整目录清空兜底）：**无冲突挂起才由工作线程即删，
/// 有冲突挂起必须保留**——resolve copy（"作为新文件导入"）决策要回读源文件建档
///（2026-09-19 BUG 4 用户实锤：原落 cache 根且导入结束即删，冲突决策读不到源
/// 不停报"文件读取失败"，重试拿死路径再投连环失败）；Flatten + Scatter。
#[tauri::command]
pub fn import_clipboard(
  app: AppHandle,
  target_folder: Option<String>,
  on_event: Channel<ImportEvent>,
) -> Result<u64, String> {
  let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("剪贴板不可用：{e}"))?;
  let (root, assets_dir) = current_paths(&app)?;
  ensure_target(&app, &target_folder)?;

  // 图片分支：截图 / 剪贴板图片 → PNG 落 cache/tmp 走管线（无冲突挂起才即删，有冲突保留
  // 供 resolve copy 回读，开库整目录清空兜底）；
  // arboard 只认 PNG / CF_DIBV5——QQ·微信截图等只写 CF_DIB 的软件走 DIB 兜底
  //（2026-09-17 实机走查「外部剪贴板以前能导入现在不行」根因）
  let clip_image: Option<(u32, u32, Vec<u8>)> = clipboard
    .get_image()
    .ok()
    .map(|img| (img.width as u32, img.height as u32, img.bytes.into_owned()))
    .or_else(|| {
      crate::adapters::clipboard::read_image_dib_fallback()
        .ok()
        .map(|rgba| (rgba.width(), rgba.height(), rgba.into_raw()))
    });
  if let Some((w, h, bytes)) = clip_image {
    let cache_dir = root.join(CACHE_DIR).join("tmp");
    std::fs::create_dir_all(&cache_dir).map_err(|e| format!("缓存目录创建失败：{e}"))?;
    let tmp = cache_dir.join(clipboard_temp_name(&cache_dir, "png"));
    save_clipboard_png(w, h, &bytes, &tmp)?;
    let opts = ImportOptions {
      structure: Structure::Flatten,
      source: Source::Copy,
      target_folder,
      paths: vec![tmp.clone()],
      imported_from: None,
      sequence_mode: SequenceMode::Scatter, // 剪贴板只有单张 PNG，永不打包
    };
    return Ok(spawn_import(app, root, assets_dir, opts, on_event, Some(tmp)));
  }

  // 文件列表分支：资源管理器复制的文件 / 文件夹（CF_HDROP）
  let files = clipboard.get().file_list().unwrap_or_default();
  if !files.is_empty() {
    let opts = ImportOptions {
      structure: Structure::Flatten,
      source: Source::Copy,
      target_folder,
      paths: files,
      imported_from: None,
      sequence_mode: SequenceMode::Scatter, // 剪贴板恒分散（4.1 定案）
    };
    return Ok(spawn_import(app, root, assets_dir, opts, on_event, None));
  }

  // base64 文本分支：别处复制的文件 base64（data URI / 裸 base64）→ 魔数定扩展名
  // → 落 cache/tmp 临时文件走管线（命名与清理同 PNG 分支口径）
  if let Ok(text) = clipboard.get_text() {
    match parse_base64_payload(&text) {
      Ok(Some((ext, bytes))) => {
        let cache_dir = root.join(CACHE_DIR).join("tmp");
        std::fs::create_dir_all(&cache_dir).map_err(|e| format!("缓存目录创建失败：{e}"))?;
        let tmp = cache_dir.join(clipboard_temp_name(&cache_dir, ext));
        std::fs::write(&tmp, &bytes).map_err(|e| format!("剪贴板文件保存失败：{e}"))?;
        let opts = ImportOptions {
          structure: Structure::Flatten,
          source: Source::Copy,
          target_folder,
          paths: vec![tmp.clone()],
          imported_from: None,
          sequence_mode: SequenceMode::Scatter, // 单文件，永不打包
        };
        return Ok(spawn_import(app, root, assets_dir, opts, on_event, Some(tmp)));
      }
      Ok(None) => return Err("剪贴板里没有可导入的内容".to_string()),
      Err(e) => return Err(e),
    }
  }

  Err("剪贴板里没有图片或文件".to_string())
}

/// 取消导入任务：置位取消标记，工作线程在下一个文件处理前中断（数量如实进 Finished）
#[tauri::command]
pub fn cancel_import(state: State<AppState>, task_id: u64) -> Result<(), String> {
  let registry = state.imports.lock().map_err(|e| e.to_string())?;
  match registry.get(&task_id) {
    Some(flag) => {
      flag.store(true, Ordering::Relaxed);
      Ok(())
    }
    None => Err("导入任务不存在或已结束".to_string()),
  }
}

/// 冲突逐项决策（PRD 10_重复添加提示）：导入挂起的 hash 冲突，前端逐个弹窗收集
/// 用户决策后一次性提交；逐项执行、单项失败不阻塞，folders.json 一次落盘。
/// copy 建档重活在锁外执行（2026-09-19 BUG 4），锁内只做校验 + Placement 登记 + 落盘
#[tauri::command]
pub fn resolve_conflicts(
  state: State<AppState>,
  items: Vec<crate::services::resolve::ConflictResolution>,
  source: String,
) -> Result<crate::services::resolve::ResolveReport, String> {
  let source = parse_source(&source)?;
  crate::services::resolve::resolve_conflicts(&state.library, &items, source)
}

/// 短锁拿库 root 与 assets 池目录并校验库存在（命令唯一需要的共享状态）
pub(crate) fn current_paths(app: &AppHandle) -> Result<(PathBuf, PathBuf), String> {
  let state = app.state::<AppState>();
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  guard.as_ref().map(|l| (l.root.clone(), l.assets_dir.clone())).ok_or_else(|| "请先打开素材库再导入".to_string())
}

/// 短锁校验目标文件夹存在（None / "root" 直接放行），失败在 spawn 前即时报错
fn ensure_target(app: &AppHandle, target_folder: &Option<String>) -> Result<(), String> {
  let Some(id) = target_folder else { return Ok(()) };
  if id == ROOT_FOLDER_ID {
    return Ok(());
  }
  let state = app.state::<AppState>();
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库再导入".to_string())?;
  if lib.index.folders.contains_key(id) {
    Ok(())
  } else {
    Err("目标文件夹不存在，可能已被删除".to_string())
  }
}

/// 注册任务 + 起工作线程：线程内 run_import（分段短锁），结束注销任务；
/// 剪贴板临时文件（cleanup）**仅无冲突挂起才删**——有冲突挂起时保留供 resolve copy 回读
/// （读不到源 = 连环"文件读取失败"，2026-09-19 BUG 4）；残留由开库 cache/tmp 清空兜底
pub(crate) fn spawn_import(
  app: AppHandle,
  root: PathBuf,
  assets_dir: PathBuf,
  opts: ImportOptions,
  on_event: Channel<ImportEvent>,
  cleanup: Option<PathBuf>,
) -> u64 {
  let task_id = NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed);
  let cancel = Arc::new(AtomicBool::new(false));
  match app.state::<AppState>().imports.lock() {
    Ok(mut registry) => {
      registry.insert(task_id, cancel.clone());
    }
    Err(_) => log::error!("导入任务 {task_id} 注册失败（锁冲突），该任务将无法取消"),
  }
  std::thread::spawn(move || {
    let state = app.state::<AppState>();
    let outcome = run_import(&state.library, root, assets_dir, opts, move |ev| {
      if let Err(e) = on_event.send(ev) {
        log::debug!("导入事件推送失败（前端可能已关闭）：{e}");
      }
    }, &cancel);
    if let Some(p) = cleanup.filter(|p| p.exists()) {
      if outcome.conflicts.is_empty() {
        let _ = std::fs::remove_file(&p);
      }
    }
    if let Ok(mut registry) = state.imports.lock() {
      registry.remove(&task_id);
    };
  });
  task_id
}

/// 剪贴板临时文件名：剪贴板-yyyyMMdd-HHmm.扩展名，重名追加 _序号（2026-09-23 序号格式自 - 改 _）
fn clipboard_temp_name(cache_dir: &Path, ext: &str) -> String {
  let base = format!("剪贴板-{}", chrono::Local::now().format("%Y%m%d-%H%M"));
  let mut name = format!("{base}.{ext}");
  let mut n = 1;
  while cache_dir.join(&name).exists() {
    name = format!("{base}_{n}.{ext}");
    n += 1;
  }
  name
}

/// 剪贴板 RGBA 数据编码为 PNG 落盘（arboard / CF_DIB 兜底两路同喂）
fn save_clipboard_png(width: u32, height: u32, bytes: &[u8], path: &Path) -> Result<(), String> {
  let rgba = image::RgbaImage::from_raw(width, height, bytes.to_vec())
    .ok_or_else(|| "剪贴板图片数据无效".to_string())?;
  image::DynamicImage::ImageRgba8(rgba).save(path).map_err(|e| format!("剪贴板图片保存失败：{e}"))
}

/// 剪贴板 base64 单文件上限（512MB）：更大请先存成文件再导入
const CLIPBOARD_BASE64_MAX: usize = 512 * 1024 * 1024;

/// base64 文本 → (扩展名, 字节)：data URI（去 `data:...;base64,` 头）与裸 base64 都认，
/// 容忍空白换行；解码后按魔数定扩展名，魔数不在表内一律不认（Ok(None)）——
/// 普通文本可能是合法 base64 字符集，靠魔数兜底防误判入库
fn parse_base64_payload(text: &str) -> Result<Option<(&'static str, Vec<u8>)>, String> {
  use base64::Engine;
  let t = text.trim();
  let body = match t.strip_prefix("data:") {
    Some(rest) => rest.split_once(',').map(|(_, b)| b).unwrap_or(rest),
    None => t,
  };
  let compact: String = body.chars().filter(|c| !c.is_whitespace()).collect();
  if compact.len() < 16 {
    return Ok(None);
  }
  let bytes = base64::engine::general_purpose::STANDARD
    .decode(&compact)
    .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(&compact))
    .ok()
    .filter(|b| !b.is_empty());
  let Some(bytes) = bytes else { return Ok(None) };
  if bytes.len() > CLIPBOARD_BASE64_MAX {
    return Err("base64 内容超过 512MB，请先存成文件再导入".to_string());
  }
  Ok(sniff_ext(&bytes).map(|ext| (ext, bytes)))
}

/// 文件魔数 → 扩展名（剪贴板 base64 落盘定名用；只收白名单内魔数可靠的格式）
fn sniff_ext(b: &[u8]) -> Option<&'static str> {
  if b.starts_with(b"8BPS") {
    return Some("psd");
  }
  if b.starts_with(b"8BPB") {
    return Some("psb");
  }
  if b.starts_with(b"\x89PNG\r\n\x1a\n") {
    return Some("png");
  }
  if b.starts_with(b"\xff\xd8\xff") {
    return Some("jpg");
  }
  if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
    return Some("gif");
  }
  if b.starts_with(b"BM") {
    return Some("bmp");
  }
  if b.len() >= 12 && b.starts_with(b"RIFF") && &b[8..12] == b"WEBP" {
    return Some("webp");
  }
  if b.len() >= 12 && b.starts_with(b"RIFF") && &b[8..12] == b"WAVE" {
    return Some("wav");
  }
  if b.len() >= 12 && &b[4..8] == b"ftyp" {
    // QuickTime 品牌给 mov，其余品牌（isom / avc1 / M4V 等）统一 mp4
    return Some(if &b[8..12] == b"qt  " { "mov" } else { "mp4" });
  }
  if b.starts_with(b"ID3") || (b.len() >= 2 && b[0] == 0xff && b[1] & 0xe0 == 0xe0) {
    return Some("mp3");
  }
  // SVG 是文本格式：开头即 <svg，或 XML 声明后前 256 字节内出现 <svg
  let head = &b[..b.len().min(256)];
  if head.starts_with(b"<svg") || (head.starts_with(b"<?xml") && head.windows(4).any(|w| w == b"<svg")) {
    return Some("svg");
  }
  None
}

fn parse_structure(s: &str) -> Result<Structure, String> {
  match s {
    "flatten" => Ok(Structure::Flatten),
    "keep" => Ok(Structure::Keep),
    _ => Err(format!("未知的层级模式：{s}（可选 flatten / keep）")),
  }
}

fn parse_source(s: &str) -> Result<Source, String> {
  match s {
    "copy" => Ok(Source::Copy),
    "move" => Ok(Source::Move),
    _ => Err(format!("未知的来源模式：{s}（可选 copy / move）")),
  }
}

fn parse_sequence_mode(s: &str) -> Result<SequenceMode, String> {
  match s {
    "pack" => Ok(SequenceMode::Pack),
    "scatter" => Ok(SequenceMode::Scatter),
    _ => Err(format!("未知的序列帧模式：{s}（可选 pack / scatter）")),
  }
}

// ---------- 文件夹树与 Placement 列表（导入弹窗 / 中间面板） ----------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderNode {
  pub id: String,
  pub name: String,
  /// 该文件夹递归 Placement 总数（含全部子文件夹；2026-07-25 用户定：整洁层级下直接数会大面积为 0）
  pub count: u32,
  /// 递归容量（字节，2026-08-30 用户定案，与 count 同口径同趟算出）：子树全部 Placement 按 asset
  /// 去重后 size 合计——同一资产在父子文件夹各放一份只计一次（占用空间语义，同 archive.rs 档案容量）
  pub total_size: u64,
  /// 所属区块 ID（2026-08-14 区块子系统：仅顶级节点有值语义，None = 「目录」；子节点恒 None 不写字段）
  #[serde(skip_serializing_if = "Option::is_none")]
  pub block: Option<String>,
  /// 自定义图标（2026-08-14 区块子系统起顶级继承区块默认图标；2026-08-18 3.3 起全层级可持，None 不写字段）
  #[serde(skip_serializing_if = "Option::is_none")]
  pub icon: Option<String>,
  /// 节点形态（2026-08-15 档案子系统一轮：Some("archive") = 档案；所有层级都带；None 不写字段）
  #[serde(skip_serializing_if = "Option::is_none")]
  pub kind: Option<String>,
  /// 自定义颜色（2026-08-18，3.3）：色档名，所有层级都带；子级视觉跟随由前端底色档承担，None 不写字段
  #[serde(skip_serializing_if = "Option::is_none")]
  pub color: Option<String>,
  /// 分组规则（2026-08-19，3.5 二轮）：所有层级都带（档案与普通文件夹通用，PRD 02-04）；
  /// 空 / None 不写字段——分组计算在前端（显示层）
  #[serde(skip_serializing_if = "Option::is_none")]
  pub groups: Option<Vec<crate::core::models::GroupDef>>,
  pub children: Vec<FolderNode>,
}

/// 区块视图（侧栏分组依据），按 Block.order 排序输出
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockView {
  pub id: String,
  pub name: String,
  pub icon: Option<String>,
}

/// 文件夹树返回（2026-08-14 区块子系统）：blocks = 区块表（order 序）；nodes = 文件夹树
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderTreeView {
  pub blocks: Vec<BlockView>,
  pub nodes: Vec<FolderNode>,
}
/// 文件夹树（导入弹窗选目标文件夹用），按 Folder.order 排序
#[tauri::command]
pub fn list_folder_tree(state: State<AppState>) -> Result<FolderTreeView, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  Ok(folder_tree(lib))
}

/// 组装文件夹树：parent=None 为顶层，递归子级；区块表按 order 排序一并输出
/// pub(crate)：server/plugin_ws（插件目标文件夹选择器数据源，8.1）同用——结构整理随 8.x 再下迁 services
pub(crate) fn folder_tree(lib: &Library) -> FolderTreeView {
  /// 返回 (节点, 子树资产 id 集)：集合供父级并集去重算 totalSize（跨子树同资产只计一次）
  fn children_of<'a>(lib: &'a Library, parent: Option<&'a str>) -> Vec<(FolderNode, std::collections::HashSet<&'a str>)> {
    let mut folders: Vec<&crate::core::models::Folder> =
      lib.folders.folders.iter().filter(|f| f.parent.as_deref() == parent).collect();
    folders.sort_by_key(|f| f.order);
    folders
      .into_iter()
      .map(|f| {
        let pairs = children_of(lib, Some(&f.id));
        let (children, child_sets): (Vec<FolderNode>, Vec<std::collections::HashSet<&str>>) = pairs.into_iter().unzip();
        let direct_all = lib.index.placements_by_folder.get(&f.id).map(|v| v.as_slice()).unwrap_or(&[]);
        // hidden 伴生缩略图资产不计数（8.1，PRD 02-15：视图统一过滤，计数/容量同口径——
        // 否则"文件数 2 只见 1 卡"割裂，2026-09-01 用户实机报）
        let direct_ps: Vec<_> = direct_all
          .iter()
          .filter(|p| lib.index.assets.get(p.asset.as_str()).map(|m| m.hidden != Some(true)).unwrap_or(false))
          .collect();
        let direct = direct_ps.len() as u32;
        let recursive = direct + children.iter().map(|c| c.count).sum::<u32>();
        // 子树资产集 = 本级 placements ∪ 各子树集；totalSize = 集内资产 size 合计（缺失索引的脏引用跳过）
        let mut subtree: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for set in child_sets {
          subtree.extend(set);
        }
        subtree.extend(direct_ps.iter().map(|p| p.asset.as_str()));
        let total_size = subtree.iter().filter_map(|id| lib.index.assets.get(*id)).map(|m| m.size).sum::<u64>();
        // block 仅顶级节点输出（子节点不持区块语义）；icon / kind / color 全层级如实输出
        //（icon 2026-08-14 区块默认图标继承 → 3.3 起子级也可持自定义图标）
        let block = if parent.is_none() { f.block.clone() } else { None };
        let node = FolderNode {
          id: f.id.clone(),
          name: f.name.clone(),
          count: recursive,
          total_size,
          block,
          icon: f.icon.clone(),
          kind: f.kind.clone(),
          color: f.color.clone(),
          groups: f.groups.clone().filter(|g| !g.is_empty()), // 空组不输出（写回统一 None 口径）
          children,
        };
        (node, subtree)
      })
      .collect()
  }
  let mut blocks: Vec<&crate::core::models::Block> = lib.folders.blocks.iter().collect();
  blocks.sort_by_key(|b| b.order);
  FolderTreeView {
    blocks: blocks.into_iter().map(|b| BlockView { id: b.id.clone(), name: b.name.clone(), icon: b.icon.clone() }).collect(),
    nodes: children_of(lib, None).into_iter().map(|(n, _)| n).collect(),
  }
}

/// 列出 Placement：folder=None 全部；Some(id) 指定文件夹（"root" 表示根目录）
#[tauri::command]
pub fn list_placements(state: State<AppState>, folder: Option<String>) -> Result<Vec<PlacementView>, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  Ok(placement_views(lib, folder.as_deref()))
}

/// Placement → 视图：资源在索引中缺失的脏 Placement 跳过
/// pub(crate)：server/mcp（AI 外接通道 list_placements）同用
pub(crate) fn placement_views(lib: &Library, folder: Option<&str>) -> Vec<PlacementView> {
  let items: Vec<&Placement> = match folder {
    Some(id) => lib.index.placements_by_folder.get(id).map(|v| v.iter().collect()).unwrap_or_default(),
    None => lib.folders.items.iter().collect(),
  };
  items.into_iter().filter_map(|p| lib.index.assets.get(&p.asset).map(|m| placement_view(lib, m, p, &lib.format_icons))).collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library;
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

  /// 同步跑导入（事件丢弃）
  fn import(h: &Harness, paths: Vec<PathBuf>, structure: Structure) {
    let opts = ImportOptions {
      structure,
      source: Source::Copy,
      target_folder: None,
      paths,
      imported_from: None,
      sequence_mode: SequenceMode::Scatter,
    };
    run_import(&h.lib, h.root.clone(), h.assets_dir.clone(), opts, |_| {}, &AtomicBool::new(false));
  }

  fn with_lib<R>(h: &Harness, f: impl FnOnce(&Library) -> R) -> R {
    f(h.lib.lock().unwrap().as_ref().unwrap())
  }

  #[test]
  fn classify_distinguishes_kinds_and_counts() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("a.txt");
    std::fs::write(&f, "x").unwrap();
    let folder = dir.path().join("夹");
    std::fs::create_dir_all(folder.join("子")).unwrap();
    std::fs::write(folder.join("1.txt"), "1").unwrap();
    std::fs::write(folder.join("子/2.txt"), "2").unwrap();

    let s = |p: &Path| p.to_string_lossy().into_owned();
    let only_files = classify_paths(vec![s(&f)]);
    assert_eq!((only_files.kind.as_str(), only_files.file_count), ("files", 1));
    let only_folder = classify_paths(vec![s(&folder)]);
    assert_eq!((only_folder.kind.as_str(), only_folder.file_count), ("folder", 2));
    let mixed = classify_paths(vec![s(&f), s(&folder)]);
    assert_eq!((mixed.kind.as_str(), mixed.file_count), ("mixed", 3));
  }

  #[test]
  fn folder_tree_nests_and_counts_recursive_placements() {
    let h = make_lib();
    let src = h._dir.path().join("参考");
    std::fs::create_dir_all(src.join("人物")).unwrap();
    std::fs::write(src.join("a.txt"), "a").unwrap();
    std::fs::write(src.join("人物/b.txt"), "b").unwrap();
    import(&h, vec![src], Structure::Keep);

    let tree = with_lib(&h, folder_tree);
    assert_eq!(tree.nodes.len(), 1);
    // count = 递归总数：父级"参考"= 自身 a.txt 1 + 子文件夹"人物" b.txt 1
    assert_eq!((tree.nodes[0].name.as_str(), tree.nodes[0].count), ("参考", 2));
    assert_eq!((tree.nodes[0].children[0].name.as_str(), tree.nodes[0].children[0].count), ("人物", 1));
    // totalSize = 递归容量（2026-08-30 用户定案）：a.txt / b.txt 各 1 字节，父级 2、子级 1
    assert_eq!(tree.nodes[0].total_size, 2);
    assert_eq!(tree.nodes[0].children[0].total_size, 1);
    // camelCase 键名防线（踩坑 031 同类）
    assert!(serde_json::to_string(&tree.nodes[0]).unwrap().contains(r#""totalSize":2"#));
  }

  /// 递归容量跨层级去重（2026-08-30 用户定案）：同一资产在父、子文件夹各一份 placement，
  /// 父级 totalSize 只计一次；count 口径（逐条计 placement）不变
  #[test]
  fn folder_tree_total_size_dedupes_assets_across_levels() {
    let h = make_lib();
    let src = h._dir.path().join("库");
    std::fs::create_dir_all(src.join("子")).unwrap();
    std::fs::write(src.join("a.txt"), "aaa").unwrap(); // 3 字节
    std::fs::write(src.join("子/b.txt"), "bb").unwrap(); // 2 字节
    import(&h, vec![src], Structure::Keep);

    // 同资产 a 再放一份进子文件夹（跨层级重复放置；folders.items 与索引同步推入）
    let (asset, child_id) = with_lib(&h, |l| {
      (
        l.folders.items.iter().find(|p| p.name == "a.txt").unwrap().asset.clone(),
        l.folders.folders.iter().find(|f| f.name == "子").unwrap().id.clone(),
      )
    });
    {
      let mut guard = h.lib.lock().unwrap();
      let lib = guard.as_mut().unwrap();
      lib.folders.items.push(Placement { asset, folder: child_id, name: "a-链接.txt".into(), order: 1, origin: false });
      lib.index.rebuild_placements(&lib.folders.items);
    }

    let tree = with_lib(&h, folder_tree);
    let top = &tree.nodes[0];
    let child = &top.children[0];
    assert_eq!((top.count, top.total_size), (3, 5), "父级：3 条 placement，资产 a 去重后 3+2 字节");
    assert_eq!((child.count, child.total_size), (2, 5), "子级：2 条 placement，a+b 两资产 5 字节");
  }

  /// hidden 伴生缩略图资产不计数（8.1，PRD 02-15）：count / totalSize 与视图过滤同口径——
  /// 否则"文件数 2 只见 1 卡"割裂（2026-09-01 用户实机报）
  #[test]
  fn folder_tree_excludes_hidden_assets_from_counts() {
    let h = make_lib();
    let src = h._dir.path().join("库");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("工程.fla"), "fla").unwrap();
    std::fs::write(src.join("封面.png"), "png").unwrap();
    import(&h, vec![src], Structure::Keep);

    // 绑定：封面.png 打 hidden 成为 工程.fla 的伴生缩略图
    {
      let mut guard = h.lib.lock().unwrap();
      let lib = guard.as_mut().unwrap();
      let (fla, webp) = (
        lib.folders.items.iter().find(|p| p.name == "工程.fla").unwrap().asset.clone(),
        lib.folders.items.iter().find(|p| p.name == "封面.png").unwrap().asset.clone(),
      );
      crate::services::asset_menu::set_thumb_asset(lib, &fla, &webp).unwrap();
    }

    let tree = with_lib(&h, folder_tree);
    let top = &tree.nodes[0];
    assert_eq!(top.count, 1, "hidden 资产位置不计入 count（仅 fla）");
    assert_eq!(top.total_size, 3, "hidden 资产容量不计入 totalSize（仅 fla 3 字节）");
  }

  /// 区块输出（2026-08-14 区块子系统）：blocks 按 order 排序；顶级节点带 block，子节点不带
  #[test]
  fn folder_tree_outputs_blocks_ordered_and_top_level_block() {
    let h = make_lib();
    {
      let mut guard = h.lib.lock().unwrap();
      let lib = guard.as_mut().unwrap();
      // 默认区块「目录」order 0；插入两个自定义区块，order 故意乱序
      lib.folders.blocks.push(crate::core::models::Block { id: "B01".into(), name: "角色".into(), icon: Some("icon-role".into()), order: 2 });
      lib.folders.blocks.push(crate::core::models::Block { id: "B02".into(), name: "场景".into(), icon: None, order: 1 });
      let top = crate::core::models::Folder {
        id: "F01".into(),
        name: "顶夹".into(),
        parent: None,
        block: Some("B01".into()),
        icon: None,
        kind: None,
        color: None,
        groups: None,
        order: 0,
      };
      let child = crate::core::models::Folder {
        id: "F01-1".into(),
        name: "子夹".into(),
        parent: Some("F01".into()),
        block: Some("B02".into()), // 脏数据：子节点即使带值也不输出
        icon: None,
        kind: None,
        color: None,
        groups: None,
        order: 0,
      };
      lib.folders.folders.push(top.clone());
      lib.folders.folders.push(child.clone());
      lib.index.folders.insert(top.id.clone(), top);
      lib.index.folders.insert(child.id.clone(), child);
    }

    let tree = with_lib(&h, folder_tree);
    assert_eq!(
      tree.blocks.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(),
      vec!["dir", "B02", "B01"],
      "blocks 按 order 排序"
    );
    assert_eq!(tree.blocks[2].icon.as_deref(), Some("icon-role"));
    assert_eq!(tree.nodes[0].block.as_deref(), Some("B01"), "顶级节点带 block");
    assert_eq!(tree.nodes[0].children[0].block, None, "子节点不带 block");
    // 序列化：子节点不输出 block 字段（skip_serializing_if）
    let text = serde_json::to_string(&tree.nodes[0].children[0]).unwrap();
    assert!(!text.contains("block"));
  }

  /// 档案形态输出（2026-08-15 档案子系统一轮）：kind 所有层级都带；普通节点 None 不写字段
  #[test]
  fn folder_tree_outputs_kind_at_all_levels() {
    let h = make_lib();
    {
      let mut guard = h.lib.lock().unwrap();
      let lib = guard.as_mut().unwrap();
      let top = crate::core::models::Folder {
        id: "F01".into(),
        name: "档案".into(),
        parent: None,
        block: None,
        icon: None,
        kind: Some("archive".into()),
        color: None,
        groups: None,
        order: 0,
      };
      let child = crate::core::models::Folder {
        id: "F01-1".into(),
        name: "子夹".into(),
        parent: Some("F01".into()),
        block: None,
        icon: None,
        kind: Some("archive".into()), // 脏数据（档案不许子级）也不拦输出：树如实反映数据
        color: None,
        groups: None,
        order: 0,
      };
      lib.folders.folders.push(top.clone());
      lib.folders.folders.push(child.clone());
      lib.index.folders.insert(top.id.clone(), top);
      lib.index.folders.insert(child.id.clone(), child);
    }

    let tree = with_lib(&h, folder_tree);
    assert_eq!(tree.nodes[0].kind.as_deref(), Some("archive"), "顶级节点带 kind");
    assert_eq!(tree.nodes[0].children[0].kind.as_deref(), Some("archive"), "子节点同样带 kind");
    let text = serde_json::to_string(&tree.nodes[0]).unwrap();
    assert!(text.contains(r#""kind":"archive""#));
  }

  #[test]
  fn placement_views_join_meta_and_skip_missing() {
    let h = make_lib();
    let png = h._dir.path().join("图.png");
    image::RgbaImage::from_pixel(10, 20, image::Rgba([0, 0, 0, 255])).save(&png).unwrap();
    import(&h, vec![png], Structure::Flatten);

    let views = with_lib(&h, |l| placement_views(l, Some(ROOT_FOLDER_ID)));
    assert_eq!(views.len(), 1);
    assert_eq!((views[0].w, views[0].h), (Some(10), Some(20)));
    assert_eq!(views[0].ext, "png");
    assert!(views[0].origin);
    assert!(views[0].thumb_path.as_ref().unwrap().ends_with("_thumbnail.webp"));
    assert_eq!(views[0].thumb_kind, Some(crate::services::views::ThumbKind::Asset));

    // 封面帧透传（5.3 卡片悬停播放范围起点）：未设置为 None；设置后随视图下发
    assert_eq!(views[0].thumb_frame, None);
    let id = views[0].asset_id.clone();
    h.lib.lock().unwrap().as_mut().unwrap().index.assets.get_mut(&id).unwrap().thumb_frame =
      Some("t:3.500".into());
    let views = with_lib(&h, |l| placement_views(l, Some(ROOT_FOLDER_ID)));
    assert_eq!(views[0].thumb_frame.as_deref(), Some("t:3.500"));

    // 资源在索引中缺失的脏 Placement 跳过
    h.lib.lock().unwrap().as_mut().unwrap().folders.items.push(Placement {
      asset: "AN-不存在".into(),
      folder: ROOT_FOLDER_ID.into(),
      name: "幽灵.txt".into(),
      order: 1,
      origin: false,
    });
    assert_eq!(with_lib(&h, |l| placement_views(l, None).len()), 1);
  }

  #[cfg(windows)]
  #[test]
  fn import_non_image_falls_back_to_format_icon() {
    let h = make_lib();
    let fla = h._dir.path().join("工程.fla");
    std::fs::write(&fla, "fake fla bytes").unwrap();
    import(&h, vec![fla], Structure::Flatten);

    let views = with_lib(&h, |l| placement_views(l, Some(ROOT_FOLDER_ID)));
    assert_eq!(views.len(), 1);
    // 无内容缩略图 → 回退系统格式图标缓存（cache/thumbs，info 内无 shell 产物）
    let thumb = views[0].thumb_path.as_deref().unwrap();
    assert!(thumb.ends_with("fla.webp") && thumb.contains("thumbs"), "应回退格式图标缓存：{thumb}");
    assert_eq!(views[0].thumb_kind, Some(crate::services::views::ThumbKind::FormatIcon));
    assert!(h.root.join("cache/thumbs/fla.webp").is_file());
    assert!(with_lib(&h, |l| l.format_icons.contains("fla")));
    // 重开库后内存集从缓存扫描恢复
    let space_root = h.root.parent().unwrap().to_path_buf();
    let reopened = library::open(&space_root, &h.root).unwrap();
    assert!(reopened.format_icons.contains("fla"));
  }

  #[test]
  fn parse_modes_reject_unknown_values() {
    assert!(parse_structure("flatten").is_ok());
    assert!(parse_structure("树形").is_err());
    assert!(parse_source("move").is_ok());
    assert!(parse_source("拖拽").is_err());
    assert!(parse_sequence_mode("pack").is_ok());
    assert!(parse_sequence_mode("打包").is_err());
  }

  #[test]
  fn detect_sequences_groups_frames_and_skips_non_sequences() {
    let dir = tempfile::tempdir().unwrap();
    let png = |name: &str| {
      let p = dir.path().join(name);
      image::RgbaImage::from_pixel(4, 4, image::Rgba([0, 0, 0, 255])).save(&p).unwrap();
    };
    for n in 1..=3 {
      png(&format!("走路{n:03}.png"));
    }
    png("单张.png");
    let sub = dir.path().join("子");
    std::fs::create_dir_all(&sub).unwrap();
    for n in 1..=4 {
      image::RgbaImage::from_pixel(4, 4, image::Rgba([1, 1, 1, 255])).save(sub.join(format!("跑_{n}.png"))).unwrap();
    }

    let s = |p: &Path| p.to_string_lossy().into_owned();
    let groups = detect_sequences(vec![s(dir.path())]);
    assert_eq!(groups.len(), 2, "同目录成组、子目录递归展开：{groups:?}");
    let walk = groups.iter().find(|g| g.name == "走路").unwrap();
    assert_eq!((walk.ext.as_str(), walk.frame_count), ("png", 3));
    assert_eq!(groups.iter().find(|g| g.name == "跑").unwrap().frame_count, 4);
    // 无序列帧 → 空（走 09 现状流程）
    assert!(detect_sequences(vec![s(&dir.path().join("单张.png"))]).is_empty());
  }

  #[test]
  fn clipboard_temp_name_dedupes_with_counter() {
    let dir = tempfile::tempdir().unwrap();
    let first = clipboard_temp_name(dir.path(), "png");
    std::fs::write(dir.path().join(&first), "x").unwrap();
    let second = clipboard_temp_name(dir.path(), "png");
    assert_ne!(first, second);
    assert!(second.ends_with("_1.png"));
    // 扩展名透传（base64 分支按魔数定名）
    assert!(clipboard_temp_name(dir.path(), "psd").ends_with(".psd"));
  }

  #[test]
  fn base64_payload_psd_magic() {
    use base64::Engine;
    let raw = b"8BPS\x00\x01\x00\x00\x00\x00\x00\x00 fake psd bytes".to_vec();
    let text = base64::engine::general_purpose::STANDARD.encode(&raw);
    let (ext, bytes) = parse_base64_payload(&text).unwrap().unwrap();
    assert_eq!(ext, "psd");
    assert_eq!(bytes, raw);
  }

  #[test]
  fn base64_payload_data_uri_and_whitespace() {
    use base64::Engine;
    // data URI 前缀 + 换行折叠都认
    let raw = b"\x89PNG\r\n\x1a\n\x00\x00 fake png".to_vec();
    let b64 = base64::engine::general_purpose::STANDARD.encode(&raw);
    let text = format!("data:image/png;base64,{}", &b64[..b64.len() / 2]) + "\n" + &b64[b64.len() / 2..];
    let (ext, bytes) = parse_base64_payload(&text).unwrap().unwrap();
    assert_eq!(ext, "png");
    assert_eq!(bytes, raw);
  }

  #[test]
  fn base64_payload_rejects_plain_text() {
    // 普通文本（含非法字符）→ 不认
    assert!(parse_base64_payload("hello world, 这不是 base64").unwrap().is_none());
  }

  #[test]
  fn base64_payload_rejects_unknown_magic() {
    use base64::Engine;
    // 合法 base64 但魔数不在表内 → 不认（防普通文本解码出垃圾入库）
    let b64 = base64::engine::general_purpose::STANDARD.encode(b"just some random text bytes!!");
    assert!(parse_base64_payload(&b64).unwrap().is_none());
  }

  #[test]
  fn sniff_ext_covers_whitelist_magics() {
    assert_eq!(sniff_ext(b"8BPS\x00\x01...."), Some("psd"));
    assert_eq!(sniff_ext(b"8BPB\x00\x02...."), Some("psb"));
    assert_eq!(sniff_ext(b"\xff\xd8\xff\xe0...."), Some("jpg"));
    assert_eq!(sniff_ext(b"GIF89a...."), Some("gif"));
    assert_eq!(sniff_ext(b"BMxxxx...."), Some("bmp"));
    assert_eq!(sniff_ext(b"RIFF....WEBP"), Some("webp"));
    assert_eq!(sniff_ext(b"RIFF....WAVE"), Some("wav"));
    assert_eq!(sniff_ext(b"\x00\x00\x00\x18ftypqt  "), Some("mov"));
    assert_eq!(sniff_ext(b"\x00\x00\x00\x18ftypisom"), Some("mp4"));
    assert_eq!(sniff_ext(b"ID3\x04\x00\x00\x00"), Some("mp3"));
    assert_eq!(sniff_ext(b"<svg xmlns=\"http://www.w3.org/2000/svg\">"), Some("svg"));
    assert_eq!(sniff_ext(b"<?xml version=\"1.0\"?>\n<svg></svg>"), Some("svg"));
    assert_eq!(sniff_ext(b"not a known format"), None);
  }
}
