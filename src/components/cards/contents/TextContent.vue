<script setup lang="ts">
// 内容件 · 文本滚动预览（text）——R5 扩展点注册表化（2026-09-21）自 FileCard 抽出，
// 注册见 src/lib/cardSpecs/text.ts（scrollThumb: true）；2026-09-22 生产接线（txt/md/docx）。
// 滚动胶囊本体在宿主 FileCard 根层（picked 压暗不波及，kc-clip 外），本件根元素即滚动容器——
// 宿主经组件 ref.$el 挂 scroll 监听 / ResizeObserver（spec scrollThumb 契约：单根元素、根即滚动容器）。
// 2026-09-22 用户实机走查三条修订：
// ① 字号固定稿值不随卡宽缩放（废 2026-09-13 cqw 等比缩放——卡片缩放只影响显示行数）；
// ② 滚轮不介入卡内（overflow hidden——卡不再是滚轮目标，滚轮恒滚页面；卡内滚动走右缘胶囊拖动，
//    hidden 元素 scrollTop 仍可程序写）；
// ③ 卡内文本让位格式角标（topInset：有角标时起始 = 角标下缘 + 4，spec 映射传入）。
defineOptions({ inheritAttrs: false });
withDefaults(defineProps<{ textTitle?: string; textBody?: string; topInset?: number }>(), {
  textTitle: "",
  textBody: "",
  topInset: 12,
});
</script>

<template>
  <div class="fc-text" :style="{ top: `${topInset}px` }">
    <div v-if="textTitle" class="fc-text-title">{{ textTitle }}</div>
    <div class="fc-text-body">{{ textBody }}</div>
  </div>
</template>

<style scoped>
/* 文本预览：卡 padding 12 内滚动（滚动内容可入渐变罩下），标题 二级 / 正文 四级 gap 8；
   字号固定稿基准值（18/26、13/18）——卡片尺寸缩放只改变可见行数，文字大小不变 */
.fc-text {
  position: absolute;
  left: 12px;
  right: 12px;
  bottom: 12px;
  /* top 由 style 绑定（topInset：格式角标让位，无角标 = 12） */
  overflow-y: hidden; /* 滚轮目标让位页面；卡内滚动由宿主胶囊拖动驱动 scrollTop */
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.fc-text-title {
  flex: none;
  font-size: 18px;
  line-height: 1.45; /* 26/18 */
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
.fc-text-body {
  font-size: 13px;
  line-height: 1.39; /* 18/13 */
  color: var(--n-l4);
  white-space: pre-wrap;
}
</style>
