use std::{
  io::{BufRead, BufReader, Write},
  process::{Child, ChildStdin, ChildStdout, Command, Stdio},
  sync::{Mutex, OnceLock},
};

use serde::Serialize;
use serde_json::{json, Value};

use super::model::{self, ModelManifest};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

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

struct WorkerProcess {
  child: Child,
  stdin: ChildStdin,
  stdout: BufReader<ChildStdout>,
  next_id: u64,
  backend: crate::core::config::SemanticDevice,
  text_only: bool,
}

impl WorkerProcess {
  fn spawn(manifest: &ModelManifest, backend: crate::core::config::SemanticDevice, text_only: bool) -> Result<Self, String> {
    let runtime = model::runtime_dir()?;
    let executable = model::worker_exe()?;
    let backend_name = if backend == crate::core::config::SemanticDevice::Gpu { "directml" } else { "cpu" };
    log::info!("[semantic] 加载模型 {} rev {}", manifest.revision.id, manifest.revision.revision);
    let mut command = Command::new(executable);
    command
      .arg("--runtime-dir")
      .arg(runtime)
      .arg("--text-model")
      .arg(&manifest.text_path)
      .arg("--vision-model")
      .arg(&manifest.vision_path)
      .arg("--tokenizer")
      .arg(&manifest.tokenizer_path)
      .arg("--backend")
      .arg(backend_name)
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped());
    if text_only {
      command.arg("--text-only");
    }
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut command, CREATE_NO_WINDOW);
    let mut child = command.spawn().map_err(|e| format!("启动语义 worker 失败：{e}"))?;
    if let Some(stderr) = child.stderr.take() {
      std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
          log::warn!("[semantic-worker] {line}");
        }
      });
    }
    let stdin = child.stdin.take().ok_or_else(|| "无法连接语义 worker 输入".to_string())?;
    let stdout = BufReader::new(child.stdout.take().ok_or_else(|| "无法连接语义 worker 输出".to_string())?);
    let mut worker = Self { child, stdin, stdout, next_id: 0, backend, text_only };
    let ready = worker.read_message()?;
    if ready.get("type").and_then(Value::as_str) != Some("ready") {
      return Err(format!("语义 worker 初始化失败：{ready}"));
    }
    Ok(worker)
  }

  fn request(&mut self, mut request: Value) -> Result<EmbeddingResponse, String> {
    self.next_id += 1;
    let id = self.next_id;
    request["id"] = json!(id);
    let text = serde_json::to_string(&request).map_err(|e| e.to_string())?;
    self.stdin.write_all(text.as_bytes()).map_err(|e| format!("发送语义请求失败：{e}"))?;
    self.stdin.write_all(b"\n").map_err(|e| e.to_string())?;
    self.stdin.flush().map_err(|e| e.to_string())?;
    let response = self.read_message()?;
    if response.get("type").and_then(Value::as_str) == Some("error") {
      return Err(response.get("message").and_then(Value::as_str).unwrap_or("语义推理失败").to_string());
    }
    if response.get("type").and_then(Value::as_str) != Some("result") || response.get("id").and_then(Value::as_u64) != Some(id) {
      return Err(format!("语义 worker 返回异常：{response}"));
    }
    let embedding: Vec<f32> = serde_json::from_value(response.get("embedding").cloned().unwrap_or_else(|| json!([]))).map_err(|e| e.to_string())?;
    if embedding.len() != model::EMBEDDING_DIM || embedding.iter().any(|value| !value.is_finite()) {
      return Err("语义 worker 返回了无效向量".into());
    }
    let actual_backend = response.get("actualBackend").and_then(Value::as_str).unwrap_or("unknown").to_string();
    Ok(EmbeddingResponse { embedding, actual_backend })
  }

  fn read_message(&mut self) -> Result<Value, String> {
    let mut line = String::new();
    let count = self.stdout.read_line(&mut line).map_err(|e| format!("读取语义 worker 失败：{e}"))?;
    if count == 0 {
      return Err("语义 worker 已退出".into());
    }
    serde_json::from_str(&line).map_err(|e| format!("语义 worker 返回格式无效：{e}"))
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
      std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let _ = self.child.kill();
    let _ = self.child.wait();
  }
}

static WORKER: OnceLock<Mutex<Option<WorkerProcess>>> = OnceLock::new();

fn worker_slot() -> &'static Mutex<Option<WorkerProcess>> {
  WORKER.get_or_init(|| Mutex::new(None))
}

fn ensure_process(
  slot: &mut Option<WorkerProcess>,
  text_only: bool,
) -> Result<&mut WorkerProcess, String> {
  let device = crate::core::config::semantic_settings().map_err(|e| e.to_string())?.device;
  if slot.as_ref().is_some_and(|worker| worker.backend != device || (worker.text_only && !text_only)) {
    *slot = None;
  }
  if slot.is_none() {
    let manifest = ModelManifest::load(&model::model_dir()?)?;
    match WorkerProcess::spawn(&manifest, device, text_only) {
      Ok(worker) => *slot = Some(worker),
      Err(error) if device == crate::core::config::SemanticDevice::Gpu => {
        log::warn!("[semantic] GPU 初始化失败，回退 CPU：{error}");
        *slot = Some(WorkerProcess::spawn(&manifest, crate::core::config::SemanticDevice::Cpu, text_only)?);
      }
      Err(error) => return Err(error),
    }
  }
  Ok(slot.as_mut().expect("worker just initialized"))
}

pub fn embed_text(text: &str) -> Result<EmbeddingResponse, String> {
  let mut guard = worker_slot().lock().map_err(|e| e.to_string())?;
  let worker = ensure_process(&mut guard, true)?;
  let requested_gpu = worker.backend == crate::core::config::SemanticDevice::Gpu;
  match worker.request(json!({ "type": "embedText", "text": text })) {
    Ok(response) => Ok(response),
    Err(error) if requested_gpu => {
      log::warn!("[semantic] GPU 推理失败，回退 CPU：{error}");
      *guard = None;
      let worker = ensure_process(&mut guard, true)?;
      worker.request(json!({ "type": "embedText", "text": text }))
    }
    Err(error) => Err(error),
  }
}

#[allow(dead_code)] // 媒体抽帧分析接入后由图像塔调用
pub fn embed_image(path: &std::path::Path) -> Result<EmbeddingResponse, String> {
  let mut guard = worker_slot().lock().map_err(|e| e.to_string())?;
  let worker = ensure_process(&mut guard, false)?;
  let requested_gpu = worker.backend == crate::core::config::SemanticDevice::Gpu;
  let path_text = path.to_string_lossy().to_string();
  match worker.request(json!({ "type": "embedImage", "path": path_text })) {
    Ok(response) => Ok(response),
    Err(error) if requested_gpu => {
      log::warn!("[semantic] GPU 推理失败，回退 CPU：{error}");
      *guard = None;
      let worker = ensure_process(&mut guard, false)?;
      worker.request(json!({ "type": "embedImage", "path": path_text }))
    }
    Err(error) => Err(error),
  }
}

pub fn test_runtime() -> Result<SemanticRuntimeStatus, String> {
  let response = embed_text("状态检查")?;
  let settings = crate::core::config::semantic_settings().map_err(|e| e.to_string())?;
  let requested_backend = if settings.device == crate::core::config::SemanticDevice::Gpu { "directml" } else { "cpu" };
  Ok(SemanticRuntimeStatus {
    requested_backend: requested_backend.into(),
    actual_backend: response.actual_backend,
    model_dir: model::model_dir()?.to_string_lossy().to_string(),
    runtime_dir: model::runtime_dir()?.to_string_lossy().to_string(),
    dimension: model::EMBEDDING_DIM,
  })
}

pub fn reset_worker() {
  if let Ok(mut guard) = worker_slot().lock() {
    *guard = None;
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::PathBuf;

  #[test]
  #[ignore]
  fn real_model_smoke() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    std::env::set_var("ANIKIT_SEMANTIC_MODEL_DIR", root.join("spike/semantic-multilingual"));
    std::env::set_var("ANIKIT_SEMANTIC_RUNTIME_DIR", root.join("src-tauri/binaries/semantic-runtime"));
    std::env::set_var("ANIKIT_SEMANTIC_WORKER_EXE", root.join("src-tauri/target/debug/semantic-worker.exe"));
    reset_worker();
    let status = test_runtime().unwrap();
    assert_eq!(status.dimension, 512);
    let response = embed_image(&root.join("spike/semantic-multilingual/assets/CLIP.png")).unwrap();
    assert_eq!(response.embedding.len(), 512);
    assert!(response.embedding.iter().all(|value| value.is_finite()));
    reset_worker();
  }
}
