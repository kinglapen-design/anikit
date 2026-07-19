<script setup lang="ts">
// 主窗口（组件布局 01）：C1 底 / 圆角 12 / 内边距 8 / 子项间距 4 / 边框 2px C4-15。
// 无边框透明窗口：最大化时去圆角与边框。
import { onBeforeUnmount, onMounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import SidebarLeft from "./components/SidebarLeft.vue";
import SidebarRight from "./components/SidebarRight.vue";
import ContentArea from "./components/ContentArea.vue";
import SplitBar from "./components/SplitBar.vue";
import { useUiStore } from "./stores/ui";

const ui = useUiStore();
const maximized = ref(false);
const appWindow = getCurrentWindow();

async function syncMaximized(): Promise<void> {
  maximized.value = await appWindow.isMaximized();
}

let resizeTimer: ReturnType<typeof setTimeout> | undefined;
function onResize(): void {
  // 拖动窗口期间高频触发，debounce 后再查窗口状态（IPC）
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(() => void syncMaximized(), 150);
}

onMounted(() => {
  void syncMaximized();
  window.addEventListener("resize", onResize);
});
onBeforeUnmount(() => {
  clearTimeout(resizeTimer);
  window.removeEventListener("resize", onResize);
});
</script>

<template>
  <div class="window" :class="{ maximized }">
    <SidebarLeft />
    <SplitBar v-if="!ui.leftCollapsed" side="left" />
    <ContentArea />
    <SplitBar v-if="!ui.rightCollapsed" side="right" />
    <SidebarRight />
  </div>
</template>

<style scoped>
.window {
  height: 100%;
  display: flex;
  gap: 0;
  padding: var(--space-8);
  background: var(--c1);
  border: var(--border-thick) solid var(--c4-15);
  border-radius: var(--radius-12);
  overflow: hidden;
}
.window.maximized {
  border-radius: 0;
  border-color: transparent;
}
</style>
