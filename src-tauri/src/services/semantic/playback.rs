use std::path::{Path, PathBuf};

use crate::core::{json_io::read_json, models::AssetMeta, paths};

use super::types::SegmentRange;

#[derive(Debug)]
pub struct PlaybackAsset {
  pub target_info_dir: PathBuf,
  pub info_dir: PathBuf,
  pub origin: PathBuf,
  pub meta: AssetMeta,
  pub source: Option<AssetMeta>,
  pub segment: Option<SegmentRange>,
}

impl PlaybackAsset {
  pub fn media_meta(&self) -> &AssetMeta {
    self.source.as_ref().unwrap_or(&self.meta)
  }
}

pub fn resolve(assets_dir: &Path, asset_id: &str) -> Result<PlaybackAsset, String> {
  let info_dir = paths::info_dir(assets_dir, asset_id);
  if !info_dir.is_dir() {
    return Err(format!("资源不存在：{asset_id}"));
  }
  let meta: AssetMeta = read_json(&info_dir.join("metadata.json")).map_err(|e| e.to_string())?;
  if meta.r#type.as_deref() != Some("segment") {
    let origin = paths::origin_file(assets_dir, asset_id, &meta.origin_name);
    return Ok(PlaybackAsset { target_info_dir: info_dir.clone(), info_dir, origin, meta, source: None, segment: None });
  }
  let source_id = meta.source.as_deref().filter(|id| !id.is_empty()).ok_or_else(|| "虚拟片段缺少来源资源".to_string())?;
  if source_id == asset_id {
    return Err("虚拟片段不能引用自身".into());
  }
  let source_dir = paths::info_dir(assets_dir, source_id);
  if !source_dir.is_dir() {
    return Err("虚拟片段的来源资源已删除，无法播放".into());
  }
  let source: AssetMeta = read_json(&source_dir.join("metadata.json")).map_err(|e| format!("虚拟片段来源读取失败：{e}"))?;
  if source.r#type.as_deref() == Some("segment") {
    return Err("不支持嵌套虚拟片段，请从原始素材创建".into());
  }
  let segment = range_from_meta(&meta)?;
  let origin = paths::origin_file(assets_dir, source_id, &source.origin_name);
  Ok(PlaybackAsset { target_info_dir: info_dir, info_dir: source_dir, origin, meta, source: Some(source), segment })
}

pub fn range_from_meta(meta: &AssetMeta) -> Result<Option<SegmentRange>, String> {
  if meta.r#type.as_deref() != Some("segment") {
    return Ok(None);
  }
  let start = seconds_ms(meta.start, "起点")?;
  let end = seconds_ms(meta.end, "终点")?;
  let range = SegmentRange { start_ms: start, end_ms: end, start_frame: meta.start_frame, end_frame: meta.end_frame };
  range.validate()?;
  Ok(Some(range))
}

fn seconds_ms(value: Option<f64>, label: &str) -> Result<i64, String> {
  let value = value.ok_or_else(|| format!("虚拟片段缺少{label}"))?;
  let millis = value * 1000.0;
  if !millis.is_finite() || millis < 0.0 || millis > i64::MAX as f64 {
    return Err(format!("虚拟片段{label}无效"));
  }
  Ok(millis.round() as i64)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn meta() -> AssetMeta {
    serde_json::from_value(serde_json::json!({
      "version": 1, "id": "a", "name": "a", "originName": "a.gif", "ext": "gif",
      "size": 0, "rating": 0, "links": [], "importedAt": "2026-10-27T00:00:00+08:00",
      "modifiedAt": "2026-10-27T00:00:00+08:00", "tags": [], "note": ""
    })).unwrap()
  }

  #[test]
  fn ranges_preserve_seconds_and_original_frame_indexes() {
    let mut meta = meta();
    meta.r#type = Some("segment".into());
    meta.start = Some(1.25);
    meta.end = Some(2.5);
    meta.start_frame = Some(17);
    meta.end_frame = Some(23);
    let range = range_from_meta(&meta).unwrap().unwrap();
    assert_eq!((range.start_ms, range.end_ms), (1250, 2500));
    assert_eq!(range.frame_bounds().unwrap(), (17, 23));
    assert_eq!(range.frame_count(), 7);
    meta.end_frame = None;
    assert!(range_from_meta(&meta).is_err());
    meta.end_frame = Some(16);
    assert!(range_from_meta(&meta).is_err());
  }
}
