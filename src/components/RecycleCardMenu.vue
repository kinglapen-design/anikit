<script setup lang="ts">
// 回收站卡片右键菜单（6.1 二轮，2026-07-29；复用 AppMenu + MenuItem，同 AssetCardMenu 约定）：
// 只读语义三入口——还原（restore_recycle，toast 带撤销）/ 彻底删除（两段式确认，无撤销，
// Del 同语义）/ 清空回收站（两段式，回收站为空时禁用）。必须 Teleport to body——
// AssetGrid 有 transform 祖先，position:fixed 会相对祖先定位（同 AssetCardMenu 注释坑）；
// 右键未选中项由 AssetGrid 先单选再开菜单（与普通视图同口径）。
import AppMenu from "./AppMenu.vue";
import MenuItem from "./MenuItem.vue";
import MenuDivider from "./MenuDivider.vue";
import {
  clearRecycle,
  deleteRecycle,
  recycleSelectedAssetIds,
  restoreRecycle,
} from "../lib/placementOps";
import { useContentStore } from "../stores/content";

defineProps<{ x: number; y: number }>();
const emit = defineEmits<{ close: [] }>();
const content = useContentStore();

async function onRestore(): Promise<void> {
  emit("close");
  await restoreRecycle(recycleSelectedAssetIds());
}
async function onDelete(): Promise<void> {
  emit("close");
  await deleteRecycle(recycleSelectedAssetIds()); // needConfirm → 确认窗（RecycleConfirmDialog）
}
async function onClear(): Promise<void> {
  emit("close");
  await clearRecycle();
}
</script>

<template>
  <Teleport to="body">
    <AppMenu :x="x" :y="y" @close="emit('close')">
      <MenuItem label="还原" @select="onRestore" />
      <MenuItem label="彻底删除" shortcut="Del" @select="onDelete" />
      <MenuDivider />
      <MenuItem
        label="清空回收站"
        :disabled="content.recycleEntries.length === 0"
        @select="onClear"
      />
    </AppMenu>
  </Teleport>
</template>
