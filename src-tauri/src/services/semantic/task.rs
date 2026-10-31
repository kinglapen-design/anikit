use std::collections::HashSet;

use rusqlite::{params, OptionalExtension};

use super::{storage::SemanticStore, types::{TaskError, TaskSnapshot}};

impl SemanticStore {
  pub fn create_task(&mut self, task_id: &str, library_root: &str, assets_dir: &str, device: &str, mode: &str, asset_ids: &[String]) -> Result<(), String> {
    if !matches!(mode, "basic" | "detailed") {
      return Err("分析任务模式无效".into());
    }
    let unique = asset_ids.iter().collect::<HashSet<_>>();
    let tx = self.conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
      "INSERT INTO analysis_tasks(id, library_root, assets_dir, device, mode, status, total) VALUES(?1, ?2, ?3, ?4, ?5, 'running', ?6)",
      params![task_id, library_root, assets_dir, device, mode, unique.len() as i64],
    ).map_err(|e| format!("创建分析任务失败：{e}"))?;
    for asset_id in unique {
      tx.execute("INSERT INTO analysis_task_items(task_id, asset_id, status) VALUES(?1, ?2, 'pending')", params![task_id, asset_id])
        .map_err(|e| format!("创建分析任务项失败：{e}"))?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
  }

  pub fn start_task_item(&self, task_id: &str, asset_id: &str) -> Result<(), String> {
    let changed = self.conn.execute(
      "UPDATE analysis_task_items SET status='running', error=NULL WHERE task_id=?1 AND asset_id=?2 AND status='pending'",
      params![task_id, asset_id],
    ).map_err(|e| e.to_string())?;
    if changed != 1 {
      return Err("分析任务项不存在或已处理".into());
    }
    Ok(())
  }

  pub fn complete_task_item(&mut self, task_id: &str, asset_id: &str) -> Result<(), String> {
    self.finish_item(task_id, asset_id, "done", None)
  }

  pub fn fail_task_item(&mut self, task_id: &str, asset_id: &str, error: &str) -> Result<(), String> {
    self.finish_item(task_id, asset_id, "failed", Some(error))
  }

  fn finish_item(&mut self, task_id: &str, asset_id: &str, status: &str, error: Option<&str>) -> Result<(), String> {
    let tx = self.conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
      "UPDATE analysis_task_items SET status=?3, error=?4 WHERE task_id=?1 AND asset_id=?2",
      params![task_id, asset_id, status, error],
    ).map_err(|e| e.to_string())?;
    let completed: i64 = tx.query_row(
      "SELECT COUNT(*) FROM analysis_task_items WHERE task_id=?1 AND status IN ('done','failed')",
      [task_id], |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    tx.execute(
      "UPDATE analysis_tasks SET completed=?2,
       status=CASE WHEN ?2=total AND status!='cancelled' THEN 'done' ELSE status END, updated_at=CURRENT_TIMESTAMP WHERE id=?1",
      params![task_id, completed],
    ).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
  }

  pub fn set_task_backend(&self, task_id: &str, backend: &str) -> Result<(), String> {
    let changed = self.conn.execute(
      "UPDATE analysis_tasks SET actual_backend=?2, updated_at=CURRENT_TIMESTAMP WHERE id=?1",
      params![task_id, backend],
    ).map_err(|e| e.to_string())?;
    if changed != 1 {
      return Err("分析任务不存在".into());
    }
    Ok(())
  }

  pub fn pause_task(&self, task_id: &str) -> Result<(), String> {
    self.set_task_status(task_id, "paused")
  }

  pub fn resume_task(&self, task_id: &str) -> Result<(), String> {
    self.set_task_status(task_id, "running")
  }

  pub fn cancel_task(&self, task_id: &str) -> Result<(), String> {
    self.set_task_status(task_id, "cancelled")
  }

  pub fn fail_task(&self, task_id: &str) -> Result<(), String> {
    self.set_task_status(task_id, "failed")
  }

  pub fn recover_interrupted(&self, library_root: &str) -> Result<usize, String> {
    let count = self.conn.execute(
      "UPDATE analysis_tasks SET status='paused', updated_at=CURRENT_TIMESTAMP WHERE status='running' AND library_root=?1",
      [library_root],
    ).map_err(|e| e.to_string())?;
    self.conn.execute(
      "UPDATE analysis_task_items SET status='pending' WHERE status='running'
       AND task_id IN (SELECT id FROM analysis_tasks WHERE library_root=?1)",
      [library_root],
    ).map_err(|e| e.to_string())?;
    Ok(count)
  }

  pub fn task_snapshot(&self, task_id: &str) -> Result<Option<TaskSnapshot>, String> {
    let Some(mut snapshot) = self.conn.query_row(
      "SELECT id, library_root, assets_dir, device, mode, actual_backend, status, total, completed FROM analysis_tasks WHERE id=?1",
      [task_id],
      |row| Ok(TaskSnapshot {
        id: row.get(0)?, library_root: row.get(1)?, assets_dir: row.get(2)?, device: row.get(3)?, mode: row.get(4)?,
        actual_backend: row.get(5)?, status: row.get(6)?, total: row.get(7)?, completed: row.get(8)?, errors: Vec::new(),
      }),
    ).optional().map_err(|e| e.to_string())? else {
      return Ok(None);
    };
    snapshot.errors = self.task_errors(task_id)?;
    Ok(Some(snapshot))
  }

  pub fn list_tasks(&self, library_root: &str) -> Result<Vec<TaskSnapshot>, String> {
    let mut stmt = self.conn.prepare(
      "SELECT id FROM analysis_tasks WHERE library_root=?1 ORDER BY created_at DESC, id DESC LIMIT 50",
    ).map_err(|e| e.to_string())?;
    let ids = stmt.query_map([library_root], |row| row.get::<_, String>(0)).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for id in ids {
      if let Some(snapshot) = self.task_snapshot(&id.map_err(|e| e.to_string())?)? {
        result.push(snapshot);
      }
    }
    Ok(result)
  }

  pub fn pending_items(&self, task_id: &str) -> Result<Vec<String>, String> {
    self.item_ids(task_id, "IN ('pending','running')")
  }

  pub fn retry_items(&self, task_id: &str) -> Result<Vec<String>, String> {
    self.item_ids(task_id, "!='done'")
  }

  fn item_ids(&self, task_id: &str, condition: &str) -> Result<Vec<String>, String> {
    let sql = format!("SELECT asset_id FROM analysis_task_items WHERE task_id=?1 AND status {condition} ORDER BY asset_id");
    let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([task_id], |row| row.get(0)).map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())
  }

  pub fn task_errors(&self, task_id: &str) -> Result<Vec<TaskError>, String> {
    let mut stmt = self.conn.prepare(
      "SELECT asset_id, error FROM analysis_task_items WHERE task_id=?1 AND error IS NOT NULL ORDER BY asset_id",
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([task_id], |row| {
      Ok(TaskError { asset_id: row.get(0)?, error: row.get::<_, Option<String>>(1)?.unwrap_or_default() })
    }).map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())
  }

  fn set_task_status(&self, task_id: &str, status: &str) -> Result<(), String> {
    let allowed: &[&str] = match status {
      "paused" => &["running"],
      "running" => &["paused"],
      "cancelled" => &["running", "paused", "cancelled"],
      "failed" => &["running", "paused"],
      _ => return Err("分析任务状态无效".into()),
    };
    let current: String = self.conn.query_row("SELECT status FROM analysis_tasks WHERE id=?1", [task_id], |row| row.get(0))
      .optional().map_err(|e| e.to_string())?.ok_or_else(|| "分析任务不存在".to_string())?;
    if !allowed.contains(&current.as_str()) {
      return Err("分析任务当前状态不允许此操作".into());
    }
    self.conn.execute(
      "UPDATE analysis_tasks SET status=?2, updated_at=CURRENT_TIMESTAMP WHERE id=?1",
      params![task_id, status],
    ).map_err(|e| e.to_string())?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn checkpoint_survives_reopen_and_recovery_is_library_scoped() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("semantic.sqlite3");
    let mut store = SemanticStore::open(&path).unwrap();
    store.create_task("task-1", "/library", "/assets", "cpu", "detailed", &["a1".into(), "a2".into(), "a1".into()]).unwrap();
    store.create_task("task-2", "/other", "/assets", "cpu", "basic", &["b1".into()]).unwrap();
    store.start_task_item("task-1", "a1").unwrap();
    store.complete_task_item("task-1", "a1").unwrap();
    store.start_task_item("task-1", "a2").unwrap();
    assert_eq!(store.recover_interrupted("/library").unwrap(), 1);
    assert_eq!(store.pending_items("task-1").unwrap(), vec!["a2"]);
    let snapshot = store.task_snapshot("task-1").unwrap().unwrap();
    assert_eq!((snapshot.status.as_str(), snapshot.mode.as_str()), ("paused", "detailed"));
    drop(store);
    let mut store = SemanticStore::open(&path).unwrap();
    assert_eq!(store.task_snapshot("task-1").unwrap().unwrap().mode, "detailed");
    assert_eq!(store.task_snapshot("task-2").unwrap().unwrap().status, "running");
    store.set_task_backend("task-1", "cpu").unwrap();
    store.resume_task("task-1").unwrap();
    store.start_task_item("task-1", "a2").unwrap();
    store.complete_task_item("task-1", "a2").unwrap();
    let snapshot = store.task_snapshot("task-1").unwrap().unwrap();
    assert_eq!((snapshot.status.as_str(), snapshot.completed), ("done", 2));
    assert_eq!(snapshot.actual_backend, "cpu");
    assert_eq!(store.list_tasks("/library").unwrap().len(), 1);
    assert!(store.resume_task("task-1").is_err());
    assert!(store.create_task("bad", "/library", "/assets", "cpu", "other", &["a".into()]).is_err());
  }

  #[test]
  fn failed_items_are_reported_and_retried() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("semantic.sqlite3");
    let mut store = SemanticStore::open(&path).unwrap();
    store.create_task("task", "/library", "/assets", "cpu", "basic", &["a".into(), "b".into()]).unwrap();
    store.fail_task_item("task", "a", "decode failed").unwrap();
    assert_eq!(store.retry_items("task").unwrap(), vec!["a", "b"]);
    assert_eq!(store.task_errors("task").unwrap()[0].error, "decode failed");
    store.complete_task_item("task", "b").unwrap();
    assert_eq!(store.retry_items("task").unwrap(), vec!["a"]);
    store.complete_task_item("task", "a").unwrap();
    assert!(store.retry_items("task").unwrap().is_empty());
    assert_eq!(store.task_snapshot("task").unwrap().unwrap().status, "done");
  }

  #[test]
  fn checkpoint_preserves_pause_and_cancel() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("semantic.sqlite3");
    let mut store = SemanticStore::open(&path).unwrap();
    store.create_task("task", "/library", "/assets", "cpu", "basic", &["a".into(), "b".into()]).unwrap();
    store.start_task_item("task", "a").unwrap();
    store.pause_task("task").unwrap();
    store.complete_task_item("task", "a").unwrap();
    assert_eq!(store.task_snapshot("task").unwrap().unwrap().status, "paused");
    store.resume_task("task").unwrap();
    store.start_task_item("task", "b").unwrap();
    store.cancel_task("task").unwrap();
    assert_eq!(store.task_snapshot("task").unwrap().unwrap().status, "cancelled");
    assert_eq!(store.pending_items("task").unwrap(), vec!["b"]);
    assert!(store.resume_task("task").is_err());
    store.cancel_task("task").unwrap();
  }

  #[test]
  fn cancelled_completion_does_not_mark_task_done() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = SemanticStore::open(&dir.path().join("semantic.sqlite3")).unwrap();
    store.create_task("task", "/library", "/assets", "cpu", "basic", &["a".into()]).unwrap();
    store.start_task_item("task", "a").unwrap();
    store.cancel_task("task").unwrap();
    store.complete_task_item("task", "a").unwrap();
    assert_eq!(store.task_snapshot("task").unwrap().unwrap().status, "cancelled");
  }
}
