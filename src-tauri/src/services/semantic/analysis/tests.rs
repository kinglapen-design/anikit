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

fn add_static_assets(lib: &mut Library, source: &Path) -> Vec<String> {
  let fixtures = lib.root.join("static-fixtures");
  std::fs::create_dir_all(&fixtures).unwrap();
  let image = image::open(source).unwrap();
  let mut ids = Vec::new();
  for (id, ext) in [
    ("AN-STATIC-PNG", "png"),
    ("AN-STATIC-JPG", "jpg"),
    ("AN-STATIC-JPEG", "jpeg"),
    ("AN-STATIC-WEBP", "webp"),
    ("AN-STATIC-BMP", "bmp"),
  ] {
    let path = fixtures.join(format!("{id}.{ext}"));
    image.save(&path).unwrap();
    add_asset(lib, path, id);
    ids.push(id.to_string());
  }
  ids
}

#[test]
fn common_static_images_pass_target_collection() {
  let (_dir, app) = make_state();
  let state = app.state::<AppState>();
  let mut guard = state.library.lock().unwrap();
  let lib = guard.as_mut().unwrap();
  let source = lib.root.join("static-source.png");
  image::RgbaImage::from_pixel(2, 2, image::Rgba([200, 30, 40, 255])).save(&source).unwrap();
  let ids = add_static_assets(lib, &source);
  let targets = collect_targets(lib, Some(ids), None, false).unwrap();
  assert_eq!(targets.len(), 5);
  assert!(targets.iter().all(|target| media::is_static_image_ext(&target.meta.ext)));
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
  let (ids, static_ids) = {
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
    let static_ids = add_static_assets(lib, &root.join("tools/test-assets/blank-test-1.png"));
    operations::finish(lib).unwrap();
    let mut ids = ["AN-VIDEO", "AN-GIF", "AN-WEBP", "AN-APNG", "AN-SEQ"].map(str::to_string).to_vec();
    ids.extend(static_ids.iter().cloned());
    (ids, static_ids)
  };
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
  for id in static_ids {
    let segments = store.asset_segments(&id).unwrap();
    assert_eq!(segments.len(), 1, "{id} 静态图应只有一个向量");
    assert_eq!((segments[0].start_ms, segments[0].end_ms, segments[0].start_frame, segments[0].end_frame), (0, 0, None, None));
    assert_eq!(segments[0].kind, "visual");
    assert!(hits.hits.iter().any(|hit| hit.best.asset_id == id), "中文搜索未返回静态图 {id}");
  }
}
