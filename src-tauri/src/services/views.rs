//! 列表视图：Placement 与其资源的联查结果（中间面板列表、导入 Asset 事件共用）。

use std::collections::HashSet;
use std::path::Path;

use serde::Serialize;

use crate::core::library::{ASSTES_DIR, CACHE_DIR};
use crate::core::models::{AssetMeta, Placement};

/// 前端列表项：放置信息 + 资源宽高 / 扩展名 / 缩略图绝对路径
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementView {
  pub asset_id: String,
  pub folder_id: String,
  pub name: String,
  pub w: Option<u32>,
  pub h: Option<u32>,
  pub ext: String,
  /// 缩略图绝对路径（info 内缩略图优先，无则系统格式图标缓存；皆无为 null）
  pub thumb_path: Option<String>,
  pub origin: bool,
  /// 序列帧帧数（PRD 01-2.5）；非序列帧为 null——前端据此把格式标签与名称后缀显示为"序列帧"
  pub frame_count: Option<u32>,
}

/// 缩略图绝对路径：info 内缩略图优先；无则回退系统格式图标缓存（4.2，按格式一份，该格式已缓存时）
pub fn thumb_path(root: &Path, meta: &AssetMeta, icons: &HashSet<String>) -> Option<String> {
  if let Some(t) = &meta.thumbnail {
    return Some(root.join(ASSTES_DIR).join(format!("{}.info", meta.id)).join(t).to_string_lossy().into_owned());
  }
  let key = if meta.ext.is_empty() { "file" } else { meta.ext.as_str() };
  if !icons.contains(key) {
    return None;
  }
  Some(root.join(CACHE_DIR).join("thumbs").join(format!("{key}.webp")).to_string_lossy().into_owned())
}

/// Placement + 资源元数据 → 视图；调用方负责跳过索引中缺失的资源（脏 Placement）
pub fn placement_view(root: &Path, meta: &AssetMeta, p: &Placement, icons: &HashSet<String>) -> PlacementView {
  PlacementView {
    asset_id: p.asset.clone(),
    folder_id: p.folder.clone(),
    name: p.name.clone(),
    w: meta.width,
    h: meta.height,
    ext: meta.ext.clone(),
    thumb_path: thumb_path(root, meta, icons),
    origin: p.origin,
    frame_count: meta.frame_count,
  }
}
