<script setup lang="ts">
// 内容件 · 字体示例字（font）——R5 扩展点注册表化（2026-09-21）自 FileCard 抽出，
// 注册见 src/lib/cardSpecs/font.ts；样式块原样随迁、值未动。
// 示例字缺省值在宿主 FileCard props（日/韩环境示例字业务侧替换）。
// fontFamily（2026-09-22 R6 font 接线）= cardFonts 注册的 FontFace 家族名——示例字以该字体
// 真渲染；缺省 / 加载失败回退系统字体（与 PreviewFont「字体加载失败，以系统字体预览」同口径）。
// 字号随卡缩放（2026-09-22 用户实机定，与文本卡「字号固定」相反口径）：基准 = 稿 220 卡 ↔
// 示例字 18px（标题档行高 26/18 恒定、行 gap 4/18 同比）；卡宽 = 本件根宽 + 24（inset 12×2 恒定）。
import { computed, onBeforeUnmount, onMounted, ref } from "vue";

defineOptions({ inheritAttrs: false });
const props = withDefaults(defineProps<{ fontLines?: string[]; fontFamily?: string }>(), {
  fontLines: () => [],
  fontFamily: "",
});

const rootRef = ref<HTMLElement>();
const fontSize = ref(18); // 初值 = 稿基准（220 卡 ↔ 18）
let ro: ResizeObserver | undefined;
onMounted(() => {
  ro = new ResizeObserver((entries) => {
    const w = entries[0]?.contentRect.width ?? 0;
    fontSize.value = ((w + 24) * 18) / 220;
  });
  if (rootRef.value) ro.observe(rootRef.value);
});
onBeforeUnmount(() => ro?.disconnect());

// 注册家族名就绪才挂 fontFamily（未就绪 = 系统字体兜底）；字号 / 行距随卡宽同比
const linesStyle = computed(() => ({
  ...(props.fontFamily ? { fontFamily: `"${props.fontFamily}", sans-serif` } : {}),
  fontSize: `${fontSize.value}px`,
  gap: `${(fontSize.value * 4) / 18}px`,
}));
</script>

<template>
  <div ref="rootRef" class="fc-font">
    <div class="fc-font-lines" :style="linesStyle">
      <span v-for="l in fontLines" :key="l" class="fc-font-line">{{ l }}</span>
    </div>
  </div>
</template>

<style scoped>
/* 字体示例字：块居中、行左对齐，标题档 亮色/四级；字号 / 行距 inline 随卡缩放（稿 220 卡 ↔ 18 字） */
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
}
.fc-font-line {
  line-height: 1.4444; /* 稿行高 26 / 字 18 恒定比例（字号 inline 随卡缩放，行高同比跟随） */
  font-weight: var(--n-fw-bold);
  color: var(--n-l4);
  white-space: nowrap;
}
</style>
