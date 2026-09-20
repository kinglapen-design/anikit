<script setup lang="ts">
// 内容件 · 字体示例字（font）——R5 扩展点注册表化（2026-09-21）自 FileCard 抽出，
// 注册见 src/lib/cardSpecs/font.ts；样式块原样随迁、值未动。
// 示例字缺省值在宿主 FileCard props（日/韩环境示例字业务侧替换）。
defineOptions({ inheritAttrs: false });
withDefaults(defineProps<{ fontLines?: string[] }>(), { fontLines: () => [] });
</script>

<template>
  <div class="fc-font">
    <div class="fc-font-lines">
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
