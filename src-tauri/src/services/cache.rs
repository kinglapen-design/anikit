//! 缓存占用统计与一键清理服务（设置面板缓存页后端，PRD 06_数据 §07：
//! 设置页展示各类缓存占用大小、提供一键清理；所有缓存均可重建，清理不影响用户数据）。
//!
//! 缓存分布与语义（均为可重建缓存；写操作红线 = 不碰任何用户数据）：
//! - 库侧（assets 池为空间级共享，多库共池不按库切分）：
//!   - 缩略图与动图 = info 内 `{stem}_thumbnail.webp|png`（metadata.thumbnail 引用）+
//!     `{stem}_anim.webp`（动图卡片播放缓存，core::paths::anim_preview_file 推导）。
//!     **只删 metadata 引用与按规则推导的缓存文件，不按文件名后缀盲扫**——用户原始文件
//!     可能恰好叫「xx_thumbnail.png」（Eagle 导出惯例），盲删会删到用户数据。
//!     缩略图删除后不会自动重建（views::thumb_path 只拼路径不判存在）→ 清理后由命令层
//!     触发全库后台重建（rebuild_missing_thumbs）；动图缓存不主动重建（卡片播放链
//!     get_asset_file_path 懒生成补提交，anim_preview::ensure_anim_preview 幂等）。
//!   - 预览与帧缓存 = cache/previews + cache/frames + cache/note-thumbs + cache/tmp
//!     （会话级，开库即清；此处再提供手动清。tmp 内含「有冲突挂起的剪贴板导入源文件」——
//!     清理会使进行中的冲突决策回读不到源（报「文件读取失败」，重新复制粘贴即可）；
//!     该窗口属用户显式操作的罕见交叠，保留清理能力并在接口文档写明）。
//!   - 音视频转码与波形 = cache/media + cache/peaks（跨会话保留）+ cache/waveforms
//!     （旧遗留目录，已无写入方；开库清理名单仍含它，一并计入清掉）。
//!   - 系统格式图标 = cache/thumbs（按扩展名缓存）。清完**立即按在库资产的实际扩展名集
//!     重提一轮**（无懒生成兜底——格式图标只在导入 / 开库时建立，不重提则非缩略图格式
//!     卡片全 blank 到重启）；内存 format_icons 集由命令层同口径同步。
//! - 工作空间侧（{空间根}/workspace/.anikit/，缓存键内嵌 mtime+size）：
//!   - 缩略图与图标 = .anikit/thumbs；音视频转码与波形 = .anikit/media + .anikit/peaks；
//!     探测缓存 = .anikit/probe.json。
//!   - **红线：.anikit/workspace.json 是用户数据**（常用固定 pinned + junction 台账），
//!     任何清理分支都不碰；cache_v2 命名版本标志同保留（删了会触发开空间一次性迁移误清）。
//!   - 与 ws_thumb_queue 的兼容（2026-10-13 确认）：去重键与失败记忆键 =
//!     （workspace 根, job_key(rel+mtime+size)）——清理只删缓存产物、源文件不动，键不变，
//!     记忆集合依旧有效；fresh_thumb_path 按当前 mtime+size 重算期望文件名查存在，
//!     文件被删即失鲜 → 下次 list_dir 自动重新入队重建，无需任何额外同步。
//!     清理前调 ws_thumb_queue::clear_pending() 收缩窗口（在跑任务落盘幂等无害）。
//!
//! 统计 / 清理均为冷路径（设置页用户触发），允许遍历 assets 池索引快照与 cache 目录树。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;

use crate::core::json_io::write_json;
use crate::core::library::{Library, CACHE_DIR};
use crate::core::models::AssetMeta;
use crate::core::paths;
use crate::services::thumbnail;

/// 缩略图重建完成事件名（负载 = RebuildReport；前端可据此刷新列表视图）
pub const THUMBS_REBUILT_EVENT: &str = "library-thumbs-rebuilt";

// ---------- 类别枚举与 id 口径 ----------

/// 缓存类别（clear_cache 入参 id 与 overview 条目 id 同源）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheCategory {
  /// 库：缩略图与动图（info 内 thumbnail + anim 缓存）
  LibThumbs,
  /// 库：预览与帧缓存（cache/previews + frames + note-thumbs + tmp）
  LibPreviews,
  /// 库：音视频转码与波形（cache/media + peaks + waveforms 旧遗留）
  LibMedia,
  /// 库：系统格式图标（cache/thumbs）
  LibIcons,
  /// 工作空间：缩略图与图标（.anikit/thumbs）
  WsThumbs,
  /// 工作空间：音视频转码与波形（.anikit/media + peaks）
  WsMedia,
  /// 工作空间：探测缓存（.anikit/probe.json）
  WsProbe,
}

impl CacheCategory {
  pub const ALL: [CacheCategory; 7] = [
    CacheCategory::LibThumbs,
    CacheCategory::LibPreviews,
    CacheCategory::LibMedia,
    CacheCategory::LibIcons,
    CacheCategory::WsThumbs,
    CacheCategory::WsMedia,
    CacheCategory::WsProbe,
  ];

  /// 解析命令入参 id；未知 id 拒绝（清理只删枚举内目录，不给任意路径开口）
  pub fn parse(id: &str) -> Result<Self, String> {
    Ok(match id {
      "lib-thumbs" => CacheCategory::LibThumbs,
      "lib-previews" => CacheCategory::LibPreviews,
      "lib-media" => CacheCategory::LibMedia,
      "lib-icons" => CacheCategory::LibIcons,
      "ws-thumbs" => CacheCategory::WsThumbs,
      "ws-media" => CacheCategory::WsMedia,
      "ws-probe" => CacheCategory::WsProbe,
      _ => return Err(format!("未知的缓存类别：{id}")),
    })
  }

  pub fn id(self) -> &'static str {
    match self {
      CacheCategory::LibThumbs => "lib-thumbs",
      CacheCategory::LibPreviews => "lib-previews",
      CacheCategory::LibMedia => "lib-media",
      CacheCategory::LibIcons => "lib-icons",
      CacheCategory::WsThumbs => "ws-thumbs",
      CacheCategory::WsMedia => "ws-media",
      CacheCategory::WsProbe => "ws-probe",
    }
  }

  /// 展示名（后端为唯一事实来源，前端零映射）
  pub fn label(self) -> &'static str {
    match self {
      CacheCategory::LibThumbs => "缩略图与动图",
      CacheCategory::LibPreviews => "预览与帧缓存",
      CacheCategory::LibMedia => "音视频转码与波形",
      CacheCategory::LibIcons => "系统格式图标",
      CacheCategory::WsThumbs => "缩略图与图标",
      CacheCategory::WsMedia => "音视频转码与波形",
      CacheCategory::WsProbe => "探测缓存",
    }
  }

  /// 所属分组 id（"library" / "workspace"）
  pub fn group(self) -> &'static str {
    match self {
      CacheCategory::LibThumbs | CacheCategory::LibPreviews | CacheCategory::LibMedia | CacheCategory::LibIcons => "library",
      CacheCategory::WsThumbs | CacheCategory::WsMedia | CacheCategory::WsProbe => "workspace",
    }
  }
}

// ---------- 返回结构 ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheItem {
  pub id: String,
  pub label: String,
  /// 占用字节数
  pub bytes: u64,
  /// 文件数
  pub files: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheGroup {
  pub id: String,
  pub label: String,
  pub items: Vec<CacheItem>,
}

/// 缓存总览：两组恒在、各项恒在；库未打开 → 资源库组各项 0；工作空间目录不存在 →
/// 工作空间组各项 0（条目不省略，前端布局稳定）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheOverview {
  pub groups: Vec<CacheGroup>,
}

/// 清理结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearOutcome {
  pub category: String,
  /// 实际释放字节数（仅统计删除成功的文件）
  pub freed_bytes: u64,
  /// 删除成功文件数
  pub removed_files: u64,
  /// 删除失败（被占用等）跳过的文件数——播放器正在播放的转码产物属此类
  pub failed_files: u64,
  /// 是否已触发全库缩略图后台重建（仅 lib-thumbs 清理后可能为 true）
  pub rebuilding_thumbs: bool,
}

// ---------- 统计 ----------

/// 目录递归求和（字节数, 文件数）：目录不存在 / 读不了按 0 计（只读探测语义）。
/// 符号链接 / junction 不跟随不计大小（缓存目录均为 APP 自管，不出现；防御性口径）
fn dir_sum(dir: &Path) -> (u64, u64) {
  let mut bytes = 0u64;
  let mut files = 0u64;
  if let Ok(rd) = fs::read_dir(dir) {
    for e in rd.flatten() {
      let Ok(ft) = e.file_type() else { continue };
      if ft.is_dir() {
        let (b, f) = dir_sum(&e.path());
        bytes += b;
        files += f;
      } else if ft.is_file() {
        bytes += e.metadata().map(|m| m.len()).unwrap_or(0);
        files += 1;
      }
    }
  }
  (bytes, files)
}

/// 单文件大小（不存在 / 读取失败按 0,0 计）
fn file_sum(path: &Path) -> (u64, u64) {
  match fs::metadata(path) {
    Ok(m) if m.is_file() => (m.len(), 1),
    _ => (0, 0),
  }
}

/// 库侧信息快照（命令层短锁克隆，锁外 IO 用）
pub struct LibSnapshot {
  pub root: PathBuf,
  pub assets_dir: PathBuf,
  /// 全池资产（id + originName + thumbnail 引用），统计 / 清理的按 ID 推导依据
  pub metas: Vec<AssetMeta>,
}

/// 单资产 info 内的缓存文件清单（缩略图 metadata 引用 + 动图推导名）。
/// thumbnail 引用做组件级校验（必须是纯文件名）——metadata 是 APP 自写可信数据，
/// 此处纵深防御拦 "../../" 形态，保证清理永不越出 info 目录
fn thumb_cache_files(assets_dir: &Path, meta: &AssetMeta) -> Vec<PathBuf> {
  let mut out = Vec::with_capacity(2);
  if let Some(t) = &meta.thumbnail {
    let mut comps = Path::new(t).components();
    if matches!(comps.next(), Some(std::path::Component::Normal(_))) && comps.next().is_none() {
      out.push(paths::info_dir(assets_dir, &meta.id).join(t));
    }
  }
  out.push(paths::anim_preview_file(assets_dir, &meta.id, &meta.origin_name));
  out
}

/// 库缩略图与动图统计（按 ID 推导 + metadata 引用，不枚举 assets/ 根——池访问纪律冷路径同守）
fn lib_thumbs_sum(snap: &LibSnapshot) -> (u64, u64) {
  let mut total = (0u64, 0u64);
  for m in &snap.metas {
    for p in thumb_cache_files(&snap.assets_dir, m) {
      let (b, f) = file_sum(&p);
      total.0 += b;
      total.1 += f;
    }
  }
  total
}

/// 单类别统计：库类需快照（None = 库未打开 → 0），工作空间类需 ws 根（None = 未开启 → 0）
fn stat_category(cat: CacheCategory, lib: Option<&LibSnapshot>, ws_root: Option<&Path>) -> (u64, u64) {
  match cat {
    CacheCategory::LibThumbs => lib.map(lib_thumbs_sum).unwrap_or((0, 0)),
    CacheCategory::LibPreviews => lib.map(|s| sum_dirs(&s.root, &["previews", "frames", "note-thumbs", "tmp"])).unwrap_or((0, 0)),
    // waveforms = 旧遗留目录（已无写入方，开库清理名单仍含它）：一并计入，清理同清
    CacheCategory::LibMedia => lib.map(|s| sum_dirs(&s.root, &["media", "peaks", "waveforms"])).unwrap_or((0, 0)),
    CacheCategory::LibIcons => lib.map(|s| dir_sum(&s.root.join(CACHE_DIR).join("thumbs"))).unwrap_or((0, 0)),
    CacheCategory::WsThumbs => ws_root.map(|r| dir_sum(&r.join(crate::services::workspace::CACHE_DIR).join("thumbs"))).unwrap_or((0, 0)),
    CacheCategory::WsMedia => ws_root.map(|r| sum_ws_dirs(r, &["media", "peaks"])).unwrap_or((0, 0)),
    CacheCategory::WsProbe => ws_root.map(|r| file_sum(&r.join(crate::services::workspace::CACHE_DIR).join("probe.json"))).unwrap_or((0, 0)),
  }
}

/// 库根 cache/ 下多个子目录求和
fn sum_dirs(lib_root: &Path, subs: &[&str]) -> (u64, u64) {
  let mut total = (0u64, 0u64);
  for sub in subs {
    let (b, f) = dir_sum(&lib_root.join(CACHE_DIR).join(sub));
    total.0 += b;
    total.1 += f;
  }
  total
}

/// 工作空间 .anikit/ 下多个子目录求和
fn sum_ws_dirs(ws_root: &Path, subs: &[&str]) -> (u64, u64) {
  let mut total = (0u64, 0u64);
  for sub in subs {
    let (b, f) = dir_sum(&ws_root.join(crate::services::workspace::CACHE_DIR).join(sub));
    total.0 += b;
    total.1 += f;
  }
  total
}

/// 总览组装：两组恒在、各项恒在，未开启侧各项 0
pub fn overview(lib: Option<&LibSnapshot>, ws_root: Option<&Path>) -> CacheOverview {
  let items_of = |group: &str| -> Vec<CacheItem> {
    CacheCategory::ALL
      .into_iter()
      .filter(|c| c.group() == group)
      .map(|c| {
        let (bytes, files) = stat_category(c, lib, ws_root);
        CacheItem { id: c.id().to_string(), label: c.label().to_string(), bytes, files }
      })
      .collect()
  };
  CacheOverview {
    groups: vec![
      CacheGroup { id: "library".to_string(), label: "资源库".to_string(), items: items_of("library") },
      CacheGroup { id: "workspace".to_string(), label: "工作空间".to_string(), items: items_of("workspace") },
    ],
  }
}

// ---------- 清理 ----------

/// 清空目录内容（顶层目录保留）：逐条删除，被占用 / 失败文件跳过计数（播放器持有中的
/// 转码产物属此类）；符号链接只删链接本体不跟随（防御——缓存目录自管不应出现）。
/// 返回（释放字节, 删除文件数, 失败文件数）
fn clear_dir_contents(dir: &Path) -> (u64, u64, u64) {
  let mut out = (0u64, 0u64, 0u64);
  let Ok(rd) = fs::read_dir(dir) else { return out };
  for e in rd.flatten() {
    let path = e.path();
    let Ok(ft) = e.file_type() else { continue };
    if ft.is_dir() {
      let sub = clear_dir_contents(&path);
      out.0 += sub.0;
      out.1 += sub.1;
      out.2 += sub.2;
      // 子目录腾空后顺手收掉；收不掉（有失败残留）留着不碍事
      let _ = fs::remove_dir(&path);
    } else {
      let len = e.metadata().map(|m| m.len()).unwrap_or(0);
      if fs::remove_file(&path).is_ok() {
        out.0 += len;
        out.1 += 1;
      } else {
        log::warn!("缓存文件删除失败（跳过）：{}", path.display());
        out.2 += 1;
      }
    }
  }
  out
}

/// 库缩略图与动图清理：只删 metadata 引用 + 按 ID/规则推导的缓存文件（见模块头红线）。
/// 返回（释放字节, 删除文件数, 失败文件数）；重建由命令层随后触发
pub fn clear_lib_thumbs(snap: &LibSnapshot) -> (u64, u64, u64) {
  let mut out = (0u64, 0u64, 0u64);
  for m in &snap.metas {
    for p in thumb_cache_files(&snap.assets_dir, m) {
      match fs::metadata(&p) {
        Ok(md) if md.is_file() => {
          if fs::remove_file(&p).is_ok() {
            out.0 += md.len();
            out.1 += 1;
          } else {
            log::warn!("缩略图缓存删除失败（跳过）：{}", p.display());
            out.2 += 1;
          }
        }
        _ => {}
      }
    }
  }
  out
}

/// 系统格式图标清理 + 即时重提：清 cache/thumbs 后，按在库资产实际用到的扩展名集
/// （无内容缩略图的资产才会用到格式图标）逐格式 ensure 重提一轮，返回（释放, 删除, 失败, 重提出的扩展名集）。
/// 不重提的后果：非缩略图格式（fla 等）卡片 blank 到重启——格式图标无懒生成兜底
///（仅导入 / 开库扫描两个建立点）
pub fn clear_lib_icons(root: &Path, assets_dir: &Path, metas: &[AssetMeta]) -> (u64, u64, u64, std::collections::HashSet<String>) {
  let out = clear_dir_contents(&root.join(CACHE_DIR).join("thumbs"));
  // 用到的扩展名集：无内容缩略图的资产（views 回退链 = format_icons 集 + cache/thumbs 文件）
  let mut exts: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
  for m in metas.iter().filter(|m| m.thumbnail.is_none()) {
    exts.insert(m.ext.clone());
  }
  let mut ready = std::collections::HashSet::new();
  for ext in exts {
    // 取该扩展名任一 origin 作为图标提取源（同扩展名同图标；源缺失试下一个）
    for m in metas.iter().filter(|m| m.thumbnail.is_none() && m.ext == ext) {
      let origin = paths::origin_file(assets_dir, &m.id, &m.origin_name);
      if origin.is_file() && thumbnail::ensure_format_icon(root, &origin, &ext) {
        ready.insert(ext.clone());
        break;
      }
    }
  }
  (out.0, out.1, out.2, ready)
}

/// 工作空间侧清理：ws-thumbs 先摘队列 pending（在跑任务落盘幂等无害）再清目录；
/// ws-probe 删单文件。workspace.json / cache_v2 不在任何分支名单内（用户数据 / 版本标志）
pub fn clear_ws_category(cat: CacheCategory, ws_root: &Path) -> (u64, u64, u64) {
  let cache = ws_root.join(crate::services::workspace::CACHE_DIR);
  match cat {
    CacheCategory::WsThumbs => {
      // 与队列键兼容（键内嵌 mtime+size，源没变键不变）：删产物后下次 list_dir
      // 经 fresh 判定自动重新入队重建；pending 先摘防写回已删目录白跑一趟
      crate::services::ws_thumb_queue::clear_pending();
      clear_dir_contents(&cache.join("thumbs"))
    }
    CacheCategory::WsMedia => {
      let a = clear_dir_contents(&cache.join("media"));
      let b = clear_dir_contents(&cache.join("peaks"));
      (a.0 + b.0, a.1 + b.1, a.2 + b.2)
    }
    CacheCategory::WsProbe => {
      let (b, f) = file_sum(&cache.join("probe.json"));
      match fs::remove_file(cache.join("probe.json")) {
        Ok(()) => (b, f, 0),
        Err(_) => (0, 0, f), // 不存在也走这里（0,0,0）；存在但删不掉计失败
      }
    }
    _ => (0, 0, 0),
  }
}

/// 库侧目录类清理（lib-previews / lib-media）
pub fn clear_lib_dirs(cat: CacheCategory, root: &Path) -> (u64, u64, u64) {
  let subs: &[&str] = match cat {
    CacheCategory::LibPreviews => &["previews", "frames", "note-thumbs", "tmp"],
    CacheCategory::LibMedia => &["media", "peaks", "waveforms"],
    _ => &[],
  };
  let mut out = (0u64, 0u64, 0u64);
  for sub in subs {
    let r = clear_dir_contents(&root.join(CACHE_DIR).join(sub));
    out.0 += r.0;
    out.1 += r.1;
    out.2 += r.2;
  }
  out
}

// ---------- 全库缩略图后台重建 ----------

/// 重建报告（THUMBS_REBUILT_EVENT 负载）
#[derive(Debug, Default, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RebuildReport {
  /// 本趟重建成功的资产数
  pub rebuilt: u32,
  /// 重建失败（源缺失 / 解码失败）回退格式图标的资产数（thumbnail 引用已清）
  pub fallback: u32,
}

impl RebuildReport {
  /// pub(crate)：命令层重建线程跨补趟累计用
  pub(crate) fn merge(&mut self, other: &RebuildReport) {
    self.rebuilt += other.rebuilt;
    self.fallback += other.fallback;
  }
}

/// 待重建条目快照（短锁取自索引）
struct RebuildTarget {
  id: String,
  name: String,
  origin_name: String,
}

/// 单趟重建：处理「metadata.thumbnail 有引用但文件已不在」的全部资产（清理后的 dangling
/// 集；顺带自愈一切其他途径丢文件的存量）。逐资产：锁外解码生成（重活不占库锁）→ 短锁内
/// 重读最新 meta → 有变化才写 metadata.json 并同步索引（维护性写，不刷 modifiedAt，
/// 同 ensure_asset_colors 懒回填口径）。
/// 空间切换护栏：短锁内 assets_dir 与趟首快照不一致 = 库已切换，磁盘写入停止（旧空间
/// 已生成的新缩略图文件留着无害），索引绝不错写进新库。
/// 返回（本趟报告, 趟首快照时缺失引用总数——0 = 无活可干，调用方据此终止补趟循环）
pub fn rebuild_missing_thumbs(lib: &Mutex<Option<Library>>) -> (RebuildReport, u32) {
  // 趟首短锁快照：缺失集 = 有 thumbnail 引用且文件已不存在
  let (assets_dir, targets) = {
    let guard = match lib.lock() {
      Ok(g) => g,
      Err(_) => return (RebuildReport::default(), 0),
    };
    let Some(l) = guard.as_ref() else { return (RebuildReport::default(), 0) };
    let assets_dir = l.assets_dir.clone();
    let targets: Vec<RebuildTarget> = l
      .index
      .assets
      .values()
      .filter(|m| m.thumbnail.as_ref().is_some_and(|t| !paths::info_dir(&l.assets_dir, &m.id).join(t).is_file()))
      .map(|m| RebuildTarget { id: m.id.clone(), name: m.name.clone(), origin_name: m.origin_name.clone() })
      .collect();
    (assets_dir, targets)
  };
  let total = targets.len() as u32;
  let mut report = RebuildReport::default();
  for t in targets {
    rebuild_one(lib, &assets_dir, &t, &mut report);
  }
  (report, total)
}

/// 单资产重建：锁外生成 → 短锁内校验库身份 → 落盘 + 索引同步
fn rebuild_one(lib: &Mutex<Option<Library>>, assets_dir: &Path, t: &RebuildTarget, report: &mut RebuildReport) {
  let info_dir = paths::info_dir(assets_dir, &t.id);
  // 序列帧资产的 origin 是 frames/ 内封面帧（info 根无实体，sequence::frames_dir_if_sequence 同口径）
  let origin = {
    let direct = info_dir.join(&t.origin_name);
    if direct.is_file() {
      direct
    } else {
      paths::frames_dir(assets_dir, &t.id).join(&t.origin_name)
    }
  };
  // 锁外重活：源缺失 / 解码失败 → None（回退格式图标，与 origin_sync 手动刷新同口径）
  let thumb = if origin.is_file() { thumbnail::generate(&info_dir, &origin, &t.name) } else { None };
  let mut guard = match lib.lock() {
    Ok(g) => g,
    Err(_) => return,
  };
  let Some(l) = guard.as_mut() else { return };
  if l.assets_dir != assets_dir {
    return; // 库 / 空间已切换：本趟快照作废（磁盘上已生成的缩略图文件无害）
  }
  let Some(mut meta) = l.index.assets.get(&t.id).cloned() else { return };
  match thumb {
    Some(out) => {
      // 磁盘已重新生成 → 计数；metadata 字段与原引用一致（同名同尺寸）则零写盘零同步
      report.rebuilt += 1;
      let unchanged = meta.thumbnail.as_deref() == Some(out.name.as_str())
        && meta.thumb_width == Some(out.width)
        && meta.thumb_height == Some(out.height)
        && meta.colors == out.colors;
      if unchanged {
        return;
      }
      meta.thumbnail = Some(out.name);
      meta.thumb_width = Some(out.width);
      meta.thumb_height = Some(out.height);
      meta.colors = out.colors;
    }
    None => {
      log::warn!("缩略图重建失败（回退格式图标）：{}（{}）", t.name, t.id);
      meta.thumbnail = None;
      meta.thumb_width = None;
      meta.thumb_height = None;
      meta.colors = Vec::new();
      report.fallback += 1;
    }
  }
  // 先落盘再同步索引（规范第 7 条）；落盘失败仅记日志——索引不更新，与盘面保持一致
  match write_json(&info_dir.join("metadata.json"), &meta) {
    Ok(()) => {
      l.index.assets.insert(t.id.clone(), meta);
    }
    Err(e) => log::error!("缩略图重建 metadata 落盘失败（{}）：{e}", t.id),
  }
}


#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library;
  use crate::core::space::WORKSPACE_DIR;
  use crate::services::import::{self as ops, PendingFile, Source};
  use crate::services::sequence;

  fn make_png(path: &Path, w: u32, h: u32) {
    image::RgbaImage::from_pixel(w, h, image::Rgba([200, 30, 30, 255])).save(path).unwrap();
  }

  /// 建测试库并导入一张真实 png（走真实建档管线 = 真缩略图，资产入索引）；返回库
  fn make_lib_with_png() -> (tempfile::TempDir, Library) {
    let dir = tempfile::tempdir().unwrap();
    let mut lib = library::create_test(dir.path());
    let src = dir.path().join("图.png");
    make_png(&src, 640, 480);
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = ops::build_asset_with_id(&lib.assets_dir, &pf, "sha256:t", "AN-IMG", Source::Copy, None).unwrap();
    assert!(meta.thumbnail.is_some(), "png 建档应产出缩略图");
    lib.index.assets.insert("AN-IMG".into(), meta);
    (dir, lib)
  }

  #[test]
  fn dir_sum_counts_nested_and_missing() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("a/b")).unwrap();
    fs::write(dir.path().join("a/一.bin"), b"12345").unwrap();
    fs::write(dir.path().join("a/b/二.bin"), b"678").unwrap();
    assert_eq!(dir_sum(&dir.path().join("a")), (8, 2), "递归求和");
    assert_eq!(dir_sum(&dir.path().join("不存在")), (0, 0), "缺失目录按 0 计");
  }

  /// 红线：metadata.thumbnail 引用做组件级校验，"../" 形态进不了清理清单（永不越出 info）
  #[test]
  fn thumb_cache_files_reject_traversal_refs() {
    let assets = Path::new("D:/空间/assets");
    let mut meta: AssetMeta = serde_json::from_value(serde_json::json!({
      "version": 1, "id": "AN-A", "name": "a", "originName": "a.png", "ext": "png", "size": 1,
      "importedAt": "2026-10-13T00:00:00+08:00", "modifiedAt": "2026-10-13T00:00:00+08:00"
    }))
    .unwrap();
    meta.thumbnail = Some("../../folders.json".into());
    let files = thumb_cache_files(assets, &meta);
    assert_eq!(files.len(), 1, "穿越引用被剔除，只剩动图推导名");
    assert!(files[0].ends_with("a_anim.webp"));
    meta.thumbnail = Some("a_thumbnail.webp".into());
    let files = thumb_cache_files(assets, &meta);
    assert_eq!(files.len(), 2);
    assert!(files[0].ends_with("AN-A.info/a_thumbnail.webp") || files[0].ends_with("AN-A.info\\a_thumbnail.webp"));
  }

  /// 分组统计：库四类 + 工作空间三类逐项字节/文件数；库 / 工作空间缺失侧各项 0（条目不省略）
  #[test]
  fn overview_groups_items_and_zeros() {
    let (dir, lib) = make_lib_with_png();
    let root = &lib.root;
    // 库侧种子：格式图标 1 个；media/peaks/waveforms 各 1；previews/frames/note-thumbs/tmp 各 1
    fs::create_dir_all(root.join("cache/thumbs")).unwrap();
    fs::write(root.join("cache/thumbs/fla.webp"), b"10").unwrap();
    for sub in ["media", "peaks", "waveforms", "previews", "frames", "note-thumbs", "tmp"] {
      fs::create_dir_all(root.join("cache").join(sub)).unwrap();
      fs::write(root.join("cache").join(sub).join("x.bin"), b"123").unwrap();
    }
    // 工作空间侧种子（ws 根 = 空间根/workspace）
    let ws = lib.space_root.join(WORKSPACE_DIR);
    fs::create_dir_all(ws.join(".anikit/thumbs")).unwrap();
    fs::write(ws.join(".anikit/thumbs/k.webp"), b"1234").unwrap();
    fs::create_dir_all(ws.join(".anikit/media")).unwrap();
    fs::write(ws.join(".anikit/media/m.mp4"), b"12").unwrap();
    fs::create_dir_all(ws.join(".anikit/peaks")).unwrap();
    fs::write(ws.join(".anikit/peaks/p.json"), b"1").unwrap();
    fs::write(ws.join(".anikit/probe.json"), b"{}").unwrap();
    fs::write(ws.join(".anikit/workspace.json"), b"{}").unwrap(); // 用户数据：不进任何统计

    let snap = LibSnapshot { root: lib.root.clone(), assets_dir: lib.assets_dir.clone(), metas: lib.index.assets.values().cloned().collect() };
    let ov = overview(Some(&snap), Some(&ws));
    assert_eq!(ov.groups.len(), 2);
    let item = |g: &str, id: &str| {
      ov.groups
        .iter()
        .find(|g0| g0.id == g)
        .and_then(|g0| g0.items.iter().find(|i| i.id == id))
        .unwrap_or_else(|| panic!("缺条目 {g}/{id}"))
        .clone()
    };
    // 库：缩略图 = 真实 png 缩略图（1 文件，字节数 = 实际大小）；动图缓存不存在不计
    let thumbs = item("library", "lib-thumbs");
    let thumb_file = lib.info_dir("AN-IMG").join("图_thumbnail.webp");
    assert_eq!((thumbs.files, thumbs.bytes), (1, fs::metadata(&thumb_file).unwrap().len()));
    assert_eq!((item("library", "lib-icons").files, item("library", "lib-icons").bytes), (1, 2));
    assert_eq!((item("library", "lib-media").files, item("library", "lib-media").bytes), (3, 9), "media+peaks+waveforms");
    assert_eq!((item("library", "lib-previews").files, item("library", "lib-previews").bytes), (4, 12));
    // 工作空间：workspace.json 不计入任何项
    assert_eq!((item("workspace", "ws-thumbs").files, item("workspace", "ws-thumbs").bytes), (1, 4));
    assert_eq!((item("workspace", "ws-media").files, item("workspace", "ws-media").bytes), (2, 3));
    assert_eq!((item("workspace", "ws-probe").files, item("workspace", "ws-probe").bytes), (1, 2));

    // 未打开库 / 工作空间目录缺失：两组各项 0，条目不省略
    let ov2 = overview(None, None);
    assert_eq!(ov2.groups.len(), 2);
    assert!(ov2.groups.iter().flat_map(|g| &g.items).all(|i| i.bytes == 0 && i.files == 0));
    assert_eq!(ov2.groups[0].items.len(), 4);
    assert_eq!(ov2.groups[1].items.len(), 3);
    drop(dir);
  }

  /// 未知类别拒绝（清理安全闸：只删枚举内目录）
  #[test]
  fn parse_rejects_unknown_ids() {
    for id in ["", "lib", "ws", "../etc", "LIB-THUMBS", "lib-thumbs "] {
      assert!(CacheCategory::parse(id).is_err(), "id={id} 应被拒绝");
    }
    for c in CacheCategory::ALL {
      assert_eq!(CacheCategory::parse(c.id()).unwrap(), c, "枚举 id 往返");
    }
  }

  /// 红线：工作空间清理绝不动 workspace.json（常用固定 + junction 台账 = 用户数据）与
  /// cache_v2 版本标志；三类各清各的
  #[test]
  fn clear_ws_categories_preserve_workspace_json_and_flag() {
    let dir = tempfile::tempdir().unwrap();
    let ws = dir.path().join("workspace");
    let cache = ws.join(".anikit");
    fs::create_dir_all(cache.join("thumbs")).unwrap();
    fs::create_dir_all(cache.join("media")).unwrap();
    fs::create_dir_all(cache.join("peaks")).unwrap();
    fs::write(cache.join("thumbs/a.webp"), b"123").unwrap();
    fs::write(cache.join("media/m.mp4"), b"12345").unwrap();
    fs::write(cache.join("peaks/p.json"), b"1").unwrap();
    fs::write(cache.join("probe.json"), b"{}").unwrap();
    fs::write(cache.join("workspace.json"), br#"{"pinned":["a"]}"#).unwrap();
    fs::write(cache.join("cache_v2"), b"2").unwrap();

    let (freed, removed, failed) = clear_ws_category(CacheCategory::WsThumbs, &ws);
    assert_eq!((freed, removed, failed), (3, 1, 0));
    assert!(!cache.join("thumbs/a.webp").exists());
    assert!(cache.join("workspace.json").is_file(), "workspace.json 绝不可删");
    assert!(cache.join("cache_v2").is_file(), "版本标志保留");
    assert!(cache.join("media/m.mp4").is_file(), "他类不动");

    let (freed, _, _) = clear_ws_category(CacheCategory::WsMedia, &ws);
    assert_eq!(freed, 6);
    assert!(!cache.join("media/m.mp4").exists() && !cache.join("peaks/p.json").exists());
    assert!(cache.join("workspace.json").is_file());

    let (freed, _, _) = clear_ws_category(CacheCategory::WsProbe, &ws);
    assert_eq!(freed, 2);
    assert!(!cache.join("probe.json").exists());
    assert!(cache.join("workspace.json").is_file() && cache.join("cache_v2").is_file());
    // 幂等：再清一次全 0
    assert_eq!(clear_ws_category(CacheCategory::WsProbe, &ws), (0, 0, 0));
  }

  /// 红线（库侧）：只删 metadata 引用 + 规则推导的缓存文件——名字恰好像缩略图的
  /// 用户原始文件（Eagle 导出的「xx_thumbnail.png」入库后 origin 即此名）绝不可删
  #[test]
  fn clear_lib_thumbs_never_deletes_origin_lookalike() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    // 正常资产：真缩略图 + 假动图缓存
    let src = dir.path().join("图.png");
    make_png(&src, 640, 480);
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = ops::build_asset_with_id(&lib.assets_dir, &pf, "sha256:a", "AN-IMG", Source::Copy, None).unwrap();
    let anim = paths::anim_preview_file(&lib.assets_dir, "AN-IMG", &meta.origin_name);
    fs::write(&anim, b"anim").unwrap();
    // 诱饵资产：origin 文件名 =「诱饵_thumbnail.png」（后缀盲扫会误删的用户数据）
    let decoy_src = dir.path().join("诱饵_thumbnail.png");
    make_png(&decoy_src, 100, 80);
    let pf2 = PendingFile { path: decoy_src, chain: Vec::new() };
    let decoy = ops::build_asset_with_id(&lib.assets_dir, &pf2, "sha256:b", "AN-DECOY", Source::Copy, None).unwrap();
    assert_eq!(decoy.origin_name, "诱饵_thumbnail.png");

    let metas: Vec<AssetMeta> = vec![meta.clone(), decoy.clone()];
    let snap = LibSnapshot { root: lib.root.clone(), assets_dir: lib.assets_dir.clone(), metas };
    let thumb_file = lib.info_dir("AN-IMG").join(meta.thumbnail.as_ref().unwrap());
    assert!(thumb_file.is_file() && anim.is_file());

    let (freed, removed, failed) = clear_lib_thumbs(&snap);
    assert_eq!(failed, 0);
    assert!(removed >= 3, "AN-IMG 缩略图+动图 + 诱饵自身缩略图：{removed}");
    assert!(freed > 0);
    assert!(!thumb_file.exists(), "引用缩略图已删");
    assert!(!anim.exists(), "动图缓存已删");
    // 用户数据毫发无损：两资产的 origin、metadata、frames 目录不动
    assert!(paths::origin_file(&lib.assets_dir, "AN-IMG", "图.png").is_file());
    assert!(paths::origin_file(&lib.assets_dir, "AN-DECOY", "诱饵_thumbnail.png").is_file(), "诱饵 origin 绝不可删");
    assert!(lib.info_dir("AN-IMG").join("metadata.json").is_file());
  }

  /// 清理→重建全链：引用缩略图删后，rebuild_missing_thumbs 重新生成（同名同尺寸），
  /// 无引用资产不碰；幂等——第二趟无活可干
  #[test]
  fn rebuild_regenerates_missing_thumbs() {
    let (dir, mut lib) = make_lib_with_png();
    // 无内容缩略图资产（fla 假字节）对照组
    let src = dir.path().join("工程.fla");
    fs::write(&src, b"fake fla").unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let fla = ops::build_asset_with_id(&lib.assets_dir, &pf, "sha256:f", "AN-FLA", Source::Copy, None).unwrap();
    assert!(fla.thumbnail.is_none());
    lib.index.assets.insert("AN-FLA".into(), fla);
    let thumb_file = lib.info_dir("AN-IMG").join("图_thumbnail.webp");
    fs::remove_file(&thumb_file).unwrap();

    let lib_lock = Mutex::new(Some(lib));
    let (report, missing) = rebuild_missing_thumbs(&lib_lock);
    assert_eq!((report.rebuilt, report.fallback), (1, 0));
    assert_eq!(missing, 1, "仅 AN-IMG 缺失引用");
    assert!(thumb_file.is_file(), "缩略图已重建");
    let guard = lib_lock.lock().unwrap();
    let l = guard.as_ref().unwrap();
    assert_eq!(l.index.assets["AN-IMG"].thumbnail.as_deref(), Some("图_thumbnail.webp"));
    let disk: AssetMeta = read_json(&l.info_dir("AN-IMG").join("metadata.json")).unwrap();
    assert_eq!(disk.thumbnail.as_deref(), Some("图_thumbnail.webp"), "落盘一致");
    assert!(l.index.assets["AN-FLA"].thumbnail.is_none(), "无引用资产不碰");
    drop(guard);
    // 幂等：第二趟 missing = 0
    let (_r, missing2) = rebuild_missing_thumbs(&lib_lock);
    assert_eq!(missing2, 0);
  }

  /// 重建失败回退（源文件缺失）：thumbnail 引用清空 + 尺寸/主色清空 + 落盘一致，
  /// 视图回退格式图标而不是破图（与 origin_sync 手动刷新回退同口径）
  #[test]
  fn rebuild_fallback_when_origin_missing() {
    let (dir, lib) = make_lib_with_png();
    let origin = paths::origin_file(&lib.assets_dir, "AN-IMG", "图.png");
    fs::remove_file(&origin).unwrap();
    let thumb_file = lib.info_dir("AN-IMG").join("图_thumbnail.webp");
    fs::remove_file(&thumb_file).unwrap();

    let lib_lock = Mutex::new(Some(lib));
    let (report, missing) = rebuild_missing_thumbs(&lib_lock);
    assert_eq!(missing, 1);
    assert_eq!((report.rebuilt, report.fallback), (0, 1));
    let guard = lib_lock.lock().unwrap();
    let l = guard.as_ref().unwrap();
    let meta = &l.index.assets["AN-IMG"];
    assert_eq!((meta.thumbnail.is_none(), meta.thumb_width, meta.thumb_height), (true, None, None));
    assert!(meta.colors.is_empty());
    let disk: AssetMeta = read_json(&l.info_dir("AN-IMG").join("metadata.json")).unwrap();
    assert!(disk.thumbnail.is_none(), "落盘一致");
    drop(dir);
  }

  /// 序列帧资产：origin = frames/ 内封面帧（info 根无实体），重建须从 frames/ 取源
  #[test]
  fn rebuild_sequence_thumb_from_frames_dir() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    let mut frames = Vec::new();
    for i in 1..=3 {
      let f = dir.path().join(format!("跑_{i}.png"));
      make_png(&f, 320, 240);
      frames.push((f, i as u32));
    }
    let meta =
      sequence::build_sequence_asset(&lib.assets_dir, "跑", "png", 1, &frames, "sha256:s", "AN-SEQ", Source::Copy, None).unwrap();
    assert!(meta.thumbnail.is_some());
    let thumb_file = lib.info_dir("AN-SEQ").join(meta.thumbnail.as_ref().unwrap());
    assert!(thumb_file.is_file());
    fs::remove_file(&thumb_file).unwrap();
    let mut lib = lib;
    lib.index.assets.insert("AN-SEQ".into(), meta);

    let lib_lock = Mutex::new(Some(lib));
    let (report, missing) = rebuild_missing_thumbs(&lib_lock);
    assert_eq!(missing, 1);
    assert_eq!((report.rebuilt, report.fallback), (1, 0), "序列帧缩略图从 frames/ 重建，不回退");
    assert!(thumb_file.is_file());
  }

  /// 目录类清理（lib-previews / lib-media）：四会话目录 + media/peaks/waveforms 全清，
  /// 顶层目录保留，字节数如实
  #[test]
  fn clear_lib_dirs_empties_subdirs() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("库");
    for sub in ["previews", "frames", "note-thumbs", "tmp", "media", "peaks", "waveforms"] {
      fs::create_dir_all(root.join("cache").join(sub)).unwrap();
      fs::write(root.join("cache").join(sub).join("x.bin"), b"12").unwrap();
    }
    let (freed, removed, failed) = clear_lib_dirs(CacheCategory::LibPreviews, &root);
    assert_eq!((freed, removed, failed), (8, 4, 0));
    assert!(root.join("cache/media/x.bin").is_file(), "他类不动");
    let (freed, removed, _) = clear_lib_dirs(CacheCategory::LibMedia, &root);
    assert_eq!((freed, removed), (6, 3), "media+peaks+waveforms（含旧遗留 waveforms）");
  }

  /// 格式图标清理 + 即时重提：cache/thumbs 清空后按在库资产扩展名集重提，
  /// 返回的扩展名集供命令层同步内存 format_icons
  #[cfg(windows)]
  #[test]
  fn clear_icons_reensures_live_exts() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    let src = dir.path().join("工程.fla");
    fs::write(&src, b"fake fla bytes").unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let fla = ops::build_asset_with_id(&lib.assets_dir, &pf, "sha256:f", "AN-FLA", Source::Copy, None).unwrap();
    assert!(fla.thumbnail.is_none());
    // 预置图标缓存 + 一个无关格式的陈旧图标（重提不覆盖 = 清理瘦身见效）
    assert!(thumbnail::ensure_format_icon(&lib.root, &lib.info_dir("AN-FLA").join("工程.fla"), "fla"));
    fs::write(lib.root.join("cache/thumbs/zzz.webp"), b"stale").unwrap();

    let metas = vec![fla];
    let (_, _, _, ready) = clear_lib_icons(&lib.root, &lib.assets_dir, &metas);
    assert!(ready.contains("fla"), "在库扩展名重提");
    assert!(lib.root.join("cache/thumbs/fla.webp").is_file());
    assert!(!lib.root.join("cache/thumbs/zzz.webp").exists(), "无引用陈旧图标清掉不复活");
  }
}
