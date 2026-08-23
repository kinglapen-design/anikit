//! relations.json（空间级关系数据，6.2 选择模式一轮，PRD 03_数据链接 06）数据层：
//! 读 / 写 / 开库自检（失效端点剔除）。业务规则（校验 / 显示名快照 / 称谓规整）在
//! services/relations.rs；core 不反向依赖 services（models.rs 头部同约）。

use std::path::{Path, PathBuf};

use crate::core::json_io::{read_json, write_json, DataError};
use crate::core::models::{AssetMeta, FoldersFile, RelationsFile, ARCHIVE_KIND};

/// 空间根关系数据文件（空间级：关系可跨库——ref 带库名，PRD 03_数据链接 06）
pub const RELATIONS_FILE: &str = "relations.json";

/// 空间根 → relations.json 路径（写点与快照记录共用单点推导）
pub fn relations_file(space_root: &Path) -> PathBuf {
  space_root.join(RELATIONS_FILE)
}

/// 读入关系表；缺文件按空表加载（6.2 新增文件，旧空间无此文件不算损坏，同 archives.json 口径）
pub fn load_relations(space_root: &Path) -> Result<RelationsFile, DataError> {
  let path = relations_file(space_root);
  if path.is_file() {
    read_json(&path)
  } else {
    Ok(RelationsFile::new())
  }
}

/// relations.json 单一写点（01_开发规范 第 1 条）：全量原子写
pub fn save_relations(space_root: &Path, file: &RelationsFile) -> Result<(), DataError> {
  write_json(&relations_file(space_root), file)
}

/// 端点目标存在判定（本库范围内）：asset → 资产元数据命中；archive → 档案形态文件夹命中
fn ref_exists(r: &crate::core::models::RelationRef, lib_name: &str, folders: &FoldersFile, assets: &[AssetMeta]) -> bool {
  if r.lib != lib_name {
    return true; // 跨库端点：目标库未打开无法校验，保留（显示侧用名称快照兜底）
  }
  match r.kind.as_str() {
    "asset" => assets.iter().any(|a| a.id == r.id),
    "archive" => folders.folders.iter().any(|f| f.id == r.id && f.kind.as_deref() == Some(ARCHIVE_KIND)),
    _ => false, // 未知 kind 视为失效（防御：手编数据）
  }
}

/// 开库自检（core::library::open 调用）：剔除 from / to 任一端点指向"本库且目标已不存在"的关系。
/// 返回是否有改动（有改动由调用方落盘——同 fix_archives 自检段风格）
pub fn prune_relations(file: &mut RelationsFile, lib_name: &str, folders: &FoldersFile, assets: &[AssetMeta]) -> bool {
  let before = file.relations.len();
  file.relations.retain(|r| ref_exists(&r.from, lib_name, folders, assets) && ref_exists(&r.to, lib_name, folders, assets));
  before != file.relations.len()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::models::{Folder, Relation, RelationRef};

  fn asset_meta(id: &str) -> AssetMeta {
    serde_json::from_value(serde_json::json!({
      "version": 1, "id": id, "name": id, "originName": format!("{id}.png"), "ext": "png", "size": 1,
      "importedAt": "2026-08-23T00:00:00+08:00", "modifiedAt": "2026-08-23T00:00:00+08:00"
    }))
    .unwrap()
  }

  fn archive_folder(id: &str) -> Folder {
    Folder {
      id: id.into(),
      name: id.into(),
      parent: None,
      block: None,
      icon: None,
      kind: Some(ARCHIVE_KIND.into()),
      color: None,
      groups: None,
      order: 0,
    }
  }

  fn rref(lib: &str, kind: &str, id: &str) -> RelationRef {
    RelationRef { lib: lib.into(), kind: kind.into(), id: id.into(), name: id.into() }
  }

  fn rel(id: &str, from: RelationRef, to: RelationRef) -> Relation {
    Relation { id: id.into(), from, to, kind: "free".into(), label: "父亲".into(), reverse_label: None }
  }

  /// 自检剔除口径：本库失效端点的关系剔除；本库有效、跨库端点（无法校验）保留
  #[test]
  fn prune_drops_invalid_local_keeps_cross_lib() {
    let folders = FoldersFile { version: 1, blocks: Vec::new(), folders: vec![archive_folder("F-OK")], items: Vec::new() };
    let assets = vec![asset_meta("AN-OK")];
    let mut file = RelationsFile::new();
    file.relations = vec![
      rel("RL-1", rref("本库", "asset", "AN-OK"), rref("本库", "archive", "F-OK")), // 全有效
      rel("RL-2", rref("本库", "asset", "AN-没了"), rref("本库", "archive", "F-OK")), // from 失效
      rel("RL-3", rref("本库", "asset", "AN-OK"), rref("本库", "archive", "F-没了")), // to 失效
      rel("RL-4", rref("他库", "asset", "AN-X"), rref("本库", "asset", "AN-OK")), // 跨库 from 保留
      rel("RL-5", rref("本库", "folder", "F-OK"), rref("本库", "asset", "AN-OK")), // 未知 kind 失效
    ];
    assert!(prune_relations(&mut file, "本库", &folders, &assets));
    let ids: Vec<&str> = file.relations.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, vec!["RL-1", "RL-4"]);
    // 再跑一遍无改动（幂等）
    assert!(!prune_relations(&mut file, "本库", &folders, &assets));
  }
}
