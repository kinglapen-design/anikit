//! 列表视图：Placement 与其资源的联查结果（中间面板列表、导入 Asset 事件共用）。

use std::path::Path;

use serde::Serialize;

use crate::core::library::ASSTES_DIR;
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
  /// 缩略图绝对路径（无缩略图为 null）
  pub thumb_path: Option<String>,
  pub origin: bool,
}

/// Placement + 资源元数据 → 视图；调用方负责跳过索引中缺失的资源（脏 Placement）
pub fn placement_view(root: &Path, meta: &AssetMeta, p: &Placement) -> PlacementView {
  let thumb_path = meta
    .thumbnail
    .as_ref()
    .map(|t| root.join(ASSTES_DIR).join(format!("{}.info", meta.id)).join(t).to_string_lossy().into_owned());
  PlacementView {
    asset_id: p.asset.clone(),
    folder_id: p.folder.clone(),
    name: p.name.clone(),
    w: meta.width,
    h: meta.height,
    ext: meta.ext.clone(),
    thumb_path,
    origin: p.origin,
  }
}
