<script setup lang="ts">
// 中栏标题栏（03_UI 布局/03_中间面板 02）：导航组（返回 / 前进 + 标题 + 搜索结果容器）
// / 滑块 / 显示子文件夹内容 / 按钮组——四组 space-between 等距。
// 返回前进阶段 1 无导航历史，禁用态 C4-20。
import { ref } from "vue";
import IconButton from "./IconButton.vue";
import SideSlider from "./SideSlider.vue";
import AppCheckbox from "./AppCheckbox.vue";
import { useUiStore } from "../stores/ui";

const ui = useUiStore();

// 搜索结果容器：搜索后从左侧滑入、取消后往左滑出（阶段 1 演示——点击标题模拟搜索 / 撤销，阶段 4 接真搜索）
const showResult = ref(false);
</script>

<template>
  <div class="content-titlebar" data-tauri-drag-region>
    <div class="nav-group">
      <div class="btn-group">
        <IconButton name="chevron-large" title="返回" class="rot-ccw" disabled />
        <IconButton name="chevron-large" title="前进" class="rot-cw" disabled />
      </div>
      <button class="crumb-text" title="演示：点击切换搜索结果" @click="showResult = !showResult">
        全部
      </button>
      <Transition name="slide-x">
        <span v-if="showResult" class="result-text">/ 搜索结果 (123)</span>
      </Transition>
    </div>
    <SideSlider v-model="ui.targetRowHeight" :min="80" :max="ui.gridWidth" />
    <AppCheckbox v-model="ui.showSubfolders" label="显示子文件夹内容" />
    <div class="btn-group">
      <IconButton name="plugin" title="插件" />
      <IconButton name="view-options" title="查看方式" />
    </div>
  </div>
</template>

<style scoped>
/* 总容器：内边距 8 / 四组 space-between 等距 */
.content-titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-8);
  flex: none;
}
/* 面包屑导航容器：返回前进按钮组（间距 4）+ 标题容器（间距 4） */
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
  border-radius: var(--radius-6);
}
/* 搜索结果容器：/ + 搜索结果 + (数量)，正文 C4-40 */
.result-text {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--c4-40);
  white-space: nowrap;
}
/* 从左侧滑入 / 往左滑出 */
.slide-x-enter-active,
.slide-x-leave-active {
  transition:
    transform 0.18s ease,
    opacity 0.18s ease;
}
.slide-x-enter-from,
.slide-x-leave-to {
  transform: translateX(-16px);
  opacity: 0;
}
</style>
