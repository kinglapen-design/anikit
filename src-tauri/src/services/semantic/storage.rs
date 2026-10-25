#![allow(dead_code)] // V1～V5 分批接线的公开契约；语义搜索 / 分析任务 / MCP 接入后移除

use std::{cmp::Ordering, fs, path::Path};

use rusqlite::{params, Connection};

use super::types::{ModelRevision, SegmentHit, SegmentInput};

const SCHEMA_VERSION: i32 = 1;

pub struct SemanticStore {
  pub(crate) conn: Connection,
}

impl SemanticStore {
  pub fn open(path: &Path) -> Result<Self, String> {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent).map_err(|e| format!("创建语义索引目录失败：{e}"))?;
    }
    let conn = Connection::open(path).map_err(|e| format!("打开语义索引失败：{e}"))?;
    conn.pragma_update(None, "foreign_keys", "ON").map_err(|e| e.to_string())?;
    conn.pragma_update(None, "journal_mode", "WAL").map_err(|e| e.to_string())?;
    conn
      .execute_batch(
        "CREATE TABLE IF NOT EXISTS semantic_models(
          id TEXT NOT NULL,
          revision TEXT NOT NULL,
          dimension INTEGER NOT NULL,
          weight_sha256 TEXT NOT NULL,
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
          PRIMARY KEY(id, revision)
        );
        CREATE TABLE IF NOT EXISTS semantic_assets(
          asset_id TEXT PRIMARY KEY,
          media_kind TEXT NOT NULL,
          source_fingerprint TEXT NOT NULL,
          model_id TEXT NOT NULL,
          model_revision TEXT NOT NULL,
          analysis_version INTEGER NOT NULL,
          status TEXT NOT NULL,
          error TEXT,
          FOREIGN KEY(model_id, model_revision) REFERENCES semantic_models(id, revision)
        );
        CREATE TABLE IF NOT EXISTS semantic_segments(
          id INTEGER PRIMARY KEY,
          asset_id TEXT NOT NULL,
          start_ms INTEGER NOT NULL,
          end_ms INTEGER NOT NULL,
          start_frame INTEGER,
          end_frame INTEGER,
          kind TEXT NOT NULL,
          text TEXT NOT NULL,
          vector BLOB NOT NULL,
          dimension INTEGER NOT NULL,
          FOREIGN KEY(asset_id) REFERENCES semantic_assets(asset_id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS semantic_segments_asset ON semantic_segments(asset_id, start_ms);
        CREATE TABLE IF NOT EXISTS analysis_tasks(
          id TEXT PRIMARY KEY,
          library_root TEXT NOT NULL,
          assets_dir TEXT NOT NULL,
          device TEXT NOT NULL,
          status TEXT NOT NULL,
          total INTEGER NOT NULL,
          completed INTEGER NOT NULL DEFAULT 0,
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
          updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS analysis_task_items(
          task_id TEXT NOT NULL,
          asset_id TEXT NOT NULL,
          status TEXT NOT NULL,
          error TEXT,
          PRIMARY KEY(task_id, asset_id),
          FOREIGN KEY(task_id) REFERENCES analysis_tasks(id) ON DELETE CASCADE
        );
        PRAGMA user_version = 1;",
      )
      .map_err(|e| format!("初始化语义索引失败：{e}"))?;
    let version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).map_err(|e| e.to_string())?;
    if version != SCHEMA_VERSION {
      return Err(format!("不支持的语义索引版本：{version}"));
    }
    Ok(Self { conn })
  }

  pub fn register_model(&self, model: &ModelRevision) -> Result<(), String> {
    if model.dimension == 0 || model.weight_sha256.len() != 64 {
      return Err("模型元数据无效".into());
    }
    self
      .conn
      .execute(
        "INSERT INTO semantic_models(id, revision, dimension, weight_sha256) VALUES(?1, ?2, ?3, ?4)
         ON CONFLICT(id, revision) DO UPDATE SET dimension=excluded.dimension, weight_sha256=excluded.weight_sha256",
        params![model.id, model.revision, model.dimension as i64, model.weight_sha256],
      )
      .map_err(|e| format!("登记模型失败：{e}"))?;
    Ok(())
  }

  pub fn replace_asset_analysis(
    &mut self,
    asset_id: &str,
    media_kind: &str,
    source_fingerprint: &str,
    model: &ModelRevision,
    analysis_version: u32,
    segments: &[SegmentInput],
  ) -> Result<(), String> {
    let normalized = segments.iter().map(|segment| normalize(&segment.vector)).collect::<Result<Vec<_>, _>>()?;
    if normalized.iter().any(|vector| vector.len() != model.dimension) {
      return Err("片段向量维度与模型不一致".into());
    }
    if segments.iter().any(|segment| segment.start_ms < 0 || segment.start_ms > segment.end_ms) {
      return Err("片段时间区间无效".into());
    }
    let tx = self.conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
      "INSERT INTO semantic_assets(asset_id, media_kind, source_fingerprint, model_id, model_revision, analysis_version, status)
       VALUES(?1, ?2, ?3, ?4, ?5, ?6, 'ready')
       ON CONFLICT(asset_id) DO UPDATE SET media_kind=excluded.media_kind,
         source_fingerprint=excluded.source_fingerprint, model_id=excluded.model_id,
         model_revision=excluded.model_revision, analysis_version=excluded.analysis_version,
         status='ready', error=NULL",
      params![asset_id, media_kind, source_fingerprint, model.id, model.revision, analysis_version],
    )
    .map_err(|e| format!("写入分析资产失败：{e}"))?;
    tx.execute("DELETE FROM semantic_segments WHERE asset_id=?1", [asset_id]).map_err(|e| e.to_string())?;
    for (segment, vector) in segments.iter().zip(normalized) {
      tx.execute(
        "INSERT INTO semantic_segments(asset_id, start_ms, end_ms, start_frame, end_frame, kind, text, vector, dimension)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
          asset_id,
          segment.start_ms,
          segment.end_ms,
          segment.start_frame,
          segment.end_frame,
          segment.kind,
          segment.text,
          encode_vector(&vector),
          model.dimension as i64
        ],
      )
      .map_err(|e| format!("写入语义片段失败：{e}"))?;
    }
    tx.commit().map_err(|e| format!("提交语义分析失败：{e}"))?;
    Ok(())
  }

  pub fn search(&self, model: &ModelRevision, query: &[f32], limit: usize) -> Result<Vec<SegmentHit>, String> {
    let query = normalize(query)?;
    if query.len() != model.dimension {
      return Err("查询向量维度与模型不一致".into());
    }
    let mut stmt = self
      .conn
      .prepare(
        "SELECT s.id, s.asset_id, s.start_ms, s.end_ms, s.start_frame, s.end_frame, s.kind, s.text, s.vector
         FROM semantic_segments s JOIN semantic_assets a ON a.asset_id=s.asset_id
         WHERE a.model_id=?1 AND a.model_revision=?2 AND a.status='ready'",
      )
      .map_err(|e| e.to_string())?;
    let rows = stmt
      .query_map(params![model.id, model.revision], |row| {
        let bytes: Vec<u8> = row.get(8)?;
        Ok((
          SegmentHit {
            segment_id: row.get(0)?,
            asset_id: row.get(1)?,
            start_ms: row.get(2)?,
            end_ms: row.get(3)?,
            start_frame: row.get(4)?,
            end_frame: row.get(5)?,
            score: 0.0,
            kind: row.get(6)?,
            text: row.get(7)?,
          },
          bytes,
        ))
      })
      .map_err(|e| format!("读取语义片段失败：{e}"))?;
    let mut hits = Vec::new();
    for row in rows {
      let (mut hit, bytes) = row.map_err(|e| e.to_string())?;
      let vector = decode_vector(&bytes)?;
      if vector.len() == query.len() {
        hit.score = dot(&query, &vector);
        hits.push(hit);
      }
    }
    hits.sort_by(|left, right| {
      right.score.partial_cmp(&left.score).unwrap_or(Ordering::Equal).then(left.segment_id.cmp(&right.segment_id))
    });
    hits.truncate(limit);
    Ok(hits)
  }

  pub fn segment_count(&self, asset_id: &str) -> Result<i64, String> {
    self
      .conn
      .query_row("SELECT COUNT(*) FROM semantic_segments WHERE asset_id=?1", [asset_id], |row| row.get(0))
      .map_err(|e| e.to_string())
  }
}

fn normalize(vector: &[f32]) -> Result<Vec<f32>, String> {
  if vector.is_empty() || vector.iter().any(|value| !value.is_finite()) {
    return Err("向量包含空值或非法数值".into());
  }
  let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
  if !norm.is_finite() || norm <= 0.0 {
    return Err("向量无法归一化".into());
  }
  Ok(vector.iter().map(|value| value / norm).collect())
}

fn encode_vector(vector: &[f32]) -> Vec<u8> {
  vector.iter().flat_map(|value| value.to_le_bytes()).collect()
}

#[allow(clippy::manual_is_multiple_of, clippy::chunks_exact_to_as_chunks)] // MSRV 1.80：两个新版 API 均不可用
fn decode_vector(bytes: &[u8]) -> Result<Vec<f32>, String> {
  if bytes.len() % 4 != 0 {
    return Err("向量二进制长度无效".into());
  }
  Ok(bytes.chunks_exact(4).map(|chunk| f32::from_le_bytes(chunk.try_into().expect("four bytes"))).collect())
}

fn dot(left: &[f32], right: &[f32]) -> f32 {
  left.iter().zip(right).map(|(left, right)| left * right).sum()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn model(dimension: usize) -> ModelRevision {
    ModelRevision { id: "test-clip".into(), revision: "r1".into(), dimension, weight_sha256: "a".repeat(64) }
  }

  fn segment(start_ms: i64, vector: Vec<f32>) -> SegmentInput {
    SegmentInput {
      start_ms,
      end_ms: start_ms + 1000,
      start_frame: Some((start_ms / 1000) as u32),
      end_frame: Some((start_ms / 1000) as u32 + 1),
      kind: "visual".into(),
      text: format!("segment-{start_ms}"),
      vector,
    }
  }

  fn store() -> (tempfile::TempDir, SemanticStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = SemanticStore::open(&dir.path().join("vectors/semantic.sqlite3")).unwrap();
    store.register_model(&model(2)).unwrap();
    (dir, store)
  }

  #[test]
  fn replacement_is_idempotent_and_search_is_deterministic() {
    let (_dir, mut store) = store();
    store
      .replace_asset_analysis("a1", "video", "fp1", &model(2), 1, &[segment(0, vec![1.0, 0.0]), segment(1000, vec![0.0, 1.0])])
      .unwrap();
    store.replace_asset_analysis("a1", "video", "fp2", &model(2), 1, &[segment(2000, vec![0.0, 2.0])]).unwrap();
    assert_eq!(store.segment_count("a1").unwrap(), 1);
    let hits = store.search(&model(2), &[0.0, 3.0], 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].start_ms, 2000);
    assert_eq!(hits[0].start_frame, Some(2));
    assert_eq!(hits[0].kind, "visual");
    assert!((hits[0].score - 1.0).abs() < 0.00001);
    assert!(store.search(&model(3), &[1.0, 0.0], 10).is_err());
  }

  #[test]
  fn invalid_vectors_and_intervals_are_rejected() {
    let (_dir, mut store) = store();
    assert!(store.replace_asset_analysis("bad", "video", "fp", &model(2), 1, &[segment(0, vec![0.0, 0.0])]).is_err());
    let mut reversed = segment(1000, vec![1.0, 0.0]);
    reversed.end_ms = 0;
    assert!(store.replace_asset_analysis("bad", "video", "fp", &model(2), 1, &[reversed]).is_err());
    assert_eq!(store.segment_count("bad").unwrap(), 0);
  }
}
