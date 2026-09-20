<script setup lang="ts">
// 内容件 · 文本滚动预览（text）——R5 扩展点注册表化（2026-09-21）自 FileCard 抽出，
// 注册见 src/lib/cardSpecs/text.ts（scrollThumb: true）；样式块原样随迁、值未动。
// 滚动胶囊本体在宿主 FileCard 根层（picked 压暗不波及，kc-clip 外），本件根元素即滚动容器——
// 宿主经组件 ref.$el 挂 scroll 监听 / ResizeObserver（spec scrollThumb 契约：单根元素、根即滚动容器）。
defineOptions({ inheritAttrs: false });
withDefaults(defineProps<{ textTitle?: string; textBody?: string }>(), {
  textTitle: "",
  textBody: "",
});
</script>

<template>
  <div class="fc-text">
    <div v-if="textTitle" class="fc-text-title">{{ textTitle }}</div>
    <div class="fc-text-body">{{ textBody }}</div>
  </div>
</template>

<style scoped>
/* 文本预览：卡 padding 12 内滚动（滚动内容可入渐变罩下），标题 二级 / 正文 四级 gap 8；
   内容随卡宽等比缩放（2026-09-13 实机走查：文档卡片需要跟随尺寸缩放）——cqw 容器查询单位，
   稿基准 300 宽：标题 18px=6cqw / 正文 13px≈4.34cqw / 内边距 12px=4cqw / gap 8px≈2.67cqw
   （cqw 对宿主 .fc-root container-type: inline-size 生效，与组件边界无关） */
.fc-text {
  position: absolute;
  inset: 4cqw;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 2.67cqw;
  scrollbar-width: none;
}
.fc-text::-webkit-scrollbar {
  display: none;
}
.fc-text-title {
  flex: none;
  font-size: 6cqw;
  line-height: 1.45; /* 26/18 */
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
.fc-text-body {
  font-size: 4.34cqw;
  line-height: 1.39; /* 18/13 */
  color: var(--n-l4);
  white-space: pre-wrap;
}
</style>
