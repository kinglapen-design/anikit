<script setup lang="ts">
// 右侧面板（1.2 重构，figma-css 01_主窗口 右侧面板段）：
// 标题栏 46（C3 + 下边框：窗口控制组由 App.vue 右上角浮块呈现，本行留作窗口拖拽区）；
// 工具栏 54（收起详情面板 + 编辑数据区块[22 稿面板，2026-08-15 点亮]，figma 12263 原位）；
// 数据展示 = DetailPanel（8 区块体系，2026-08-15 一轮）。
// 收展 = 常驻挂载 + 宽度由 App.vue rAF 驱动逐帧写入（v6，同 SidebarLeft）；窗口控制组在 App 层
// 浮块恒在窗口右上角，不参与收展（2026-08-08 用户定：解决窗口控制随收展跳变）。
import { ref } from "vue";
import IconButton from "./IconButton.vue";
import DetailPanel from "./DetailPanel.vue";
import PickerPanel from "./PickerPanel.vue";
import SectionEditPanel from "./SectionEditPanel.vue";
import { usePickerStore } from "../stores/picker";
import { useUiStore } from "../stores/ui";
import { useSideThumb } from "../lib/useSideThumb";
import { sidebarShowWidths } from "../lib/useSidebarAnim";

const ui = useUiStore();
const picker = usePickerStore();
const bodyRef = ref<HTMLElement>();
const thumb = useSideThumb(bodyRef);
// 滚动条悬停显隐（2026-08-10 用户定：悬停面板 / 拖拽 thumb 时才出现）
const hover = ref(false);

// 22 稿「编辑数据区块」面板：按钮右缘对齐、下方展开；再次点击按钮收起（toggle，同 ViewOptionsMenu 口径）
const sectionAnchor = ref({ x: 0, y: 0 });
const sectionBtnRef = ref<HTMLElement>();
function toggleSectionEdit(e: MouseEvent): void {
  if (ui.sectionEditOpen && ui.sectionEditHost === "right") {
    ui.sectionEditOpen = false;
    return;
  }
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
  sectionAnchor.value = { x: rect.right - 175, y: rect.bottom + 4 };
  ui.sectionEditHost = "right";
  ui.sectionEditOpen = true;
}
</script>

<template>
  <aside
    class="sidebar-right"
    :style="{ width: `${sidebarShowWidths.right}px`, '--sb-w': `${ui.rightWidth}px` }"
    @mouseenter="hover = true"
    @mouseleave="hover = false"
  >
    <!-- 标题栏 46：窗口控制组由 App.vue 右上角浮块呈现，本行仅作窗口拖拽区 -->
    <header class="bar-title" data-tauri-drag-region />
    <!-- 选择模式（6.2 一轮）：右侧详情栏整体临时替换为备选区（PRD 03_数据链接 04 四重标识之一）；
         工具栏 / 详情面板 / 22 稿面板本模式不渲染（编辑类，与"右键仅查看类"同口径） -->
    <PickerPanel v-if="picker.active" />
    <template v-else>
      <!-- 工具栏（figma 12263，54 / padding 12 16）：收起详情面板 + 编辑数据区块 -->
      <div class="panel-toolbar">
        <IconButton name="collapse-detail" title="收起详情面板" @click="ui.rightCollapsed = true" />
        <span ref="sectionBtnRef" style="display: contents">
          <IconButton name="edit-data-section" title="编辑数据区块" @click="toggleSectionEdit" />
        </span>
      </div>
      <div class="side-content">
        <div ref="bodyRef" class="detail-body">
          <DetailPanel />
        </div>
        <span
          class="side-thumb"
          :class="{ visible: thumb.show && (hover || thumb.dragging) }"
          :style="{ top: `${thumb.top}px`, height: `${thumb.height}px` }"
          @pointerdown="thumb.onThumbDown"
        />
      </div>
      <!-- 22 稿「编辑数据区块」面板（会话级 ui.sectionEditOpen，机制同 ViewOptionsMenu；
           档案侧栏工具行铅笔同开此面板（host = archive 时归那边渲染） -->
      <SectionEditPanel
        v-if="ui.sectionEditOpen && ui.sectionEditHost === 'right'"
        :anchor="sectionAnchor"
        :trigger="sectionBtnRef ?? null"
        @close="ui.sectionEditOpen = false"
      />
    </template>
  </aside>
</template>

<style scoped>
/* 右侧面板（figma 11967）：column，宽度可拖（SplitBar，默认 250）。
   收展 = 常驻挂载 + 宽度由 App.vue rAF 驱动逐帧写入（同 SidebarLeft 的 v6 定案；
   本文件禁挂 width 的 CSS transition）；子块定宽 --sb-w + align-end =
   收窄时内容锚右缘、左缘裁剪（向右滑出观感），不挤压 */
.sidebar-right {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  flex: none;
  min-height: 0;
  overflow: hidden;
  background: var(--c3);
}
/* 标题栏（figma 11986）：46（含边框，box-sizing——与中间标签栏行对齐）/ 下边框 暗-悬停 /
   padding 8 12——窗口控制组由 App.vue 右上角浮块呈现（收展不联动），本行仅作窗口拖拽区。
   注意：标题栏行无左边框（figma 分隔线从内容区起，2026-08-08 走查修正） */
.bar-title {
  width: var(--sb-w); /* 定宽：aside 收窄时裁剪不挤压 */
  height: 46px;
  box-sizing: border-box;
  flex: none;
  border-bottom: 1px solid var(--line-hover);
}
/* 工具栏（figma 12263）：54（含边框）/ padding 12 16 / 间距 4 / 下边框 + 左边框 暗-悬停 */
.panel-toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  width: var(--sb-w);
  height: 54px;
  box-sizing: border-box;
  padding: var(--space-12) var(--space-16);
  flex: none;
  border-bottom: 1px solid var(--line-hover);
  border-left: 1px solid var(--line-hover);
}
/* 详情内容区（figma 12239 左边框 暗-悬停 = 与中间面板分隔线） */
.side-content {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1;
  width: var(--sb-w);
  min-height: 0;
  border-left: 1px solid var(--line-hover);
}
.detail-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
}
/* 悬浮滚轮条（2026-08-10 用户定二轮：6px 胶囊紧贴面板右边框 / 实色 常态 C4、悬停 C5——
   透明色在通栏分割线上交叉叠加、4px 间距让分隔线视觉截断，均作废）；
   悬停显隐：默认透明不接收事件，visible（悬停面板 / 拖拽中）淡入 */
.side-thumb {
  position: absolute;
  right: 0;
  width: 6px;
  border-radius: var(--radius-pill);
  background: var(--c4);
  z-index: 101;
  opacity: 0;
  pointer-events: none;
  transition:
    opacity 0.15s ease,
    background 0.12s ease;
}
.side-thumb.visible {
  opacity: 1;
  pointer-events: auto;
}
.side-thumb:hover {
  background: var(--c5);
}
</style>
