//! MCP 工具注册表（一类一文件 + mod.rs 注册处一行；PRD 08-06 工具注册表模式）
//!
//! 参数 id 归一化（2026-10-08 S5，PRD 02-09）：asset_id / folder_id 字段挂 deserialize_with =
//! de_ref_id 系——anikit:// 与 http://127.0.0.1 形态链接 AI 整段透传即可（core::anikit_ref::normalize_ref_id）

pub mod archives;
pub mod assets;
pub mod folders;
pub mod groups;
pub mod import;
pub mod library;
pub mod prompt;
pub mod search;
pub mod structure;
pub mod text;
pub mod trash;
pub mod write;

use serde::Deserialize;

/// 单 id 字段归一化（serde deserialize_with；Params struct 的 asset_id / folder_id 用）
pub fn de_ref_id<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
  Ok(crate::core::anikit_ref::normalize_ref_id(&String::deserialize(d)?))
}

/// Option id 字段版（缺省字段 serde 自动 None，本函数只在字段存在时调用）
pub fn de_ref_id_opt<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
  Ok(Option::<String>::deserialize(d)?.map(|s| crate::core::anikit_ref::normalize_ref_id(&s)))
}

/// Vec id 字段版（move_folders 的 folder_ids 等批量参数）
pub fn de_ref_id_vec<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
  Ok(Vec::<String>::deserialize(d)?.into_iter().map(|s| crate::core::anikit_ref::normalize_ref_id(&s)).collect())
}
