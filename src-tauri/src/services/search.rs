//! 分词搜索（3.6 一轮，PRD 03_查找文件 01）：空格分词 AND；逐关键字子串包含，不区分大小写；
//! 链接字段匹配前先做 URL decode（抓取链接中编码的关键词可命中，如 q=%E6%A1%A3%E6%A1%88 输"档案"可命中）。
//!
//! 命中范围（MVP 落地子集）：
//! - 素材 = 资产名 / 各 Placement 显示名 / 扩展名 / 链接 / 所在路径链（区块 / 各级文件夹 / 档案名）
//! - 档案 = 档案名 / 徽标文本 / 属性 / 描述 / 链接
//! 资产侧属性 / 描述 / 提示词 / 关系 / 音色与标注名 / 注释随对应功能就位后纳入（阶段六）。
//!
//! 范围口径（2026-08-19 用户定）：
//! - 当前目录跟随递归显示开关（scope.recursive）；快速访问视图 = 当前视图即当前范围（回收站里搜回收站）；
//! - 全局（`~` 前缀；2026-08-31 tag 化 chips 废除后手敲仍生效）= 全库，回收站除外（回收站资产无活 Placement，天然排除）；
//! - 档案区仅文件夹视图出（快速访问同既有口径不出）；非全局时档案仅直属口径
//!   （同 stripArchivesFor，递归开关不影响档案区），全局时 = 全库命中档案。

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::core::library::Library;
use crate::core::models::{AssetMeta, Folder, Placement, ROOT_FOLDER_ID};

/// 搜索范围（前端按视口组装）：viewKey + 递归开关 + 全局标记；query 含 ~ 前缀时强制全局
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchScope {
  pub view_key: String,
  pub recursive: bool,
  pub global: bool,
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
/// 属性 + 描述（标题 + 内容——资产侧属性·描述 6.1 就位纳入，PRD 03_查找文件 01 节）
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
  for p in placements {
    texts.push(p.name.to_lowercase());
    if let Some(path) = path_text(lib, &p.folder) {
      texts.push(path.to_lowercase());
    }
  }
  texts
}

/// 档案可搜文本集（小写化）：档案名 / 徽标文本 / 属性 / 描述（标题 + 内容）/ 链接（decode）
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

pub fn search(lib: &Library, query: &str, scope: &SearchScope) -> SearchHits {
  let (keywords, force_global) = parse_query(query);
  if keywords.is_empty() {
    return SearchHits::default();
  }
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

  // 回收站范围（非全局时）：快照 placements 补文本；命中键 folderId 恒 "recycle"（前端回收站视图键口径）
  if !global && scope.view_key == "quick:recycle" {
    for item in &lib.recycle.items {
      let Some(meta) = lib.index.assets.get(&item.asset) else { continue };
      if matches(&asset_texts(lib, meta, &item.placements), &keywords) {
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
    if !matches(&asset_texts(lib, meta, placements), &keywords) {
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
      if in_scope {
        hits.placements.push(SearchHitKey { asset_id: meta.id.clone(), folder_id: p.folder.clone() });
      }
    }
  }

  // 档案命中：仅文件夹视图出档案区（快速访问同既有口径不出）；非全局 = 仅直属（同 stripArchivesFor
  // 口径，递归开关不影响档案区）；全局 = 全库命中档案
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
    if in_scope && matches(&archive_texts(lib, f), &keywords) {
      hits.archives.push(f.id.clone());
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
    SearchScope { view_key: view_key.into(), recursive: false, global: false }
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
}
