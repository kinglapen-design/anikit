<script setup lang="ts">
// 向导第三 / 四步（38 稿）：浏览器扩展 / 桥接插件安装——后续版本（PRD 01_储存空间 3.1）。
// 内容区三个 48×48 占位块（稿内未选中态图标的 C1 底圆角块规格）+ 占位提示行；
// 无真实软件 logo 资源，不引外部图，无可点假按钮。
withDefaults(defineProps<{ text: string; count?: number }>(), { count: 3 });
</script>

<template>
  <div class="step-placeholder">
    <div class="blocks">
      <span v-for="i in count" :key="i" class="block" />
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
/* 占位块（38 稿未选中态图标规格）：48×48 / C1 底 / 圆角 6 */
.block {
  width: var(--icon-48);
  height: var(--icon-48);
  background: var(--c1);
  border-radius: var(--radius-6);
}
.hint {
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  color: var(--text-label);
}
</style>
