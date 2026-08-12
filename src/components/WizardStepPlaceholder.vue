<script setup lang="ts">
// 向导第三 / 四步（38 稿）：浏览器扩展 / 桥接插件安装——后续版本（PRD 01_储存空间 3.1）。
// 占位图标真实化（2026-08-12 用户定）：系统提取的浏览器 / Adobe 主程序图标（PNG data URL），
// 默认压暗（grayscale + brightness），悬停变彩色，点击无反应（功能后续版本）；
// 未安装 / 提取失败回退 C1 占位块。文字提示行保留。
export interface PlaceholderItem {
  id: string;
  icon: string | null;
}

withDefaults(defineProps<{ text: string; items?: PlaceholderItem[] }>(), { items: undefined });
</script>

<template>
  <div class="step-placeholder">
    <div class="blocks">
      <template v-if="items && items.length > 0">
        <span v-for="it in items" :key="it.id" class="block" :class="{ real: it.icon }">
          <img v-if="it.icon" :src="it.icon" class="icon" :alt="it.id" draggable="false" />
        </span>
      </template>
      <span v-else v-for="i in 3" :key="i" class="block" />
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
  padding: var(--space-24) var(--space-48);
  gap: var(--space-12);
  width: 100%;
  flex: 1;
  min-height: 0;
}
.blocks {
  display: flex;
  flex-direction: row;
  gap: var(--space-24);
}
/* 图标位（38 稿未选中态规格）：48×48 / C1 底 / 圆角 6 */
.block {
  display: flex;
  align-items: center;
  justify-content: center;
  width: var(--icon-48);
  height: var(--icon-48);
  background: var(--c1);
  border-radius: var(--radius-6);
}
/* 真实图标：默认压暗近稿内未选中观感，悬停还原彩色；点击无反应（占位） */
.block .icon {
  width: 100%;
  height: 100%;
  object-fit: contain;
  filter: grayscale(1) brightness(0.55);
  transition: filter 0.2s ease;
  cursor: default;
  user-select: none;
}
.block:hover .icon {
  filter: none;
}
.hint {
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  color: var(--text-label);
}
</style>
