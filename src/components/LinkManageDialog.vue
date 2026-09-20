<script setup lang="ts">
// 链接管理弹窗（17 稿，6.2 三轮；PRD 03_数据链接 02 链接区块）：
// 管理**一个文件**的链接分布（右键「文件链接管理」Ctrl+J / 详情链接区块添加钮两个入口，
// 2026-08-24 用户定；工具栏/新增菜单不设——"链接管理是针对某个文件的"）。
// 左列 = 侧边栏文件树照搬（用户定）：区块分组（content.blockSections），文件夹格两档底色 =
// 02_目录 item（顶级 C5 + 1px 亮-选中 border / 其他层级 #1F2020）+ 自定义色 tint 传递
//（--custom-{tint} 主档 / 子级 -bg，TreeItem 同口径），数量 = 节点递归 Placement 总数（同树）；
// 组头纯显示（用户定"组头只是显示用处"——+ 钮不接线）；末尾补「未分类」特殊组（稿无，
// 文件在未分类也要可管理）。右列 = 该文件在该文件夹的位置行：名称框（点击 = 编辑态品牌描边
// 输入框就地改显示名）+ ★ 原始 / 链接徽章 + ✓；同文件夹重复链接平铺多行（用户定，后续版本再重构）。
// ✓ 交互：亮 = 有位置（点 = 取消勾选移除）；无位置行常态右列空、行 hover 出淡 ✓（点 = 添加，
// 名称默认当前显示名，同文件夹唯一化后端处理）。**草稿式**：勾选/改名不即时生效，
// 确认 = diff 三组（增/删/改）sync_asset_links 一次提交（单撤销步）；取消 / X / Esc = 全部放弃。
// 移除 = 直接删 Placement 记录不进回收站（用户定）；后端护栏：位置归 0 报错（防孤儿资产）。
// 档案链接管理随档案多位置立项（2026-08-24 用户定，09 待办）。
// 壳 = modals/BaseModal framed（2026-09-20 R2 旧件烧减轮换代，旧 AppModal standard + AppButton +
// AppSearchBar + 根 IconButton 退役）：602 宽 = 内容 552 + padding 48 + 新壳 1px border×2
//（旧 1.5 border「+3 补偿」口径随壳换代改 +2，速查 §1）；framed 壳 padding 0，各段自承 padding 0 24；
// 按钮序随新口径翻转次左主右、宽自适应（88 定宽废）。
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import ActionButton from "./buttons/ActionButton.vue";
import Checkbox from "./buttons/Checkbox.vue";
import AppIcon from "./AppIcon.vue";
import BaseModal from "./modals/BaseModal.vue";
import IconButton from "./buttons/IconButton.vue";
import TextInput from "./inputs/TextInput.vue";
import ScrollThumb from "./ScrollThumb.vue";
import { syncAssetLinks, type SyncLinkAdd, type SyncLinkRemove, type SyncLinkRename } from "../lib/placementOps";
import { toNewTint } from "../lib/customColors";
import { useContentStore, type FolderNode } from "../stores/content";
import { useSelectionStore } from "../stores/selection";

const content = useContentStore();
const selection = useSelectionStore();

const draft = computed(() => selection.linkManageDraft);
const assetId = computed(() => draft.value?.assetId ?? "");

// —— 草稿行（打开时刻 placements 快照 + 弹窗内编辑）——
interface DraftRow {
  key: string;
  folderId: string;
  origin: boolean;
  name: string; // 草稿名（编辑态改写）
  origName: string; // 打开时名（diff 用；isNew = ""）
  isNew: boolean;
  removed: boolean;
}
const rows = ref<DraftRow[]>([]);
const query = ref("");
const onlyChecked = ref(false);
const busy = ref(false);
const bodyRef = ref<HTMLElement>();

onMounted(() => {
  rows.value = content.placements
    .filter((p) => p.assetId === assetId.value)
    .map((p, i) => ({
      key: `e${i}`,
      folderId: p.folderId,
      origin: p.origin,
      name: p.name,
      origName: p.name,
      isNew: false,
      removed: false,
    }));
  window.addEventListener("keydown", onKey, true);
});
onBeforeUnmount(() => window.removeEventListener("keydown", onKey, true));

function onKey(e: KeyboardEvent): void {
  if (e.key === "Escape" && editingKey.value === "") {
    e.stopPropagation();
    onCancel();
  }
}

function onCancel(): void {
  selection.linkManageDraft = null;
}

// —— 树（左列照搬侧边栏：区块分组 + 递归节点；搜索 = 命中保留祖先且全展开）——
interface Line {
  node: FolderNode;
  depth: number;
  tint: string | null; // 自定义色档（节点自身 ?? 父级传递；子级视觉 -bg）
}
const collapsed = ref(new Set<string>()); // 折叠节点 id（默认全展开）
const blockCollapsed = ref(new Set<string>()); // 折叠区块 id

function matchTree(nodes: FolderNode[]): FolderNode[] {
  const q = query.value.trim().toLowerCase();
  if (!q) return nodes;
  const walk = (list: FolderNode[]): FolderNode[] =>
    list
      .map((n) => ({ ...n, children: walk(n.children) }))
      .filter((n) => n.name.toLowerCase().includes(q) || n.children.length > 0);
  return walk(nodes);
}

function flatten(nodes: FolderNode[], parentTint: string | null, depth: number, out: Line[]): void {
  for (const n of nodes) {
    const tint = n.color ?? parentTint;
    out.push({ node: n, depth, tint });
    const open = query.value.trim() !== "" || !collapsed.value.has(n.id);
    if (open && n.children.length > 0) flatten(n.children, tint, depth + 1, out);
  }
}

const sections = computed(() => {
  const out: { id: string; name: string; count: number; lines: Line[] }[] = [];
  for (const sec of content.blockSections) {
    const nodes = matchTree(sec.nodes);
    const lines: Line[] = [];
    if (!blockCollapsed.value.has(sec.block.id)) flatten(nodes, null, 0, lines);
    out.push({ id: sec.block.id, name: sec.block.name, count: sec.nodes.reduce((a, n) => a + n.count, 0), lines });
  }
  // 未分类特殊组（稿无，实现补：root 位置也要可管理）；搜索命中「未分类」时保留
  const rootPlacements = content.placements.filter((p) => p.folderId === "root");
  const rootCount = rootPlacements.length;
  // 容量口径同后端（2026-08-30 用户定案）：按 asset 去重合计 size
  const seen = new Set<string>();
  const rootSize = rootPlacements.reduce((a, p) => (seen.has(p.assetId) ? a : (seen.add(p.assetId), a + p.size)), 0);
  const rootHit = query.value.trim() === "" || "未分类".includes(query.value.trim());
  if (rootHit) {
    const lines: Line[] = [];
    if (!blockCollapsed.value.has("__root__")) {
      lines.push({
        node: { id: "root", name: "未分类", count: rootCount, totalSize: rootSize, children: [] },
        depth: 0,
        tint: null,
      });
    }
    out.push({ id: "__root__", name: "未分类", count: rootCount, lines });
  }
  return out;
});

// 仅显示已选项：行过滤 = 该文件夹有（未移除）草稿
function draftsOf(folderId: string): DraftRow[] {
  return rows.value.filter((d) => d.folderId === folderId && !d.removed);
}
function visibleLines(lines: Line[]): Line[] {
  if (!onlyChecked.value) return lines;
  return lines.filter((l) => draftsOf(l.node.id).length > 0);
}

// —— ✓ 勾选：有草稿行点 ✓ = 移除；无草稿行点淡 ✓ = 添加（名称默认当前显示名）——
function toggleRemove(d: DraftRow): void {
  d.removed = true;
}
function addDraft(folderId: string): void {
  rows.value.push({
    key: `n${Date.now()}_${rows.value.length}`,
    folderId,
    origin: false,
    name: draft.value?.name ?? "",
    origName: "",
    isNew: true,
    removed: false,
  });
}

// —— 名称编辑（点击名称框 = 编辑态：C2 底 + 1.5px 品牌描边输入框；blur/Enter 提交本地草稿，Esc 还原）——
const editingKey = ref("");
const editDraft = ref("");
const editInputRef = ref<HTMLInputElement[]>([]); // v-for 内模板 ref = 数组收集
function startEdit(d: DraftRow): void {
  editingKey.value = d.key;
  editDraft.value = d.name;
  void nextTick(() => editInputRef.value[0]?.select());
}
function commitEdit(d: DraftRow): void {
  if (editingKey.value !== d.key) return; // blur 晚到双触发护栏
  editingKey.value = "";
  const t = editDraft.value.trim();
  // 空名还原：已有行回打开时名，新增行回默认显示名
  d.name = t || (d.isNew ? (draft.value?.name ?? "") : d.origName);
}
function cancelEdit(): void {
  editingKey.value = "";
}

// —— 确认 = diff 三组一次提交（全空 = 直接关）——
async function onConfirm(): Promise<void> {
  if (busy.value) return;
  const adds: SyncLinkAdd[] = rows.value
    .filter((d) => d.isNew && !d.removed)
    .map((d) => ({ folderId: d.folderId, name: d.name.trim() }));
  const removes: SyncLinkRemove[] = rows.value
    .filter((d) => !d.isNew && d.removed)
    .map((d) => ({ folderId: d.folderId, origin: d.origin, name: d.origName }));
  const renames: SyncLinkRename[] = rows.value
    .filter((d) => !d.isNew && !d.removed && d.name.trim() !== "" && d.name.trim() !== d.origName)
    .map((d) => ({ folderId: d.folderId, origin: d.origin, oldName: d.origName, newName: d.name.trim() }));
  if (adds.length === 0 && removes.length === 0 && renames.length === 0) {
    onCancel();
    return;
  }
  busy.value = true;
  try {
    const ok = await syncAssetLinks(assetId.value, adds, removes, renames);
    if (ok) selection.linkManageDraft = null;
  } finally {
    busy.value = false;
  }
}

// 文件夹格样式：tint 经 toNewTint 转正（7.1 二十四轮迁新名）——顶级 = --n-custom-{tint}-2 暗档 /
// 子级 = 50% color-mix 派生（TreeItem 同口径，旧 --custom-{tint}/-bg 体系废除）；无 tint = 两档默认
function cellStyle(line: Line): Record<string, string> | undefined {
  const t = toNewTint(line.tint ?? undefined);
  if (!t) return undefined;
  return { background: line.depth === 0 ? `var(--n-custom-${t}-2)` : `color-mix(in srgb, var(--n-custom-${t}-2) 50%, transparent)` };
}
function toggleNode(id: string): void {
  if (collapsed.value.has(id)) collapsed.value.delete(id);
  else collapsed.value.add(id);
  collapsed.value = new Set(collapsed.value);
}
function toggleBlock(id: string): void {
  if (blockCollapsed.value.has(id)) blockCollapsed.value.delete(id);
  else blockCollapsed.value.add(id);
  blockCollapsed.value = new Set(blockCollapsed.value);
}
</script>

<template>
  <BaseModal title="链接管理" :width="602" framed class="link-manage" @close="onCancel">
    <!-- 搜索框行（过滤文件夹名，命中保留祖先全展开；inputs/TextInput lg + 左置 search 图标 +
         清除钮；framed 壳 padding 0 → 段自承 padding 0 24，MovePanel 同口径） -->
    <div class="search-row">
      <TextInput v-model="query" size="lg" leading-icon="search" clearable placeholder="搜索" />
    </div>
    <!-- 表头（稿：34 高 / padding-bottom 12 / 底线 1px 暗-悬停；两列均分 16 标题档标签色） -->
    <div class="head-row">
      <span class="head-cell">路径</span>
      <span class="head-cell">名称</span>
    </div>
    <!-- 树形容器（稿：604 高 / 组间 24；ScrollThumb 悬浮滚轮） -->
    <div class="tree-wrap">
      <div ref="bodyRef" class="tree-body kit-scroll">
        <div v-for="sec in sections" :key="sec.id" class="block-group">
          <!-- 组头（用户定"只是显示用处"：名称 - 数量 + 折叠；+ 钮不接线） -->
          <div class="group-head" @click="toggleBlock(sec.id)">
            <AppIcon name="tree-expand-arrow" :size="12" class="chev" :class="{ closed: blockCollapsed.has(sec.id) }" />
            <span class="group-title">{{ sec.name }} - {{ sec.count.toLocaleString() }}</span>
            <span class="spacer" />
            <IconButton name="add-small" variant="bare" size="sm" disabled title="添加" @click.stop />
          </div>
          <!-- 行（gap 4）：文件夹格 270 + 右列（名称框 232 + ✓）平铺多行 -->
          <template v-for="line in visibleLines(sec.lines)" :key="line.node.id">
            <div
              v-for="d in draftsOf(line.node.id).length > 0 ? draftsOf(line.node.id) : [null]"
              :key="d ? d.key : `${line.node.id}__empty`"
              class="line"
              :class="{ 'has-link': draftsOf(line.node.id).length > 0 }"
            >
              <!-- 文件夹格（稿：270×32 / padding 6 8 / 圆角 6；顶级 C5 + 1px 亮-选中 / 其他层级 #1F2020） -->
              <div class="folder-cell" :class="[line.depth === 0 ? 'root' : 'child']" :style="cellStyle(line)">
                <button
                  v-if="line.node.children.length > 0"
                  class="expand"
                  @click.stop="toggleNode(line.node.id)"
                >
                  <AppIcon name="tree-expand-arrow" :size="12" class="chev" :class="{ closed: collapsed.has(line.node.id) }" />
                </button>
                <span v-else class="expand placeholder" />
                <AppIcon :name="line.node.kind === 'archive' ? 'archive' : 'folder-closed'" :size="18" class="deco" />
                <span class="fname" v-tip="line.node.name">{{ line.node.name }}</span>
                <span class="count">{{ line.node.count.toLocaleString() }}</span>
              </div>
              <!-- 右列：有草稿 = 名称框 + 徽章 + 亮 ✓；无草稿 = 行 hover 出淡 ✓（稿"无链接"行右列空） -->
              <div v-if="d" class="link-cell" :class="{ editing: editingKey === d.key }">
                <input
                  v-if="editingKey === d.key"
                  ref="editInputRef"
                  v-model="editDraft"
                  class="name-edit"
                  maxlength="100"
                  @keydown.enter.prevent="commitEdit(d)"
                  @keydown.esc.prevent="cancelEdit"
                  @blur="commitEdit(d)"
                  @click.stop
                />
                <span v-else class="lname" v-tip="d.name" @click="startEdit(d)">{{ d.name }}</span>
                <AppIcon v-if="d.origin" name="original-file-badge" :size="15" class="badge" />
                <AppIcon v-else name="linked-file-badge" :size="13" class="badge link" />
                <button class="check on" title="移除链接" @click.stop="toggleRemove(d)">
                  <AppIcon name="check" :size="15" />
                </button>
              </div>
              <div v-else class="link-cell empty">
                <button class="check ghost" title="添加链接" @click.stop="addDraft(line.node.id)">
                  <AppIcon name="check" :size="15" />
                </button>
              </div>
            </div>
          </template>
        </div>
      </div>
      <ScrollThumb :body="bodyRef" />
    </div>
    <template #actions>
      <Checkbox v-model="onlyChecked" label="仅显示已选项" class="only-checked" />
      <span class="actions-spacer" />
      <ActionButton label="取消" variant="cancel" size="lg" :disabled="busy" @click="onCancel" />
      <ActionButton label="确认" variant="primary" size="lg" :disabled="busy" @click="onConfirm" />
    </template>
  </BaseModal>
</template>

<style scoped>
/* 17 稿：600×826（内容宽 552）——壳宽 602 由 BaseModal width prop 给（552 + padding 48 + 新壳 1px border×2；
   旧 1.5 border「+3 补偿」口径随壳换代改 +2，速查表 §1）；framed 壳 padding 0，各段自承 padding 0 24；
   小窗口兜底 max-height（旧 AppModal 同口径） */
.link-manage :deep(.bm-modal) {
  max-width: calc(100vw - 96px);
  max-height: calc(100vh - 96px);
}
/* 搜索框行（framed 段自承 padding 0 24，MovePanel 同口径） */
.search-row {
  padding: 0 var(--n-space-24);
  flex: none;
}
/* 表头（稿：34 高 / padding-bottom 12 / 底线 1px 暗-悬停；两列均分，16/22 700 标签色） */
.head-row {
  display: flex;
  gap: var(--n-space-12);
  padding: 0 var(--n-space-24) var(--n-space-12);
  border-bottom: 1px solid var(--n-l-a3);
  flex: none;
}
.head-cell {
  flex: 1;
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l3);
}
/* 树形容器（稿 604 高；悬浮滚轮 ScrollThumb 贴右缘）——framed 壳 padding 0，wrap 通栏自承
   padding-left 24 天然抵弹窗缘（旧负 margin 对随壳换代简化，2026-09-13 贴右缘口径沿），
   .tree-body padding-right 24 补回使行原位不动、滚轮贴缘 */
.tree-wrap {
  position: relative;
  height: 604px;
  min-height: 0;
  display: flex;
  flex-direction: column;
  padding-left: var(--n-space-24);
}
.tree-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-24);
  overflow-y: auto;
  overflow-x: hidden;
  padding-right: var(--n-space-24);
}
.block-group {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
}
/* 组头（稿：20 高 / padding-left 8；12/17 标签色；+ 钮不接线——组头纯显示） */
.group-head {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  height: 20px;
  padding-left: var(--n-space-8);
  cursor: pointer;
}
.group-title {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
}
.spacer {
  flex: 1;
}
.chev {
  color: var(--n-l3);
  flex: none;
}
/* 行（稿：32 高 gap 12；行间 4） */
.line {
  position: relative;
  display: flex;
  gap: var(--n-space-12);
  height: 32px;
  border-radius: var(--n-radius-6);
}
/* 行 hover（稿"悬停"态：整行 2px 亮-选中描边；::after 伪元素画防布局抖动，速查表 §7） */
.line:hover::after {
  content: "";
  position: absolute;
  inset: 0;
  border: 2px solid var(--n-l-a2);
  border-radius: var(--n-radius-6);
  pointer-events: none;
}
/* 文件夹格（稿：270×32 / padding 6 8 / gap 6 / 圆角 6；
   顶级 C5 + 1px 亮-选中 border（02_目录 item 根父级）/ 其他层级 #1F2020；tint 经 cellStyle 覆盖底色） */
.folder-cell {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  width: 270px;
  height: 32px;
  box-sizing: border-box;
  padding: var(--n-space-6) var(--n-space-8);
  border-radius: var(--n-radius-6);
  flex: none;
}
.folder-cell.root {
  background: var(--n-g5);
  border: 1px solid var(--n-l-a2);
}
.folder-cell.child {
  background: #1f2020;
}
.expand {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 12px;
  height: 12px;
  padding: 0;
  border: none;
  background: transparent;
  cursor: pointer;
  flex: none;
}
.expand.placeholder {
  cursor: default;
}
.deco {
  color: var(--n-l2);
  flex: none;
}
.fname {
  flex: 1;
  min-width: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.count {
  flex: none;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
  text-align: right;
}
/* 名称框（稿：232×32 / padding 6 8 / gap 6 / C4 底 / 1px 暗-悬停 / 圆角 6；
   聚焦（编辑）态 = n-g2 底 + 2px 品牌描边（2026-09-20 用户定 2px 档） */
.link-cell {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  width: 232px;
  height: 32px;
  box-sizing: border-box;
  padding: var(--n-space-6) var(--n-space-8);
  border-radius: var(--n-radius-6);
  background: var(--n-g4);
  box-shadow: inset 0 0 0 1px var(--n-l-a3);
  flex: none;
}
.link-cell.editing {
  background: var(--n-g2);
  box-shadow: inset 0 0 0 2px var(--n-brand);
}
.link-cell.empty {
  background: transparent;
  box-shadow: none;
  justify-content: flex-end;
}
.lname {
  flex: 1;
  min-width: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  cursor: pointer;
}
.name-edit {
  flex: 1;
  min-width: 0;
  background: transparent;
  border: none;
  outline: none;
  padding: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
}
.badge {
  flex: none;
}
.badge.link {
  color: var(--n-l3);
}
/* ✓（稿：18×18 位 check 15px；亮 = 常态色 / 淡 = 极淡——无链接行 hover 才出） */
.check {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  padding: 0;
  border: none;
  background: transparent;
  cursor: pointer;
  flex: none;
  color: var(--n-l2);
}
.check.ghost {
  color: var(--n-l4);
  opacity: 0;
  transition: opacity 0.12s ease;
}
.line:hover .check.ghost {
  opacity: 1;
}
/* 底栏：仅显示已选项居左（actions slot 右对齐口径上加左侧件）；
   framed 壳 padding 0 → 按钮行自承 padding（次左主右宽自适应，88 定宽随 AppButton 退役废除） */
.actions-spacer {
  flex: 1;
}
.only-checked {
  align-self: center;
}
.link-manage :deep(.bm-actions) {
  padding: 0 var(--n-space-24) var(--n-space-24);
  flex: none;
}
</style>
