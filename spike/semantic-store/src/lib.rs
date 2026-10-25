use std::{cmp::Ordering, collections::HashSet, fs, path::Path};

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};

const SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct ModelRevision {
    pub id: String,
    pub revision: String,
    pub dimension: usize,
    pub weight_sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SegmentInput {
    pub start_ms: i64,
    pub end_ms: i64,
    pub start_frame: Option<u32>,
    pub end_frame: Option<u32>,
    pub kind: String,
    pub text: String,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SegmentHit {
    pub segment_id: i64,
    pub asset_id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub start_frame: Option<u32>,
    pub end_frame: Option<u32>,
    pub score: f32,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskSnapshot {
    pub id: String,
    pub status: String,
    pub total: i64,
    pub completed: i64,
}

pub struct SemanticStore {
    conn: Connection,
}

impl SemanticStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch(
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
        )?;
        let version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version != SCHEMA_VERSION {
            bail!("unsupported semantic schema version {version}");
        }
        Ok(Self { conn })
    }

    pub fn register_model(&self, model: &ModelRevision) -> Result<()> {
        if model.dimension == 0 || model.weight_sha256.len() != 64 {
            bail!("invalid model metadata");
        }
        self.conn.execute(
      "INSERT INTO semantic_models(id, revision, dimension, weight_sha256) VALUES(?1, ?2, ?3, ?4)
       ON CONFLICT(id, revision) DO UPDATE SET dimension=excluded.dimension, weight_sha256=excluded.weight_sha256",
      params![model.id, model.revision, model.dimension as i64, model.weight_sha256],
    )?;
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
    ) -> Result<()> {
        let normalized = segments
            .iter()
            .map(|segment| normalize(&segment.vector))
            .collect::<Result<Vec<_>>>()?;
        if normalized
            .iter()
            .any(|vector| vector.len() != model.dimension)
        {
            bail!("segment dimension does not match model");
        }
        if segments
            .iter()
            .any(|segment| segment.start_ms > segment.end_ms)
        {
            bail!("segment start exceeds end");
        }
        let tx = self.conn.transaction()?;
        tx.execute(
      "INSERT INTO semantic_assets(asset_id, media_kind, source_fingerprint, model_id, model_revision, analysis_version, status)
       VALUES(?1, ?2, ?3, ?4, ?5, ?6, 'ready')
       ON CONFLICT(asset_id) DO UPDATE SET media_kind=excluded.media_kind,
         source_fingerprint=excluded.source_fingerprint, model_id=excluded.model_id,
         model_revision=excluded.model_revision, analysis_version=excluded.analysis_version,
         status='ready', error=NULL",
      params![asset_id, media_kind, source_fingerprint, model.id, model.revision, analysis_version],
    )?;
        tx.execute(
            "DELETE FROM semantic_segments WHERE asset_id=?1",
            [asset_id],
        )?;
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
      )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn search(
        &self,
        model: &ModelRevision,
        query: &[f32],
        limit: usize,
    ) -> Result<Vec<SegmentHit>> {
        let query = normalize(query)?;
        if query.len() != model.dimension {
            bail!("query dimension does not match model");
        }
        let mut stmt = self.conn.prepare(
      "SELECT s.id, s.asset_id, s.start_ms, s.end_ms, s.start_frame, s.end_frame, s.text, s.vector
       FROM semantic_segments s JOIN semantic_assets a ON a.asset_id=s.asset_id
       WHERE a.model_id=?1 AND a.model_revision=?2 AND a.status='ready'",
    )?;
        let rows = stmt.query_map(params![model.id, model.revision], |row| {
            let bytes: Vec<u8> = row.get(7)?;
            Ok((
                SegmentHit {
                    segment_id: row.get(0)?,
                    asset_id: row.get(1)?,
                    start_ms: row.get(2)?,
                    end_ms: row.get(3)?,
                    start_frame: row.get(4)?,
                    end_frame: row.get(5)?,
                    score: 0.0,
                    text: row.get(6)?,
                },
                bytes,
            ))
        })?;
        let mut hits = Vec::new();
        for row in rows {
            let (mut hit, bytes) = row?;
            let vector = decode_vector(&bytes)?;
            if vector.len() == query.len() {
                hit.score = dot(&query, &vector);
                hits.push(hit);
            }
        }
        hits.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(Ordering::Equal)
                .then(left.segment_id.cmp(&right.segment_id))
        });
        hits.truncate(limit);
        Ok(hits)
    }

    pub fn create_task(
        &mut self,
        task_id: &str,
        library_root: &str,
        assets_dir: &str,
        device: &str,
        asset_ids: &[String],
    ) -> Result<()> {
        let unique = asset_ids.iter().collect::<HashSet<_>>();
        let tx = self.conn.transaction()?;
        tx.execute(
      "INSERT INTO analysis_tasks(id, library_root, assets_dir, device, status, total) VALUES(?1, ?2, ?3, ?4, 'running', ?5)",
      params![task_id, library_root, assets_dir, device, unique.len() as i64],
    )?;
        for asset_id in unique {
            tx.execute("INSERT INTO analysis_task_items(task_id, asset_id, status) VALUES(?1, ?2, 'pending')", params![task_id, asset_id])?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn complete_task_item(&mut self, task_id: &str, asset_id: &str) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
      "UPDATE analysis_task_items SET status='done', error=NULL WHERE task_id=?1 AND asset_id=?2",
      params![task_id, asset_id],
    )?;
        let completed: i64 = tx.query_row(
            "SELECT COUNT(*) FROM analysis_task_items WHERE task_id=?1 AND status='done'",
            [task_id],
            |row| row.get(0),
        )?;
        tx.execute(
      "UPDATE analysis_tasks SET completed=?2, status=CASE WHEN ?2=total THEN 'done' ELSE 'running' END,
       updated_at=CURRENT_TIMESTAMP WHERE id=?1",
      params![task_id, completed],
    )?;
        tx.commit()?;
        Ok(())
    }

    pub fn pause_task(&self, task_id: &str) -> Result<()> {
        self.set_task_status(task_id, "paused")
    }

    pub fn resume_task(&self, task_id: &str) -> Result<()> {
        self.set_task_status(task_id, "running")
    }

    pub fn recover_interrupted(&self) -> Result<usize> {
        let count = self.conn.execute(
            "UPDATE analysis_tasks SET status='paused' WHERE status='running'",
            [],
        )?;
        Ok(count)
    }

    pub fn task_snapshot(&self, task_id: &str) -> Result<Option<TaskSnapshot>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, status, total, completed FROM analysis_tasks WHERE id=?1",
                [task_id],
                |row| {
                    Ok(TaskSnapshot {
                        id: row.get(0)?,
                        status: row.get(1)?,
                        total: row.get(2)?,
                        completed: row.get(3)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn pending_items(&self, task_id: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT asset_id FROM analysis_task_items WHERE task_id=?1 AND status='pending' ORDER BY asset_id")?;
        let rows = stmt.query_map([task_id], |row| row.get(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn segment_count(&self, asset_id: &str) -> Result<i64> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM semantic_segments WHERE asset_id=?1",
            [asset_id],
            |row| row.get(0),
        )?)
    }

    fn set_task_status(&self, task_id: &str, status: &str) -> Result<()> {
        let changed = self.conn.execute(
      "UPDATE analysis_tasks SET status=?2, updated_at=CURRENT_TIMESTAMP WHERE id=?1 AND status!='done'",
      params![task_id, status],
    )?;
        if changed == 0 {
            bail!("task missing or already done: {task_id}");
        }
        Ok(())
    }
}

fn normalize(vector: &[f32]) -> Result<Vec<f32>> {
    if vector.is_empty() || vector.iter().any(|value| !value.is_finite()) {
        bail!("invalid vector");
    }
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if !norm.is_finite() || norm <= 0.0 {
        bail!("invalid vector norm");
    }
    Ok(vector.iter().map(|value| value / norm).collect())
}

fn encode_vector(vector: &[f32]) -> Vec<u8> {
    vector
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn decode_vector(bytes: &[u8]) -> Result<Vec<f32>> {
    if !bytes.len().is_multiple_of(4) {
        bail!("invalid vector blob");
    }
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| f32::from_le_bytes(*chunk))
        .collect())
}

fn dot(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(dimension: usize) -> ModelRevision {
        ModelRevision {
            id: "test-clip".into(),
            revision: "r1".into(),
            dimension,
            weight_sha256: "a".repeat(64),
        }
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
            .replace_asset_analysis(
                "a1",
                "video",
                "fp1",
                &model(2),
                1,
                &[segment(0, vec![1.0, 0.0]), segment(1000, vec![0.0, 1.0])],
            )
            .unwrap();
        store
            .replace_asset_analysis(
                "a1",
                "video",
                "fp2",
                &model(2),
                1,
                &[segment(2000, vec![0.0, 2.0])],
            )
            .unwrap();
        assert_eq!(store.segment_count("a1").unwrap(), 1);
        let hits = store.search(&model(2), &[0.0, 3.0], 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].start_ms, 2000);
        assert_eq!(hits[0].start_frame, Some(2));
        assert!((hits[0].score - 1.0).abs() < 0.00001);
        assert!(store.search(&model(3), &[1.0, 0.0], 10).is_err());
    }

    #[test]
    fn invalid_vectors_and_intervals_are_rejected() {
        let (_dir, mut store) = store();
        assert!(
            store
                .replace_asset_analysis(
                    "bad",
                    "video",
                    "fp",
                    &model(2),
                    1,
                    &[segment(0, vec![0.0, 0.0])]
                )
                .is_err()
        );
        let mut reversed = segment(1000, vec![1.0, 0.0]);
        reversed.end_ms = 0;
        assert!(
            store
                .replace_asset_analysis("bad", "video", "fp", &model(2), 1, &[reversed])
                .is_err()
        );
        assert_eq!(store.segment_count("bad").unwrap(), 0);
    }

    #[test]
    fn task_checkpoint_survives_reopen_and_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("semantic.sqlite3");
        let task_id = "task-1";
        {
            let mut store = SemanticStore::open(&path).unwrap();
            store
                .create_task(
                    task_id,
                    "/library",
                    "/assets",
                    "cpu",
                    &["a1".into(), "a2".into(), "a1".into()],
                )
                .unwrap();
            store.complete_task_item(task_id, "a1").unwrap();
            assert_eq!(store.pending_items(task_id).unwrap(), vec!["a2"]);
        }
        let mut store = SemanticStore::open(&path).unwrap();
        assert_eq!(store.recover_interrupted().unwrap(), 1);
        assert_eq!(
            store.task_snapshot(task_id).unwrap().unwrap().status,
            "paused"
        );
        store.resume_task(task_id).unwrap();
        store.complete_task_item(task_id, "a2").unwrap();
        let snapshot = store.task_snapshot(task_id).unwrap().unwrap();
        assert_eq!(snapshot.status, "done");
        assert_eq!(snapshot.completed, 2);
        assert!(store.resume_task(task_id).is_err());
    }
}
