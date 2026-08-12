//! 列表视图：Placement 与其资源的联查结果（中间面板列表、导入 Asset 事件共用）。

use std::collections::HashSet;

use serde::Serialize;

use crate::core::library::{Library, CACHE_DIR};
use crate::core::models::{AssetMeta, Placement};

/// 缩略图来源（4.2）：asset = info 内内容缩略图；formatIcon = 系统格式图标缓存
/// ——前端据此对格式图标 contain 显示（图标方形不裁切，2026-07-22 用户定）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ThumbKind {
  Asset,
  FormatIcon,
}

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
  /// 缩略图来源（无缩略图为 null）
  pub thumb_kind: Option<ThumbKind>,
  pub origin: bool,
  /// 序列帧帧数（PRD 01-2.5）；动图（GIF/WEBP 动图）同字段（5.1）；非序列帧非动图为 null
  pub frame_count: Option<u32>,
  /// 动图标记（5.1）：frame_count 非空且 ext 为 gif/webp——前端据此与序列帧区分（角标 / 名称后缀）
  pub animated: bool,
  /// 音视频 / 动图时长（秒，5.1）；无时长资源为 null——前端卡片标签据此显示 mm:ss
  pub duration: Option<f64>,
  /// 音频 BPM（只读内嵌标签，5.1）；无标签为 null
  pub bpm: Option<f64>,
  /// 导入渠道标记（PRD 01-2.3）："inbox" = 收件箱渠道；快速访问"已导入"据此持久显示未摘标资产
  pub imported_from: Option<String>,
  /// 逐帧笔记条数冗余（5.2 展示层：卡片笔记角标据此显隐）；无笔记为 null
  pub note_count: Option<u32>,
  /// 封面帧（PRD 01）：序列帧/动图 = frames/ 内帧名；视频 = `t:{秒}`（5.2 set_video_thumb 写入，
  /// 卡片悬停播放范围起点 = 该秒数，5.3）；未设置为 null
  pub thumb_frame: Option<String>,
  /// 帧率（序列帧 = 播放帧率默认 24；视频 = 素材自身帧率，5.3 帧化改造补——前端帧号显示 /
  /// 逐帧步进按 1/fps）；无帧概念资源为 null
  pub fps: Option<f64>,
}

/// 格式图标缓存键（空扩展名归 "file"）
fn icon_key(ext: &str) -> &str {
  if ext.is_empty() { "file" } else { ext }
}

/// 缩略图绝对路径：info 内缩略图优先；无则回退系统格式图标缓存（4.2，按格式一份，该格式已缓存时）
pub fn thumb_path(lib: &Library, meta: &AssetMeta, icons: &HashSet<String>) -> Option<String> {
  if let Some(t) = &meta.thumbnail {
    return Some(lib.info_dir(&meta.id).join(t).to_string_lossy().into_owned());
  }
  let key = icon_key(&meta.ext);
  if !icons.contains(key) {
    return None;
  }
  Some(lib.root.join(CACHE_DIR).join("thumbs").join(format!("{key}.webp")).to_string_lossy().into_owned())
}

/// Placement + 资源元数据 → 视图；调用方负责跳过索引中缺失的资源（脏 Placement）
pub fn placement_view(lib: &Library, meta: &AssetMeta, p: &Placement, icons: &HashSet<String>) -> PlacementView {
  let thumb_kind = if meta.thumbnail.is_some() {
    Some(ThumbKind::Asset)
  } else if icons.contains(icon_key(&meta.ext)) {
    Some(ThumbKind::FormatIcon)
  } else {
    None
  };
  PlacementView {
    asset_id: p.asset.clone(),
    folder_id: p.folder.clone(),
    name: p.name.clone(),
    w: meta.width,
    h: meta.height,
    ext: meta.ext.clone(),
    thumb_path: thumb_path(lib, meta, icons),
    thumb_kind,
    origin: p.origin,
    frame_count: meta.frame_count,
    animated: meta.frame_count.is_some() && matches!(meta.ext.as_str(), "gif" | "webp"),
    duration: meta.duration,
    bpm: meta.bpm,
    imported_from: meta.imported_from.clone(),
    note_count: meta.note_count,
    thumb_frame: meta.thumb_frame.clone(),
    fps: meta.fps,
  }
}
