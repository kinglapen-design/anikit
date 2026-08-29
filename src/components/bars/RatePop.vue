<script setup lang="ts">
// 倍率播放弹层（播放条-倍率播放稿）：壳 .fb-pop + padding 12 + gap 8；
// 项 = 高 26 padding 4/8 圆角 6 + 通用/五级底 + 立体感-通用，文字 数字-大 亮色/三级（宽随文案自适应）；
// hover 底升 通用/六级；选中 = 品牌/常态底 + 立体感-主要按钮 + 文字 亮色/一级（稿 prose）。
// 档位顺序按稿降序：8x → 0.25x。
import "./bars.css";

withDefaults(
  defineProps<{
    rates?: readonly string[];
    forceHover?: string; // 预览页静态验收：某档 hover
  }>(),
  {
    rates: () => ["8x", "4x", "3x", "2x", "1.75x", "1.5x", "1.25x", "1x", "0.75x", "0.5x", "0.25x"],
    forceHover: undefined,
  },
);
const model = defineModel<string>({ default: "1x" });
</script>

<template>
  <div class="fb-pop rp-root">
    <button
      v-for="r in rates"
      :key="r"
      class="rp-item"
      :class="[{ on: r === model, 'force-hover': r === forceHover }]"
      @click="model = r"
    >
      {{ r }}
    </button>
  </div>
</template>

<style scoped>
.rp-root {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  padding: var(--n-space-12);
  width: max-content;
}
.rp-item {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  height: 26px;
  padding: var(--n-space-4) var(--n-space-8);
  border-radius: var(--n-radius-6);
  background: var(--n-g5);
  box-shadow: var(--n-effect-raised);
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num-lg);
  line-height: var(--n-lh-num-lg);
  font-weight: var(--n-fw-medium);
  color: var(--n-l3);
  white-space: nowrap;
  transition:
    background var(--n-dur-150) var(--n-ease-in-out),
    color var(--n-dur-150) var(--n-ease-in-out);
}
.rp-item:hover,
.rp-item.force-hover {
  background: var(--n-g6);
}
.rp-item.on {
  background: var(--n-brand);
  box-shadow: var(--n-effect-raised-primary);
  color: var(--n-l1);
}
</style>
