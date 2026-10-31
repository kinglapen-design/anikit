use crate::core::models::AssetMeta;

use super::{
  control::AnalysisControl, provider_http::EvidenceImage, provider_media, provider_schema,
  provider_types::{ProviderKind, ProviderSettings, SemanticDocumentInput}, types::SegmentHit, worker,
};

const PROMPT_VERSION: &str = "semantic-v1";

pub struct ProviderAnalysis {
  pub documents: Vec<SemanticDocumentInput>,
  pub actual_backend: String,
}

pub(crate) struct ProviderAsset<'a> {
  pub meta: &'a AssetMeta,
  pub assets_dir: &'a std::path::Path,
  pub asset_id: &'a str,
  pub segments: &'a [SegmentHit],
  pub fingerprint: &'a str,
}

pub(crate) fn analyze_asset_with(
  asset: ProviderAsset<'_>,
  control: &AnalysisControl,
  settings: &ProviderSettings,
  analyze: impl FnOnce(&str, &[EvidenceImage]) -> Result<String, String>,
) -> Result<ProviderAnalysis, String> {
  settings.require_configured()?;
  let evidence = provider_media::collect(asset.meta, asset.assets_dir, asset.asset_id, asset.segments, asset.fingerprint, control)?;
  let prompt = provider_schema::prompt(&evidence.frames);
  control.check()?;
  let content = analyze(&prompt, &evidence.frames)?;
  control.check()?;
  let times = evidence.frames.iter().map(|frame| frame.timestamp_ms).collect::<Vec<_>>();
  let (description, events) = provider_schema::parse_response(&content, evidence.frames.len(), &times, evidence.duration_ms)?;
  let provider = match settings.kind { ProviderKind::OpenAiCompatible => "openai-compatible", ProviderKind::Ollama => "ollama" };
  let mut documents = Vec::new();
  let mut actual_backend = "unknown".to_string();
  if let (Some(first), Some(last)) = (evidence.frames.first(), evidence.frames.last()) {
    let (vector, backend) = embed(&description, control)?;
    actual_backend = backend;
    documents.push(SemanticDocumentInput {
      kind: "description".into(), text: description.clone(),
      start_ms: first.timestamp_ms, end_ms: last.timestamp_ms,
      start_frame: first.source_frame, end_frame: last.source_frame,
      certainty: "observed".into(), provider: provider.into(), model: settings.model.clone(),
      prompt_version: PROMPT_VERSION.into(), source_json: serde_json::json!({ "description": description }).to_string(),
      frame_ms: times.clone(), vector,
    });
  }
  for event in events {
    let text = if event.certainty == "hypothesis" { format!("候选因果（推测）：{}", event.text) } else { format!("连续事件（观察）：{}", event.text) };
    let (vector, backend) = embed(&text, control)?;
    actual_backend = backend;
    let (first, last) = (event.evidence.first().copied(), event.evidence.last().copied());
    documents.push(SemanticDocumentInput {
      kind: "event".into(), text, start_ms: event.start_ms, end_ms: event.end_ms,
      start_frame: first.and_then(|index| evidence.frames[index].source_frame),
      end_frame: last.and_then(|index| evidence.frames[index].source_frame),
      certainty: event.certainty.clone(), provider: provider.into(), model: settings.model.clone(),
      prompt_version: PROMPT_VERSION.into(), source_json: serde_json::to_string(&event).map_err(|e| e.to_string())?,
      frame_ms: event.evidence.iter().map(|index| evidence.frames[*index].timestamp_ms).collect(),
      vector,
    });
  }
  Ok(ProviderAnalysis { documents, actual_backend })
}

fn embed(text: &str, control: &AnalysisControl) -> Result<(Vec<f32>, String), String> {
  let response = worker::embed_text(text)?;
  control.check()?;
  Ok((response.embedding, response.actual_backend))
}
