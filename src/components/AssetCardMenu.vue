<script setup lang="ts">
// 素材卡片右键菜单（6.1 C6，2026-07-28 用户定案；最小集，其余菜单项 6.3 补设计）：
// 重命名（单选可用，F2 同语义）/ 粘贴（内部剪贴板非空可用 → 弹关联弹窗 C5）/
// 粘贴为副本（Ctrl+Shift+V 同语义，不弹窗）/ 删除（Del 同语义，两段式）。
// 必须 Teleport to body——AssetGrid 有 transform 祖先，position:fixed 会相对祖先定位
//（同 InboxDiskSection.vue 注释坑）；右键未选中项由 AssetGrid 先单选再开菜单。
import AppMenu from "./AppMenu.vue";
import MenuItem from "./MenuItem.vue";
import MenuDivider from "./MenuDivider.vue";
import {
  deletePlacements,
  pasteClipboard,
  pasteDuplicate,
} from "../lib/placementOps";
import { useSelectionStore } from "../stores/selection";

defineProps<{ x: number; y: number }>();
const emit = defineEmits<{ close: [] }>();
const selection = useSelectionStore();

function onRename(): void {
  emit("close");
  if (selection.count === 1) selection.renamingKey = selection.first; // 就地重命名（C2）
}
async function onPaste(): Promise<void> {
  emit("close");
  await pasteClipboard(); // copy → 关联弹窗 / cut → 移动（仲裁在 ops 内）
}
async function onPasteDuplicate(): Promise<void> {
  emit("close");
  await pasteDuplicate();
}
async function onDelete(): Promise<void> {
  emit("close");
  await deletePlacements(selection.selectedRefs());
}
</script>

<template>
  <Teleport to="body">
    <AppMenu :x="x" :y="y" @close="emit('close')">
      <MenuItem label="重命名" shortcut="F2" :disabled="selection.count !== 1" @select="onRename" />
      <MenuDivider />
      <MenuItem label="粘贴" shortcut="Ctrl+V" :disabled="!selection.clipboard" @select="onPaste" />
      <MenuItem
        label="粘贴为副本"
        shortcut="Ctrl+Shift+V"
        :disabled="!selection.clipboard"
        @select="onPasteDuplicate"
      />
      <MenuDivider />
      <MenuItem label="删除" shortcut="Del" @select="onDelete" />
    </AppMenu>
  </Teleport>
</template>
