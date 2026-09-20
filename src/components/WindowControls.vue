<script setup lang="ts">
// 窗口控制组：最小化 / 最大化 / 关闭（关闭由 Rust 侧 CloseRequested 拦截转托盘；
// 关闭按钮 hover 错误色）。vertical 用于右栏折叠窄条内的竖排。
import { getCurrentWindow } from "@tauri-apps/api/window";
import IconButton from "./buttons/IconButton.vue";

withDefaults(defineProps<{ vertical?: boolean }>(), { vertical: false });
const appWindow = getCurrentWindow();
</script>

<template>
  <div class="window-controls" :class="{ vertical }">
    <IconButton name="window-minimize" title="最小化" @click="appWindow.minimize()" />
    <IconButton name="window-maximize" title="最大化" @click="appWindow.toggleMaximize()" />
    <IconButton name="window-close" title="关闭" @click="appWindow.close()" />
  </div>
</template>

<style scoped>
.window-controls {
  display: flex;
  gap: var(--space-4);
}
.window-controls.vertical {
  flex-direction: column;
  align-items: center;
}
</style>
