//! 命令 API：设置面板缓存页（PRD 06_数据 §07）——缓存占用统计 + 一键清理。
//! 读命令（get_cache_overview）未开库 / 未开空间降级为该项 0 不报错（参照
//! list_space_libraries 先例）；写命令（clear_cache）未开库 / 未开空间报中文错，
//! 未知类别 id 拒绝。业务全在 services::cache（统计求和 / 删除边界 / 重建核心），
//! 本层只做状态快照、后台重建线程触发与内存 format_icons 同步。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter, Manager, State};

use crate::commands::library::AppState;
use crate::core::space::WORKSPACE_DIR;
use crate::services::cache::{self, CacheCategory, CacheOverview, ClearOutcome, LibSnapshot, RebuildReport};

/// 库快照（短锁克隆，锁外 IO）：全池资产元数据（统计 / 清理按 ID 推导的依据）
fn lib_snapshot(state: &AppState) -> Option<LibSnapshot> {
  let guard = state.library.lock().ok()?;
  let l = guard.as_ref()?;
  Some(LibSnapshot { root: l.root.clone(), assets_dir: l.assets_dir.clone(), metas: l.index.assets.values().cloned().collect() })
}

/// 当前空间工作空间根（不存在 / 未开空间 = None）
fn ws_root(state: &AppState) -> Option<PathBuf> {
  state.space.lock().ok()?.as_ref().map(|s| s.root.join(WORKSPACE_DIR))
}

/// 缓存占用总览（设置面板缓存页数据源）：两组恒在、各项恒在（未开启侧各项 0，
/// 条目不省略——前端布局稳定）。async + spawn_blocking：目录树遍历不占 tokio worker
///（规范第 2 条，ws_dir_stats 同径）
#[tauri::command]
pub async fn get_cache_overview(state: State<'_, AppState>) -> Result<CacheOverview, String> {
  let snap = lib_snapshot(&state);
  let ws = ws_root(&state);
  // overview 本身不失败（只读探测语义），spawn_blocking 只需解 JoinError 一层
  tauri::async_runtime::spawn_blocking(move || cache::overview(snap.as_ref(), ws.as_deref()))
    .await
    .map_err(|e| format!("缓存统计任务异常：{e}"))
}

/// 一键清理指定类别缓存。未知 id 拒绝；库类需已开库、工作空间类需已开空间（中文错）。
/// lib-thumbs 清理后触发全库缩略图后台重建（视图层只拼路径不判存在，不重建 = 卡片破图），
/// 完成经 `library-thumbs-rebuilt` 事件通知；lib-icons 清理后按在库扩展名集即时重提并
/// 同步内存 format_icons。被占用文件（如播放中的转码产物）跳过计入 failedFiles
#[tauri::command]
pub async fn clear_cache(app: AppHandle, state: State<'_, AppState>, category: String) -> Result<ClearOutcome, String> {
  let cat = CacheCategory::parse(&category)?;
  match cat.group() {
    "library" => clear_lib_side(&app, &state, cat).await,
    _ => clear_ws_side(&state, cat).await,
  }
}

/// 库侧清理：快照 → spawn_blocking 删除 → 类别收尾（图标内存集同步 / 缩略图重建触发）
async fn clear_lib_side(app: &AppHandle, state: &State<'_, AppState>, cat: CacheCategory) -> Result<ClearOutcome, String> {
  let snap = lib_snapshot(state).ok_or_else(|| "请先打开资源库".to_string())?;
  let assets_dir = snap.assets_dir.clone(); // 切库护栏比对用（快照身份）
  let (freed, removed, failed, icon_exts) = tauri::async_runtime::spawn_blocking(move || match cat {
    CacheCategory::LibThumbs => {
      let (a, b, c) = cache::clear_lib_thumbs(&snap);
      (a, b, c, None)
    }
    CacheCategory::LibIcons => {
      let (a, b, c, ready) = cache::clear_lib_icons(&snap.root, &snap.assets_dir, &snap.metas);
      (a, b, c, Some(ready))
    }
    _ => {
      let (a, b, c) = cache::clear_lib_dirs(cat, &snap.root);
      (a, b, c, None)
    }
  })
  .await
  .map_err(|e| format!("缓存清理任务异常：{e}"))?;

  // 格式图标内存集同步（切库护栏：assets_dir 不一致 = 清理期间已切库，不写新库内存）
  if let Some(ready) = icon_exts {
    if let Ok(mut guard) = state.library.lock() {
      if let Some(l) = guard.as_mut() {
        if l.assets_dir == assets_dir {
          l.format_icons = ready;
        }
      }
    }
  }
  let rebuilding = cat == CacheCategory::LibThumbs;
  if rebuilding {
    spawn_thumb_rebuild(app);
  }
  Ok(ClearOutcome { category: cat.id().to_string(), freed_bytes: freed, removed_files: removed, failed_files: failed, rebuilding_thumbs: rebuilding })
}

/// 工作空间侧清理：ws 根缺失 = 无缓存可清，幂等成功（统计侧同样按 0 计）
async fn clear_ws_side(state: &State<'_, AppState>, cat: CacheCategory) -> Result<ClearOutcome, String> {
  let ws = ws_root(state).ok_or_else(|| "请先打开储存空间".to_string())?;
  let (freed, removed, failed) = if ws.is_dir() {
    tauri::async_runtime::spawn_blocking(move || cache::clear_ws_category(cat, &ws))
      .await
      .map_err(|e| format!("缓存清理任务异常：{e}"))?
  } else {
    (0, 0, 0)
  };
  Ok(ClearOutcome { category: cat.id().to_string(), freed_bytes: freed, removed_files: removed, failed_files: failed, rebuilding_thumbs: false })
}

// ---------- 全库缩略图后台重建（单实例线程 + 补跑标记） ----------

/// 重建运行锁 + 补跑标记（清理与在跑重建交错：后删的文件由补趟兜底，不丢）
static REBUILD_RUNNING: AtomicBool = AtomicBool::new(false);
static REBUILD_AGAIN: AtomicBool = AtomicBool::new(false);

/// 触发全库缩略图后台重建（lib-thumbs 清理后调用）：已在跑则置补跑标记由在跑线程收尾后
/// 再来一趟（本函数幂等重入）；线程完成经 THUMBS_REBUILT_EVENT 通知前端刷新列表
fn spawn_thumb_rebuild(app: &AppHandle) {
  if REBUILD_RUNNING.swap(true, Ordering::SeqCst) {
    REBUILD_AGAIN.store(true, Ordering::SeqCst);
    return;
  }
  let app = app.clone();
  std::thread::Builder::new()
    .name("thumb-rebuild".into())
    .spawn(move || {
      let mut total = RebuildReport::default();
      loop {
        let state = app.state::<AppState>();
        let (report, _missing) = cache::rebuild_missing_thumbs(&state.library);
        total.merge(&report);
        if REBUILD_AGAIN.swap(false, Ordering::SeqCst) {
          continue; // 本趟期间又有清理请求：仍持运行锁，立即补趟
        }
        // 先放锁再复查补跑标记：复查前到达的请求已置 AGAIN（被本趟兜住）；
        // 复查后到达的请求看到 RUNNING=false 会自起新线程——两侧都不丢
        REBUILD_RUNNING.store(false, Ordering::SeqCst);
        if !REBUILD_AGAIN.swap(false, Ordering::SeqCst) || REBUILD_RUNNING.swap(true, Ordering::SeqCst) {
          break; // 无补跑，或已有新线程接管
        }
      }
      if total.rebuilt + total.fallback > 0 {
        log::info!("全库缩略图重建完成：重建 {} / 回退格式图标 {}", total.rebuilt, total.fallback);
      }
      if let Err(e) = app.emit(cache::THUMBS_REBUILT_EVENT, total) {
        log::debug!("{} 事件推送失败（前端可能已关闭）：{e}", cache::THUMBS_REBUILT_EVENT);
      }
    })
    .expect("缩略图重建线程启动失败");
}
