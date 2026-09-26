//! 自定义数据服务（6.1，PRD 03_数据链接 02/06）：文件与档案的属性（打词）/ 描述（标题 + 内容）编辑。
//! 文件侧落 metadata.json（info 内单一写点，先落盘再同步索引，规范第 7 条）；
//! 档案侧落 archives.json（core::library::save_archives 单一写点，记录懒建）。
//! 属性池（32 稿「已有属性标签」）= 全库聚合：全部资产 tags + 全部档案 props 去重；
//! 池内删除 = 全库失效（32 稿 prose：二次确认后所有填写了该属性的地方都失效）。

use crate::core::json_io::write_json;
use crate::core::library::{self, Library};
use crate::core::models::{DescItem, ARCHIVE_KIND};
use crate::core::paths;
use crate::services::operations;

/// 属性规整：逐项 trim，去空串、去重保序
fn normalize_props(props: &[String]) -> Vec<String> {
  let mut out: Vec<String> = Vec::with_capacity(props.len());
  for p in props {
    let t = p.trim();
    if !t.is_empty() && !out.iter().any(|x| x == t) {
      out.push(t.to_string());
    }
  }
  out
}

/// 描述规整：标题 / 内容 trim，双空条目丢弃
fn normalize_descriptions(items: &[DescItem]) -> Vec<DescItem> {
  items
    .iter()
    .map(|d| DescItem { title: d.title.trim().to_string(), content: d.content.trim().to_string() })
    .filter(|d| !d.title.is_empty() || !d.content.is_empty())
    .collect()
}

/// 设置文件属性（tags 字段语义 = 属性词，2026-08-04 标签体系取消后复用免迁移）。
/// 返回是否有改动（无改动 = 未落盘，命令层据此前撤销事务取消）
pub fn set_asset_props(lib: &mut Library, asset_id: &str, props: &[String]) -> Result<bool, String> {
  let meta = lib.index.assets.get(asset_id).ok_or_else(|| "资源不存在，可能已被删除".to_string())?.clone();
  let next = normalize_props(props);
  if next == meta.tags {
    return Ok(false);
  }
  write_asset_meta(lib, meta, |m| m.tags = next)?;
  Ok(true)
}

/// 设置文件描述（metadata.descriptions，6.1 新增字段）。返回是否有改动
pub fn set_asset_descriptions(lib: &mut Library, asset_id: &str, descriptions: &[DescItem]) -> Result<bool, String> {
  let meta = lib.index.assets.get(asset_id).ok_or_else(|| "资源不存在，可能已被删除".to_string())?.clone();
  let next = normalize_descriptions(descriptions);
  if next == meta.descriptions {
    return Ok(false);
  }
  write_asset_meta(lib, meta, |m| m.descriptions = next)?;
  Ok(true)
}

/// metadata 落盘 + 索引同步（先落盘再更新）+ modified_at 刷新（库内变更时刻，3.5 排序键同口径）
/// pub(crate)：services::prompt（2026-09-26 提示词轮）复用同一写点（metadata.json 单一写点纪律）
pub(crate) fn write_asset_meta(lib: &mut Library, mut meta: crate::core::models::AssetMeta, apply: impl FnOnce(&mut crate::core::models::AssetMeta)) -> Result<(), String> {
  apply(&mut meta);
  meta.modified_at = operations::now();
  let dir = paths::info_dir(&lib.assets_dir, &meta.id);
  write_json(&dir.join("metadata.json"), &meta).map_err(|e| e.to_string())?;
  let id = meta.id.clone();
  lib.index.assets.insert(id, meta);
  Ok(())
}

/// 档案记录读写前置：文件夹存在且为档案；记录懒建（同 ensure_archive_record 口径，不落 created_at——
/// 编辑数据不是新建 / 转换，created_at 保持转换当次语义；无记录时 created_at 留空）
/// pub(crate)：services::prompt（2026-09-26 提示词轮）复用同一前置（档案提示词懒建记录）
pub(crate) fn archive_record_mut<'a>(lib: &'a mut Library, folder_id: &str) -> Result<&'a mut crate::core::models::ArchiveData, String> {
  let f = lib.index.folders.get(folder_id).ok_or_else(|| "目标档案不存在，可能已被删除".to_string())?;
  if f.kind.as_deref() != Some(ARCHIVE_KIND) {
    return Err("目标不是档案".to_string());
  }
  if lib.archives.archives.iter().all(|a| a.folder_id != folder_id) {
    lib.archives.archives.push(crate::core::models::ArchiveData::new(folder_id));
  }
  Ok(lib.archives.archives.iter_mut().find(|a| a.folder_id == folder_id).expect("懒建后必有"))
}

/// 设置档案属性。返回是否有改动
pub fn set_archive_props(lib: &mut Library, folder_id: &str, props: &[String]) -> Result<bool, String> {
  let next = normalize_props(props);
  let rec = archive_record_mut(lib, folder_id)?;
  if rec.props == next {
    return Ok(false);
  }
  rec.props = next;
  library::save_archives(lib).map_err(|e| e.to_string())?;
  Ok(true)
}

/// 设置档案描述。返回是否有改动
pub fn set_archive_descriptions(lib: &mut Library, folder_id: &str, descriptions: &[DescItem]) -> Result<bool, String> {
  let next = normalize_descriptions(descriptions);
  let rec = archive_record_mut(lib, folder_id)?;
  if rec.descriptions == next {
    return Ok(false);
  }
  rec.descriptions = next;
  library::save_archives(lib).map_err(|e| e.to_string())?;
  Ok(true)
}

/// 属性池（32 稿「已有属性标签：」）：全库聚合去重，按小写字典序（稳定可测）
pub fn list_prop_pool(lib: &Library) -> Vec<String> {
  let mut pool: Vec<String> = Vec::new();
  for meta in lib.index.assets.values() {
    for t in &meta.tags {
      if !pool.iter().any(|x| x == t) {
        pool.push(t.clone());
      }
    }
  }
  for a in &lib.archives.archives {
    for p in &a.props {
      if !pool.iter().any(|x| x == p) {
        pool.push(p.clone());
      }
    }
  }
  pool.sort_by_key(|a| a.to_lowercase());
  pool
}

/// 属性使用面（全局删除前只读扫描）：哪些资产 tags / 档案 props 含该词——命令层据此 record undo 快照
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PropUsage {
  pub asset_ids: Vec<String>,
  pub archive_ids: Vec<String>,
}

pub fn prop_usage(lib: &Library, word: &str) -> PropUsage {
  let mut usage = PropUsage::default();
  for meta in lib.index.assets.values() {
    if meta.tags.iter().any(|t| t == word) {
      usage.asset_ids.push(meta.id.clone());
    }
  }
  usage.asset_ids.sort();
  for a in &lib.archives.archives {
    if a.props.iter().any(|p| p == word) {
      usage.archive_ids.push(a.folder_id.clone());
    }
  }
  usage.archive_ids.sort();
  usage
}

/// 全局删除属性（32 稿池内 ×，警告二次确认后执行）：全部命中资产 tags / 档案 props 移除该词并落盘。
/// 调用方须先 prop_usage + undo record 对应文件
pub fn delete_prop_globally(lib: &mut Library, word: &str) -> Result<(), String> {
  let usage = prop_usage(lib, word);
  for id in &usage.asset_ids {
    let meta = lib.index.assets.get(id).expect("usage 扫描后资产必在").clone();
    write_asset_meta(lib, meta, |m| m.tags.retain(|t| t != word))?;
  }
  if !usage.archive_ids.is_empty() {
    for a in lib.archives.archives.iter_mut().filter(|a| usage.archive_ids.contains(&a.folder_id)) {
      a.props.retain(|p| p != word);
    }
    library::save_archives(lib).map_err(|e| e.to_string())?;
  }
  log::info!("全局删除属性「{word}」：资产 {} 处 / 档案 {} 处", usage.asset_ids.len(), usage.archive_ids.len());
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library::ARCHIVES_FILE;
  use crate::core::models::{AssetMeta, Folder};
  use crate::core::undo::UndoManager;

  struct Harness {
    _dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { _dir: dir, lib }
  }

  fn add_asset(lib: &mut Library, id: &str, tags: &[&str]) {
    let meta: AssetMeta = serde_json::from_value(serde_json::json!({
      "version": 1, "id": id, "name": id, "originName": format!("{id}.png"), "ext": "png", "size": 100,
      "importedAt": "2026-08-23T00:00:00+08:00", "modifiedAt": "2026-08-23T00:00:00+08:00", "tags": tags
    }))
    .unwrap();
    lib.index.assets.insert(id.into(), meta);
    // 落 info 目录 + metadata（write_asset_meta 覆写前文件须存在目录）
    let dir = paths::info_dir(&lib.assets_dir, id);
    std::fs::create_dir_all(&dir).unwrap();
    write_json(&dir.join("metadata.json"), lib.index.assets.get(id).unwrap()).unwrap();
  }

  fn add_archive(lib: &mut Library, id: &str, props: &[&str]) {
    let folder = Folder {
      id: id.into(),
      name: id.into(),
      parent: None,
      block: None,
      icon: None,
      kind: Some(ARCHIVE_KIND.into()),
      color: None,
      groups: None,
      order: 0,
    };
    lib.folders.folders.push(folder.clone());
    lib.index.folders.insert(id.into(), folder);
    let mut rec = crate::core::models::ArchiveData::new(id);
    rec.props = props.iter().map(|s| s.to_string()).collect();
    lib.archives.archives.push(rec);
  }

  #[test]
  fn set_asset_props_normalizes_and_persists() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", &[]);
    let changed = set_asset_props(&mut h.lib, "AN-A", &[" 走路 ".into(), "跑步".into(), "走路".into(), "  ".into()]).unwrap();
    assert!(changed);
    assert_eq!(h.lib.index.assets["AN-A"].tags, vec!["走路", "跑步"], "trim + 去重保序");
    let disk: AssetMeta = read_json(&paths::info_dir(&h.lib.assets_dir, "AN-A").join("metadata.json")).unwrap();
    assert_eq!(disk.tags, vec!["走路", "跑步"], "落盘");
    // 无改动 = false 不落盘
    assert!(!set_asset_props(&mut h.lib, "AN-A", &["走路".into(), "跑步".into()]).unwrap());
    assert!(set_asset_props(&mut h.lib, "AN-不存在", &[]).is_err());
  }

  #[test]
  fn set_asset_descriptions_persists() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", &[]);
    let items = vec![
      DescItem { title: " 身份 ".into(), content: "主角".into() },
      DescItem { title: " ".into(), content: " ".into() }, // 双空丢弃
    ];
    assert!(set_asset_descriptions(&mut h.lib, "AN-A", &items).unwrap());
    assert_eq!(h.lib.index.assets["AN-A"].descriptions.len(), 1);
    assert_eq!(h.lib.index.assets["AN-A"].descriptions[0].title, "身份");
    let disk: AssetMeta = read_json(&paths::info_dir(&h.lib.assets_dir, "AN-A").join("metadata.json")).unwrap();
    assert_eq!(disk.descriptions[0].content, "主角", "落盘");
    // 存量 metadata 无 descriptions 字段 default 读入（models 层兜底；此处验证清空语义）
    assert!(set_asset_descriptions(&mut h.lib, "AN-A", &[]).unwrap());
    assert!(h.lib.index.assets["AN-A"].descriptions.is_empty());
  }

  #[test]
  fn archive_props_and_descriptions_lazy_record() {
    let mut h = make_lib();
    add_archive(&mut h.lib, "F-A", &["旧"]);
    add_folder_plain(&mut h.lib, "F-PLAIN");
    assert!(set_archive_props(&mut h.lib, "F-A", &["新".into()]).unwrap());
    assert_eq!(h.lib.archives.archives.iter().find(|a| a.folder_id == "F-A").unwrap().props, vec!["新"]);
    // 无记录档案懒建（不落 created_at——编辑数据不改创建时间语义）
    add_folder_kind_archive(&mut h.lib, "F-B");
    assert!(set_archive_descriptions(&mut h.lib, "F-B", &[DescItem { title: "t".into(), content: "c".into() }]).unwrap());
    let rec = h.lib.archives.archives.iter().find(|a| a.folder_id == "F-B").unwrap();
    assert_eq!(rec.descriptions.len(), 1);
    assert!(rec.created_at.is_empty(), "编辑懒建不写 created_at");
    let disk: crate::core::models::ArchivesFile = read_json(&h.lib.root.join(ARCHIVES_FILE)).unwrap();
    assert_eq!(disk.archives.len(), 2, "落盘");
    // 非档案报错
    assert!(set_archive_props(&mut h.lib, "F-PLAIN", &[]).is_err());
  }

  fn add_folder_plain(lib: &mut Library, id: &str) {
    let folder = Folder {
      id: id.into(),
      name: id.into(),
      parent: None,
      block: None,
      icon: None,
      kind: None,
      color: None,
      groups: None,
      order: 0,
    };
    lib.folders.folders.push(folder.clone());
    lib.index.folders.insert(id.into(), folder);
  }

  fn add_folder_kind_archive(lib: &mut Library, id: &str) {
    add_archive(lib, id, &[]);
    lib.archives.archives.retain(|a| a.folder_id != id); // 只留 kind 标记，无记录
  }

  #[test]
  fn prop_pool_aggregates_and_sorts() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", &["走路", "主角"]);
    add_asset(&mut h.lib, "AN-B", &["跑步", "主角"]); // 跨资产重复
    add_archive(&mut h.lib, "F-A", &["二维", "走路"]); // 与资产重复
    let pool = list_prop_pool(&h.lib);
    assert_eq!(pool, vec!["主角", "二维", "走路", "跑步"], "聚合去重 + 码点序（稳定即可，稿无顺序口径）");
  }

  /// 全局删除：资产 tags + 档案 props 全扫移除；undo 回滚（record metadata + archives.json）
  #[test]
  fn delete_prop_globally_removes_everywhere_and_undoes() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", &["走路", "主角"]);
    add_asset(&mut h.lib, "AN-B", &["跑步"]);
    add_archive(&mut h.lib, "F-A", &["走路"]);
    library::save_archives(&h.lib).unwrap();
    library::save_folders(&h.lib).unwrap();

    let usage = prop_usage(&h.lib, "走路");
    assert_eq!(usage, PropUsage { asset_ids: vec!["AN-A".into()], archive_ids: vec!["F-A".into()] });

    let meta_path = paths::info_dir(&h.lib.assets_dir, "AN-A").join("metadata.json");
    let mut undo = UndoManager::default();
    undo.begin("删除属性");
    undo.record_file(&meta_path).unwrap();
    undo.record_file(&h.lib.root.join(ARCHIVES_FILE)).unwrap();
    delete_prop_globally(&mut h.lib, "走路").unwrap();
    undo.commit().unwrap();

    assert_eq!(h.lib.index.assets["AN-A"].tags, vec!["主角"]);
    assert_eq!(h.lib.index.assets["AN-B"].tags, vec!["跑步"], "未命中资产不动");
    assert!(h.lib.archives.archives.iter().find(|a| a.folder_id == "F-A").unwrap().props.is_empty());

    undo.undo().unwrap();
    library::reload(&mut h.lib).unwrap();
    assert_eq!(h.lib.index.assets["AN-A"].tags, vec!["走路", "主角"], "undo 恢复资产 tags");
    assert_eq!(h.lib.archives.archives.iter().find(|a| a.folder_id == "F-A").unwrap().props, vec!["走路"], "undo 恢复档案 props");
  }
}
