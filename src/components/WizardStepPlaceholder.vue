<script setup lang="ts">
// 向导第三 / 四步（38 稿）：浏览器扩展 / 桥接插件安装——后续版本（PRD 01_储存空间 3.1）。
// 图标 = 用户导出的 on/off 双态 SVG（01_icon-default）：默认 off 暗态，悬停 on 彩色
//（2026-08-12 用户定），点击无反应（功能后续版本）。
import AppIcon from "./AppIcon.vue";

export interface PlaceholderIcon {
  off: string;
  on: string;
}

withDefaults(defineProps<{ text: string; icons?: PlaceholderIcon[] }>(), { icons: undefined });
</script>

<template>
  <div class="step-placeholder">
    <div class="blocks">
      <span v-for="it in icons ?? []" :key="it.off" class="block">
        <AppIcon :name="it.off" :size="48" class="ic off" />
        <AppIcon :name="it.on" :size="48" class="ic on" />
      </span>
    </div>
    <span class="hint">{{ text }}</span>
  </div>
</template>

<style scoped>
/* 内容区（38 稿）：居中 / padding 24 48 / gap 24 */
.step-placeholder {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  padding: var(--n-space-24) var(--space-48);
  gap: var(--n-space-12);
  width: 100%;
  flex: 1;
  min-height: 0;
}
.blocks {
  display: flex;
  flex-direction: row;
  gap: var(--n-space-24);
}
/* 双态图标位：48×48 无底（SVG 自带完整图形）；点击无反应（占位） */
.block {
  position: relative;
  width: 48px;
  height: 48px;
  cursor: default;
  user-select: none;
}
.ic {
  position: absolute;
  inset: 0;
  transition: opacity 0.2s ease;
}
.ic.on {
  opacity: 0;
}
.block:hover .ic.on {
  opacity: 1;
}
.block:hover .ic.off {
  opacity: 0;
}
.hint {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
}
</style>
