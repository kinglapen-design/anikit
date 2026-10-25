use std::{fs, io::Read, path::{Path, PathBuf}};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::types::ModelRevision;

pub const MODEL_ID: &str = "sentence-transformers/clip-ViT-B-32-multilingual-v1+openai/clip-vit-base-patch32";
pub const MODEL_REVISION: &str = "58edf8cada9e398793dca955574a48cbb7f18be2+3d74acf9a28c67741b2f4f2ea7635f0aaf6f0268";
pub const EMBEDDING_DIM: usize = 512;

#[derive(Debug, Clone)]
pub struct ModelManifest {
  pub revision: ModelRevision,
  pub text_path: PathBuf,
  pub vision_path: PathBuf,
  pub tokenizer_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ManifestDocument {
  text_model_revision: String,
  image_model_revision: String,
  embedding_dimension: usize,
  files: Vec<ManifestFile>,
}

#[derive(Debug, Deserialize)]
struct ManifestFile {
  path: String,
  sha256: String,
}

impl ModelManifest {
  pub fn load(model_dir: &Path) -> Result<Self, String> {
    let model_dir = canonical_existing(model_dir, "语义模型目录")?;
    let candidates = [model_dir.join("model-manifest.json"), model_dir.join("outputs/model-manifest.json")];
    let manifest_path = candidates
      .iter()
      .find(|path| path.is_file())
      .ok_or_else(|| format!("基础语义模型清单不存在：{}", model_dir.display()))?;
    let bytes = fs::read(manifest_path).map_err(|e| format!("读取模型清单失败：{e}"))?;
    let document: ManifestDocument = serde_json::from_slice(&bytes).map_err(|e| format!("模型清单格式无效：{e}"))?;
    let expected_revision = MODEL_REVISION.to_string();
    let actual_revision = format!("{}+{}", document.text_model_revision, document.image_model_revision);
    if actual_revision != expected_revision {
      return Err(format!("模型 revision 不匹配：{actual_revision}"));
    }
    if document.embedding_dimension != EMBEDDING_DIM {
      return Err(format!("模型向量维度不匹配：{}", document.embedding_dimension));
    }
    let file = |name: &str| -> Result<(PathBuf, String), String> {
      let record = document.files.iter().find(|file| file.path == name).ok_or_else(|| format!("模型清单缺少 {name}"))?;
      let path = model_dir.join(name);
      if !path.is_file() {
        return Err(format!("模型文件不存在：{}", path.display()));
      }
      let actual = sha256_file(&path)?;
      if actual != record.sha256 {
        return Err(format!("模型文件 SHA-256 不匹配：{name}"));
      }
      Ok((path, actual))
    };
    let (text_path, text_hash) = file("models/text.onnx")?;
    let (vision_path, _) = file("models/vision.onnx")?;
    let (tokenizer_path, _) = file("models/tokenizer.json")?;
    Ok(Self {
      revision: ModelRevision { id: MODEL_ID.into(), revision: expected_revision, dimension: EMBEDDING_DIM, weight_sha256: text_hash },
      text_path,
      vision_path,
      tokenizer_path,
    })
  }
}

pub fn model_dir() -> Result<PathBuf, String> {
  if let Some(path) = std::env::var_os("ANIKIT_SEMANTIC_MODEL_DIR") {
    return canonical_existing(&PathBuf::from(path), "语义模型目录");
  }
  if let Some(path) = crate::core::config::semantic_settings().map_err(|e| e.to_string())?.model_dir {
    return canonical_existing(&PathBuf::from(path), "语义模型目录");
  }
  dirs::data_local_dir()
    .map(|dir| dir.join("AniKit").join("models").join("multilingual-v1").join(MODEL_REVISION))
    .ok_or_else(|| "无法定位语义模型目录".into())
}

pub fn runtime_dir() -> Result<PathBuf, String> {
  if let Some(path) = std::env::var_os("ANIKIT_SEMANTIC_RUNTIME_DIR") {
    return canonical_existing(&PathBuf::from(path), "语义运行时目录");
  }
  let exe = std::env::current_exe().map_err(|e| format!("定位当前程序失败：{e}"))?;
  if let Some(parent) = exe.parent() {
    let portable = parent.join("semantic-runtime");
    if portable.is_dir() {
      return canonical_existing(&portable, "语义运行时目录");
    }
    let development = parent.join("..").join("..").join("binaries").join("semantic-runtime");
    if development.is_dir() {
      return canonical_existing(&development, "语义运行时目录");
    }
  }
  Err("未找到语义运行时（请先安装 ORT / DirectML 运行时）".into())
}

pub fn worker_exe() -> Result<PathBuf, String> {
  if let Some(path) = std::env::var_os("ANIKIT_SEMANTIC_WORKER_EXE") {
    return canonical_existing(&PathBuf::from(path), "语义 worker");
  }
  let exe = std::env::current_exe().map_err(|e| format!("定位当前程序失败：{e}"))?;
  let path = exe.parent().unwrap_or_else(|| Path::new(".")).join("semantic-worker.exe");
  if path.is_file() {
    canonical_existing(&path, "语义 worker")
  } else {
    Err(format!("语义 worker 不存在：{}（请先构建 semantic-worker）", path.display()))
  }
}

fn canonical_existing(path: &Path, label: &str) -> Result<PathBuf, String> {
  fs::canonicalize(path).map_err(|e| format!("{label}不存在或不可读 {}：{e}", path.display()))
}

fn sha256_file(path: &Path) -> Result<String, String> {
  let mut file = fs::File::open(path).map_err(|e| format!("读取模型文件失败 {}：{e}", path.display()))?;
  let mut hasher = Sha256::new();
  let mut buffer = vec![0_u8; 1024 * 1024];
  loop {
    let count = file.read(&mut buffer).map_err(|e| format!("读取模型文件失败：{e}"))?;
    if count == 0 {
      break;
    }
    hasher.update(&buffer[..count]);
  }
  Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn model_manifest_validates_revision_and_hashes() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("models")).unwrap();
    for name in ["text.onnx", "vision.onnx", "tokenizer.json"] {
      fs::write(dir.path().join("models").join(name), name.as_bytes()).unwrap();
    }
    let files = ["text.onnx", "vision.onnx", "tokenizer.json"].iter().map(|name| {
      let hash = sha256_file(&dir.path().join("models").join(name)).unwrap();
      serde_json::json!({ "path": format!("models/{name}"), "sha256": hash })
    }).collect::<Vec<_>>();
    let document = serde_json::json!({
      "text_model_revision": "58edf8cada9e398793dca955574a48cbb7f18be2",
      "image_model_revision": "3d74acf9a28c67741b2f4f2ea7635f0aaf6f0268",
      "embedding_dimension": 512,
      "files": files,
    });
    fs::write(dir.path().join("model-manifest.json"), serde_json::to_vec(&document).unwrap()).unwrap();
    let manifest = ModelManifest::load(dir.path()).unwrap();
    assert_eq!(manifest.revision.dimension, 512);
    assert!(manifest.text_path.is_file());
    fs::write(dir.path().join("models/text.onnx"), b"changed").unwrap();
    assert!(ModelManifest::load(dir.path()).is_err());
  }
}
