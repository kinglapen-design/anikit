use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::library::AppState;
use crate::core::library::{Library, TEMP_DIR};
use crate::core::models::{AssetMeta, ROOT_FOLDER_ID};
use crate::services::{anim, media as media_support, sequence};

use super::{control::AnalysisControl, media, model::{self, ModelManifest}, provider, storage::SemanticStore, types::TaskSnapshot, worker};

#[derive(Clone, Copy, PartialEq, Eq)]
enum AnalysisMode {
  Basic,
  Detailed,
}

impl AnalysisMode {
  fn as_str(self) -> &'static str {
    match self {
      Self::Basic => "basic",
      Self::Detailed => "detailed",
    }
  }

  fn from_stored(value: &str) -> Result<Self, String> {
    match value {
      "basic" => Ok(Self::Basic),
      "detailed" => Ok(Self::Detailed),
      _ => Err("分析任务模式无效".into()),
    }
  }
}

enum AnalysisOutput {
  Basic(media::MediaAnalysis),
  Detailed(provider::ProviderAnalysis),
}

static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisProgress {
  pub task_id: String,
  pub status: String,
  pub current_asset_id: Option<String>,
  pub completed: i64,
  pub total: i64,
  pub actual_backend: String,
  pub error: Option<String>,
}

pub struct AnalysisTarget {
  pub asset_id: String,
  pub meta: AssetMeta,
}

struct TaskContext {
  root: PathBuf,
  assets_dir: PathBuf,
  mode: AnalysisMode,
  provider_settings: Option<super::provider_types::ProviderSettings>,
}

pub fn start<R: tauri::Runtime>(app: &AppHandle<R>, asset_ids: Option<Vec<String>>, folder_id: Option<String>, recursive: bool) -> Result<String, String> {
  start_with_mode(app, asset_ids, folder_id, recursive, AnalysisMode::Basic, None)
}

pub fn start_detailed<R: tauri::Runtime>(app: &AppHandle<R>, asset_ids: Vec<String>) -> Result<String, String> {
  start_detailed_with(app, asset_ids, super::provider_config::settings()?)
}

pub(crate) fn start_detailed_with<R: tauri::Runtime>(app: &AppHandle<R>, asset_ids: Vec<String>, settings: super::provider_types::ProviderSettings) -> Result<String, String> {
  start_with_mode(app, Some(asset_ids), None, false, AnalysisMode::Detailed, Some(settings))
}

fn start_with_mode<R: tauri::Runtime>(
  app: &AppHandle<R>, asset_ids: Option<Vec<String>>, folder_id: Option<String>, recursive: bool,
  mode: AnalysisMode, provider_settings: Option<super::provider_types::ProviderSettings>,
) -> Result<String, String> {
  let provider_settings = if mode == AnalysisMode::Detailed {
    let settings = match provider_settings {
      Some(settings) => settings,
      None => super::provider_config::settings()?,
    };
    settings.require_configured()?;
    Some(settings)
  } else {
    None
  };
  let state = app.state::<AppState>();
  let (root, assets_dir, mut targets) = {
    let guard = state.library.lock().map_err(|e| e.to_string())?;
    let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
    let targets = collect_targets(lib, asset_ids, folder_id, recursive)?;
    (lib.root.clone(), lib.assets_dir.clone(), targets)
  };
  targets.retain(|target| {
    if !target.meta.ext.eq_ignore_ascii_case("png") || anim::is_anim(&target.meta) || sequence::frames_dir_if_sequence(&assets_dir, &target.asset_id, &target.meta).is_some() {
      return true;
    }
    media::is_apng(&crate::core::paths::info_dir(&assets_dir, &target.asset_id).join(&target.meta.origin_name))
  });
  if targets.is_empty() {
    return Err("没有可分析的视频、动图或序列帧".into());
  }
  let manifest = ModelManifest::load(&model::model_dir()?)?;
  let task_id = format!("sem-{}-{}", chrono::Local::now().timestamp_millis(), NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed));
  let control = Arc::new(AnalysisControl::new());
  state.semantic_tasks.lock().map_err(|e| e.to_string()).and_then(|mut tasks| {
    if tasks.is_empty() {
      tasks.insert(task_id.clone(), control.clone());
      Ok(())
    } else {
      Err("已有语义分析任务未结束，请先完成或取消".to_string())
    }
  })?;
  let ids = targets.iter().map(|target| target.asset_id.clone()).collect::<Vec<_>>();
  let device = crate::core::config::semantic_settings().map_err(|e| e.to_string())?.device;
  let device_name = requested_device(device);
  let created = (|| {
    worker::pin_task(manifest.clone(), device)?;
    let mut store = open_store(&root)?;
    store.register_model(&manifest.revision)?;
    store.create_task(&task_id, &root.to_string_lossy(), &assets_dir.to_string_lossy(), device_name, mode.as_str(), &ids)
  })();
  if let Err(error) = created {
    state.semantic_tasks.lock().map_err(|e| e.to_string())?.remove(&task_id);
    worker::reset_worker();
    return Err(error);
  }
  let app = app.clone();
  let task = task_id.clone();
  std::thread::spawn(move || {
    if let Err(error) = run_task(&app, task.clone(), control, manifest, targets, TaskContext { root, assets_dir, mode, provider_settings }) {
      log::error!("[semantic] 分析任务异常：{error}");
    }
  });
  Ok(task_id)
}

fn run_task<R: tauri::Runtime>(
  app: &AppHandle<R>, task_id: String, control: Arc<AnalysisControl>, manifest: ModelManifest, targets: Vec<AnalysisTarget>, context: TaskContext,
) -> Result<(), String> {
  let root = &context.root;
  let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    run_task_inner(app, &task_id, &control, &manifest, targets, &context)
  })).unwrap_or_else(|_| Err("分析任务异常中断".into()));
  app.state::<AppState>().semantic_tasks.lock().map_err(|e| e.to_string())?.remove(&task_id);
  worker::reset_worker();
  if let Err(error) = &result {
    if let Ok(store) = open_store(&context.root) {
      if store.task_snapshot(&task_id)?.is_some_and(|task| matches!(task.status.as_str(), "running" | "paused")) {
        store.fail_task(&task_id)?;
      }
      let snapshot = store.task_snapshot(&task_id)?.ok_or_else(|| "分析任务不存在".to_string())?;
      emit(app, root, &task_id, "failed", None, snapshot.completed, snapshot.total, Some(error));
    }
  }
  result
}

fn run_task_inner<R: tauri::Runtime>(
  app: &AppHandle<R>, task_id: &str, control: &Arc<AnalysisControl>, manifest: &ModelManifest, targets: Vec<AnalysisTarget>, context: &TaskContext,
) -> Result<(), String> {
  let root = &context.root;
  let assets_dir = &context.assets_dir;
  let mode = context.mode;
  let provider_settings = context.provider_settings.as_ref();
  let mut store = open_store(root)?;
  let total = targets.len() as i64;
  let mut completed = 0;
  let mut cancelled = false;
  for target in targets {
    if control.check().is_err() {
      cancelled = true;
      break;
    }
    let asset_id = target.asset_id;
    emit(app, root, task_id, "running", Some(&asset_id), completed, total, None);
    store.start_task_item(task_id, &asset_id)?;
    let outcome = match mode {
      AnalysisMode::Basic => {
        let work_dir = root.join(TEMP_DIR).join("semantic").join(task_id).join(&asset_id);
        media::analyze_asset(&target.meta, assets_dir, &asset_id, &work_dir, control).map(AnalysisOutput::Basic)
      }
      AnalysisMode::Detailed => (|| {
        store.ensure_provider_ready(&asset_id, &manifest.revision, media::ANALYSIS_VERSION)?;
        let fingerprint = store.asset_source_fingerprint(&asset_id)?.ok_or_else(|| "请先进行基础画面分析".to_string())?;
        let segments = store.asset_segments(&asset_id)?;
        let settings = provider_settings.ok_or_else(|| "详细分析配置缺失".to_string())?;
        let asset = provider::ProviderAsset { meta: &target.meta, assets_dir, asset_id: &asset_id, segments: &segments, fingerprint: &fingerprint };
        provider::analyze_asset_with(asset, control, settings, |instructions, frames| {
          super::provider_http::analyze(settings, instructions, frames)
        }).map(AnalysisOutput::Detailed)
      })(),
    };
    if control.is_cancelled() {
      cancelled = true;
      break;
    }
    match outcome {
      Ok(AnalysisOutput::Basic(analysis)) => {
        store.set_task_backend(task_id, &analysis.actual_backend)?;
        store.replace_asset_analysis(&asset_id, analysis.kind.as_str(), &analysis.fingerprint, &manifest.revision, media::ANALYSIS_VERSION, &analysis.segments)?;
        store.complete_task_item(task_id, &asset_id)?;
        completed += 1;
      }
      Ok(AnalysisOutput::Detailed(analysis)) => {
        store.set_task_backend(task_id, &analysis.actual_backend)?;
        store.replace_provider_analysis(&asset_id, &manifest.revision, &analysis.documents)?;
        store.complete_task_item(task_id, &asset_id)?;
        completed += 1;
      }
      Err(error) if control.is_cancelled() => {
        log::info!("[semantic] {task_id} 取消于 {asset_id}：{error}");
        cancelled = true;
        break;
      }
      Err(error) => {
        if matches!(mode, AnalysisMode::Detailed) {
          store.fail_provider_analysis(&asset_id, &error)?;
        } else {
          store.fail_asset_analysis(&asset_id, &error)?;
        }
        store.fail_task_item(task_id, &asset_id, &error)?;
        completed += 1;
      }
    }
    emit(app, root, task_id, "running", Some(&asset_id), completed, total, None);
  }
  if cancelled {
    store.cancel_task(task_id)?;
    emit(app, root, task_id, "cancelled", None, completed, total, Some("分析已取消"));
  } else {
    emit(app, root, task_id, "done", None, completed, total, None);
  }
  Ok(())
}

pub fn pause<R: tauri::Runtime>(app: &AppHandle<R>, task_id: &str) -> Result<(), String> {
  let state = app.state::<AppState>();
  let root = current_root(&state)?;
  let snapshot = task_in_library(&root, task_id)?;
  let control = state.semantic_tasks.lock().map_err(|e| e.to_string())?.get(task_id).cloned().ok_or_else(|| "分析任务不在运行中".to_string())?;
  control.pause()?;
  if let Err(error) = open_store(&root)?.pause_task(task_id) {
    control.resume()?;
    return Err(error);
  }
  emit(app, &root, task_id, "paused", None, snapshot.completed, snapshot.total, None);
  Ok(())
}

pub fn resume<R: tauri::Runtime>(app: &AppHandle<R>, task_id: &str) -> Result<String, String> {
  let state = app.state::<AppState>();
  let root = current_root(&state)?;
  let snapshot = task_in_library(&root, task_id)?;
  if snapshot.status != "paused" {
    return Err("分析任务不在暂停状态".into());
  }
  let control = state.semantic_tasks.lock().map_err(|e| e.to_string())?.get(task_id).cloned();
  if let Some(control) = control {
    control.resume()?;
    open_store(&root)?.resume_task(task_id)?;
    emit(app, &root, task_id, "running", None, snapshot.completed, snapshot.total, None);
    return Ok(task_id.into());
  }
  let items = open_store(&root)?.pending_items(task_id)?;
  if items.is_empty() {
    return Err("没有可恢复的待分析资产".into());
  }
  start_with_mode(app, Some(items), None, false, AnalysisMode::from_stored(&snapshot.mode)?, None)
}

pub fn cancel<R: tauri::Runtime>(app: &AppHandle<R>, task_id: &str) -> Result<(), String> {
  let state = app.state::<AppState>();
  let root = current_root(&state)?;
  let snapshot = task_in_library(&root, task_id)?;
  let control = state.semantic_tasks.lock().map_err(|e| e.to_string())?.get(task_id).cloned();
  if let Some(control) = control {
    control.cancel()?;
  }
  open_store(&root)?.cancel_task(task_id)?;
  emit(app, &root, task_id, "cancelled", None, snapshot.completed, snapshot.total, Some("分析已取消"));
  Ok(())
}

pub fn retry<R: tauri::Runtime>(app: &AppHandle<R>, task_id: &str) -> Result<String, String> {
  let state = app.state::<AppState>();
  let root = current_root(&state)?;
  let snapshot = task_in_library(&root, task_id)?;
  if matches!(snapshot.status.as_str(), "running" | "paused") {
    return Err("请先完成或取消当前分析任务".into());
  }
  let items = open_store(&root)?.retry_items(task_id)?;
  if items.is_empty() {
    return Err("没有需要重试的资产".into());
  }
  start_with_mode(app, Some(items), None, false, AnalysisMode::from_stored(&snapshot.mode)?, None)
}

pub fn get_task<R: tauri::Runtime>(app: &AppHandle<R>, task_id: &str) -> Result<Option<TaskSnapshot>, String> {
  let root = current_root(&app.state::<AppState>())?;
  match open_store(&root)?.task_snapshot(task_id)? {
    Some(snapshot) if snapshot.library_root == root.to_string_lossy() => Ok(Some(snapshot)),
    _ => Ok(None),
  }
}

pub fn list_tasks<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<Vec<TaskSnapshot>, String> {
  let root = current_root(&app.state::<AppState>())?;
  open_store(&root)?.list_tasks(&root.to_string_lossy())
}

fn task_in_library(root: &Path, task_id: &str) -> Result<TaskSnapshot, String> {
  open_store(root)?.task_snapshot(task_id)?.filter(|snapshot| snapshot.library_root == root.to_string_lossy()).ok_or_else(|| "分析任务不属于当前素材库".into())
}

fn current_root(state: &AppState) -> Result<PathBuf, String> {
  state.library.lock().map_err(|e| e.to_string())?.as_ref().map(|lib| lib.root.clone()).ok_or_else(|| "请先打开素材库".into())
}

fn open_store(root: &Path) -> Result<SemanticStore, String> {
  SemanticStore::open_library(root)
}

fn requested_device(device: crate::core::config::SemanticDevice) -> &'static str {
  if device == crate::core::config::SemanticDevice::Gpu { "directml" } else { "cpu" }
}

#[allow(clippy::too_many_arguments)]
fn emit<R: tauri::Runtime>(app: &AppHandle<R>, root: &Path, task_id: &str, status: &str, asset_id: Option<&str>, completed: i64, total: i64, error: Option<&str>) {
  let snapshot = open_store(root).and_then(|store| store.task_snapshot(task_id)).ok().flatten();
  let status = if status == "running" { snapshot.as_ref().map_or_else(|| status.to_string(), |task| task.status.clone()) } else { status.to_string() };
  let progress = AnalysisProgress {
    task_id: task_id.into(), status, current_asset_id: asset_id.map(str::to_owned), completed, total,
    actual_backend: snapshot.map_or_else(|| "unknown".into(), |task| task.actual_backend), error: error.map(str::to_owned),
  };
  let _ = app.emit("semantic-analysis-progress", progress);
}

fn collect_targets(lib: &Library, asset_ids: Option<Vec<String>>, folder_id: Option<String>, recursive: bool) -> Result<Vec<AnalysisTarget>, String> {
  let mut targets = BTreeMap::new();
  if let Some(ids) = asset_ids {
    for id in ids {
      if let Some(meta) = lib.index.assets.get(&id).filter(|meta| supported(lib, &id, meta)) {
        targets.entry(id.clone()).or_insert_with(|| AnalysisTarget { asset_id: id, meta: meta.clone() });
      }
    }
  } else if let Some(folder) = folder_id {
    let folders = folder_ids(lib, &folder, recursive);
    for placement in lib.index.placements_by_folder.values().flatten() {
      if !folders.contains(&placement.folder) {
        continue;
      }
      if let Some(meta) = lib.index.assets.get(&placement.asset).filter(|meta| supported(lib, &placement.asset, meta)) {
        targets.insert(placement.asset.clone(), AnalysisTarget { asset_id: placement.asset.clone(), meta: meta.clone() });
      }
    }
  } else {
    return Err("请选择要分析的素材或文件夹".into());
  }
  Ok(targets.into_values().collect())
}

fn folder_ids(lib: &Library, folder: &str, recursive: bool) -> HashSet<String> {
  if folder == ROOT_FOLDER_ID && recursive {
    return lib.index.placements_by_folder.keys().cloned().collect();
  }
  let mut result = HashSet::from([folder.to_string()]);
  if !recursive {
    return result;
  }
  let mut pending = vec![folder.to_string()];
  while let Some(parent) = pending.pop() {
    for child in lib.index.folders.values().filter(|folder| folder.parent.as_deref() == Some(parent.as_str())) {
      if result.insert(child.id.clone()) {
        pending.push(child.id.clone());
      }
    }
  }
  result
}

fn supported(lib: &Library, asset_id: &str, meta: &AssetMeta) -> bool {
  if meta.hidden == Some(true) || meta.r#type.as_deref() == Some("segment") {
    return false;
  }
  if sequence::frames_dir_if_sequence(&lib.assets_dir, asset_id, meta).is_some() || anim::is_anim(meta) {
    return true;
  }
  let origin = lib.info_dir(asset_id).join(&meta.origin_name);
  media_support::is_video(&origin) || meta.ext.eq_ignore_ascii_case("png")
}

/// 单资产分析状态（详情面板「语义分析」行）：可分析判定走 supported 同口径；
/// 段数 / 详细文档数全容错——模型目录未配置或索引不可读时按 0 返回，不打断面板展示
pub fn asset_status<R: tauri::Runtime>(app: &AppHandle<R>, asset_id: &str) -> Result<super::types::SemanticAssetStatus, String> {
  let state = app.state::<AppState>();
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  let analyzable = lib.index.assets.get(asset_id).is_some_and(|meta| supported(lib, asset_id, meta));
  let mut status = super::types::SemanticAssetStatus { analyzable, segment_count: 0, detailed_count: 0 };
  if analyzable {
    if let (Ok(store), Ok(manifest)) = (open_store(&lib.root), model::model_dir().and_then(|dir| ModelManifest::load(&dir))) {
      status.segment_count = store.segment_count_for_model(asset_id, &manifest.revision, super::media::ANALYSIS_VERSION).unwrap_or(0);
      status.detailed_count = store.provider_document_count(asset_id).unwrap_or(0);
    }
  }
  Ok(status)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{core::{library, models::Placement, space}, services::{import::{self, PendingFile}, operations}};

  fn make_state() -> (tempfile::TempDir, tauri::App<tauri::test::MockRuntime>) {
    let dir = tempfile::tempdir().unwrap();
    let mut space = space::create(dir.path(), "语义测试空间", false).unwrap();
    let lib = library::create(&mut space, "语义测试库").unwrap();
    let state = AppState {
      space: std::sync::Mutex::new(Some(space)), library: std::sync::Mutex::new(Some(lib)),
      imports: std::sync::Mutex::new(std::collections::HashMap::new()), inbox: std::sync::Mutex::new(None),
      workspace: std::sync::Mutex::new(Vec::new()), undo: std::sync::Mutex::new(crate::core::undo::UndoManager::default()),
      ws_searches: std::sync::Mutex::new(std::collections::HashMap::new()), semantic_tasks: std::sync::Mutex::new(std::collections::HashMap::new()),
    };
    let app = tauri::test::mock_app();
    assert!(app.manage(state));
    (dir, app)
  }

  fn add_asset(lib: &mut Library, path: PathBuf, id: &str) {
    let pending = PendingFile { path, chain: Vec::new() };
    let meta = import::build_asset_with_id(&lib.assets_dir, &pending, "sha256:t", id, import::Source::Copy, None).unwrap();
    lib.index.assets.insert(id.into(), meta);
    lib.folders.items.push(Placement { asset: id.into(), folder: ROOT_FOLDER_ID.into(), name: id.into(), order: 0, origin: true });
  }

  #[test]
  #[ignore]
  fn real_analysis_and_search() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    std::env::set_var("ANIKIT_SEMANTIC_MODEL_DIR", root.join("spike/semantic-multilingual"));
    std::env::set_var("ANIKIT_SEMANTIC_RUNTIME_DIR", root.join("src-tauri/binaries/semantic-runtime"));
    std::env::set_var("ANIKIT_SEMANTIC_WORKER_EXE", root.join("src-tauri/target/debug/semantic-worker.exe"));
    worker::reset_worker();
    let (_dir, app) = make_state();
    let handle = app.handle();
    {
      let state = handle.state::<AppState>();
      let mut guard = state.library.lock().unwrap();
      let lib = guard.as_mut().unwrap();
      add_asset(lib, root.join("spike/decode-proto/testdata/big.mp4"), "AN-VIDEO");
      add_asset(lib, root.join("tools/test-assets/anim3.gif"), "AN-GIF");
      add_asset(lib, root.join("tools/test-assets/anim3.webp"), "AN-WEBP");
      add_asset(lib, root.join("tools/test-assets/anim3.png"), "AN-APNG");
      add_asset(lib, root.join("tools/test-assets/blank-test-1.png"), "AN-SEQ");
      let info = lib.info_dir("AN-SEQ");
      std::fs::create_dir_all(info.join("frames")).unwrap();
      std::fs::rename(info.join("blank-test-1.png"), info.join("frames/blank-test-1.png")).unwrap();
      for name in ["blank-test-2.png", "blank-test-3.png"] {
        std::fs::copy(root.join("tools/test-assets").join(name), info.join("frames").join(name)).unwrap();
      }
      lib.index.assets.get_mut("AN-SEQ").unwrap().fps = Some(24.0);
      operations::finish(lib).unwrap();
    }
    let ids = ["AN-VIDEO", "AN-GIF", "AN-WEBP", "AN-APNG", "AN-SEQ"].map(str::to_string).to_vec();
    let task_id = start(&handle, Some(ids), None, false).unwrap();
    for _ in 0..600 {
      let snapshot = get_task(&handle, &task_id).unwrap().unwrap();
      if matches!(snapshot.status.as_str(), "done" | "cancelled" | "failed") {
        assert_eq!(snapshot.status, "done", "分析未完成：{snapshot:?}");
        assert_eq!(snapshot.completed, snapshot.total);
        assert!(snapshot.errors.is_empty(), "存在失败素材：{snapshot:?}");
        assert!(snapshot.actual_backend == "cpu" || snapshot.actual_backend.starts_with("directml"));
        break;
      }
      std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let scope = crate::services::search::SearchScope { view_key: "~".into(), recursive: true, global: true, filters: Default::default() };
    let hits = crate::services::semantic::search::semantic_search(&handle, "电脑屏幕上的软件界面", &scope, Some(10)).unwrap();
    assert!(hits.hits.iter().any(|hit| hit.best.asset_id == "AN-SEQ"), "真实中文检索未命中屏幕序列帧：{hits:?}");
    let library_root = handle.state::<AppState>().library.lock().unwrap().as_ref().unwrap().root.clone();
    let store = SemanticStore::open_library(&library_root).unwrap();
    let video = store.asset_segments("AN-VIDEO").unwrap();
    assert!(video.iter().all(|segment| segment.start_ms >= 0 && segment.start_ms < 10000 && segment.end_ms >= segment.start_ms));
    let sequence = store.asset_segments("AN-SEQ").unwrap();
    assert_eq!(sequence.iter().map(|segment| segment.start_frame).collect::<Vec<_>>(), vec![Some(0), Some(1), Some(2)]);
    assert_eq!(sequence.iter().map(|segment| segment.start_ms).collect::<Vec<_>>(), vec![0, 42, 83]);
    for id in ["AN-GIF", "AN-WEBP", "AN-APNG"] {
      let segments = store.asset_segments(id).unwrap();
      assert!(!segments.is_empty(), "{id} 没有动图片段");
      assert_eq!(segments[0].start_ms, 0);
      assert!(segments.iter().all(|segment| segment.start_frame.is_some() && segment.start_frame == segment.end_frame));
    }
  }
}
