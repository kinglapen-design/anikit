<script setup lang="ts">
// 内容件 · 字体示例字（font）——R5 扩展点注册表化（2026-09-21）自 FileCard 抽出，
// 注册见 src/lib/cardSpecs/font.ts；样式块原样随迁、值未动。
// 示例字缺省值在宿主 FileCard props（日/韩环境示例字业务侧替换）。
// fontFamily（2026-09-22 R6 font 接线）= cardFonts 注册的 FontFace 家族名——示例字以该字体
// 真渲染；缺省 / 加载失败回退系统字体（与 PreviewFont「字体加载失败，以系统字体预览」同口径）。
import { computed } from "vue";

defineOptions({ inheritAttrs: false });
const props = withDefaults(defineProps<{ fontLines?: string[]; fontFamily?: string }>(), {
  fontLines: () => [],
  fontFamily: "",
});
// 注册家族名就绪才挂 fontFamily（模板字面量避开模板内引号转义）；未就绪 = 系统字体兜底
const linesStyle = computed(() =>
  props.fontFamily ? { fontFamily: `"${props.fontFamily}", sans-serif` } : undefined,
);
</script>

<template>
  <div class="fc-font">
    <div class="fc-font-lines" :style="linesStyle">
      <span v-for="l in fontLines" :key="l" class="fc-font-line">{{ l }}</span>
    </div>
  </div>
</template>

<style scoped>
/* 字体示例字：块居中、行左对齐，标题档 亮色/四级 */
.fc-font {
  position: absolute;
  inset: var(--n-space-12);
  display: flex;
  align-items: center;
  justify-content: center;
}
.fc-font-lines {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--n-space-4);
}
.fc-font-line {
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l4);
  white-space: nowrap;
}
</style>
