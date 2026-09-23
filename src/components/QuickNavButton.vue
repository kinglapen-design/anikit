<script setup lang="ts">
// 快速到顶 / 到底圆钮（2026-09-23 用户定）：视口右下角纯圆浮钮（搜索框壳同款视觉 = IconButton
// circle 变体），往下持续滚动出现「滚到底」、往上持续滚动出现「回到顶」——显隐状态机由调用方
// 按各自滚动源驱动（AssetGrid 虚拟滚动 onUserScroll / SequencePackView 原生 scroll），
// 本组件只管呈现（定位 / 方向箭头 / 出入场动画）与点击上报。
import IconButton from "./buttons/IconButton.vue";

const props = defineProps<{ dir: "top" | "bottom" | null }>();
const emit = defineEmits<{ click: [] }>();
</script>

<template>
  <Transition name="qn-reveal">
    <IconButton
      v-if="props.dir"
      variant="circle"
      name="chevron-large"
      class="quick-nav"
      :class="props.dir"
      :title="props.dir === 'top' ? '回到顶部' : '滚到底部'"
      @click="emit('click')"
    />
  </Transition>
</template>

<style scoped>
/* 定位：视口右下角——底 12 与多功能悬浮栏同水平线，右 20 让开滚动条（命中区 12）+ 8 间隙；
   z 与滚动条同档（悬浮层） */
.quick-nav {
  position: absolute;
  right: 20px;
  bottom: 12px;
  z-index: 10;
}
/* chevron-large 源图形朝左：旋转定向（悬浮栏收起钮 rotate 同口径先例）——左 →上 / 下 */
.quick-nav.top :deep(.app-icon) {
  transform: rotate(90deg);
}
.quick-nav.bottom :deep(.app-icon) {
  transform: rotate(-90deg);
}
/* 出入场：下浮入 / 下沉出（位移 8 + 透明度 250ms 缓入缓出——悬浮栏族动画档同口径，§18） */
.qn-reveal-enter-active,
.qn-reveal-leave-active {
  transition:
    transform var(--n-dur-250) var(--n-ease-in-out),
    opacity var(--n-dur-250) var(--n-ease-in-out);
}
.qn-reveal-enter-from,
.qn-reveal-leave-to {
  transform: translateY(8px);
  opacity: 0;
}
</style>
