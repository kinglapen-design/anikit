use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use sha2::{Digest, Sha256};

use crate::commands::library::AppState;
use crate::core::json_io::write_json;
use crate::core::library::{Library, CACHE_DIR};
use crate::core::models::{new_asset_id, AssetMeta, ROOT_FOLDER_ID};
use crate::services::import::{self, PendingFile, Source};
use crate::services::{anim, operations, sequence, thumbnail};
use crate::services::views::{placement_view, PlacementView};

use super::{media, model::{self, ModelManifest}, segment_export, storage::SemanticStore, types::{SegmentRange, SegmentHit}};

pub struct PreparedSegment {
  source_id: String,
  meta: AssetMeta,
  target_folder: String,
  placement_name: String,
  info_dir: PathBuf,
  backup: PathBuf,
}

pub fn prepare(state: &AppState, asset_id: &str, segment_id: i64, target_folder: Option<&str>, physical: bool) -> Result<PreparedSegment, String> {
  prepare_at(state, asset_id, segment_id, target_folder, physical, &model::model_dir()?)
}

pub(crate) fn prepare_at(state: &AppState, asset_id: &str, segment_id: i64, target_folder: Option<&str>, physical: bool, model_dir: &std::path::Path) -> Result<PreparedSegment, String> {
  let (root, assets_dir, source, folder) = {
    let guard = state.library.lock().map_err(|e| e.to_string())?;
    let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
    let source = lib.index.assets.get(asset_id).cloned().ok_or_else(|| "来源资源不存在".to_string())?;
    if source.r#type.as_deref() == Some("segment") {
      return Err("不支持嵌套虚拟片段，请从原始素材创建".into());
    }
    let folder = target_folder.unwrap_or(ROOT_FOLDER_ID);
    operations::check_folder(lib, folder)?;
    (lib.root.clone(), lib.assets_dir.clone(), source, folder.to_string())
  };
  let manifest = ModelManifest::load(model_dir)?;
  let fingerprint = media::source_fingerprint(&source, &assets_dir, &source.id)?;
  let store = SemanticStore::open_library(&root)?;
  store.ensure_provider_ready(&source.id, &manifest.revision, media::ANALYSIS_VERSION)?;
  if store.asset_source_fingerprint(&source.id)?.as_deref() != Some(fingerprint.as_str()) {
    return Err("源文件已变化，请重新分析后再创建片段".into());
  }
  let hit = store.asset_segments(&source.id)?.into_iter().find(|hit| hit.segment_id == segment_id)
    .ok_or_else(|| "语义片段不存在，请重新搜索后再创建".to_string())?;
  let range = hit_range(&hit)?;
  if sequence::frames_dir_if_sequence(&assets_dir, &source.id, &source).is_some() || anim::is_anim(&source) {
    range.frame_bounds()?;
  }
  build(root, assets_dir, source, folder, range, physical)
}

fn build(root: PathBuf, assets_dir: PathBuf, source: AssetMeta, folder: String, range: SegmentRange, physical: bool) -> Result<PreparedSegment, String> {
  let id = new_asset_id();
  let info_dir = crate::core::paths::info_dir(&assets_dir, &id);
  if info_dir.exists() {
    return Err("资源 ID 冲突，请重试".into());
  }
  let work_dir = root.join(CACHE_DIR).join("tmp").join("semantic-segments").join(&id);
  let name = format!("{} {}-{}ms", source.name, range.start_ms, range.end_ms);
  let cancel = AtomicBool::new(false);
  let result = (|| {
    let (meta, placement_name) = if physical {
      let output = segment_export::export(&source, &assets_dir, &range, &work_dir, &name, &cancel)?;
      let file = PendingFile { path: output.path, chain: Vec::new() };
      let hash = import::sha256_file(&file.path).map_err(|e| format!("读取物理片段失败：{e}"))?;
      let mut meta = import::build_asset_with_id(&assets_dir, &file, &hash, &id, Source::Copy, None)?;
      if meta.ext != output.extension {
        return Err("物理片段的导出格式与入库格式不一致".into());
      }
      attach_source(&mut meta, &source, &range);
      meta.name = name.clone();
      write_json(&info_dir.join("metadata.json"), &meta).map_err(|e| e.to_string())?;
      let placement_name = meta.origin_name.clone();
      (meta, placement_name)
    } else {
      let origin = crate::core::paths::info_dir(&assets_dir, &source.id).join(&source.origin_name);
      let kind = segment_export::media_kind(&source, &assets_dir, &origin)?;
      let mut meta = source.clone();
      attach_source(&mut meta, &source, &range);
      meta.id = id.clone();
      meta.name = name.clone();
      meta.r#type = Some("segment".into());
      meta.size = 0;
      meta.hash = format!("sha256:{:x}", Sha256::digest(format!("{}:{}:{}:{:?}:{:?}", source.hash, range.start_ms, range.end_ms, range.start_frame, range.end_frame)));
      meta.duration = Some(range.duration_secs());
      meta.frame_count = if kind == media::MediaKind::Video { None } else { Some(range.frame_count()) };
      meta.thumb_frame = match kind {
        media::MediaKind::Video => Some(format!("t:{:.3}", range.start_ms as f64 / 1000.0)),
        media::MediaKind::Anim => Some(format!("f:{}", range.frame_bounds()?.0)),
        media::MediaKind::Sequence => {
          let dir = sequence::frames_dir_if_sequence(&assets_dir, &source.id, &source).ok_or_else(|| "序列帧目录不存在".to_string())?;
          let frames = sequence::ordered_frame_paths(&dir)?;
          let index = range.frame_bounds()?.0 as usize;
          frames.get(index).and_then(|path| path.file_name()).map(|name| name.to_string_lossy().into_owned())
        }
      };
      let now = operations::now();
      meta.imported_at = now.clone();
      meta.modified_at = now.clone();
      meta.created_at = Some(now);
      meta.imported_from = None;
      meta.rating = 0;
      meta.links.clear();
      meta.tags.clear();
      meta.descriptions.clear();
      meta.note.clear();
      meta.note_count = None;
      meta.hidden = None;
      meta.thumb_asset_id = None;
      meta.prompt = None;
      fs::create_dir_all(&info_dir).map_err(|e| format!("创建虚拟片段目录失败：{e}"))?;
      let cover = segment_export::write_evidence_frame(&source, &assets_dir, &range, &work_dir, &cancel)?;
      let thumb = thumbnail::generate(&info_dir, &cover, &name).ok_or_else(|| "片段缩略图生成失败".to_string())?;
      meta.thumbnail = Some(thumb.name);
      meta.thumb_width = Some(thumb.width);
      meta.thumb_height = Some(thumb.height);
      meta.colors = thumb.colors;
      write_json(&info_dir.join("metadata.json"), &meta).map_err(|e| e.to_string())?;
      let placement_name = format!("{name}.{}", meta.ext);
      (meta, placement_name)
    };
    Ok(PreparedSegment {
      source_id: source.id,
      meta,
      target_folder: folder,
      placement_name,
      backup: root.join(CACHE_DIR).join("tmp").join(format!("undo-{id}.info")),
      info_dir: info_dir.clone(),
    })
  })();
  if result.is_err() {
    let _ = fs::remove_dir_all(&info_dir);
  }
  let _ = fs::remove_dir_all(&work_dir);
  result
}

fn attach_source(meta: &mut AssetMeta, source: &AssetMeta, range: &SegmentRange) {
  meta.source = Some(source.id.clone());
  meta.start = Some(range.start_ms as f64 / 1000.0);
  meta.end = Some(range.end_ms as f64 / 1000.0);
  meta.start_frame = range.start_frame;
  meta.end_frame = range.end_frame;
}

pub fn commit(lib: &mut Library, prepared: PreparedSegment) -> Result<(PlacementView, PathBuf, PathBuf), String> {
  let result = (|| {
    operations::check_folder(lib, &prepared.target_folder)?;
    if !lib.index.assets.contains_key(&prepared.source_id) {
      return Err("来源资源已删除，未创建片段".into());
    }
    let id = prepared.meta.id.clone();
    let placement = import::add_placement(lib, id.clone(), prepared.target_folder, prepared.placement_name, true, &mut HashMap::new());
    lib.index.assets.insert(id, prepared.meta.clone());
    operations::finish(lib)?;
    Ok((placement_view(lib, &prepared.meta, &placement, &lib.format_icons), prepared.info_dir.clone(), prepared.backup))
  })();
  if result.is_err() {
    let _ = fs::remove_dir_all(&prepared.info_dir);
  }
  result
}

pub fn discard(prepared: &PreparedSegment) {
  if let Err(error) = fs::remove_dir_all(&prepared.info_dir) {
    log::warn!("清理未提交的片段失败：{error}");
  }
}

pub fn hit_range(hit: &SegmentHit) -> Result<SegmentRange, String> {
  SegmentRange::from_hit(hit)
}

#[cfg(test)]
mod tests;
