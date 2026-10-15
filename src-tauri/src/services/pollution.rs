//! 污染文件业务（2026-10-16，1.0 收尾第 5 项，PRD 03_数据链接 02「污染文件」——6.2 工程恢复文件同物）：
//! 资产 info 文件夹内不属于 AniKit 已知文件集的磁盘条目（崩溃恢复文件 RECOVER_* / 恢复_*、
//! 外部程序误写、用户误拖入、改名残留缓存）对详情「链接关系」tab 可见。
//!
//! 已知集 = 字段驱动 + 写死名，**禁后缀盲扫**（cache.rs 红线：用户 origin 可能恰叫
//! xx_thumbnail.png / xx_anim.webp，盲扫必误伤）：写死名 metadata.json / notes.json /
//! annotation.json（备份清单同口径）+ origin 实体（metadata.originName）+ 缩略图
//!（metadata.thumbnail）+ 动图小动画缓存推导名（{origin词干}_anim.webp，paths::anim_preview_file
//! 单点）+ frames/ 目录（仅序列帧形态已知——其余资产冒出 frames/ 算异物）。
//! 三动作 = 打开资源管理器（复用 reveal_orphan_info 池内校验，命令层零新增）/
//! 导入（既有导入管线 move 语义 = 导入即清污，前端 probeSequences 全链继承）/ 丢回收站（本模块）。

use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path};

use crate::core::models::AssetMeta;
use crate::core::paths;

/// 污染条目（详情链接 tab 污染文件区块行）：path = 完整路径（reveal 直传）；isDir 分流导入 kind
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PollutionItem {
  pub name: String,
  pub path: String,
  pub is_dir: bool,
}

/// 纯文件名单组件校验（metadata 字段引用与命令入参共用）：唯一 Normal 组件才合法——
/// 拦 ".." / "a/b" / 绝对路径形态，保证一切操作永不越出 info 目录（thumb_cache_files 同口径纵深防御）
fn single_file_component(name: &str) -> bool {
  let mut comps = Path::new(name).components();
  matches!(comps.next(), Some(Component::Normal(_))) && comps.next().is_none()
}

/// info 文件夹已知文件集（污染判定唯一出处）。入集名字不要求磁盘存在（按名比对，不比路径）。
fn known_info_entries(assets_dir: &Path, meta: &AssetMeta) -> HashSet<String> {
  let mut known: HashSet<String> =
    ["metadata.json", "notes.json", "annotation.json"].iter().map(|s| (*s).to_string()).collect();
  if single_file_component(&meta.origin_name) {
    known.insert(meta.origin_name.clone());
  }
  if let Some(t) = &meta.thumbnail {
    if single_file_component(t) {
      known.insert(t.clone());
    }
  }
  // 动图小动画缓存（推导名写死，无 metadata 字段）：取文件名部分入集
  let anim = paths::anim_preview_file(assets_dir, &meta.id, &meta.origin_name);
  if let Some(name) = anim.file_name() {
    known.insert(name.to_string_lossy().into_owned());
  }
  // frames/ 仅序列帧形态已知（origin 不在 info 根 + frames/ 存在，sequence::frames_dir_if_sequence 同口径）
  if crate::services::sequence::frames_dir_if_sequence(assets_dir, &meta.id, meta).is_some() {
    known.insert("frames".to_string());
  }
  known
}

/// 污染清单（详情区块数据源）：info 单层 read_dir 过滤已知集，按名字典序稳定输出；
/// info 目录缺失 = 无可见异物（空壳/孤儿治理由开库自检另管）
pub fn list_pollution(assets_dir: &Path, meta: &AssetMeta) -> Result<Vec<PollutionItem>, String> {
  let info = paths::info_dir(assets_dir, &meta.id);
  if !info.is_dir() {
    return Ok(Vec::new());
  }
  let known = known_info_entries(assets_dir, meta);
  let mut items = Vec::new();
  for entry in fs::read_dir(&info).map_err(|e| format!("info 目录读取失败：{e}"))?.flatten() {
    let name = entry.file_name().to_string_lossy().into_owned();
    if known.contains(&name) {
      continue;
    }
    let path = entry.path();
    items.push(PollutionItem { name, path: path.to_string_lossy().into_owned(), is_dir: path.is_dir() });
  }
  items.sort_by(|a, b| a.name.cmp(&b.name));
  Ok(items)
}

/// 丢回收站（系统回收站，工作空间删除同口径，无确认弹窗——回收站即兜底）：
/// 三层校验 = 纯文件名单组件 + 确实不在已知集 + 存在，保证永不误删库管文件
///（metadata / origin / 缩略图 / 帧目录），再交回收站适配器
pub fn trash_pollution(assets_dir: &Path, meta: &AssetMeta, name: &str) -> Result<(), String> {
  if !single_file_component(name) {
    return Err("非法文件名".to_string());
  }
  if known_info_entries(assets_dir, meta).contains(name) {
    return Err("该条目属于资产数据，不能按污染文件处理".to_string());
  }
  let target = paths::info_dir(assets_dir, &meta.id).join(name);
  if !target.exists() {
    return Err("条目已不存在（可能刚被移动或删除）".to_string());
  }
  crate::adapters::recycle_bin::delete_to_recycle(&[target])
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library;

  fn meta(id: &str, origin: &str) -> AssetMeta {
    serde_json::from_value(serde_json::json!({
      "version": 1, "id": id, "name": "雪牧", "originName": origin, "ext": "fla", "size": 100,
      "importedAt": "2026-10-16T00:00:00+08:00", "modifiedAt": "2026-10-16T00:00:00+08:00"
    }))
    .unwrap()
  }

  fn touch(dir: &Path, rel: &str) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(&p, b"x").unwrap();
  }

  /// 已知集全要素：写死名 / origin / 缩略图 / 动图缓存 / 序列帧 frames 全不误判，异物文件与目录列出
  #[test]
  fn list_filters_known_set() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    let mut m = meta("AN-A", "雪牧.fla");
    m.thumbnail = Some("雪牧_thumbnail.webp".into());
    let info = lib.info_dir("AN-A");
    fs::create_dir_all(&info).unwrap();
    for f in ["metadata.json", "notes.json", "annotation.json", "雪牧.fla", "雪牧_thumbnail.webp", "雪牧_anim.webp"] {
      touch(&info, f);
    }
    touch(&info, "恢复_雪牧.fla");
    touch(&info, "RECOVER_雪牧.fla");
    fs::create_dir_all(info.join("误拖文件夹")).unwrap();

    let items = list_pollution(&lib.assets_dir, &m).unwrap();
    let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, vec!["RECOVER_雪牧.fla", "恢复_雪牧.fla", "误拖文件夹"], "只列异物且字典序");
    assert!(items[2].is_dir && !items[0].is_dir);
    assert!(items[0].path.ends_with("RECOVER_雪牧.fla"), "完整路径供 reveal 直传");

    // 陈旧词干动图缓存（改名残留）判污染——rename_origin 已同步改名堵增量，存量由用户清
    touch(&info, "旧词干_anim.webp");
    let items = list_pollution(&lib.assets_dir, &m).unwrap();
    assert!(items.iter().any(|i| i.name == "旧词干_anim.webp"));
  }

  /// frames/ 目录：仅序列帧形态（origin 不在 info 根 + frames/ 存在）算已知；普通资产冒出 frames/ 判异物
  #[test]
  fn frames_dir_known_only_for_sequence() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    // 序列帧形态：origin 指向 frames/ 内封面帧，info 根无实体
    let seq = meta("AN-SEQ", "走路_001.png");
    let info = lib.info_dir("AN-SEQ");
    touch(&info, "frames/走路_001.png");
    touch(&info, "metadata.json");
    let items = list_pollution(&lib.assets_dir, &seq).unwrap();
    assert!(items.is_empty(), "序列帧 frames/ 已知：{items:?}");

    // 普通资产（origin 在根）冒出 frames/ = 异物
    let m = meta("AN-N", "雪牧.fla");
    let info = lib.info_dir("AN-N");
    touch(&info, "雪牧.fla");
    fs::create_dir_all(info.join("frames")).unwrap();
    let items = list_pollution(&lib.assets_dir, &m).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "frames");
  }

  /// info 目录缺失 / 空目录 = 空清单
  #[test]
  fn list_missing_info_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    let m = meta("AN-X", "a.fla");
    assert!(list_pollution(&lib.assets_dir, &m).unwrap().is_empty());
  }

  /// 删除三层校验：越界名 / 已知集成员（库管文件）/ 不存在 全拒
  #[test]
  fn trash_validation_rejects() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    let mut m = meta("AN-A", "雪牧.fla");
    m.thumbnail = Some("雪牧_thumbnail.webp".into());
    let info = lib.info_dir("AN-A");
    touch(&info, "metadata.json");
    touch(&info, "雪牧.fla");

    assert!(trash_pollution(&lib.assets_dir, &m, "..").is_err(), ".. 拒");
    assert!(trash_pollution(&lib.assets_dir, &m, "a/b").is_err(), "分隔符拒");
    assert!(trash_pollution(&lib.assets_dir, &m, "metadata.json").is_err(), "库管文件拒");
    assert!(trash_pollution(&lib.assets_dir, &m, "雪牧.fla").is_err(), "origin 拒");
    assert!(trash_pollution(&lib.assets_dir, &m, "雪牧_thumbnail.webp").is_err(), "缩略图拒");
    assert!(trash_pollution(&lib.assets_dir, &m, "雪牧_anim.webp").is_err(), "动图缓存拒");
    let e = trash_pollution(&lib.assets_dir, &m, "不存在.txt").unwrap_err();
    assert!(e.contains("已不存在"), "{e}");
  }

  /// 真删（Windows 回收站适配器先例 = junction.rs 实删进真回收站）：异物消失、库管文件无恙
  #[cfg(windows)]
  #[test]
  fn trash_moves_foreign_file_to_recycle() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    let m = meta("AN-A", "雪牧.fla");
    let info = lib.info_dir("AN-A");
    touch(&info, "metadata.json");
    touch(&info, "雪牧.fla");
    touch(&info, "恢复_雪牧.fla");
    trash_pollution(&lib.assets_dir, &m, "恢复_雪牧.fla").unwrap();
    assert!(!info.join("恢复_雪牧.fla").exists(), "异物已进回收站");
    assert!(info.join("雪牧.fla").is_file() && info.join("metadata.json").is_file(), "库管文件不动");
  }
}
