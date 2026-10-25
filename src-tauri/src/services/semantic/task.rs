#![allow(dead_code)] // V1～V5 分批接线的公开契约；分析任务与 MCP 接入后移除

use std::collections::HashSet;

use rusqlite::{params, OptionalExtension};

use super::{storage::SemanticStore, types::TaskSnapshot};

impl SemanticStore {
  pub fn create_task(
    &mut self,
    task_id: &str,
    library_root: &str,
    assets_dir: &str,
    device: &str,
    asset_ids: &[String],
  ) -> Result<(), String> {
    let unique = asset_ids.iter().collect::<HashSet<_>>();
    let tx = self.conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
      "INSERT INTO analysis_tasks(id, library_root, assets_dir, device, status, total) VALUES(?1, ?2, ?3, ?4, 'running', ?5)",
      params![task_id, library_root, assets_dir, device, unique.len() as i64],
    )
    .map_err(|e| format!("创建分析任务失败：{e}"))?;
    for asset_id in unique {
      tx.execute("INSERT INTO analysis_task_items(task_id, asset_id, status) VALUES(?1, ?2, 'pending')", params![task_id, asset_id])
        .map_err(|e| format!("创建分析任务项失败：{e}"))?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
  }

  pub fn complete_task_item(&mut self, task_id: &str, asset_id: &str) -> Result<(), String> {
    let tx = self.conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
      "UPDATE analysis_task_items SET status='done', error=NULL WHERE task_id=?1 AND asset_id=?2",
      params![task_id, asset_id],
    )
    .map_err(|e| e.to_string())?;
    let completed: i64 = tx
      .query_row("SELECT COUNT(*) FROM analysis_task_items WHERE task_id=?1 AND status='done'", [task_id], |row| row.get(0))
      .map_err(|e| e.to_string())?;
    tx.execute(
      "UPDATE analysis_tasks SET completed=?2, status=CASE WHEN ?2=total THEN 'done' ELSE 'running' END,
       updated_at=CURRENT_TIMESTAMP WHERE id=?1",
      params![task_id, completed],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
  }

  pub fn pause_task(&self, task_id: &str) -> Result<(), String> {
    self.set_task_status(task_id, "paused")
  }

  pub fn resume_task(&self, task_id: &str) -> Result<(), String> {
    self.set_task_status(task_id, "running")
  }

  pub fn recover_interrupted(&self) -> Result<usize, String> {
    self.conn.execute("UPDATE analysis_tasks SET status='paused' WHERE status='running'", []).map_err(|e| e.to_string())
  }

  pub fn task_snapshot(&self, task_id: &str) -> Result<Option<TaskSnapshot>, String> {
    self
      .conn
      .query_row(
        "SELECT id, library_root, assets_dir, device, status, total, completed FROM analysis_tasks WHERE id=?1",
        [task_id],
        |row| {
          Ok(TaskSnapshot {
            id: row.get(0)?,
            library_root: row.get(1)?,
            assets_dir: row.get(2)?,
            device: row.get(3)?,
            status: row.get(4)?,
            total: row.get(5)?,
            completed: row.get(6)?,
          })
        },
      )
      .optional()
      .map_err(|e| e.to_string())
  }

  pub fn pending_items(&self, task_id: &str) -> Result<Vec<String>, String> {
    let mut stmt = self
      .conn
      .prepare("SELECT asset_id FROM analysis_task_items WHERE task_id=?1 AND status='pending' ORDER BY asset_id")
      .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([task_id], |row| row.get(0)).map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())
  }

  fn set_task_status(&self, task_id: &str, status: &str) -> Result<(), String> {
    let changed = self
      .conn
      .execute(
        "UPDATE analysis_tasks SET status=?2, updated_at=CURRENT_TIMESTAMP WHERE id=?1 AND status!='done'",
        params![task_id, status],
      )
      .map_err(|e| e.to_string())?;
    if changed == 0 {
      return Err("分析任务不存在或已完成".into());
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn checkpoint_survives_reopen_and_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("semantic.sqlite3");
    let task_id = "task-1";
    {
      let mut store = SemanticStore::open(&path).unwrap();
      store.create_task(task_id, "/library", "/assets", "cpu", &["a1".into(), "a2".into(), "a1".into()]).unwrap();
      store.complete_task_item(task_id, "a1").unwrap();
      assert_eq!(store.pending_items(task_id).unwrap(), vec!["a2"]);
    }
    let mut store = SemanticStore::open(&path).unwrap();
    assert_eq!(store.recover_interrupted().unwrap(), 1);
    assert_eq!(store.task_snapshot(task_id).unwrap().unwrap().status, "paused");
    store.resume_task(task_id).unwrap();
    store.complete_task_item(task_id, "a2").unwrap();
    let snapshot = store.task_snapshot(task_id).unwrap().unwrap();
    assert_eq!(snapshot.status, "done");
    assert_eq!(snapshot.completed, 2);
    assert_eq!(snapshot.library_root, "/library");
    assert_eq!(snapshot.device, "cpu");
    assert!(store.resume_task(task_id).is_err());
  }
}
