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
  /// 资源库名称不合法（Windows 文件夹命名规则）
  NameInvalid(&'static str),
  /// 该文件夹不是 AniKit 储存空间（space.json 缺失 / 损坏，打开准入校验）
  NotASpace,
  /// 旧版库结构（库根自含 asstes/ 或空间缺 space.json）：不迁移，引导新建（2026-08-12 定案）
  LegacyLibrary,
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
      DataError::NameInvalid(msg) => write!(f, "{msg}"),
      DataError::NotASpace => write!(f, "该文件夹不是 AniKit 储存空间"),
      DataError::LegacyLibrary => write!(f, "旧版库结构，请新建资源库"),
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

/// 原子写 JSON：写入同名 .tmp 后原子替换目标（Windows = MoveFileExW 覆盖式 rename，
/// 无「先删后换」窗口期，踩坑 095；POSIX = rename 原子覆盖）。
/// 输出带缩进，保证人类可读（记事本可救急，PRD 06-03）。
pub fn write_json<T: Serialize>(path: &Path, data: &T) -> Result<(), DataError> {
  let text =
    serde_json::to_string_pretty(data).map_err(|e| DataError::Corrupt(format!("序列化失败：{e}")))?;
  write_text_atomic(path, &text)
}

/// 原子写文本（.tmp 先写后换）：撤销回滚直接写快照原文，与 write_json 同一套落盘语义
pub(crate) fn write_text_atomic(path: &Path, text: &str) -> Result<(), DataError> {
  let tmp = path.with_extension("json.tmp");
  fs::write(&tmp, text)?;
  replace_file(&tmp, path)
}

/// 原子替换目标文件（2026-09-29 踩坑 095：原「先删目标再 rename」在删与换之间有窗口期——
/// 进程恰在窗口内被杀（强杀 / 关电源）= 目标永久丢失。config.json 消失后启动按默认重建，
/// ui / 窗口几何由前端内存态随即写回无感，唯 MCP 写入闸 / 插件服务开关等无前端快照的子树
/// 静默回默认（用户实机「MCP 写入开关自己关掉」病根））：
/// Windows = MoveFileExW（REPLACE_EXISTING 覆盖 + WRITE_THROUGH 写穿落盘）原子替换无窗口期；
/// 其余平台 fs::rename 本就原子覆盖。
#[cfg(windows)]
fn replace_file(tmp: &Path, path: &Path) -> Result<(), DataError> {
  use std::os::windows::ffi::OsStrExt;
  use windows::Win32::Storage::FileSystem::{
    MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
  };
  use windows::core::PCWSTR;
  let to_wide = |p: &Path| -> Vec<u16> { p.as_os_str().encode_wide().chain(Some(0)).collect() };
  let from_w = to_wide(tmp);
  let to_w = to_wide(path);
  unsafe {
    MoveFileExW(
      PCWSTR(from_w.as_ptr()),
      PCWSTR(to_w.as_ptr()),
      MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
    )
  }
  .map_err(|e| DataError::Io(std::io::Error::other(e)))?;
  Ok(())
}

/// 非 Windows：POSIX rename 原子覆盖已存在目标
#[cfg(not(windows))]
fn replace_file(tmp: &Path, path: &Path) -> Result<(), DataError> {
  fs::rename(tmp, path)?;
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
  fn overwrite_existing_target() {
    // 目标已存在时覆盖写（原子替换路径，踩坑 095 前是「先删后换」）
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    fs::write(&path, r#"{ "version": 1, "settings": { "old": true } }"#).unwrap();
    let data = crate::core::models::FoldersFile::new();
    write_json(&path, &data).unwrap();
    let back: crate::core::models::FoldersFile = read_json(&path).unwrap();
    assert_eq!(data, back);
    assert!(!dir.path().join("config.json.tmp").exists());
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
