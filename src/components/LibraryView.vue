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
import { computed, nextTick, reactive, ref, watch } from "vue";
import SectionHeader from "./SectionHeader.vue";
import TreeItem from "./TreeItem.vue";
import FolderTree from "./FolderTree.vue";
import AppModal from "./AppModal.vue";
import AppButton from "./AppButton.vue";
import { beginFolderDrag, dragView } from "../lib/dragOperation";
import { createAndName } from "../lib/menuOps";
import { openContextMenu } from "../lib/contextMenus";
import { recycleBlankSpec } from "../lib/menuSpecs/recycle";
import { currentSpace, setAutoImport } from "../lib/libraryOps";
import { reorderBlocks } from "../lib/blockOps";
import { isBlockOpen, openBlock, toggleBlockOpen, treeExpansion } from "../lib/treeExpansion";
import { useContentStore, type FolderNode } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";

const content = useContentStore();
const selection = useSelectionStore();
const tabs = useTabsStore();

// 树定位跟随（2026-08-20 用户定）：导航进文件夹视口（双击文件夹卡片 / 树外任何入口）时——
// 展开祖先链 + 所在区块，并滚动到该节点；原仅 sidebarSelected 高亮，父级折叠时当前文件夹
// 在树上看不见（视觉焦点丢失）。自身不展开（定位语义 = 看到它）；树上点击导航时节点本可见，
// scrollIntoView(nearest) 不位移，天然无副作用。folders 就绪前（启动恢复期）找不到节点则跳过，
// folders 到达后补定位一次（watch 第二源）
watch(
  [() => content.sidebarSelected, () => content.folders],
  async ([key]) => {
    if (!key.startsWith("folder:")) return;
    const id = key.slice(7);
    // 祖先链（顶级在前）：找不到 = 数据未就绪或已删除，跳过
    const trail: string[] = [];
    const walk = (nodes: FolderNode[], acc: string[]): boolean => {
      for (const n of nodes) {
        if (n.id === id) {
          trail.push(...acc);
          return true;
        }
        if (walk(n.children, [...acc, n.id])) return true;
      }
      return false;
    };
    if (!walk(content.folders, [])) return;
    for (const aid of trail) treeExpansion.openSet.add(aid);
    // 所在区块折叠时展开（折叠区块内节点 DOM 不渲染，不展开滚动不到）
    const topId = trail.length > 0 ? trail[0] : id;
    const topNode = content.folders.find((n) => n.id === topId);
    if (topNode?.block) openBlock(topNode.block);
    await nextTick();
    document
      .querySelector(`.library-view .tree-item[data-folder-id="${CSS.escape(id)}"]`)
      ?.scrollIntoView({ block: "nearest" });
  },
);

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

// 区块标题拖拽排序（2026-08-14 用户定：与切换资源库同款让位机制——拖动段跟随指针无过渡、
// 被跨越段反向让位 0.18s 过渡；松手提交 reorder_blocks，归位帧禁过渡防"弹回去"）。
// 不走拖拽引擎：区块段高不定（头 + 展开的树），引擎的间隙/插入线模型不适配；
// 本地 pointer 追踪与库菜单（ContextMenu libDrag）同范式。不挂 pointer capture（折叠 click 保留），
// 僵尸兜底 = buttons 自清 + 划出窗口取消（与引擎同口径）
const BLOCK_GAP = 24; // .library-view 段间距 --space-24
const blockDrag = reactive({ id: "", startY: 0, dy: 0, from: -1, to: -1 });
const blockSettling = ref(false); // 松手归位帧：禁段 transform 过渡
let blockMoved = false; // 超阈值视为拖动：松手后抑制折叠 click
let blockSpans: { mid: number }[] = []; // 各段中线（拖动起时 DOM 快照，含 gap 累计）
let blockStep = 0; // 被拖段高 + gap（让位位移量）

function onBlockHeadPointerDown(blockId: string, e: PointerEvent): void {
  if (e.button !== 0) return;
  if ((e.target as HTMLElement).closest(".add-btn")) return; // 加号不启动
  const secs = content.blockSections;
  const from = secs.findIndex((s) => s.block.id === blockId);
  if (from < 0) return;
  // 各段高度快照（含折叠态——折叠段只有头高）；段序 = blockSections 序
  const els = [...document.querySelectorAll<HTMLElement>(".library-view .block.tree")];
  let acc = 0;
  blockSpans = els.map((el) => {
    const h = el.offsetHeight;
    const span = { mid: acc + h / 2 };
    acc += h + BLOCK_GAP;
    return span;
  });
  blockStep = (els[from]?.offsetHeight ?? 0) + BLOCK_GAP;
  Object.assign(blockDrag, { id: blockId, startY: e.clientY, dy: 0, from, to: from });
  blockMoved = false;
  window.addEventListener("pointermove", onBlockDragMove);
  window.addEventListener("pointerup", onBlockDragEnd, { once: true });
  window.addEventListener("blur", onBlockDragCancel, { once: true });
  document.addEventListener("mouseleave", onBlockDragCancel, { once: true });
}

// 拖动段中线越过邻段中线 → 目标位迁移（让位动画由 blockSectionStyle 驱动）
function onBlockDragMove(e: PointerEvent): void {
  if (!blockDrag.id) return;
  if ((e.buttons & 1) === 0) {
    onBlockDragCancel(); // 僵尸自清：pointerup 丢失（窗外松手等）
    return;
  }
  blockDrag.dy = e.clientY - blockDrag.startY;
  if (Math.abs(blockDrag.dy) > 3) blockMoved = true;
  const c = (blockSpans[blockDrag.from]?.mid ?? 0) + blockDrag.dy;
  let to = blockDrag.from;
  for (let i = 0; i < blockSpans.length; i++) {
    if (i > blockDrag.from && c > blockSpans[i].mid) to = i; // 下移：越过几个中线落几位
    // 上移：首个被越过的中线即落点（须 break——不 break 时更大的 i 也会命中并把 to 盖回去，
    // 大步跨越多段上移会少落一位；DetailPanel 区块拖拽 2026-08-15 实机踩中同款）
    if (i < blockDrag.from && c < blockSpans[i].mid) {
      to = i;
      break;
    }
  }
  blockDrag.to = to;
}

function onBlockDragEnd(): void {
  offBlockDrag();
  const { from, to } = blockDrag;
  const moved = blockMoved;
  blockDrag.id = "";
  if (!moved || from < 0 || to === from) return;
  const ids = content.blockSections.map((s) => s.block.id);
  const [m] = ids.splice(from, 1);
  ids.splice(to, 0, m);
  void reorderBlocks(ids); // 后端全集校验 + undo 快照 + refresh 在封装内
  // 抑制紧随 click 误折叠（与引擎 suppressClick 同口径）
  dragView.suppressClick = true;
  setTimeout(() => (dragView.suppressClick = false), 0);
  // 松手即到位：让位段视觉本已在目标位，清 transform 必须禁过渡（库菜单 2026-08-12 走查同坑）
  blockSettling.value = true;
  setTimeout(() => (blockSettling.value = false), 50);
}

function onBlockDragCancel(): void {
  offBlockDrag();
  blockDrag.id = "";
}

function offBlockDrag(): void {
  window.removeEventListener("pointermove", onBlockDragMove);
  window.removeEventListener("pointerup", onBlockDragEnd);
  window.removeEventListener("blur", onBlockDragCancel);
  document.removeEventListener("mouseleave", onBlockDragCancel);
}

// 让位位移：拖动段跟随指针（无过渡、置顶）；被跨越段反向让出一步（transition 在 .block.tree 根样式）
function blockSectionStyle(i: number): string {
  if (!blockDrag.id) return "";
  if (i === blockDrag.from) {
    return `transform: translateY(${blockDrag.dy}px); transition: none; position: relative; z-index: 2;`;
  }
  if (blockSettling.value) return "";
  const { from, to } = blockDrag;
  if (from < to && i > from && i <= to) return `transform: translateY(${-blockStep}px)`;
  if (to < from && i >= to && i < from) return `transform: translateY(${blockStep}px)`;
  return "";
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
  <div class="library-view" :class="{ settling: blockSettling }">
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
         区块头挂 data-block-id——文件夹拖入 = 移入该区块根级（并入高亮，引擎）；
         区块互拖 = 让位式排序（本地 pointer 追踪，库菜单同款动画） -->
    <section
      v-for="(sec, i) in content.blockSections"
      :key="sec.block.id"
      class="block tree"
      :style="blockSectionStyle(i)"
    >
      <div
        class="block-head"
        :class="{
          'drop-into': dragView.block?.id === sec.block.id,
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
/* 文件树区块段（2026-08-14 多区块）：每区块独立成段自然堆叠（原单区块 flex:1 撑满作废）；
   transform 过渡 = 区块拖拽让位动画（库菜单同款；拖动段 inline 禁过渡跟随指针） */
.block.tree {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  flex: none;
  transition: transform 0.18s ease;
}
/* 松手归位帧：让位段视觉本已在目标位，清 transform 禁过渡防"弹回去" */
.library-view.settling .block.tree {
  transition: none;
}
/* 区块标题行落点视觉（与 TreeItem 同口径）：into = 品牌 30% 罩层 + 1.5px 品牌边框线 */
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
/* item 组：间距 4（figma 目录树 gap 4） */
.items {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
</style>
