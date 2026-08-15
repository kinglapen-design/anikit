<script setup lang="ts">
// 编辑数据区块面板（22_展开菜单 - 编辑数据区块，3.2 一轮 8 区块体系）：
// 面板机制同 ViewOptionsMenu（AppMenu 定位 / 外点关闭 / Esc 关闭，右缘与原 175 菜单位置对齐）；
// 404 宽 / C2 / 1.5px 亮-选中真 border（速查表 §1，覆盖 AppMenu 默认 inset 画框，选择器提权 §6）/
// 圆角 8 / 投影 token / padding 12 24；8 行 × 52 + 上下 padding = 440 高（稿值，内容自动撑出）。
// 行（52 高 / gap 8 / 行底 1px 暗-悬停，末行去线 §7）：下移箭头 + 上移箭头
//（IconButton chevron-small：下移在前旋转 180、上移在后；首行禁上移 / 末行禁下移，
// 禁用态 --line-active——实现与 specificity 处理照抄 BlockManageRow）+ 标题（14/20 常态；
// visible=false 行标题禁用色）+ 眼睛（eye-open / eye-closed 18px，点击 toggle）。
// 操作即时生效（写 ui store，防抖持久化既有机制自动覆盖）。
// 行拖拽排序（2026-08-15 走查 9 项）：与侧栏区块标题同款让位机制（LibraryView blockDrag 范式，
// 本地 pointer 追踪不走拖拽引擎）——行高恒定 52，步进 = 52；拖动行跟随指针无过渡 + C2 底盖缝，
// 被跨越行反向让位 0.18s 过渡；松手写 ui.moveDetailSectionTo，归位帧禁过渡防"弹回去"；
// 上下箭头保留（22 稿原有）。箭头 / 眼睛上不启动拖拽（closest .icon-btn 豁免）。
import { computed, reactive, ref } from "vue";
import AppMenu from "./AppMenu.vue";
import IconButton from "./IconButton.vue";
import { detailSectionTitle } from "../lib/detailSections";
import { useUiStore } from "../stores/ui";

const ui = useUiStore();
const props = defineProps<{ anchor: { x: number; y: number }; trigger?: HTMLElement | null }>();
const emit = defineEmits<{ close: [] }>();

const WINDOW_W = 404;
// 原级联菜单宽 175：窗口加宽后右缘与原位置对齐（不超出窗口右缘，同 ViewOptionsMenu 口径）
const winX = computed(() => props.anchor.x - (WINDOW_W - 175));

// —— 行拖拽排序（让位式，行高恒定） ——
const ROW_H = 52;
const rowDrag = reactive({ id: "", startY: 0, dy: 0, from: -1, to: -1 });
const rowSettling = ref(false); // 松手归位帧：禁行 transform 过渡

function onRowPointerDown(id: string, i: number, e: PointerEvent): void {
  if (e.button !== 0) return;
  if ((e.target as HTMLElement).closest(".icon-btn")) return; // 箭头 / 眼睛不启动拖拽
  Object.assign(rowDrag, { id, startY: e.clientY, dy: 0, from: i, to: i });
  window.addEventListener("pointermove", onRowDragMove);
  window.addEventListener("pointerup", onRowDragEnd, { once: true });
  window.addEventListener("blur", onRowDragCancel, { once: true });
  document.addEventListener("mouseleave", onRowDragCancel, { once: true });
}

// 拖动行中线越过邻行中线 → 目标位迁移（行高恒定，dy 按 52 步进取整即等价）
function onRowDragMove(e: PointerEvent): void {
  if (!rowDrag.id) return;
  if ((e.buttons & 1) === 0) {
    onRowDragCancel(); // 僵尸自清：pointerup 丢失（窗外松手等，与引擎同口径）
    return;
  }
  rowDrag.dy = e.clientY - rowDrag.startY;
  const max = ui.detailSections.length - 1;
  rowDrag.to = Math.max(0, Math.min(max, rowDrag.from + Math.round(rowDrag.dy / ROW_H)));
}

function onRowDragEnd(): void {
  offRowDrag();
  const { id, from, to } = rowDrag;
  rowDrag.id = "";
  if (from < 0 || to === from) return;
  ui.moveDetailSectionTo(id, to);
  // 松手即到位：让位行视觉本已在目标位，清 transform 必须禁过渡（库菜单 2026-08-12 走查同坑）
  rowSettling.value = true;
  setTimeout(() => (rowSettling.value = false), 50);
}

function onRowDragCancel(): void {
  offRowDrag();
  rowDrag.id = "";
}

function offRowDrag(): void {
  window.removeEventListener("pointermove", onRowDragMove);
  window.removeEventListener("pointerup", onRowDragEnd);
  window.removeEventListener("blur", onRowDragCancel);
  document.removeEventListener("mouseleave", onRowDragCancel);
}

// 让位位移：拖动行跟随指针（无过渡、置顶、C2 底盖缝）；被跨越行反向让出一步
//（transition 在 .se-row 根样式，归位帧 .settling 禁过渡）
function rowStyle(i: number): string {
  if (!rowDrag.id) return "";
  if (i === rowDrag.from) {
    return `transform: translateY(${rowDrag.dy}px); transition: none; position: relative; z-index: 2; background: var(--c2);`;
  }
  if (rowSettling.value) return "";
  const { from, to } = rowDrag;
  if (from < to && i > from && i <= to) return `transform: translateY(${-ROW_H}px)`;
  if (to < from && i >= to && i < from) return `transform: translateY(${ROW_H}px)`;
  return "";
}
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
    :class="{ settling: rowSettling }"
    @close="emit('close')"
  >
    <div
      v-for="(s, i) in ui.detailSections"
      :key="s.id"
      class="se-row"
      :class="{ off: !s.visible }"
      :style="rowStyle(i)"
      @pointerdown="onRowPointerDown(s.id, i, $event)"
    >
      <!-- 排序箭头（22 稿：下移在前旋转 180、上移在后；首行禁上移 / 末行禁下移） -->
      <IconButton
        small
        name="chevron-small"
        class="sort-arrow arrow-down"
        :disabled="i === ui.detailSections.length - 1"
        title="下移"
        @click="ui.moveDetailSection(s.id, 1)"
      />
      <IconButton
        small
        name="chevron-small"
        class="sort-arrow arrow-up"
        :disabled="i === 0"
        title="上移"
        @click="ui.moveDetailSection(s.id, -1)"
      />
      <span class="row-title">{{ detailSectionTitle(s.id) }}</span>
      <IconButton
        :name="s.visible ? 'eye-open' : 'eye-closed'"
        :title="s.visible ? '隐藏区块' : '显示区块'"
        class="eye"
        @click="ui.toggleDetailSection(s.id)"
      />
    </div>
  </AppMenu>
</template>

<style scoped>
/* 22 稿壳：1.5px 亮-选中真 border（速查表 §1）+ 投影 token——覆盖 AppMenu 默认 inset 画框，
   选择器提权压过组件自带 .app-menu（§6 口径） */
.section-edit.app-menu {
  border: var(--border-strong) solid var(--line-active);
  box-shadow: var(--shadow-pop);
}
/* 行（22 稿）：52 高 / padding 12 0 / gap 8 / 行底 1px 暗-悬停，末行去线（速查表 §7）；
   transform 过渡 = 行拖拽让位动画（拖动行 inline 禁过渡跟随指针）；整行可拖（抓手光标） */
.se-row {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  height: 52px;
  box-sizing: border-box;
  padding: var(--space-12) 0;
  border-bottom: 1px solid var(--line-hover);
  flex: none;
  cursor: grab;
  transition: transform 0.18s ease;
}
/* 松手归位帧：让位行视觉本已在目标位，清 transform 禁过渡防"弹回去" */
.section-edit.settling .se-row {
  transition: none;
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
