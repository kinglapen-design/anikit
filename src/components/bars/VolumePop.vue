<script setup lang="ts">
// 音量滑块弹层（播放条-音量滑块稿）：壳 .fb-pop + padding 12，宽 124 = 稿值；滑块 = BarSlider dark 满宽 100。
import BarSlider from "./BarSlider.vue";
import "./bars.css";

const model = defineModel<number>({ default: 0.2 }); // 0~1
</script>

<template>
  <div class="fb-pop vp-root">
    <BarSlider v-model="model" tone="dark" />
  </div>
</template>

<style scoped>
.vp-root {
  display: flex;
  align-items: center;
  width: 124px; /* 稿值（padding 12 + 滑块 100 链） */
  padding: var(--n-space-12);
}
</style>
