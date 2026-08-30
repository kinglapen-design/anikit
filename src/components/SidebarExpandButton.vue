<script setup lang="ts">
// 左栏收起时的"展开侧边栏"按钮（dockview prefixHeaderActions，标签栏行首）：
// 2026-08-08 收展定案——左栏整栏收起，展开按钮在标签页行（不占工具栏；工具栏左侧留给档案看板）。
// 未收起时不渲染。出现/消失 = 透明度过渡 150ms 缓入缓出（2026-08-30 用户定，token 通用档）。
import IconButton from "./IconButton.vue";
import { useUiStore } from "../stores/ui";

const ui = useUiStore();
</script>

<template>
  <Transition name="sb-expand">
    <IconButton
      v-if="ui.leftCollapsed"
      name="collapse-sidebar"
      title="展开侧边栏"
      class="flip-h"
      @click="ui.leftCollapsed = false"
    />
  </Transition>
</template>

<style scoped>
.flip-h :deep(svg) {
  transform: scaleX(-1);
}
.sb-expand-enter-active,
.sb-expand-leave-active {
  transition: opacity var(--n-dur-150) var(--n-ease-in-out);
}
.sb-expand-enter-from,
.sb-expand-leave-to {
  opacity: 0;
}
</style>
