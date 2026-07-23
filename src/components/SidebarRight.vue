<script setup lang="ts">
// 右侧边栏（C3）：标题栏（收起 / 置顶 + 窗口控制）/ 详情视图 / AI 三视图占位 / 底部视图切换。
import { computed, ref } from "vue";
import AppIcon from "./AppIcon.vue";
import IconButton from "./IconButton.vue";
import WindowControls from "./WindowControls.vue";
import ViewSwitch, { type ViewOption } from "./ViewSwitch.vue";
import DetailPanel from "./DetailPanel.vue";
import { useUiStore, type RightView } from "../stores/ui";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useSideThumb } from "../lib/useSideThumb";

const ui = useUiStore();
const bodyRef = ref<HTMLElement>();
const thumb = useSideThumb(bodyRef);
const appWindow = getCurrentWindow();

const rightOptions: ViewOption[] = [
  { value: "detail", icon: "detail-view", title: "详情视图" },
  { value: "aiChat", icon: "ai-chat-view", title: "AI 对话框" },
  { value: "aiPaint", icon: "ai-paint-view", title: "AI 绘画框" },
  { value: "aiVideo", icon: "ai-video-view", title: "AI 视频框" },
];

const aiIcons: Record<Exclude<RightView, "detail">, string> = {
  aiChat: "ai-chat-view",
  aiPaint: "ai-paint-view",
  aiVideo: "ai-video-view",
};
const placeholderIcon = computed(() =>
  ui.rightView === "detail" ? "detail-view" : aiIcons[ui.rightView],
);

async function togglePin(): Promise<void> {
  ui.alwaysOnTop = !ui.alwaysOnTop;
  await appWindow.setAlwaysOnTop(ui.alwaysOnTop);
}
</script>

<template>
  <aside v-if="!ui.rightCollapsed" class="sidebar-right" :style="{ width: `${ui.rightWidth}px` }">
    <header class="bar-title" data-tauri-drag-region>
      <IconButton name="toggle-sidebar-right" title="收起侧边栏" @click="ui.rightCollapsed = true" />
      <IconButton
        :name="ui.alwaysOnTop ? 'window-pin-off' : 'window-pin-on'"
        title="窗口置顶"
        @click="togglePin"
      />
      <span class="spacer" />
      <WindowControls />
    </header>

    <div ref="bodyRef" class="view-body">
      <DetailPanel v-if="ui.rightView === 'detail'" />
      <div v-else class="ai-placeholder">
        <AppIcon :name="placeholderIcon" />
        <p>该视图为占位，后续阶段实现。</p>
      </div>
    </div>
    <span
      v-show="thumb.show"
      class="side-thumb"
      :style="{ top: `${thumb.top}px`, height: `${thumb.height}px` }"
    />

    <footer class="bar-footer">
      <ViewSwitch v-model="ui.rightView" :options="rightOptions" />
    </footer>
  </aside>
</template>

<style scoped>
.sidebar-right {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: none;
  background: var(--c3);
  /* 三栏面板无圆角无边框，仅左侧 1px border-2 作视觉分割线（2026-07-23 线框定案） */
  box-shadow: inset 1px 0 0 0 var(--border-2);
  padding: var(--space-4);
  min-height: 0;
}
/* 悬浮滚动条：4px 胶囊 border-1、距右 4px、不占布局 */
.side-thumb {
  position: absolute;
  right: 4px;
  width: 4px;
  border-radius: 2px;
  background: var(--border-1);
  pointer-events: none;
  z-index: 10;
}
/* 组件布局 06.1：标题栏内容容器内边距 8 / 子项间距自适应 */
.bar-title {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-8);
}
.spacer {
  flex: 1;
}
/* 原生条由 base.css 全局隐藏，滚动条为悬浮 side-thumb（不占位） */
.view-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}
.ai-placeholder {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-8);
  color: var(--font-icon-1);
}
.ai-placeholder p {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
}
/* 组件布局 06.7：视图切换内容容器内边距 4 8 8 8 */
.bar-footer {
  padding: var(--space-4) var(--space-8) var(--space-8) var(--space-8);
}
</style>
