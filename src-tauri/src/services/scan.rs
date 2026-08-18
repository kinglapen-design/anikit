//! 重复文件扫描（只读）：按存量 metadata.hash 分组，同 hash ≥ 2 个不同资产 = 重复组。
//! 同一资产多条 Placement 不算重复；扫描范围 = 锚点过滤（组内任一资产命中范围则整组返回）。

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::core::library::Library;
use crate::core::models::{AssetMeta, Placement, ROOT_FOLDER_ID};
use crate::services::views::{thumb_path, ThumbKind};

/// 重复组内单资产的 Placement 摘要（前端合并组装 PlacementRef 用）
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupPlacement {
  pub folder_id: String,
  pub origin: bool,
}

/// 重复组内单资产视图（origin Placement 决定显示名与所在文件夹）
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupAssetView {
  pub asset_id: String,
  pub name: String,
  pub folder_id: String,
  pub folder_name: String,
  pub w: Option<u32>,
  pub h: Option<u32>,
  pub ext: String,
  pub size: u64,
  pub thumb_path: Option<String>,
  pub thumb_kind: Option<ThumbKind>,
  pub placements: Vec<DupPlacement>,
}

/// 重复组：同 hash 的资产列表（assets[0] = 导入最早，默认保留者）
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupGroupView {
  pub hash: String,
  pub size: u64,
  pub assets: Vec<DupAssetView>,
}

/// 扫描范围：整库 / 指定文件夹 / 指定资产集（锚点过滤：组内任一资产命中则整组返回）
/// rename_all = 变体名（kind 值）；rename_all_fields = 变体字段（folderId / assetIds，
/// 2026-08-13 实机修复：rename_all 不管字段名，Folder 曾因等 folder_id 报 missing field）
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ScanScope {
  All,
  Folder { folder_id: String },
  Assets { asset_ids: Vec<String> },
}

/// origin Placement（无 origin 取第一条兜底）；无 Placement 返回 None
fn origin_placement(placements: &[Placement]) -> Option<&Placement> {
  placements.iter().find(|p| p.origin).or_else(|| placements.first())
}

/// 文件夹显示名：root / 未知 ID → "未分类"
fn folder_name(lib: &Library, folder_id: &str) -> String {
  if folder_id == ROOT_FOLDER_ID {
    return "未分类".to_string();
  }
  lib.index.folders.get(folder_id).map(|f| f.name.clone()).unwrap_or_else(|| "未分类".to_string())
}

pub fn scan_duplicates(lib: &Library, scope: &ScanScope, icons: &HashSet<String>) -> Vec<DupGroupView> {
  // 按 hash 分组：空 hash 跳过；无 Placement 的资产不作候选也不作锚点
  let mut by_hash: HashMap<&str, Vec<&AssetMeta>> = HashMap::new();
  for meta in lib.index.assets.values() {
    if meta.hash.is_empty() || lib.index.placements_of(&meta.id).is_empty() {
      continue;
    }
    by_hash.entry(meta.hash.as_str()).or_default().push(meta);
  }

  let anchor_hit = |meta: &AssetMeta| match scope {
    ScanScope::All => true,
    ScanScope::Folder { folder_id } => {
      origin_placement(lib.index.placements_of(&meta.id)).is_some_and(|p| p.folder == *folder_id)
    }
    ScanScope::Assets { asset_ids } => asset_ids.contains(&meta.id),
  };

  let mut groups: Vec<DupGroupView> = Vec::new();
  for (hash, mut metas) in by_hash {
    if metas.len() < 2 || !metas.iter().any(|m| anchor_hit(m)) {
      continue;
    }
    // 组内 imported_at 升序（RFC3339 字典序即时间序，最早 = 默认保留者排首），平手按 id
    metas.sort_by(|a, b| a.imported_at.cmp(&b.imported_at).then_with(|| a.id.cmp(&b.id)));
    groups.push(DupGroupView {
      hash: hash.to_string(),
      size: metas[0].size,
      assets: metas.iter().map(|m| dup_asset_view(lib, m, icons)).collect(),
    });
  }
  // 组间：可节约空间 (n-1) * size 降序，平手按首资产名升序
  groups.sort_by(|a, b| {
    let saving = |g: &DupGroupView| (g.assets.len() as u64 - 1) * g.size;
    saving(b).cmp(&saving(a)).then_with(|| a.assets[0].name.cmp(&b.assets[0].name))
  });
  groups
}

/// 资产 + origin Placement → 视图（缩略图口径与 placement_view 一致）
fn dup_asset_view(lib: &Library, meta: &AssetMeta, icons: &HashSet<String>) -> DupAssetView {
  let placements = lib.index.placements_of(&meta.id);
  let origin = origin_placement(placements).expect("候选资产必有 Placement");
  let key = if meta.ext.is_empty() { "file" } else { meta.ext.as_str() };
  let thumb_kind = if meta.thumbnail.is_some() {
    Some(ThumbKind::Asset)
  } else if icons.contains(key) {
    Some(ThumbKind::FormatIcon)
  } else {
    None
  };
  DupAssetView {
    asset_id: meta.id.clone(),
    name: origin.name.clone(),
    folder_id: origin.folder.clone(),
    folder_name: folder_name(lib, &origin.folder),
    w: meta.width,
    h: meta.height,
    ext: meta.ext.clone(),
    size: meta.size,
    thumb_path: thumb_path(lib, meta, icons),
    thumb_kind,
    placements: placements
      .iter()
      .map(|p| DupPlacement { folder_id: p.folder.clone(), origin: p.origin })
      .collect(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library;
  use crate::core::models::{Folder, Placement};

  // scope 反序列化回归（2026-08-13 实机报错：rename_all 不改字段名，folderId 曾被等成 folder_id）
  #[test]
  fn scope_deserializes_camel_case_fields() {
    let all: ScanScope = serde_json::from_str(r#"{"kind":"all"}"#).unwrap();
    assert_eq!(all, ScanScope::All);
    let folder: ScanScope = serde_json::from_str(r#"{"kind":"folder","folderId":"f1"}"#).unwrap();
    assert_eq!(folder, ScanScope::Folder { folder_id: "f1".into() });
    let assets: ScanScope = serde_json::from_str(r#"{"kind":"assets","assetIds":["a1","a2"]}"#).unwrap();
    assert_eq!(assets, ScanScope::Assets { asset_ids: vec!["a1".into(), "a2".into()] });
  }

  struct Harness {
    _dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { _dir: dir, lib }
  }

  fn add_folder(lib: &mut Library, id: &str, name: &str) {
    let folder = Folder { id: id.into(), name: name.into(), parent: None, block: None, icon: None, kind: None, color: None, order: 0 };
    lib.folders.folders.push(folder.clone());
    lib.index.folders.insert(id.into(), folder);
  }

  /// 造资产（纯内存）+ 一条 origin Placement
  fn add_asset(lib: &mut Library, id: &str, name: &str, hash: &str, size: u64, imported_at: &str, folder: &str) {
    lib.index.assets.insert(
      id.into(),
      AssetMeta {
        version: 1,
        id: id.into(),
        name: name.into(),
        origin_name: name.into(),
        ext: "png".into(),
        size,
        hash: hash.into(),
        width: None,
        height: None,
        duration: None,
        bpm: None,
        rating: 0,
        links: Vec::new(),
        imported_at: imported_at.into(),
        imported_from: None,
        modified_at: imported_at.into(),
        tags: Vec::new(),
        note: String::new(),
        note_count: None,
        thumbnail: None,
        fps: None,
        frame_count: None,
        thumb_frame: None,
        r#type: None,
        source: None,
        start: None,
        end: None,
      },
    );
    add_placement(lib, id, name, folder, true);
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

  fn scan(lib: &Library, scope: &ScanScope) -> Vec<DupGroupView> {
    scan_duplicates(lib, scope, &HashSet::new())
  }

  #[test]
  fn groups_only_when_hash_shared_by_two_assets() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "甲.png", "sha256:x", 100, "2026-08-01T10:00:00+08:00", "root");
    add_asset(&mut h.lib, "AN-B", "乙.png", "sha256:x", 100, "2026-08-02T10:00:00+08:00", "root");
    add_asset(&mut h.lib, "AN-C", "丙.png", "sha256:y", 50, "2026-08-03T10:00:00+08:00", "root");
    // 空 hash 不参与
    add_asset(&mut h.lib, "AN-D", "丁.png", "", 50, "2026-08-04T10:00:00+08:00", "root");

    let groups = scan(&h.lib, &ScanScope::All);
    assert_eq!(groups.len(), 1, "独 hash / 空 hash 不成组");
    assert_eq!(groups[0].hash, "sha256:x");
    assert_eq!(groups[0].assets.len(), 2);
  }

  #[test]
  fn same_asset_multiple_placements_not_duplicated() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "文件夹");
    add_asset(&mut h.lib, "AN-A", "甲.png", "sha256:x", 100, "2026-08-01T10:00:00+08:00", "root");
    add_placement(&mut h.lib, "AN-A", "链接.png", "F01", false);

    assert!(scan(&h.lib, &ScanScope::All).is_empty(), "同一资产多条 Placement 不算重复");
  }

  #[test]
  fn folder_scope_returns_whole_group_when_anchor_hits() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "FA", "夹A");
    add_folder(&mut h.lib, "FB", "夹B");
    add_asset(&mut h.lib, "AN-A", "甲.png", "sha256:x", 100, "2026-08-01T10:00:00+08:00", "FA");
    add_asset(&mut h.lib, "AN-B", "乙.png", "sha256:x", 100, "2026-08-02T10:00:00+08:00", "FB");

    let groups = scan(&h.lib, &ScanScope::Folder { folder_id: "FA".into() });
    assert_eq!(groups.len(), 1, "锚点命中则整组返回");
    let ids: Vec<&str> = groups[0].assets.iter().map(|a| a.asset_id.as_str()).collect();
    assert_eq!(ids, ["AN-A", "AN-B"], "甲（FA）与乙（FB）都在组内");
    // 锚点不命中则整组不返回
    assert!(scan(&h.lib, &ScanScope::Folder { folder_id: "F-无关".into() }).is_empty());
  }

  #[test]
  fn assets_scope_filters_by_selected_ids() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "甲.png", "sha256:x", 100, "2026-08-01T10:00:00+08:00", "root");
    add_asset(&mut h.lib, "AN-B", "乙.png", "sha256:x", 100, "2026-08-02T10:00:00+08:00", "root");
    add_asset(&mut h.lib, "AN-C", "丙.png", "sha256:y", 50, "2026-08-03T10:00:00+08:00", "root");

    let hit = scan(&h.lib, &ScanScope::Assets { asset_ids: vec!["AN-B".into()] });
    assert_eq!(hit.len(), 1, "选中乙则其所在组返回");
    assert_eq!(hit[0].hash, "sha256:x");

    let miss = scan(&h.lib, &ScanScope::Assets { asset_ids: vec!["AN-C".into()] });
    assert!(miss.is_empty(), "选中无关资产不返回该组");
  }

  #[test]
  fn group_sorted_by_imported_at_asc() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-LATE", "晚.png", "sha256:x", 100, "2026-08-02T10:00:00+08:00", "root");
    add_asset(&mut h.lib, "AN-EARLY", "早.png", "sha256:x", 100, "2026-08-01T10:00:00+08:00", "root");

    let groups = scan(&h.lib, &ScanScope::All);
    assert_eq!(groups[0].assets[0].asset_id, "AN-EARLY", "导入最早 = 默认保留者排首");
    assert_eq!(groups[0].assets[1].asset_id, "AN-LATE");
  }
}
