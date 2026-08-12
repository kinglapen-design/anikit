//! 命令 API：收件箱"工程文件"磁盘实况（4.4，PRD 05-01，2026-07-25 语义重定）。
//! 实况 = 磁盘事实来源：不建档、不写库、不建持久状态；手动导入 = 移走入库（剪切语义，
//! 入口动作名"导入"），不做复制导入（避免磁盘实况与库内资产双份）。
//! 所有磁盘命令只操作 inbox 根下一级条目，resolve_entry 统一防路径穿越。

use std::path::{Path, PathBuf};

use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

use crate::adapters::{inbox_watch, shell_open};
use crate::commands::import::{current_paths, spawn_import};
use crate::commands::library::AppState;
use crate::services::import::{ImportEvent, ImportOptions, SequenceMode, Source, Structure};
use crate::services::inbox::{self, InboxEntry, INBOX_DIR};

/// 短锁取当前空间根（收件箱目录锚点：inbox 空间级共用，2026-08-12 用户定）；无空间中文报错
fn current_space_root(app: &AppHandle) -> Result<PathBuf, String> {
  let state = app.state::<AppState>();
  let guard = state.space.lock().map_err(|e| e.to_string())?;
  guard.as_ref().map(|s| s.root.clone()).ok_or_else(|| "请先打开储存空间".to_string())
}

/// 磁盘实况快层快照（"工程文件"区块数据源；读失败按空列降级，不让单个坏条目拖垮整列）。
/// 必须快：目录递归大小 / 占用先给 0 / false 占位，由后台 detail worker 经
/// "inbox-disk-details" 事件补齐（前端收到后整体替换）
#[tauri::command]
pub fn list_inbox_snapshot(app: AppHandle) -> Result<Vec<InboxEntry>, String> {
  let root = current_space_root(&app)?;
  Ok(inbox::snapshot_fast(&root))
}

/// 手动导入单个条目：移走入库（Move + Keep 层级；imported_from = "inbox" 维持渠道标记，
/// 撞 hash 自动合并口径与自动导入一致）。占用条目禁止导入，返回中文人话；
/// Channel 复用现有导入事件流（11_导入进度条 / 10_重复添加提示）。
/// 条目路径按空间根推导（收件箱空间级），导入目标仍是当前打开的库
#[tauri::command]
pub fn import_inbox_entry(app: AppHandle, name: String, on_event: Channel<ImportEvent>) -> Result<u64, String> {
  let space_root = current_space_root(&app)?;
  let (root, assets_dir) = current_paths(&app)?;
  let path = resolve_entry(&space_root.join(INBOX_DIR), &name)?;
  if !path.exists() {
    return Err(format!("「{name}」已不在收件箱，可能刚被移走或删除"));
  }
  if !inbox_watch::entry_movable(&path) {
    return Err(format!("「{name}」正被其他软件占用，请先在对应软件里关闭它再导入"));
  }
  let opts = ImportOptions {
    structure: Structure::Keep,
    source: Source::Move,
    target_folder: None,
    paths: vec![path],
    imported_from: Some("inbox".to_string()),
    sequence_mode: SequenceMode::Scatter,
  };
  Ok(spawn_import(app, root, assets_dir, opts, on_event, None))
}

/// 用系统默认程序打开条目（工程中查看 / 继续编辑）
#[tauri::command]
pub fn open_inbox_entry(app: AppHandle, name: String) -> Result<(), String> {
  let root = current_space_root(&app)?;
  let path = resolve_entry(&root.join(INBOX_DIR), &name)?;
  if !path.exists() {
    return Err(format!("「{name}」已不在收件箱，可能刚被移走或删除"));
  }
  shell_open::open(&path)
}

/// 重命名条目（仅 inbox 下一级；同名冲突 / 占用返回中文人话）
#[tauri::command]
pub fn rename_inbox_entry(app: AppHandle, name: String, new_name: String) -> Result<(), String> {
  let root = current_space_root(&app)?;
  let dir = root.join(INBOX_DIR);
  let from = resolve_entry(&dir, &name)?;
  let to = resolve_entry(&dir, &new_name)?;
  if !from.exists() {
    return Err(format!("「{name}」已不在收件箱，可能刚被移走或删除"));
  }
  if name != new_name && to.exists() {
    return Err(format!("已有同名条目「{new_name}」，请换个名字"));
  }
  if !inbox_watch::entry_movable(&from) {
    return Err(format!("「{name}」正被其他软件占用，请先在对应软件里关闭它再重命名"));
  }
  std::fs::rename(&from, &to).map_err(|e| format!("重命名失败：{e}"))
}

/// 删除条目（文件 remove_file / 目录 remove_dir_all；占用等系统错误原样透传）
#[tauri::command]
pub fn delete_inbox_entry(app: AppHandle, name: String) -> Result<(), String> {
  let root = current_space_root(&app)?;
  let path = resolve_entry(&root.join(INBOX_DIR), &name)?;
  if !path.exists() {
    return Ok(()); // 实况滞后、条目已不在磁盘，视作已删除
  }
  let result = if path.is_dir() { std::fs::remove_dir_all(&path) } else { std::fs::remove_file(&path) };
  result.map_err(|e| format!("删除「{name}」失败：{e}"))
}

/// 校验条目名并解析为 inbox 下一级路径：拒绝路径分隔符与 Windows 非法字符（防路径穿越）；
/// canonicalize 双保险——解析结果的父目录必须等于 inbox 根（防符号链接等异常逃逸）
fn resolve_entry(dir: &Path, name: &str) -> Result<PathBuf, String> {
  if name.is_empty()
    || name == "."
    || name == ".."
    || name.chars().any(|c| matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
  {
    return Err(format!("条目名不合法：{name}"));
  }
  let path = dir.join(name);
  let parent = path.parent().and_then(|p| p.canonicalize().ok());
  let root = dir.canonicalize().ok();
  match (parent, root) {
    (Some(p), Some(r)) if p == r => Ok(path),
    _ => Err(format!("条目名不合法：{name}")),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn harness() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("inbox");
    std::fs::create_dir_all(&dir).unwrap();
    (tmp, dir)
  }

  /// 路径穿越与 Windows 非法字符一律拒绝
  #[test]
  fn resolve_entry_rejects_traversal_and_illegal_chars() {
    let (_tmp, dir) = harness();
    for bad in ["..", ".", "", "a/b", "a\\b", "a:b", "a*b", "a?b", "a\"b", "a<b", "a>b", "a|b"] {
      assert!(resolve_entry(&dir, bad).is_err(), "应拒绝：{bad:?}");
    }
    let ok = resolve_entry(&dir, "工程.fla").unwrap();
    assert_eq!(ok, dir.join("工程.fla"));
  }

  /// 快层快照：白名单判定 / 排序照全；文件大小取元数据；
  /// 目录大小 / 占用先给 0 / false 占位（details 后台补齐）
  #[test]
  fn snapshot_fast_lists_top_level_entries() {
    let (_tmp, dir) = harness();
    std::fs::write(dir.join("图.PNG"), "1234").unwrap(); // 大小写不敏感
    std::fs::write(dir.join("工程.fla"), "xx").unwrap();
    std::fs::create_dir_all(dir.join("夹/子")).unwrap();
    std::fs::write(dir.join("夹/a.png"), "1").unwrap();
    std::fs::write(dir.join("夹/子/b.psd"), "12").unwrap();
    let empty = dir.join("空夹");
    std::fs::create_dir_all(&empty).unwrap();
    std::fs::create_dir_all(dir.join("混夹")).unwrap();
    std::fs::write(dir.join("混夹/a.png"), "1").unwrap();
    std::fs::write(dir.join("混夹/工程.fla"), "2").unwrap();

    let list = inbox::snapshot_fast(dir.parent().unwrap());
    let by_name = |n: &str| list.iter().find(|e| e.name == n).unwrap();
    assert!(by_name("图.PNG").auto_importable);
    assert!(!by_name("工程.fla").auto_importable);
    assert!(by_name("夹").auto_importable, "全白名单目录可自动导入");
    assert!(!by_name("空夹").auto_importable, "空目录不自动导入");
    assert!(!by_name("混夹").auto_importable, "混入非白名单整目录不自动导入");
    assert_eq!(by_name("图.PNG").size, 4, "文件大小快层直取元数据");
    assert_eq!(by_name("夹").size, 0, "目录大小快层先给 0 占位");
    assert!(by_name("夹").is_dir);
    assert!(!by_name("夹").occupied, "目录占用快层先给 false 占位");
    assert!(by_name("夹").icon_path.is_none(), "目录恒无格式图标");
    // 按名称小写排序
    let names: Vec<&str> = list.iter().map(|e| e.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort_by_key(|a| a.to_lowercase());
    assert_eq!(names, sorted);
  }

  /// 慢层 details：目录递归大小与占用探测照全口径
  #[test]
  fn snapshot_details_computes_recursive_size_and_occupied() {
    let (_tmp, dir) = harness();
    std::fs::create_dir_all(dir.join("夹/子")).unwrap();
    std::fs::write(dir.join("夹/a.png"), "1").unwrap();
    std::fs::write(dir.join("夹/子/b.psd"), "12").unwrap();

    let list = inbox::snapshot_details(dir.parent().unwrap());
    let by_name = |n: &str| list.iter().find(|e| e.name == n).unwrap();
    assert_eq!(by_name("夹").size, 3);
    assert!(by_name("夹").is_dir);
    assert!(!by_name("夹").occupied);
  }

  /// 格式图标分层：快层只命中已缓存（不触发 shell，未缓存 None）；
  /// details 生成缓存后快层再扫即命中（shell 仅 Windows 可用，测试同）
  #[cfg(windows)]
  #[test]
  fn snapshot_icon_path_fast_cached_only_details_generates() {
    let (_tmp, dir) = harness();
    let root = dir.parent().unwrap();
    std::fs::write(dir.join("图.png"), "1234").unwrap();

    let list = inbox::snapshot_fast(root);
    assert!(list.iter().all(|e| e.icon_path.is_none()), "未缓存时快层不触发 shell");

    let details = inbox::snapshot_details(root);
    let icon = details.iter().find(|e| e.name == "图.png").unwrap().icon_path.as_ref().unwrap();
    assert!(std::path::Path::new(icon).is_file(), "details 应生成格式图标缓存");

    let list2 = inbox::snapshot_fast(root);
    let hit = list2.iter().find(|e| e.name == "图.png").unwrap().icon_path.as_ref().unwrap();
    assert_eq!(hit, icon, "缓存生成后快层应命中同一路径");
  }

  /// 白名单判定：扩展名大小写不敏感；无扩展名不可自动导入
  #[test]
  fn auto_importable_whitelist_case_insensitive() {
    let (_tmp, dir) = harness();
    for (name, expect) in [
      ("a.png", true),
      ("a.PSD", true),
      ("a.ai", true),
      ("a.fla", false),
      ("a.mp4", true), // 视频纳入白名单（2026-07-25 用户补）
      ("a.m4v", true),
      ("a.mkv", false), // PRD 5.1 MVP 视频口径以外仍不自动导入
      ("a.mp3", true), // 音频纳入白名单（5.1 波形/元数据就位，2026-07-26）
      ("a.wma", true),
      ("无扩展名", false),
    ] {
      let p = dir.join(name);
      std::fs::write(&p, "x").unwrap();
      assert_eq!(inbox::entry_auto_importable(&p), expect, "{name}");
    }
  }
}
