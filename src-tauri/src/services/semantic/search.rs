use std::collections::HashMap;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::{commands::library::AppState, services::search::{self, SearchHitKey, SearchScope}};

use super::{model::{self, ModelManifest}, storage::SemanticStore, types::SegmentHit, worker};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticAssetHit {
  pub placements: Vec<SearchHitKey>,
  pub best: SegmentHit,
  pub segments: Vec<SegmentHit>,
  pub match_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticSearchHits {
  pub hits: Vec<SemanticAssetHit>,
  pub actual_backend: String,
  pub model_revision: String,
}

pub fn semantic_search<R: tauri::Runtime>(app: &AppHandle<R>, query: &str, scope: &SearchScope, limit: Option<usize>) -> Result<SemanticSearchHits, String> {
  if query.trim().is_empty() {
    return Err("请输入语义搜索内容".into());
  }
  let state = app.state::<AppState>();
  let (root, placement_map) = {
    let guard = state.library.lock().map_err(|e| e.to_string())?;
    let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
    let candidates = search::search_candidates(lib, scope);
    let mut map: HashMap<String, Vec<SearchHitKey>> = HashMap::new();
    for placement in candidates.placements {
      map.entry(placement.asset_id.clone()).or_default().push(placement);
    }
    (lib.root.clone(), map)
  };
  if placement_map.is_empty() {
    return Ok(SemanticSearchHits { hits: Vec::new(), actual_backend: "unknown".into(), model_revision: model::MODEL_REVISION.into() });
  }
  let candidate_ids = placement_map.keys().cloned().collect::<std::collections::HashSet<_>>();
  let store = SemanticStore::open_library(&root)?;
  if store.segment_total()? == 0 {
    return Ok(SemanticSearchHits { hits: Vec::new(), actual_backend: "unknown".into(), model_revision: model::MODEL_REVISION.into() });
  }
  if !store.has_ready_segments_for_assets(model::MODEL_ID, model::MODEL_REVISION, super::media::ANALYSIS_VERSION, &candidate_ids)? {
    return Ok(SemanticSearchHits { hits: Vec::new(), actual_backend: "unknown".into(), model_revision: model::MODEL_REVISION.into() });
  }
  let manifest = ModelManifest::load(&model::model_dir()?)?;
  let query_embedding = worker::embed_text(query.trim())?;
  let limit = limit.unwrap_or(30).clamp(1, 100);
  let hits = if scope.global {
    store.search(&manifest.revision, super::media::ANALYSIS_VERSION, &query_embedding.embedding, limit)?
  } else {
    store.search_scoped(&manifest.revision, super::media::ANALYSIS_VERSION, &query_embedding.embedding, &candidate_ids, limit)?
  };
  let mut hits = group_hits(hits, &placement_map);
  for hit in &mut hits {
    hit.match_count = store.segment_count_for_model(&hit.best.asset_id, &manifest.revision, super::media::ANALYSIS_VERSION)?;
  }
  Ok(SemanticSearchHits { hits, actual_backend: query_embedding.actual_backend, model_revision: manifest.revision.revision })
}

fn group_hits(hits: Vec<SegmentHit>, placement_map: &HashMap<String, Vec<SearchHitKey>>) -> Vec<SemanticAssetHit> {
  let mut grouped: Vec<SemanticAssetHit> = Vec::new();
  let mut indices: HashMap<String, usize> = HashMap::new();
  for hit in hits {
    let Some(placements) = placement_map.get(&hit.asset_id) else { continue };
    if let Some(index) = indices.get(&hit.asset_id).copied() {
      grouped[index].segments.push(hit);
      continue;
    }
    indices.insert(hit.asset_id.clone(), grouped.len());
    grouped.push(SemanticAssetHit {
      placements: placements.clone(), best: hit.clone(), segments: vec![hit], match_count: 0,
    });
  }
  grouped
}

#[cfg(test)]
mod tests {
  use super::*;

  fn hit(asset_id: &str, segment_id: i64, kind: &str, score: f32) -> SegmentHit {
    SegmentHit {
      segment_id, asset_id: asset_id.into(), start_ms: 0, end_ms: 1000,
      start_frame: None, end_frame: None, score, kind: kind.into(), certainty: None, text: String::new(),
    }
  }

  #[test]
  fn grouping_preserves_fused_asset_best_and_segment_order() {
    let placements = HashMap::from([
      ("a1".to_string(), vec![SearchHitKey { asset_id: "a1".into(), folder_id: "f1".into() }]),
      ("a2".to_string(), vec![SearchHitKey { asset_id: "a2".into(), folder_id: "f2".into() }]),
    ]);
    let grouped = group_hits(vec![
      hit("a1", 1, "visual", 0.2),
      hit("a1", 2, "event", 0.9),
      hit("a2", 3, "visual", 0.8),
    ], &placements);
    assert_eq!(grouped.iter().map(|hit| hit.best.asset_id.as_str()).collect::<Vec<_>>(), ["a1", "a2"]);
    assert_eq!(grouped[0].best.segment_id, 1);
    assert_eq!(grouped[0].segments.iter().map(|hit| hit.segment_id).collect::<Vec<_>>(), [1, 2]);
  }
}
