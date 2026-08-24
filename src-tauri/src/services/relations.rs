//! 关系服务（6.2 选择模式一轮，PRD 03_数据链接 02/06）：文件与档案之间的关系建立 / 编辑 / 删除。
//! 数据落空间根 relations.json（core::relations 单一写点，原子写；关系可跨库，ref 携库名 + 显示名快照）。
//! 本轮 kind 恒 free（自由关系）；voice 音色随 6.2 链接音频轮接入。
//! 反向称谓（reverse_label）= 用户在选择模式备选区右框手动填写（2026-08-23 用户定：不做词典推导），
//! 空串规整为 None（反向显示回退 label）。

use std::path::Path;

use crate::core::library::Library;
use crate::core::models::{
  new_relation_id, Relation, RelationRef, ARCHIVE_KIND, RELATION_KIND_FREE, RELATION_KIND_VOICE,
};
use crate::core::relations::{load_relations, save_relations};
use crate::services::media::AUDIO_EXTS;

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

/// 批量建立关系（选择模式多选确认，2026-08-24 用户定：选一个 = 追加一条而非切换——
/// 一次为同一发起方加父亲/儿子/母亲/专武多条）。**任一失败全不写**（批量原子：校验全前置，一次落盘）。
/// kind = free：label（to 对 from 的称谓）trim 必填、to ≠ from；
/// kind = voice（6.2 链接音频轮，PRD 03 数据链接 02 音色）：无称谓——label 不校验、规整后空存空串，
/// to 必须是音频 asset（mp3/wav/wma），from 可是 asset / archive（档案也能链音频，区块体系共用），
/// 但 from 不得为音频 asset（2026-08-24 用户定方案 1：voice 恒单向 使用者→音频，防"音频的音频"递归）。
/// 返回写入的完整记录组（含生成 id 与显示名快照，前端就地插入免重拉）
pub fn add_batch(
  space_root: &Path,
  lib: &Library,
  from: (&str, &str),
  kind: &str,
  items: Vec<(&str, &str, &str, Option<&str>)>, // (to_kind, to_id, label, reverse_label)
) -> Result<Vec<Relation>, String> {
  if items.is_empty() {
    return Err("请先选择文件".to_string());
  }
  if kind != RELATION_KIND_FREE && kind != RELATION_KIND_VOICE {
    return Err("未知的关系类型".to_string());
  }
  let lib_name = lib.root.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
  let from_ref = resolve_ref(lib, &lib_name, from.0, from.1)?;
  // 方案 1（2026-08-24 用户定）：voice 恒单向 使用者→音频——from 是音频 asset 即"音频的音频"递归，拒
  if kind == RELATION_KIND_VOICE && from.0 == "asset" {
    if let Some(meta) = lib.index.assets.get(from.1) {
      if AUDIO_EXTS.contains(&meta.ext.to_lowercase().as_str()) {
        return Err(format!("「{}」是音频文件，不能作为使用方", meta.name));
      }
    }
  }
  let mut rels: Vec<Relation> = Vec::with_capacity(items.len());
  for (to_kind, to_id, label, reverse) in items {
    let label = if kind == RELATION_KIND_VOICE {
      String::new() // 音色无称谓（PRD：双向显示"使用的音频 / 在哪些文件里使用过"）
    } else {
      normalize_label(label).ok_or_else(|| "请填写关系称谓".to_string())?
    };
    if same_ref(from, (to_kind, to_id)) {
      return Err("不能和自身建立关系".to_string());
    }
    if kind == RELATION_KIND_VOICE {
      let meta = lib.index.assets.get(to_id).ok_or_else(|| "目标文件不存在，可能已被删除".to_string())?;
      if to_kind != "asset" || !AUDIO_EXTS.contains(&meta.ext.to_lowercase().as_str()) {
        return Err(format!("「{}」不是音频文件", meta.name));
      }
    }
    let to_ref = resolve_ref(lib, &lib_name, to_kind, to_id)?;
    rels.push(Relation {
      id: new_relation_id(),
      from: from_ref.clone(),
      to: to_ref,
      kind: kind.to_string(),
      label,
      reverse_label: if kind == RELATION_KIND_VOICE { None } else { reverse.and_then(normalize_label) },
    });
  }
  let mut file = load_relations(space_root).map_err(|e| e.to_string())?;
  file.relations.extend(rels.iter().cloned());
  save_relations(space_root, &file).map_err(|e| e.to_string())?;
  Ok(rels)
}

/// 编辑关系称谓（详情关系 item 点称谓框二次修改，03 数据 item 稿 §4）：
/// label 必填 / reverse_label 空串规整 None。返回是否有改动（无改动未落盘）。
///（换绑端点 2026-08-24 用户定废除——"要换就重新再添加"，改称谓保留本命令）
pub fn update(space_root: &Path, id: &str, label: &str, reverse_label: Option<&str>) -> Result<bool, String> {
  let label = normalize_label(label).ok_or_else(|| "请填写关系称谓".to_string())?;
  let mut file = load_relations(space_root).map_err(|e| e.to_string())?;
  let Some(rel) = file.relations.iter_mut().find(|r| r.id == id) else {
    return Err("关系不存在，可能已被删除".to_string());
  };
  let next_reverse = reverse_label.and_then(normalize_label);
  if rel.label == label && rel.reverse_label == next_reverse {
    return Ok(false);
  }
  rel.label = label;
  rel.reverse_label = next_reverse;
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
    let ext = name.rsplit('.').next().unwrap_or_default().to_lowercase();
    let meta: AssetMeta = serde_json::from_value(serde_json::json!({
      "version": 1, "id": id, "name": name, "originName": format!("{name}.png"), "ext": ext, "size": 100,
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

  /// 批量建立：全字段（含显示名快照 / 库名）落盘往返；reverse 空串规整 None 不写字段
  #[test]
  fn add_batch_persists_full_records() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "雪牧.fla");
    add_asset(&mut h.lib, "AN-C", "阿拉蕾.fla");
    add_archive(&mut h.lib, "F-B", "雪天");
    let rels = add_batch(
      &h.lib.space_root,
      &h.lib,
      ("asset", "AN-A"),
      RELATION_KIND_FREE,
      vec![("archive", "F-B", " 父亲 ", Some("儿子")), ("asset", "AN-C", "专武", Some("  "))],
    )
    .unwrap();
    assert_eq!(rels.len(), 2, "一次多条");
    assert!(rels[0].id.starts_with("RL"));
    assert_eq!(rels[0].from, RelationRef { lib: lib_name(&h.lib), kind: "asset".into(), id: "AN-A".into(), name: "雪牧.fla".into() });
    assert_eq!(rels[0].to, RelationRef { lib: lib_name(&h.lib), kind: "archive".into(), id: "F-B".into(), name: "雪天".into() });
    assert_eq!(rels[0].kind, RELATION_KIND_FREE);
    assert_eq!(rels[0].label, "父亲", "trim 规整");
    assert_eq!(rels[0].reverse_label.as_deref(), Some("儿子"));
    assert_eq!(rels[1].label, "专武");
    assert_eq!(rels[1].reverse_label, None, "空串规整 None");
    assert!(!serde_json::to_string(&rels[1]).unwrap().contains("reverseLabel"));
    // 落盘回读一致
    let disk: RelationsFile = read_json(&h.lib.space_root.join(RELATIONS_FILE)).unwrap();
    assert_eq!(disk.relations, rels);
  }

  /// 批量原子：任一失败全不写；空清单 / 空称谓 / 自身 / 目标不存在 / 普通文件夹冒充档案
  #[test]
  fn add_batch_validates_atomically() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "雪牧.fla");
    add_archive(&mut h.lib, "F-B", "雪天");
    assert!(add_batch(&h.lib.space_root, &h.lib, ("asset", "AN-A"), RELATION_KIND_FREE, vec![]).is_err(), "空清单");
    assert!(add_batch(&h.lib.space_root, &h.lib, ("asset", "AN-A"), RELATION_KIND_FREE, vec![("archive", "F-B", "  ", None)]).is_err(), "空称谓");
    assert!(add_batch(&h.lib.space_root, &h.lib, ("asset", "AN-A"), RELATION_KIND_FREE, vec![("asset", "AN-A", "x", None)]).is_err(), "自身");
    // 第二条失效 → 整批不落盘
    assert!(
      add_batch(
        &h.lib.space_root,
        &h.lib,
        ("asset", "AN-A"),
        RELATION_KIND_FREE,
        vec![("archive", "F-B", "父亲", None), ("asset", "AN-不存在", "儿子", None)],
      )
      .is_err()
    );
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
    assert!(add_batch(&h.lib.space_root, &h.lib, ("asset", "AN-A"), RELATION_KIND_FREE, vec![("archive", "F-P", "x", None)]).is_err());
    // 未知 kind 报错
    assert!(add_batch(&h.lib.space_root, &h.lib, ("asset", "AN-A"), "unknown", vec![("archive", "F-B", "x", None)]).is_err());
    // 全部失败 = 不落盘（文件都不存在）
    assert!(!h.lib.space_root.join(RELATIONS_FILE).exists());
  }

  /// 音色（voice，6.2 链接音频轮）：无称谓——label 不校验存空串、reverse 恒 None；
  /// to 必须音频 asset（非音频 / 档案作 to 报错），from 可以是档案；批量原子同 free
  #[test]
  fn add_batch_voice() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "雪牧.fla");
    add_asset(&mut h.lib, "AN-V", "主题曲.mp3");
    add_asset(&mut h.lib, "AN-W", "插曲.WMA"); // 扩展名大写也认（小写比较）
    add_archive(&mut h.lib, "F-B", "雪天");
    // from = 档案也合法（区块体系共用）；label 传了也忽略
    let rels = add_batch(
      &h.lib.space_root,
      &h.lib,
      ("archive", "F-B"),
      RELATION_KIND_VOICE,
      vec![("asset", "AN-V", "某称谓", Some("反向")), ("asset", "AN-W", "", None)],
    )
    .unwrap();
    assert_eq!(rels.len(), 2);
    assert_eq!(rels[0].kind, RELATION_KIND_VOICE);
    assert_eq!(rels[0].label, "", "voice 无称谓存空串");
    assert_eq!(rels[0].reverse_label, None, "voice reverse 恒 None");
    assert_eq!(rels[0].from.kind, "archive");
    // from = 音频 asset 报错（方案 1 单向：音频不能作使用方，防"音频的音频"递归）
    assert!(add_batch(&h.lib.space_root, &h.lib, ("asset", "AN-V"), RELATION_KIND_VOICE, vec![("asset", "AN-W", "", None)]).is_err(), "音频作 from");
    // to = 非音频 asset / 档案 / 自身 全报错；整批不落盘
    assert!(add_batch(&h.lib.space_root, &h.lib, ("asset", "AN-A"), RELATION_KIND_VOICE, vec![("asset", "AN-A", "", None)]).is_err(), "自身");
    assert!(add_batch(&h.lib.space_root, &h.lib, ("archive", "F-B"), RELATION_KIND_VOICE, vec![("archive", "F-B", "", None)]).is_err(), "档案作 to");
    assert!(
      add_batch(&h.lib.space_root, &h.lib, ("archive", "F-B"), RELATION_KIND_VOICE, vec![("asset", "AN-V", "", None), ("asset", "AN-A", "", None)]).is_err(),
      "混入非音频整批失败"
    );
    // 已有的 2 条不受影响，失败批未追加
    let disk: RelationsFile = read_json(&h.lib.space_root.join(RELATIONS_FILE)).unwrap();
    assert_eq!(disk.relations.len(), 2);
  }

  /// 编辑称谓：label / reverse 修改与清空；无改动 = false 不落盘
  #[test]
  fn update_label_roundtrip() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "雪牧.fla");
    add_archive(&mut h.lib, "F-B", "雪天");
    let rels = add_batch(&h.lib.space_root, &h.lib, ("asset", "AN-A"), RELATION_KIND_FREE, vec![("archive", "F-B", "父亲", Some("儿子"))]).unwrap();
    let id = &rels[0].id;

    // 无改动 = false
    assert!(!update(&h.lib.space_root, id, "父亲", Some("儿子")).unwrap());
    // 改称谓 + reverse 清空
    assert!(update(&h.lib.space_root, id, "老爸", Some("")).unwrap());
    let disk: RelationsFile = read_json(&h.lib.space_root.join(RELATIONS_FILE)).unwrap();
    assert_eq!(disk.relations[0].label, "老爸");
    assert_eq!(disk.relations[0].reverse_label, None);
    // 空 label / 不存在的关系报错
    assert!(update(&h.lib.space_root, id, "  ", None).is_err());
    assert!(update(&h.lib.space_root, "RL-不存在", "x", None).is_err());
  }

  /// 删除：存在删除落盘；不存在报错
  #[test]
  fn remove_deletes() {
    let mut h = make_lib();
    add_asset(&mut h.lib, "AN-A", "雪牧.fla");
    add_archive(&mut h.lib, "F-B", "雪天");
    let rels = add_batch(&h.lib.space_root, &h.lib, ("asset", "AN-A"), RELATION_KIND_FREE, vec![("archive", "F-B", "父亲", None)]).unwrap();
    remove(&h.lib.space_root, &rels[0].id).unwrap();
    let disk: RelationsFile = read_json(&h.lib.space_root.join(RELATIONS_FILE)).unwrap();
    assert!(disk.relations.is_empty());
    assert!(remove(&h.lib.space_root, &rels[0].id).is_err(), "重复删除报错");
  }
}
