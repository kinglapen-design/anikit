<script setup lang="ts">
// 更改颜色网格面板（2026-08-18，3.3 / 05 稿 §4）：10 格 = 默认（C5-底色，清除自定义色）
// + CUSTOM_COLORS 9 色，5 列网格（item 31×31 圆角 4，列 gap 8 行 gap 12）；
// 当前项选中 = 2px 文字/常态 border（border-box 内缩不撑布局，稿首格即选中样例）；
// hover 叠暗-悬停（保留色底，--cell-bg 机制同 TreeItem --tint-bg），选中项 hover 不再变色。
// 同步语义（2026-08-18 用户定，与图标同口径、无复选框）：父级改色默认影响子级——
// 后代颜色与本节点旧链路色一致的跟随调整，不一致的（用户单独区分过）不动；
// 无色后代不写数据（视觉自动跟随 = 该色底色版本 --custom-{色}-bg，由树渲染承担）。
// 点击格 = 一次性任务类：执行后关菜单（done 由 ContextMenu 收口）。
import { computed } from "vue";
import { CUSTOM_COLORS } from "../lib/customColors";
import { setFolderColor } from "../lib/placementOps";
import { useContentStore, type FolderNode } from "../stores/content";

const props = defineProps<{ folderId: string }>();
const emit = defineEmits<{ done: [] }>();
const content = useContentStore();

function findNode(nodes: FolderNode[], id: string): FolderNode | null {
  for (const n of nodes) {
    if (n.id === id) return n;
    const hit = findNode(n.children, id);
    if (hit) return hit;
  }
  return null;
}

// 当前颜色回显（null = 未自定义，选中首格）
const current = computed(() => findNode(content.folders, props.folderId)?.color ?? null);

async function pick(tint: string | null): Promise<void> {
  if (await setFolderColor(props.folderId, tint)) emit("done");
}
</script>

<template>
  <div class="color-grid">
    <button class="cell default" :class="{ cur: current === null }" aria-label="默认" @click="pick(null)"></button>
    <button
      v-for="c in CUSTOM_COLORS"
      :key="c.tint"
      class="cell"
      :class="{ cur: current === c.tint }"
      :style="{ '--cell-bg': `var(--custom-${c.tint})` }"
      :aria-label="c.label"
      @click="pick(c.tint)"
    ></button>
  </div>
</template>

<style scoped>
/* 5 列网格（05 稿 §4：item 31×31、列 gap 8、行 gap 12） */
.color-grid {
  display: grid;
  grid-template-columns: repeat(5, 31px);
  gap: var(--space-12) var(--space-8);
}
.cell {
  width: 31px;
  height: 31px;
  box-sizing: border-box; /* 选中 border 内缩不撑布局（稿 box-sizing: border-box） */
  border-radius: var(--radius-4);
  background: var(--cell-bg);
}
.cell.default {
  background: var(--c5-base); /* 稿 §4 首格 = C5-底色（清除自定义色） */
}
/* hover 叠 暗-悬停（保留色底）；选中 = 2px 文字/常态 border（稿首格样例），选中项 hover 不再变色 */
.cell:hover:not(.cur) {
  background: linear-gradient(var(--line-hover), var(--line-hover)), var(--cell-bg);
}
.cell.default:hover:not(.cur) {
  background: linear-gradient(var(--line-hover), var(--line-hover)), var(--c5-base);
}
.cell.cur {
  border: 2px solid var(--text-normal);
}
</style>
