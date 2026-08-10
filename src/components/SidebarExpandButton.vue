<script setup lang="ts">
// 左栏收起时的"展开侧边栏"按钮（dockview prefixHeaderActions，标签栏行首）：
// 2026-08-08 收展定案——左栏整栏收起，展开按钮在标签页行（不占工具栏；工具栏左侧留给档案看板）。
// 未收起时不渲染。
import IconButton from "./IconButton.vue";
import { useUiStore } from "../stores/ui";

const ui = useUiStore();
</script>

<template>
  <IconButton
    v-if="ui.leftCollapsed"
    name="collapse-sidebar"
    title="展开侧边栏"
    class="flip-h"
    @click="ui.leftCollapsed = false"
  />
</template>

<style scoped>
.flip-h :deep(svg) {
  transform: scaleX(-1);
}
</style>
