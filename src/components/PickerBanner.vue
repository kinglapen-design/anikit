<script setup lang="ts">
// 选择模式目的横幅（figma-css 02_布局/02_主窗口选择模式）：品牌底胶囊浮于中间面板顶部——
// 稿口径「从上至下滑入，随后循环上下移动，引起注目」：进入动画 0.3s 滑入，随后 2.6s 循环浮动。
// 文案 = 目的 + 发起名（picker.bannerText）+ Esc 取消标注（PRD 03_数据链接 04）。
import { usePickerStore } from "../stores/picker";

const picker = usePickerStore();
</script>

<template>
  <!-- 外层恒定位（translateX 居中）；内层做动画（slide-in 与 float 都是纯 translateY，不与居中冲突） -->
  <div class="banner-anchor">
    <div class="banner">
      <span class="text">{{ picker.bannerText }}</span>
      <span class="esc">Esc 取消</span>
    </div>
  </div>
</template>

<style scoped>
/* 定位锚：中间面板顶部（稿 top 60），窗口水平居中，不接收事件（浮层不挡下方卡片操作） */
.banner-anchor {
  position: absolute;
  top: 60px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 50;
  pointer-events: none;
}
/* 稿：品牌色/常态 底 + 1px 亮-选中 border + 投影 0 4 40 60% + 圆角 6 + padding 8 12 + gap 6 */
.banner {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  box-sizing: border-box;
  padding: var(--space-8) var(--space-12);
  border-radius: var(--radius-6);
  background: var(--brand);
  border: 1px solid var(--line-active);
  box-shadow: 0 4px 40px rgba(0, 0, 0, 0.6);
  animation:
    banner-in 0.3s ease-out,
    banner-float 2.6s ease-in-out 0.35s infinite;
}
.text {
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--text-highlight);
  white-space: nowrap;
}
/* Esc 取消标注（PRD 口径；稿无样式——高亮色降透明附后） */
.esc {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--text-highlight);
  opacity: 0.7;
  white-space: nowrap;
}
/* 从上至下滑入 */
@keyframes banner-in {
  from {
    transform: translateY(-48px);
    opacity: 0;
  }
  to {
    transform: translateY(0);
    opacity: 1;
  }
}
/* 随后循环上下移动（引起注目） */
@keyframes banner-float {
  0%,
  100% {
    transform: translateY(0);
  }
  50% {
    transform: translateY(-6px);
  }
}
</style>
