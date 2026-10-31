use rusqlite::{params, OptionalExtension};

use super::{provider_types::SemanticDocumentInput, storage::{encode_vector, normalize, SemanticStore}, types::{ModelRevision, SegmentInput}};

impl SemanticStore {
  pub fn ensure_provider_ready(&self, asset_id: &str, model: &ModelRevision, analysis_version: u32) -> Result<(), String> {
    let readiness = self.conn.query_row(
      "SELECT model_id, model_revision, analysis_version, status FROM semantic_assets WHERE asset_id=?1",
      [asset_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, u32>(2)?, row.get::<_, String>(3)?)),
    ).optional().map_err(|e| e.to_string())?.ok_or_else(|| "请先进行基础画面分析".to_string())?;
    if readiness.0 != model.id || readiness.1 != model.revision || readiness.2 != analysis_version {
      return Err("基础分析与当前模型或分析版本不一致，请重新分析".into());
    }
    if readiness.3 != "ready" {
      return Err("基础画面分析尚未完成，请重新分析后再进行详细分析".into());
    }
    Ok(())
  }

  pub fn fail_provider_analysis(&self, asset_id: &str, error: &str) -> Result<(), String> {
    self.conn.execute("UPDATE semantic_assets SET error=?2 WHERE asset_id=?1", params![asset_id, error])
      .map_err(|e| format!("记录外部分析失败：{e}"))?;
    Ok(())
  }

  pub fn replace_provider_analysis(&mut self, asset_id: &str, model: &ModelRevision, documents: &[SemanticDocumentInput]) -> Result<(), String> {
    self.ensure_provider_ready(asset_id, model, super::media::ANALYSIS_VERSION)?;
    for document in documents {
      if !matches!(document.kind.as_str(), "description" | "event") {
        return Err("外部分析文档类型无效".into());
      }
      if !matches!(document.certainty.as_str(), "observed" | "hypothesis") {
        return Err("外部分析确定性标记无效".into());
      }
      if document.text.trim().is_empty() || document.start_ms < 0 || document.start_ms > document.end_ms {
        return Err("外部分析文档内容或时间区间无效".into());
      }
      if normalize(&document.vector)?.len() != model.dimension {
        return Err("外部分析向量维度与模型不一致".into());
      }
    }
    let tx = self.conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM semantic_segments WHERE asset_id=?1 AND kind IN ('description','event')", [asset_id]).map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM semantic_documents WHERE asset_id=?1", [asset_id]).map_err(|e| e.to_string())?;
    for document in documents {
      let segment = SegmentInput {
        start_ms: document.start_ms, end_ms: document.end_ms,
        start_frame: document.start_frame, end_frame: document.end_frame,
        kind: document.kind.clone(), text: document.text.clone(), vector: document.vector.clone(),
      };
      tx.execute(
        "INSERT INTO semantic_segments(asset_id, start_ms, end_ms, start_frame, end_frame, kind, text, vector, dimension)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![asset_id, segment.start_ms, segment.end_ms, segment.start_frame, segment.end_frame,
          segment.kind, segment.text, encode_vector(&normalize(&segment.vector)?), model.dimension as i64],
      ).map_err(|e| format!("写入外部分析片段失败：{e}"))?;
      tx.execute(
        "INSERT INTO semantic_documents(segment_id, asset_id, kind, certainty, provider, model, prompt_version, source_json, frame_ms)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![tx.last_insert_rowid(), asset_id, document.kind, document.certainty, document.provider,
          document.model, document.prompt_version, document.source_json, serde_json::to_string(&document.frame_ms).map_err(|e| e.to_string())?],
      ).map_err(|e| format!("保存外部分析来源失败：{e}"))?;
    }
    tx.commit().map_err(|e| format!("提交外部分析失败：{e}"))?;
    Ok(())
  }

  #[cfg(test)]
  pub fn provider_document_count(&self, asset_id: &str) -> Result<i64, String> {
    self.conn.query_row("SELECT COUNT(*) FROM semantic_documents WHERE asset_id=?1", [asset_id], |row| row.get(0)).map_err(|e| e.to_string())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::services::semantic::types::SegmentHit;

  fn model() -> ModelRevision {
    ModelRevision { id: "test-clip".into(), revision: "r1".into(), dimension: 2, weight_sha256: "a".repeat(64) }
  }

  #[test]
  fn provider_documents_preserve_visuals_and_record_certainty() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = SemanticStore::open(&dir.path().join("semantic.sqlite3")).unwrap();
    store.register_model(&model()).unwrap();
    let visual = SegmentInput {
      start_ms: 0, end_ms: 4000, start_frame: None, end_frame: None,
      kind: "visual".into(), text: "0–4000".into(), vector: vec![1.0, 0.0],
    };
    store.replace_asset_analysis("a1", "video", "fp", &model(), 1, &[visual.clone()]).unwrap();
    let document = SemanticDocumentInput {
      kind: "event".into(), text: "连续事件（观察）：机器人洗碗后碗碎裂".into(),
      start_ms: 1000, end_ms: 2000, start_frame: Some(1), end_frame: Some(2), certainty: "hypothesis".into(),
      provider: "ollama".into(), model: "test-vlm".into(), prompt_version: "semantic-v1".into(),
      source_json: "{\"text\":\"机器人洗碗后碗碎裂\"}".into(), frame_ms: vec![1000, 2000], vector: vec![0.0, 1.0],
    };
    store.replace_provider_analysis("a1", &model(), std::slice::from_ref(&document)).unwrap();
    assert_eq!(store.provider_document_count("a1").unwrap(), 1);
    assert_eq!(store.segment_count("a1").unwrap(), 2);
    let hits = store.search(&model(), 1, &[0.0, 1.0], 10).unwrap();
    assert_eq!(hits[0].certainty.as_deref(), Some("hypothesis"));
    assert!(hits.iter().any(|hit| hit.kind == "visual"));
    store.replace_asset_analysis("a1", "video", "fp2", &model(), 1, &[visual]).unwrap();
    assert_eq!(store.provider_document_count("a1").unwrap(), 0);
    assert!(store.asset_segments("a1").unwrap().iter().all(|hit: &SegmentHit| hit.kind == "visual"));
  }

  #[test]
  fn provider_readiness_requires_current_ready_analysis() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = SemanticStore::open(&dir.path().join("semantic.sqlite3")).unwrap();
    store.register_model(&model()).unwrap();
    assert!(store.ensure_provider_ready("missing", &model(), 1).is_err());
    let visual = SegmentInput {
      start_ms: 0, end_ms: 1000, start_frame: None, end_frame: None,
      kind: "visual".into(), text: "0–1000".into(), vector: vec![1.0, 0.0],
    };
    store.replace_asset_analysis("a1", "video", "fp", &model(), 1, &[visual.clone()]).unwrap();
    store.ensure_provider_ready("a1", &model(), 1).unwrap();
    assert!(store.ensure_provider_ready("a1", &model(), 2).is_err());
    store.fail_asset_analysis("a1", "failed").unwrap();
    assert!(store.ensure_provider_ready("a1", &model(), 1).is_err());
    store.replace_asset_analysis("a1", "video", "fp", &model(), 2, &[visual]).unwrap();
    assert!(store.ensure_provider_ready("a1", &model(), 1).is_err());
    store.ensure_provider_ready("a1", &model(), 2).unwrap();
  }
}
