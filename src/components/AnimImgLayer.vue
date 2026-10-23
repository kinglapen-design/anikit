<script setup lang="ts">
// 动图卡片播放层（2026-09-02 回滚原生 <img> + 小动画缓存恢复）：播放源 = info/{stem}_anim.webp
// 短边 320 animated webp（2.3 口径恢复——导入一次转码，卡片播放解码成本与原尺寸脱钩；
// 缓存未生成回退原尺寸源文件原生循环，生成完成经 preview-anim-ready 热切换）。
// 此前阶段四 WS 帧流卡片层（AnimCanvasLayer）实机性能否决：N 卡 = N 个 Rust 全尺寸解码线程 +
// RGBA 裸帧 WS 传输 + 主线程 putImageData，十几张 1080p 即 ~18% CPU 风扇狂转。
// 帧流仍留预览面板（播放条 / 逐帧 / 倍速 / 帧条需帧级控制），卡片层无帧控制需求（PRD 04：
// 悬停即播放原生循环、移出还原静态封面）。路径解析失败（pending / 非直渲）不播，下层静态封面兜底。
// 骨架（进入延迟 / 主窗口隐藏暂停 / disposed 护栏）收编 useHoverPlayer。
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AUTOPLAY_ENTER_DELAY, HOVER_ENTER_DELAY, useHoverPlayer } from "../lib/useHoverPlayer";
import { getAssetFilePath } from "../lib/mediaOps";
import { useUiStore } from "../stores/ui";
import type { PlacementView } from "../stores/content";

// auto = 自动播放触发（视口可见性驱动）：进入延迟走 100ms 短档（2026-08-23 滚回恢复提速）；
// 缺省 false = 悬停触发 300ms 防划过
const props = withDefaults(defineProps<{ asset: PlacementView; auto?: boolean }>(), { auto: false });
const ui = useUiStore();

const src = ref<string | null>(null);
const loaded = ref(false);

const { isDisposed } = useHoverPlayer({
  load: start,
  pause: unload, // 主窗口隐藏：卸载回静态封面（对齐"隐藏停止"语义）
  resume: () => {
    if (!src.value) void start(); // 重新显示：已被 pause 卸载则重载（组件仍在 = 悬停/自动播放态存活）
  },
  cleanup: unload,
  enterDelay: props.auto ? AUTOPLAY_ENTER_DELAY : HOVER_ENTER_DELAY,
});

async function start(): Promise<void> {
  // ws 合成项（2026-10-23 悬停放行）：ws 无 info 小动画缓存——直渲原文件 Chromium 原生循环
  //（GIF 按扩展名直认，AssetCard isAnim 口径）；库资产维持下段 assetId 链不动
  if (props.asset.wsPath) {
    src.value = convertFileSrc(props.asset.wsPath);
    return;
  }
  // 播放源代理（8.1）：fla 等工程卡片的伴生 WebP 动图，播放目标是 hidden 资产而非自身
  let r;
  try {
    r = await getAssetFilePath(props.asset.playAssetId ?? props.asset.assetId);
  } catch {
    return; // 解析失败（资产缺失等）：不播，下层静态封面兜底
  }
  if (isDisposed()) return;
  // 小动画缓存（320）优先；未生成回退原尺寸源文件（已提交懒生成，就绪事件到达热切换）
  const p = r.animPreview ?? r.path;
  src.value = p ? convertFileSrc(p) : null;
}

function unload(): void {
  src.value = null;
  loaded.value = false;
}

// 播放源热切换（2026-09-02 用户实机报：换绑伴生缩略图后卡片仍播旧动画）：源 id 变 = 卸载重载，
// loaded 回落让下层新封面先露出直到新图就绪
watch(
  () => props.asset.playAssetId ?? props.asset.assetId,
  (id, old) => {
    if (!old || id === old) return;
    unload();
    void start();
  },
);

// 小动画就绪热切换：懒生成完成即把播放中的原尺寸源换成 320 缓存（img 换 src 期间旧图保持不闪；
// 未在播不处理——下次 start 的 animPreview ?? path 自然命中）
let unlisten: (() => void) | undefined;
onMounted(async () => {
  const un = await listen<{ assetId: string; path: string }>("preview-anim-ready", (e) => {
    if (!src.value) return;
    if (e.payload.assetId !== (props.asset.playAssetId ?? props.asset.assetId)) return;
    src.value = convertFileSrc(e.payload.path);
  });
  if (isDisposed()) un(); // await 期间组件已卸载：立即反注册防泄漏
  else unlisten = un;
});
onBeforeUnmount(() => unlisten?.());
</script>

<template>
  <!-- 加载完成前 img 隐藏（下层静态封面可见）；铺设口径同 .img（grid contain） -->
  <img
    v-show="loaded"
    class="anim-img"
    :class="{ contain: ui.layoutMode === 'grid' }"
    :src="src ?? undefined"
    alt=""
    @load="loaded = true"
  />
</template>

<style scoped>
.anim-img {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  /* 播放中衬卡片底色（g2）：帧透明像素不透下层静态首帧图——大透明区动图（CEP 伴生 WebP）
     首曝"第一帧永远在后面"（2026-09-01）；GIF 等不透明帧像素全覆盖无视觉变化 */
  background: var(--n-g2);
}
.anim-img.contain {
  object-fit: contain;
}
</style>
