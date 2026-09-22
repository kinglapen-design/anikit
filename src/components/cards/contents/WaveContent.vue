<script setup lang="ts">
// 内容件 · 音频波形（audio）——R5 扩展点注册表化（2026-09-21）自 FileCard 抽出，
// 注册见 src/lib/cardSpecs/audio.ts；样式块原样随迁、值未动（扫过显隐的祖先态选择器
// .fc-root:hover / .force-hover / .playing 照原样，scoped 只给末段加本件 data-v，照常命中）。
// 根数按容器宽走（2026-09-22 R6 audio 接线）：稿 47 根 = 376 区 / 节距 8（4 线 + 4 距）——
// 根数跟宽度定（尺寸滑块改卡宽时 4px/4gap 恒保稿值、两端铺满），peaks 超量部分 max 池化
// 重采样（保峰值形态，稿 prose「美观第一位」）；prose「根数一致」指不随时长（后端恒定
// 128 根供给），非恒定根数。
import { computed, onBeforeUnmount, onMounted, ref } from "vue";

defineOptions({ inheritAttrs: false });
const props = withDefaults(defineProps<{ peaks?: number[]; progress?: number }>(), {
  peaks: () => [],
  progress: 0,
});

const hasProgress = computed(() => props.progress > 0);

// 竖线节距（px）= 稿值 4 线宽 + 4 间距（与下方 scoped 样式同值，改样式须同步）
const BAR_PITCH = 8;

const rootRef = ref<HTMLElement>();
const barCount = ref(0);
let ro: ResizeObserver | undefined;
onMounted(() => {
  ro = new ResizeObserver((entries) => {
    const w = entries[0]?.contentRect.width ?? 0;
    barCount.value = Math.floor(w / BAR_PITCH);
  });
  if (rootRef.value) ro.observe(rootRef.value);
});
onBeforeUnmount(() => ro?.disconnect());

/** peaks → barCount 根：超量分桶取 max 池化（保峰值辨识度）；不足原样（居中留白，实不会出现——后端恒供 128 根） */
const displayPeaks = computed(() => {
  const src = props.peaks;
  const m = Math.min(barCount.value, src.length);
  if (m <= 0) return [] as number[];
  if (m === src.length) return src;
  const step = src.length / m;
  const out: number[] = [];
  for (let i = 0; i < m; i++) {
    const s0 = Math.floor(i * step);
    const s1 = Math.max(Math.floor((i + 1) * step), s0 + 1);
    let peak = 0;
    for (let j = s0; j < s1 && j < src.length; j++) peak = Math.max(peak, src[j] ?? 0);
    out.push(peak);
  }
  return out;
});

// 波形竖线高度：peaks 归一化 → 百分比（最低 8% ≈ 稿最小 13px / 容器 176）
function barH(p: number): string {
  return `${Math.min(100, Math.max(8, p * 100))}%`;
}
</script>

<template>
  <div ref="rootRef" class="fc-wave">
    <div class="fc-wave-row">
      <span v-for="(p, i) in displayPeaks" :key="i" class="fc-wave-bar" :style="{ height: barH(p) }" />
    </div>
    <div
      v-if="hasProgress"
      class="fc-wave-row played"
      :style="{ clipPath: `inset(0 ${(1 - progress) * 100}% 0 0)` }"
    >
      <span v-for="(p, i) in displayPeaks" :key="i" class="fc-wave-bar" :style="{ height: barH(p) }" />
    </div>
  </div>
</template>

<style scoped>
/* 音频波形：竖线 4px 圆头 gap 4 通用/五级，垂直居中；扫过层 = 品牌色同构层 clip-path 按进度裁。
   扫过显隐同悬停控件口径（hover 才现，2026-08-29 用户定），播放中恒显 */
.fc-wave {
  position: absolute;
  inset: var(--n-space-12);
  display: flex;
  align-items: center;
}
.fc-wave-row {
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: center;
  gap: var(--n-space-4);
  width: 100%;
  height: 100%;
}
.fc-wave-row.played {
  position: absolute;
  inset: 0;
  opacity: 0;
  transition: opacity var(--n-dur-150) var(--n-ease-in-out);
}
.fc-root:hover .fc-wave-row.played,
.fc-root.force-hover .fc-wave-row.played,
.fc-root.playing .fc-wave-row.played {
  opacity: 1;
}
.fc-wave-bar {
  flex: none;
  width: 4px;
  border-radius: 99px; /* 稿值 99 = 胶囊圆头（Token.md 圆角无此档，非新造 token） */
  background: var(--n-g5);
}
.fc-wave-row.played .fc-wave-bar {
  background: var(--n-brand);
}
</style>
