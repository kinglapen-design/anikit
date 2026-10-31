use std::collections::{BTreeMap, HashMap};

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
  let store = SemanticStore::open_library(&root)?;
  if store.segment_total()? == 0 {
    return Ok(SemanticSearchHits { hits: Vec::new(), actual_backend: "unknown".into(), model_revision: model::MODEL_REVISION.into() });
  }
  let manifest = ModelManifest::load(&model::model_dir()?)?;
  let query_embedding = worker::embed_text(query.trim())?;
  let limit = limit.unwrap_or(30).clamp(1, 100);
  let hits = store.search(&manifest.revision, super::media::ANALYSIS_VERSION, &query_embedding.embedding, limit)?;
  let mut grouped = BTreeMap::new();
  for hit in hits {
    let Some(placements) = placement_map.get(&hit.asset_id) else { continue };
    let entry = grouped.entry(hit.asset_id.clone()).or_insert_with(|| SemanticAssetHit {
      placements: placements.clone(), best: hit.clone(), segments: Vec::new(), match_count: 0,
    });
    if hit.score > entry.best.score {
      entry.best = hit.clone();
    }
    entry.segments.push(hit);
  }
  let mut hits = grouped.into_values().collect::<Vec<_>>();
  for hit in &mut hits {
    hit.match_count = store.segment_count_for_model(&hit.best.asset_id, &manifest.revision, super::media::ANALYSIS_VERSION)?;
    hit.segments.sort_by(|left, right| right.score.partial_cmp(&left.score).unwrap_or(std::cmp::Ordering::Equal));
  }
  hits.sort_by(|left, right| right.best.score.partial_cmp(&left.best.score).unwrap_or(std::cmp::Ordering::Equal).then(left.best.asset_id.cmp(&right.best.asset_id)));
  Ok(SemanticSearchHits { hits, actual_backend: query_embedding.actual_backend, model_revision: manifest.revision.revision })
}
