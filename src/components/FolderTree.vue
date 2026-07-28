<script setup lang="ts">
// 文件夹树（递归）：展开 / 折叠状态每层自管，选中态写入 content store（阶段 1 仅样式）。
// 缩进线（02_左侧边栏 04）：竖线为子层容器级连续线（从子项展开占位中心 x5 贯穿整层，
// 穿过有子项的行与 item 间距，止于最后子项中心）；横线由叶子 TreeItem 自画（x5→x9）。
// 新建文件夹"先建后命名"（2026-07-29 复验三轮，替换原"命名中临时节点"）：入口直接以默认名
// 建库（LibraryView），成功返回的新 id 写入 selection.namingFolderId，本组件对应节点进
// naming 输入态——Enter / Esc / 失焦一律保存：名非空且变了 → renameFolder，空或不变保留
// 默认名；命名态节点不参与拖拽 / 右键（不挂 data-folder-id，naming 态 TreeItem 是 div 非 button）。
// 树 item 右键（C3）：「新建同级文件夹 / 新建子文件夹」菜单宿主在 LibraryView（Teleport body）。
import { nextTick, ref } from "vue";
import TreeItem from "./TreeItem.vue";
import { dragView, visibleTreeFolderIds } from "../lib/dragOperation";
import { renameFolder } from "../lib/placementOps";
import { useContentStore, type FolderNode } from "../stores/content";
import { useSelectionStore } from "../stores/selection";

const props = withDefaults(
  defineProps<{
    nodes: FolderNode[];
    depth?: number;
    forceOpenIds?: Set<string>; // 命名节点的祖先链强制展开（新建子文件夹时父级可能折叠）
  }>(),
  { depth: 0, forceOpenIds: undefined },
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

// 选中：有效拖拽结束后的紧随 click 不触发（C4 引擎置位）；
// 多选（2026-07-28 复验反馈 5）：Ctrl = 加选切换 / Shift = 按可见展开态扁平化顺序锚点范围选
//（文件夹选择域在 selection store，与素材互斥由 clickFolder 内部做）——多选不切换当前文件夹视图
function onSelect(id: string, e: MouseEvent): void {
  if (dragView.suppressClick) return;
  if (e.ctrlKey || e.metaKey || e.shiftKey) {
    selection.clickFolder(id, { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey }, visibleTreeFolderIds());
    return;
  }
  content.sidebarSelected = `folder:${id}`;
  // 2026-07-29 复验二轮 2 根因：树 plain click 原先只导航、不入文件夹选择域——
  // sidebarSelected 高亮冒充选中态，用户拖"看着选中实则不在域内"的项时引擎按
  // "拖拽未选中项 = 单选再拖"把已有多选清成单选（树多选排序只动被拖一项）。
  // 修为 plain click 同步单选入域；App.vue 的 sidebarSelected watch 会 selection.clear()
  //（pre flush 先于渲染执行），nextTick 让过它再入域
  void nextTick(() => selection.clickFolder(id));
}

// 树 item 右键（C3）：菜单宿主在 LibraryView（AppMenu 必须 Teleport body）；
// 命名态节点不出菜单（2026-07-29 复验三轮）
function onItemMenu(e: MouseEvent, node: FolderNode): void {
  if (selection.namingFolderId === node.id) return;
  selection.treeMenu = { x: e.clientX, y: e.clientY, node };
}

// 命名保存（先建后命名）：非空且与当前名（默认名）不同 → renameFolder；
// 空或不变 = 保留默认名直接退出，不再有"取消新建"
async function onNameCommit(node: FolderNode, name: string): Promise<void> {
  if (selection.namingFolderId !== node.id) return; // Enter 提交后 blur 晚到的双触发护栏
  selection.namingFolderId = null;
  if (!name || name === node.name) return;
  await renameFolder(node.id, name);
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
        :selected="content.sidebarSelected === `folder:${node.id}` || selection.isFolderSelected(node.id)"
        :dimmed="dragView.folderIds?.has(node.id) ?? false"
        :last="index === nodes.length - 1"
        :folder-id="selection.namingFolderId === node.id ? undefined : node.id"
        :data-drag-name="node.name"
        :naming="selection.namingFolderId === node.id"
        guide
        @toggle="toggle(node.id)"
        @select="onSelect(node.id, $event)"
        @name-commit="onNameCommit(node, $event)"
      />
      <div
        v-if="isOpen(node.id) && node.children.length > 0"
        class="ft-children"
        :style="{ '--guide-x': `${depth * 20 + 24}px` }"
      >
        <FolderTree :nodes="node.children" :depth="depth + 1" :force-open-ids="forceOpenIds" />
      </div>
    </div>
  </template>
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
</style>
