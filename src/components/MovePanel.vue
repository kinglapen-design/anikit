<script setup lang="ts">
// 移动面板（3.4；2026-09-08 换代 = 布局/模态窗口/移动 稿，旧 29 稿 AppModal 壳口径退役）：
// 移动文件 / 文件夹 / 档案共用——文件夹树选落点：
// 壳 = modals/BaseModal framed 变体（460 宽稿值：padding 0 / 列 gap 12 + 标题栏 54 带底线与关闭 X）；
// 搜索框 = inputs/TextInput lg + 左置 search 图标（稿占位「搜索文件夹...」）；
// 区块标题 = items/SectionTitle block 型（添加钮稿有画——点亮不接业务：移动窗新建文件夹未立项）；
// item = MoveNode（items/TreeRowItem）：行级 hover 三分（上/下 = 移为前/后兄弟，中 = 并入为子项），
// 单击弹出警告确认（ConfirmPrompt alert warning，发起名与目标父级名品牌色高亮——29 稿 prose 沿），
// 确认执行。默认全部展开（折叠态本地自管）；移动文件夹时发起项自身及后代不渲染
// （不能移入自己，后端护栏之外的 UI 前置）；树恒按自定义序（order 字段）渲染——
// before/after 落点所见即所得，不受侧栏"排序依据"显示模式影响。
// 业务：文件 = move_placements / 文件夹·档案 = move_folders（复用后端护栏：档案不作父级、禁入自身/子级）。
import { computed, ref } from "vue";
import BaseModal from "./modals/BaseModal.vue";
import TextInput from "./inputs/TextInput.vue";
import SectionTitle from "./items/SectionTitle.vue";
import ScrollThumb from "./ScrollThumb.vue";
import MoveNode, { type MoveZone } from "./MoveNode.vue";
import { expandEnter, expandLeave } from "../lib/expandAnim";
import { useContentStore, type FolderNode } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { moveFolders, movePlacements } from "../lib/placementOps";

const content = useContentStore();
const selection = useSelectionStore();
const draft = computed(() => selection.moveDraft);

function findNode(id: string, nodes: FolderNode[] = content.folders): FolderNode | null {
  for (const n of nodes) {
    if (n.id === id) return n;
    const hit = findNode(id, n.children);
    if (hit) return hit;
  }
  return null;
}

// 标题 = 确认按钮文案（29 稿 prose：移动文件 / 移动文件夹 / 移动档案）
const title = computed(() => {
  const d = draft.value;
  if (!d) return "";
  if (d.kind === "assets") return "移动文件";
  if (d.ids.length === 1) return findNode(d.ids[0])?.kind === "archive" ? "移动档案" : "移动文件夹";
  return "移动文件夹";
});

// 发起名（确认文案高亮段）：单个 = 名；多个 = "N 个素材 / N 个文件夹"
const sourceName = computed(() => {
  const d = draft.value;
  if (!d) return "";
  if (d.kind === "assets") {
    if (d.refs.length === 1) {
      const r = d.refs[0];
      return content.placements.find((p) => p.assetId === r.assetId && p.folderId === r.folderId)?.name ?? "";
    }
    return `${d.refs.length} 个素材`;
  }
  return d.ids.length === 1 ? (content.folderName(d.ids[0]) ?? d.ids[0]) : `${d.ids.length} 个文件夹`;
});

// 排除集（移动文件夹）：发起 ids + 全部后代（不能移入自己）
const excluded = computed<Set<string>>(() => {
  const out = new Set<string>();
  const d = draft.value;
  if (d?.kind !== "folders") return out;
  const collect = (n: FolderNode): void => {
    out.add(n.id);
    n.children.forEach(collect);
  };
  for (const id of d.ids) {
    const n = findNode(id);
    if (n) collect(n);
  }
  return out;
});

// —— 搜索（命中节点或其后代保留；搜索时强制展开）与折叠（默认全展开）——
const query = ref("");
const collapsed = ref(new Set<string>()); // item 折叠
const blockCollapsed = ref(new Set<string>()); // 区块折叠
function toggle(id: string): void {
  const next = new Set(collapsed.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  collapsed.value = next;
}
function toggleBlock(id: string): void {
  const next = new Set(blockCollapsed.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  blockCollapsed.value = next;
}

function filterNodes(nodes: FolderNode[]): FolderNode[] {
  const q = query.value.trim().toLowerCase();
  const out: FolderNode[] = [];
  for (const n of nodes) {
    if (excluded.value.has(n.id)) continue;
    const children = filterNodes(n.children);
    if (q && !n.name.toLowerCase().includes(q) && children.length === 0) continue;
    out.push({ ...n, children });
  }
  return out;
}
const searching = computed(() => query.value.trim().length > 0);
const sections = computed(() =>
  content.blockSections
    .map((sec) => ({ ...sec, nodes: filterNodes(sec.nodes) }))
    .filter((sec) => sec.nodes.length > 0),
);
function countNodes(nodes: FolderNode[]): number {
  return nodes.reduce((s, n) => s + 1 + countNodes(n.children), 0);
}

// —— hover 三分（落点视觉）——
const hint = ref<{ id: string; zone: MoveZone } | null>(null);
function onHover(id: string, zone: MoveZone): void {
  hint.value = { id, zone };
}
function onLeave(id: string): void {
  if (hint.value?.id === id) hint.value = null;
}

// 落点定位（全量树非过滤树）：父级 / 同级列表 / 所在区块
function locate(id: string): { parentId: string | null; siblings: FolderNode[]; blockId: string; blockName: string } | null {
  const walk = (nodes: FolderNode[], parent: string | null): { parentId: string | null; siblings: FolderNode[] } | null => {
    for (const n of nodes) {
      if (n.id === id) return { parentId: parent, siblings: nodes };
      const hit = walk(n.children, n.id);
      if (hit) return hit;
    }
    return null;
  };
  for (const sec of content.blockSections) {
    const hit = walk(sec.nodes, null);
    if (hit) return { ...hit, blockId: sec.block.id, blockName: sec.block.name };
  }
  return null;
}

// 单击 item = 按当前 hover 分区计算落点 → 警告确认（alert warning，29 稿 prose 文案 + 品牌色高亮）
function onPick(node: FolderNode): void {
  const d = draft.value;
  if (!d) return;
  const zone = hint.value?.id === node.id ? hint.value.zone : "into";
  const loc = locate(node.id);
  if (!loc) return;
  let targetName: string;
  let exec: () => Promise<boolean>;
  if (zone === "into") {
    targetName = node.name;
    exec = d.kind === "assets" ? () => movePlacements(d.refs, node.id) : () => moveFolders(d.ids, node.id);
  } else {
    const idx = loc.siblings.findIndex((n) => n.id === node.id);
    const beforeId = zone === "before" ? node.id : (loc.siblings[idx + 1]?.id ?? null);
    targetName = loc.parentId
      ? (content.folderName(loc.parentId) ?? loc.parentId)
      : d.kind === "assets"
        ? "未分类"
        : loc.blockName;
    exec =
      d.kind === "assets"
        ? () => movePlacements(d.refs, loc.parentId ?? "root")
        : () => moveFolders(d.ids, loc.parentId, beforeId, loc.parentId === null ? loc.blockId : null);
  }
  selection.confirmPrompt = {
    title: title.value,
    body: "",
    bodySegments: [
      { text: "是否将「" },
      { text: sourceName.value, accent: true },
      { text: "」移动至「" },
      { text: targetName, accent: true },
      { text: "」中" },
    ],
    confirmLabel: title.value,
    alert: "warning",
    // 成功才关面板；失败 toast 由 runOp 直出，面板保留可重选落点
    onConfirm: () =>
      void (async () => {
        if (await exec()) selection.moveDraft = null;
      })(),
  };
}

function onClose(): void {
  selection.moveDraft = null;
}

const treeRef = ref<HTMLElement>();
</script>

<template>
  <BaseModal v-if="draft" :title="title" :width="460" framed class="move-panel" @close="onClose">
    <!-- 搜索框（移动稿：0 24 padding；lg 34 高 + 左置搜索图标 亮色/三级） -->
    <div class="mv-search">
      <TextInput v-model="query" leading-icon="search" placeholder="搜索文件夹..." />
    </div>
    <!-- 树形容器（移动稿：0 24 padding / 区块间 gap 24；滚轮 = ScrollThumb 悬浮件） -->
    <div class="mv-wrap">
      <div ref="treeRef" class="mv-tree kit-scroll">
        <section v-for="sec in sections" :key="sec.block.id" class="mv-block">
          <!-- 区块标题（稿 = 标题 item block 型：标题 / 数量 + 展开钮 + 添加钮；
               添加钮稿有画但移动窗新建文件夹未立项——点亮不接业务，同菜单栏口径） -->
          <SectionTitle
            variant="block"
            :title="sec.block.name"
            :count="countNodes(sec.nodes)"
            :expanded="!blockCollapsed.has(sec.block.id)"
            :with-more="false"
            @toggle="toggleBlock(sec.block.id)"
          />
          <Transition :css="false" @enter="expandEnter" @leave="expandLeave">
            <div v-if="!blockCollapsed.has(sec.block.id)" class="mv-nodes">
              <MoveNode
                v-for="n in sec.nodes"
                :key="n.id"
                :node="n"
                :depth="0"
                :collapsed="collapsed"
                :force-open="searching"
                :hint="hint"
                @toggle="toggle"
                @hover="onHover"
                @leave="onLeave"
                @pick="onPick"
              />
            </div>
          </Transition>
        </section>
      </div>
      <ScrollThumb :body="treeRef" />
    </div>
  </BaseModal>
</template>

<style scoped>
/* 移动稿壳：460 宽稿值由 BaseModal width prop 给（border-box 含 1px 边）；
   高 822 稿值 + 窗口封顶留呼吸（旧 29 稿 906+96 封顶口径沿）；
   class 透传落在 BaseModal 根 .bm-mask 上 → 壳样式须 :deep(.bm-modal) */
.move-panel :deep(.bm-modal) {
  height: min(822px, calc(100vh - 96px));
}
.mv-search {
  padding: 0 var(--n-space-24);
  flex: none;
}
.mv-wrap {
  position: relative;
  flex: 1;
  min-height: 0;
}
/* 树形容器：0 24 padding（稿）；区块间 gap 24（稿），区块内 标题→树 gap 8（稿 文件夹-区块 gap 8） */
.mv-tree {
  height: 100%;
  overflow-y: auto;
  padding: 0 var(--n-space-24);
  display: flex;
  flex-direction: column;
  gap: var(--n-space-24);
}
.mv-block {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
  flex: none;
}
/* 区块内树 item 间 gap 2（移动稿 树形容器 gap 2） */
.mv-nodes {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
}
</style>
