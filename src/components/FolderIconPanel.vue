<script setup lang="ts">
// 更改图标网格面板（2026-08-18，3.3 / 05 稿 §3）：FolderIconGrid（30 格 = 默认 + 29 自定义，
// 2026-09-01 二十五轮换 02_自定义 新双色集；archive 档案专属同日出清单）。
// 同步语义（用户当日定，与颜色同口径、无复选框）：父级改图标默认影响子级——
// 后代图标与本节点当前图标一致的跟随调整，不一致的（用户单独区分过）不动。
// 点击格 = 一次性任务类：执行后关菜单（done 由 ContextMenu 收口）。
import { computed } from "vue";
import FolderIconGrid from "./FolderIconGrid.vue";
import { setFolderIcon } from "../lib/placementOps";
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

// 当前图标回显（null = 默认文件夹图标，选中首格）
const current = computed(() => findNode(content.folders, props.folderId)?.icon ?? null);

async function pick(name: string | null): Promise<void> {
  if (await setFolderIcon(props.folderId, name)) emit("done");
}
</script>

<template>
  <FolderIconGrid :current="current" @select="pick" />
</template>
