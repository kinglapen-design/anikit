<script setup lang="ts">
// 列表式列头钉顶壳（7.1 内容容器轮改造）：视觉芯 = cards/ListHeader（高 34 / padding 8/12 /
// 列 123 与列表行对齐——通用三级底 + 立体感-通用），本壳承担钉顶定位与排序接线——
// 排序业务 = 3.5 已有（lib/sortPlacements + ui.sortBy/sortAsc，ViewOptionsMenu 同径直写）：
// 点当前排序列 = 翻转升降；点新列 = 换键升序；尺寸列无 SortKey 档纯展示（内容容器轮定）。
// 列头随内容滚动至顶部后钉住（自定义 transform 滚动无 CSS sticky，AssetGrid 手动 top 驱动）。
import { computed } from "vue";
import ListHeaderCore from "./cards/ListHeader.vue";
import { useUiStore } from "../stores/ui";
import type { SortKey } from "../lib/sortPlacements";

const ui = useUiStore();

// 列 key ↔ SortKey 映射（尺寸列无键不排序——cards/ListHeader 已拦 emit，此处双闸）
const COL_TO_SORT: Record<string, SortKey> = { name: "name", ext: "ext", size: "size", mtime: "modifiedAt" };
const SORT_TO_COL = computed(() => {
  const inv: Record<string, string> = {};
  for (const [col, sk] of Object.entries(COL_TO_SORT)) inv[sk] = col;
  return inv;
});
const sortKey = computed(() => SORT_TO_COL.value[ui.sortBy] ?? "");
const sortDir = computed<"asc" | "desc">(() => (ui.sortAsc ? "asc" : "desc"));
function onSort(col: string): void {
  const sk = COL_TO_SORT[col];
  if (!sk) return;
  if (ui.sortBy === sk) ui.sortAsc = !ui.sortAsc;
  else {
    ui.sortBy = sk;
    ui.sortAsc = true;
  }
}
</script>

<template>
  <!-- 钉顶壳：absolute 满宽（左 12 = PAD.left / 右缘 12 = 网格 PAD.right，与行 box 左右缘齐——列头列与行列严格对齐）；
       top 由 AssetGrid :style 驱动（滚过顶钉住） -->
  <div class="list-header">
    <ListHeaderCore :sort-key="sortKey" :sort-dir="sortDir" @sort="onSort" />
  </div>
</template>

<style scoped>
/* 钉顶壳（7.1 换代）：列头满宽区左 12（PAD.left，2026-10-15 入布局）右 12 对齐行缘
   （旧 padding 右 24 对齐口径随新件同构列构废除）；高 34 = 芯件稿值（旧 46 废除）；
   滚动条区由 PAD.right 承担与网格卡同口径 */
.list-header {
  position: absolute;
  top: 0;
  left: 12px;
  right: 12px;
  z-index: 5;
}
</style>
