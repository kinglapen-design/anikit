//! 内存索引（PRD 06-03）：打开库时建立，运行期查询全部走内存。
//! 写操作先落盘再更新索引（01_开发规范 第 7 条）。

use std::collections::{HashMap, HashSet};

use crate::core::models::{AssetMeta, Folder, FoldersFile, Placement, RecycleFile};

#[derive(Debug, Default)]
pub struct LibraryIndex {
  /// 资源 ID → 元数据
  pub assets: HashMap<String, AssetMeta>,
  /// 资源 ID → 全部 Placement（详情面板"已关联文件夹"反查，PRD 01-3.3）
  pub placements_by_asset: HashMap<String, Vec<Placement>>,
  /// 文件夹 ID → 内部 Placement 列表（中间面板列表）
  pub placements_by_folder: HashMap<String, Vec<Placement>>,
  /// 文件夹 ID → 文件夹
  pub folders: HashMap<String, Folder>,
}

impl LibraryIndex {
  pub fn build(folders_file: &FoldersFile, assets: Vec<AssetMeta>) -> Self {
    let mut index = LibraryIndex {
      assets: assets.into_iter().map(|a| (a.id.clone(), a)).collect(),
      folders: folders_file.folders.iter().map(|f| (f.id.clone(), f.clone())).collect(),
      ..Default::default()
    };
    index.rebuild_placements(&folders_file.items);
    index
  }

  /// 由 Placement 全量重建两张反查表（批量改动 Placement 后统一调用，如冲突决策落库）
  pub fn rebuild_placements(&mut self, items: &[Placement]) {
    self.placements_by_asset.clear();
    self.placements_by_folder.clear();
    for p in items {
      self.placements_by_asset.entry(p.asset.clone()).or_default().push(p.clone());
      self.placements_by_folder.entry(p.folder.clone()).or_default().push(p.clone());
    }
  }

  /// 反查资源关联的全部 Placement（无关联时返回空切片）
  pub fn placements_of(&self, asset_id: &str) -> &[Placement] {
    self.placements_by_asset.get(asset_id).map(Vec::as_slice).unwrap_or(&[])
  }

  /// 按磁盘文件名（originName）反查最新一笔资产（2026-10-23 编辑器贴图直链 by-name 形态，PRD 02-09）：
  /// 等值比较两端小写归一（Windows 文件名大小写不敏感）；排除回收站与 hidden 伴生资产；
  /// 多笔命中取 imported_at 最新
  pub fn latest_by_origin_name(&self, name: &str, recycle: &RecycleFile) -> Option<&AssetMeta> {
    let needle = name.to_lowercase();
    let recycled: HashSet<&str> = recycle.items.iter().map(|i| i.asset.as_str()).collect();
    self
      .assets
      .values()
      .filter(|a| a.origin_name.to_lowercase() == needle)
      .filter(|a| a.hidden != Some(true) && !recycled.contains(a.id.as_str()))
      .max_by(|a, b| imported_at_cmp(&a.imported_at, &b.imported_at))
  }
}

/// imported_at 比较：RFC3339 解析后比时刻（chrono 在依赖内，防跨 offset 字典序失真）；
/// 任一端解析失败回退字符串序（同 offset 字典序可比）
fn imported_at_cmp(a: &str, b: &str) -> std::cmp::Ordering {
  match (chrono::DateTime::parse_from_rfc3339(a), chrono::DateTime::parse_from_rfc3339(b)) {
    (Ok(x), Ok(y)) => x.cmp(&y),
    _ => a.cmp(b),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::models::{FoldersFile, RecycleItem};

  fn meta(id: &str, origin: &str, imported_at: &str) -> AssetMeta {
    serde_json::from_value(serde_json::json!({
      "version": 1, "id": id, "name": id, "originName": origin, "ext": "png", "size": 1,
      "importedAt": imported_at, "modifiedAt": imported_at
    }))
    .unwrap()
  }

  fn index_of(assets: Vec<AssetMeta>) -> LibraryIndex {
    LibraryIndex { assets: assets.into_iter().map(|a| (a.id.clone(), a)).collect(), ..Default::default() }
  }

  fn recycle_of(ids: &[&str]) -> RecycleFile {
    RecycleFile {
      version: 1,
      items: ids
        .iter()
        .map(|id| RecycleItem { asset: id.to_string(), placements: Vec::new(), deleted_at: "t".into() })
        .collect(),
    }
  }

  #[test]
  fn reverse_lookup_by_asset() {
    let json = r#"{
      "version": 1,
      "folders": [
        { "id": "F01", "name": "人物", "parent": null, "order": 0 },
        { "id": "F02", "name": "动画01", "parent": null, "order": 1 }
      ],
      "items": [
        { "asset": "AN-A", "folder": "F01", "name": "男人.fla", "order": 0, "origin": true },
        { "asset": "AN-A", "folder": "F02", "name": "张三.fla", "order": 0 },
        { "asset": "AN-B", "folder": "F02", "name": "走路.fla", "order": 1, "origin": true }
      ]
    }"#;
    let file: FoldersFile = serde_json::from_str(json).unwrap();
    let index = LibraryIndex::build(&file, Vec::new());

    // 一份资源出现在两个文件夹（PRD 01-3 链接语义）
    let placements = index.placements_of("AN-A");
    assert_eq!(placements.len(), 2);
    assert_eq!(placements[0].name, "男人.fla");
    assert_eq!(placements[1].name, "张三.fla");
    assert_eq!(index.placements_of("AN-不存在").len(), 0);

    // 按文件夹分组
    assert_eq!(index.placements_by_folder["F01"].len(), 1);
    assert_eq!(index.placements_by_folder["F02"].len(), 2);
    assert_eq!(index.folders.len(), 2);
  }

  /// by-name 解析：同名两笔取 imported_at 最新（跨 offset 也按真实时刻比）
  #[test]
  fn latest_by_origin_name_picks_newest() {
    let index = index_of(vec![
      meta("AN-OLD", "贴图-a.png", "2026-10-20T10:00:00+08:00"),
      meta("AN-NEW", "贴图-a.png", "2026-10-23T10:00:00+08:00"),
    ]);
    let hit = index.latest_by_origin_name("贴图-a.png", &RecycleFile::new()).unwrap();
    assert_eq!(hit.id, "AN-NEW");
  }

  /// by-name 解析：回收站资产排除（最新一笔在回收站则取次新；全部在回收站 = None）
  #[test]
  fn latest_by_origin_name_excludes_recycled() {
    let index = index_of(vec![
      meta("AN-OLD", "贴图-a.png", "2026-10-20T10:00:00+08:00"),
      meta("AN-NEW", "贴图-a.png", "2026-10-23T10:00:00+08:00"),
    ]);
    let hit = index.latest_by_origin_name("贴图-a.png", &recycle_of(&["AN-NEW"])).unwrap();
    assert_eq!(hit.id, "AN-OLD", "最新一笔在回收站应跳过取次新");
    assert!(index.latest_by_origin_name("贴图-a.png", &recycle_of(&["AN-NEW", "AN-OLD"])).is_none());
  }

  /// by-name 解析：hidden 伴生资产（缩略图代理源）不参与解析
  #[test]
  fn latest_by_origin_name_excludes_hidden() {
    let mut hid = meta("AN-HID", "贴图-a.png", "2026-10-23T10:00:00+08:00");
    hid.hidden = Some(true);
    let index = index_of(vec![meta("AN-VIS", "贴图-a.png", "2026-10-20T10:00:00+08:00"), hid]);
    let hit = index.latest_by_origin_name("贴图-a.png", &RecycleFile::new()).unwrap();
    assert_eq!(hit.id, "AN-VIS");
  }

  /// by-name 解析：大小写归一（Windows 文件名大小写不敏感）
  #[test]
  fn latest_by_origin_name_case_insensitive() {
    let index = index_of(vec![meta("AN-A", "贴图-A.PNG", "2026-10-23T10:00:00+08:00")]);
    let hit = index.latest_by_origin_name("贴图-a.png", &RecycleFile::new()).unwrap();
    assert_eq!(hit.id, "AN-A");
  }

  /// by-name 解析：查无 = None
  #[test]
  fn latest_by_origin_name_miss_returns_none() {
    let index = index_of(vec![meta("AN-A", "贴图-a.png", "2026-10-23T10:00:00+08:00")]);
    assert!(index.latest_by_origin_name("贴图-b.png", &RecycleFile::new()).is_none());
  }
}
