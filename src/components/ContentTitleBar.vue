<script setup lang="ts">
// 中栏标题栏（组件布局 05.1）：返回 / 前进 + 面包屑 + 滑块 + 显示子文件夹内容 + 插件 / 查看方式。
// 返回前进阶段 1 无导航历史，禁用态；查看方式二级菜单无设计，仅摆按钮。
import AppIcon from "./AppIcon.vue";
import IconButton from "./IconButton.vue";
import SideSlider from "./SideSlider.vue";
import AppCheckbox from "./AppCheckbox.vue";
import { useUiStore } from "../stores/ui";

const ui = useUiStore();
</script>

<template>
  <div class="content-titlebar" data-tauri-drag-region>
    <IconButton name="chevron-large" title="返回" class="rot-ccw" disabled />
    <IconButton name="chevron-large" title="前进" class="rot-cw" disabled />
    <button class="crumb">
      <span class="crumb-text">全部</span>
      <AppIcon name="chevron-small" :size="8" />
    </button>
    <span class="spacer" />
    <SideSlider v-model="ui.targetRowHeight" :min="120" :max="400" />
    <AppCheckbox v-model="ui.showSubfolders" label="显示子文件夹内容" />
    <span class="spacer" />
    <IconButton name="plugin" title="插件" />
    <IconButton name="view-options" title="查看方式" />
  </div>
</template>

<style scoped>
/* 内容容器：内边距 8 / 子项间距自适应 */
.content-titlebar {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-8);
  flex: none;
}
/* chevron-large 原图标朝下：返回 = 逆时针 90°（朝左），前进 = 顺时针 90°（朝右） */
.rot-ccw :deep(svg) {
  transform: rotate(90deg);
}
.rot-cw :deep(svg) {
  transform: rotate(-90deg);
}
.crumb {
  display: inline-flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-4);
  border-radius: var(--radius-6);
  color: var(--c4);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.crumb:hover {
  background: var(--c4-7);
}
.spacer {
  flex: 1;
}
</style>
