mod preprocess;

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{Context, Result, anyhow, bail};
use ort::{
    environment::Environment,
    ep::{self, ExecutionProvider},
    session::{Session, builder::GraphOptimizationLevel},
    value::Tensor,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokenizers::Tokenizer;
use windows_sys::Win32::System::{
    LibraryLoader::SetDllDirectoryW,
    ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
    Threading::GetCurrentProcess,
};

const CONTEXT_LENGTH: usize = 52;
const EMBEDDING_DIM: usize = 512;

#[derive(Clone, Copy, PartialEq)]
enum Backend {
    Cpu,
    DirectMl,
}

impl Backend {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "cpu" => Ok(Self::Cpu),
            "directml" => Ok(Self::DirectMl),
            _ => bail!("--backend must be cpu or directml, got {value:?}"),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::DirectMl => "directml",
        }
    }

    fn requested_providers(self) -> Vec<&'static str> {
        match self {
            Self::Cpu => vec!["CPUExecutionProvider"],
            Self::DirectMl => vec!["DmlExecutionProvider", "CPUExecutionProvider"],
        }
    }
}

struct Args {
    root: PathBuf,
    backend: Backend,
    prepare_native: bool,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut root = std::env::var_os("SPIKE_ROOT")
            .map(PathBuf::from)
            .unwrap_or(std::env::current_dir()?);
        let mut backend = Backend::Cpu;
        let mut prepare_native = false;
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--root" => {
                    root = PathBuf::from(args.next().context("--root requires a path")?);
                }
                "--backend" => {
                    backend = Backend::parse(&args.next().context("--backend requires a value")?)?;
                }
                "--prepare-native" => prepare_native = true,
                "-h" | "--help" => {
                    println!(
                        "semantic-clip-spike [--root PATH] [--backend cpu|directml] [--prepare-native]"
                    );
                    std::process::exit(0);
                }
                _ => bail!("unknown argument: {arg}"),
            }
        }
        Ok(Self {
            root: root
                .canonicalize()
                .with_context(|| format!("invalid root: {}", root.display()))?,
            backend,
            prepare_native,
        })
    }
}

#[derive(Deserialize)]
struct Reference {
    texts: Vec<TextReference>,
    image: ImageReference,
    cosine_similarities: BTreeMap<String, f32>,
    official_candidates: Vec<String>,
    official_candidate_probabilities: Vec<f32>,
}

#[derive(Deserialize)]
struct TextReference {
    text: String,
    token_ids: Vec<u32>,
    unnormalized: Vec<f32>,
    normalized: Vec<f32>,
}

#[derive(Deserialize)]
struct ImageReference {
    unnormalized: Vec<f32>,
    normalized: Vec<f32>,
}

#[derive(Deserialize)]
struct NativePythonReference {
    tensor_sha256: String,
    image: ImageReference,
    cosine_similarities: BTreeMap<String, f32>,
    official_candidate_probabilities: Vec<f32>,
}

struct Embedding {
    raw: Vec<f32>,
    unit: Vec<f32>,
    shape: Vec<i64>,
}

#[derive(Serialize)]
struct VectorComparison {
    shape: Vec<i64>,
    finite: bool,
    unnormalized_l2: f32,
    normalized_l2: f32,
    max_abs_diff_unnormalized: f32,
    max_abs_diff_normalized: f32,
    first_8_normalized: Vec<f32>,
}

#[derive(Serialize)]
struct MemoryUsage {
    rss_bytes: usize,
    peak_rss_bytes: usize,
}

#[derive(Serialize)]
struct ProfileSummary {
    path: String,
    node_provider_counts: BTreeMap<String, usize>,
}

fn main() {
    let args = match Args::parse() {
        Ok(args) => args,
        Err(error) => {
            eprintln!("argument error: {error:#}");
            std::process::exit(2);
        }
    };
    let output = args.root.join("outputs");
    if let Err(error) = fs::create_dir_all(&output) {
        eprintln!("cannot create {}: {error}", output.display());
        std::process::exit(2);
    }
    let report_path = output.join(format!("rust-{}-native.json", args.backend.as_str()));
    let mut report = json!({
        "status": "running",
        "requested_backend": args.backend.as_str(),
        "requested_execution_providers": args.backend.requested_providers(),
        "actual_backend": "not_run",
        "backend_verified": false,
        "ort_crate": "2.0.0-rc.15",
        "ort_api_version": ort::MINOR_VERSION,
        "runtime_directory": args.root.join("runtime"),
        "comparison_tolerance": if args.backend == Backend::Cpu { 0.0002_f32 } else { 0.005_f32 },
    });
    let result = run(&args, &mut report);
    match result {
        Ok(()) => {
            if args.prepare_native {
                report["status"] = json!("prepared");
            } else {
                report["status"] = json!("passed");
                report["backend_verified"] = json!(true);
            }
        }
        Err(error) => {
            report["status"] = json!("failed");
            report["error"] = json!(format!("{error:#}"));
            eprintln!("spike failed: {error:#}");
        }
    }
    let rendered = serde_json::to_string_pretty(&report).expect("report serialization");
    if let Err(error) = fs::write(&report_path, format!("{rendered}\n")) {
        eprintln!("cannot write {}: {error}", report_path.display());
        std::process::exit(2);
    }
    println!("{rendered}");
    if report["status"] != "passed" && report["status"] != "prepared" {
        std::process::exit(1);
    }
}

fn run(args: &Args, report: &mut Value) -> Result<()> {
    if args.prepare_native {
        prepare_native_input(args, report)?;
        return Ok(());
    }
    let runtime = args.root.join("runtime");
    set_dll_directory(&runtime)?;
    report["private_runtime_dlls"] = json!(dll_records(&runtime)?);
    report["memory"] = json!({ "process_start": memory_usage()? });

    let start = Instant::now();
    let environment = ort::init_from(runtime.join("onnxruntime.dll"))
        .map_err(|error| anyhow!(error.to_string()))?
        .with_name("semantic-clip-spike")
        .with_telemetry(false)
        .build()?;
    report["timings_ms"] = json!({ "environment_init": milliseconds(start) });
    report["ort_build_info"] = json!(ort::info());
    let directml_available = ep::DirectML::is_available()?;
    report["directml_available"] = json!(directml_available);
    if args.backend == Backend::DirectMl && !directml_available {
        bail!("DirectML was requested but is not available in the loaded runtime");
    }

    let reference: Reference = serde_json::from_slice(
        &fs::read(args.root.join("outputs/reference.json")).context("read Python reference")?,
    )?;
    let tokenizer = Tokenizer::from_file(args.root.join("models/tokenizer.json"))
        .map_err(|error| anyhow!(error.to_string()))?;
    let mut text_session = create_session(
        &environment,
        args,
        "text",
        &args.root.join("models/text.onnx"),
        report,
    )?;
    report["memory"]["after_text_session"] = json!(memory_usage()?);
    let mut vision_session = create_session(
        &environment,
        args,
        "vision",
        &args.root.join("models/vision.onnx"),
        report,
    )?;
    report["memory"]["after_vision_session"] = json!(memory_usage()?);

    let tolerance = if args.backend == Backend::Cpu {
        0.0002_f32
    } else {
        0.005_f32
    };
    let mut text_results = Vec::new();
    let mut unit_texts = BTreeMap::new();
    let mut text_timings = BTreeMap::new();
    for item in &reference.texts {
        let encoding = tokenizer
            .encode(item.text.as_str(), true)
            .map_err(|error| anyhow!(error.to_string()))?;
        let ids = encoding.get_ids();
        if ids != item.token_ids {
            bail!("Rust token IDs differ from Python for {:?}", item.text);
        }
        let input = ids.iter().map(|id| i64::from(*id)).collect::<Vec<_>>();
        if input.len() != CONTEXT_LENGTH {
            bail!("expected {CONTEXT_LENGTH} token IDs, got {}", input.len());
        }
        let tensor = Tensor::from_array(([1_usize, CONTEXT_LENGTH], input))?;
        let start = Instant::now();
        let embedding = infer(&mut text_session, tensor, "unnorm_text_features")?;
        text_timings.insert(item.text.clone(), milliseconds(start));
        let comparison =
            compare_embedding(&embedding, &item.unnormalized, &item.normalized, tolerance)?;
        text_results.push(json!({
            "text": item.text,
            "token_ids": item.token_ids,
            "token_ids_match_python": true,
            "comparison": comparison,
        }));
        unit_texts.insert(item.text.clone(), embedding.unit);
    }
    report["text_results"] = json!(text_results);
    report["timings_ms"]["text_inference"] = json!(text_timings);
    report["memory"]["after_text_inference"] = json!(memory_usage()?);

    let image_path = args.root.join("vendor/Chinese-CLIP/examples/pokemon.jpeg");
    let native_image = preprocess::load_jpeg(&image_path)?;
    let python_image_data = read_image_input(&args.root.join("outputs/image_input.f32"))?;
    let preprocessing_comparison = preprocess::compare(&native_image.data, &python_image_data)?;
    let (_, native_tensor_hash) = write_native_tensor(args, &native_image.data)?;
    let native_reference_path = args.root.join("outputs/native-python-reference.json");
    let native_reference: NativePythonReference =
        serde_json::from_slice(&fs::read(&native_reference_path).with_context(|| {
            format!(
                "read {}; run --prepare-native and scripts/native_reference.py first",
                native_reference_path.display()
            )
        })?)?;
    if native_reference.tensor_sha256 != native_tensor_hash {
        bail!(
            "native Python reference is stale; rerun --prepare-native and scripts/native_reference.py"
        );
    }
    report["native_preprocessing"] = json!({
        "source": image_path.to_string_lossy(),
        "source_dimensions": [native_image.source_width, native_image.source_height],
        "output_shape": [1, 3, 224, 224],
        "pipeline": "JPEG decode + direct 224x224 CatmullRom bicubic resize + RGB NCHW normalize; no center crop",
        "decode_resize_normalize_ms": native_image.elapsed_ms,
        "python_reference_tensor": "outputs/image_input.f32 (comparison only; not used for inference)",
        "comparison_to_python": preprocessing_comparison,
        "native_tensor_used_for_inference": true,
        "python_native_reference": native_reference_path,
        "python_native_reference_tensor_sha256": &native_reference.tensor_sha256,
    });
    let image_tensor = Tensor::from_array(([1_usize, 3, 224, 224], native_image.data.clone()))?;
    let start = Instant::now();
    let image_embedding = infer(&mut vision_session, image_tensor, "unnorm_image_features")?;
    report["timings_ms"]["vision_inference"] = json!(milliseconds(start));
    let image_comparison = compare_embedding(
        &image_embedding,
        &native_reference.image.unnormalized,
        &native_reference.image.normalized,
        tolerance,
    )?;
    report["image_result"] = json!({
        "source": "Rust native JPEG preprocessing; no shared FP32 model input",
        "comparison_to_python_native_tensor": image_comparison,
        "cross_decoder_difference_from_python_pil": {
            "max_abs_diff_unnormalized": max_abs_diff(&image_embedding.raw, &reference.image.unnormalized)?,
            "max_abs_diff_normalized": max_abs_diff(&image_embedding.unit, &reference.image.normalized)?,
        },
    });
    report["memory"]["after_vision_inference"] = json!(memory_usage()?);

    let mut similarities = BTreeMap::new();
    let mut similarity_results = Vec::new();
    let mut cross_decoder_max_cosine_diff = 0.0_f32;
    for (text, unit_text) in &unit_texts {
        let similarity = dot(&image_embedding.unit, unit_text);
        let python_similarity = native_reference.cosine_similarities[text];
        let python_pil_similarity = reference.cosine_similarities[text];
        if (similarity - python_similarity).abs() > tolerance {
            bail!(
                "cosine mismatch for {text:?}: Rust={similarity}, Python-native={python_similarity}"
            );
        }
        cross_decoder_max_cosine_diff =
            cross_decoder_max_cosine_diff.max((similarity - python_pil_similarity).abs());
        similarities.insert(text.clone(), similarity);
        similarity_results.push(json!({
            "text": text,
            "rust_cosine": similarity,
            "python_native_cosine": python_similarity,
            "python_pil_cosine": python_pil_similarity,
        }));
    }
    report["similarities"] = json!(similarity_results);
    report["cross_decoder_max_cosine_diff"] = json!(cross_decoder_max_cosine_diff);
    let probabilities = softmax_probabilities(&reference.official_candidates, &similarities)?;
    let probability_diff = probabilities
        .iter()
        .zip(&native_reference.official_candidate_probabilities)
        .fold(0.0_f32, |acc, (left, right)| acc.max((left - right).abs()));
    if probability_diff > tolerance {
        bail!("official sample probability mismatch vs Python-native: {probability_diff}");
    }
    let cross_decoder_probability_diff = probabilities
        .iter()
        .zip(&reference.official_candidate_probabilities)
        .fold(0.0_f32, |acc, (left, right)| acc.max((left - right).abs()));
    let best = reference.official_candidates[probabilities
        .iter()
        .enumerate()
        .max_by(|left, right| left.1.total_cmp(right.1))
        .context("empty probabilities")?
        .0]
        .clone();
    if best != "皮卡丘" {
        bail!("official sample best class is {best:?}, expected 皮卡丘");
    }
    report["official_sample"] = json!({
        "candidates": reference.official_candidates,
        "probabilities": probabilities,
        "python_native_probabilities": native_reference.official_candidate_probabilities,
        "python_pil_probabilities": reference.official_candidate_probabilities,
        "max_abs_probability_diff_vs_python_native": probability_diff,
        "cross_decoder_max_probability_diff": cross_decoder_probability_diff,
        "best": best,
    });

    let robot = reference
        .texts
        .first()
        .context("missing robot text reference")?;
    if robot.text != "机器人洗碗" {
        bail!("unexpected first text reference: {:?}", robot.text);
    }
    warmed_benchmark(
        &mut text_session,
        &mut vision_session,
        robot,
        &image_path,
        report,
    )?;

    let text_profile = finish_profile(&mut text_session)?;
    let vision_profile = finish_profile(&mut vision_session)?;
    let providers = text_profile
        .node_provider_counts
        .iter()
        .chain(&vision_profile.node_provider_counts)
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    let has_dml = providers
        .iter()
        .any(|name| name.to_ascii_lowercase().contains("dml"));
    let has_cpu = providers
        .iter()
        .any(|name| name.to_ascii_lowercase().contains("cpu"));
    report["profiles"] = json!({ "text": text_profile, "vision": vision_profile });
    report["actual_backend"] = match (has_dml, has_cpu) {
        (true, true) => json!("DirectML+CPU (ORT profiled node placement)"),
        (true, false) => json!("DirectML (ORT profiled node placement)"),
        (false, true) => json!("CPU only (ORT profiled node placement)"),
        (false, false) => json!("unknown (no provider fields in ORT profiles)"),
    };
    if args.backend == Backend::DirectMl && !has_dml {
        bail!(
            "DirectML was requested, but ORT profiling shows no DML nodes; treating this as visible CPU fallback, not GPU success"
        );
    }
    if args.backend == Backend::Cpu && (has_dml || !has_cpu) {
        bail!("CPU-only session has unexpected profiled providers: {providers:?}");
    }
    report["memory"]["final"] = json!(memory_usage()?);
    Ok(())
}

fn prepare_native_input(args: &Args, report: &mut Value) -> Result<()> {
    let image_path = args.root.join("vendor/Chinese-CLIP/examples/pokemon.jpeg");
    let native = preprocess::load_jpeg(&image_path)?;
    let python = read_image_input(&args.root.join("outputs/image_input.f32"))?;
    let comparison = preprocess::compare(&native.data, &python)?;
    let (tensor_path, tensor_hash) = write_native_tensor(args, &native.data)?;
    report["native_preprocessing"] = json!({
        "source": image_path.to_string_lossy(),
        "source_dimensions": [native.source_width, native.source_height],
        "output_shape": [1, 3, 224, 224],
        "pipeline": "JPEG decode + direct 224x224 CatmullRom bicubic resize + RGB NCHW normalize; no center crop",
        "decode_resize_normalize_ms": native.elapsed_ms,
        "comparison_to_python": comparison,
        "native_tensor_written": tensor_path,
        "native_tensor_sha256": tensor_hash,
        "next_step": "Run scripts/native_reference.py, then run without --prepare-native",
    });
    Ok(())
}

fn write_native_tensor(args: &Args, data: &[f32]) -> Result<(PathBuf, String)> {
    let bytes = preprocess::little_endian_bytes(data);
    let hash = preprocess::sha256(&bytes);
    let path = args.root.join("outputs/native_image_input.f32");
    fs::write(&path, bytes).with_context(|| format!("write {}", path.display()))?;
    Ok((path, hash))
}

fn warmed_benchmark(
    text_session: &mut Session,
    vision_session: &mut Session,
    robot: &TextReference,
    image_path: &Path,
    report: &mut Value,
) -> Result<()> {
    let ids = robot
        .token_ids
        .iter()
        .map(|id| i64::from(*id))
        .collect::<Vec<_>>();
    for _ in 0..3 {
        let tensor = Tensor::from_array(([1_usize, CONTEXT_LENGTH], ids.clone()))?;
        infer(text_session, tensor, "unnorm_text_features")?;
    }
    let mut text_times = Vec::new();
    for _ in 0..20 {
        let tensor = Tensor::from_array(([1_usize, CONTEXT_LENGTH], ids.clone()))?;
        let start = Instant::now();
        infer(text_session, tensor, "unnorm_text_features")?;
        text_times.push(milliseconds(start));
    }
    for _ in 0..2 {
        let native = preprocess::load_jpeg(image_path)?;
        let tensor = Tensor::from_array(([1_usize, 3, 224, 224], native.data))?;
        infer(vision_session, tensor, "unnorm_image_features")?;
    }
    let mut preprocess_times = Vec::new();
    let mut vision_times = Vec::new();
    for _ in 0..10 {
        let native = preprocess::load_jpeg(image_path)?;
        preprocess_times.push(native.elapsed_ms);
        let tensor = Tensor::from_array(([1_usize, 3, 224, 224], native.data))?;
        let start = Instant::now();
        infer(vision_session, tensor, "unnorm_image_features")?;
        vision_times.push(milliseconds(start));
    }
    report["warmed_benchmark"] = json!({
        "text_query": robot.text,
        "text_warmup_iterations": 3,
        "text_measured_iterations": 20,
        "vision_warmup_iterations": 2,
        "vision_measured_iterations": 10,
        "percentile_method": "linear interpolation over sorted measured samples",
        "native_preprocess_ms": timing_summary(&preprocess_times),
        "text_inference_ms": timing_summary(&text_times),
        "vision_inference_ms": timing_summary(&vision_times),
        "raw_samples": {
            "native_preprocess_ms": preprocess_times,
            "text_inference_ms": text_times,
            "vision_inference_ms": vision_times,
        },
    });
    report["memory"]["after_warmed_benchmark"] = json!(memory_usage()?);
    Ok(())
}

fn timing_summary(samples: &[f64]) -> Value {
    if samples.is_empty() {
        return json!({ "count": 0 });
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    json!({
        "count": sorted.len(),
        "min": sorted[0],
        "p50": percentile(&sorted, 0.50),
        "p95": percentile(&sorted, 0.95),
        "mean": sorted.iter().sum::<f64>() / sorted.len() as f64,
        "max": sorted[sorted.len() - 1],
    })
}

fn percentile(sorted: &[f64], quantile: f64) -> f64 {
    let position = (sorted.len() - 1) as f64 * quantile;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let fraction = position - lower as f64;
    sorted[lower] * (1.0 - fraction) + sorted[upper] * fraction
}

fn create_session(
    environment: &Environment,
    args: &Args,
    kind: &str,
    model: &Path,
    report: &mut Value,
) -> Result<Session> {
    let profile = args.root.join("outputs").join(format!(
        "rust-{}-{kind}-profile.json",
        args.backend.as_str()
    ));
    let providers = match args.backend {
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
    let start = Instant::now();
    let session = Session::builder(environment)?
        .with_execution_providers(providers)
        .map_err(|error| anyhow!(error.to_string()))?
        .with_optimization_level(GraphOptimizationLevel::All)
        .map_err(|error| anyhow!(error.to_string()))?
        .with_intra_threads(4)
        .map_err(|error| anyhow!(error.to_string()))?
        .with_profiling(&profile)
        .map_err(|error| anyhow!(error.to_string()))?
        .commit_from_file(model)
        .with_context(|| {
            format!(
                "create {kind} session with {} backend",
                args.backend.as_str()
            )
        })?;
    let elapsed = milliseconds(start);
    println!("{kind} session load: {elapsed:.3} ms");
    report["timings_ms"][format!("{kind}_session_load").as_str()] = json!(elapsed);
    Ok(session)
}

fn infer(
    session: &mut Session,
    tensor: Tensor<impl ort::value::PrimitiveTensorElementType + std::fmt::Debug>,
    output: &str,
) -> Result<Embedding> {
    let outputs = session.run(ort::inputs![tensor])?;
    let (shape, data) = outputs[output].try_extract_tensor::<f32>()?;
    let shape = shape.iter().copied().collect::<Vec<_>>();
    if shape != [1, EMBEDDING_DIM as i64] || data.len() != EMBEDDING_DIM {
        bail!(
            "unexpected {output} shape {shape:?}, elements={}",
            data.len()
        );
    }
    let raw = data.to_vec();
    let norm = raw.iter().map(|value| value * value).sum::<f32>().sqrt();
    if !norm.is_finite() || norm <= 0.0 {
        bail!("invalid raw embedding norm: {norm}");
    }
    let unit = raw.iter().map(|value| value / norm).collect();
    Ok(Embedding { raw, unit, shape })
}

fn compare_embedding(
    embedding: &Embedding,
    expected_raw: &[f32],
    expected_unit: &[f32],
    tolerance: f32,
) -> Result<VectorComparison> {
    let finite = embedding.raw.iter().all(|value| value.is_finite());
    if !finite {
        bail!("embedding contains NaN or infinity");
    }
    let raw_diff = max_abs_diff(&embedding.raw, expected_raw)?;
    let unit_diff = max_abs_diff(&embedding.unit, expected_unit)?;
    if raw_diff > tolerance || unit_diff > tolerance {
        bail!(
            "embedding differs from Python: raw={raw_diff}, normalized={unit_diff}, tolerance={tolerance}"
        );
    }
    let unnormalized_l2 = embedding
        .raw
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    let normalized_l2 = embedding
        .unit
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    if (normalized_l2 - 1.0).abs() > 0.00001 {
        bail!("normalized embedding L2 is {normalized_l2}");
    }
    Ok(VectorComparison {
        shape: embedding.shape.clone(),
        finite,
        unnormalized_l2,
        normalized_l2,
        max_abs_diff_unnormalized: raw_diff,
        max_abs_diff_normalized: unit_diff,
        first_8_normalized: embedding.unit.iter().take(8).copied().collect(),
    })
}

fn max_abs_diff(left: &[f32], right: &[f32]) -> Result<f32> {
    if left.len() != right.len() {
        bail!("vector lengths differ: {} != {}", left.len(), right.len());
    }
    Ok(left
        .iter()
        .zip(right)
        .fold(0.0_f32, |acc, (left, right)| acc.max((left - right).abs())))
}

fn dot(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn softmax_probabilities(
    candidates: &[String],
    similarities: &BTreeMap<String, f32>,
) -> Result<Vec<f32>> {
    let logits = candidates
        .iter()
        .map(|text| {
            similarities
                .get(text)
                .copied()
                .map(|value| 100.0 * value)
                .context("missing similarity")
        })
        .collect::<Result<Vec<_>>>()?;
    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let exponentials = logits
        .iter()
        .map(|value| (*value - max).exp())
        .collect::<Vec<_>>();
    let sum = exponentials.iter().sum::<f32>();
    Ok(exponentials.iter().map(|value| value / sum).collect())
}

fn read_image_input(path: &Path) -> Result<Vec<f32>> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let expected = 1 * 3 * 224 * 224 * size_of::<f32>();
    if bytes.len() != expected {
        bail!("image input has {} bytes, expected {expected}", bytes.len());
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("four bytes")))
        .collect())
}

fn finish_profile(session: &mut Session) -> Result<ProfileSummary> {
    let path = session.end_profiling()?;
    let content = fs::read_to_string(&path).with_context(|| format!("read ORT profile {path}"))?;
    let value: Value = serde_json::from_str(&content).context("parse ORT profile")?;
    let mut node_provider_counts = BTreeMap::new();
    collect_profile_providers(&value, &mut node_provider_counts);
    Ok(ProfileSummary {
        path,
        node_provider_counts,
    })
}

fn collect_profile_providers(value: &Value, providers: &mut BTreeMap<String, usize>) {
    match value {
        Value::Object(object) => {
            if let Some(provider) = object
                .get("args")
                .and_then(Value::as_object)
                .and_then(|args| args.get("provider"))
                .and_then(Value::as_str)
            {
                *providers.entry(provider.to_owned()).or_default() += 1;
            }
            for child in object.values() {
                collect_profile_providers(child, providers);
            }
        }
        Value::Array(array) => {
            for child in array {
                collect_profile_providers(child, providers);
            }
        }
        _ => {}
    }
}

fn dll_records(runtime: &Path) -> Result<Vec<Value>> {
    let mut records = Vec::new();
    for entry in fs::read_dir(runtime)? {
        let entry = entry?;
        let path = entry.path();
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("dll"))
        {
            continue;
        }
        records.push(json!({
            "name": entry.file_name().to_string_lossy(),
            "bytes": entry.metadata()?.len(),
        }));
    }
    records.sort_by_key(|record| record["name"].as_str().unwrap_or_default().to_owned());
    if records.is_empty() {
        bail!("no private runtime DLLs found in {}", runtime.display());
    }
    Ok(records)
}

fn set_dll_directory(runtime: &Path) -> Result<()> {
    let wide = runtime
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    if unsafe { SetDllDirectoryW(wide.as_ptr()) } == 0 {
        bail!(
            "SetDllDirectoryW({}) failed: {}",
            runtime.display(),
            std::io::Error::last_os_error()
        );
    }
    Ok(())
}

fn memory_usage() -> Result<MemoryUsage> {
    let mut counters = PROCESS_MEMORY_COUNTERS::default();
    counters.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    if unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) } == 0 {
        bail!(
            "GetProcessMemoryInfo failed: {}",
            std::io::Error::last_os_error()
        );
    }
    Ok(MemoryUsage {
        rss_bytes: counters.WorkingSetSize,
        peak_rss_bytes: counters.PeakWorkingSetSize,
    })
}

fn milliseconds(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

use std::os::windows::ffi::OsStrExt;
