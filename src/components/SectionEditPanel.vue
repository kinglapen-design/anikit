<script setup lang="ts">
// 编辑数据区块面板（22_展开菜单 - 编辑数据区块，3.2 一轮区块体系）：
// 面板机制同 ViewOptionsMenu（AppMenu 定位 / 外点关闭 / Esc 关闭，右缘与原 175 菜单位置对齐）；
// 404 宽 / C2 / 1.5px 亮-选中真 border（速查表 §1，覆盖 AppMenu 默认 inset 画框，选择器提权 §6）/
// 圆角 8 / 投影 token / padding 12 24；7 行 × 52 + 上下 padding = 388 高（稿值等比）。
// 行（52 高 / gap 8 / 行底 1px 暗-悬停，末行去线 §7）：下移箭头 + 上移箭头
//（IconButton chevron-small：下移在前旋转 180、上移在后；首行禁上移 / 末行禁下移，
// 禁用态 --line-active——实现与 specificity 处理照抄 BlockManageRow）+ 标题（14/20 常态；
// visible=false 行标题禁用色）+ 眼睛（eye-open / eye-closed 18px，点击 toggle）。
// 排序 = 仅上下箭头（2026-08-15 用户定：面板内行拖拽拆除，详情面板内拖拽保留在 DetailPanel）；
// 箭头交换动画 = TransitionGroup move 钩子（30 稿 BlockManageDialog 同款 0.18s 位移过渡，
// 行高 52 恒定同参数）。
// 操作即时生效（写 ui store，防抖持久化既有机制自动覆盖）。
// 文件 / 档案各一套配置（2026-08-23 用户定，原共用一份联动改动互串）：本面板编辑哪套 =
// 跟随当前选中——选中档案卡片 = 档案套，选中文件 / 无选中 = 文件套（与 DetailPanel 档案判定
// 同逻辑：文件夹选择域首项 kind=archive）；档案侧栏入口随面板删除（2026-08-30）撤除
import { computed } from "vue";
import AppMenu from "./AppMenu.vue";
import IconButton from "./IconButton.vue";
import { detailSectionTitle, findFolderNode } from "../lib/detailSections";
import { useUiStore } from "../stores/ui";
import { useContentStore } from "../stores/content";
import { useSelectionStore } from "../stores/selection";

const ui = useUiStore();
const content = useContentStore();
const selection = useSelectionStore();
const props = defineProps<{ anchor: { x: number; y: number }; trigger?: HTMLElement | null }>();
const emit = defineEmits<{ close: [] }>();

const WINDOW_W = 404;
// 原级联菜单宽 175：窗口加宽后右缘与原位置对齐（不超出窗口右缘，同 ViewOptionsMenu 口径）
const winX = computed(() => props.anchor.x - (WINDOW_W - 175));

// 本面板编辑的配置套（文件 / 档案独立，2026-08-23）：跟随当前选中——选中档案卡片 = 档案套，
// 选中文件 / 无选中 = 文件套（与 DetailPanel 档案判定同逻辑：文件夹选择域首项 kind=archive）
const host = computed<"file" | "archive">(() => {
  for (const id of selection.selectedFolders) {
    const n = findFolderNode(content.folders, id);
    return n?.kind === "archive" ? "archive" : "file";
  }
  return "file";
});
const rows = computed(() => ui.sectionsFor(host.value));
</script>

<template>
  <AppMenu
    :x="winX"
    :y="anchor.y"
    :width="WINDOW_W"
    pad="var(--space-12) var(--space-24)"
    gap="0"
    :trigger="trigger ?? null"
    class="section-edit"
    @close="emit('close')"
  >
    <!-- TransitionGroup：箭头排序时两行交换位置给 0.18s 位移过渡（30 稿 BlockManageDialog 同款） -->
    <TransitionGroup name="se-swap" tag="div" class="se-rows">
      <div
        v-for="(s, i) in rows"
        :key="s.id"
        class="se-row"
        :class="{ off: !s.visible }"
      >
        <!-- 排序箭头（22 稿：下移在前旋转 180、上移在后；首行禁上移 / 末行禁下移） -->
        <IconButton
          small
          name="chevron-small"
          class="sort-arrow arrow-down"
          :disabled="i === rows.length - 1"
          title="下移"
          @click="ui.moveDetailSection(s.id, 1, host)"
        />
        <IconButton
          small
          name="chevron-small"
          class="sort-arrow arrow-up"
          :disabled="i === 0"
          title="上移"
          @click="ui.moveDetailSection(s.id, -1, host)"
        />
        <span class="row-title">{{ detailSectionTitle(s.id) }}</span>
        <IconButton
          :name="s.visible ? 'eye-open' : 'eye-closed'"
          :title="s.visible ? '隐藏区块' : '显示区块'"
          class="eye"
          @click="ui.toggleDetailSection(s.id, host)"
        />
      </div>
    </TransitionGroup>
  </AppMenu>
</template>

<style scoped>
/* 22 稿壳：1.5px 亮-选中真 border（速查表 §1）+ 投影 token——覆盖 AppMenu 默认 inset 画框，
   选择器提权压过组件自带 .app-menu（§6 口径） */
.section-edit.app-menu {
  border: var(--border-strong) solid var(--line-active);
  box-shadow: var(--shadow-pop);
}
/* 排序交换动画（30 稿同款）：TransitionGroup move 钩子量位差补过渡 */
.se-swap-move {
  transition: transform 0.18s ease;
}
/* 行（22 稿）：52 高 / padding 12 0 / gap 8 / 行底 1px 暗-悬停，末行去线（速查表 §7） */
.se-row {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  height: 52px;
  box-sizing: border-box;
  padding: var(--space-12) 0;
  border-bottom: 1px solid var(--line-hover);
  flex: none;
}
.se-row:last-child {
  border-bottom: none;
}
/* 下箭头：chevron-small 源图形朝上，旋转 180 朝下（BlockManageRow 同款） */
.arrow-down :deep(svg) {
  transform: rotate(180deg);
}
/* 排序箭头（BlockManageRow 同款）：可用态高亮档；禁用态 --line-active 原样呈现
   （取消 IconButton 自带半透明）。选择器压过 .icon-btn.small / .icon-btn:disabled */
.se-row .icon-btn.sort-arrow {
  color: var(--text-highlight);
}
.se-row .icon-btn.sort-arrow:disabled {
  color: var(--line-active);
  opacity: 1;
}
/* 标题（22 稿）：正文 14/20 常态色，弹性占满；隐藏行标题禁用色（稿注"眼睛切换标题要禁用"） */
.row-title {
  flex: 1;
  min-width: 0;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.se-row.off .row-title {
  color: var(--line-active);
}
/* 眼睛（22 稿：18px 图标）：常态色 */
.se-row .icon-btn.eye {
  color: var(--text-normal);
}
</style>
