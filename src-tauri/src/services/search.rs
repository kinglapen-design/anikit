//! 分词搜索（3.6 一轮，PRD 03_查找文件 01）：空格分词 AND；逐关键字子串包含，不区分大小写；
//! 链接字段匹配前先做 URL decode（抓取链接中编码的关键词可命中，如 q=%E6%A1%A3%E6%A1%88 输"档案"可命中）。
//!
//! 命中范围（MVP 落地子集）：
//! - 素材 = 资产名 / 各 Placement 显示名 / 扩展名 / 链接 / 所在路径链（区块 / 各级文件夹 / 档案名）
//!   / 属性 / 描述 / 提示词（绑定显示名 + 内容快照，2026-09-26 提示词轮纳入）
//! - 档案 = 档案名 / 徽标文本 / 属性 / 描述 / 链接 / 提示词（同资产口径）
//!
//! 资产侧关系 / 音色与标注名 / 注释随对应功能就位后纳入。
//!
//! 范围口径（2026-08-19 用户定）：
//! - 当前目录跟随递归显示开关（scope.recursive）；快速访问视图 = 当前视图即当前范围（回收站里搜回收站）；
//! - 全局（`~` 前缀；2026-08-31 tag 化 chips 废除后手敲仍生效）= 全库，回收站除外（回收站资产无活 Placement，天然排除）；
//! - 档案区仅文件夹视图出（快速访问同既有口径不出）；非全局时档案仅直属口径
//!   （同 stripArchivesFor，递归开关不影响档案区），全局时 = 全库命中档案。

use std::collections::HashSet;

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::core::library::Library;
use crate::core::models::{ArchiveData, AssetMeta, Folder, Placement, ROOT_FOLDER_ID};

/// 搜索范围（前端按视口组装）：viewKey + 递归开关 + 全局标记；query 含 ~ 前缀时强制全局
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchScope {
  pub view_key: String,
  pub recursive: bool,
  pub global: bool,
  /// 筛选条件（2026-10-09 筛选业务一轮，PRD 03_查找文件 02）：serde default 空 = 不施加，
  /// 旧调用零改动；条件全空等价旧行为
  #[serde(default)]
  pub filters: SearchFilters,
}

/// 命中 Placement 键（与前端选择 / 拖拽键同口径 `${assetId}|${folderId}`）
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHitKey {
  pub asset_id: String,
  pub folder_id: String,
}

/// 搜索命中结果：素材 Placement 键集 + 命中档案 folderId 集（排序输出保证确定性）
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHits {
  pub placements: Vec<SearchHitKey>,
  pub archives: Vec<String>,
}

/// 解析查询：剥 `//` 分词前缀与 `~` 全局前缀（可交叉、顺序不限；`/` 无语义——筛选走悬浮栏
/// 按钮组另轮（2026-08-31 定案），按普通文本搜），Unicode 空白分词（含全角空格），转小写。返回（关键字，强制全局）
fn parse_query(query: &str) -> (Vec<String>, bool) {
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
  let keywords = q.split_whitespace().map(|k| k.to_lowercase()).collect();
  (keywords, global)
}

/// URL percent 解码：`%XX` → 字节后 UTF-8 lossy，`+` → 空格（query 惯例）；非法序列原样保留
fn percent_decode(s: &str) -> String {
  if !s.contains('%') && !s.contains('+') {
    return s.to_string();
  }
  let bytes = s.as_bytes();
  let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
  let mut i = 0;
  while i < bytes.len() {
    if bytes[i] == b'%' && i + 2 < bytes.len() {
      if let Ok(hex) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
        out.push(hex);
        i += 3;
        continue;
      }
    }
    out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
    i += 1;
  }
  String::from_utf8_lossy(&out).into_owned()
}

/// Placement 所在路径文本（命中范围：区块 / 各级文件夹 / 档案名）：沿 parent 上溯收集文件夹名
///（档案 kind 节点名自然含入），顶级节点附所属区块名；root / 失效 id 无路径
fn path_text<'a>(lib: &'a Library, folder_id: &str) -> Option<String> {
  if folder_id == ROOT_FOLDER_ID {
    return None;
  }
  let mut names: Vec<&'a str> = Vec::new();
  let mut top_block: Option<&'a str> = None;
  let mut cur = folder_id;
  for _ in 0..64 {
    // 数据异常成环兜底
    let Some(f) = lib.index.folders.get(cur) else { break };
    names.push(f.name.as_str());
    match &f.parent {
      Some(p) => cur = p.as_str(),
      None => {
        top_block = f.block.as_deref();
        break;
      }
    }
  }
  if names.is_empty() {
    return None;
  }
  let mut text = names.join(" ");
  if let Some(block_id) = top_block {
    if let Some(b) = lib.folders.blocks.iter().find(|b| b.id == block_id) {
      text.push(' ');
      text.push_str(&b.name);
    }
  }
  Some(text)
}

/// 资产可搜文本集（小写化）：资产名 / 扩展名 / 链接（decode）/ 各 Placement 显示名 + 路径链 /
/// 属性 + 描述（标题 + 内容——资产侧属性·描述 6.1 就位纳入，PRD 03_查找文件 01 节）/
/// 提示词（绑定显示名 + 内容快照，2026-09-26 提示词轮）
fn asset_texts(lib: &Library, meta: &AssetMeta, placements: &[Placement]) -> Vec<String> {
  let mut texts = Vec::with_capacity(6 + placements.len() * 2);
  texts.push(meta.name.to_lowercase());
  texts.push(meta.ext.to_lowercase());
  for link in &meta.links {
    texts.push(percent_decode(link).to_lowercase());
  }
  for t in &meta.tags {
    texts.push(t.to_lowercase());
  }
  for d in &meta.descriptions {
    texts.push(d.title.to_lowercase());
    texts.push(d.content.to_lowercase());
  }
  if let Some(p) = &meta.prompt {
    texts.push(p.name.to_lowercase());
    texts.push(p.content.to_lowercase());
  }
  for p in placements {
    texts.push(p.name.to_lowercase());
    if let Some(path) = path_text(lib, &p.folder) {
      texts.push(path.to_lowercase());
    }
  }
  texts
}

/// 档案可搜文本集（小写化）：档案名 / 徽标文本 / 属性 / 描述（标题 + 内容）/ 链接（decode）/
/// 提示词（绑定显示名 + 内容快照，2026-09-26 提示词轮）
fn archive_texts(lib: &Library, folder: &Folder) -> Vec<String> {
  let mut texts = vec![folder.name.to_lowercase()];
  if let Some(a) = lib.archives.archives.iter().find(|a| a.folder_id == folder.id) {
    if let Some(b) = &a.badge {
      texts.push(b.text.to_lowercase());
    }
    for p in &a.props {
      texts.push(p.to_lowercase());
    }
    for d in &a.descriptions {
      texts.push(d.title.to_lowercase());
      texts.push(d.content.to_lowercase());
    }
    for l in &a.links {
      texts.push(percent_decode(l).to_lowercase());
    }
    if let Some(p) = &a.prompt {
      texts.push(p.name.to_lowercase());
      texts.push(p.content.to_lowercase());
    }
  }
  texts
}

/// 逐关键字子串包含 AND：每个关键字至少在文本集一项中出现
fn matches(texts: &[String], keywords: &[String]) -> bool {
  keywords.iter().all(|k| texts.iter().any(|t| t.contains(k.as_str())))
}

/// 递归收集文件夹后代 id（不含自身；folders 平表按 parent 扫）
fn collect_descendants<'a>(lib: &'a Library, id: &str, set: &mut HashSet<&'a str>) {
  let mut frontier = vec![id];
  while let Some(cur) = frontier.pop() {
    for f in &lib.folders.folders {
      if f.parent.as_deref() == Some(cur) && set.insert(f.id.as_str()) {
        frontier.push(f.id.as_str());
      }
    }
  }
}

// ---------- 筛选（2026-10-09 筛选业务一轮，PRD 03_查找文件 02 节）----------
//
// 口径：维度内多选 OR、跨维度 AND、与分词搜索 AND；无有效条件（has_active = false）= 不施加
// 筛选（等价旧行为，档案区照出）。筛选激活时档案区默认不出（筛选 = 找文件）；例外 = 扩展名维度
// 勾选虚拟类别 "archive"（2026-10-12 用户定）：档案按可判定维度参与（属性 / 所在文件夹 = parent /
// 添加日期 = created_at），资产专属维度（颜色 / 时长 / 比例 / 大小 / 链接状态 / 修改日期）激活即出局。
// 链接状态四态：origin / linked = 位置级（该卡是否原始放置）；thumb = 资产有伴生缩略图引用
// （thumb_asset_id，8.1 缩略图链接）；none = 资产仅 1 条活位置（无任何链接件）。
// 扩展名虚拟类别 "seq" = 序列帧（frames_dir_if_sequence 磁盘口径，与导出 / 重命名同一判定）；
// "archive" = 档案（资产侧永不命中——档案不是资产，判定在档案区段）。

/// 筛选条件集（内嵌 SearchScope；逐维度定义见各字段注释）
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchFilters {
  /// 主色近似（"#RRGGBB" 多选；资产任一主色与任一目标色 RGB 距离 ≤ 64 即中）
  pub colors: Vec<String>,
  /// 时长范围（unit = frames / seconds / minutes；frames 按素材自身 fps 换算，无 fps 不命中）
  pub duration: Option<DurationRange>,
  /// 属性词（精确等值，不区分大小写）
  pub tags: Vec<String>,
  /// 扩展名（小写；"seq" = 序列帧虚拟类别）
  pub exts: Vec<String>,
  /// 所在文件夹 id（自身或后代——树形复选圈整棵子树，2026-10-09 三轮定；"root" = 未分类）
  pub folder_ids: Vec<String>,
  /// 比例档位（landscape / square / portrait）或自定义 "w:h"
  pub ratios: Vec<String>,
  /// 链接状态（origin / linked / thumb / none）
  pub link_states: Vec<String>,
  /// 文件大小范围（字节）
  pub size: Option<ByteRange>,
  /// 添加日期（imported_at）
  pub imported: Option<DateCond>,
  /// 修改日期（modified_at = 库内数据变更时刻）
  pub modified: Option<DateCond>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DurationRange {
  pub unit: String,
  pub min: Option<f64>,
  pub max: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ByteRange {
  pub min: Option<u64>,
  pub max: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DateCond {
  /// 预设档位多选：today / yesterday / week（近 7 天）/ month（近 30 天）/ year（近 365 天）/ earlier（365 天前）
  pub buckets: Vec<String>,
  /// 自定义日期（年 / 月 / 日任意组合，填写项全等即中——只填年 = 该年全部）
  pub custom: Option<Ymd>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Ymd {
  pub year: Option<i32>,
  pub month: Option<u32>,
  pub day: Option<u32>,
}

impl DateCond {
  fn has_active(&self) -> bool {
    !self.buckets.is_empty()
      || self.custom.as_ref().is_some_and(|c| c.year.is_some() || c.month.is_some() || c.day.is_some())
  }
}

impl SearchFilters {
  /// 有任一有效筛选条件（「空壳」不算：范围双端空 / 日期档位与自定义全空）
  fn has_active(&self) -> bool {
    !self.colors.is_empty()
      || !self.tags.is_empty()
      || !self.exts.is_empty()
      || !self.folder_ids.is_empty()
      || !self.ratios.is_empty()
      || !self.link_states.is_empty()
      || self.duration.as_ref().is_some_and(|d| d.min.is_some() || d.max.is_some())
      || self.size.as_ref().is_some_and(|s| s.min.is_some() || s.max.is_some())
      || self.imported.as_ref().is_some_and(DateCond::has_active)
      || self.modified.as_ref().is_some_and(DateCond::has_active)
  }

  /// 资产级维度 AND（位置无关）：任一维度不过 = 整资产出局；link_states 的 thumb / none 成分
  /// 与 origin / linked 同维度 OR 混合，整体留位置级判定
  fn asset_pass(&self, lib: &Library, meta: &AssetMeta, today: NaiveDate) -> bool {
    if !self.colors.is_empty() && !colors_hit(&self.colors, &meta.colors) {
      return false;
    }
    if let Some(d) = &self.duration {
      if (d.min.is_some() || d.max.is_some()) && !duration_hit(d, meta) {
        return false;
      }
    }
    if !self.tags.is_empty() && !tags_hit(&self.tags, &meta.tags) {
      return false;
    }
    if !self.exts.is_empty() && !exts_hit(lib, &self.exts, meta) {
      return false;
    }
    if !self.ratios.is_empty() && !ratios_hit(&self.ratios, meta) {
      return false;
    }
    if let Some(s) = &self.size {
      if (s.min.is_some() || s.max.is_some()) && !size_hit(s, meta.size) {
        return false;
      }
    }
    if let Some(c) = &self.imported {
      if c.has_active() && !date_hit(c, &meta.imported_at, today) {
        return false;
      }
    }
    if let Some(c) = &self.modified {
      if c.has_active() && !date_hit(c, &meta.modified_at, today) {
        return false;
      }
    }
    true
  }

  /// 位置级维度 AND：所在文件夹（folder_filter = 预展开含后代集）+ 链接状态
  fn placement_pass(&self, meta: &AssetMeta, p: &Placement, alive: usize, folder_filter: Option<&HashSet<&str>>) -> bool {
    if let Some(set) = folder_filter {
      if !set.contains(p.folder.as_str()) {
        return false;
      }
    }
    let valid: Vec<&String> = self.link_states.iter().filter(|s| LINK_STATES.contains(&s.as_str())).collect();
    if !valid.is_empty() && !valid.iter().any(|s| link_state_hit(s, meta, p, alive)) {
      return false;
    }
    true
  }

  /// 档案级维度 AND（入场券 = exts 含 "archive" 虚拟类别，由调用处判）：可判定维度 = 属性 /
  /// 所在文件夹（parent，含后代集同资产位置口径）/ 添加日期（created_at）；资产专属维度
  ///（颜色 / 时长 / 比例 / 大小 / 链接状态 / 修改日期）激活 = 档案出局（跨维度 AND）
  fn archive_pass(&self, data: Option<&ArchiveData>, folder: &Folder, folder_filter: Option<&HashSet<&str>>, today: NaiveDate) -> bool {
    if !self.colors.is_empty() || !self.ratios.is_empty() || !self.link_states.is_empty() {
      return false;
    }
    if self.duration.as_ref().is_some_and(|d| d.min.is_some() || d.max.is_some()) {
      return false;
    }
    if self.size.as_ref().is_some_and(|s| s.min.is_some() || s.max.is_some()) {
      return false;
    }
    if self.modified.as_ref().is_some_and(DateCond::has_active) {
      return false;
    }
    if !self.tags.is_empty() {
      let props: &[String] = data.map_or(&[], |a| &a.props);
      if !tags_hit(&self.tags, props) {
        return false;
      }
    }
    if let Some(set) = folder_filter {
      if !folder.parent.as_deref().is_some_and(|p| set.contains(p)) {
        return false;
      }
    }
    if let Some(c) = &self.imported {
      if c.has_active() {
        let Some(a) = data else { return false };
        if !date_hit(c, &a.created_at, today) {
          return false;
        }
      }
    }
    true
  }
}

/// "#RRGGBB" 解析（大小写通吃，容忍缺 # 前缀）
fn parse_hex_color(s: &str) -> Option<(f64, f64, f64)> {
  let h = s.trim().trim_start_matches('#');
  if h.len() != 6 {
    return None;
  }
  let r = u8::from_str_radix(h.get(0..2)?, 16).ok()?;
  let g = u8::from_str_radix(h.get(2..4)?, 16).ok()?;
  let b = u8::from_str_radix(h.get(4..6)?, 16).ok()?;
  Some((r as f64, g as f64, b as f64))
}

/// 颜色近似容差（RGB 欧氏距离平方 64²）：提取色板几乎永不被精确等值命中，按同色系近似匹配
const COLOR_TOLERANCE_SQ: f64 = 64.0 * 64.0;

fn colors_hit(targets: &[String], palette: &[String]) -> bool {
  let targets: Vec<(f64, f64, f64)> = targets.iter().filter_map(|t| parse_hex_color(t)).collect();
  if targets.is_empty() {
    return true; // 目标全无法解析 = 维度不参与（防垃圾输入清空结果）
  }
  palette.iter().filter_map(|c| parse_hex_color(c)).any(|c| {
    targets.iter().any(|t| (t.0 - c.0).powi(2) + (t.1 - c.1).powi(2) + (t.2 - c.2).powi(2) <= COLOR_TOLERANCE_SQ)
  })
}

fn duration_hit(range: &DurationRange, meta: &AssetMeta) -> bool {
  let Some(seconds) = meta.duration else { return false };
  let value = match range.unit.as_str() {
    "frames" => match meta.fps {
      Some(fps) if fps > 0.0 => (seconds * fps).round(),
      _ => return false,
    },
    "minutes" => seconds / 60.0,
    _ => seconds, // seconds 与未识别单位一律按秒（前端只发三档）
  };
  range.min.map_or(true, |min| value >= min) && range.max.map_or(true, |max| value <= max)
}

fn tags_hit(want: &[String], have: &[String]) -> bool {
  want.iter().any(|w| have.iter().any(|h| h.eq_ignore_ascii_case(w)))
}

fn exts_hit(lib: &Library, want: &[String], meta: &AssetMeta) -> bool {
  want.iter().any(|w| {
    let w = w.trim_start_matches('.').to_lowercase();
    if w == "seq" {
      crate::services::sequence::frames_dir_if_sequence(&lib.assets_dir, &meta.id, meta).is_some()
    } else {
      meta.ext.eq_ignore_ascii_case(&w)
    }
  })
}

/// 方图容差（宽高比 ∈ 1/1.05..=1.05 判方）
const SQUARE_RATIO: f64 = 1.05;
/// 自定义比例容差（|w/h - a/b| 相对偏差 ≤ 1%）
const CUSTOM_RATIO_TOLERANCE: f64 = 0.01;

fn parse_ratio(s: &str) -> Option<(f64, f64)> {
  let (a, b) = s.split_once(':')?;
  let a = a.trim().parse::<f64>().ok()?;
  let b = b.trim().parse::<f64>().ok()?;
  (a > 0.0 && b > 0.0).then_some((a, b))
}

fn ratios_hit(want: &[String], meta: &AssetMeta) -> bool {
  let valid: Vec<&str> = want
    .iter()
    .map(String::as_str)
    .filter(|r| matches!(*r, "landscape" | "portrait" | "square") || parse_ratio(r).is_some())
    .collect();
  if valid.is_empty() {
    return true; // 全无法解析 = 维度不参与
  }
  let (Some(w), Some(h)) = (meta.width, meta.height) else { return false };
  if w == 0 || h == 0 {
    return false;
  }
  let ratio = w as f64 / h as f64;
  valid.iter().any(|r| match *r {
    "landscape" => ratio > SQUARE_RATIO,
    "portrait" => ratio < 1.0 / SQUARE_RATIO,
    "square" => (1.0 / SQUARE_RATIO..=SQUARE_RATIO).contains(&ratio),
    custom => {
      let (a, b) = parse_ratio(custom).unwrap();
      (ratio - a / b).abs() / (a / b) <= CUSTOM_RATIO_TOLERANCE
    }
  })
}

const LINK_STATES: [&str; 4] = ["origin", "linked", "thumb", "none"];

fn link_state_hit(state: &str, meta: &AssetMeta, p: &Placement, alive: usize) -> bool {
  match state {
    "origin" => p.origin,
    "linked" => !p.origin,
    "thumb" => meta.thumb_asset_id.is_some(),
    "none" => alive <= 1,
    _ => false,
  }
}

fn size_hit(range: &ByteRange, size: u64) -> bool {
  range.min.map_or(true, |min| size >= min) && range.max.map_or(true, |max| size <= max)
}

/// RFC3339（含时区偏移）→ 本地日历日
fn local_date_of(rfc3339: &str) -> Option<NaiveDate> {
  chrono::DateTime::parse_from_rfc3339(rfc3339)
    .ok()
    .map(|d| d.with_timezone(&chrono::Local).date_naive())
}

fn bucket_hit(bucket: &str, date: NaiveDate, today: NaiveDate) -> bool {
  let age = (today - date).num_days();
  match bucket {
    "today" => age == 0,
    "yesterday" => age == 1,
    "week" => (0..=6).contains(&age),
    "month" => (0..=29).contains(&age),
    "year" => (0..=364).contains(&age),
    "earlier" => age >= 365,
    _ => false,
  }
}

fn date_hit(cond: &DateCond, rfc3339: &str, today: NaiveDate) -> bool {
  let Some(date) = local_date_of(rfc3339) else { return false };
  let custom_hit = cond.custom.as_ref().is_some_and(|c| {
    (c.year.is_some() || c.month.is_some() || c.day.is_some())
      && c.year.map_or(true, |y| date.year() == y)
      && c.month.map_or(true, |m| date.month() == m)
      && c.day.map_or(true, |d| date.day() == d)
  });
  custom_hit || cond.buckets.iter().any(|b| bucket_hit(b, date, today))
}

/// 语义检索候选集：复用分词搜索的范围、回收站 / hidden 与筛选规则，不做关键词过滤。
pub fn search_candidates(lib: &Library, scope: &SearchScope) -> SearchHits {
  search_impl(lib, &[], false, scope)
}

pub fn search(lib: &Library, query: &str, scope: &SearchScope) -> SearchHits {
  let (keywords, force_global) = parse_query(query);
  if keywords.is_empty() && !scope.filters.has_active() {
    return SearchHits::default();
  }
  search_impl(lib, &keywords, force_global, scope)
}

fn search_impl(lib: &Library, keywords: &[String], force_global: bool, scope: &SearchScope) -> SearchHits {
  let filtering = scope.filters.has_active();
  let today = chrono::Local::now().date_naive();
  let global = force_global || scope.global;

  // 文件夹范围集（folder 视图且非全局）：recursive 含后代
  let folder_set: Option<HashSet<&str>> = if global {
    None
  } else {
    scope.view_key.strip_prefix("folder:").map(|id| {
      let mut set = HashSet::from([id]);
      if scope.recursive {
        collect_descendants(lib, id, &mut set);
      }
      set
    })
  };

  let mut hits = SearchHits::default();

  // 文件夹筛选预展开（树形复选 = 含后代：勾父文件夹圈整棵子树，2026-10-09 三轮定）
  let folder_filter: Option<HashSet<&str>> = if scope.filters.folder_ids.is_empty() {
    None
  } else {
    let mut set = HashSet::new();
    for id in &scope.filters.folder_ids {
      set.insert(id.as_str());
      collect_descendants(lib, id, &mut set);
    }
    Some(set)
  };

  // 回收站范围（非全局时）：快照 placements 补文本；命中键 folderId 恒 "recycle"（前端回收站视图键口径）；
  // 筛选同口径施加（资产级对 meta，位置级对快照 placements，「无链接」按快照条数）
  if !global && scope.view_key == "quick:recycle" {
    for item in &lib.recycle.items {
      let Some(meta) = lib.index.assets.get(&item.asset) else { continue };
      if filtering && !scope.filters.asset_pass(lib, meta, today) {
        continue;
      }
      if !keywords.is_empty() && !matches(&asset_texts(lib, meta, &item.placements), keywords) {
        continue;
      }
      if item.placements.iter().any(|p| scope.filters.placement_pass(meta, p, item.placements.len(), folder_filter.as_ref())) {
        hits.placements.push(SearchHitKey { asset_id: item.asset.clone(), folder_id: "recycle".into() });
      }
    }
    hits.placements.sort();
    return hits;
  }

  let recent_set: HashSet<&str> =
    if !global && scope.view_key == "quick:recent" {
      lib.recent.items.iter().map(|r| r.asset.as_str()).collect()
    } else {
      HashSet::new()
    };

  for meta in lib.index.assets.values() {
    if meta.hidden == Some(true) {
      continue; // hidden 伴生缩略图资产（8.1，PRD 02-15）：不是用户管理对象，全范围搜索不命中
    }
    let placements = lib.index.placements_of(&meta.id);
    if placements.is_empty() {
      continue; // 回收站资产（无活 Placement）不参与——全局 = 全库不含回收站（2026-08-19 用户定）
    }
    // 资产级范围预过滤
    if !global {
      match scope.view_key.as_str() {
        "quick:inbox" if meta.imported_from.as_deref() != Some("inbox") => continue,
        "quick:recent" if !recent_set.contains(meta.id.as_str()) => continue,
        _ => {}
      }
    }
    // 筛选资产级维度短路（先于文本组装，省 asset_texts 开销）
    if filtering && !scope.filters.asset_pass(lib, meta, today) {
      continue;
    }
    if !keywords.is_empty() && !matches(&asset_texts(lib, meta, placements), keywords) {
      continue;
    }
    for p in placements {
      let in_scope = if global {
        true
      } else if let Some(set) = &folder_set {
        set.contains(p.folder.as_str())
      } else {
        match scope.view_key.as_str() {
          "quick:uncategorized" => p.folder == ROOT_FOLDER_ID,
          _ => true, // quick:all / quick:inbox / quick:recent
        }
      };
      if in_scope && scope.filters.placement_pass(meta, p, placements.len(), folder_filter.as_ref()) {
        hits.placements.push(SearchHitKey { asset_id: meta.id.clone(), folder_id: p.folder.clone() });
      }
    }
  }

  // 档案命中：仅文件夹视图出档案区（快速访问同既有口径不出）；非全局 = 仅直属（同 stripArchivesFor
  // 口径，递归开关不影响档案区）；全局 = 全库命中档案。
  // 筛选激活时档案区默认不出（筛选 = 找文件，2026-10-09 定）；例外 = 扩展名维度含虚拟类别
  // "archive"（2026-10-12 用户定）——档案经 archive_pass 按可判定维度参与筛选
  let archive_category =
    scope.filters.exts.iter().any(|w| w.trim_start_matches('.').eq_ignore_ascii_case("archive"));
  if !filtering || archive_category {
    for f in &lib.folders.folders {
      if f.kind.as_deref() != Some("archive") {
        continue;
      }
      let in_scope = if global {
        true
      } else {
        match scope.view_key.strip_prefix("folder:") {
          Some(id) => f.parent.as_deref() == Some(id),
          None => false,
        }
      };
      if !in_scope {
        continue;
      }
      if filtering {
        let data = lib.archives.archives.iter().find(|a| a.folder_id == f.id);
        if !scope.filters.archive_pass(data, f, folder_filter.as_ref(), today) {
          continue;
        }
      }
      if matches(&archive_texts(lib, f), keywords) {
        hits.archives.push(f.id.clone());
      }
    }
  }

  hits.placements.sort();
  hits.archives.sort();
  hits
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library::{self, Library};
  use crate::core::models::{
    ArchiveData, Badge, Block, DescItem, Folder, RecycleItem, RecentItem,
  };

  struct Harness {
    _dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let mut lib = library::create_test(dir.path());
    // 补一个自定义区块（默认区块「目录」由 create_test / FoldersFile 默认注入）
    lib.folders.blocks.push(Block { id: "B2".into(), name: "参考区".into(), icon: None, order: 1 });
    Harness { _dir: dir, lib }
  }

  fn add_folder_ex(lib: &mut Library, id: &str, name: &str, parent: Option<&str>, block: Option<&str>, kind: Option<&str>) {
    let folder = Folder {
      id: id.into(),
      name: name.into(),
      parent: parent.map(|s| s.into()),
      block: block.map(|s| s.into()),
      icon: None,
      kind: kind.map(|s| s.into()),
      color: None,
      groups: None,
      order: 0,
    };
    lib.folders.folders.push(folder.clone());
    lib.index.folders.insert(id.into(), folder);
  }

  fn add_folder(lib: &mut Library, id: &str, name: &str) {
    add_folder_ex(lib, id, name, None, None, None);
  }

  fn add_asset_ex(
    lib: &mut Library,
    id: &str,
    name: &str,
    ext: &str,
    links: &[&str],
    imported_from: Option<&str>,
    folder: &str,
  ) {
    lib.index.assets.insert(id.into(), AssetMeta {
      version: 1,
      id: id.into(),
      name: name.into(),
      origin_name: format!("{name}.{ext}"),
      ext: ext.into(),
      size: 100,
      hash: format!("sha256:{id}"),
      width: None,
      height: None,
      thumb_width: None,
      thumb_height: None,
      hidden: None,
      thumb_asset_id: None,
      duration: None,
      bpm: None,
      rating: 0,
      links: links.iter().map(|s| s.to_string()).collect(),
      imported_at: "2026-08-01T10:00:00+08:00".into(),
      imported_from: imported_from.map(|s| s.into()),
      created_at: None,
      modified_at: "2026-08-01T10:00:00+08:00".into(),
      tags: Vec::new(),
      descriptions: Vec::new(),
      note: String::new(),
      note_count: None,
      thumbnail: None,
      colors: Vec::new(),
      fps: None,
      frame_count: None,
      thumb_frame: None,
      r#type: None,
      source: None,
      start: None,
      end: None,
      start_frame: None,
      end_frame: None,
      prompt: None,
    });
    add_placement(lib, id, &format!("{name}.{ext}"), folder, true);
  }

  fn add_asset(lib: &mut Library, id: &str, name: &str, folder: &str) {
    add_asset_ex(lib, id, name, "png", &[], None, folder);
  }

  fn add_placement(lib: &mut Library, asset: &str, name: &str, folder: &str, origin: bool) {
    lib.folders.items.push(Placement {
      asset: asset.into(),
      folder: folder.into(),
      name: name.into(),
      order: 0,
      origin,
    });
    lib.index.rebuild_placements(&lib.folders.items);
  }

  fn scope(view_key: &str) -> SearchScope {
    SearchScope { view_key: view_key.into(), recursive: false, global: false, filters: SearchFilters::default() }
  }

  fn scope_filtered(view_key: &str, filters: SearchFilters) -> SearchScope {
    SearchScope { view_key: view_key.into(), recursive: false, global: false, filters }
  }

  fn hit_keys(hits: &SearchHits) -> Vec<String> {
    hits.placements.iter().map(|k| format!("{}|{}", k.asset_id, k.folder_id)).collect()
  }

  #[test]
  fn and_keywords_substring_case_insensitive() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "人物-开心-走路", "root");
    add_asset(&mut h.lib, "AN-B", "人物-伤心-走路", "root");
    add_asset(&mut h.lib, "AN-C", "人物-开心-跑步", "root");

    let hits = search(&h.lib, "人物 走路", &scope("quick:all"));
    assert_eq!(hit_keys(&hits), ["AN-A|root", "AN-B|root"], "空格分词 AND");

    // 大小写不敏感（扩展名）
    let hits = search(&h.lib, "PNG", &scope("quick:all"));
    assert_eq!(hits.placements.len(), 3, "扩展名命中不区分大小写");

    // 单关键字子串
    let hits = search(&h.lib, "跑步", &scope("quick:all"));
    assert_eq!(hit_keys(&hits), ["AN-C|root"]);

    // 空查询 / 纯前缀 = 空命中
    assert!(search(&h.lib, "  ", &scope("quick:all")).placements.is_empty());
    assert!(search(&h.lib, "//", &scope("quick:all")).placements.is_empty());
  }

  #[test]
  fn placement_display_name_and_asset_name_both_searchable() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "文件夹");
    add_asset(&mut h.lib, "AN-A", "资产名甲", "root");
    add_placement(&mut h.lib, "AN-A", "链接显示名乙.png", "F01", false);

    // 资产名命中 → 范围内全部 Placement 键都收
    let hits = search(&h.lib, "资产名甲", &scope("quick:all"));
    assert_eq!(hit_keys(&hits), ["AN-A|F01", "AN-A|root"]);
    // 链接显示名命中
    let hits = search(&h.lib, "显示名乙", &scope("quick:all"));
    assert_eq!(hit_keys(&hits), ["AN-A|F01", "AN-A|root"]);
  }

  /// 资产侧属性 / 描述命中（6.1 就位纳入，PRD 03_查找文件 01「阶段六纳入」）
  #[test]
  fn asset_tags_and_descriptions_searchable() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "图甲", "root");
    add_asset(&mut h.lib, "AN-B", "图乙", "root");
    h.lib.index.assets.get_mut("AN-A").unwrap().tags = vec!["拔剑".into()];
    h.lib.index.assets.get_mut("AN-B").unwrap().descriptions =
      vec![crate::core::models::DescItem { title: "身份".into(), content: "沙雕动画的御用主角".into() }];

    assert_eq!(hit_keys(&search(&h.lib, "拔剑", &scope("quick:all"))), ["AN-A|root"], "属性命中");
    assert_eq!(hit_keys(&search(&h.lib, "御用主角", &scope("quick:all"))), ["AN-B|root"], "描述内容命中");
    assert_eq!(hit_keys(&search(&h.lib, "身份", &scope("quick:all"))), ["AN-B|root"], "描述标题命中");
  }

  /// 提示词命中（2026-09-26 提示词轮）：资产与档案侧的绑定显示名 + 内容快照纳入搜索
  #[test]
  fn prompt_name_and_content_searchable() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "图甲", "root");
    add_asset(&mut h.lib, "AN-B", "图乙", "root");
    add_folder(&mut h.lib, "F01", "项目");
    add_folder_ex(&mut h.lib, "FA", "档案丙", Some("F01"), None, Some("archive"));
    h.lib.index.assets.get_mut("AN-A").unwrap().prompt = Some(crate::core::models::PromptBinding {
      asset_id: Some("AN-DOC".into()),
      name: "雪牧提示词.md".into(),
      content: "masterpiece, 拔剑姿势".into(),
      updated_at: "2026-09-26T00:00:00+08:00".into(),
    });
    h.lib.index.assets.get_mut("AN-B").unwrap().prompt = Some(crate::core::models::PromptBinding {
      asset_id: None,
      name: "扫描结果".into(),
      content: "扫描出的符文咒文".into(),
      updated_at: "2026-09-26T00:00:00+08:00".into(),
    });
    h.lib.archives.archives.push(ArchiveData {
      folder_id: "FA".into(),
      badge: None,
      avatar: None,
      props: Vec::new(),
      descriptions: Vec::new(),
      links: Vec::new(),
      created_at: String::new(),
      prompt: Some(crate::core::models::PromptBinding {
        asset_id: Some("AN-DOC".into()),
        name: "设定集提示词".into(),
        content: "档案级世界观咒文".into(),
        updated_at: "2026-09-26T00:00:00+08:00".into(),
      }),
    });

    assert_eq!(hit_keys(&search(&h.lib, "拔剑姿势", &scope("quick:all"))), ["AN-A|root"], "已指定态内容快照命中");
    assert_eq!(hit_keys(&search(&h.lib, "雪牧提示词", &scope("quick:all"))), ["AN-A|root"], "绑定显示名命中");
    assert_eq!(hit_keys(&search(&h.lib, "符文咒文", &scope("quick:all"))), ["AN-B|root"], "已扫描态快照命中");
    let hits = search(&h.lib, "世界观咒文", &scope("folder:F01"));
    assert_eq!(hits.archives, ["FA"], "档案提示词内容命中（folder 视图直属口径）");
  }

  #[test]
  fn link_url_decoded_before_match() {
    let mut h = make_lib();
    add_asset_ex(&mut h.lib, "AN-A", "无关", "png", &["https://x.com/?q=%E6%A1%A3%E6%A1%88+走路"], None, "root");

    let hits = search(&h.lib, "档案", &scope("quick:all"));
    assert_eq!(hit_keys(&hits), ["AN-A|root"], "编码关键词 decode 后可命中");
    let hits = search(&h.lib, "走路", &scope("quick:all"));
    assert_eq!(hit_keys(&hits), ["AN-A|root"], "+ 按 query 惯例转空格后命中");
  }

  #[test]
  fn path_chain_names_searchable() {
    let mut h = make_lib();
    add_folder_ex(&mut h.lib, "FT", "顶级夹", None, Some("B2"), None);
    add_folder_ex(&mut h.lib, "FC", "角色设定", Some("FT"), None, None);
    add_asset(&mut h.lib, "AN-A", "无关联名", "FC");

    // 各级文件夹名
    assert_eq!(hit_keys(&search(&h.lib, "角色设定", &scope("quick:all"))), ["AN-A|FC"]);
    assert_eq!(hit_keys(&search(&h.lib, "顶级夹", &scope("quick:all"))), ["AN-A|FC"]);
    // 区块名
    assert_eq!(hit_keys(&search(&h.lib, "参考区", &scope("quick:all"))), ["AN-A|FC"]);
    // 默认区块「目录」同链路（root 资产无路径不命中）
    add_asset(&mut h.lib, "AN-B", "乙", "root");
    assert!(search(&h.lib, "目录", &scope("quick:all")).placements.is_empty());
  }

  #[test]
  fn archive_name_in_path_and_archive_hit_fields() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "项目");
    add_folder_ex(&mut h.lib, "FA", "分镜档案", Some("F01"), None, Some("archive"));
    add_asset(&mut h.lib, "AN-A", "无关联名", "FA");

    // 素材路径链含档案名
    assert_eq!(hit_keys(&search(&h.lib, "分镜档案", &scope("quick:all"))), ["AN-A|FA"]);
    // 档案名命中 → 档案区（folder 视图直属口径：FA 直属 F01）
    let hits = search(&h.lib, "分镜档案", &scope("folder:F01"));
    assert_eq!(hits.archives, ["FA"]);

    // 徽标 / 属性 / 描述 / 链接全字段
    h.lib.archives.archives.push(ArchiveData {
      folder_id: "FA".into(),
      badge: Some(Badge { text: "进行中".into(), color: "#fff".into() }),
      avatar: None,
      props: vec!["二维动画".into()],
      descriptions: vec![DescItem { title: "用途".into(), content: "片头分镜".into() }],
      links: vec!["https://ref.com/?q=%E5%8F%82%E8%80%83".into()],
      created_at: String::new(),
      prompt: None,
    });
    for kw in ["进行中", "二维动画", "用途", "片头分镜", "参考"] {
      let hits = search(&h.lib, kw, &scope("folder:F01"));
      assert_eq!(hits.archives, ["FA"], "档案字段命中：{kw}");
    }
  }

  #[test]
  fn folder_scope_follows_recursive_flag() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "父夹");
    add_folder_ex(&mut h.lib, "F02", "子夹", Some("F01"), None, None);
    add_asset(&mut h.lib, "AN-A", "走路父", "F01");
    add_asset(&mut h.lib, "AN-B", "走路子", "F02");
    add_asset(&mut h.lib, "AN-C", "走路外", "root");

    let mut s = scope("folder:F01");
    assert_eq!(hit_keys(&search(&h.lib, "走路", &s)), ["AN-A|F01"], "非递归仅直属");
    s.recursive = true;
    assert_eq!(hit_keys(&search(&h.lib, "走路", &s)), ["AN-A|F01", "AN-B|F02"], "递归含后代");
    // 范围键收集按 Placement 各自文件夹过滤：全局时别处同名链接也收
    s.global = true;
    let hits = search(&h.lib, "走路", &s);
    assert_eq!(hit_keys(&hits), ["AN-A|F01", "AN-B|F02", "AN-C|root"], "全局 = 全库");
  }

  #[test]
  fn quick_scopes() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "文件夹");
    add_asset(&mut h.lib, "AN-A", "走路甲", "root");
    add_placement(&mut h.lib, "AN-A", "走路甲.png", "F01", false);
    add_asset_ex(&mut h.lib, "AN-B", "走路乙", "png", &[], Some("inbox"), "root");
    add_asset(&mut h.lib, "AN-C", "走路丙", "F01");
    h.lib.recent.items.push(RecentItem { asset: "AN-C".into(), viewed_at: "2026-08-19T10:00:00+08:00".into() });

    // 未分类 = 仅 root
    assert_eq!(hit_keys(&search(&h.lib, "走路", &scope("quick:uncategorized"))), ["AN-A|root", "AN-B|root"]);
    // 自动导入 = imported_from = inbox（链接不在收件箱渠道不收）
    assert_eq!(hit_keys(&search(&h.lib, "走路", &scope("quick:inbox"))), ["AN-B|root"]);
    // 最近使用 = recent 集（其全部 Placement 键收）
    assert_eq!(hit_keys(&search(&h.lib, "走路", &scope("quick:recent"))), ["AN-C|F01"]);
    // 全部
    let hits = search(&h.lib, "走路", &scope("quick:all"));
    assert_eq!(hit_keys(&hits), ["AN-A|F01", "AN-A|root", "AN-B|root", "AN-C|F01"]);
  }

  #[test]
  fn recycle_scope_and_global_excludes_recycle() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "走路甲", "root");
    // AN-B 进回收站：placements 移入快照
    add_asset(&mut h.lib, "AN-B", "走路乙", "root");
    h.lib.folders.items.retain(|p| p.asset != "AN-B");
    h.lib.index.rebuild_placements(&h.lib.folders.items);
    h.lib.recycle.items.push(RecycleItem {
      asset: "AN-B".into(),
      placements: vec![Placement { asset: "AN-B".into(), folder: "root".into(), name: "走路乙.png".into(), order: 0, origin: true }],
      deleted_at: "2026-08-19T10:00:00+08:00".into(),
    });

    // 回收站里搜回收站
    assert_eq!(hit_keys(&search(&h.lib, "走路", &scope("quick:recycle"))), ["AN-B|recycle"]);
    // 全局排除回收站
    let mut s = scope("quick:recycle");
    s.global = true;
    assert_eq!(hit_keys(&search(&h.lib, "走路", &s)), ["AN-A|root"], "全局 = 全库不含回收站");
    // 全部视图同样不含
    assert_eq!(hit_keys(&search(&h.lib, "走路", &scope("quick:all"))), ["AN-A|root"]);
  }

  #[test]
  fn prefixes_parse() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "文件夹");
    add_asset(&mut h.lib, "AN-A", "走路", "root");
    add_asset(&mut h.lib, "AN-B", "走路", "F01");

    // // 前缀 = 显式分词（与直接输入等效）
    let s = scope("folder:F01");
    assert_eq!(hit_keys(&search(&h.lib, "//走路", &s)), ["AN-B|F01"]);
    // ~ 前缀 = 本条强制全局（scope 非全局也生效）
    assert_eq!(hit_keys(&search(&h.lib, "~走路", &s)), ["AN-A|root", "AN-B|F01"]);
    // 前缀交叉顺序不限
    assert_eq!(hit_keys(&search(&h.lib, "//~走路", &s)), ["AN-A|root", "AN-B|F01"]);
    assert_eq!(hit_keys(&search(&h.lib, "~//走路", &s)), ["AN-A|root", "AN-B|F01"]);
    // 单 / = 筛选未落地，按普通文本搜（不命中任何名称）
    assert!(search(&h.lib, "/走路", &s).placements.is_empty());
  }

  // ---------- 筛选（2026-10-09 筛选业务一轮）----------

  /// 距今 N 天的本地中午 RFC3339（日期档位测试用；中午避开时区歧义时刻）
  fn days_ago_rfc3339(days: i64) -> String {
    let d = chrono::Local::now().date_naive() - chrono::Duration::days(days);
    d.and_hms_opt(12, 0, 0).unwrap().and_local_timezone(chrono::Local).unwrap().to_rfc3339()
  }

  fn set_asset(lib: &mut Library, id: &str, f: impl FnOnce(&mut AssetMeta)) {
    f(lib.index.assets.get_mut(id).unwrap());
  }

  #[test]
  fn filters_exts_or_and_cross_with_keywords() {
    let mut h = make_lib();
    add_asset_ex(&mut h.lib, "AN-A", "男人走路", "fla", &[], None, "root");
    add_asset_ex(&mut h.lib, "AN-B", "男人跑步", "psd", &[], None, "root");
    add_asset_ex(&mut h.lib, "AN-C", "男人睡觉", "png", &[], None, "root");

    // 纯筛选（无关键字）：维度内多选 OR
    let f = SearchFilters { exts: vec!["fla".into(), "psd".into()], ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-A|root", "AN-B|root"]);
    // 筛选 × 搜索 AND
    let f = SearchFilters { exts: vec!["fla".into(), "psd".into()], ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "跑步", &scope_filtered("quick:all", f))), ["AN-B|root"]);
    // 大小写不敏感 + 容忍点前缀
    let f = SearchFilters { exts: vec![".FLA".into()], ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-A|root"]);
  }

  #[test]
  fn filters_cross_dimension_and() {
    let mut h = make_lib();
    add_asset_ex(&mut h.lib, "AN-A", "小动画", "fla", &[], None, "root");
    add_asset_ex(&mut h.lib, "AN-B", "大动画", "fla", &[], None, "root");
    add_asset_ex(&mut h.lib, "AN-C", "大图", "png", &[], None, "root");
    set_asset(&mut h.lib, "AN-B", |m| m.size = 5000);
    set_asset(&mut h.lib, "AN-C", |m| m.size = 5000);

    // 跨维度 AND：fla 且 ≥1000 字节
    let f = SearchFilters {
      exts: vec!["fla".into()],
      size: Some(ByteRange { min: Some(1000), max: None }),
      ..Default::default()
    };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-B|root"]);
  }

  #[test]
  fn filters_colors_tolerance() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "红图", "root");
    add_asset(&mut h.lib, "AN-B", "绿图", "root");
    add_asset(&mut h.lib, "AN-C", "无色板", "root");
    set_asset(&mut h.lib, "AN-A", |m| m.colors = vec!["#FF0000".into()]);
    set_asset(&mut h.lib, "AN-B", |m| m.colors = vec!["#00FF00".into()]);

    // 距离 13 ≤ 64 命中（同色系近似）
    let f = SearchFilters { colors: vec!["#F52222".into()], ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-A|root"]);
    // 距离 128 > 64 不中；无色板资产不命中颜色条件
    let f = SearchFilters { colors: vec!["#FF8000".into()], ..Default::default() };
    assert!(search(&h.lib, "", &scope_filtered("quick:all", f)).placements.is_empty());
    // 目标全无法解析 = 维度不参与（防垃圾输入清空结果）
    let f = SearchFilters { colors: vec!["垃圾".into()], ..Default::default() };
    assert_eq!(search(&h.lib, "", &scope_filtered("quick:all", f)).placements.len(), 3);
  }

  #[test]
  fn filters_duration_units() {
    let mut h = make_lib();
    add_asset_ex(&mut h.lib, "AN-A", "十秒动画", "gif", &[], None, "root");
    add_asset_ex(&mut h.lib, "AN-B", "两分钟歌", "mp3", &[], None, "root");
    add_asset(&mut h.lib, "AN-C", "静态图", "root");
    set_asset(&mut h.lib, "AN-A", |m| { m.duration = Some(10.0); m.fps = Some(24.0); });
    set_asset(&mut h.lib, "AN-B", |m| m.duration = Some(120.0));

    // 秒：5..=20 → A（C 无时长不命中）
    let f = SearchFilters { duration: Some(DurationRange { unit: "seconds".into(), min: Some(5.0), max: Some(20.0) }), ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-A|root"]);
    // 分：1..=3 → B
    let f = SearchFilters { duration: Some(DurationRange { unit: "minutes".into(), min: Some(1.0), max: Some(3.0) }), ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-B|root"]);
    // 帧：10s×24fps = 240 帧 → A；B 无 fps 不命中帧条件
    let f = SearchFilters { duration: Some(DurationRange { unit: "frames".into(), min: Some(200.0), max: Some(260.0) }), ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-A|root"]);
  }

  #[test]
  fn filters_tags_case_insensitive_or() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "图甲", "root");
    add_asset(&mut h.lib, "AN-B", "图乙", "root");
    add_asset(&mut h.lib, "AN-C", "图丙", "root");
    set_asset(&mut h.lib, "AN-A", |m| m.tags = vec!["拔剑".into()]);
    set_asset(&mut h.lib, "AN-B", |m| m.tags = vec!["Q版".into(), "主角".into()]);

    // 维度内 OR + 大小写不敏感
    let f = SearchFilters { tags: vec!["拔剑".into(), "q版".into()], ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-A|root", "AN-B|root"]);
    // 无该属性词不中
    let f = SearchFilters { tags: vec!["不存在".into()], ..Default::default() };
    assert!(search(&h.lib, "", &scope_filtered("quick:all", f)).placements.is_empty());
  }

  #[test]
  fn filters_ratios_presets_and_custom() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "横图", "root");
    add_asset(&mut h.lib, "AN-B", "方图", "root");
    add_asset(&mut h.lib, "AN-C", "竖图", "root");
    add_asset(&mut h.lib, "AN-D", "近方图", "root");
    add_asset_ex(&mut h.lib, "AN-E", "工程", "fla", &[], None, "root"); // 无宽高
    set_asset(&mut h.lib, "AN-A", |m| { m.width = Some(1920); m.height = Some(1080); });
    set_asset(&mut h.lib, "AN-B", |m| { m.width = Some(1000); m.height = Some(1000); });
    set_asset(&mut h.lib, "AN-C", |m| { m.width = Some(1080); m.height = Some(1920); });
    set_asset(&mut h.lib, "AN-D", |m| { m.width = Some(1049); m.height = Some(1000); });

    let f = SearchFilters { ratios: vec!["landscape".into()], ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-A|root"], "横图（无宽高资产不命中）");
    let f = SearchFilters { ratios: vec!["square".into()], ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-B|root", "AN-D|root"], "方图含 1.049 近方");
    let f = SearchFilters { ratios: vec!["portrait".into()], ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-C|root"], "竖图");
    // 自定义 16:9（容差 1%）
    let f = SearchFilters { ratios: vec!["16:9".into()], ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-A|root"]);
    // 全无法解析 = 维度不参与
    let f = SearchFilters { ratios: vec!["垃圾".into()], ..Default::default() };
    assert_eq!(search(&h.lib, "", &scope_filtered("quick:all", f)).placements.len(), 5);
  }

  #[test]
  fn filters_size_range() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "小", "root");
    add_asset(&mut h.lib, "AN-B", "中", "root");
    add_asset(&mut h.lib, "AN-C", "大", "root");
    set_asset(&mut h.lib, "AN-B", |m| m.size = 1000);
    set_asset(&mut h.lib, "AN-C", |m| m.size = 9000);

    // 双端
    let f = SearchFilters { size: Some(ByteRange { min: Some(500), max: Some(5000) }), ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-B|root"]);
    // 仅 min / 仅 max
    let f = SearchFilters { size: Some(ByteRange { min: Some(5000), max: None }), ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-C|root"]);
    let f = SearchFilters { size: Some(ByteRange { min: None, max: Some(500) }), ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-A|root"]);
  }

  #[test]
  fn filters_date_buckets_and_custom() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "今天", "root");
    add_asset(&mut h.lib, "AN-B", "昨天", "root");
    add_asset(&mut h.lib, "AN-C", "十天前", "root");
    add_asset(&mut h.lib, "AN-D", "百天前", "root");
    add_asset(&mut h.lib, "AN-E", "去年", "root");
    set_asset(&mut h.lib, "AN-A", |m| m.imported_at = days_ago_rfc3339(0));
    set_asset(&mut h.lib, "AN-B", |m| m.imported_at = days_ago_rfc3339(1));
    set_asset(&mut h.lib, "AN-C", |m| m.imported_at = days_ago_rfc3339(10));
    set_asset(&mut h.lib, "AN-D", |m| m.imported_at = days_ago_rfc3339(100));
    set_asset(&mut h.lib, "AN-E", |m| m.imported_at = days_ago_rfc3339(400));

    let imported = |buckets: &[&str]| SearchFilters {
      imported: Some(DateCond { buckets: buckets.iter().map(|s| s.to_string()).collect(), custom: None }),
      ..Default::default()
    };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", imported(&["today"])))), ["AN-A|root"]);
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", imported(&["yesterday"])))), ["AN-B|root"]);
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", imported(&["week"])))), ["AN-A|root", "AN-B|root"], "近 7 天");
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", imported(&["month"])))), ["AN-A|root", "AN-B|root", "AN-C|root"], "近 30 天");
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", imported(&["year"])))), ["AN-A|root", "AN-B|root", "AN-C|root", "AN-D|root"], "近 365 天");
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", imported(&["earlier"])))), ["AN-E|root"], "更早 = 365 天前");
    // 档位多选 OR
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", imported(&["today", "earlier"])))), ["AN-A|root", "AN-E|root"]);
    // 自定义：年+月+日全等（今天）→ 只中 A；只填年 = 该年全部（400 天前恒属去年）
    let this_year = chrono::Local::now().date_naive().year();
    let f = SearchFilters {
      imported: Some(DateCond { buckets: vec![], custom: Some(Ymd { year: Some(this_year), month: None, day: None }) }),
      ..Default::default()
    };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-A|root", "AN-B|root", "AN-C|root", "AN-D|root"]);
    // 修改日期维度独立：C 数据今天改过
    set_asset(&mut h.lib, "AN-C", |m| m.modified_at = days_ago_rfc3339(0));
    let f = SearchFilters {
      modified: Some(DateCond { buckets: vec!["today".into()], custom: None }),
      ..Default::default()
    };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-C|root"]);
  }

  #[test]
  fn filters_folder_ids_placement_level() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "文件夹");
    add_folder_ex(&mut h.lib, "F02", "子夹", Some("F01"), None, None);
    add_asset(&mut h.lib, "AN-A", "两位置", "root");
    add_placement(&mut h.lib, "AN-A", "两位置.png", "F01", false);
    add_asset(&mut h.lib, "AN-B", "夹内", "F01");
    add_asset(&mut h.lib, "AN-C", "子夹内", "F02");

    // 位置级判定：AN-A 仅 F01 位置命中，root 位置不带出；**含后代**（勾父圈整棵子树，2026-10-09 三轮定）
    let f = SearchFilters { folder_ids: vec!["F01".into()], ..Default::default() };
    assert_eq!(
      hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))),
      ["AN-A|F01", "AN-B|F01", "AN-C|F02"],
      "自身或后代（F02 子夹随 F01 命中）"
    );
    // root 直属（未分类）
    let f = SearchFilters { folder_ids: vec!["root".into()], ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-A|root"]);
    // 仅子夹 = 不含父
    let f = SearchFilters { folder_ids: vec!["F02".into()], ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-C|F02"]);
  }

  #[test]
  fn filters_link_states() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "文件夹");
    add_asset(&mut h.lib, "AN-A", "有链接", "root");
    add_placement(&mut h.lib, "AN-A", "有链接.png", "F01", false);
    add_asset(&mut h.lib, "AN-B", "独立", "root");
    add_asset(&mut h.lib, "AN-C", "有缩略图", "root");
    set_asset(&mut h.lib, "AN-C", |m| m.thumb_asset_id = Some("AN-T".into()));

    let link = |states: &[&str]| SearchFilters { link_states: states.iter().map(|s| s.to_string()).collect(), ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", link(&["origin"])))),
      ["AN-A|root", "AN-B|root", "AN-C|root"], "原始位置");
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", link(&["linked"])))), ["AN-A|F01"], "链接件");
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", link(&["thumb"])))), ["AN-C|root"], "有伴生缩略图引用");
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", link(&["none"])))), ["AN-B|root", "AN-C|root"], "仅 1 条活位置");
    // 维度内 OR（链接件 ∪ 缩略图）
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", link(&["linked", "thumb"])))), ["AN-A|F01", "AN-C|root"]);
  }

  #[test]
  fn filters_active_suppresses_archives() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "父夹");
    add_folder_ex(&mut h.lib, "AR01", "角色档案", Some("F01"), None, Some("archive"));
    add_asset_ex(&mut h.lib, "AN-A", "角色图", "png", &[], None, "F01");

    // 无筛选：档案照出（回归）
    let hits = search(&h.lib, "角色", &scope("folder:F01"));
    assert_eq!(hits.archives, ["AR01"]);
    // 筛选激活：档案区不出
    let f = SearchFilters { exts: vec!["png".into()], ..Default::default() };
    let hits = search(&h.lib, "角色", &scope_filtered("folder:F01", f));
    assert_eq!(hit_keys(&hits), ["AN-A|F01"]);
    assert!(hits.archives.is_empty(), "筛选激活 = 档案区不出");
    // 空壳筛选（范围双端空）= 未激活，档案照出
    let f = SearchFilters { duration: Some(DurationRange { unit: "seconds".into(), min: None, max: None }), ..Default::default() };
    let hits = search(&h.lib, "角色", &scope_filtered("folder:F01", f));
    assert_eq!(hits.archives, ["AR01"], "空壳条件不算激活");
  }

  #[test]
  fn filters_recycle_scope() {
    let mut h = make_lib();
    add_asset_ex(&mut h.lib, "AN-B", "回收动画", "fla", &[], None, "root");
    add_asset(&mut h.lib, "AN-C", "回收图", "root");
    h.lib.folders.items.retain(|p| p.asset != "AN-B" && p.asset != "AN-C");
    h.lib.index.rebuild_placements(&h.lib.folders.items);
    for (id, name) in [("AN-B", "回收动画.fla"), ("AN-C", "回收图.png")] {
      h.lib.recycle.items.push(RecycleItem {
        asset: id.into(),
        placements: vec![Placement { asset: id.into(), folder: "root".into(), name: name.into(), order: 0, origin: true }],
        deleted_at: "2026-08-19T10:00:00+08:00".into(),
      });
    }

    // 回收站视口筛选：资产级维度施加于快照
    let f = SearchFilters { exts: vec!["fla".into()], ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:recycle", f))), ["AN-B|recycle"]);
    // 位置级（快照 origin）+ 文本 AND
    let f = SearchFilters { link_states: vec!["origin".into()], ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "回收", &scope_filtered("quick:recycle", f))), ["AN-B|recycle", "AN-C|recycle"]);
  }

  #[test]
  fn filters_ext_sequence_virtual() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-S", "走路序列", "root");
    // 序列帧磁盘口径（frames_dir_if_sequence）：info 根无 origin 文件 + frames/ 目录存在
    let info = crate::core::paths::info_dir(&h.lib.assets_dir, "AN-S");
    std::fs::create_dir_all(info.join("frames")).unwrap();
    add_asset(&mut h.lib, "AN-P", "普通图", "root"); // info 根与 frames 均不存在 → 不命中

    let f = SearchFilters { exts: vec!["seq".into()], ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-S|root"], "序列帧虚拟类别");
    // 与真实扩展名混选 OR
    let f = SearchFilters { exts: vec!["seq".into(), "png".into()], ..Default::default() };
    assert_eq!(hit_keys(&search(&h.lib, "", &scope_filtered("quick:all", f))), ["AN-P|root", "AN-S|root"]);
  }

  /// 档案虚拟类别（2026-10-12 用户定）：exts 含 "archive" 时档案区参与筛选，按可判定维度 AND
  #[test]
  fn filters_archive_virtual_category() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "项目");
    add_folder_ex(&mut h.lib, "AR-A", "角色档案", Some("F01"), None, Some("archive"));
    add_folder_ex(&mut h.lib, "AR-B", "场景档案", None, None, Some("archive")); // 根级档案
    add_asset_ex(&mut h.lib, "AN-A", "角色图", "png", &[], None, "F01");
    h.lib.archives.archives.push(ArchiveData {
      folder_id: "AR-A".into(),
      badge: None,
      avatar: None,
      props: vec!["主角".into()],
      descriptions: Vec::new(),
      links: Vec::new(),
      created_at: days_ago_rfc3339(3),
      prompt: None,
    });

    let cat = |exts: &[&str]| SearchFilters { exts: exts.iter().map(|s| s.to_string()).collect(), ..Default::default() };
    // 纯类别筛选：folder 视图直属口径（AR-B 根级，folder:F01 视口不出）；资产无 archive 扩展名不中
    let hits = search(&h.lib, "", &scope_filtered("folder:F01", cat(&["archive"])));
    assert_eq!(hits.archives, ["AR-A"]);
    assert!(hits.placements.is_empty());
    // 全局 = 全库档案
    let mut g = scope_filtered("quick:all", cat(&["archive"]));
    g.global = true;
    assert_eq!(search(&h.lib, "", &g).archives, ["AR-A", "AR-B"]);
    // 维度内 OR：archive ∪ png → 档案与 png 资产同中
    let hits = search(&h.lib, "", &scope_filtered("folder:F01", cat(&["archive", "png"])));
    assert_eq!(hits.archives, ["AR-A"]);
    assert_eq!(hit_keys(&hits), ["AN-A|F01"]);
    // 大小写 / 点前缀容忍（同资产侧 exts 惯例）
    assert_eq!(search(&h.lib, "", &scope_filtered("folder:F01", cat(&[".Archive"]))).archives, ["AR-A"]);
    // 跨维度 AND——属性：命中「主角」出，不中的词剔出
    let f = SearchFilters { tags: vec!["主角".into()], ..cat(&["archive"]) };
    assert_eq!(search(&h.lib, "", &scope_filtered("folder:F01", f)).archives, ["AR-A"]);
    let f = SearchFilters { tags: vec!["配角".into()], ..cat(&["archive"]) };
    assert!(search(&h.lib, "", &scope_filtered("folder:F01", f)).archives.is_empty());
    // 所在文件夹维度（含后代集同口径）：AR-A parent = F01 中；AR-B 根级无 parent 不中
    let f = SearchFilters { folder_ids: vec!["F01".into()], ..cat(&["archive"]) };
    let mut g = scope_filtered("quick:all", f);
    g.global = true;
    assert_eq!(search(&h.lib, "", &g).archives, ["AR-A"]);
    // 添加日期维度（created_at）：3 天前 ∈ week；earlier 不中
    let f = SearchFilters {
      imported: Some(DateCond { buckets: vec!["week".into()], custom: None }),
      ..cat(&["archive"])
    };
    assert_eq!(search(&h.lib, "", &scope_filtered("folder:F01", f)).archives, ["AR-A"]);
    let f = SearchFilters {
      imported: Some(DateCond { buckets: vec!["earlier".into()], custom: None }),
      ..cat(&["archive"])
    };
    assert!(search(&h.lib, "", &scope_filtered("folder:F01", f)).archives.is_empty());
    // 资产专属维度激活 = 档案出局（大小 / 颜色 / 修改日期各验一处代表）
    let f = SearchFilters { size: Some(ByteRange { min: Some(1), max: None }), ..cat(&["archive"]) };
    assert!(search(&h.lib, "", &scope_filtered("folder:F01", f)).archives.is_empty());
    let f = SearchFilters { colors: vec!["#FF0000".into()], ..cat(&["archive"]) };
    assert!(search(&h.lib, "", &scope_filtered("folder:F01", f)).archives.is_empty());
    // 筛选 × 搜索 AND：关键字命中档案名才出
    assert_eq!(search(&h.lib, "角色", &scope_filtered("folder:F01", cat(&["archive"]))).archives, ["AR-A"]);
    assert!(search(&h.lib, "不存在词", &scope_filtered("folder:F01", cat(&["archive"]))).archives.is_empty());
    // 未勾档案类别 = 旧口径档案区不出（filters_active_suppresses_archives 同义回归）
    assert!(search(&h.lib, "", &scope_filtered("folder:F01", cat(&["png"]))).archives.is_empty());
  }
}
