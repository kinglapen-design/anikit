use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{Context, Result, anyhow, bail};
use image::{RgbImage, imageops::FilterType};
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

const CONTEXT_LENGTH: usize = 64;
const IMAGE_SIZE: usize = 512;
const EMBEDDING_DIM: usize = 768;
const TEXT_WARMED_RUNS: usize = 20;
const IMAGE_WARMED_RUNS: usize = 10;
const IMAGE_MEAN: [f32; 3] = [0.5, 0.5, 0.5];
const IMAGE_STD: [f32; 3] = [0.5, 0.5, 0.5];

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

    fn tolerance(self) -> f32 {
        match self {
            Self::Cpu => 0.0002,
            Self::DirectMl => 0.005,
        }
    }
}

struct Args {
    root: PathBuf,
    backend: Backend,
    eval: bool,
    manifest: Option<PathBuf>,
    queries: Option<PathBuf>,
    language: String,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut root = std::env::var_os("SPIKE_ROOT")
            .map(PathBuf::from)
            .unwrap_or(std::env::current_dir()?);
        let mut backend = Backend::Cpu;
        let mut eval = false;
        let mut manifest = None;
        let mut queries = None;
        let mut language = "zh".to_owned();
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--root" => {
                    root = PathBuf::from(args.next().context("--root requires a path")?);
                }
                "--backend" => {
                    backend = Backend::parse(&args.next().context("--backend requires a value")?)?;
                }
                "--eval" => eval = true,
                "--manifest" => {
                    manifest = Some(PathBuf::from(
                        args.next().context("--manifest requires a path")?,
                    ));
                    eval = true;
                }
                "--queries" => {
                    queries = Some(PathBuf::from(
                        args.next().context("--queries requires a path")?,
                    ));
                    eval = true;
                }
                "--language" => {
                    language = args.next().context("--language requires zh or en")?;
                    if language != "zh" && language != "en" {
                        bail!("--language must be zh or en");
                    }
                }
                "-h" | "--help" => {
                    println!(
                        "semantic-siglip-spike [--root PATH] [--backend cpu|directml] [--eval] [--manifest PATH] [--queries PATH] [--language zh|en]"
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
            eval,
            manifest,
            queries,
            language,
        })
    }

    fn manifest_path(&self) -> PathBuf {
        self.manifest.clone().unwrap_or_else(|| {
            self.root
                .parent()
                .expect("spike root has a parent")
                .join("semantic-eval/manifest/quality-manifest.json")
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
    logit_scale: f32,
    logit_bias: f32,
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

#[derive(Clone)]
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

#[derive(Serialize)]
struct TimingSummary {
    runs: usize,
    percentile_method: &'static str,
    p50_ms: f64,
    p95_ms: f64,
    min_ms: f64,
    max_ms: f64,
    mean_ms: f64,
    samples_ms: Vec<f64>,
}

struct PreprocessedImage {
    pixels: Vec<f32>,
    source_width: u32,
    source_height: u32,
    resized_width: u32,
    resized_height: u32,
    crop_left: u32,
    crop_top: u32,
}

#[derive(Deserialize)]
struct EvalManifest {
    schema_version: u32,
    source: EvalSource,
    trajectory_count: usize,
    image_count: usize,
    review_status: String,
    dishwashing_assessment: Value,
    trajectories: Vec<EvalTrajectory>,
}

#[derive(Deserialize, Serialize)]
struct EvalSource {
    dataset: String,
    license: String,
    attribution: String,
}

#[derive(Deserialize)]
struct EvalTrajectory {
    sample_id: String,
    category: String,
    annotations: Vec<String>,
    visual_review: EvalVisualReview,
    frames: Vec<EvalFrame>,
}

#[derive(Deserialize)]
struct EvalVisualReview {
    #[serde(default)]
    scene_tags: Vec<String>,
}

#[derive(Deserialize)]
struct EvalFrame {
    image_id: String,
    frame_label: String,
    frame_index: u32,
    path: String,
}

#[derive(Deserialize)]
struct QueryDocument {
    schema_version: u32,
    metric_definitions: Value,
    queries: Vec<EvalQuery>,
}

#[derive(Deserialize)]
struct EvalQuery {
    id: String,
    text_zh: String,
    text_en: String,
    annotation_basis: String,
    positive: EvalSelector,
    #[serde(default)]
    hard_negatives: BTreeMap<String, EvalSelector>,
    #[serde(default)]
    diagnostic_only: bool,
}

#[derive(Deserialize)]
struct EvalSelector {
    #[serde(default)]
    image_ids: Option<Vec<String>>,
    #[serde(default)]
    tags_any: Option<Vec<String>>,
    #[serde(default)]
    samples: Option<Vec<String>>,
    #[serde(default)]
    frames: Option<Vec<String>>,
    #[serde(default)]
    basis: Option<String>,
}

struct EvalImage {
    image_id: String,
    sample_id: String,
    category: String,
    frame_label: String,
    frame_index: u32,
    annotations: Vec<String>,
    embedding: Vec<f32>,
}

struct HardNegativeStat {
    best_rank: usize,
    margin: f32,
    positive_outscores: bool,
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
    let report_path = if args.eval {
        output.join(format!("quality-{}.json", args.backend.as_str()))
    } else {
        output.join(format!("rust-{}.json", args.backend.as_str()))
    };
    let mut report = json!({
        "status": "running",
        "mode": if args.eval { "quality" } else { "verification" },
        "requested_backend": args.backend.as_str(),
        "requested_execution_providers": args.backend.requested_providers(),
        "actual_backend": "not_run",
        "backend_verified": false,
        "ort_crate": "2.0.0-rc.15",
        "ort_api_version": ort::MINOR_VERSION,
        "runtime_directory": args.root.join("runtime"),
        "comparison_tolerance": args.backend.tolerance(),
    });
    let result = run(&args, &mut report);
    match result {
        Ok(()) => {
            report["status"] = json!("passed");
            report["backend_verified"] = json!(true);
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
    if report["status"] != "passed" {
        std::process::exit(1);
    }
}

fn run(args: &Args, report: &mut Value) -> Result<()> {
    let startup = Instant::now();
    let runtime = args.root.join("runtime");
    set_dll_directory(&runtime)?;
    report["private_runtime_dlls"] = json!(dll_records(&runtime)?);
    report["memory"] = json!({ "process_start": memory_usage()? });

    let start = Instant::now();
    let environment = ort::init_from(runtime.join("onnxruntime.dll"))
        .map_err(|error| anyhow!(error.to_string()))?
        .with_name("semantic-siglip-spike")
        .with_telemetry(false)
        .build()?;
    report["timings_ms"] = json!({ "environment_init": milliseconds(start) });
    report["ort_build_info"] = json!(ort::info());
    let directml_available = ep::DirectML::is_available()?;
    report["directml_available"] = json!(directml_available);
    if args.backend == Backend::DirectMl && !directml_available {
        bail!("DirectML was requested but is not available in the loaded runtime");
    }

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

    if args.eval {
        run_quality(
            args,
            &tokenizer,
            &mut text_session,
            &mut vision_session,
            report,
        )?;
    } else {
        run_verification(
            args,
            startup,
            &tokenizer,
            &mut text_session,
            &mut vision_session,
            report,
        )?;
    }

    let text_profile = finish_profile(&mut text_session)?;
    let vision_profile = finish_profile(&mut vision_session)?;
    verify_profile_providers(args.backend, "text", &text_profile)?;
    verify_profile_providers(args.backend, "vision", &vision_profile)?;
    let providers = text_profile
        .node_provider_counts
        .iter()
        .chain(&vision_profile.node_provider_counts)
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();
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
    report["memory"]["final"] = json!(memory_usage()?);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_verification(
    args: &Args,
    startup: Instant,
    tokenizer: &Tokenizer,
    text_session: &mut Session,
    vision_session: &mut Session,
    report: &mut Value,
) -> Result<()> {
    let reference: Reference = serde_json::from_slice(
        &fs::read(args.root.join("outputs/reference.json")).context("read Python reference")?,
    )?;
    let tolerance = args.backend.tolerance();
    let mut text_results = Vec::new();
    let mut unit_texts = BTreeMap::new();
    let mut text_warmed = None;
    for (index, item) in reference.texts.iter().enumerate() {
        let (ids, mask) = encode_text(tokenizer, &item.text)?;
        let ids_u32 = ids.iter().map(|value| *value as u32).collect::<Vec<_>>();
        if ids_u32 != item.token_ids {
            bail!("Rust token IDs differ from Python for {:?}", item.text);
        }
        let (id_tensor, mask_tensor) = text_tensors(&ids, &mask)?;
        let start = Instant::now();
        let embedding = infer_text(text_session, id_tensor, mask_tensor)?;
        let elapsed = milliseconds(start);
        if index == 0 {
            report["timings_ms"]["text_cold_inference"] = json!(elapsed);
            report["timings_ms"]["cold_start_to_first_text"] = json!(milliseconds(startup));
            let (warm_ids, warm_mask) = text_tensors(&ids, &mask)?;
            let warmup = infer_text(text_session, warm_ids, warm_mask)?;
            validate_unit(&warmup)?;
            let mut samples = Vec::with_capacity(TEXT_WARMED_RUNS);
            for _ in 0..TEXT_WARMED_RUNS {
                let (run_ids, run_mask) = text_tensors(&ids, &mask)?;
                let start = Instant::now();
                let measured = infer_text(text_session, run_ids, run_mask)?;
                samples.push(milliseconds(start));
                validate_unit(&measured)?;
            }
            text_warmed = Some(summarize(samples));
        } else {
            report["timings_ms"][format!("text_inference_{index}")] = json!(elapsed);
        }
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
    report["timings_ms"]["text_warmed"] = json!(text_warmed);
    report["memory"]["after_text_inference"] = json!(memory_usage()?);

    let preprocess_start = Instant::now();
    let native = preprocess_image(&args.root.join("assets/CLIP.png"))?;
    report["timings_ms"]["native_image_preprocess"] = json!(milliseconds(preprocess_start));
    let python_pixels = read_image_input(&args.root.join("outputs/image_input.f32"))?;
    let pixel_max_diff = max_abs_diff(&native.pixels, &python_pixels)?;
    let pixel_mean_diff = native
        .pixels
        .iter()
        .zip(&python_pixels)
        .map(|(left, right)| (left - right).abs() as f64)
        .sum::<f64>()
        / native.pixels.len() as f64;
    report["native_preprocessing"] = json!({
        "source_width": native.source_width,
        "source_height": native.source_height,
        "resized_width": native.resized_width,
        "resized_height": native.resized_height,
        "crop_left": native.crop_left,
        "crop_top": native.crop_top,
        "filter": "Triangle bilinear; direct resize to 512x512 without center crop; RGB / 255 then 0.5/0.5 normalization",
        "max_abs_diff_vs_python_tensor": pixel_max_diff,
        "mean_abs_diff_vs_python_tensor": pixel_mean_diff,
    });

    let start = Instant::now();
    let native_embedding = infer_vision(
        vision_session,
        Tensor::from_array(([1_usize, 3, IMAGE_SIZE, IMAGE_SIZE], native.pixels.clone()))?,
    )?;
    report["timings_ms"]["vision_cold_inference"] = json!(milliseconds(start));
    let native_comparison = compare_embedding(
        &native_embedding,
        &reference.image.unnormalized,
        &reference.image.normalized,
        0.05,
    )?;
    report["image_result"] = json!({
        "source": "OpenAI official CLIP.png decoded and preprocessed natively in Rust",
        "comparison_tolerance_including_preprocessing": 0.05,
        "comparison": native_comparison,
    });

    let python_embedding = infer_vision(
        vision_session,
        Tensor::from_array(([1_usize, 3, IMAGE_SIZE, IMAGE_SIZE], python_pixels))?,
    )?;
    let python_comparison = compare_embedding(
        &python_embedding,
        &reference.image.unnormalized,
        &reference.image.normalized,
        tolerance,
    )?;
    report["python_tensor_image_result"] = json!({ "comparison": python_comparison });
    let warmup = infer_vision(
        vision_session,
        Tensor::from_array(([1_usize, 3, IMAGE_SIZE, IMAGE_SIZE], native.pixels.clone()))?,
    )?;
    validate_unit(&warmup)?;
    let mut samples = Vec::with_capacity(IMAGE_WARMED_RUNS);
    for _ in 0..IMAGE_WARMED_RUNS {
        let tensor =
            Tensor::from_array(([1_usize, 3, IMAGE_SIZE, IMAGE_SIZE], native.pixels.clone()))?;
        let start = Instant::now();
        let measured = infer_vision(vision_session, tensor)?;
        samples.push(milliseconds(start));
        validate_unit(&measured)?;
    }
    report["timings_ms"]["vision_warmed"] = json!(summarize(samples));
    report["memory"]["after_vision_inference"] = json!(memory_usage()?);

    let mut similarities = BTreeMap::new();
    let mut similarity_results = Vec::new();
    for (text, unit_text) in &unit_texts {
        let similarity = dot(&native_embedding.unit, unit_text);
        let python_similarity = reference.cosine_similarities[text];
        if (similarity - python_similarity).abs() > 0.05 {
            bail!("cosine mismatch for {text:?}: Rust={similarity}, Python={python_similarity}");
        }
        similarities.insert(text.clone(), similarity);
        similarity_results.push(json!({
            "text": text,
            "rust_cosine": similarity,
            "python_cosine": python_similarity,
        }));
    }
    report["similarities"] = json!(similarity_results);
    let probabilities = sigmoid_probabilities(
        &reference.official_candidates,
        &similarities,
        reference.logit_scale,
        reference.logit_bias,
    )?;
    let probability_diff = probabilities
        .iter()
        .zip(&reference.official_candidate_probabilities)
        .fold(0.0_f32, |acc, (left, right)| acc.max((left - right).abs()));
    if probability_diff > 0.05 {
        bail!("official sample probability mismatch: {probability_diff}");
    }
    let best = reference.official_candidates[probabilities
        .iter()
        .enumerate()
        .max_by(|left, right| left.1.total_cmp(right.1))
        .context("empty probabilities")?
        .0]
        .clone();
    if best != "a diagram" {
        bail!("official sample best class is {best:?}, expected a diagram");
    }
    report["official_sample"] = json!({
        "candidates": reference.official_candidates,
        "probabilities": probabilities,
        "python_probabilities": reference.official_candidate_probabilities,
        "max_abs_probability_diff": probability_diff,
        "best": best,
        "logit_scale": reference.logit_scale,
        "logit_bias": reference.logit_bias,
    });
    Ok(())
}

fn run_quality(
    args: &Args,
    tokenizer: &Tokenizer,
    text_session: &mut Session,
    vision_session: &mut Session,
    report: &mut Value,
) -> Result<()> {
    let started = Instant::now();
    let manifest_path = args.manifest_path().canonicalize().with_context(|| {
        format!(
            "quality manifest not found: {}",
            args.manifest_path().display()
        )
    })?;
    let manifest: EvalManifest = serde_json::from_slice(
        &fs::read(&manifest_path).with_context(|| format!("read {}", manifest_path.display()))?,
    )?;
    if manifest.schema_version != 1 {
        bail!(
            "unsupported quality manifest schema {}",
            manifest.schema_version
        );
    }
    if !manifest.review_status.starts_with("reviewed_all") {
        bail!("quality manifest has not been visually reviewed");
    }
    if manifest.trajectories.len() != manifest.trajectory_count {
        bail!("trajectory_count does not match trajectories");
    }
    let eval_root = manifest_path
        .parent()
        .and_then(Path::parent)
        .context("manifest must be under semantic-eval/manifest")?;
    let queries_path = match &args.queries {
        Some(path) => path
            .canonicalize()
            .with_context(|| format!("queries not found: {}", path.display()))?,
        None => eval_root
            .join("manifest/queries.json")
            .canonicalize()
            .context("default queries.json not found")?,
    };
    let query_document: QueryDocument = serde_json::from_slice(
        &fs::read(&queries_path).with_context(|| format!("read {}", queries_path.display()))?,
    )?;
    if query_document.schema_version != 1 {
        bail!("unsupported query schema {}", query_document.schema_version);
    }

    let mut images = Vec::new();
    let mut image_ids = BTreeSet::new();
    let mut image_inference_ms = Vec::new();
    for trajectory in &manifest.trajectories {
        for frame in &trajectory.frames {
            if !image_ids.insert(frame.image_id.clone()) {
                bail!("duplicate image_id {}", frame.image_id);
            }
            let path = eval_root.join(&frame.path);
            let preprocessed = preprocess_image(&path)
                .with_context(|| format!("preprocess {} ({})", frame.image_id, path.display()))?;
            let tensor =
                Tensor::from_array(([1_usize, 3, IMAGE_SIZE, IMAGE_SIZE], preprocessed.pixels))?;
            let start = Instant::now();
            let embedding = infer_vision(vision_session, tensor)?;
            image_inference_ms.push(milliseconds(start));
            images.push(EvalImage {
                image_id: frame.image_id.clone(),
                sample_id: trajectory.sample_id.clone(),
                category: trajectory.category.clone(),
                frame_label: frame.frame_label.clone(),
                frame_index: frame.frame_index,
                annotations: trajectory.annotations.clone(),
                embedding: embedding.unit,
            });
        }
    }
    if images.len() != manifest.image_count {
        bail!(
            "image_count {} does not match {} frames",
            manifest.image_count,
            images.len()
        );
    }
    if images.is_empty() {
        bail!("quality manifest has no images");
    }
    let image_index_by_id = images
        .iter()
        .enumerate()
        .map(|(index, image)| (image.image_id.clone(), index))
        .collect::<BTreeMap<_, _>>();

    let mut rankings = Vec::new();
    let mut aggregate_queries = Vec::new();
    let mut diagnostic_queries = Vec::new();
    let mut top1 = 0_usize;
    let mut top5 = 0_usize;
    let mut top10 = 0_usize;
    let mut reciprocal_rank_sum = 0.0_f64;
    let mut text_inference_ms = Vec::new();
    let mut hard_negative_groups: BTreeMap<String, Vec<HardNegativeStat>> = BTreeMap::new();
    let mut typical_errors = Vec::new();
    for query in &query_document.queries {
        let text = if args.language == "zh" {
            &query.text_zh
        } else {
            &query.text_en
        };
        let (ids, mask) = encode_text(tokenizer, text)?;
        let (id_tensor, mask_tensor) = text_tensors(&ids, &mask)?;
        let start = Instant::now();
        let query_embedding = infer_text(text_session, id_tensor, mask_tensor)?;
        text_inference_ms.push(milliseconds(start));
        let scores = images
            .iter()
            .map(|image| dot(&query_embedding.unit, &image.embedding))
            .collect::<Vec<_>>();
        if !scores.iter().all(|value| value.is_finite()) {
            bail!("non-finite similarities for {}", query.id);
        }
        let mut ranked = (0..images.len())
            .map(|index| (index, scores[index]))
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .1
                .total_cmp(&left.1)
                .then_with(|| images[left.0].image_id.cmp(&images[right.0].image_id))
        });
        let mut ranks = vec![0_usize; images.len()];
        for (rank, (index, _)) in ranked.iter().enumerate() {
            ranks[*index] = rank + 1;
        }
        let positive_ids = resolve_selector(&query.positive, &manifest, &image_ids)?;
        let positive_set = positive_ids.iter().cloned().collect::<BTreeSet<_>>();
        if !query.diagnostic_only && positive_ids.is_empty() {
            bail!("non-diagnostic query has no positives: {}", query.id);
        }
        let best_positive_rank = positive_ids
            .iter()
            .map(|image_id| ranks[image_index_by_id[image_id]])
            .min();
        let best_positive_score = positive_ids
            .iter()
            .map(|image_id| scores[image_index_by_id[image_id]])
            .reduce(f32::max);
        let reciprocal_rank = best_positive_rank
            .map(|rank| 1.0 / rank as f64)
            .unwrap_or(0.0);
        let top = ranked
            .iter()
            .take(10)
            .enumerate()
            .map(|(rank, (index, score))| {
                let image = &images[*index];
                json!({
                    "rank": rank + 1,
                    "image_id": image.image_id,
                    "similarity": score,
                    "is_positive": positive_set.contains(&image.image_id),
                    "sample_id": image.sample_id,
                    "category": image.category,
                    "frame_label": image.frame_label,
                    "frame_index": image.frame_index,
                    "annotations": image.annotations,
                })
            })
            .collect::<Vec<_>>();

        let mut hard_negative_records = serde_json::Map::new();
        for (class_name, selector) in &query.hard_negatives {
            let negative_ids = resolve_selector(selector, &manifest, &image_ids)?;
            let negative_set = negative_ids.iter().cloned().collect::<BTreeSet<_>>();
            let overlap = positive_set.intersection(&negative_set).collect::<Vec<_>>();
            if !overlap.is_empty() {
                bail!(
                    "positive/hard-negative overlap for {}: {overlap:?}",
                    query.id
                );
            }
            if negative_ids.is_empty() {
                continue;
            }
            let positive_rank =
                best_positive_rank.context("hard-negative query has no positive")?;
            let positive_score =
                best_positive_score.context("hard-negative query has no positive")?;
            let best_negative_id = negative_ids
                .iter()
                .min_by_key(|image_id| ranks[image_index_by_id[*image_id]])
                .context("empty hard-negative set")?;
            let best_negative_index = image_index_by_id[best_negative_id];
            let best_negative_rank = ranks[best_negative_index];
            let best_negative_score = scores[best_negative_index];
            let margin = positive_score - best_negative_score;
            let positive_outscores = positive_score > best_negative_score;
            let count_before_first_positive = negative_ids
                .iter()
                .filter(|image_id| ranks[image_index_by_id[*image_id]] < positive_rank)
                .count();
            hard_negative_records.insert(
                class_name.clone(),
                json!({
                    "basis": selector.basis,
                    "image_ids": negative_ids,
                    "best_image_id": best_negative_id,
                    "best_rank": best_negative_rank,
                    "best_similarity": best_negative_score,
                    "margin_best_positive_minus_best_negative": margin,
                    "positive_outscores_negative": positive_outscores,
                    "count_before_first_positive": count_before_first_positive,
                }),
            );
            hard_negative_groups
                .entry(class_name.clone())
                .or_default()
                .push(HardNegativeStat {
                    best_rank: best_negative_rank,
                    margin,
                    positive_outscores,
                });
        }
        for (class_name, details) in &hard_negative_records {
            let positive_outscores = details["positive_outscores_negative"]
                .as_bool()
                .unwrap_or(false);
            let best_rank = details["best_rank"].as_u64().unwrap_or(usize::MAX as u64);
            if !positive_outscores || best_rank <= 5 {
                typical_errors.push(json!({
                    "query_id": query.id,
                    "type": class_name,
                    "best_positive_rank": best_positive_rank,
                    "details": details,
                }));
            }
        }
        if best_positive_rank != Some(1) {
            let items = top
                .iter()
                .filter(|item| !item["is_positive"].as_bool().unwrap_or(false))
                .take(3)
                .cloned()
                .collect::<Vec<_>>();
            if !items.is_empty() {
                typical_errors.push(json!({
                    "query_id": query.id,
                    "type": "top_ranked_nonpositives",
                    "best_positive_rank": best_positive_rank,
                    "items": items,
                }));
            }
        }
        let record = json!({
            "id": query.id,
            "text": text,
            "annotation_basis": query.annotation_basis,
            "diagnostic_only": query.diagnostic_only,
            "positive_image_ids": positive_ids,
            "positive_count": positive_ids.len(),
            "best_positive_rank": best_positive_rank,
            "reciprocal_rank": reciprocal_rank,
            "hits": {
                "1": best_positive_rank.is_some_and(|rank| rank <= 1),
                "5": best_positive_rank.is_some_and(|rank| rank <= 5),
                "10": best_positive_rank.is_some_and(|rank| rank <= 10),
            },
            "top10": top,
            "hard_negatives": hard_negative_records,
        });
        if query.diagnostic_only {
            diagnostic_queries.push(record.clone());
        } else {
            top1 += usize::from(best_positive_rank.is_some_and(|rank| rank <= 1));
            top5 += usize::from(best_positive_rank.is_some_and(|rank| rank <= 5));
            top10 += usize::from(best_positive_rank.is_some_and(|rank| rank <= 10));
            reciprocal_rank_sum += reciprocal_rank;
            aggregate_queries.push(record.clone());
        }
        rankings.push(record);
    }
    let query_count = aggregate_queries.len();
    if query_count == 0 {
        bail!("query document has no non-diagnostic queries");
    }
    let divisor = query_count as f64;
    let mut hard_negative_aggregate = serde_json::Map::new();
    for (name, records) in &hard_negative_groups {
        let count = records.len();
        let mean_margin = records.iter().map(|record| record.margin).sum::<f32>() / count as f32;
        let min_margin = records
            .iter()
            .map(|record| record.margin)
            .fold(f32::INFINITY, f32::min);
        let max_margin = records
            .iter()
            .map(|record| record.margin)
            .fold(f32::NEG_INFINITY, f32::max);
        hard_negative_aggregate.insert(
            name.clone(),
            json!({
                "query_count": count,
                "positive_outscore_rate": records.iter().filter(|record| record.positive_outscores).count() as f64 / count as f64,
                "hard_negative_top1_rate": records.iter().filter(|record| record.best_rank == 1).count() as f64 / count as f64,
                "mean_margin": mean_margin,
                "min_margin": min_margin,
                "max_margin": max_margin,
            }),
        );
    }
    report["quality"] = json!({
        "schema_version": 1,
        "language": args.language,
        "manifest": manifest_path,
        "queries": queries_path,
        "source": manifest.source,
        "review_status": manifest.review_status,
        "dishwashing_assessment": manifest.dishwashing_assessment,
        "metric_definitions": query_document.metric_definitions,
        "dataset": {
            "trajectory_count": manifest.trajectory_count,
            "image_count": images.len(),
            "regular_query_count": query_count,
            "diagnostic_query_count": diagnostic_queries.len(),
        },
        "aggregate": {
            "top1_hit_rate": top1 as f64 / divisor,
            "top5_hit_rate": top5 as f64 / divisor,
            "top10_hit_rate": top10 as f64 / divisor,
            "mrr": reciprocal_rank_sum / divisor,
            "hard_negatives": hard_negative_aggregate,
        },
        "per_query": aggregate_queries,
        "diagnostic_queries": diagnostic_queries,
        "rankings": rankings,
        "typical_errors": typical_errors,
    });
    report["timings_ms"]["quality_total"] = json!(milliseconds(started));
    report["timings_ms"]["quality_image_inference"] = json!(summarize(image_inference_ms));
    report["timings_ms"]["quality_text_inference"] = json!(summarize(text_inference_ms));
    Ok(())
}

fn resolve_selector(
    selector: &EvalSelector,
    manifest: &EvalManifest,
    all_image_ids: &BTreeSet<String>,
) -> Result<Vec<String>> {
    if let Some(image_ids) = &selector.image_ids {
        let unknown = image_ids
            .iter()
            .filter(|image_id| !all_image_ids.contains(*image_id))
            .collect::<Vec<_>>();
        if !unknown.is_empty() {
            bail!("unknown image IDs in selector: {unknown:?}");
        }
        let unique = image_ids.iter().collect::<BTreeSet<_>>();
        if unique.len() != image_ids.len() {
            bail!("duplicate image IDs in selector");
        }
        return Ok(image_ids.clone());
    }
    let tags = selector
        .tags_any
        .as_ref()
        .map(|values| values.iter().collect::<BTreeSet<_>>());
    let samples = selector
        .samples
        .as_ref()
        .map(|values| values.iter().collect::<BTreeSet<_>>());
    let default_frames = vec!["first".to_owned(), "mid".to_owned(), "last".to_owned()];
    let selected_frames = selector.frames.as_ref().unwrap_or(&default_frames);
    let selected_frames = selected_frames.iter().collect::<BTreeSet<_>>();
    let mut result = Vec::new();
    for trajectory in &manifest.trajectories {
        let selected = if let Some(tags) = &tags {
            trajectory
                .visual_review
                .scene_tags
                .iter()
                .any(|tag| tags.contains(tag))
        } else if let Some(samples) = &samples {
            samples.contains(&trajectory.sample_id)
        } else {
            false
        };
        if !selected {
            continue;
        }
        for frame in &trajectory.frames {
            if selected_frames.contains(&frame.frame_label) {
                if !all_image_ids.contains(&frame.image_id) {
                    bail!("selector produced unknown image ID {}", frame.image_id);
                }
                result.push(frame.image_id.clone());
            }
        }
    }
    Ok(result)
}

fn create_session(
    environment: &Environment,
    args: &Args,
    kind: &str,
    model: &Path,
    report: &mut Value,
) -> Result<Session> {
    let profile = args.root.join("outputs").join(format!(
        "{}-{}-{kind}-profile.json",
        if args.eval { "quality" } else { "rust" },
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

fn encode_text(tokenizer: &Tokenizer, text: &str) -> Result<(Vec<i64>, Vec<i64>)> {
    let encoding = tokenizer
        .encode(text, true)
        .map_err(|error| anyhow!(error.to_string()))?;
    if encoding.get_ids().len() != CONTEXT_LENGTH {
        bail!(
            "expected {CONTEXT_LENGTH} token IDs, got {}",
            encoding.get_ids().len()
        );
    }
    let ids = encoding
        .get_ids()
        .iter()
        .map(|value| i64::from(*value))
        .collect::<Vec<_>>();
    let mask = encoding
        .get_attention_mask()
        .iter()
        .map(|value| i64::from(*value))
        .collect::<Vec<_>>();
    if mask.len() != CONTEXT_LENGTH {
        bail!(
            "expected {CONTEXT_LENGTH} attention values, got {}",
            mask.len()
        );
    }
    Ok((ids, mask))
}

fn text_tensors(ids: &[i64], mask: &[i64]) -> Result<(Tensor<i64>, Tensor<i64>)> {
    Ok((
        Tensor::from_array(([1_usize, CONTEXT_LENGTH], ids.to_vec()))?,
        Tensor::from_array(([1_usize, CONTEXT_LENGTH], mask.to_vec()))?,
    ))
}

fn infer_text(session: &mut Session, ids: Tensor<i64>, _mask: Tensor<i64>) -> Result<Embedding> {
    let outputs = session.run(ort::inputs![ids])?;
    let (raw_shape, raw) = {
        let (shape, data) = outputs["unnorm_text_features"].try_extract_tensor::<f32>()?;
        (shape.iter().copied().collect::<Vec<_>>(), data.to_vec())
    };
    let (unit_shape, unit) = {
        let (shape, data) = outputs["text_features"].try_extract_tensor::<f32>()?;
        (shape.iter().copied().collect::<Vec<_>>(), data.to_vec())
    };
    make_embedding(raw_shape, raw, unit_shape, unit)
}

fn infer_vision(session: &mut Session, pixels: Tensor<f32>) -> Result<Embedding> {
    let outputs = session.run(ort::inputs![pixels])?;
    let (raw_shape, raw) = {
        let (shape, data) = outputs["unnorm_image_features"].try_extract_tensor::<f32>()?;
        (shape.iter().copied().collect::<Vec<_>>(), data.to_vec())
    };
    let (unit_shape, unit) = {
        let (shape, data) = outputs["image_features"].try_extract_tensor::<f32>()?;
        (shape.iter().copied().collect::<Vec<_>>(), data.to_vec())
    };
    make_embedding(raw_shape, raw, unit_shape, unit)
}

fn make_embedding(
    raw_shape: Vec<i64>,
    raw: Vec<f32>,
    unit_shape: Vec<i64>,
    unit: Vec<f32>,
) -> Result<Embedding> {
    let expected = [1, EMBEDDING_DIM as i64];
    if raw_shape != expected || unit_shape != expected {
        bail!("unexpected embedding shapes: {raw_shape:?}, {unit_shape:?}");
    }
    if raw.len() != EMBEDDING_DIM || unit.len() != EMBEDDING_DIM {
        bail!(
            "unexpected embedding lengths: {}, {}",
            raw.len(),
            unit.len()
        );
    }
    let embedding = Embedding {
        raw,
        unit,
        shape: unit_shape,
    };
    validate_unit(&embedding)?;
    Ok(embedding)
}

fn validate_unit(embedding: &Embedding) -> Result<f32> {
    if !embedding.raw.iter().all(|value| value.is_finite())
        || !embedding.unit.iter().all(|value| value.is_finite())
    {
        bail!("embedding contains NaN or infinity");
    }
    let norm = embedding
        .unit
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    if !norm.is_finite() || (norm - 1.0).abs() > 0.00001 {
        bail!("normalized embedding L2 is {norm}");
    }
    Ok(norm)
}

fn compare_embedding(
    embedding: &Embedding,
    expected_raw: &[f32],
    expected_unit: &[f32],
    tolerance: f32,
) -> Result<VectorComparison> {
    let normalized_l2 = validate_unit(embedding)?;
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
    Ok(VectorComparison {
        shape: embedding.shape.clone(),
        finite: true,
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

fn sigmoid_probabilities(
    candidates: &[String],
    similarities: &BTreeMap<String, f32>,
    logit_scale: f32,
    logit_bias: f32,
) -> Result<Vec<f32>> {
    candidates
        .iter()
        .map(|text| {
            let cosine = similarities
                .get(text)
                .copied()
                .context("missing similarity")?;
            let logit = logit_scale * cosine + logit_bias;
            Ok(1.0 / (1.0 + (-logit).exp()))
        })
        .collect()
}

fn preprocess_image(path: &Path) -> Result<PreprocessedImage> {
    let source = image::open(path)
        .with_context(|| format!("decode image {}", path.display()))?
        .to_rgb8();
    let (source_width, source_height) = (source.width(), source.height());
    if source_width == 0 || source_height == 0 {
        bail!("empty image {}", path.display());
    }
    let resized = image::imageops::resize(
        &source,
        IMAGE_SIZE as u32,
        IMAGE_SIZE as u32,
        FilterType::Triangle,
    );
    Ok(PreprocessedImage {
        pixels: normalize_image(&resized),
        source_width,
        source_height,
        resized_width: IMAGE_SIZE as u32,
        resized_height: IMAGE_SIZE as u32,
        crop_left: 0,
        crop_top: 0,
    })
}

fn normalize_image(image: &RgbImage) -> Vec<f32> {
    let plane = IMAGE_SIZE * IMAGE_SIZE;
    let mut output = vec![0.0_f32; 3 * plane];
    for (x, y, pixel) in image.enumerate_pixels() {
        let offset = y as usize * IMAGE_SIZE + x as usize;
        for channel in 0..3 {
            output[channel * plane + offset] =
                (f32::from(pixel[channel]) / 255.0 - IMAGE_MEAN[channel]) / IMAGE_STD[channel];
        }
    }
    output
}

fn read_image_input(path: &Path) -> Result<Vec<f32>> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let expected = 3 * IMAGE_SIZE * IMAGE_SIZE * size_of::<f32>();
    if bytes.len() != expected {
        bail!("image input has {} bytes, expected {expected}", bytes.len());
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("four bytes")))
        .collect())
}

fn summarize(mut samples: Vec<f64>) -> TimingSummary {
    assert!(!samples.is_empty());
    let mean_ms = samples.iter().sum::<f64>() / samples.len() as f64;
    samples.sort_by(f64::total_cmp);
    let runs = samples.len();
    TimingSummary {
        runs,
        percentile_method: "nearest-rank",
        p50_ms: percentile(&samples, 0.50),
        p95_ms: percentile(&samples, 0.95),
        min_ms: samples[0],
        max_ms: samples[runs - 1],
        mean_ms,
        samples_ms: samples,
    }
}

fn percentile(sorted: &[f64], percentile: f64) -> f64 {
    let rank = (percentile * sorted.len() as f64).ceil() as usize;
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

fn verify_profile_providers(backend: Backend, kind: &str, profile: &ProfileSummary) -> Result<()> {
    let providers = profile
        .node_provider_counts
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let has_dml = providers
        .iter()
        .any(|name| name.to_ascii_lowercase().contains("dml"));
    let has_cpu = providers
        .iter()
        .any(|name| name.to_ascii_lowercase().contains("cpu"));
    match backend {
        Backend::DirectMl if !has_dml => bail!(
            "DirectML was requested, but {kind} ORT profiling shows no DML nodes: {providers:?}"
        ),
        Backend::Cpu if has_dml || !has_cpu => {
            bail!("CPU-only {kind} session has unexpected profiled providers: {providers:?}")
        }
        _ => Ok(()),
    }
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
