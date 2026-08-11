<script setup lang="ts">
// 文件夹树（递归）：展开 / 折叠状态每层自管，选中态写入 content store（阶段 1 仅样式）。
// 层级与缩进线（2026-08-08 用户简化定案）：item 不携带层级 padding——子层容器 padding-left 20
// 承担缩进；缩进竖线 = 子层容器级连续线（left 14 = 父级 item 左 padding 8 + 箭头容器半宽 6，
// 正对父级展开按钮尖角，01_主窗口注释 8），纵贯首子项顶部至末子项底部（top 0 bottom 0）。
// 新建文件夹"先建后命名"（2026-07-29 复验三轮，替换原"命名中临时节点"）：入口直接以默认名
// 建库（LibraryView），成功返回的新 id 写入 selection.namingFolderId，本组件对应节点进
// naming 输入态——Enter / Esc / 失焦一律保存：名非空且变了 → renameFolder，空或不变保留
// 默认名；命名态节点不参与拖拽 / 右键（不挂 data-folder-id，naming 态 TreeItem 是 div 非 button）。
// 树 item 右键：全局菜单宿主（lib/contextMenus → App.vue <ContextMenu>），05 稿 320 宽 + 搜索框。
import { computed, nextTick } from "vue";
import TreeItem from "./TreeItem.vue";
import { dragView, visibleTreeFolderIds } from "../lib/dragOperation";
import { renameFolder } from "../lib/placementOps";
import { openContextMenu } from "../lib/contextMenus";
import { treeItemSpec } from "../lib/menuSpecs/folder";
import { isTreeOpen, toggleOpen } from "../lib/treeExpansion";
import { useContentStore, type FolderNode } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";
import { useUiStore } from "../stores/ui";

const props = withDefaults(
  defineProps<{
    nodes: FolderNode[];
    depth?: number; // 本层层级（内容缩进与缩进线定位用；item 底色块满宽不缩进）
    parentId?: string | null; // 本层父级 id（顶层 = null）：树排序"当前层级"与右键菜单判定、根父级形态判定用
    forceOpenIds?: Set<string>; // 命名节点的祖先链强制展开（新建子文件夹时父级可能折叠）
  }>(),
  { depth: 0, parentId: null, forceOpenIds: undefined },
);
const content = useContentStore();
const selection = useSelectionStore();
const tabs = useTabsStore();
const ui = useUiStore();

// 树排序（6.3 右键菜单"查看-排序"）：custom = folders.json order 原序；
// level* 仅锚定层级（ui.treeSortAnchor = 右键时记录的本层父级）、all* 全部层级按标题
const sortedNodes = computed(() => {
  const mode = ui.treeSort;
  if (mode === "custom") return props.nodes;
  const all = mode.startsWith("all");
  if (!all && (props.parentId ?? null) !== ui.treeSortAnchor) return props.nodes;
  const desc = mode.endsWith("Desc");
  return [...props.nodes].sort((a, b) =>
    desc ? b.name.localeCompare(a.name, "zh") : a.name.localeCompare(b.name, "zh"),
  );
});

// 展开态：模块级共享（lib/treeExpansion，2026-07-29——原每层自管，右键"展开/折叠所有"无从着力）
function isOpen(id: string): boolean {
  return isTreeOpen(id) || (props.forceOpenIds?.has(id) ?? false);
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
  tabs.openView(`folder:${id}`);
  // 2026-07-29 复验二轮 2 根因：树 plain click 原先只导航、不入文件夹选择域——
  // sidebarSelected 高亮冒充选中态，用户拖"看着选中实则不在域内"的项时引擎按
  // "拖拽未选中项 = 单选再拖"把已有多选清成单选（树多选排序只动被拖一项）。
  // 修为 plain click 同步单选入域；App.vue 的 sidebarSelected watch 会 selection.clear()
  //（pre flush 先于渲染执行），nextTick 让过它再入域
  void nextTick(() => selection.clickFolder(id));
}

// 树 item 右键（6.3）：全局菜单宿主（lib/contextMenus）；命名态节点不出菜单（2026-07-29 复验三轮）；
// 右键未选中项先单选（文件夹选择域，与素材互斥由 clickFolder 内部做）；
// 菜单取消（点外 / Esc 未执行动作）时恢复原选择（2026-08-10 用户反馈：取消后品牌色选中态残留）
function onItemMenu(e: MouseEvent, node: FolderNode): void {
  if (selection.namingFolderId === node.id) return;
  const opts = { width: 320, searchable: true }; // 1.3：树节点右键复用 05 文件夹稿（320 宽 + 搜索框）
  if (!selection.isFolderSelected(node.id)) {
    const snapshot = new Set(selection.selectedFolders);
    const prevAnchor = selection.folderAnchor;
    selection.clickFolder(node.id);
    openContextMenu(e, treeItemSpec(node, props.parentId ?? null), (executed) => {
      if (executed) return;
      selection.selectedFolders.clear();
      for (const id of snapshot) selection.selectedFolders.add(id);
      selection.folderAnchor = prevAnchor;
    }, opts);
    return;
  }
  openContextMenu(e, treeItemSpec(node, props.parentId ?? null), undefined, opts);
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
  <template v-for="node in sortedNodes" :key="node.id">
    <div class="ft-node" @contextmenu.prevent.stop="onItemMenu($event, node)">
      <!-- 当前目录（sidebarSelected）= selected 实心高亮（你在这里）；选择域成员 =
           domain-selected 品牌色描边（文件夹卡片选中联动 / 树多选）——两态互斥，
           当前目录优先（2026-08-10 用户定：区分双高亮，防"停在哪个目录"混淆） -->
      <TreeItem
        :icon="isOpen(node.id) ? 'folder-open' : 'folder-closed'"
        :label="node.name"
        :count="node.count"
        :depth="depth"
        :variant="parentId === null ? 'root' : 'child'"
        :expandable="node.children.length > 0"
        :open="isOpen(node.id)"
        :selected="content.sidebarSelected === `folder:${node.id}`"
        :domain-selected="selection.isFolderSelected(node.id) && content.sidebarSelected !== `folder:${node.id}`"
        :dimmed="dragView.folderIds?.has(node.id) ?? false"
        :folder-id="selection.namingFolderId === node.id ? undefined : node.id"
        :data-drag-name="node.name"
        :naming="selection.namingFolderId === node.id"
        @toggle="toggleOpen(node.id)"
        @select="onSelect(node.id, $event)"
        @name-commit="onNameCommit(node, $event)"
      />
      <div
        v-if="isOpen(node.id) && node.children.length > 0"
        class="ft-children"
        :style="{ '--guide-x': `${depth * 20 + 14}px` }"
      >
        <FolderTree :nodes="node.children" :depth="depth + 1" :parent-id="node.id" :force-open-ids="forceOpenIds" />
      </div>
    </div>
  </template>
</template>

<style scoped>
/* 节点：TreeItem 与展开子层间距 4（figma 40_左侧边栏 目录树 gap 4） */
.ft-node {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
/* 子层容器：不缩进（item 底色块满宽，内容缩进由 TreeItem padding-left 承担，2026-08-08 用户定） */
.ft-children {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
/* 缩进竖线（01_主窗口注释 8）：横坐标 = 父级箭头尖角（父级 padding-left 8+depth×20 + 箭头容器
   半宽 6，即 --guide-x = depth×20+14）；纵坐标首子项顶部 → 末子项底部（top 0 bottom 0 贯穿）。
   z-index 1：item 底色块满宽后会盖住同层先绘制的线（2026-08-08 走查修复），线在 item 空白
   padding 区（内容从 depth×20+28 起），不压内容 */
.ft-children::before {
  content: "";
  position: absolute;
  left: var(--guide-x);
  top: 0;
  bottom: 0;
  width: 1px;
  background: var(--line-hover);
  z-index: 1;
  pointer-events: none;
}
</style>
