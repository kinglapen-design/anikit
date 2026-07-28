//! 内存索引（PRD 06-03）：打开库时建立，运行期查询全部走内存。
//! 写操作先落盘再更新索引（01_开发规范 第 7 条）。

use std::collections::HashMap;

use crate::core::models::{AssetMeta, Folder, FoldersFile, Placement};

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
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::models::FoldersFile;

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
}
