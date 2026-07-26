// 高清波形按需重采样（2026-07-26 波形清晰度修复，对齐 Eagle）：观察元素实际尺寸
// （CSS px × devicePixelRatio），防抖 250ms 调 get_waveform 由后端按该像素重渲 PCM 波形，
// 任意窗口宽度 1px 级清晰。返回产物绝对路径；未就绪 / 生成失败（组件缺失 / 提取失败）返回
// null，调用方回退 info 波形缩略图拉伸（旧行为）。仅预览音频素材时发起请求。
import { onBeforeUnmount, ref, watch, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { usePreviewStore } from "../stores/preview";

export function useHiresWaveform(elRef: Ref<HTMLElement | undefined>): Ref<string | null> {
  const preview = usePreviewStore();
  const path = ref<string | null>(null);
  let timer = 0;
  let token = 0; // 在途请求令牌：切素材 / 元素卸载后在途结果作废

  async function request(): Promise<void> {
    const el = elRef.value;
    const assetId = preview.current?.assetId;
    if (!el || !assetId || preview.kind !== "audio") return;
    const t = ++token;
    const rect = el.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    try {
      const r = await invoke<{ path: string }>("get_waveform", {
        assetId,
        width: Math.max(1, Math.round(rect.width * dpr)),
        height: Math.max(1, Math.round(rect.height * dpr)),
      });
      if (t === token) path.value = r.path;
    } catch {
      // 生成失败：维持 null，调用方回退 info 缩略图（不打扰用户，纯观感降级）
    }
  }

  function schedule(): void {
    window.clearTimeout(timer);
    timer = window.setTimeout(() => void request(), 250);
  }

  watch(
    [elRef, () => preview.current?.assetId],
    ([el], _prev, onCleanup) => {
      token += 1;
      path.value = null; // 切素材回退缩略图兜底，等新图防串图
      if (!el) return;
      const ro = new ResizeObserver(schedule);
      ro.observe(el); // observe 即触发一次首帧尺寸回调 → 首次请求由此而来
      onCleanup(() => ro.disconnect());
    },
    { immediate: true, flush: "post" },
  );

  onBeforeUnmount(() => window.clearTimeout(timer));
  return path;
}
