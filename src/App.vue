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
/* 全透明窗口：容器内缩 20px 作阴影带，圆角与阴影纯 CSS 绘制（跨平台一致）。
   阴影参数暂无 token，初值 0 4 24 rgba(0,0,0,.5)，验收时与用户确认。 */
.window {
  height: calc(100% - 40px);
  margin: 20px;
  display: flex;
  gap: 0;
  padding: var(--space-8);
  background: var(--c1);
  border: var(--border-thick) solid var(--c4-15);
  border-radius: var(--radius-12);
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.5);
  overflow: hidden;
}
.window.maximized {
  height: 100%;
  margin: 0;
  border-radius: 0;
  border-color: transparent;
  box-shadow: none;
}
</style>
