use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRevision {
  pub id: String,
  pub revision: String,
  pub dimension: usize,
  pub weight_sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SegmentInput {
  pub start_ms: i64,
  pub end_ms: i64,
  pub start_frame: Option<u32>,
  pub end_frame: Option<u32>,
  pub kind: String,
  pub text: String,
  pub vector: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentHit {
  pub segment_id: i64,
  pub asset_id: String,
  pub start_ms: i64,
  pub end_ms: i64,
  pub start_frame: Option<u32>,
  pub end_frame: Option<u32>,
  pub score: f32,
  pub kind: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub certainty: Option<String>,
  pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentRange {
  pub start_ms: i64,
  pub end_ms: i64,
  pub start_frame: Option<u32>,
  pub end_frame: Option<u32>,
}

impl SegmentRange {
  pub fn from_hit(hit: &SegmentHit) -> Result<Self, String> {
    let range = Self { start_ms: hit.start_ms, end_ms: hit.end_ms, start_frame: hit.start_frame, end_frame: hit.end_frame };
    range.validate()?;
    Ok(range)
  }

  pub fn validate(&self) -> Result<(), String> {
    if self.start_ms < 0 || self.end_ms < self.start_ms {
      return Err("片段时间区间无效".into());
    }
    match (self.start_frame, self.end_frame) {
      (Some(start), Some(end)) if start <= end => Ok(()),
      (None, None) => Ok(()),
      (Some(_), Some(_)) => Err("片段原始帧区间无效".into()),
      _ => Err("片段原始帧索引不完整".into()),
    }
  }

  pub fn frame_bounds(&self) -> Result<(u32, u32), String> {
    self.start_frame.zip(self.end_frame).ok_or_else(|| "该片段缺少原始帧索引，无法定位".into())
  }

  pub fn frame_count(&self) -> u32 {
    self.end_frame.unwrap_or(0).saturating_sub(self.start_frame.unwrap_or(0)).saturating_add(1)
  }

  pub fn duration_secs(&self) -> f64 {
    (self.end_ms - self.start_ms) as f64 / 1000.0
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskError {
  pub asset_id: String,
  pub error: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSnapshot {
  pub id: String,
  pub library_root: String,
  pub assets_dir: String,
  pub device: String,
  pub mode: String,
  pub actual_backend: String,
  pub status: String,
  pub total: i64,
  pub completed: i64,
  pub errors: Vec<TaskError>,
}

/// 单资产分析状态（详情面板「语义分析」行）：analyzable = 可否参与分析（视频/动图/序列帧/png 动图同
/// analysis::supported 口径）；segment_count = 当前模型版本有效分段数；detailed_count = 详细分析文档数
///（description/event）。两者全容错——模型未配置或索引不可读按 0 返回
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticAssetStatus {
  pub analyzable: bool,
  pub segment_count: i64,
  pub detailed_count: i64,
}
