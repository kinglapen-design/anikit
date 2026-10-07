//! 工作空间搜索编排（PRD 02「搜索」双层架构，2026-10-03 落地；PRD 03：ws 分词搜索仅命中文件名与路径）：
//!
//! **Everything 层**（探测到运行中实例，手写 IPC 见 adapters::everything）：
//! - scope rel → `path:"<abs>" file: 关键字…` 查询——`file:` = 仅文件（目录由侧栏树导航，
//!   搜索网格只渲染文件卡）；**搜索串原样透传、绝不整体加引号**（引号 = 短语字面搜索，
//!   修饰符全失效——tools/everything-ipc-probe.py 探针实录）
//! - junction 链接目录 = Everything 按真实目标建档（门路径下查不到）：每条落入范围的登记链接
//!   对其目标补一条查询，命中按「目标前缀 → 链接 rel」映射回词法 rel（与方向 b 共享 links 台账）
//! - 命中映射 = 词法前缀剥（大小写不敏感 ASCII 比较 + 精确回退）；`.anikit` 剔除；限流 MAX_HITS
//!
//! **回退爬盘层**（无 Everything）：Enter 触发的显式慢搜，**不接 `~` 全局**（2026-10-03 用户定：
//! 全局慢搜开销大无意义，前端通知栏指路下载 Everything）；dir_stats 同款栈式遍历（穿透 junction
//! + canonical 防回环 + .anikit 跳过）+ rayon 按批并行读目录 + 取消令牌 + 流式批次回调。

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;
use serde::Serialize;

use crate::adapters::{everything, shell_path};
use crate::services::workspace::{self, LinkEntry, WsEntry};

/// 单次搜索命中上限（防巨量结果灌爆前端；Everything 全量计数与聚合属数量标记轮，不在此处）
pub const MAX_HITS: u32 = 1000;

/// 爬盘进度事件名（负载 = CrawlEvent；命令层流式推送）
pub const CRAWL_EVENT: &str = "ws-search-crawl";

/// 爬盘进度 / 命中批次事件负载：done=true 收尾（cancelled 区分取消与自然结束）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlEvent {
  pub task_id: u64,
  pub scanned: u64,
  pub hits: Vec<WsEntry>,
  pub done: bool,
  pub cancelled: bool,
}

/// 搜索出口：命中集 / Everything 不可用（前端据此落回退层引导）
#[derive(Debug)]
pub enum WsSearchOutcome {
  Hits(Vec<WsEntry>),
  Unavailable,
}

/// 剥 `~` 全局前缀与 `//` 分词前缀（可交叉、顺序不限；与库 search.parse_query / 前端
/// keywordsOf 同规则）→（关键字原文集，全局标记）。关键字不转小写——Everything 不区分
/// 大小写，回退层匹配时自行小写化
pub fn parse_query(query: &str) -> (Vec<String>, bool) {
  let mut q = query.trim();
  let mut global = false;
  loop {
    if let Some(rest) = q.strip_prefix("//") {
      q = rest.trim_start();
    } else if let Some(rest) = q.strip_prefix('~') {
      global = true;
      q = rest.trim_start();
    } else {
      break;
    }
  }
  (q.split_whitespace().map(|k| k.to_string()).collect(), global)
}

// ---------- Everything 层 ----------

/// Everything 搜索：root = workspace 根；view_rel = 当前视口目录（`~` 全局时调用方已换算为 ""）。
/// 目录缺失 / 越界 → Err（同 list_dir 口径）；Everything 不可用 → Ok(Unavailable)
pub fn search(root: &Path, scope_rel: &str, keywords: &[String]) -> Result<WsSearchOutcome, String> {
  let links = workspace::read_links(root);
  let legs = scope_legs(root, scope_rel, &links)?;
  let kw = keywords.join(" ");
  let mut out = Vec::new();
  for (i, leg) in legs.iter().enumerate() {
    match query_abs(&leg.anchor, &kw) {
      Ok(hits) => out.extend(map_hits(&hits, leg)),
      Err(WsSearchOutcome::Unavailable) if i == 0 => return Ok(WsSearchOutcome::Unavailable), // 主腿不可用 = 整体降级
      Err(_) => log::warn!("链接 {:?} 目标查询失败（跳过该链接）", legs.get(i).map(|l| &l.base)),
    }
    if out.len() >= MAX_HITS as usize {
      break;
    }
  }
  // rel 去重（链接目标恰落在 root 内等重叠场景）+ 限流
  let mut seen = HashSet::new();
  out.retain(|e| seen.insert(e.rel_path.clone()));
  // 剔除索引滞后幽灵命中（Everything 对文件夹索引 / USN 盲区有滞后——用户刚搬走 / 删除的文件
  // 可能还在索引里；不剔除则预览 / 缩略图 / 打开目标文件夹全落空并触发幽灵回退连环 toast。
  // 一词法拼接 + is_file 单 syscall，千条毫秒级；链接门路径跟随穿透正常工作）
  out.retain(|e| root.join(e.rel_path.replace('/', "\\")).is_file());
  out.truncate(MAX_HITS as usize);
  for e in &mut out {
    fill_cached_thumb(root, e);
  }
  Ok(WsSearchOutcome::Hits(out))
}

/// 查询腿：一条 Everything 查询的锚点与命中映射前缀。
/// anchor = 实际查询的绝对路径（Everything 只认真实路径；链接子树 = 目标侧路径）；
/// base = 命中映射回的词法 rel 前缀（**门词法**——scope 在链接子树时 base 仍是 scope_rel 门路径）
struct QueryLeg {
  anchor: PathBuf,
  base: String,
}

/// 范围 → 查询腿集：主腿（scope 本身）+ 范围内每条登记链接的补查腿。
/// **rel 恒为工作空间根相对词法路径**（2026-10-03 修复：旧版命中按 scope 锚点剥前缀，
/// 子目录视口搜索时 rel 少了视口前缀——预览 / 缩略图 / 跳转全落空）；
/// 断链门不补查（门不可达内容本就不该出现，一次 stat 廉价）
fn scope_legs(root: &Path, scope_rel: &str, links: &[LinkEntry]) -> Result<Vec<QueryLeg>, String> {
  let abs = workspace::resolve_under(root, scope_rel)?;
  // scope 落在链接子树：主腿锚 = 链接目标 + 子路径（Everything 按真实目标建档，门路径下查不到）
  let main = match links.iter().find(|l| !scope_rel.is_empty() && (scope_rel == l.rel || scope_rel.starts_with(&format!("{}/", l.rel)))) {
    Some(link) => {
      let sub = scope_rel.strip_prefix(&link.rel).unwrap_or("").trim_start_matches('/');
      let target = shell_path::to_shell_string(Path::new(&link.target));
      QueryLeg { anchor: if sub.is_empty() { PathBuf::from(&target) } else { Path::new(&target).join(sub.replace('/', "\\")) }, base: scope_rel.to_string() }
    }
    None => QueryLeg { anchor: abs, base: scope_rel.to_string() },
  };
  let mut legs = vec![main];
  for link in links {
    let covered_by_main = !scope_rel.is_empty() && (scope_rel == link.rel || scope_rel.starts_with(&format!("{}/", link.rel)));
    let in_scope = scope_rel.is_empty() || link.rel.starts_with(&format!("{scope_rel}/"));
    if covered_by_main || !in_scope {
      continue;
    }
    let target = shell_path::to_shell_string(Path::new(&link.target));
    if !Path::new(&target).is_dir() {
      continue;
    }
    legs.push(QueryLeg { anchor: PathBuf::from(target), base: link.rel.clone() });
  }
  Ok(legs)
}

/// 单条 `path:"<abs>" file: 关键字` 查询；**abs 先过 to_shell_string**——resolve_under 对存在
/// 路径返回 canonicalize 形态（`\\?\` 逐字前缀），Everything 索引是plain形态，不去壳必零命中（踩坑 078 同源）
fn query_abs(abs: &Path, kw: &str) -> Result<Vec<everything::Hit>, WsSearchOutcome> {
  let abs = shell_path::to_shell_string(abs);
  let query = if kw.is_empty() { format!("path:\"{abs}\" file:") } else { format!("path:\"{abs}\" file: {kw}") };
  match everything::query(&query, MAX_HITS) {
    Ok(hits) => Ok(hits),
    Err(e) => {
      log::warn!("Everything 查询失败（降级不可用态）：{e:?}");
      Err(WsSearchOutcome::Unavailable)
    }
  }
}

/// 命中映射：剥查询腿锚点前缀（词法，ASCII 大小写不敏感 + 精确回退）→ 拼 leg.base 词法前缀；
/// .anikit 剔除（仅根级可能出现）
fn map_hits(hits: &[everything::Hit], leg: &QueryLeg) -> Vec<WsEntry> {
  let prefix = shell_path::to_shell_string(&leg.anchor);
  hits
    .iter()
    .filter_map(|h| {
      let sub = strip_prefix_ci(&h.path, &prefix)?;
      if sub.is_empty() {
        return None; // 锚自身（file: 查询不会出现，防御）
      }
      let rel = if leg.base.is_empty() { sub.replace('\\', "/") } else { format!("{}/{}", leg.base, sub.replace('\\', "/")) };
      if rel == workspace::CACHE_DIR || rel.starts_with(&format!("{}/", workspace::CACHE_DIR)) {
        return None;
      }
      Some(hit_to_entry(rel, h))
    })
    .collect()
}

/// 词法前缀剥（路径分隔符归一）：返回剥掉前缀后的子路径（无前缀 / 前缀不对齐目录边界 = None）
fn strip_prefix_ci<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
  let head = path.get(..prefix.len())?;
  if !head.eq_ignore_ascii_case(prefix) && head != prefix {
    return None;
  }
  let rest = &path[prefix.len()..];
  if rest.is_empty() {
    return Some(""); // 恰为锚自身（目录条目；调用方按需丢弃）
  }
  rest.strip_prefix(['\\', '/']).filter(|s| !s.is_empty())
}

/// 搜索命中条目的缓存缩略图回填（list_dir entry() 同口径 ①②④）：①内容缩略图缓存新鲜 = asset
///（w/h 读缓存文件头回填，视频等非图片恢复真实比例）；②失败记忆命中 → 按扩展名格式图标已在
/// 缓存才给（exe 文件级图标与未缓存图标不现场提取——逐命中 shell 提取太贵，交前端可见卡上报
/// 走缩略图队列事件落回）；③可解码但未生成 = 不动（null → 前端上报生成 → 事件回填）
fn fill_cached_thumb(root: &Path, e: &mut WsEntry) {
  if e.is_dir {
    return;
  }
  if let Some(p) = workspace::fresh_thumb_path(root, &e.rel_path, e.mtime_ms, e.size) {
    if e.w.is_none() {
      if let Ok((tw, th)) = image::image_dimensions(std::path::Path::new(&p)) {
        e.w = Some(tw);
        e.h = Some(th);
      }
    }
    e.thumb_path = Some(p);
    e.thumb_kind = Some(crate::services::views::ThumbKind::Asset);
    return;
  }
  if workspace::is_thumb_failed(root, &e.rel_path, e.mtime_ms, e.size) && !e.ext.is_empty() {
    let icon = crate::services::thumbnail::format_icon_path_in(&workspace::thumbs_dir_of(root), &e.ext);
    if icon.is_file() {
      e.thumb_path = Some(icon.to_string_lossy().into_owned());
      e.thumb_kind = Some(crate::services::views::ThumbKind::FormatIcon);
    }
  }
}

/// Everything 命中 → 磁盘实况条目（缩略图 / 宽高缓存在 search() 统一回填——纯映射保持无 IO 可测）
fn hit_to_entry(rel: String, hit: &everything::Hit) -> WsEntry {
  let name = rel.rsplit('/').next().unwrap_or(rel.as_str()).to_string();
  let ext = if hit.is_dir { String::new() } else { name.rsplit_once('.').map(|(_, e)| e.to_lowercase()).unwrap_or_default() };
  WsEntry {
    name,
    rel_path: rel,
    is_dir: hit.is_dir,
    has_sub_dir: false,
    is_link: false,
    link_ok: false,
    size: hit.size.unwrap_or(0),
    mtime_ms: hit.mtime_ms.unwrap_or(0),
    ctime_ms: 0,
    w: None,
    h: None,
    ext,
    thumb_path: None,
    thumb_kind: None,
  }
}

// ---------- 回退爬盘层 ----------

/// Enter 触发的慢速遍历搜索（仅当前目录子树；`~` 全局由命令层拒收——2026-10-03 用户定案）。
/// 流式回调：on_batch(累计已扫, 本批命中)；收尾（done / cancelled）由命令层统一标注。
/// 取消 = cancel 置位。遍历骨架 = dir_stats 同款（穿透 junction + canonical 已访问集防回环
/// + .anikit 跳过）+ rayon 按批（16 目录 / 批）并行 read_dir；读取失败子树静默跳过
pub fn crawl(root: &Path, scope_rel: &str, keywords: &[String], cancel: &AtomicBool, mut on_batch: impl FnMut(u64, Vec<WsEntry>)) -> Result<u64, String> {
  let abs = workspace::resolve_under(root, scope_rel)?;
  if !abs.is_dir() {
    return Err("目录已不存在或已被移动".to_string());
  }
  let kws: Vec<String> = keywords.iter().map(|k| k.to_lowercase()).collect();
  let mut seen: HashSet<PathBuf> = HashSet::new();
  if let Ok(c) = abs.canonicalize() {
    seen.insert(c); // 根自身入集：junction 指回根的环在源头掐断
  }
  // 栈持（绝对路径, 词法 rel）对：resolve_under 对存在目录回 canonical 形态（\\?\ 前缀），
  // 子项路径与其同源无法对原始 root 剥前缀；rel 全程词法递推（junction 门子树同构）零 strip_prefix
  let mut stack = vec![(abs, scope_rel.to_string())];
  let mut scanned = 0u64;
  let mut last_emit = 0u64;
  while !stack.is_empty() {
    if cancel.load(Ordering::Relaxed) {
      return Ok(scanned);
    }
    let batch: Vec<(PathBuf, String)> = stack.drain(..stack.len().min(16)).collect();
    let outs: Vec<DirScan> = batch.par_iter().map(|(d, rel)| scan_dir(d, rel, &kws)).collect();
    let mut hits = Vec::new();
    for out in outs {
      scanned += out.scanned;
      hits.extend(out.hits);
      for (sub, sub_rel) in out.subdirs {
        // junction 防回环（dir_stats 同口径）：canonicalize 已访问集；断链门跳过子树
        if let Ok(c) = sub.canonicalize() {
          if seen.insert(c) {
            stack.push((sub, sub_rel));
          }
        }
      }
    }
    if !hits.is_empty() || scanned - last_emit >= 512 {
      last_emit = scanned;
      let mut hits = std::mem::take(&mut hits);
      for e in &mut hits {
        fill_cached_thumb(root, e);
      }
      on_batch(scanned, hits);
    }
  }
  Ok(scanned)
}

/// 单目录扫描产出（crawl 批并行的单元）
struct DirScan {
  scanned: u64,
  subdirs: Vec<(PathBuf, String)>,
  hits: Vec<WsEntry>,
}

/// 读一层目录：统计条目数、收集子目录（含 junction 门，词法 rel 递推）、按关键字命中文件
///（名或 rel 子串 AND——PRD 03 ws 口径：仅文件名与路径）
fn scan_dir(dir: &Path, dir_rel: &str, kws: &[String]) -> DirScan {
  let mut out = DirScan { scanned: 0, subdirs: Vec::new(), hits: Vec::new() };
  let Ok(rd) = std::fs::read_dir(dir) else { return out };
  for e in rd.flatten() {
    let name = e.file_name().to_string_lossy().into_owned();
    if name == workspace::CACHE_DIR {
      continue;
    }
    out.scanned += 1;
    let path = e.path();
    let rel = if dir_rel.is_empty() { name.clone() } else { format!("{dir_rel}/{name}") };
    let Ok(ft) = e.file_type() else { continue };
    if ft.is_dir() || (ft.is_symlink() && crate::adapters::junction::is_reparse_dir(&path)) {
      out.subdirs.push((path, rel));
      continue;
    }
    if kws.is_empty() {
      continue;
    }
    let haystack = format!("{} {}", name.to_lowercase(), rel.to_lowercase());
    if kws.iter().all(|k| haystack.contains(k.as_str())) {
      out.hits.push(crawl_entry(&rel, &name, &e));
    }
  }
  out
}

/// 爬盘命中 → 磁盘实况条目（metadata 现读：大小 / mtime；缓存缩略图由 crawl() 发射前统一回填
/// ——fresh 跳过入队零事件的坑（新鲜缓存静默跳过 = 无事件回填，搜索集非目录缓存，必须就地给），
/// 未生成的仍走前端可见卡上报管线）
fn crawl_entry(rel: &str, name: &str, e: &std::fs::DirEntry) -> WsEntry {
  let meta = e.metadata().ok();
  let ext = name.rsplit_once('.').map(|(_, x)| x.to_lowercase()).unwrap_or_default();
  WsEntry {
    name: name.to_string(),
    rel_path: rel.to_string(),
    is_dir: false,
    has_sub_dir: false,
    is_link: false,
    link_ok: false,
    size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
    mtime_ms: meta
      .and_then(|m| m.modified().ok())
      .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
      .map(|d| d.as_millis() as i64)
      .unwrap_or(0),
    ctime_ms: 0,
    w: None,
    h: None,
    ext,
    thumb_path: None,
    thumb_kind: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn make_ws() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("workspace");
    std::fs::create_dir_all(&root).unwrap();
    (dir, root)
  }

  #[test]
  fn parse_query_strips_prefixes() {
    assert_eq!(parse_query("~ // 分镜 02"), (vec!["分镜".to_string(), "02".to_string()], true));
    assert_eq!(parse_query("  海报 "), (vec!["海报".to_string()], false));
    assert_eq!(parse_query("~"), (Vec::<String>::new(), true));
  }

  #[test]
  fn strip_prefix_ci_ascii_case() {
    assert_eq!(strip_prefix_ci("E:\\WS\\a\\b.png", "e:\\ws"), Some("a\\b.png"));
    assert_eq!(strip_prefix_ci("E:\\ws2\\b.png", "E:\\ws"), None); // 前缀须对齐目录边界
    assert_eq!(strip_prefix_ci("E:\\ws", "E:\\ws"), Some(""));
    assert_eq!(strip_prefix_ci("D:\\ws\\a", "E:\\ws"), None);
  }

  #[test]
  fn map_hits_excludes_anikit_and_maps_rel() {
    let leg = QueryLeg { anchor: PathBuf::from("E:\\ws"), base: String::new() };
    let hits = vec![
      everything::Hit { path: "E:\\ws\\分镜\\01.png".into(), is_dir: false, size: Some(1), mtime_ms: Some(100) },
      everything::Hit { path: "E:\\ws\\.anikit\\cache.png".into(), is_dir: false, size: Some(1), mtime_ms: None },
      everything::Hit { path: "D:\\elsewhere\\x.png".into(), is_dir: false, size: None, mtime_ms: None },
    ];
    let out = map_hits(&hits, &leg);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].rel_path, "分镜/01.png");
    assert_eq!(out[0].name, "01.png");
    assert_eq!(out[0].ext, "png");
  }

  /// 回归（2026-10-03 实机三报根因）：子目录视口搜索的命中 rel 必须是**工作空间根相对**——
  /// 旧版按 scope 锚点剥前缀丢了视口段（预览 / 缩略图 / 打开目标文件夹全落空）
  #[test]
  fn map_hits_scoped_stays_root_relative() {
    let leg = QueryLeg { anchor: PathBuf::from("E:\\ws\\分镜"), base: "分镜".to_string() };
    let hits = vec![everything::Hit { path: "E:\\ws\\分镜\\子\\01.png".into(), is_dir: false, size: Some(1), mtime_ms: None }];
    let out = map_hits(&hits, &leg);
    assert_eq!(out[0].rel_path, "分镜/子/01.png");
  }

  #[test]
  fn map_hits_link_prefix_back_to_door() {
    let leg = QueryLeg { anchor: PathBuf::from("D:\\外部库"), base: "素材库".to_string() };
    let hits = vec![everything::Hit { path: "D:\\外部库\\镜头\\a.fla".into(), is_dir: false, size: Some(2), mtime_ms: None }];
    let out = map_hits(&hits, &leg);
    assert_eq!(out[0].rel_path, "素材库/镜头/a.fla");
  }

  #[test]
  fn scope_legs_inside_link_uses_target_with_door_base() {
    let (_d, root) = make_ws();
    let links = vec![LinkEntry { rel: "门".to_string(), target: "D:\\target".to_string() }];
    let legs = scope_legs(&root, "门/子层", &links).unwrap();
    assert_eq!(legs[0].anchor, Path::new("D:\\target").join("子层"));
    assert_eq!(legs[0].base, "门/子层"); // 命中映射回门词法（不丢 scope 深度）
    assert_eq!(legs.len(), 1); // 主腿已覆盖，不再补查
  }

  #[test]
  fn crawl_matches_name_and_path_and_skips_cache() {
    let (_d, root) = make_ws();
    std::fs::create_dir_all(root.join("分镜/01")).unwrap();
    std::fs::write(root.join("分镜/01/镜头a.png"), b"x").unwrap();
    std::fs::write(root.join("分镜/02.png"), b"x").unwrap();
    std::fs::write(root.join("无关.txt"), b"x").unwrap();
    std::fs::create_dir_all(root.join(".anikit")).unwrap();
    std::fs::write(root.join(".anikit/分镜缓存.png"), b"x").unwrap();
    let cancel = AtomicBool::new(false);
    let mut all = Vec::new();
    let scanned = crawl(&root, "", &["分镜".to_string()], &cancel, |_n, hits| all.extend(hits)).unwrap();
    assert!(scanned >= 5);
    all.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    assert_eq!(all.iter().map(|e| e.rel_path.as_str()).collect::<Vec<_>>(), ["分镜/01/镜头a.png", "分镜/02.png"]); // 路径命中 + 名命中
  }

  #[test]
  fn crawl_scope_subdir_and_cancel() {
    let (_d, root) = make_ws();
    std::fs::create_dir_all(root.join("a/b")).unwrap();
    std::fs::write(root.join("a/b/x.png"), b"x").unwrap();
    std::fs::write(root.join("a/y.png"), b"x").unwrap();
    let cancel = AtomicBool::new(false);
    let mut all = Vec::new();
    crawl(&root, "a/b", &["x".to_string()], &cancel, |_n, hits| all.extend(hits)).unwrap();
    assert_eq!(all.len(), 1);
    assert!(all[0].rel_path.ends_with("x.png"));
    // 预置取消：立即收尾零命中
    let cancelled = AtomicBool::new(true);
    let mut n = 0;
    crawl(&root, "", &["x".to_string()], &cancelled, |_, _| n += 1).unwrap();
    assert_eq!(n, 0);
  }

  #[test]
  fn crawl_missing_dir_errs() {
    let (_d, root) = make_ws();
    let cancel = AtomicBool::new(false);
    assert!(crawl(&root, "不存在", &["x".to_string()], &cancel, |_, _| {}).is_err());
  }

  /// 真机全链路（需本机 Everything + 真实工作空间 E:\anikit\workspace 存在）：
  /// cargo test -p app ws_search::tests::live_real_workspace -- --ignored --nocapture
  /// 打印命中的 rel / size / mtime / thumb 回填实况 + 父目录 list_dir 可用性 + 音视频判定
  #[test]
  #[ignore = "需本机 Everything + 真实工作空间"]
  fn live_real_workspace() {
    let root = Path::new(r"E:\anikit\workspace");
    if !root.is_dir() {
      eprintln!("跳过：真实工作空间不存在");
      return;
    }
    let WsSearchOutcome::Hits(entries) = search(root, "", &["224".to_string()]).expect("搜索应可用") else {
      panic!("Everything 不可用");
    };
    eprintln!("命中 {} 条", entries.len());
    for e in &entries {
      eprintln!("  rel={} size={} mtime={} thumb={:?} kind={:?}", e.rel_path, e.size, e.mtime_ms, e.thumb_path.is_some(), e.thumb_kind);
      // ③ 导航目标可用性：父目录 list_dir 必须能列（幽灵回退链排查）
      let parent = e.rel_path.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
      let listed = workspace::list_dir(root, parent);
      eprintln!("    父目录 [{parent}] list_dir = {:?}", listed.as_ref().map(|v| v.len()));
      // ① 音视频按路径预览判定链（media_source_by_path 同口径）
      let abs = workspace::resolve_under(root, &e.rel_path).unwrap();
      eprintln!("    is_file={} is_under={} is_audio={} is_video={}",
        abs.is_file(),
        workspace::is_under(root, &abs),
        crate::services::media::is_audio(&abs),
        crate::services::media::is_video(&abs));
    }
    assert!(!entries.is_empty(), "真实工作空间搜 224 应有命中");
  }

  /// 真机定点（需本机 Everything + 真实工作空间）：对 "224.wav" 与截图 png 逐条查
  /// 判定链 + 缩略图缓存命中实况（轻量版：不跑 list_dir——大目录 probe.json 开销被上轮实测拖慢）
  #[test]
  #[ignore = "需本机 Everything + 真实工作空间"]
  fn live_real_workspace_focused() {
    let root = Path::new(r"E:\anikit\workspace");
    if !root.is_dir() {
      eprintln!("跳过：真实工作空间不存在");
      return;
    }
    for kw in ["224.wav", "定妆照"] {
      // 定点：带 scope 搜（实机三报的场景 = 子目录视口搜索；根 scope 掩盖过 rel 映射 bug）
      let WsSearchOutcome::Hits(entries) = search(root, "02_创作/01_原创动画/01_计划中", &[kw.to_string()]).expect("搜索应可用") else {
        panic!("Everything 不可用");
      };
      eprintln!("== [{kw}] 命中 {} 条", entries.len());
      for e in entries.iter().take(8) {
        let abs = workspace::resolve_under(root, &e.rel_path).unwrap();
        let fresh = workspace::fresh_thumb_path(root, &e.rel_path, e.mtime_ms, e.size);
        eprintln!(
          "  rel={}\n    is_file={} is_under={} is_audio={} ext={} size={} mtime={}\n    回填 thumb={:?} kind={:?} fresh缓存={:?}",
          e.rel_path,
          abs.is_file(),
          workspace::is_under(root, &abs),
          crate::services::media::is_audio(&abs),
          e.ext,
          e.size,
          e.mtime_ms,
          e.thumb_path.is_some(),
          e.thumb_kind,
          fresh,
        );
      }
    }
  }
}
