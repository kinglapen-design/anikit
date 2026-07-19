<script setup lang="ts">
// 中栏标题栏（03_UI 布局/03_中间面板 02）：返回 / 前进 + 标题（当前父级文件夹）
// + 滑块（min 80 / max = 内容区宽）+ 显示子文件夹内容 + 插件 / 查看方式。
// 返回前进阶段 1 无导航历史，禁用态 C4-20；搜索结果容器阶段 4 接入。
import IconButton from "./IconButton.vue";
import SideSlider from "./SideSlider.vue";
import AppCheckbox from "./AppCheckbox.vue";
import { useUiStore } from "../stores/ui";

const ui = useUiStore();
</script>

<template>
  <div class="content-titlebar" data-tauri-drag-region>
    <div class="nav-group">
      <IconButton name="chevron-large" title="返回" class="rot-ccw" disabled />
      <IconButton name="chevron-large" title="前进" class="rot-cw" disabled />
      <span class="crumb-text">全部</span>
    </div>
    <span class="spacer" />
    <SideSlider v-model="ui.targetRowHeight" :min="80" :max="ui.gridWidth" />
    <AppCheckbox v-model="ui.showSubfolders" label="显示子文件夹内容" />
    <span class="spacer" />
    <IconButton name="plugin" title="插件" />
    <IconButton name="view-options" title="查看方式" />
  </div>
</template>

<style scoped>
/* 总容器：内边距 8 / space-between */
.content-titlebar {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-8);
  flex: none;
}
/* 面包屑导航容器：返回前进按钮组（间距 4）+ 标题容器 */
.nav-group {
  display: flex;
  align-items: center;
  gap: var(--space-4);
}
/* chevron-large 原图标朝下：返回 = 顺时针 90°（朝左），前进 = 逆时针 90°（朝右） */
.rot-ccw :deep(svg) {
  transform: rotate(90deg);
}
.rot-cw :deep(svg) {
  transform: rotate(-90deg);
}
.crumb-text {
  padding: 0 var(--space-4);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--c4);
  white-space: nowrap;
}
.spacer {
  flex: 1;
}
</style>
