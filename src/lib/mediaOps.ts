// 媒体解析命令封装（1.3 重构，规范第 1 条）：get_asset_file_path / get_media_source /
// list_sequence_frames / prepare_anim_strips 命令统一出口——store 与组件一律走本层，不再裸 invoke。
// 类型以后端序列化结构为唯一来源（src-tauri/src/commands/preview.rs、media.rs；
// serde camelCase，Option ↔ TS null）。本层不吞错：失败原样抛出，各调用点按原有 try/catch 处理。
import { invoke } from "@tauri-apps/api/core";

// get_asset_file_path 返回：path = webview 可渲染文件绝对路径（直渲格式 = 源路径；
// psd/psb = 已缓存的解码预览；暂不可得为 null）；pending = 高清层后台解析中（完成经 preview-full-ready 推送）；
// animPreview = 动图小动画播放缓存（2.3 恢复：info/{stem}_anim.webp 短边 320 animated webp），
// 卡片播放优先于 path；未生成时为 null（已提交后台懒生成，完成经 preview-anim-ready 热切换）
export interface AssetFilePath {
  path: string | null;
  pending: boolean;
  animPreview: string | null;
}

// get_media_source 返回：transcoded = 返回的是后端转码产物（mp4/wav，false = 源文件）；
// fps = 素材自身帧率（视频，自愈后的准确值随响应下发；音频 / 未知为 null）
export interface MediaSource {
  path: string;
  transcoded: boolean;
  fps: number | null;
}

// 单帧条目：fullPath = 帧原图绝对路径；stripPath = 帧条小图（null = 后台生成中，
// 批次完成经 preview-strips-ready 渐进填充）
export interface FrameEntry {
  name: string;
  fullPath: string;
  stripPath: string | null;
}

// list_sequence_frames 返回（序列帧专用；动图不走此命令——2026-08-20 解包废除，
// 播放/帧条/设封面对 origin 运行时解码）
export interface SequenceFrames {
  fps: number | null;
  thumbFrame: string | null;
  frames: FrameEntry[];
}

// prepare_anim_strips 返回的单条（name = `f:{索引0起}`，与 preview-strips-ready 事件 items 同构）
export interface AnimStripItem {
  name: string;
  stripPath: string;
}

// 预览文件路径解析：webview 可直渲格式给源路径；psd/psb 缓存命中直给、未命中进解析队列（pending=true）
export async function getAssetFilePath(assetId: string): Promise<AssetFilePath> {
  return await invoke<AssetFilePath>("get_asset_file_path", { assetId });
}

// 播放源解析：可直播格式直给源路径；其余后端转码兜底（transcoded=true，转码可能数十秒）。
// in-flight 去重（2026-09-13 视频多选/排序卡死修复）：悬停层 / 预览 / 链接面板并发请求同一资产时
// 共享同一趟后端解析（探测/转码），settle 后即删——不长期缓存结果，"设为原始文件"换源后不重吃旧路径
const mediaSourceInflight = new Map<string, Promise<MediaSource>>();
export async function getMediaSource(assetId: string): Promise<MediaSource> {
  let p = mediaSourceInflight.get(assetId);
  if (!p) {
    p = invoke<MediaSource>("get_media_source", { assetId });
    mediaSourceInflight.set(assetId, p);
    p.finally(() => {
      if (mediaSourceInflight.get(assetId) === p) mediaSourceInflight.delete(assetId);
    });
  }
  return await p;
}

// 序列帧帧清单：立即返回（帧条小图后台渐进）
export async function listSequenceFrames(assetId: string): Promise<SequenceFrames> {
  return await invoke<SequenceFrames>("list_sequence_frames", { assetId });
}

// 动图帧条：立即返回已缓存清单（cache/frames/{id}/f-*.webp），后台队列单趟流式解码补齐
// 未缓存帧（批次经 preview-strips-ready 渐进推送）
export async function prepareAnimStrips(assetId: string): Promise<{ items: AnimStripItem[] }> {
  return await invoke<{ items: AnimStripItem[] }>("prepare_anim_strips", { assetId });
}
