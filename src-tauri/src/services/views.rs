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
  /// 缩略图实际像素尺寸（5.4，2026-08-23 用户定）：布局比例跟缩略图走——自定义缩略图后自适应；
  /// null（存量数据 / 无缩略图）前端回退 w/h（导入时二者比例一致，无感知）
  pub thumb_w: Option<u32>,
  pub thumb_h: Option<u32>,
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
  // —— 以下四项服务 3.5 排序（名称 / 大小 / 时长 / 扩展名外的时间与大小键）；名称等既有字段同用 ——
  /// 文件大小（字节）
  pub size: u64,
  /// 源文件磁盘创建时间（RFC3339）；存量数据 / 读取失败为 null——前端排序排尾
  pub created_at: Option<String>,
  /// 库内数据最后变更时刻（RFC3339）
  pub modified_at: String,
  /// 导入时刻（RFC3339）
  pub imported_at: String,
  /// 逐帧笔记条数冗余（5.2 展示层：卡片笔记角标据此显隐）；无笔记为 null
  pub note_count: Option<u32>,
  /// 封面帧（PRD 01）：序列帧/动图 = frames/ 内帧名；视频 = `t:{秒}`（5.2 set_video_thumb 写入，
  /// 卡片悬停播放范围起点 = 该秒数，5.3）；未设置为 null
  pub thumb_frame: Option<String>,
  /// 帧率（序列帧 = 播放帧率默认 24；视频 = 素材自身帧率，5.3 帧化改造补——前端帧号显示 /
  /// 逐帧步进按 1/fps）；无帧概念资源为 null
  pub fps: Option<f64>,
  /// hidden 标记（PRD 02-15，8.1）：被设为缩略图的资产——普通视图统一过滤，仅「缩略图」视图显示
  pub hidden: bool,
  /// 播放源代理（8.1，PRD 02-15）：thumb_asset_id 有效时 = hidden 伴生资产 id——卡片动图播放
  /// （悬停 / 自动播放）的帧流开流目标（fla 自身无可解码帧流，伴生 WebP 才是画面源）；无代理为 None
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub play_asset_id: Option<String>,
  /// 标签（metadata.tags，2026-09-17 分组「标签」规则的卡片级数据通道——与排序四字段同径带出）
  pub tags: Vec<String>,
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
  // 伴生缩略图代理（8.1，PRD 02-15 thumbAssetId）：工程资产（fla 等）的缩略图四件
  // （路径 / 来源 / 实际尺寸）改取目标 hidden 资产——fla 卡片直接显示伴生 WebP 的图。
  // 目标须 hidden 才代理（防止「还原为素材」途中脏引用显示普通资产）；失效引用由开库自检清除
  let thumb_meta = meta
    .thumb_asset_id
    .as_deref()
    .and_then(|id| lib.index.assets.get(id))
    .filter(|t| t.hidden == Some(true));
  let source = thumb_meta.unwrap_or(meta);
  let thumb_kind = if source.thumbnail.is_some() {
    Some(ThumbKind::Asset)
  } else if icons.contains(icon_key(&source.ext)) {
    Some(ThumbKind::FormatIcon)
  } else {
    None
  };
  // 播放字段同代理（卡片动图播放：fla 的 animated/frame_count/fps/duration/thumb_frame
  // 全从伴生资产来——帧流开流目标走 play_asset_id）；无代理 = 自身值
  let (frame_count, fps, duration, thumb_frame) = match thumb_meta {
    Some(t) => (t.frame_count, t.fps, t.duration, t.thumb_frame.clone()),
    None => (meta.frame_count, meta.fps, meta.duration, meta.thumb_frame.clone()),
  };
  let animated = frame_count.is_some() && matches!(source.ext.as_str(), "gif" | "webp");
  PlacementView {
    asset_id: p.asset.clone(),
    folder_id: p.folder.clone(),
    name: p.name.clone(),
    w: meta.width,
    h: meta.height,
    thumb_w: source.thumb_width,
    thumb_h: source.thumb_height,
    ext: meta.ext.clone(),
    thumb_path: thumb_path(lib, source, icons),
    thumb_kind,
    origin: p.origin,
    frame_count,
    animated,
    duration,
    bpm: meta.bpm,
    imported_from: meta.imported_from.clone(),
    size: meta.size,
    created_at: meta.created_at.clone(),
    modified_at: meta.modified_at.clone(),
    imported_at: meta.imported_at.clone(),
    note_count: meta.note_count,
    thumb_frame,
    fps,
    hidden: meta.hidden == Some(true),
    play_asset_id: thumb_meta.map(|t| t.id.clone()),
    tags: meta.tags.clone(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library;
  use crate::services::import::{self as ops, PendingFile, Source};
  use std::collections::HashMap;

  /// 伴生缩略图代理（8.1，PRD 02-15）：工程资产带 thumb_asset_id 时，
  /// 缩略图四件（路径 / 来源 / 尺寸）取目标 hidden 资产；目标未 hidden 不代理（半态护栏）
  #[test]
  fn thumb_asset_id_proxies_thumb_fields() {
    let dir = tempfile::tempdir().unwrap();
    let mut lib = library::create_test(dir.path());
    // 造资产：fla（fake 字节）+ webp 缩略图资产（fake 字节；thumbnail 字段直接内存注入——
    // 本测试只验证代理指向，不读真实图）
    for (id, file) in [("AN-F", "工程.fla"), ("AN-T", "封面.webp")] {
      let src = dir.path().join(file);
      std::fs::write(&src, b"fake").unwrap();
      let pf = PendingFile { path: src, chain: Vec::new() };
      let meta = ops::build_asset_with_id(&lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
      lib.index.assets.insert(id.into(), meta);
      ops::add_placement(&mut lib, id.into(), "root".into(), file.into(), true, &mut HashMap::new());
    }
    {
      let t = lib.index.assets.get_mut("AN-T").unwrap();
      t.hidden = Some(true);
      t.thumbnail = Some("封面_thumbnail.webp".into());
      t.thumb_width = Some(320);
      t.thumb_height = Some(180);
      t.frame_count = Some(17); // 伴生 WebP 是动图（播放代理验证）
      t.fps = Some(25.0);
      t.duration = Some(0.68);
    }
    lib.index.assets.get_mut("AN-F").unwrap().thumb_asset_id = Some("AN-T".into());

    let meta_f = lib.index.assets["AN-F"].clone();
    let p = lib.index.placements_of("AN-F")[0].clone();
    let view = placement_view(&lib, &meta_f, &p, &lib.format_icons);
    let thumb = view.thumb_path.unwrap();
    assert!(thumb.contains("AN-T"), "代理取 hidden 资产的缩略图路径：{thumb}");
    assert_eq!(view.thumb_kind, Some(ThumbKind::Asset));
    assert_eq!((view.thumb_w, view.thumb_h), (Some(320), Some(180)), "布局尺寸跟随缩略图资产");
    assert!(!view.hidden, "工程侧自身不 hidden");
    // 播放字段代理（卡片动图播放：fla 卡片 animated + 帧流开流目标 = 伴生资产）
    assert!(view.animated, "fla 卡片按伴生 WebP 显示为动图");
    assert_eq!(view.frame_count, Some(17));
    assert_eq!(view.fps, Some(25.0));
    assert_eq!(view.duration, Some(0.68));
    assert_eq!(view.play_asset_id.as_deref(), Some("AN-T"));
    let view_t = placement_view(&lib, &lib.index.assets["AN-T"].clone(), &lib.index.placements_of("AN-T")[0].clone(), &lib.format_icons);
    assert!(view_t.hidden, "hidden 资产视图带标记（前端过滤依据）");
    assert_eq!(view_t.play_asset_id, None, "hidden 资产自身无代理");

    // 半态护栏：目标未 hidden 时不代理（回退自身缩略图逻辑）
    lib.index.assets.get_mut("AN-T").unwrap().hidden = None;
    let view2 = placement_view(&lib, &meta_f, &p, &lib.format_icons);
    assert!(!view2.thumb_path.map(|t| t.contains("AN-T")).unwrap_or(false), "目标未 hidden 不代理");
    assert!(!view2.animated && view2.play_asset_id.is_none(), "播放代理同解除");
  }
}
