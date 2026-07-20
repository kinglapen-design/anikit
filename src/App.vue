<script setup lang="ts">
// 主窗口（03_UI 布局/01）：C1 底 / 圆角 8 / padding 0 / 间距 0 / 外边框 1px C4-15。
// 全透明无边框窗口：最大化时去圆角与边框；resize 判定在可视边缘内 8px（startResizeDragging）。
import { onBeforeUnmount, onMounted, ref } from "vue";
import { getCurrentWindow, type ResizeDirection } from "@tauri-apps/api/window";
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

// 可视窗口边缘 resize 判定（透明阴影带外缘的 tao 物理判定保留作补充）
const resizeZones: { dir: ResizeDirection; cls: string }[] = [
  { dir: "North", cls: "n" },
  { dir: "South", cls: "s" },
  { dir: "East", cls: "e" },
  { dir: "West", cls: "w" },
  { dir: "NorthEast", cls: "ne" },
  { dir: "NorthWest", cls: "nw" },
  { dir: "SouthEast", cls: "se" },
  { dir: "SouthWest", cls: "sw" },
];

function startResize(dir: ResizeDirection, e: MouseEvent): void {
  if (e.button !== 0) return;
  e.preventDefault();
  void appWindow.startResizeDragging(dir);
}
</script>

<template>
  <div class="window" :class="{ maximized }">
    <SidebarLeft />
    <SplitBar v-if="!ui.leftCollapsed" side="left" />
    <ContentArea />
    <SplitBar v-if="!ui.rightCollapsed" side="right" />
    <SidebarRight />
    <template v-if="!maximized">
      <div
        v-for="z in resizeZones"
        :key="z.cls"
        class="resize-zone"
        :class="z.cls"
        @mousedown="startResize(z.dir, $event)"
      />
    </template>
  </div>
</template>

<style scoped>
/* 全透明窗口：容器内缩 20px 作阴影带，圆角与阴影纯 CSS 绘制（跨平台一致）。
   主窗口（03_UI 布局/01）：padding 6 / 圆角 8 / 阴影 0 4 40 rgba(0,0,0,.4)。 */
.window {
  position: relative;
  height: calc(100% - 40px);
  margin: 20px;
  display: flex;
  gap: 0;
  padding: 0;
  background: var(--c1);
  border-radius: var(--radius-8);
  /* Figma 描边为 inside（不占空间）：边框一律 inset 阴影绘制，padding 即标注值。
     投影 blur 收到 16px：阴影带仅 20px，blur 40 会被窗口物理边缘裁成硬边（工程限制） */
  box-shadow:
    0 4px 16px rgba(0, 0, 0, 0.4),
    inset 0 0 0 1px var(--c4-15);
  overflow: hidden;
}
.window.maximized {
  height: 100%;
  margin: 0;
  border-radius: 0;
  box-shadow: none;
}
/* 可视边缘内 8px 的 resize 判定热区（startResizeDragging 手动发起） */
.resize-zone {
  position: absolute;
  z-index: 100;
}
.resize-zone.n {
  top: 0;
  left: 8px;
  right: 8px;
  height: 8px;
  cursor: n-resize;
}
.resize-zone.s {
  bottom: 0;
  left: 8px;
  right: 8px;
  height: 8px;
  cursor: s-resize;
}
.resize-zone.e {
  top: 8px;
  bottom: 8px;
  right: 0;
  width: 8px;
  cursor: e-resize;
}
.resize-zone.w {
  top: 8px;
  bottom: 8px;
  left: 0;
  width: 8px;
  cursor: w-resize;
}
.resize-zone.ne {
  top: 0;
  right: 0;
  width: 8px;
  height: 8px;
  cursor: ne-resize;
}
.resize-zone.nw {
  top: 0;
  left: 0;
  width: 8px;
  height: 8px;
  cursor: nw-resize;
}
.resize-zone.se {
  bottom: 0;
  right: 0;
  width: 8px;
  height: 8px;
  cursor: se-resize;
}
.resize-zone.sw {
  bottom: 0;
  left: 0;
  width: 8px;
  height: 8px;
  cursor: sw-resize;
}
</style>
