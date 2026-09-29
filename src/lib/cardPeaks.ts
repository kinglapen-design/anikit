// 音频卡片波形 peaks 懒加载（2026-09-22 R6 卡片三变体接线 · audio 变体，稿 = 文件卡片-音频）：
// 音频卡内容区 = 竖线波形（WaveContent），peaks = 后端 mono PCM 分桶 max-abs 归一化
//（get_audio_peaks，恒定 128 根不随时长——稿 prose「根数一致、美观第一位」；产物
// cache/peaks/{assetId}.json 跨会话缓存）。虚拟滚动只渲染可见卡 → 调用即视口懒加载；
// 会话级缓存 key = assetId，附带 modifiedAt 版本——源文件替换等任何 modifiedAt 变化后
// 自动重拉（免手动失效登记，同 cardText 口径）；inflight 去重防同卡并发。
// 工作空间分支（2026-09-26 填充卡对接）：条目带 wsPath 时走 get_audio_peaks_by_path
//（产物缓存换锚 workspace/.anikit/peaks/{urlsafe(rel)}-{mtime_ms}-{size}.json，文件名内嵌
// 实况判新鲜）——assetId = "ws:"+rel 合成假 id 严禁发后端，仅作前端缓存键（纯前端键允许）。
import { invoke } from "@tauri-apps/api/core";
import type { PlacementView } from "../stores/content";

interface CacheEntry {
  modifiedAt: string | null;
  peaks: number[];
}

const cache = new Map<string, CacheEntry>();
const inflight = new Map<string, Promise<number[]>>();

/** 懒拉卡片波形 peaks（缓存命中且版本一致直给；失败回退空数组并缓存——空波形 = 纯色底，
 *  modifiedAt 变化后自然重试；非音频 / 解码失败由后端中文报错，此处静默兜底） */
export function cardPeaksFor(asset: Pick<PlacementView, "assetId" | "modifiedAt" | "wsPath">): Promise<number[]> {
  const id = asset.assetId;
  const hit = cache.get(id);
  if (hit && hit.modifiedAt === asset.modifiedAt) return Promise.resolve(hit.peaks);
  const flying = inflight.get(id);
  if (flying) return flying;
  const request = asset.wsPath
    ? invoke<number[]>("get_audio_peaks_by_path", { absPath: asset.wsPath })
    : invoke<number[]>("get_audio_peaks", { assetId: id });
  const p = request
    .then((peaks) => {
      cache.set(id, { modifiedAt: asset.modifiedAt, peaks });
      return peaks;
    })
    .catch(() => {
      cache.set(id, { modifiedAt: asset.modifiedAt, peaks: [] });
      return [] as number[];
    })
    .finally(() => inflight.delete(id));
  inflight.set(id, p);
  return p;
}
