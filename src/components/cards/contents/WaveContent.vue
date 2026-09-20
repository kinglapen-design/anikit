<script setup lang="ts">
// 内容件 · 音频波形（audio）——R5 扩展点注册表化（2026-09-21）自 FileCard 抽出，
// 注册见 src/lib/cardSpecs/audio.ts；样式块原样随迁、值未动（扫过显隐的祖先态选择器
// .fc-root:hover / .force-hover / .playing 照原样，scoped 只给末段加本件 data-v，照常命中）。
import { computed } from "vue";

defineOptions({ inheritAttrs: false });
const props = withDefaults(defineProps<{ peaks?: number[]; progress?: number }>(), {
  peaks: () => [],
  progress: 0,
});

const hasProgress = computed(() => props.progress > 0);

// 波形竖线高度：peaks 归一化 → 百分比（最低 8% ≈ 稿最小 13px / 容器 176）
function barH(p: number): string {
  return `${Math.min(100, Math.max(8, p * 100))}%`;
}
</script>

<template>
  <div class="fc-wave">
    <div class="fc-wave-row">
      <span v-for="(p, i) in peaks" :key="i" class="fc-wave-bar" :style="{ height: barH(p) }" />
    </div>
    <div
      v-if="hasProgress"
      class="fc-wave-row played"
      :style="{ clipPath: `inset(0 ${(1 - progress) * 100}% 0 0)` }"
    >
      <span v-for="(p, i) in peaks" :key="i" class="fc-wave-bar" :style="{ height: barH(p) }" />
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
