//! 关系服务（6.2 选择模式一轮，PRD 03_数据链接 02/06）：文件与档案之间的关系建立 / 编辑 / 删除。
//! 数据落空间根 relations.json（core::relations 单一写点，原子写；关系可跨库，ref 携库名 + 显示名快照）。
//! 本轮 kind 恒 free（自由关系）；voice 音色随 6.2 链接音频轮接入。
//! 反向称谓（reverse_label）= 用户在选择模式备选区右框手动填写（2026-08-23 用户定：不做词典推导），
//! 空串规整为 None（反向显示回退 label）。

use std::path::Path;

use crate::core::library::Library;
use crate::core::models::{
  new_relation_id, Relation, RelationRef, ARCHIVE_KIND, RELATION_KIND_FREE,
};
use crate::core::relations::{load_relations, save_relations};

/// 端点校验 + 显示名快照：asset → 资产元数据名；archive → 档案文件夹名。
/// 目标不存在给中文人话（规范第 6 条）
fn resolve_ref(lib: &Library, lib_name: &str, kind: &str, id: &str) -> Result<RelationRef, String> {
  match kind {
    "asset" => {
      let meta = lib.index.assets.get(id).ok_or_else(|| "目标文件不存在，可能已被删除".to_string())?;
      Ok(RelationRef { lib: lib_name.to_string(), kind: kind.to_string(), id: id.to_string(), name: meta.name.clone() })
    }
    "archive" => {
      let f = lib.index.folders.get(id).ok_or_else(|| "目标档案不存在，可能已被删除".to_string())?;
      if f.kind.as_deref() != Some(ARCHIVE_KIND) {
        return Err("目标不是档案".to_string());
      }
      Ok(RelationRef { lib: lib_name.to_string(), kind: kind.to_string(), id: id.to_string(), name: f.name.clone() })
    }
    _ => Err("未知的关系目标类型".to_string()),
  }
}

/// 称谓规整：trim；空串 → None（reverse 未填写的存形态）
fn normalize_label(s: &str) -> Option<String> {
  let t = s.trim();
  if t.is_empty() { None } else { Some(t.to_string()) }
}

/// 同一端点判定（同库同 kind 同 id；跨库不算同一对象——选择模式本轮只能选当前库目标）
fn same_ref(a: (&str, &str), b: (&str, &str)) -> bool {
  a.0 == b.0 && a.1 == b.1
}

/// 建立关系。label（to 对 from 的称谓）trim 后必填；from / to 不得同一端点。
/// 返回写入的完整记录（含生成 id 与显示名快照，前端就地插入免重拉）
pub fn add(
  space_root: &Path,
  lib: &Library,
  from: (&str, &str),
  to: (&str, &str),
  label: &str,
  reverse_label: Option<&str>,
) -> Result<Relation, String> {
  let label = normalize_label(label).ok_or_else(|| "请填写关系称谓".to_string())?;
  if same_ref(from, to) {
    return Err("不能和自身建立关系".to_string());
  }
  let lib_name = lib.root.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
  let from_ref = resolve_ref(lib, &lib_name, from.0, from.1)?;
  let to_ref = resolve_ref(lib, &lib_name, to.0, to.1)?;
  let rel = Relation {
    id: new_relation_id(),
    from: from_ref,
    to: to_ref,
    kind: RELATION_KIND_FREE.to_string(),
    label,
    reverse_label: reverse_label.and_then(normalize_label),
  };
  let mut file = load_relations(space_root).map_err(|e| e.to_string())?;
  file.relations.push(rel.clone());
  save_relations(space_root, &file).map_err(|e| e.to_string())?;
  Ok(rel)
}

/// 编辑关系：改称谓（label 必填 / reverse_label 空串规整 None）+ 可选换绑端点（选择模式二次修改，
/// 03 数据 item 稿 §4：点击文件进入选择模式换绑）——new_to / new_from 二选一（被换端 = 当前详情
/// 目标的对方：目标视角在 from 侧换 to、在 to 侧换 from）。返回是否有改动（无改动未落盘）
pub fn update(
  space_root: &Path,
  lib: &Library,
  id: &str,
  label: &str,
  reverse_label: Option<&str>,
  new_to: Option<(&str, &str)>,
  new_from: Option<(&str, &str)>,
) -> Result<bool, String> {
  let label = normalize_label(label).ok_or_else(|| "请填写关系称谓".to_string())?;
  let mut file = load_relations(space_root).map_err(|e| e.to_string())?;
  let Some(idx) = file.relations.iter().position(|r| r.id == id) else {
    return Err("关系不存在，可能已被删除".to_string());
  };
  let lib_name = lib.root.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
  // 换绑端点：校验存在 + 不得与另一端点同端点；resolve 出新快照
  let cur = file.relations[idx].clone();
  let next_to = match new_to {
    Some((kind, tid)) => {
      if same_ref((kind, tid), (cur.from.kind.as_str(), cur.from.id.as_str())) {
        return Err("不能和自身建立关系".to_string());
      }
      resolve_ref(lib, &lib_name, kind, tid)?
    }
    None => cur.to.clone(),
  };
  let next_from = match new_from {
    Some((kind, fid)) => {
      if same_ref((kind, fid), (next_to.kind.as_str(), next_to.id.as_str())) {
        return Err("不能和自身建立关系".to_string());
      }
      resolve_ref(lib, &lib_name, kind, fid)?
    }
    None => cur.from.clone(),
  };
  let next_reverse = reverse_label.and_then(normalize_label);
  if cur.label == label && cur.reverse_label == next_reverse && cur.to == next_to && cur.from == next_from {
    return Ok(false);
  }
  let rel = &mut file.relations[idx];
  rel.label = label;
  rel.reverse_label = next_reverse;
  rel.to = next_to;
  rel.from = next_from;
  save_relations(space_root, &file).map_err(|e| e.to_string())?;
  Ok(true)
}

/// 删除关系（详情关系 item ×）
pub fn remove(space_root: &Path, id: &str) -> Result<(), String> {
  let mut file = load_relations(space_root).map_err(|e| e.to_string())?;
  let before = file.relations.len();
  file.relations.retain(|r| r.id != id);
  if file.relations.len() == before {
    return Err("关系不存在，可能已被删除".to_string());
  }
  save_relations(space_root, &file).map_err(|e| e.to_string())?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::{read_json, write_json};
  use crate::core::models::{AssetMeta, Folder, RelationsFile};
  use crate::core::paths;
  use crate::core::relations::RELATIONS_FILE;

  struct Harness {
    _dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = crate::core::library::create_test(dir.path());
    Harness { _dir: dir, lib }
  }

  fn lib_name(lib: &Library) -> String {
    lib.root.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
  }

  fn add_asset(lib: &mut Library, id: &str, name: &str) {
    let meta: AssetMeta = serde_json::from_value(serde_json::json!({
      "version": 1, "id": id, "name": name, "originName": format!("{name}.png"), "ext": "png", "size": 100,
      "importedAt": "2026-08-23T00:00:00+08:00", "modifiedAt": "2026-08-23T00:00:00+08:00"
    }))
    .unwrap();
    lib.index.assets.insert(id.into(), meta);
    let dir = paths::info_dir(&lib.assets_dir, id);
    std::fs::create_dir_all(&dir).unwrap();
    write_json(&dir.join("metadata.json"), lib.index.assets.get(id).unwrap()).unwrap();
  }

  fn add_archive(lib: &mut Library, id: &str, name: &str) {
    let folder = Folder {
      id: id.into(),
      name: name.into(),
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
  }

  /// 建立：全字段（含显示名快照 / 库名）落盘往返；reverse 空串规整 None 不写字段
  #[test]
  fn add_persists_full_record() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "雪牧.fla");
    add_archive(&mut h.lib, "F-B", "雪天");
    let rel = add(
      &h.lib.space_root,
      &h.lib,
      ("asset", "AN-A"),
      ("archive", "F-B"),
      " 父亲 ",
      Some("儿子"),
    )
    .unwrap();
    assert_eq!(rel.kind, RELATION_KIND_FREE);
    assert!(rel.id.starts_with("RL"));
    assert_eq!(rel.from, RelationRef { lib: lib_name(&h.lib), kind: "asset".into(), id: "AN-A".into(), name: "雪牧.fla".into() });
    assert_eq!(rel.to, RelationRef { lib: lib_name(&h.lib), kind: "archive".into(), id: "F-B".into(), name: "雪天".into() });
    assert_eq!(rel.label, "父亲", "trim 规整");
    assert_eq!(rel.reverse_label.as_deref(), Some("儿子"));
    // 落盘回读一致
    let disk: RelationsFile = read_json(&h.lib.space_root.join(RELATIONS_FILE)).unwrap();
    assert_eq!(disk.relations, vec![rel]);
    // reverse 空串 → None（序列化不写字段）
    let rel2 = add(&h.lib.space_root, &h.lib, ("archive", "F-B"), ("asset", "AN-A"), "儿女", Some("  ")).unwrap();
    assert_eq!(rel2.reverse_label, None);
    assert!(!serde_json::to_string(&rel2).unwrap().contains("reverseLabel"));
  }

  /// 校验：空称谓 / 自身 / 目标不存在 / 普通文件夹冒充档案
  #[test]
  fn add_validates() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "雪牧.fla");
    add_archive(&mut h.lib, "F-B", "雪天");
    assert!(add(&h.lib.space_root, &h.lib, ("asset", "AN-A"), ("archive", "F-B"), "  ", None).is_err());
    assert!(add(&h.lib.space_root, &h.lib, ("asset", "AN-A"), ("asset", "AN-A"), "x", None).is_err(), "自身");
    assert!(add(&h.lib.space_root, &h.lib, ("asset", "AN-不存在"), ("archive", "F-B"), "x", None).is_err());
    assert!(add(&h.lib.space_root, &h.lib, ("asset", "AN-A"), ("archive", "F-不存在"), "x", None).is_err());
    // 普通文件夹（无 kind）作 archive 端点报错
    let plain = Folder {
      id: "F-P".into(),
      name: "普通".into(),
      parent: None,
      block: None,
      icon: None,
      kind: None,
      color: None,
      groups: None,
      order: 1,
    };
    h.lib.folders.folders.push(plain.clone());
    h.lib.index.folders.insert("F-P".into(), plain);
    assert!(add(&h.lib.space_root, &h.lib, ("asset", "AN-A"), ("archive", "F-P"), "x", None).is_err());
    // 全部失败 = 不落盘（文件都不存在）
    assert!(!h.lib.space_root.join(RELATIONS_FILE).exists());
  }

  /// 编辑：改称谓（含 reverse 清空）+ 换绑 to；无改动 = false 不落盘
  #[test]
  fn update_label_and_retarget() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "雪牧.fla");
    add_asset(&mut h.lib, "AN-C", "阿拉蕾.fla");
    add_archive(&mut h.lib, "F-B", "雪天");
    let rel = add(&h.lib.space_root, &h.lib, ("asset", "AN-A"), ("archive", "F-B"), "父亲", Some("儿子")).unwrap();

    // 无改动 = false
    assert!(!update(&h.lib.space_root, &h.lib, &rel.id, "父亲", Some("儿子"), None, None).unwrap());
    // 改称谓 + reverse 清空
    assert!(update(&h.lib.space_root, &h.lib, &rel.id, "老爸", Some(""), None, None).unwrap());
    let disk: RelationsFile = read_json(&h.lib.space_root.join(RELATIONS_FILE)).unwrap();
    assert_eq!(disk.relations[0].label, "老爸");
    assert_eq!(disk.relations[0].reverse_label, None);
    // 换绑 to（archive → asset，快照跟着新目标）
    assert!(update(&h.lib.space_root, &h.lib, &rel.id, "老爸", None, Some(("asset", "AN-C")), None).unwrap());
    let disk: RelationsFile = read_json(&h.lib.space_root.join(RELATIONS_FILE)).unwrap();
    assert_eq!(disk.relations[0].to.id, "AN-C");
    assert_eq!(disk.relations[0].to.name, "阿拉蕾.fla");
    // 换绑 from（对方端同径）
    assert!(update(&h.lib.space_root, &h.lib, &rel.id, "老爸", None, None, Some(("archive", "F-B"))).unwrap());
    let disk: RelationsFile = read_json(&h.lib.space_root.join(RELATIONS_FILE)).unwrap();
    assert_eq!(disk.relations[0].from.id, "F-B");
    // 换绑到另一端点自身报错；不存在的关系报错
    assert!(update(&h.lib.space_root, &h.lib, &rel.id, "老爸", None, Some(("archive", "F-B")), None).is_err());
    assert!(update(&h.lib.space_root, &h.lib, "RL-不存在", "x", None, None, None).is_err());
  }

  /// 删除：存在删除落盘；不存在报错
  #[test]
  fn remove_deletes() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "雪牧.fla");
    add_archive(&mut h.lib, "F-B", "雪天");
    let rel = add(&h.lib.space_root, &h.lib, ("asset", "AN-A"), ("archive", "F-B"), "父亲", None).unwrap();
    remove(&h.lib.space_root, &rel.id).unwrap();
    let disk: RelationsFile = read_json(&h.lib.space_root.join(RELATIONS_FILE)).unwrap();
    assert!(disk.relations.is_empty());
    assert!(remove(&h.lib.space_root, &rel.id).is_err(), "重复删除报错");
  }
}
