<script setup lang="ts">
// 资源库视图（PRD 04-1）：快速访问区块 + 文件树区块（03_UI 布局/02_左侧边栏 04 树形容器）。
// 新建文件夹（2026-07-29 复验三轮改"先建后命名"）：三入口——区块头 + 按钮（跟随
// sidebarSelected：folder → 建兄弟插其下一行；quick:* → 根级最底部）、树 item 右键菜单
//（同级 / 子级）、树空白处右键（根级最底部）；统一流程 = 直接以同级唯一默认名
//（"新建文件夹"、"新建文件夹-2"…）调 createFolder 入库 → 成功后 selection.namingFolderId
// 定位新节点进命名态（FolderTree / TreeItem naming），不再有"命名中临时节点 / 取消新建"。
// 树 item 拖拽起点（C4）：pointerdown 超阈值进拖拽引擎，未过阈值 = 原 click 选中语义。
import { computed, ref } from "vue";
import SectionHeader from "./SectionHeader.vue";
import TreeItem from "./TreeItem.vue";
import FolderTree from "./FolderTree.vue";
import AppMenu from "./AppMenu.vue";
import MenuItem from "./MenuItem.vue";
import { beginFolderDrag } from "../lib/dragOperation";
import { createFolder } from "../lib/placementOps";
import { useContentStore, type FolderNode } from "../stores/content";
import { useSelectionStore } from "../stores/selection";

const content = useContentStore();
const selection = useSelectionStore();
const quickOpen = ref(true);
const treeOpen = ref(true);

const quickItems = [
  { key: "all", icon: "library-all", label: "全部" },
  { key: "uncategorized", icon: "uncategorized", label: "未分类" },
  { key: "inbox", icon: "inbox", label: "收件箱" },
  { key: "recent", icon: "recent", label: "最近使用" },
  { key: "trash", icon: "trash", label: "回收站" },
] as const;

// 数量口径（4.4）：全部=全部 Placement；未分类=root；收件箱=importedFrom=inbox 未摘标；
// 最近使用 / 回收站对应 6.1 未开工，先不出数量标签
function quickCount(key: (typeof quickItems)[number]["key"]): number | undefined {
  switch (key) {
    case "all":
      return content.placements.length;
    case "uncategorized":
      return content.uncategorizedCount;
    case "inbox":
      return content.inboxCount;
    default:
      return undefined;
  }
}

// —— 新建文件夹（先建后命名，2026-07-29 复验三轮）——
// 父级查找（建兄弟用）：命中顶层 = null（根级）
function findParentId(nodes: FolderNode[], id: string, parent: string | null = null): string | null | undefined {
  for (const n of nodes) {
    if (n.id === id) return parent;
    const hit = findParentId(n.children, id, n.id);
    if (hit !== undefined) return hit;
  }
  return undefined;
}

// 同级唯一默认名："新建文件夹"、"新建文件夹-2"…（按目标层级现有兄弟名递进）
function uniqueFolderName(parentId: string | null): string {
  let level = content.folders;
  if (parentId) {
    const walk = (nodes: FolderNode[]): FolderNode[] | null => {
      for (const n of nodes) {
        if (n.id === parentId) return n.children;
        const hit = walk(n.children);
        if (hit) return hit;
      }
      return null;
    };
    level = walk(content.folders) ?? [];
  }
  const names = new Set(level.map((n) => n.name));
  if (!names.has("新建文件夹")) return "新建文件夹";
  let i = 2;
  while (names.has(`新建文件夹-${i}`)) i++;
  return `新建文件夹-${i}`;
}

// 统一入口：直接以默认名建库 → 成功返回的新 id 进命名态（FolderTree 按 id 定位 naming 节点）
async function createAndName(parentId: string | null, afterFolderId: string | null): Promise<void> {
  const id = await createFolder(parentId, uniqueFolderName(parentId), afterFolderId);
  if (id) selection.namingFolderId = id;
}

// 命名节点祖先链：强制展开让新节点可见（新建子文件夹时父级可能折叠）；
// 命名态结束（namingFolderId 清空）后未手动展开的父级恢复折叠
const forceOpenIds = computed(() => {
  const out = new Set<string>();
  const target = selection.namingFolderId;
  if (!target) return out;
  const walk = (nodes: FolderNode[], trail: string[]): boolean => {
    for (const n of nodes) {
      if (n.id === target) {
        for (const id of trail) out.add(id);
        return true;
      }
      if (walk(n.children, [...trail, n.id])) return true;
    }
    return false;
  };
  walk(content.folders, []);
  return out;
});

// 区块头 + 按钮：folder:{id} → 建兄弟插其下一行；quick:* → 根级最底部
function onAddFolder(): void {
  const sel = content.sidebarSelected;
  if (sel.startsWith("folder:")) {
    const id = sel.slice(7);
    void createAndName(findParentId(content.folders, id) ?? null, id);
  } else {
    void createAndName(null, null);
  }
}

// 树 item 右键菜单（AppMenu 必须 Teleport body；菜单点击不自动关，动作内手动关）
function menuNewSibling(): void {
  const m = selection.treeMenu;
  selection.treeMenu = null;
  if (!m) return;
  void createAndName(findParentId(content.folders, m.node.id) ?? null, m.node.id);
}
function menuNewChild(): void {
  const m = selection.treeMenu;
  selection.treeMenu = null;
  if (!m) return;
  void createAndName(m.node.id, null);
}
// 树容器空白处右键 = 根级最底部新建（2026-07-28 定案）
function onTreeBlankMenu(e: MouseEvent): void {
  if ((e.target as HTMLElement).closest(".tree-item")) return;
  e.preventDefault();
  void createAndName(null, null);
}

// 树 item 拖拽起点（C4）：文件夹同级排序；快速访问项无 data-folder-id 天然不作起点
function onTreePointerDown(e: PointerEvent): void {
  const item = (e.target as HTMLElement).closest(".tree-item[data-folder-id]");
  if (item) beginFolderDrag(e, item.getAttribute("data-folder-id")!);
}
</script>

<template>
  <div class="library-view">
    <section class="block quick">
      <SectionHeader title="快速访问" pad="0 8px 0 16px" v-model:open="quickOpen" />
      <div v-if="quickOpen" class="items">
        <TreeItem
          v-for="q in quickItems"
          :key="q.key"
          :icon="q.icon"
          :label="q.label"
          :count="quickCount(q.key)"
          :selected="content.sidebarSelected === `quick:${q.key}`"
          @select="content.sidebarSelected = `quick:${q.key}`"
        />
      </div>
    </section>
    <section class="block tree">
      <SectionHeader
        title="文件夹"
        :count="content.folderTotal"
        addable
        pad="0 8px 0 16px"
        v-model:open="treeOpen"
        @add="onAddFolder"
      />
      <div
        v-if="treeOpen"
        class="items"
        @pointerdown="onTreePointerDown"
        @contextmenu="onTreeBlankMenu"
      >
        <FolderTree :nodes="content.folders" :force-open-ids="forceOpenIds" />
      </div>
    </section>

    <!-- 树 item 右键菜单（C3）：Teleport body（与卡片菜单同约，防 transform 祖先定位坑） -->
    <Teleport to="body">
      <AppMenu
        v-if="selection.treeMenu"
        :x="selection.treeMenu.x"
        :y="selection.treeMenu.y"
        @close="selection.treeMenu = null"
      >
        <MenuItem label="新建同级文件夹" @select="menuNewSibling" />
        <MenuItem label="新建子文件夹" @select="menuNewChild" />
      </AppMenu>
    </Teleport>
  </div>
</template>

<style scoped>
.library-view {
  display: flex;
  flex-direction: column;
}
/* 快速访问区块：内边距 8 0 / 间距 8（2026-07-23 用户定） */
.block.quick {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  padding: var(--space-8) 0;
}
/* 文件树区块：内边距 8 0 / 间距 8，填满剩余 */
.block.tree {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  padding: var(--space-8) 0;
  flex: 1;
}
/* item 组：内边距 0 8 0 0 / 间距 2（2026-07-23 用户定，原左 4） */
.items {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: 0 var(--space-8) 0 0;
}
</style>
