<script setup lang="ts">
// 更改图标网格面板（2026-08-18，3.3 / 05 稿 §3）：FolderIconGrid（20 格）+ 底部「应用至子级」
// 复选框（稿未画，用户 2026-08-18 要求补）：勾选 = 全部非档案后代统一换（PRD 02-45）；
// 每次打开面板默认不勾。点击格 = 一次性任务类：执行后关菜单（done 由 ContextMenu 收口）。
import { computed, ref } from "vue";
import AppCheckbox from "./AppCheckbox.vue";
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
const applyChildren = ref(false);

async function pick(name: string | null): Promise<void> {
  if (await setFolderIcon(props.folderId, name, applyChildren.value)) emit("done");
}
</script>

<template>
  <div class="folder-icon-panel">
    <FolderIconGrid :current="current" @select="pick" />
    <AppCheckbox v-model="applyChildren" label="应用至子级" />
  </div>
</template>

<style scoped>
.folder-icon-panel {
  display: flex;
  flex-direction: column;
  gap: var(--space-12);
}
</style>
