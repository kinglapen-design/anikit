// 媒体解析命令封装（1.3 重构，规范第 1 条）：get_asset_file_path / get_media_source /
// list_sequence_frames 三命令统一出口——store 与组件一律走本层，不再裸 invoke。
// 类型以后端序列化结构为唯一来源（src-tauri/src/commands/preview.rs、media.rs；
// serde camelCase，Option ↔ TS null）。本层不吞错：失败原样抛出，各调用点按原有 try/catch 处理。
import { invoke } from "@tauri-apps/api/core";

// get_asset_file_path 返回：path = webview 可渲染文件绝对路径（直渲格式 = 源路径；
// psd/psb = 已缓存的解码预览；暂不可得为 null）；pending = 高清层后台解析中（完成经 preview-full-ready 推送）；
// animPreview = 动图小动画播放缓存（info 内短边 320 animated webp，2.3）——非动图恒 null；
// 动图缓存缺失时为 null 并由后端懒生成（本次回退 path 源文件播放，下次命中）
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
// 批次完成经 preview-strips-ready 渐进填充）；delayMs = 该帧显示时长 ms
// （动图从 frames.json 逐帧下发；序列帧无 frames.json 恒 null，前端按 1000/fps 预填）
export interface FrameEntry {
  name: string;
  fullPath: string;
  stripPath: string | null;
  delayMs: number | null;
}

// list_sequence_frames 返回（5.2：序列帧 / 动图统一入口）：
// 序列帧 = fps 非 null + delayMs 恒 null + unpacking 恒 false；
// 动图已解包 = fps 恒 null + 逐帧 delayMs；unpacking = 动图后台解包中（frames 空，
// 完成经 anim-unpacked 事件通知后重调）
export interface SequenceFrames {
  fps: number | null;
  thumbFrame: string | null;
  frames: FrameEntry[];
  unpacking: boolean;
}

// 预览文件路径解析：webview 可直渲格式给源路径；psd/psb 缓存命中直给、未命中进解析队列（pending=true）
export async function getAssetFilePath(assetId: string): Promise<AssetFilePath> {
  return await invoke<AssetFilePath>("get_asset_file_path", { assetId });
}

// 播放源解析：可直播格式直给源路径；其余后端转码兜底（transcoded=true，转码可能数十秒）
export async function getMediaSource(assetId: string): Promise<MediaSource> {
  return await invoke<MediaSource>("get_media_source", { assetId });
}

// 序列帧 / 动图帧清单：立即返回（帧条小图后台渐进）；动图未解包返回空清单 + unpacking=true
export async function listSequenceFrames(assetId: string): Promise<SequenceFrames> {
  return await invoke<SequenceFrames>("list_sequence_frames", { assetId });
}
