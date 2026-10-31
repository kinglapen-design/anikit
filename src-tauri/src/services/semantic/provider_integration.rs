use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::{Value, json};
use tauri::Manager;

use crate::commands::library::AppState;
use crate::core::config::SemanticDevice;
use crate::core::library::{self, Library, TEMP_DIR};
use crate::core::models::{AssetMeta, Placement, ROOT_FOLDER_ID};
use crate::core::{space, paths};
use crate::services::import::{self, PendingFile};
use crate::services::operations;
use crate::services::semantic::control::AnalysisControl;
use crate::services::semantic::media::{self, MediaAnalysis};
use crate::services::semantic::model::{self, ModelManifest};
use crate::services::semantic::provider;
use crate::services::semantic::provider_http;
use crate::services::semantic::provider_types::{ProviderKind, ProviderSettings};
use crate::services::semantic::storage::SemanticStore;
use crate::services::semantic::worker;

const ASSET_ID: &str = "AN-PROVIDER-GIF";

struct WorkerReset;

impl Drop for WorkerReset {
  fn drop(&mut self) {
    worker::reset_worker();
  }
}

struct Fixture {
  _dir: tempfile::TempDir,
  app: tauri::App<tauri::test::MockRuntime>,
  root: PathBuf,
  assets_dir: PathBuf,
  meta: AssetMeta,
  manifest: ModelManifest,
  analysis: MediaAnalysis,
  visual_vectors: Vec<Vec<u8>>,
}

fn test_app(dir: &Path) -> tauri::App<tauri::test::MockRuntime> {
  let mut space = space::create(dir, "外部语义测试空间", false).unwrap();
  let lib = library::create(&mut space, "外部语义测试库").unwrap();
  let state = AppState {
    space: std::sync::Mutex::new(Some(space)), library: std::sync::Mutex::new(Some(lib)),
    imports: std::sync::Mutex::new(std::collections::HashMap::new()), inbox: std::sync::Mutex::new(None),
    workspace: std::sync::Mutex::new(Vec::new()), undo: std::sync::Mutex::new(crate::core::undo::UndoManager::default()),
    ws_searches: std::sync::Mutex::new(std::collections::HashMap::new()), semantic_tasks: std::sync::Mutex::new(std::collections::HashMap::new()),
  };
  let app = tauri::test::mock_app();
  assert!(app.manage(state));
  app
}

fn import_gif(lib: &mut Library, source: &Path) {
  let pending = PendingFile { path: source.to_path_buf(), chain: Vec::new() };
  let meta = import::build_asset_with_id(&lib.assets_dir, &pending, "sha256:provider-test", ASSET_ID, import::Source::Copy, None).unwrap();
  lib.index.assets.insert(ASSET_ID.into(), meta);
  lib.folders.items.push(Placement { asset: ASSET_ID.into(), folder: ROOT_FOLDER_ID.into(), name: ASSET_ID.into(), order: 0, origin: true });
  operations::finish(lib).unwrap();
}

fn visual_vectors(store: &SemanticStore) -> Vec<Vec<u8>> {
  let mut stmt = store.conn.prepare("SELECT vector FROM semantic_segments WHERE asset_id=?1 AND kind='visual' ORDER BY id").unwrap();
  stmt.query_map([ASSET_ID], |row| row.get(0)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap()
}

fn fixture() -> Fixture {
  let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
  std::env::set_var("ANIKIT_SEMANTIC_MODEL_DIR", project.join("spike/semantic-multilingual"));
  std::env::set_var("ANIKIT_SEMANTIC_RUNTIME_DIR", project.join("src-tauri/binaries/semantic-runtime"));
  std::env::set_var("ANIKIT_SEMANTIC_WORKER_EXE", project.join("src-tauri/target/debug/semantic-worker.exe"));
  let dir = tempfile::tempdir().unwrap();
  let app = test_app(dir.path());
  let handle = app.handle();
  let (root, assets_dir, meta) = {
    let state = handle.state::<AppState>();
    let mut guard = state.library.lock().unwrap();
    let lib = guard.as_mut().unwrap();
    import_gif(lib, &project.join("tools/test-assets/anim3.gif"));
    (lib.root.clone(), lib.assets_dir.clone(), lib.index.assets[ASSET_ID].clone())
  };
  let manifest = ModelManifest::load(&model::model_dir().unwrap()).unwrap();
  worker::pin_task(manifest.clone(), SemanticDevice::Cpu).unwrap();
  let analysis = media::analyze_asset(&meta, &assets_dir, ASSET_ID, &root.join(TEMP_DIR).join("provider-integration"), &AnalysisControl::new()).unwrap();
  let mut store = SemanticStore::open_library(&root).unwrap();
  store.register_model(&manifest.revision).unwrap();
  store.replace_asset_analysis(ASSET_ID, analysis.kind.as_str(), &analysis.fingerprint, &manifest.revision, media::ANALYSIS_VERSION, &analysis.segments).unwrap();
  let visual_vectors = visual_vectors(&store);
  assert!(!visual_vectors.is_empty());
  Fixture { _dir: dir, app, root, assets_dir, meta, manifest, analysis, visual_vectors }
}

fn read_request(stream: &std::net::TcpStream) -> String {
  stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
  let mut reader = BufReader::new(stream.try_clone().unwrap());
  let mut raw = String::new();
  let mut length = 0;
  loop {
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    if line.to_ascii_lowercase().starts_with("content-length:") {
      length = line.split(':').nth(1).unwrap().trim().parse().unwrap();
    }
    let done = matches!(line.as_str(), "\r\n" | "\n");
    raw.push_str(&line);
    if done { break; }
  }
  let mut body = vec![0; length];
  reader.read_exact(&mut body).unwrap();
  raw.push_str(std::str::from_utf8(&body).unwrap());
  raw
}

fn mock_provider(times: &[i64]) -> (ProviderSettings, Receiver<String>, JoinHandle<()>) {
  let second = times.get(1).copied().unwrap_or(times[0]);
  let last = *times.last().unwrap();
  let evidence_end = times.len() - 1;
  let document = json!({
    "description": "机器人正在洗碗",
    "events": [
      {"text":"机器人拿碗并连续洗碗","startMs":times[0],"endMs":second,"certainty":"observed","evidence":[0, evidence_end.min(1)]},
      {"text":"洗碗之后可能打碎碗","startMs":second,"endMs":last,"certainty":"hypothesis","evidence":[evidence_end.min(1), evidence_end]}
    ]
  }).to_string();
  let response = json!({"choices":[{"message":{"content":document}}]}).to_string();
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let endpoint = format!("http://{}", listener.local_addr().unwrap());
  let (tx, rx) = mpsc::channel();
  let handle = std::thread::spawn(move || {
    let (mut stream, _) = listener.accept().unwrap();
    tx.send(read_request(&stream)).unwrap();
    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len());
    stream.write_all(response.as_bytes()).unwrap();
  });
  let settings = ProviderSettings { kind: ProviderKind::OpenAiCompatible, endpoint: format!("{endpoint}/v1"), model: "mock-vision".into(), timeout_secs: 5 };
  (settings, rx, handle)
}

#[test]
#[ignore]
fn real_provider_loopback_and_offline_search() {
  let _reset = WorkerReset;
  let fixture = fixture();
  let store = SemanticStore::open_library(&fixture.root).unwrap();
  store.ensure_provider_ready(ASSET_ID, &fixture.manifest.revision, media::ANALYSIS_VERSION).unwrap();
  assert!(store.ensure_provider_ready(ASSET_ID, &fixture.manifest.revision, media::ANALYSIS_VERSION + 1).is_err());
  let base_segments = store.asset_segments(ASSET_ID).unwrap();
  let times = base_segments.iter().map(|segment| segment.start_ms).collect::<Vec<_>>();
  let (settings, rx, server) = mock_provider(&times);
  let control = AnalysisControl::new();
  let asset = provider::ProviderAsset {
    meta: &fixture.meta, assets_dir: &fixture.assets_dir, asset_id: ASSET_ID,
    segments: &base_segments, fingerprint: &fixture.analysis.fingerprint,
  };
  let detailed = provider::analyze_asset_with(asset, &control, &settings, |prompt, frames| {
    provider_http::analyze_with_key(&settings, prompt, frames, None)
  }).unwrap();
  let request = rx.recv_timeout(Duration::from_secs(5)).unwrap();
  server.join().unwrap();
  assert!(!request.to_ascii_lowercase().contains("authorization:"));
  let body: Value = serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
  let content = body["messages"][0]["content"].as_array().unwrap();
  assert_eq!(content.len() - 1, times.len());
  let prompt = content[0]["text"].as_str().unwrap();
  assert!(prompt.contains("稀疏采样"));
  for time in &times {
    assert!(prompt.contains(&format!("{time} ms")), "缺少真实证据时间：{time}");
  }
  assert_eq!(detailed.actual_backend, "cpu");
  assert_eq!(detailed.documents.len(), 3);
  let mut store = store;
  store.replace_provider_analysis(ASSET_ID, &fixture.manifest.revision, &detailed.documents).unwrap();
  assert_eq!(visual_vectors(&store), fixture.visual_vectors);
  assert_eq!(store.provider_document_count(ASSET_ID).unwrap(), 3);
  let segments = store.asset_segments(ASSET_ID).unwrap();
  assert!(segments.iter().any(|segment| segment.certainty.as_deref() == Some("observed")));
  assert!(segments.iter().any(|segment| segment.certainty.as_deref() == Some("hypothesis")));
  let provenance: (String, String, String, String) = store.conn.query_row(
    "SELECT provider, model, prompt_version, frame_ms FROM semantic_documents WHERE asset_id=?1 AND certainty='hypothesis'",
    [ASSET_ID], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
  ).unwrap();
  assert_eq!(provenance.0, "openai-compatible");
  assert_eq!(provenance.1, "mock-vision");
  assert_eq!(provenance.2, "semantic-v1");
  let frame_ms: Vec<i64> = serde_json::from_str(&provenance.3).unwrap();
  assert_eq!(frame_ms.first(), times.get(1).or_else(|| times.first()));
  assert_eq!(frame_ms.last(), times.last());
  let scope = crate::services::search::SearchScope { global: true, view_key: "~".into(), recursive: true, filters: Default::default() };
  let hits = crate::services::semantic::search::semantic_search(fixture.app.handle(), "机器人洗碗后打碎碗", &scope, Some(10)).unwrap();
  assert_eq!(hits.actual_backend, "cpu");
  let hit = hits.hits.iter().find(|hit| hit.best.asset_id == ASSET_ID).expect("详细事件应可离线检索");
  assert!(hit.segments.iter().any(|segment| segment.kind == "event" && segment.certainty.as_deref() == Some("hypothesis") && segment.start_ms == times.get(1).copied().unwrap_or(times[0])));
  let stale_asset = provider::ProviderAsset {
    meta: &fixture.meta, assets_dir: &fixture.assets_dir, asset_id: ASSET_ID,
    segments: &base_segments, fingerprint: "sha256:stale",
  };
  let stale = provider::analyze_asset_with(stale_asset, &control, &settings, |_, _| {
    panic!("源指纹不一致时不得调用 provider")
  });
  assert!(matches!(stale, Err(ref error) if error.contains("源文件已变化")));
  let info = paths::info_dir(&fixture.assets_dir, ASSET_ID);
  assert!(info.join(&fixture.meta.origin_name).is_file());
}
