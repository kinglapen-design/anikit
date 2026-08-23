<script setup lang="ts">
// 详情区块栈共享组合（2026-08-15 任务 A 三项，自 DetailPanel 抽出）：
// pinned 缩略图区块（恒第一，体系外）+ DetailNotes（可选，仅文件）+ 注册表 7 区块按
// 配置排序显隐渲染（**文件 / 档案各一套**：ui.detailSections / detailSectionsArchive，
// 2026-08-23 用户定独立设置，按 target.kind 分流）。DetailPanel（右侧详情面板）与
// ArchiveSidePanel（档案二级侧边栏内容区）复用同一套，避免两套拷贝。
// 面板内拖拽排序（2026-08-15 定案）：拖区块标题行重排，让位动画 = 侧栏区块标题拖拽
//（LibraryView blockDrag）同款本地 pointer 追踪；pinned / DetailNotes 不参与；松手写
// ui.moveDetailSectionTo；拖出面板取消兜底 = buttons 自清 + blur/mouseleave（侧栏范式）。
import { computed, reactive, ref, type Component } from "vue";
import DetailNotes from "./DetailNotes.vue";
import SectionPinned from "./sections/SectionPinned.vue";
import SectionCustom from "./sections/SectionCustom.vue";
import SectionInfo from "./sections/SectionInfo.vue";
import SectionLinks from "./sections/SectionLinks.vue";
import SectionEmpty from "./sections/SectionEmpty.vue";
import { detailSectionTitle, type DetailTarget } from "../lib/detailSections";
import { dragView } from "../lib/dragOperation";
import { useUiStore } from "../stores/ui";

const props = withDefaults(defineProps<{ target: DetailTarget; notes?: boolean }>(), { notes: false });
const ui = useUiStore();

// 区块 id → 组件（副本 / 关系 / 音频 / 提示词一轮空态共用 SectionEmpty）
const SECTION_COMPS: Record<string, Component> = {
  custom: SectionCustom,
  info: SectionInfo,
  copies: SectionEmpty,
  links: SectionLinks,
  relations: SectionEmpty,
  audio: SectionEmpty,
  prompt: SectionEmpty,
};

// 可见区块（当前目标套配置顺序，visible 过滤；标题取注册表）：
// 文件与档案各一套独立配置（2026-08-23 用户定，ui.detailSections / detailSectionsArchive）——
// 排序 / 显隐按 target.kind 分流，档案侧改动不再串到文件详情
const sectionHost = computed(() => (props.target.kind === "archive" ? ("archive" as const) : ("file" as const)));
const sections = computed(() =>
  ui.sectionsFor(sectionHost.value).filter((s) => s.visible).map((s) => ({ id: s.id, title: detailSectionTitle(s.id) })),
);

// —— 面板内拖拽排序（LibraryView blockDrag 同款让位范式；区块高不定，DOM 快照中线判定） ——
const panelRef = ref<HTMLElement>();
const secDrag = reactive({ id: "", startY: 0, dy: 0, from: -1, to: -1 });
const secSettling = ref(false); // 松手归位帧：禁块 transform 过渡
let secMoved = false; // 超阈值视为拖动：松手后抑制折叠 click
let secSpans: { mid: number }[] = []; // 各块中线（拖动起时 DOM 快照，块间无 gap）
let secStep = 0; // 被拖块高（让位位移量）

// 可排序块 = 全部 detail-block 除去 pinned（缩略图区块）与 DetailNotes（体系外），DOM 序 = sections 序
function sortableEls(): HTMLElement[] {
  const panel = panelRef.value;
  if (!panel) return [];
  return [...panel.querySelectorAll<HTMLElement>(".detail-block:not(.pinned):not(.detail-notes)")];
}

function onSectionPointerDown(e: PointerEvent): void {
  if (e.button !== 0) return;
  const t = e.target as HTMLElement;
  if (t.closest(".icon-btn")) return; // 按钮上不启动拖拽
  const header = t.closest(".section-header");
  if (!header) return; // 仅区块标题行可拖
  const block = header.closest(".detail-block");
  if (!block || block.classList.contains("pinned") || block.classList.contains("detail-notes")) return;
  const els = sortableEls();
  const from = els.indexOf(block as HTMLElement);
  if (from < 0) return;
  let acc = 0;
  secSpans = els.map((el) => {
    const h = el.offsetHeight;
    const s = { mid: acc + h / 2 };
    acc += h;
    return s;
  });
  secStep = els[from]?.offsetHeight ?? 0;
  Object.assign(secDrag, { id: sections.value[from]?.id ?? "", startY: e.clientY, dy: 0, from, to: from });
  secMoved = false;
  window.addEventListener("pointermove", onSecDragMove);
  window.addEventListener("pointerup", onSecDragEnd, { once: true });
  window.addEventListener("blur", onSecDragCancel, { once: true });
  document.addEventListener("mouseleave", onSecDragCancel, { once: true });
}

// 拖动块中线越过邻块中线 → 目标位迁移（让位动画由 sectionStyle 驱动）
function onSecDragMove(e: PointerEvent): void {
  if (!secDrag.id) return;
  if ((e.buttons & 1) === 0) {
    onSecDragCancel(); // 僵尸自清：pointerup 丢失（窗外松手等，与引擎同口径）
    return;
  }
  secDrag.dy = e.clientY - secDrag.startY;
  if (Math.abs(secDrag.dy) > 3) secMoved = true;
  const c = (secSpans[secDrag.from]?.mid ?? 0) + secDrag.dy;
  let to = secDrag.from;
  for (let i = 0; i < secSpans.length; i++) {
    if (i > secDrag.from && c > secSpans[i].mid) to = i; // 下移：越过几个中线落几位
    // 上移：首个被越过的中线即落点（须 break——不 break 时更大的 i 也会命中并把 to 盖回去，
    // 跨多块上移会少落一位；LibraryView 原循环未 break，连续小步移动时不暴露，大步跨越才现形）
    if (i < secDrag.from && c < secSpans[i].mid) {
      to = i;
      break;
    }
  }
  secDrag.to = to;
}

function onSecDragEnd(): void {
  offSecDrag();
  const { from, to } = secDrag;
  const moved = secMoved;
  secDrag.id = "";
  if (!moved || from < 0 || to === from) return;
  // 可见序 → 全量序映射（隐藏区块不渲染，落点按目标可见块的全量 index 写；host = 当前目标套）
  const draggedId = sections.value[from]?.id;
  const targetId = sections.value[to]?.id;
  if (draggedId && targetId) {
    const fullTo = ui.sectionsFor(sectionHost.value).findIndex((s) => s.id === targetId);
    ui.moveDetailSectionTo(draggedId, fullTo, sectionHost.value);
  }
  // 抑制紧随 click 误折叠（与引擎 suppressClick 同口径，SectionHeader.onFold 已消费）
  dragView.suppressClick = true;
  setTimeout(() => (dragView.suppressClick = false), 0);
  // 松手即到位：让位块视觉本已在目标位，清 transform 必须禁过渡（库菜单 2026-08-12 走查同坑）
  secSettling.value = true;
  setTimeout(() => (secSettling.value = false), 50);
}

function onSecDragCancel(): void {
  offSecDrag();
  secDrag.id = "";
}

function offSecDrag(): void {
  window.removeEventListener("pointermove", onSecDragMove);
  window.removeEventListener("pointerup", onSecDragEnd);
  window.removeEventListener("blur", onSecDragCancel);
  document.removeEventListener("mouseleave", onSecDragCancel);
}

// 让位位移：拖动块跟随指针（无过渡、置顶、C3 底盖缝）；被跨越块反向让出一步
//（transition 在 detail-block 公共规则，归位帧 .settling 禁过渡）
function sectionStyle(i: number): string {
  if (!secDrag.id) return "";
  if (i === secDrag.from) {
    return `transform: translateY(${secDrag.dy}px); transition: none; position: relative; z-index: 2; background: var(--c3);`;
  }
  if (secSettling.value) return "";
  const { from, to } = secDrag;
  if (from < to && i > from && i <= to) return `transform: translateY(${-secStep}px)`;
  if (to < from && i >= to && i < from) return `transform: translateY(${secStep}px)`;
  return "";
}
</script>

<template>
  <div
    ref="panelRef"
    class="detail-panel"
    :class="{ settling: secSettling }"
    @pointerdown="onSectionPointerDown"
  >
    <!-- 缩略图区块：独立永远置顶（不可隐藏/排序，不进注册表） -->
    <SectionPinned :target="props.target" />
    <!-- 笔记区块（体系外，5.2 既有业务）：有笔记才显示，固定插缩略图区块之后、排序区块之前 -->
    <DetailNotes v-if="notes && props.target.kind === 'file'" />
    <component
      v-for="(s, i) in sections"
      :key="s.id"
      :is="SECTION_COMPS[s.id]"
      :target="props.target"
      :title="s.title"
      :style="sectionStyle(i)"
    />
  </div>
</template>

<style scoped>
/* 详情容器：区块统一规则（2026-07-23 用户定）——内边距 12 16 / 间距 8 /
   通栏分割线 border-2 以 inset box-shadow 绘制（在背景之上，hover 底色 C2 盖不住；
   首个区块不画）；侧栏外层无 padding，分割线横贯。
   transform 过渡 = 面板内拖拽让位动画（2026-08-15 定案，侧栏区块标题同款；
   拖动块 inline 禁过渡跟随指针，归位帧 .settling 禁过渡防"弹回去"） */
.detail-panel {
  display: flex;
  flex-direction: column;
}
.detail-panel :deep(.detail-block) {
  gap: var(--space-8);
  padding: var(--space-12) var(--space-16);
  box-shadow: inset 0 1px 0 0 var(--border-2);
  transition: transform 0.18s ease;
}
.detail-panel.settling :deep(.detail-block) {
  transition: none;
}
.detail-panel :deep(.detail-block:first-child) {
  box-shadow: none;
}
/* 缩略图区块（pinned：无副标题、永远置顶不参与排序）——公共规则与本规则均不生效，
   实际 padding/gap 由 SectionPinned 内覆盖归零后组自控（稿为准） */
.detail-panel :deep(.detail-block.pinned) {
  gap: var(--space-16);
  padding: var(--space-8) var(--space-16) 12px;
}
</style>
