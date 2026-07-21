//! JSON 读写底层：所有数据文件的统一出入口。
//! - 原子写入：先写同名 .tmp 再替换目标，避免中途崩溃留下半截文件；
//! - 版本门禁：读取即校验 "version" 字段，高于当前版本拒绝（PRD 06-05 高版本禁止写入）；
//! - 错误不吞，向上抛中文人话（01_开发规范 第 6 条）。
//!
//! 单一写点纪律：每个数据文件只有一个模块负责写（如 folders.json 由 core::library 负责），
//! 其他模块只能通过它的接口操作，不允许各自拼路径直接写。

use std::fs;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::core::models::DATA_VERSION;

#[derive(Debug)]
pub enum DataError {
  Io(std::io::Error),
  /// JSON 解析失败或缺 version 字段（文件损坏，PRD 06-04 提示回滚）
  Corrupt(String),
  /// 文件版本高于当前软件（新版软件写过的库用旧版打开，禁止写入）
  VersionTooNew(u32),
  /// 该文件夹不是 AniKit 资源库（打开准入校验）
  NotALibrary,
  /// 该文件夹已是资源库（创建时误选）
  AlreadyLibrary,
  /// 目标文件夹非空（创建要求空文件夹）
  NotEmpty,
}

impl std::fmt::Display for DataError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      DataError::Io(e) => write!(f, "文件读写失败：{e}"),
      DataError::Corrupt(msg) => write!(f, "数据文件损坏：{msg}"),
      DataError::VersionTooNew(v) => write!(
        f,
        "数据文件版本（v{v}）高于当前软件支持的版本（v{DATA_VERSION}），请升级软件后再打开"
      ),
      DataError::NotALibrary => write!(f, "该文件夹不是 AniKit 资源库"),
      DataError::AlreadyLibrary => write!(f, "该文件夹已是 AniKit 资源库，请改用打开资源库"),
      DataError::NotEmpty => write!(f, "目标文件夹不是空文件夹，请选择空文件夹创建资源库"),
    }
  }
}

impl std::error::Error for DataError {}

impl From<std::io::Error> for DataError {
  fn from(e: std::io::Error) -> Self {
    DataError::Io(e)
  }
}

/// 读 JSON 并校验版本：文件必须带 "version" 字段且不高于 DATA_VERSION。
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, DataError> {
  let text = fs::read_to_string(path)?;
  // 先只取 version 做门禁，再整体反序列化
  let probe: serde_json::Value =
    serde_json::from_str(&text).map_err(|e| DataError::Corrupt(format!("{}：{e}", path.display())))?;
  let version = probe
    .get("version")
    .and_then(|v| v.as_u64())
    .ok_or_else(|| DataError::Corrupt(format!("{} 缺少 version 字段", path.display())))?;
  if version > DATA_VERSION as u64 {
    return Err(DataError::VersionTooNew(version as u32));
  }
  serde_json::from_str(&text).map_err(|e| DataError::Corrupt(format!("{}：{e}", path.display())))
}

/// 原子写 JSON：写入同名 .tmp 后替换目标（Windows 的 rename 不覆盖已有文件，先删后换；
/// 删与换之间的窗口期极短且 .tmp 仍在，可人工救急）。
/// 输出带缩进，保证人类可读（记事本可救急，PRD 06-03）。
pub fn write_json<T: Serialize>(path: &Path, data: &T) -> Result<(), DataError> {
  let text =
    serde_json::to_string_pretty(data).map_err(|e| DataError::Corrupt(format!("序列化失败：{e}")))?;
  let tmp = path.with_extension("json.tmp");
  fs::write(&tmp, text)?;
  if path.exists() {
    fs::remove_file(path)?;
  }
  fs::rename(&tmp, path)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn write_then_read_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("folders.json");
    let data = crate::core::models::FoldersFile::new();
    write_json(&path, &data).unwrap();
    let back: crate::core::models::FoldersFile = read_json(&path).unwrap();
    assert_eq!(data, back);
    // .tmp 不残留
    assert!(!dir.path().join("folders.json.tmp").exists());
  }

  #[test]
  fn rejects_version_too_new() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("folders.json");
    fs::write(&path, r#"{ "version": 999, "folders": [], "items": [] }"#).unwrap();
    let result = read_json::<crate::core::models::FoldersFile>(&path);
    assert!(matches!(result, Err(DataError::VersionTooNew(999))));
  }

  #[test]
  fn rejects_missing_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("folders.json");
    fs::write(&path, r#"{ "folders": [], "items": [] }"#).unwrap();
    let result = read_json::<crate::core::models::FoldersFile>(&path);
    assert!(matches!(result, Err(DataError::Corrupt(_))));
  }

  #[test]
  fn rejects_broken_json() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("folders.json");
    fs::write(&path, "{ not json").unwrap();
    let result = read_json::<crate::core::models::FoldersFile>(&path);
    assert!(matches!(result, Err(DataError::Corrupt(_))));
  }
}
