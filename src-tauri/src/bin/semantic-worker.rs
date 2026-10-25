use std::{
  collections::BTreeMap,
  fs,
  io::{BufRead, Write},
  path::{Path, PathBuf},
  process::ExitCode,
};

use image::{imageops::FilterType, RgbImage};
use ort::{
  environment::Environment,
  ep::{self, ExecutionProvider},
  session::{builder::GraphOptimizationLevel, Session},
  value::Tensor,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tokenizers::Tokenizer;

const CONTEXT_LENGTH: usize = 128;
const EMBEDDING_DIM: usize = 512;
const IMAGE_SIZE: usize = 224;
const IMAGE_MEAN: [f32; 3] = [0.48145466, 0.4578275, 0.40821073];
#[allow(clippy::excessive_precision)] // CLIP 官方预处理参数，不能截断改值
const IMAGE_STD: [f32; 3] = [0.26862954, 0.26130258, 0.27577711];

#[derive(Clone, Copy, PartialEq)]
enum Backend {
  Cpu,
  DirectMl,
}

impl Backend {
  fn parse(value: &str) -> Result<Self, String> {
    match value {
      "cpu" => Ok(Self::Cpu),
      "directml" => Ok(Self::DirectMl),
      _ => Err(format!("未知推理后端：{value}")),
    }
  }

  fn as_str(self) -> &'static str {
    match self {
      Self::Cpu => "cpu",
      Self::DirectMl => "directml",
    }
  }
}

struct Args {
  runtime: PathBuf,
  text_model: PathBuf,
  vision_model: PathBuf,
  tokenizer_path: PathBuf,
  backend: Backend,
  text_only: bool,
}

impl Args {
  fn parse() -> Result<Self, String> {
    let mut runtime = None;
    let mut text_model = None;
    let mut vision_model = None;
    let mut tokenizer_path = None;
    let mut backend = Backend::Cpu;
    let mut text_only = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
      match arg.as_str() {
        "--runtime-dir" => runtime = Some(PathBuf::from(args.next().ok_or("--runtime-dir 缺少路径")?)),
        "--text-model" => text_model = Some(PathBuf::from(args.next().ok_or("--text-model 缺少路径")?)),
        "--vision-model" => vision_model = Some(PathBuf::from(args.next().ok_or("--vision-model 缺少路径")?)),
        "--tokenizer" => tokenizer_path = Some(PathBuf::from(args.next().ok_or("--tokenizer 缺少路径")?)),
        "--backend" => backend = Backend::parse(&args.next().ok_or("--backend 缺少取值")?)?,
        "--text-only" => text_only = true,
        _ => return Err(format!("未知参数：{arg}")),
      }
    }
    Ok(Self {
      runtime: fs::canonicalize(runtime.ok_or("缺少 --runtime-dir")?).map_err(|e| format!("语义运行时路径无效：{e}"))?,
      text_model: fs::canonicalize(text_model.ok_or("缺少 --text-model")?).map_err(|e| format!("文本模型路径无效：{e}"))?,
      vision_model: fs::canonicalize(vision_model.ok_or("缺少 --vision-model")?).map_err(|e| format!("图像模型路径无效：{e}"))?,
      tokenizer_path: fs::canonicalize(tokenizer_path.ok_or("缺少 --tokenizer")?).map_err(|e| format!("tokenizer 路径无效：{e}"))?,
      backend,
      text_only,
    })
  }
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Request {
  #[serde(rename = "embedText")]
  EmbedText { id: u64, text: String },
  #[serde(rename = "embedImage")]
  EmbedImage { id: u64, path: String },
  #[serde(rename = "shutdown")]
  Shutdown { id: u64 },
}

struct Worker {
  backend: Backend,
  tokenizer: Tokenizer,
  text: Session,
  vision: Option<Session>,
  actual_backend: Option<String>,
}

impl Worker {
  fn new(args: Args) -> Result<Self, String> {
    set_dll_directory(&args.runtime)?;
    let environment = ort::init_from(args.runtime.join("onnxruntime.dll"))
      .map_err(|e| e.to_string())?
      .with_name("anikit-semantic-worker")
      .with_telemetry(false)
      .build()
      .map_err(|e| e.to_string())?;
    if args.backend == Backend::DirectMl && !ep::DirectML::is_available().map_err(|e| e.to_string())? {
      return Err("当前运行时没有可用的 DirectML 后端".into());
    }
    let tokenizer = Tokenizer::from_file(&args.tokenizer_path).map_err(|e| e.to_string())?;
    let profile_dir = std::env::temp_dir().join("anikit-semantic-profiles").join(std::process::id().to_string());
    fs::create_dir_all(&profile_dir).map_err(|e| format!("创建 profiling 目录失败：{e}"))?;
    let text = create_session(&environment, args.backend, &args.text_model, &profile_dir.join("text.json"))?;
    let vision = if args.text_only {
      None
    } else {
      Some(create_session(&environment, args.backend, &args.vision_model, &profile_dir.join("vision.json"))?)
    };
    println!("{}", json!({ "type": "ready", "requestedBackend": args.backend.as_str() }));
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    Ok(Self { backend: args.backend, tokenizer, text, vision, actual_backend: None })
  }

  fn embed_text(&mut self, text: &str) -> Result<Vec<f32>, String> {
    let encoding = self.tokenizer.encode(text, true).map_err(|e| e.to_string())?;
    let ids = encoding.get_ids();
    if ids.len() != CONTEXT_LENGTH {
      return Err(format!("文本 token 数量异常：{}", ids.len()));
    }
    let ids = ids.iter().map(|value| i64::from(*value)).collect::<Vec<_>>();
    let mask = encoding.get_attention_mask().iter().map(|value| i64::from(*value)).collect::<Vec<_>>();
    let id_tensor = Tensor::from_array(([1_usize, CONTEXT_LENGTH], ids)).map_err(|e| e.to_string())?;
    let mask_tensor = Tensor::from_array(([1_usize, CONTEXT_LENGTH], mask)).map_err(|e| e.to_string())?;
    let outputs = self.text.run(ort::inputs![id_tensor, mask_tensor]).map_err(|e| e.to_string())?;
    extract_embedding(&outputs, "unnorm_text_features")
  }

  fn embed_image(&mut self, path: &Path) -> Result<Vec<f32>, String> {
    let Some(session) = self.vision.as_mut() else {
      return Err("图像模型未加载".into());
    };
    let source = image::open(path).map_err(|e| format!("读取图片失败 {}：{e}", path.display()))?.to_rgb8();
    if source.width() == 0 || source.height() == 0 {
      return Err("图片尺寸为零".into());
    }
    let (width, height) = if source.width() <= source.height() {
      (IMAGE_SIZE as u32, (IMAGE_SIZE as u64 * source.height() as u64 / source.width() as u64) as u32)
    } else {
      ((IMAGE_SIZE as u64 * source.width() as u64 / source.height() as u64) as u32, IMAGE_SIZE as u32)
    };
    let resized = image::imageops::resize(&source, width, height, FilterType::CatmullRom);
    let cropped = image::imageops::crop_imm(
      &resized,
      (width - IMAGE_SIZE as u32) / 2,
      (height - IMAGE_SIZE as u32) / 2,
      IMAGE_SIZE as u32,
      IMAGE_SIZE as u32,
    )
    .to_image();
    let pixels = normalize_image(&cropped);
    let tensor = Tensor::from_array(([1_usize, 3, IMAGE_SIZE, IMAGE_SIZE], pixels)).map_err(|e| e.to_string())?;
    let outputs = session.run(ort::inputs![tensor]).map_err(|e| e.to_string())?;
    extract_embedding(&outputs, "unnorm_image_features")
  }

  fn finish_profiles(&mut self) -> Result<String, String> {
    if let Some(actual) = &self.actual_backend {
      return Ok(actual.clone());
    }
    let mut providers = BTreeMap::new();
    let text_profile = self.text.end_profiling().map_err(|e| e.to_string())?;
    collect_providers(&text_profile, &mut providers)?;
    if let Some(vision) = self.vision.as_mut() {
      let profile = vision.end_profiling().map_err(|e| e.to_string())?;
      collect_providers(&profile, &mut providers)?;
    }
    let has_dml = providers.keys().any(|name| name.to_ascii_lowercase().contains("dml"));
    let has_cpu = providers.keys().any(|name| name.to_ascii_lowercase().contains("cpu"));
    if self.backend == Backend::DirectMl && !has_dml {
      return Err("DirectML 请求未落到 GPU 节点".into());
    }
    if self.backend == Backend::Cpu && (has_dml || !has_cpu) {
      return Err("CPU 会话出现意外执行后端".into());
    }
    let actual = match (has_dml, has_cpu) {
      (true, true) => "directml+cpu",
      (true, false) => "directml",
      (false, true) => "cpu",
      (false, false) => return Err("无法确认实际执行后端".into()),
    };
    self.actual_backend = Some(actual.into());
    Ok(self.actual_backend.clone().unwrap())
  }
}

fn create_session(environment: &Environment, backend: Backend, model: &Path, profile: &Path) -> Result<Session, String> {
  let providers = match backend {
    Backend::Cpu => vec![ep::CPU::default().build().error_on_failure()],
    Backend::DirectMl => vec![
      ep::DirectML::default()
        .with_performance_preference(ep::directml::PerformancePreference::HighPerformance)
        .with_device_filter(ep::directml::DeviceFilter::Gpu)
        .build()
        .error_on_failure(),
      ep::CPU::default().build().error_on_failure(),
    ],
  };
  Session::builder(environment)
    .map_err(|e| e.to_string())?
    .with_execution_providers(providers)
    .map_err(|e| e.to_string())?
    .with_optimization_level(GraphOptimizationLevel::All)
    .map_err(|e| e.to_string())?
    .with_intra_threads(4)
    .map_err(|e| e.to_string())?
    .with_profiling(profile)
    .map_err(|e| e.to_string())?
    .commit_from_file(model)
    .map_err(|e| format!("加载模型 {} 失败：{e}", model.display()))
}

fn extract_embedding(outputs: &ort::session::SessionOutputs, name: &str) -> Result<Vec<f32>, String> {
  let (shape, data) = outputs[name].try_extract_tensor::<f32>().map_err(|e| e.to_string())?;
  if shape.iter().copied().collect::<Vec<_>>() != [1, EMBEDDING_DIM as i64] || data.len() != EMBEDDING_DIM {
    return Err(format!("模型输出形状异常：{shape:?}"));
  }
  let norm = data.iter().map(|value| value * value).sum::<f32>().sqrt();
  if !norm.is_finite() || norm <= 0.0 {
    return Err("模型输出无法归一化".into());
  }
  let unit = data.iter().map(|value| value / norm).collect::<Vec<_>>();
  let unit_norm = unit.iter().map(|value| value * value).sum::<f32>().sqrt();
  if (unit_norm - 1.0).abs() > 0.00001 {
    return Err("模型输出归一化失败".into());
  }
  Ok(unit)
}

fn normalize_image(image: &RgbImage) -> Vec<f32> {
  let plane = IMAGE_SIZE * IMAGE_SIZE;
  let mut output = vec![0.0_f32; 3 * plane];
  for (x, y, pixel) in image.enumerate_pixels() {
    let offset = y as usize * IMAGE_SIZE + x as usize;
    for channel in 0..3 {
      output[channel * plane + offset] = (f32::from(pixel[channel]) / 255.0 - IMAGE_MEAN[channel]) / IMAGE_STD[channel];
    }
  }
  output
}

fn collect_providers(path: &str, providers: &mut BTreeMap<String, usize>) -> Result<(), String> {
  let value: Value = serde_json::from_slice(&fs::read(path).map_err(|e| format!("读取 profiling 失败：{e}"))?).map_err(|e| e.to_string())?;
  fn walk(value: &Value, providers: &mut BTreeMap<String, usize>) {
    match value {
      Value::Object(object) => {
        if let Some(provider) = object.get("args").and_then(Value::as_object).and_then(|args| args.get("provider")).and_then(Value::as_str) {
          *providers.entry(provider.to_owned()).or_default() += 1;
        }
        for child in object.values() {
          walk(child, providers);
        }
      }
      Value::Array(array) => {
        for child in array {
          walk(child, providers);
        }
      }
      _ => {}
    }
  }
  walk(&value, providers);
  Ok(())
}

#[cfg(windows)]
fn set_dll_directory(runtime: &Path) -> Result<(), String> {
  use std::os::windows::ffi::OsStrExt;
  use windows::{core::PCWSTR, Win32::System::LibraryLoader::SetDllDirectoryW};
  let wide = runtime.as_os_str().encode_wide().chain(Some(0)).collect::<Vec<_>>();
  unsafe { SetDllDirectoryW(PCWSTR(wide.as_ptr())) }.map_err(|e| format!("设置语义运行时目录失败：{e}"))?;
  Ok(())
}

fn main() -> ExitCode {
  match run() {
    Ok(()) => ExitCode::SUCCESS,
    Err(error) => {
      eprintln!("semantic-worker：{error}");
      ExitCode::FAILURE
    }
  }
}

fn run() -> Result<(), String> {
  let mut worker = Worker::new(Args::parse()?)?;
  let stdin = std::io::stdin();
  for line in stdin.lock().lines() {
    let line = line.map_err(|e| format!("读取请求失败：{e}"))?;
    if line.trim().is_empty() {
      continue;
    }
    let request: Request = serde_json::from_str(&line).map_err(|e| format!("请求格式无效：{e}"))?;
    match request {
      Request::EmbedText { id, text } => {
        let result = worker.embed_text(&text).and_then(|embedding| {
          let actual = worker.finish_profiles()?;
          Ok((actual, embedding))
        });
        match result {
          Ok((actual, embedding)) => write_response(json!({ "type": "result", "id": id, "actualBackend": actual, "embedding": embedding }))?,
          Err(error) => write_response(json!({ "type": "error", "id": id, "message": error }))?,
        }
      }
      Request::EmbedImage { id, path } => {
        let result = worker.embed_image(Path::new(&path)).and_then(|embedding| {
          let actual = worker.finish_profiles()?;
          Ok((actual, embedding))
        });
        match result {
          Ok((actual, embedding)) => write_response(json!({ "type": "result", "id": id, "actualBackend": actual, "embedding": embedding }))?,
          Err(error) => write_response(json!({ "type": "error", "id": id, "message": error }))?,
        }
      }
      Request::Shutdown { id } => {
        write_response(json!({ "type": "shutdown", "id": id }))?;
        return Ok(());
      }
    }
  }
  Ok(())
}

fn write_response(value: Value) -> Result<(), String> {
  let mut stdout = std::io::stdout().lock();
  serde_json::to_writer(&mut stdout, &value).map_err(|e| e.to_string())?;
  stdout.write_all(b"\n").map_err(|e| e.to_string())?;
  stdout.flush().map_err(|e| e.to_string())
}
