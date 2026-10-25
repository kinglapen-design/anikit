#![allow(dead_code)] // V1～V5 分批接线的公开契约；语义搜索 / 分析任务 / MCP 接入后移除

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
  pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSnapshot {
  pub id: String,
  pub library_root: String,
  pub assets_dir: String,
  pub device: String,
  pub status: String,
  pub total: i64,
  pub completed: i64,
}
