<script setup lang="ts">
// 资源库视图（PRD 04-1）：快速访问区块 + 文件树区块（03_UI 布局/02_左侧边栏 04 树形容器）。
// 多区块（2026-08-14 区块子系统，PRD 02 §01）：文件树按区块分段——每区块 =
// SectionHeader（标题 / 顶级文件夹数 / 加号 / grip 拖拽排序）+ FolderTree（该区块顶级节点）；
// 区块展开态 = lib/treeExpansion 模块级 closedSet（默认开）；区块头挂 data-block-id 兼作
// 文件夹拖拽落点（移入该区块根级）与区块排序拖拽起点。
// 新建文件夹（2026-07-29 复验三轮改"先建后命名"）：入口——区块头 + 按钮（该区块根级最底部，
// 2026-08-14 起不再跟随 sidebarSelected 建兄弟）、树 item 右键菜单（同级 / 子级）；
// 统一流程 = 直接以同级唯一默认名调 createFolder 入库 →
// 成功后 selection.namingFolderId 定位新节点进命名态，不再有"命名中临时节点 / 取消新建"。
// 树空白右键 2026-08-11 用户定废除（判定范围只有 4px item 间距，无实用价值；
// 后期多区块后"在哪个区块新建"难判定）。
// 树 item 拖拽起点（C4）：pointerdown 超阈值进拖拽引擎，未过阈值 = 原 click 选中语义。
import { computed, ref } from "vue";
import SectionHeader from "./SectionHeader.vue";
import TreeItem from "./TreeItem.vue";
import FolderTree from "./FolderTree.vue";
import AppModal from "./AppModal.vue";
import AppButton from "./AppButton.vue";
import { beginFolderDrag, beginBlockDrag, dragView } from "../lib/dragOperation";
import { createAndName } from "../lib/menuOps";
import { openContextMenu } from "../lib/contextMenus";
import { recycleBlankSpec } from "../lib/menuSpecs/recycle";
import { currentSpace, setAutoImport } from "../lib/libraryOps";
import { isBlockOpen, openBlock, toggleBlockOpen } from "../lib/treeExpansion";
import { useContentStore, type FolderNode } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";

const content = useContentStore();
const selection = useSelectionStore();
const tabs = useTabsStore();

// 自动导入关闭时点击"自动导入"项：确认弹窗（2026-08-12 用户定）——开启 = 写库级设置 +
// 挂载收件箱盘符 + 启动监听后进入视图；取消 = 停留当前视图
const autoImportPrompt = ref(false);
const autoImportBusy = ref(false);

function onQuickSelect(key: string): void {
  if (key === "inbox" && !currentSpace.autoImport) {
    autoImportPrompt.value = true;
    return;
  }
  tabs.openView(`quick:${key}`);
}

async function onEnableAutoImport(): Promise<void> {
  if (autoImportBusy.value) return;
  autoImportBusy.value = true;
  try {
    if (await setAutoImport(true)) {
      autoImportPrompt.value = false;
      tabs.openView("quick:inbox");
    }
  } finally {
    autoImportBusy.value = false;
  }
}

// 快速访问（PRD 02-15 定案六项：全部 / 未分类 / 最近使用 / 自动导入 / 缩略图 / 回收站）：
// "缩略图"视图待 hidden 资产业务（5.x）接入后放；收件箱 = PRD"自动导入"
const quickItems = [
  { key: "all", icon: "library", label: "全部" },
  { key: "uncategorized", icon: "uncategorized", label: "未分类" },
  { key: "recent", icon: "recent", label: "最近使用" },
  { key: "inbox", icon: "auto-import", label: "自动导入" },
  { key: "thumbnail", icon: "thumbnail", label: "缩略图" },
  { key: "recycle", icon: "trash", label: "回收站" },
] as const;

// 数量口径（4.4）：全部=全部 Placement；未分类=root；收件箱=importedFrom=inbox 未摘标；
// 最近使用 / 回收站（6.1 二轮，2026-07-29）= recentAssets / recycleEntries 实长；
// 工作空间 / 标签 / 快捷入口占位视图不接（对应阶段未开工）
function quickCount(key: (typeof quickItems)[number]["key"]): number | undefined {
  switch (key) {
    case "all":
      return content.allCount;
    case "uncategorized":
      return content.uncategorizedCount;
    case "inbox":
      return content.inboxCount;
    case "recent":
      return content.recentAssets.length;
    case "recycle":
      return content.recycleEntries.length;
    default:
      return undefined;
  }
}

// —— 新建文件夹（先建后命名，2026-07-29 复验三轮）——
// 统一流程（createAndName / uniqueFolderName 已提为 lib/menuOps 共享：
// 树右键 / 文件夹卡片 / 区块头 + 按钮共用）；命名节点祖先链强制展开让新节点可见
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

// 区块头 + 按钮（2026-08-14 多区块）：该区块根级最底部新建；区块折叠时先强制展开让新节点可见
function onAddFolder(blockId: string): void {
  openBlock(blockId);
  void createAndName(null, null, blockId);
}

// 区块标题拖拽起点（区块排序；添加按钮在 beginBlockDrag 内排除）
function onBlockHeadPointerDown(blockId: string, e: PointerEvent): void {
  beginBlockDrag(e, blockId);
}

// 树 item 拖拽起点（C4）：文件夹同级排序；快速访问项无 data-folder-id 天然不作起点
function onTreePointerDown(e: PointerEvent): void {
  const item = (e.target as HTMLElement).closest(".tree-item[data-folder-id]");
  if (item) beginFolderDrag(e, item.getAttribute("data-folder-id")!);
}
// 快速访问项右键（13 稿回收站菜单，2026-08-11）：目前仅"回收站"有菜单——
// 还原全部文件 / 清空回收站（recycleBlankSpec，宽 148）；其余快速访问项无右键
function onQuickMenu(key: string, e: MouseEvent): void {
  if (key !== "recycle") return;
  e.preventDefault();
  openContextMenu(e, recycleBlankSpec(), undefined, { width: 148 });
}
</script>

<template>
  <div class="library-view">
    <!-- 快速访问（1.2 新稿：无区块标题，直接 item 列表） -->
    <section class="block quick">
      <div class="items">
        <TreeItem
          v-for="q in quickItems"
          :key="q.key"
          variant="quick"
          :icon="q.icon"
          :label="q.label"
          :count="quickCount(q.key)"
          :selected="content.sidebarSelected === `quick:${q.key}`"
          @select="onQuickSelect(q.key)"
          @contextmenu="onQuickMenu(q.key, $event)"
        />
      </div>
    </section>
    <!-- 区块段（2026-08-14 区块子系统）：每区块 = 标题行 + 该区块顶级文件夹树；
         区块头挂 data-block-id——文件夹拖入 = 移入该区块根级（并入高亮），
         区块互拖 = 排序（上/下半插入线，与树 item 排序视觉同口径） -->
    <section v-for="sec in content.blockSections" :key="sec.block.id" class="block tree">
      <div
        class="block-head"
        :class="{
          'drop-into': dragView.block?.id === sec.block.id && dragView.block.zone === 'into',
          'drop-before': dragView.block?.id === sec.block.id && dragView.block.zone === 'before',
          'drop-after': dragView.block?.id === sec.block.id && dragView.block.zone === 'after',
        }"
        :data-block-id="sec.block.id"
        :data-drag-name="sec.block.name"
        @pointerdown="onBlockHeadPointerDown(sec.block.id, $event)"
      >
        <SectionHeader
          :title="sec.block.name"
          :count="sec.nodes.length"
          addable
          grip
          pad="0 0 0 8px"
          :open="isBlockOpen(sec.block.id)"
          @update:open="toggleBlockOpen(sec.block.id)"
          @add="onAddFolder(sec.block.id)"
        />
      </div>
      <div
        v-if="isBlockOpen(sec.block.id)"
        class="items block-tree"
        :data-block-id="sec.block.id"
        @pointerdown="onTreePointerDown"
      >
        <FolderTree :nodes="sec.nodes" :force-open-ids="forceOpenIds" />
      </div>
    </section>
    <!-- 自动导入未开启确认弹窗（confirm 形态：无 X，点遮罩 / 取消关闭） -->
    <AppModal
      v-if="autoImportPrompt"
      variant="confirm"
      icon="auto-import"
      title="开启自动导入"
      @close="autoImportPrompt = false"
    >
      <p class="prompt-text">
        当前资源库未开启自动导入。开启后将挂载收件箱盘符，复制进收件箱的文件会自动导入资源库。
      </p>
      <template #actions>
        <AppButton label="开启" variant="primary" :disabled="autoImportBusy" @click="onEnableAutoImport" />
        <AppButton label="取消" :disabled="autoImportBusy" @click="autoImportPrompt = false" />
      </template>
    </AppModal>
  </div>
</template>

<style scoped>
/* 自动导入确认弹窗：正文常态色，限宽换行 */
.prompt-text {
  margin: 0;
  max-width: 320px;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
}
/* 1.2 新稿（01_主窗口 树容器）：两区块间距 24；容器 padding 由 SidebarLeft 承担 */
.library-view {
  display: flex;
  flex-direction: column;
  gap: var(--space-24);
}
/* 快速访问区块：item 间距 4（figma 853） */
.block.quick {
  display: flex;
  flex-direction: column;
  flex: none;
}
/* 文件树区块段（2026-08-14 多区块）：每区块独立成段自然堆叠（原单区块 flex:1 撑满作废） */
.block.tree {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  flex: none;
}
/* 区块标题行落点视觉（与 TreeItem 同口径）：into = 品牌 30% 罩层 + 1.5px 品牌边框线；
   before/after = 2px 品牌插入线于上/下沿 */
.block-head {
  position: relative;
  border-radius: var(--radius-6);
}
.block-head.drop-into {
  background: linear-gradient(var(--brand-1-30), var(--brand-1-30));
}
/* into 描边用 ::after 伪元素 border（2026-08-14 走查：inset box-shadow 1.5px 在分数物理像素下
   边缘渗线断出像素点——文件夹卡片 2026-08-10 定案教训，同 .folder-card::after 机制） */
.block-head.drop-into::after {
  content: "";
  position: absolute;
  inset: 0;
  border: 1.5px solid var(--brand-border);
  border-radius: var(--radius-6);
  pointer-events: none;
  z-index: 1;
}
.block-head.drop-before::before,
.block-head.drop-after::after {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  height: 2px;
  background: var(--brand);
  z-index: 1;
  pointer-events: none;
}
.block-head.drop-before::before {
  top: 0;
}
.block-head.drop-after::after {
  bottom: 0;
}
/* item 组：间距 4（figma 目录树 gap 4） */
.items {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
</style>
