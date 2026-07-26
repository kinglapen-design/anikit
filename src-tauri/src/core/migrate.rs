//! 版本迁移骨架（PRD 06-05）：
//! - 旧版本升级：打开库时检测，低于当前版本则逐级迁移（v1→v2→v3），迁移前自动备份一次；
//! - 高版本降级：版本高于当前软件则拒绝（json_io 读取门禁已挡），提示升级软件。
//!
//! 当前 DATA_VERSION = 1，MIGRATIONS 为空。后续升级版本时：
//! 1. DATA_VERSION + 1；
//! 2. 在 MIGRATIONS 末尾追加一级迁移函数（索引 i 对应 v(i+1) → v(i+2)）。

use std::path::Path;

use serde_json::Value;

use crate::core::json_io::{read_json, write_json, DataError};
use crate::core::library::data_files;
use crate::core::models::DATA_VERSION;

/// 一级迁移：把 JSON 数据从 vN 升到 vN+1（直接改 value，version 字段由框架统一递增）。
type Migration = fn(&mut Value) -> Result<(), String>;

/// 迁移注册表：索引 i 对应 v(i+1) → v(i+2)。
static MIGRATIONS: &[Migration] = &[];

/// 库级迁移检查：任一数据文件版本过低 → 先自动备份一次 → 逐个逐级迁移（PRD 06-05）。
pub fn migrate_library_if_needed(root: &Path) -> Result<(), DataError> {
  let files = data_files(root)?;
  let needs = files
    .iter()
    .any(|p| file_version(p).map(|v| v < DATA_VERSION).unwrap_or(false));
  if !needs {
    return Ok(());
  }
  crate::core::backup::backup_library(root)?; // 迁移前自动备份
  for p in files {
    migrate_json_file(&p)?;
  }
  log::info!("数据迁移完成：{}", root.display());
  Ok(())
}

/// 单文件迁移：已是最新返回 Ok(false)，执行过迁移返回 Ok(true)。
pub fn migrate_json_file(path: &Path) -> Result<bool, DataError> {
  let mut value: Value = read_json(path)?;
  if migrate_value(&mut value, MIGRATIONS, DATA_VERSION)? {
    write_json(path, &value)?;
    return Ok(true);
  }
  Ok(false)
}

/// 逐级迁移核心：从文件当前版本一直升到 target。
/// 抽出来独立可测（测试用自定义 migrations 与 target 模拟多版本场景）。
fn migrate_value(value: &mut Value, migrations: &[Migration], target: u32) -> Result<bool, DataError> {
  let mut version = value
    .get("version")
    .and_then(|v| v.as_u64())
    .ok_or_else(|| DataError::Corrupt("缺少 version 字段".into()))? as u32;
  if version < 1 {
    return Err(DataError::Corrupt(format!("非法版本号 v{version}")));
  }
  if version == target {
    return Ok(false);
  }
  if version > target {
    return Err(DataError::VersionTooNew(version));
  }
  while version < target {
    let step = migrations
      .get(version as usize - 1)
      .ok_or_else(|| DataError::Corrupt(format!("缺少 v{version} → v{} 的迁移步骤", version + 1)))?;
    step(value).map_err(DataError::Corrupt)?;
    version += 1;
    value["version"] = Value::from(version);
  }
  Ok(true)
}

/// 读文件版本号（损坏 / 缺字段返回 None，由读取门禁统一报错）
fn file_version(path: &Path) -> Option<u32> {
  let text = std::fs::read_to_string(path).ok()?;
  serde_json::from_str::<Value>(&text).ok()?.get("version")?.as_u64().map(|v| v as u32)
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  fn v1_to_v2(v: &mut Value) -> Result<(), String> {
    v["v1_marker"] = json!(true);
    Ok(())
  }

  fn v2_to_v3(v: &mut Value) -> Result<(), String> {
    v["v2_marker"] = json!(true);
    Ok(())
  }

  #[test]
  fn migrates_step_by_step() {
    let migrations: &[Migration] = &[v1_to_v2, v2_to_v3];
    let mut value = json!({ "version": 1 });
    let migrated = migrate_value(&mut value, migrations, 3).unwrap();
    assert!(migrated);
    assert_eq!(value["version"], json!(3));
    assert_eq!(value["v1_marker"], json!(true));
    assert_eq!(value["v2_marker"], json!(true));
  }

  #[test]
  fn already_latest_is_noop() {
    let mut value = json!({ "version": 1 });
    let migrated = migrate_value(&mut value, &[], 1).unwrap();
    assert!(!migrated);
  }

  #[test]
  fn rejects_version_too_new() {
    let mut value = json!({ "version": 5 });
    assert!(matches!(migrate_value(&mut value, &[], 1), Err(DataError::VersionTooNew(5))));
  }

  #[test]
  fn missing_migration_step_is_error() {
    // 只有 v1→v2，却要升到 v3：缺步骤必须报错而不是静默跳过
    let mut value = json!({ "version": 1 });
    assert!(matches!(migrate_value(&mut value, &[v1_to_v2], 3), Err(DataError::Corrupt(_))));
  }
}
