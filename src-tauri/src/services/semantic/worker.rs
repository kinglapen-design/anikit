use std::{
  io::{BufRead, BufReader, Write},
  path::Path,
  process::{Child, ChildStdin, Command, Stdio},
  sync::{Mutex, Once, OnceLock, atomic::{AtomicBool, Ordering}, mpsc},
  time::{Duration, Instant},
};

use serde::Serialize;
use serde_json::{json, Value};

use crate::core::config::SemanticDevice;
use super::{model::{self, ModelManifest}, types::ModelRevision};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(180);
const TEXT_IDLE_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingResponse {
  pub embedding: Vec<f32>,
  pub actual_backend: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticRuntimeStatus {
  pub requested_backend: String,
  pub actual_backend: String,
  pub model_dir: String,
  pub runtime_dir: String,
  pub dimension: usize,
}

#[derive(Debug)]
struct WorkerError {
  message: String,
  retryable: bool,
}

impl WorkerError {
  fn transport(message: impl Into<String>) -> Self {
    Self { message: message.into(), retryable: true }
  }

  fn cancelled() -> Self {
    Self { message: "分析已取消".into(), retryable: false }
  }

  fn map<T>(result: Result<T, String>) -> Result<T, Self> {
    result.map_err(Self::transport)
  }
}

struct WorkerProcess {
  child: Child,
  stdin: ChildStdin,
  receiver: mpsc::Receiver<Result<String, String>>,
  next_id: u64,
  requested: SemanticDevice,
  backend: SemanticDevice,
  revision: ModelRevision,
  text_only: bool,
  healthy: bool,
  last_used: Instant,
}

impl WorkerProcess {
  fn spawn(manifest: &ModelManifest, backend: SemanticDevice, text_only: bool, cancel: Option<&AtomicBool>) -> Result<Self, WorkerError> {
    let mut command = worker_command(manifest, backend, text_only)?;
    let mut child = command.spawn().map_err(|e| WorkerError::transport(format!("启动语义 worker 失败：{e}")))?;
    if let Some(stderr) = child.stderr.take() {
      std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
          log::warn!("[semantic-worker] {line}");
        }
      });
    }
    let stdin = child.stdin.take().ok_or_else(|| WorkerError::transport("无法连接语义 worker 输入"))?;
    let stdout = child.stdout.take().ok_or_else(|| WorkerError::transport("无法连接语义 worker 输出"))?;
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
      for line in BufReader::new(stdout).lines() {
        let failed = line.is_err();
        if sender.send(line.map_err(|e| e.to_string())).is_err() || failed {
          break;
        }
      }
    });
    let mut worker = Self {
      child, stdin, receiver, next_id: 0, requested: backend, backend,
      revision: manifest.revision.clone(), text_only, healthy: true, last_used: Instant::now(),
    };
    let ready = worker.read_message(cancel)?;
    if ready.get("type").and_then(Value::as_str) != Some("ready") {
      return Err(WorkerError::transport(format!("语义 worker 初始化失败：{ready}")));
    }
    Ok(worker)
  }

  fn request(&mut self, mut request: Value, cancel: Option<&AtomicBool>) -> Result<EmbeddingResponse, WorkerError> {
    self.next_id += 1;
    let id = self.next_id;
    request["id"] = json!(id);
    let text = WorkerError::map(serde_json::to_string(&request).map_err(|e| e.to_string()))?;
    self.send(&text)?;
    let response = self.read_message(cancel)?;
    if response.get("type").and_then(Value::as_str) == Some("error") {
      let message = response.get("message").and_then(Value::as_str).unwrap_or("语义推理失败").to_string();
      let retryable = response.get("retryable").and_then(Value::as_bool).unwrap_or(true);
      return Err(WorkerError { message, retryable });
    }
    if response.get("type").and_then(Value::as_str) != Some("result") || response.get("id").and_then(Value::as_u64) != Some(id) {
      self.healthy = false;
      return Err(WorkerError::transport(format!("语义 worker 返回异常：{response}")));
    }
    let embedding: Vec<f32> = serde_json::from_value(response.get("embedding").cloned().unwrap_or_else(|| json!([]))).map_err(|e| {
      self.healthy = false;
      WorkerError::transport(format!("语义 worker 向量格式无效：{e}"))
    })?;
    if embedding.len() != model::EMBEDDING_DIM || embedding.iter().any(|value| !value.is_finite()) {
      self.healthy = false;
      return Err(WorkerError::transport("语义 worker 返回了无效向量"));
    }
    self.last_used = Instant::now();
    let actual_backend = response.get("actualBackend").and_then(Value::as_str).unwrap_or("unknown").to_string();
    Ok(EmbeddingResponse { embedding, actual_backend })
  }

  fn send(&mut self, text: &str) -> Result<(), WorkerError> {
    let result = self.stdin.write_all(text.as_bytes()).and_then(|_| self.stdin.write_all(b"\n")).and_then(|_| self.stdin.flush());
    result.map_err(|e| {
      self.healthy = false;
      WorkerError::transport(format!("发送语义请求失败：{e}"))
    })
  }

  fn read_message(&mut self, cancel: Option<&AtomicBool>) -> Result<Value, WorkerError> {
    let deadline = Instant::now() + RESPONSE_TIMEOUT;
    loop {
      if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        self.healthy = false;
        return Err(WorkerError::cancelled());
      }
      let result = self.receiver.recv_timeout(Duration::from_millis(100));
      match result {
        Ok(Ok(line)) => {
          let parsed = serde_json::from_str(&line).map_err(|e| format!("语义 worker 返回格式无效：{e}"));
          if parsed.is_err() {
            self.healthy = false;
          }
          return WorkerError::map(parsed);
        }
        Ok(Err(error)) => {
          self.healthy = false;
          return Err(WorkerError::transport(format!("读取语义 worker 失败：{error}")));
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
          self.healthy = false;
          return Err(WorkerError::transport("语义 worker 已退出"));
        }
        Err(mpsc::RecvTimeoutError::Timeout) if Instant::now() >= deadline => {
          self.healthy = false;
          return Err(WorkerError::transport("语义 worker 响应超时"));
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {}
      }
    }
  }
}

impl Drop for WorkerProcess {
  fn drop(&mut self) {
    let _ = self.stdin.write_all(b"{\"type\":\"shutdown\",\"id\":0}\n");
    let _ = self.stdin.flush();
    for _ in 0..10 {
      if self.child.try_wait().ok().flatten().is_some() {
        return;
      }
      std::thread::sleep(Duration::from_millis(50));
    }
    let _ = self.child.kill();
    let _ = self.child.wait();
  }
}

fn worker_command(manifest: &ModelManifest, backend: SemanticDevice, text_only: bool) -> Result<Command, WorkerError> {
  let backend = if backend == SemanticDevice::Gpu { "directml" } else { "cpu" };
  log::info!("[semantic] 加载模型 {} rev {}", manifest.revision.id, manifest.revision.revision);
  let mut command = Command::new(WorkerError::map(model::worker_exe())?);
  command
    .arg("--runtime-dir").arg(WorkerError::map(model::runtime_dir())?)
    .arg("--text-model").arg(&manifest.text_path)
    .arg("--vision-model").arg(&manifest.vision_path)
    .arg("--tokenizer").arg(&manifest.tokenizer_path)
    .arg("--backend").arg(backend)
    .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
  if text_only {
    command.arg("--text-only");
  }
  #[cfg(windows)]
  std::os::windows::process::CommandExt::creation_flags(&mut command, CREATE_NO_WINDOW);
  Ok(command)
}

#[derive(Clone)]
struct TaskRuntime {
  manifest: ModelManifest,
  device: SemanticDevice,
}

static WORKER: OnceLock<Mutex<Option<WorkerProcess>>> = OnceLock::new();
static TASK: OnceLock<Mutex<Option<TaskRuntime>>> = OnceLock::new();
static GPU_FALLBACK: OnceLock<Mutex<Option<ModelRevision>>> = OnceLock::new();
static MANIFEST_CACHE: OnceLock<Mutex<Option<(std::path::PathBuf, ModelManifest)>>> = OnceLock::new();
static IDLE_WATCHER: Once = Once::new();

fn worker_slot() -> &'static Mutex<Option<WorkerProcess>> {
  IDLE_WATCHER.call_once(|| {
    std::thread::spawn(|| loop {
      std::thread::sleep(Duration::from_secs(30));
      if let Ok(mut guard) = worker_slot().lock() {
        if guard.as_ref().is_some_and(|worker| worker.text_only && worker.last_used.elapsed() >= TEXT_IDLE_TIMEOUT) {
          *guard = None;
        }
      }
    });
  });
  WORKER.get_or_init(|| Mutex::new(None))
}

fn runtime() -> Result<(ModelManifest, SemanticDevice), String> {
  if let Some(task) = TASK.get_or_init(|| Mutex::new(None)).lock().map_err(|e| e.to_string())?.clone() {
    return Ok((task.manifest, task.device));
  }
  let settings = crate::core::config::semantic_settings().map_err(|e| e.to_string())?;
  let directory = model::model_dir()?;
  let cache = MANIFEST_CACHE.get_or_init(|| Mutex::new(None));
  if let Some(manifest) = cache.lock().map_err(|e| e.to_string())?.as_ref().filter(|(path, _)| path == &directory).map(|(_, manifest)| manifest.clone()) {
    return Ok((manifest, settings.device));
  }
  let manifest = ModelManifest::load(&directory)?;
  *cache.lock().map_err(|e| e.to_string())? = Some((directory, manifest.clone()));
  Ok((manifest, settings.device))
}

fn ensure_process<'a>(slot: &'a mut Option<WorkerProcess>, text_only: bool, cancel: Option<&AtomicBool>) -> Result<&'a mut WorkerProcess, WorkerError> {
  let (manifest, device) = WorkerError::map(runtime())?;
  let replace = slot.as_ref().is_some_and(|worker| {
    !worker.healthy || worker.revision != manifest.revision || worker.requested != device || (worker.text_only && !text_only)
  });
  if replace {
    *slot = None;
  }
  if slot.is_none() {
    let forced_cpu = GPU_FALLBACK.get_or_init(|| Mutex::new(None)).lock().map_err(|e| WorkerError::transport(e.to_string()))?
      .as_ref() == Some(&manifest.revision);
    let backend = if forced_cpu { SemanticDevice::Cpu } else { device };
    match WorkerProcess::spawn(&manifest, backend, text_only, cancel) {
      Ok(worker) => *slot = Some(worker),
      Err(error) if backend == SemanticDevice::Gpu && error.retryable => {
        log::warn!("[semantic] GPU 初始化失败，回退 CPU：{}", error.message);
        mark_gpu_fallback(&manifest.revision)?;
        let mut worker = WorkerProcess::spawn(&manifest, SemanticDevice::Cpu, text_only, cancel)?;
        worker.requested = device;
        *slot = Some(worker);
      }
      Err(error) => return Err(error),
    }
  }
  Ok(slot.as_mut().expect("worker just initialized"))
}

fn mark_gpu_fallback(revision: &ModelRevision) -> Result<(), WorkerError> {
  *GPU_FALLBACK.get_or_init(|| Mutex::new(None)).lock().map_err(|e| WorkerError::transport(e.to_string()))? = Some(revision.clone());
  Ok(())
}

fn embed(request: Value, text_only: bool, cancel: Option<&AtomicBool>) -> Result<EmbeddingResponse, String> {
  let mut guard = worker_slot().lock().map_err(|e| e.to_string())?;
  let requested_gpu = ensure_process(&mut guard, text_only, cancel).map_err(|e| e.message)?.backend == SemanticDevice::Gpu;
  let result = guard.as_mut().expect("worker exists").request(request.clone(), cancel);
  let revision = guard.as_ref().map(|worker| worker.revision.clone());
  let healthy = guard.as_ref().is_some_and(|worker| worker.healthy);
  if !healthy {
    *guard = None;
  }
  if let Err(error) = &result {
    if requested_gpu && error.retryable {
      log::warn!("[semantic] GPU 推理失败，回退 CPU：{}", error.message);
      *guard = None;
      if let Some(revision) = revision {
        mark_gpu_fallback(&revision).map_err(|e| e.message)?;
      }
      return guard.as_mut().expect("worker exists").request(request, cancel).map_err(|e| e.message);
    }
  }
  result.map_err(|error| error.message)
}

pub fn embed_text(text: &str) -> Result<EmbeddingResponse, String> {
  embed(json!({ "type": "embedText", "text": text }), true, None)
}

pub fn embed_image(path: &Path, cancel: Option<&AtomicBool>) -> Result<EmbeddingResponse, String> {
  embed(json!({ "type": "embedImage", "path": path.to_string_lossy() }), false, cancel)
}

pub fn pin_task(manifest: ModelManifest, device: SemanticDevice) -> Result<(), String> {
  let mut task = TASK.get_or_init(|| Mutex::new(None)).lock().map_err(|e| e.to_string())?;
  if task.is_some() {
    return Err("已有语义分析运行时".into());
  }
  *task = Some(TaskRuntime { manifest, device });
  Ok(())
}

pub fn test_runtime() -> Result<SemanticRuntimeStatus, String> {
  let response = embed_text("状态检查")?;
  let settings = crate::core::config::semantic_settings().map_err(|e| e.to_string())?;
  let requested_backend = if settings.device == SemanticDevice::Gpu { "directml" } else { "cpu" };
  Ok(SemanticRuntimeStatus {
    requested_backend: requested_backend.into(), actual_backend: response.actual_backend,
    model_dir: model::model_dir()?.to_string_lossy().to_string(), runtime_dir: model::runtime_dir()?.to_string_lossy().to_string(),
    dimension: model::EMBEDDING_DIM,
  })
}

pub fn reset_worker() {
  if let Ok(mut task) = TASK.get_or_init(|| Mutex::new(None)).lock() {
    *task = None;
  }
  if let Ok(mut fallback) = GPU_FALLBACK.get_or_init(|| Mutex::new(None)).lock() {
    *fallback = None;
  }
  if let Ok(mut cache) = MANIFEST_CACHE.get_or_init(|| Mutex::new(None)).lock() {
    *cache = None;
  }
  if let Ok(mut guard) = worker_slot().lock() {
    *guard = None;
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::PathBuf;

  fn prepare() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    std::env::set_var("ANIKIT_SEMANTIC_MODEL_DIR", root.join("spike/semantic-multilingual"));
    std::env::set_var("ANIKIT_SEMANTIC_RUNTIME_DIR", root.join("src-tauri/binaries/semantic-runtime"));
    std::env::set_var("ANIKIT_SEMANTIC_WORKER_EXE", root.join("src-tauri/target/debug/semantic-worker.exe"));
    reset_worker();
    root
  }

  #[test]
  #[ignore]
  fn real_model_smoke() {
    let root = prepare();
    let status = test_runtime().unwrap();
    assert_eq!(status.dimension, 512);
    let response = embed_image(&root.join("spike/semantic-multilingual/assets/CLIP.png"), None).unwrap();
    assert_eq!(response.embedding.len(), 512);
    assert!(response.embedding.iter().all(|value| value.is_finite()));
    reset_worker();
  }

  #[test]
  #[ignore]
  fn real_worker_cancel_smoke() {
    let root = prepare();
    let cancel = AtomicBool::new(true);
    assert!(embed_image(&root.join("spike/semantic-multilingual/assets/CLIP.png"), Some(&cancel)).is_err());
    reset_worker();
  }

  #[test]
  fn task_runtime_pin_is_exclusive() {
    let manifest = ModelManifest {
      revision: ModelRevision { id: "test".into(), revision: "r1".into(), dimension: 512, weight_sha256: "a".repeat(64) },
      text_path: PathBuf::new(), vision_path: PathBuf::new(), tokenizer_path: PathBuf::new(),
    };
    reset_worker();
    pin_task(manifest.clone(), SemanticDevice::Cpu).unwrap();
    assert!(pin_task(manifest, SemanticDevice::Cpu).is_err());
    reset_worker();
  }
}
