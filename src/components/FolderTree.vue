<script setup lang="ts">
// 文件夹树（递归）：展开 / 折叠状态每层自管，选中态写入 content store（阶段 1 仅样式）。
// 缩进线（02_左侧边栏 04）：竖线为子层容器级连续线（从子项展开占位中心 x5 贯穿整层，
// 穿过有子项的行与 item 间距，止于最后子项中心）；横线由叶子 TreeItem 自画（x5→x9）。
// 新建文件夹"命名中"临时节点（6.1 C3，2026-07-28 定案）：前端临时态插入——确认名称后才调
// create_folder（后端无 rename_folder），取消则不入库；名称校验同创建库表单（Windows 规则，
// 违规输入框描边 --error + 提示行，确认时后端也会校验）。
// 树 item 右键（C3）：「新建同级文件夹 / 新建子文件夹」菜单宿主在 LibraryView（Teleport body）。
import { computed, ref } from "vue";
import TreeItem from "./TreeItem.vue";
import { dragView } from "../lib/dragOperation";
import { createFolder, validateWindowsName } from "../lib/placementOps";
import { useContentStore, type FolderNode } from "../stores/content";
import { useSelectionStore } from "../stores/selection";

const props = withDefaults(
  defineProps<{
    nodes: FolderNode[];
    depth?: number;
    parentId?: string | null; // 本级节点列表的父文件夹（根级 = null），命名节点落点判定用
    forceOpenIds?: Set<string>; // 命名节点的祖先链强制展开（新建子文件夹时父级可能折叠）
  }>(),
  { depth: 0, parentId: null, forceOpenIds: undefined },
);
const content = useContentStore();
const selection = useSelectionStore();

const openSet = ref(new Set<string>());
function toggle(id: string): void {
  const next = new Set(openSet.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  openSet.value = next;
}
function isOpen(id: string): boolean {
  return openSet.value.has(id) || (props.forceOpenIds?.has(id) ?? false);
}

// 选中：有效拖拽结束后的紧随 click 不触发（C4 引擎置位）
function onSelect(id: string): void {
  if (dragView.suppressClick) return;
  content.sidebarSelected = `folder:${id}`;
}

// 树 item 右键（C3）：菜单宿主在 LibraryView（AppMenu 必须 Teleport body）
function onItemMenu(e: MouseEvent, node: FolderNode): void {
  selection.treeMenu = { x: e.clientX, y: e.clientY, node };
}

// —— "命名中"临时节点（C3）：本层是草稿落点时，插在 afterFolderId 之后（null = 末尾）——
const draftHere = computed(() =>
  selection.newFolderDraft && selection.newFolderDraft.parentId === props.parentId
    ? selection.newFolderDraft
    : null,
);
// 命名节点插入下标（nodes 数组内，在其后渲染）；afterFolderId 未命中（陈旧）按末尾处理
const draftIndex = computed(() => {
  if (!draftHere.value) return -1;
  const after = draftHere.value.afterFolderId;
  if (after === null) return props.nodes.length - 1;
  const i = props.nodes.findIndex((n) => n.id === after);
  return i < 0 ? props.nodes.length - 1 : i;
});
const nameError = ref("");
async function onNameCommit(name: string): Promise<void> {
  const draft = draftHere.value;
  if (!draft) return;
  const msg = validateWindowsName(name);
  if (msg) {
    nameError.value = msg;
    return;
  }
  // 后端同规则再校验一道；失败文案内联提示并保持输入态
  nameError.value = await createFolder(draft.parentId, name, draft.afterFolderId);
}
function onNameCancel(): void {
  if (draftHere.value) selection.newFolderDraft = null;
  nameError.value = "";
}
</script>

<template>
  <template v-for="(node, index) in nodes" :key="node.id">
    <div class="ft-node" @contextmenu.prevent.stop="onItemMenu($event, node)">
      <TreeItem
        :icon="isOpen(node.id) ? 'folder-open' : 'folder-closed'"
        :label="node.name"
        :count="node.count"
        :depth="depth"
        :expandable="node.children.length > 0"
        :open="isOpen(node.id)"
        :selected="content.sidebarSelected === `folder:${node.id}`"
        :last="index === nodes.length - 1"
        :folder-id="node.id"
        :data-drag-name="node.name"
        guide
        @toggle="toggle(node.id)"
        @select="onSelect(node.id)"
      />
      <div
        v-if="isOpen(node.id) && node.children.length > 0"
        class="ft-children"
        :style="{ '--guide-x': `${depth * 20 + 24}px` }"
      >
        <FolderTree
          :nodes="node.children"
          :depth="depth + 1"
          :parent-id="node.id"
          :force-open-ids="forceOpenIds"
        />
      </div>
    </div>
    <!-- "命名中"临时节点：插到 afterFolderId 兄弟之后（draftIndex 命中行尾追加） -->
    <div v-if="draftHere && draftIndex === index" class="ft-node naming">
      <TreeItem
        icon="folder-closed"
        label=""
        :depth="depth"
        :last="index === nodes.length - 1"
        guide
        naming
        :naming-error="!!nameError"
        @name-commit="onNameCommit"
        @name-cancel="onNameCancel"
      />
      <p v-if="nameError" class="name-error">{{ nameError }}</p>
    </div>
  </template>
  <!-- 空文件夹的"命名中"节点（新建子文件夹进空目录：v-for 不循环也要落点） -->
  <div v-if="draftHere && nodes.length === 0" class="ft-node naming">
    <TreeItem
      icon="folder-closed"
      label=""
      :depth="depth"
      last
      guide
      naming
      :naming-error="!!nameError"
      @name-commit="onNameCommit"
      @name-cancel="onNameCancel"
    />
    <p v-if="nameError" class="name-error">{{ nameError }}</p>
  </div>
</template>

<style scoped>
/* 节点：TreeItem 与展开子层之间同样 2px 间距（父级与第一个子级不紧贴） */
.ft-node {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.ft-children {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
/* 缩进竖线（02_左侧边栏 04）：从父级装饰图标中心（= 子项 item 容器左缘，depth×14+25）
   贯穿整层——穿过 item 间距与嵌套子层区域（子级的子级旁父级线仍在，两条并存），
   止于最后子项中心（L 收笔）；各层线间距 = 层级缩进 14px。
   颜色 = 装饰图标 fill 色（font-icon-4），自定义文件夹颜色时同步 */
.ft-children::before {
  content: "";
  position: absolute;
  left: var(--guide-x);
  top: 0;
  bottom: 14px;
  width: 1px;
  background: var(--font-icon-6);
}
/* 命名节点校验提示行（C3）：caption 档 / --error */
.name-error {
  margin: 0;
  padding-left: var(--space-16);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--error);
  word-break: break-all;
}
</style>
