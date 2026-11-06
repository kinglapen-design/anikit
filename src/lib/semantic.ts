import { invoke } from "@tauri-apps/api/core";
import type { SearchFilters } from "../stores/search";

export type SemanticMode = "auto" | "keyword" | "semantic";

export interface SegmentHit {
  segmentId: number;
  assetId: string;
  startMs: number;
  endMs: number;
  startFrame: number | null;
  endFrame: number | null;
  score: number;
  kind: string;
  certainty?: "observed" | "hypothesis";
  text: string;
}

export interface SegmentRange {
  startMs: number;
  endMs: number;
  startFrame: number | null;
  endFrame: number | null;
}

export function segmentFrameCount(range: SegmentRange | null | undefined): number {
  return range?.startFrame != null && range.endFrame != null ? range.endFrame - range.startFrame + 1 : 0;
}

export function createSemanticSegment(assetId: string, segmentId: number, physical: boolean, targetFolderId?: string): Promise<import("../stores/content").PlacementView> {
  return invoke("create_semantic_segment", { request: { assetId, segmentId, physical, targetFolderId } });
}

export interface SemanticAssetHit {
  placements: { assetId: string; folderId: string }[];
  best: SegmentHit;
  segments: SegmentHit[];
  matchCount: number;
}

export interface SemanticSearchHits {
  hits: SemanticAssetHit[];
  actualBackend: string;
  modelRevision: string;
}

/** 定位段选择：segments 已由后端按分通道融合相关度排序，跳过覆盖全片、无定位价值的
 * description 后取首个窄段；2026-11-07 起不再固定优先 event，避免绕过通道融合。 */
export function pickLocateSegment(hit: SemanticAssetHit): SegmentHit {
  return hit.segments.find((segment) => segment.kind !== "description") ?? hit.best;
}

/** 语义命中帧缩略图：复用笔记抽帧命令（幂等缓存）——视频 `t:{秒}` / 动图 `f:{帧索引}` /
 * 序列帧帧名直给；取不到返回 null（前端保持原封面，不打扰） */
export async function getSemanticHitThumb(assetId: string, key: string): Promise<string | null> {
  const r = await invoke<{ path: string | null }>("get_note_thumb", { assetId, key });
  return r.path;
}

export interface SemanticScope {
  viewKey: string;
  recursive: boolean;
  global: boolean;
  filters: SearchFilters;
}

export interface SemanticSettings {
  device: "cpu" | "gpu";
  modelDir: string | null;
}

export interface SemanticRuntimeStatus {
  requestedBackend: string;
  actualBackend: string;
  modelDir: string;
  runtimeDir: string;
  dimension: number;
}

export interface AnalysisProgress {
  taskId: string;
  status: string;
  currentAssetId: string | null;
  completed: number;
  total: number;
  actualBackend: string;
  error: string | null;
}

export interface AnalysisTask {
  id: string;
  libraryRoot: string;
  assetsDir: string;
  device: string;
  mode: "basic" | "detailed";
  actualBackend: string;
  status: string;
  total: number;
  completed: number;
  errors: { assetId: string; error: string }[];
}

/** 单资产分析状态（详情面板「语义分析」行）：analyzable = 可否参与分析（视频/动图/序列帧）；
 * segmentCount = 当前模型版本有效分段数；detailedCount = 详细分析文档数（description/event） */
export interface SemanticAssetStatus {
  analyzable: boolean;
  segmentCount: number;
  detailedCount: number;
}

export type ProviderKind = "openai-compatible" | "ollama";

export interface ProviderSettings {
  kind: ProviderKind;
  endpoint: string;
  model: string;
  timeoutSecs: number;
}

export interface ProviderConfigView {
  settings: ProviderSettings;
  hasKey: boolean;
}

export interface ProviderConnection {
  ok: boolean;
  message: string;
  latencyMs: number;
}

export function semanticSearch(query: string, scope: SemanticScope, limit = 30): Promise<SemanticSearchHits> {
  return invoke("semantic_search", { query, scope, limit });
}

export function startSemanticAnalysis(assetIds?: string[], folderId?: string, recursive = false): Promise<string> {
  return invoke("start_semantic_analysis", { request: { assetIds, folderId, recursive } });
}

export function startDetailedSemanticAnalysis(assetIds: string[]): Promise<string> {
  return invoke("start_detailed_semantic_analysis", { assetIds });
}

export function getSemanticSettings(): Promise<SemanticSettings> {
  return invoke("get_semantic_settings");
}

export function saveSemanticSettings(settings: SemanticSettings): Promise<void> {
  return invoke("save_semantic_settings", { settings });
}

export function testSemanticRuntime(): Promise<SemanticRuntimeStatus> {
  return invoke("test_semantic_runtime");
}

export function getSemanticAnalysisTask(taskId: string): Promise<AnalysisTask | null> {
  return invoke("get_semantic_analysis_task", { taskId });
}

export function getSemanticAssetStatus(assetId: string): Promise<SemanticAssetStatus> {
  return invoke("get_semantic_asset_status", { assetId });
}

export function listSemanticAnalysisTasks(): Promise<AnalysisTask[]> {
  return invoke("list_semantic_analysis_tasks");
}

export function pauseSemanticAnalysis(taskId: string): Promise<void> {
  return invoke("pause_semantic_analysis", { taskId });
}

export function resumeSemanticAnalysis(taskId: string): Promise<string> {
  return invoke("resume_semantic_analysis", { taskId });
}

export function cancelSemanticAnalysis(taskId: string): Promise<void> {
  return invoke("cancel_semantic_analysis", { taskId });
}

export function retrySemanticAnalysis(taskId: string): Promise<string> {
  return invoke("retry_semantic_analysis", { taskId });
}

export function getSemanticProviderSettings(): Promise<ProviderConfigView> {
  return invoke("get_semantic_provider_settings");
}

export function saveSemanticProviderSettings(settings: ProviderSettings): Promise<void> {
  return invoke("save_semantic_provider_settings", { settings });
}

export function setSemanticProviderKey(key: string): Promise<void> {
  return invoke("set_semantic_provider_key", { key });
}

export function clearSemanticProviderKey(): Promise<void> {
  return invoke("clear_semantic_provider_key");
}

export function testSemanticProvider(): Promise<ProviderConnection> {
  return invoke("test_semantic_provider");
}
