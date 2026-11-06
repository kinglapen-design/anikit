use std::{fs, path::{Path, PathBuf}};

use rusqlite::{params, Connection, OptionalExtension};

use super::types::{ModelRevision, SegmentHit, SegmentInput};

const SCHEMA_VERSION: i32 = 4;

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
          mode TEXT NOT NULL DEFAULT 'basic',
          actual_backend TEXT NOT NULL DEFAULT 'unknown',
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
        CREATE TABLE IF NOT EXISTS semantic_documents(
          id INTEGER PRIMARY KEY,
          segment_id INTEGER NOT NULL UNIQUE,
          asset_id TEXT NOT NULL,
          kind TEXT NOT NULL,
          certainty TEXT NOT NULL,
          provider TEXT NOT NULL,
          model TEXT NOT NULL,
          prompt_version TEXT NOT NULL,
          source_json TEXT NOT NULL,
          frame_ms TEXT NOT NULL,
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
          FOREIGN KEY(segment_id) REFERENCES semantic_segments(id) ON DELETE CASCADE,
          FOREIGN KEY(asset_id) REFERENCES semantic_assets(asset_id) ON DELETE CASCADE
        );",
      )
      .map_err(|e| format!("初始化语义索引失败：{e}"))?;
    let version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).map_err(|e| e.to_string())?;
    if version > SCHEMA_VERSION {
      return Err(format!("不支持的语义索引版本：{version}"));
    }
    if version == 1 {
      conn.execute_batch("ALTER TABLE analysis_tasks ADD COLUMN actual_backend TEXT NOT NULL DEFAULT 'unknown';").map_err(|e| format!("迁移语义索引失败：{e}"))?;
    }
    if (1..SCHEMA_VERSION).contains(&version) {
      conn.execute_batch("ALTER TABLE analysis_tasks ADD COLUMN mode TEXT NOT NULL DEFAULT 'basic';").map_err(|e| format!("迁移语义任务模式失败：{e}"))?;
    }
    conn.pragma_update(None, "user_version", SCHEMA_VERSION).map_err(|e| e.to_string())?;
    Ok(Self { conn })
  }

  pub fn open_library(root: &Path) -> Result<Self, String> {
    static RECOVERED: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<PathBuf>>> = std::sync::OnceLock::new();
    let store = Self::open(&root.join(crate::core::library::VECTORS_DIR).join("semantic.sqlite3"))?;
    let recovered = RECOVERED.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()));
    let first = recovered.lock().map_err(|e| e.to_string())?.insert(root.to_path_buf());
    if first {
      store.recover_interrupted(&root.to_string_lossy())?;
    }
    Ok(store)
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

  pub fn fail_asset_analysis(&mut self, asset_id: &str, error: &str) -> Result<(), String> {
    let tx = self.conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM semantic_segments WHERE asset_id=?1", [asset_id]).map_err(|e| e.to_string())?;
    tx.execute("UPDATE semantic_assets SET status='failed', error=?2 WHERE asset_id=?1", params![asset_id, error]).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| format!("标记语义分析失败：{e}"))?;
    Ok(())
  }

  pub fn search(&self, model: &ModelRevision, analysis_version: u32, query: &[f32], limit: usize) -> Result<Vec<SegmentHit>, String> {
    self.search_impl(model, analysis_version, query, None, limit)
  }

  pub fn search_scoped(
    &self,
    model: &ModelRevision,
    analysis_version: u32,
    query: &[f32],
    asset_ids: &std::collections::HashSet<String>,
    limit: usize,
  ) -> Result<Vec<SegmentHit>, String> {
    self.search_impl(model, analysis_version, query, Some(asset_ids), limit)
  }

  pub fn has_ready_segments_for_assets(
    &self,
    model_id: &str,
    model_revision: &str,
    analysis_version: u32,
    asset_ids: &std::collections::HashSet<String>,
  ) -> Result<bool, String> {
    let ids = asset_ids.iter().collect::<Vec<_>>();
    for chunk in ids.chunks(500) {
      let placeholders = vec!["?"; chunk.len()].join(",");
      let sql = format!(
        "SELECT 1 FROM semantic_segments s JOIN semantic_assets a ON a.asset_id=s.asset_id
         WHERE a.model_id=?1 AND a.model_revision=?2 AND a.analysis_version=?3 AND a.status='ready'
           AND s.asset_id IN ({placeholders}) LIMIT 1",
      );
      let mut values = vec![
        rusqlite::types::Value::Text(model_id.into()),
        rusqlite::types::Value::Text(model_revision.into()),
        rusqlite::types::Value::Integer(analysis_version.into()),
      ];
      values.extend(chunk.iter().map(|id| rusqlite::types::Value::Text((*id).clone())));
      let mut stmt = self.conn.prepare(&sql).map_err(|e| format!("检查范围语义片段失败：{e}"))?;
      let mut rows = stmt.query(rusqlite::params_from_iter(values)).map_err(|e| format!("检查范围语义片段失败：{e}"))?;
      if rows.next().map_err(|e| format!("检查范围语义片段失败：{e}"))?.is_some() {
        return Ok(true);
      }
    }
    Ok(false)
  }

  fn search_impl(
    &self,
    model: &ModelRevision,
    analysis_version: u32,
    query: &[f32],
    asset_ids: Option<&std::collections::HashSet<String>>,
    limit: usize,
  ) -> Result<Vec<SegmentHit>, String> {
    let query = normalize(query)?;
    if query.len() != model.dimension {
      return Err("查询向量维度与模型不一致".into());
    }
    let mut stmt = self.conn.prepare(
      "SELECT s.id, s.asset_id, s.start_ms, s.end_ms, s.start_frame, s.end_frame, s.kind, s.text, s.vector, d.certainty
       FROM semantic_segments s JOIN semantic_assets a ON a.asset_id=s.asset_id
       LEFT JOIN semantic_documents d ON d.segment_id=s.id
       WHERE a.model_id=?1 AND a.model_revision=?2 AND a.analysis_version=?3 AND a.status='ready'",
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![model.id, model.revision, analysis_version], |row| {
      let asset_id: String = row.get(1)?;
      if asset_ids.is_some_and(|ids| !ids.contains(&asset_id)) {
        return Ok(None);
      }
      let bytes: Vec<u8> = row.get(8)?;
      Ok(Some((SegmentHit {
        segment_id: row.get(0)?, asset_id, start_ms: row.get(2)?, end_ms: row.get(3)?,
        start_frame: row.get(4)?, end_frame: row.get(5)?, score: 0.0, kind: row.get(6)?, certainty: row.get(9)?, text: row.get(7)?,
      }, bytes)))
    }).map_err(|e| format!("读取语义片段失败：{e}"))?;
    let mut hits = Vec::new();
    for row in rows {
      let Some((mut hit, bytes)) = row.map_err(|e| e.to_string())? else { continue };
      let vector = decode_vector(&bytes)?;
      if vector.len() == query.len() {
        hit.score = dot(&query, &vector);
        hits.push(hit);
      }
    }
    Ok(super::rank::rank_segments(hits, limit))
  }

  pub fn asset_segments(&self, asset_id: &str) -> Result<Vec<SegmentHit>, String> {
    let mut stmt = self.conn.prepare(
      "SELECT s.id, s.asset_id, s.start_ms, s.end_ms, s.start_frame, s.end_frame, s.kind, s.text, d.certainty
       FROM semantic_segments s LEFT JOIN semantic_documents d ON d.segment_id=s.id WHERE s.asset_id=?1 ORDER BY s.start_ms, s.id",
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([asset_id], |row| Ok(SegmentHit {
      segment_id: row.get(0)?, asset_id: row.get(1)?, start_ms: row.get(2)?, end_ms: row.get(3)?,
      start_frame: row.get(4)?, end_frame: row.get(5)?, score: 0.0, kind: row.get(6)?, certainty: row.get(8)?, text: row.get(7)?,
    })).map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())
  }

  pub fn asset_source_fingerprint(&self, asset_id: &str) -> Result<Option<String>, String> {
    self.conn.query_row("SELECT source_fingerprint FROM semantic_assets WHERE asset_id=?1", [asset_id], |row| row.get(0))
      .optional().map_err(|e| e.to_string())
  }

  pub fn segment_total(&self) -> Result<i64, String> {
    self.conn.query_row("SELECT COUNT(*) FROM semantic_segments", [], |row| row.get(0)).map_err(|e| e.to_string())
  }

  #[cfg(test)]
  pub fn segment_count(&self, asset_id: &str) -> Result<i64, String> {
    self.conn.query_row("SELECT COUNT(*) FROM semantic_segments WHERE asset_id=?1", [asset_id], |row| row.get(0)).map_err(|e| e.to_string())
  }

  pub fn segment_count_for_model(&self, asset_id: &str, model: &ModelRevision, analysis_version: u32) -> Result<i64, String> {
    self.conn.query_row(
      "SELECT COUNT(*) FROM semantic_segments s JOIN semantic_assets a ON a.asset_id=s.asset_id
       WHERE s.asset_id=?1 AND a.model_id=?2 AND a.model_revision=?3 AND a.analysis_version=?4 AND a.status='ready'",
      params![asset_id, model.id, model.revision, analysis_version],
      |row| row.get(0),
    ).map_err(|e| e.to_string())
  }
}

pub(crate) fn normalize(vector: &[f32]) -> Result<Vec<f32>, String> {
  if vector.is_empty() || vector.iter().any(|value| !value.is_finite()) {
    return Err("向量包含空值或非法数值".into());
  }
  let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
  if !norm.is_finite() || norm <= 0.0 {
    return Err("向量无法归一化".into());
  }
  Ok(vector.iter().map(|value| value / norm).collect())
}

pub(crate) fn encode_vector(vector: &[f32]) -> Vec<u8> {
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
    let hits = store.search(&model(2), 1, &[0.0, 3.0], 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].start_ms, 2000);
    assert_eq!(hits[0].start_frame, Some(2));
    assert_eq!(hits[0].kind, "visual");
    assert!((hits[0].score - 1.0).abs() < 0.00001);
    assert!(store.search(&model(3), 1, &[1.0, 0.0], 10).is_err());
  }

  #[test]
  fn scoped_search_filters_before_ranking_and_checks_ready_segments() {
    let (_dir, mut store) = store();
    store.replace_asset_analysis("a1", "video", "fp1", &model(2), 1, &[segment(0, vec![1.0, 0.0])]).unwrap();
    store.replace_asset_analysis("a2", "video", "fp2", &model(2), 1, &[segment(0, vec![0.0, 1.0])]).unwrap();
    let only_a1 = std::collections::HashSet::from(["a1".to_string()]);
    assert!(store.has_ready_segments_for_assets(&model(2).id, &model(2).revision, 1, &only_a1).unwrap());
    let hits = store.search_scoped(&model(2), 1, &[0.0, 1.0], &only_a1, 1).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].asset_id, "a1");
    assert_eq!(hits[0].score, 0.0);

    let missing = std::collections::HashSet::from(["missing".to_string()]);
    assert!(!store.has_ready_segments_for_assets(&model(2).id, &model(2).revision, 1, &missing).unwrap());
    assert!(store.search_scoped(&model(2), 1, &[0.0, 1.0], &missing, 1).unwrap().is_empty());

    let over_one_chunk = (0..500).map(|index| format!("missing-{index}")).chain(["a2".to_string()]).collect();
    assert!(store.has_ready_segments_for_assets(&model(2).id, &model(2).revision, 1, &over_one_chunk).unwrap());
  }

  #[test]
  fn mixed_channels_use_rank_fusion_instead_of_raw_cross_channel_scores() {
    let (_dir, mut store) = store();
    store.replace_asset_analysis("visual-target", "video", "fp1", &model(2), 1, &[segment(0, vec![0.9, 0.19_f32.sqrt()])]).unwrap();
    store.replace_asset_analysis("text-target", "video", "fp2", &model(2), 1, &[segment(0, vec![0.0, 1.0])]).unwrap();
    store.conn.execute(
      "INSERT INTO semantic_segments(asset_id, start_ms, end_ms, start_frame, end_frame, kind, text, vector, dimension)
       VALUES('text-target', 0, 1000, 0, 1, 'event', '文字高匹配', ?1, 2)",
      [encode_vector(&[1.0, 0.0])],
    ).unwrap();
    let hits = store.search(&model(2), 1, &[1.0, 0.0], 10).unwrap();
    assert_eq!(hits[0].asset_id, "visual-target");
    assert_eq!(hits[0].kind, "visual");
    assert!((hits[0].score - 0.9).abs() < 0.00001);
    assert!(hits.iter().any(|hit| hit.asset_id == "text-target" && hit.kind == "event" && hit.score == 1.0));
  }

  #[test]
  fn analysis_versions_are_isolated_and_failure_removes_stale_segments() {
    let (_dir, mut store) = store();
    store.replace_asset_analysis("a1", "video", "fp1", &model(2), 1, &[segment(0, vec![1.0, 0.0])]).unwrap();
    assert!(store.search(&model(2), 2, &[1.0, 0.0], 10).unwrap().is_empty());
    assert_eq!(store.segment_count_for_model("a1", &model(2), 1).unwrap(), 1);
    store.fail_asset_analysis("a1", "decode failed").unwrap();
    assert_eq!(store.segment_count("a1").unwrap(), 0);
    assert!(store.asset_segments("a1").unwrap().is_empty());
    assert!(store.search(&model(2), 1, &[1.0, 0.0], 10).unwrap().is_empty());
  }

  #[test]
  fn v1_task_schema_migrates_backend_and_default_mode() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("semantic.sqlite3");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(
      "CREATE TABLE analysis_tasks(
        id TEXT PRIMARY KEY, library_root TEXT NOT NULL, assets_dir TEXT NOT NULL,
        device TEXT NOT NULL, status TEXT NOT NULL, total INTEGER NOT NULL,
        completed INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
      );
      INSERT INTO analysis_tasks(id, library_root, assets_dir, device, status, total)
        VALUES('legacy', '/library', '/assets', 'cpu', 'running', 1);
      PRAGMA user_version=1;",
    ).unwrap();
    drop(conn);
    let store = SemanticStore::open(&path).unwrap();
    let snapshot = store.task_snapshot("legacy").unwrap().unwrap();
    assert_eq!(snapshot.actual_backend, "unknown");
    assert_eq!(snapshot.mode, "basic");
    assert_eq!(snapshot.status, "running");
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
