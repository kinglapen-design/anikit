<script setup lang="ts">
// 目的树选择器（2026-10-03 库 ↔ 工作空间互通，PRD 02「与资源库互通」节）：
// 「导出到工作空间 / 导入到库」共用——移动面板样式（壳 / 搜索 / MoveNode 树全复用，
// 稿 = 布局/模态窗口/移动）的目的地单选弹窗；与移动面板的差异 = 行级 hover 恒并入
//（MoveNode intoOnly：无兄弟排序落点）、数据源按 draft.tree 二选一：
// "library" = 库文件夹树（blockSections 含区块，首行「未分类」= 根落点）；
// "ws" = 工作空间目录树（合成「工作空间」根行，懒加载逐级展开——展开即 ensureDir，
// 断链目录不作落点不渲染；搜索只覆盖已展开加载的范围）。
// 单击落点 = 警告确认（ConfirmPrompt alert warning，移动面板同口径两段高亮）→
// 执行 draft.onPick（导出 = export_assets/export_folder 落 ws 目录；导入 = importDraft
// 接既有导入弹窗），成功才关面板（失败 toast 由编排层直出，面板保留可重选）。
import { computed, ref, watch } from "vue";
import BaseModal from "./modals/BaseModal.vue";
import TextInput from "./inputs/TextInput.vue";
import SectionTitle from "./items/SectionTitle.vue";
import ScrollThumb from "./ScrollThumb.vue";
import MoveNode, { type MoveZone } from "./MoveNode.vue";
import { expandEnter, expandLeave } from "../lib/expandAnim";
import { useContentStore, type FolderNode } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useWsStore } from "../stores/ws";
import "./moveTree.css";

const content = useContentStore();
const selection = useSelectionStore();
const ws = useWsStore();
const draft = computed(() => selection.destTreeDraft);
const isWs = computed(() => draft.value?.tree === "ws");

// ws 模式打开即加载根目录（懒加载起点；深层随展开 ensureDir）
watch(draft, (d) => {
  if (d?.tree === "ws") void ws.ensureDir("");
}, { immediate: true });

// —— 数据源（ws 合成 FolderNode 树：只下钻已加载目录，loaded 缓存响应式驱动）——
function buildWsNodes(rel: string): FolderNode[] {
  return (ws.dirs.get(rel) ?? [])
    .filter((e) => e.isDir && !(e.isLink && !e.linkOk)) // 断链目录不可作落点
    .map((e) => ({
      id: `ws:${e.relPath}`,
      name: e.name,
      count: 0,
      totalSize: 0,
      kind: "folder",
      children: ws.loaded[e.relPath] ? buildWsNodes(e.relPath) : [],
    }));
}
const wsTree = computed<FolderNode[]>(() => [
  {
    id: "ws:",
    name: content.viewTitle("ws:"),
    count: 0,
    totalSize: 0,
    kind: "folder",
    children: buildWsNodes(""),
  },
]);

// 库模式首行「未分类」（= 根落点，onPick target null）
const uncatNode: FolderNode = { id: "__uncat", name: "未分类", count: 0, totalSize: 0, kind: "folder", children: [] };

// —— 搜索（命中节点或其后代保留；搜索时强制展开。ws 模式只覆盖已展开加载的范围）——
const query = ref("");
const searching = computed(() => query.value.trim().length > 0);
function filterNodes(nodes: FolderNode[]): FolderNode[] {
  const q = query.value.trim().toLowerCase();
  const out: FolderNode[] = [];
  for (const n of nodes) {
    const children = filterNodes(n.children);
    if (q && !n.name.toLowerCase().includes(q) && children.length === 0) continue;
    out.push({ ...n, children });
  }
  return out;
}
const wsFiltered = computed(() => (searching.value ? filterNodes(wsTree.value) : wsTree.value));
const sections = computed(() =>
  content.blockSections
    .map((sec) => ({ ...sec, nodes: filterNodes(sec.nodes) }))
    .filter((sec) => sec.nodes.length > 0),
);
function countNodes(nodes: FolderNode[]): number {
  return nodes.reduce((s, n) => s + 1 + countNodes(n.children), 0);
}

// —— 折叠：库模式默认全展开（移动面板同口径）；ws 模式深层默认折叠（懒加载随展开），根恒展开 ——
const collapsed = ref(new Set<string>());
const blockCollapsed = ref(new Set<string>());
const wsExpanded = ref(new Set<string>(["ws:"]));
const wsCollapsed = computed(() => {
  const out = new Set<string>();
  const walk = (nodes: FolderNode[]): void => {
    for (const n of nodes) {
      if (!wsExpanded.value.has(n.id)) out.add(n.id);
      walk(n.children);
    }
  };
  walk(wsTree.value);
  return out;
});
function toggle(id: string): void {
  if (isWs.value) {
    const next = new Set(wsExpanded.value);
    if (next.has(id)) next.delete(id);
    else {
      next.add(id);
      if (id.startsWith("ws:")) void ws.ensureDir(id.slice(3)); // 展开即懒加载子层
    }
    wsExpanded.value = next;
    return;
  }
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

// MoveNode 变体入参（ws 模式）：展开箭头 = hasSubDir（懒加载未加载时 children 为空）；
// 数量恒 0 不渲染（TreeRowItem 0 不显示口径——磁盘实况无库计数）
function wsExpandable(node: FolderNode): boolean {
  if (node.id === "ws:") return true;
  const rel = node.id.slice(3);
  const parent = rel.includes("/") ? rel.slice(0, rel.lastIndexOf("/")) : "";
  const entry = ws.dirs.get(parent)?.find((e) => e.relPath === rel);
  return entry?.hasSubDir ?? node.children.length > 0;
}
const zeroCount = (): number => 0;

// —— hover 并入高亮（intoOnly 变体无分区，hint 恒 into）——
const hint = ref<{ id: string; zone: MoveZone } | null>(null);
function onHover(id: string, zone: MoveZone): void {
  hint.value = { id, zone };
}
function onLeave(id: string): void {
  if (hint.value?.id === id) hint.value = null;
}

// 单击落点 = 警告确认（移动面板同口径）：target —— ws 模式 = rel（"" = 根）、
// 库模式 = folderId（未分类行 = null）；确认执行成功才关面板
function onPick(node: FolderNode): void {
  const d = draft.value;
  if (!d) return;
  const target = isWs.value ? node.id.slice(3) : node.id === "__uncat" ? null : node.id;
  selection.confirmPrompt = {
    title: d.title,
    body: "",
    bodySegments: [
      { text: "是否将" },
      { text: `「${d.sourceName}」`, accent: true },
      { text: isWs.value ? "导出至" : "导入至" },
      { text: `「${node.name}」`, accent: true },
      { text: "中" },
    ],
    confirmLabel: d.title,
    alert: "warning",
    onConfirm: () =>
      void (async () => {
        if (await d.onPick(target)) selection.destTreeDraft = null;
      })(),
  };
}

function onClose(): void {
  selection.destTreeDraft = null;
}

const treeRef = ref<HTMLElement>();
</script>

<template>
  <BaseModal v-if="draft" :title="draft.title" :width="460" framed class="move-panel" @close="onClose">
    <!-- 搜索框（移动稿同口径：0 24 padding；lg + 左置搜索图标） -->
    <div class="mv-search">
      <TextInput v-model="query" leading-icon="search" placeholder="搜索文件夹..." />
    </div>
    <div class="mv-wrap">
      <div ref="treeRef" class="mv-tree kit-scroll">
        <!-- 工作空间目录树（合成根行，无区块标题；懒加载逐级展开） -->
        <div v-if="isWs" class="mv-nodes">
          <MoveNode
            v-for="n in wsFiltered"
            :key="n.id"
            :node="n"
            :depth="0"
            :collapsed="wsCollapsed"
            :force-open="searching"
            :hint="hint"
            into-only
            :expandable="wsExpandable"
            :count-of="zeroCount"
            @toggle="toggle"
            @hover="onHover"
            @leave="onLeave"
            @pick="onPick"
          />
        </div>
        <!-- 库文件夹树（首行「未分类」= 根落点；区块结构同移动面板） -->
        <template v-else>
          <div class="mv-nodes">
            <MoveNode
              :node="uncatNode"
              :depth="0"
              :collapsed="collapsed"
              :force-open="searching"
              :hint="hint"
              into-only
              @toggle="toggle"
              @hover="onHover"
              @leave="onLeave"
              @pick="onPick"
            />
          </div>
          <section v-for="sec in sections" :key="sec.block.id" class="mv-block">
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
                  into-only
                  @toggle="toggle"
                  @hover="onHover"
                  @leave="onLeave"
                  @pick="onPick"
                />
              </div>
            </Transition>
          </section>
        </template>
      </div>
      <ScrollThumb :body="treeRef" />
    </div>
  </BaseModal>
</template>
