<script setup lang="ts">
// 预览态标题栏（4.3/5.2）：返回 + 索引 x/y / 缩放滑块+百分比 / 工具按钮组。
// 布局模式照 ContentTitleBar（space-between 等距、中栏图标 80%）。
// image/sequence/anim/video：缩放组 + 适合窗口 + 原图大小（源文件 PSD·AI·SVG 去旋转翻转、加默认应用打开；
// anim/video 无默认应用打开）；audio：仅返回、索引、插件、新窗口打开、上/下一个；
// other：返回、索引、插件、默认应用打开、新窗口打开、上/下一个。
// window 模式（新窗口打开）：返回 + 索引替换为文件名标题，加置顶与完整窗口控制按钮（PRD 4.3）。
import { computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import IconButton from "./IconButton.vue";
import SideSlider from "./SideSlider.vue";
import WindowControls from "./WindowControls.vue";
import { usePreviewStore } from "../stores/preview";
import { useUiStore } from "../stores/ui";

withDefaults(defineProps<{ window?: boolean }>(), { window: false });

const preview = usePreviewStore();
const ui = useUiStore();
const appWindow = getCurrentWindow();

// 滑块 = 缩放百分比（2%~800% 线性），瞄点在视图中心（2026-07-25 定案）
const zoomPct = computed({
  get: () => Math.round(preview.zoom * 100),
  set: (v: number) => preview.zoomAt(v / 100, preview.viewW / 2, preview.viewH / 2),
});

// 新窗口打开（4.3）：同资产复用聚焦
async function openInNewWindow(): Promise<void> {
  if (!preview.assetKey) return;
  await invoke("open_preview_window", { assetKey: preview.assetKey });
}

async function togglePin(): Promise<void> {
  ui.alwaysOnTop = !ui.alwaysOnTop;
  await appWindow.setAlwaysOnTop(ui.alwaysOnTop);
}
</script>

<template>
  <div class="preview-titlebar" data-tauri-drag-region>
    <div class="nav-group">
      <template v-if="!window">
        <IconButton name="chevron-large" title="返回" class="rot-180" @click="preview.close()" />
        <span class="index-text">{{ preview.index + 1 }} / {{ preview.total }}</span>
      </template>
      <span v-else class="index-text">{{ preview.current?.name ?? "预览" }}</span>
    </div>
    <div v-if="preview.viewable" class="zoom-group">
      <SideSlider v-model="zoomPct" :min="2" :max="800" />
      <span class="zoom-text">{{ Math.round(preview.zoom * 100) }}%</span>
    </div>
    <div class="btn-group">
      <IconButton name="ai-tools" title="插件" />
      <template v-if="preview.viewable">
        <IconButton v-if="!preview.isSource" name="rotate-cw" title="旋转" @click="preview.rotateCw()" />
        <IconButton v-if="!preview.isSource" name="flip-h" title="水平翻转" @click="preview.toggleFlip()" />
        <IconButton name="fit-window" title="适合窗口" @click="preview.fit()" />
        <IconButton name="original-size" title="原图大小" @click="preview.actual()" />
      </template>
      <IconButton
        v-if="preview.isSource || preview.kind === 'other'"
        name="open-default"
        title="默认应用打开"
        @click="preview.openDefaultApp()"
      />
      <IconButton
        v-if="!window"
        name="open-new-window"
        title="新窗口打开"
        @click="openInNewWindow"
      />
      <!-- 上一个 / 下一个 = nav-history，与面包屑后退 / 前进同款（2026-08-20 用户定） -->
      <IconButton
        name="nav-history"
        title="上一个"
        :disabled="preview.index <= 0"
        @click="preview.step(-1)"
      />
      <IconButton
        name="nav-history"
        title="下一个"
        class="rot-180"
        :disabled="preview.index >= preview.total - 1"
        @click="preview.step(1)"
      />
      <template v-if="window">
        <IconButton
          :name="ui.alwaysOnTop ? 'window-pin-off' : 'window-pin-on'"
          title="窗口置顶"
          @click="togglePin"
        />
        <WindowControls />
      </template>
    </div>
  </div>
</template>

<style scoped>
.preview-titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  /* 内边距 12 16（与 ContentTitleBar 一致；2026-08-10 随 42 稿外层 padding 归零补右左 16） */
  padding: var(--space-12) var(--space-16);
  flex: none;
}
/* 中栏标题栏图标按钮 80%（2026-07-23 用户定） */
.preview-titlebar :deep(.icon-btn) {
  color: var(--font-icon-2);
}
.nav-group {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  min-width: 0;
}
.btn-group {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  flex: none;
}
.zoom-group {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  flex: none;
}
/* chevron-large 原图标朝右（18px 箭头）：返回 = 旋转 180° 朝左；
   上/下一个 = nav-history 与面包屑后退/前进同款，下一个旋转 180° 朝右（figma 41 口径）。
   2026-08-20 修：1.1 图标换血后 chevron-large 由朝下变朝右，旧 rot-ccw/rot-cw（±90°）约定
   失效——返回曾显示朝下、上/下一个曾显示 ↓↑。 */
.rot-180 :deep(svg) {
  transform: rotate(180deg);
}
.index-text {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  white-space: nowrap;
}
.zoom-text {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
  white-space: nowrap;
  min-width: 38px;
  text-align: right;
}
</style>
