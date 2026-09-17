<script setup lang="ts">
// 批量重命名弹窗（37 稿，3.4，2026-08-18）：双列对照（原始文件名 / 新的文件名）+ 底部
// 模式切换（AppSegmented：序号命名 / 替换命名——同一时刻只显示当前模式的一行输入，PRD 02-14）。
// 实时预览只读无"应用"钮：序号 = 名称 + 起始编号（位宽按选中数 <100 两位 / ≥100 三位）；
// 替换 = 查找文本全处替换，原名称匹配段品牌色高亮（Eagle 交互参考：视觉上把高亮部分换成新的）。
// 规则只作用词干、扩展名不动；撞名不拦截（后端 rename_placements 不唯一化）；
// 失败项跳过 toast 汇总不整体回滚；无变化项不提交（名称/查找为空 → 该项保持原名）。
import { computed, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppModal from "./AppModal.vue";
import AppInput from "./AppInput.vue";
import AppButton from "./AppButton.vue";
import AppSegmented from "./AppSegmented.vue";
import ScrollThumb from "./ScrollThumb.vue";
import { useContentStore } from "../stores/content";
import { useSelectionStore, type PlacementRef } from "../stores/selection";
import { renamePlacementsBatch, type RenameItemInput } from "../lib/placementOps";

const content = useContentStore();
const selection = useSelectionStore();

// —— 行数据（renameDraft = 打开时刻选中集快照；名称/缩略图读 content.placements 实况）——
interface Row {
  ref: PlacementRef;
  name: string;
  thumb: string | null;
  contain: boolean; // 格式图标 contain 不裁切（速查表 §6）
  ext: string;
}
const rows = computed<Row[]>(() => {
  const draft = selection.renameDraft ?? [];
  const byKey = new Map(content.placements.map((p) => [`${p.assetId}|${p.folderId}|${p.origin}`, p]));
  return draft.map((ref) => {
    const p = byKey.get(`${ref.assetId}|${ref.folderId}|${ref.origin}`);
    return {
      ref,
      name: p?.name ?? "",
      thumb: p?.thumbPath ? convertFileSrc(p.thumbPath) : null,
      contain: p?.thumbKind === "formatIcon",
      ext: p?.ext ?? "",
    };
  });
});

// —— 模式与输入（segmented 切换，单显一行；起始编号非数字回退 1）——
const MODES = [
  { value: "sequence", label: "序号命名" },
  { value: "replace", label: "替换命名" },
] as const;
const mode = ref<string>("sequence");
const seqName = ref("");
const seqStart = ref("1");
const findText = ref("");
const replaceText = ref("");

// TAB 循环聚焦（2026-09-13 实机走查：输入一个后手不离键盘——查找↔替换、名称↔起始编号互跳）
const seqNameRef = ref<InstanceType<typeof AppInput>>();
const seqStartRef = ref<InstanceType<typeof AppInput>>();
const findRef = ref<InstanceType<typeof AppInput>>();
const replaceRef = ref<InstanceType<typeof AppInput>>();

// 位宽自动（PRD 02-14：<100 两位、≥100 三位）
const padWidth = computed(() => (rows.value.length < 100 ? 2 : 3));
const startNum = computed(() => {
  const n = parseInt(seqStart.value, 10);
  return Number.isFinite(n) && n >= 0 ? n : 1;
});

// 词干/扩展名拆分（与后端 unique_name 同规则：末点为界、无扩展名或点为首不拆）
function splitName(name: string): { stem: string; ext: string } {
  const i = name.lastIndexOf(".");
  return i > 0 ? { stem: name.slice(0, i), ext: name.slice(i) } : { stem: name, ext: "" };
}

// 新词干（null = 该项不改名：序号模式名称为空 / 替换模式查找为空或无匹配）
function newStem(index: number, stem: string): string | null {
  if (mode.value === "sequence") {
    const base = seqName.value.trim();
    if (!base) return null;
    return `${base}${String(startNum.value + index).padStart(padWidth.value, "0")}`;
  }
  if (!findText.value || !stem.includes(findText.value)) return null;
  return stem.split(findText.value).join(replaceText.value);
}
function previewName(row: Row, index: number): string {
  const { stem, ext } = splitName(row.name);
  const ns = newStem(index, stem);
  return ns === null ? row.name : ns + ext;
}

// 原名称高亮分段（替换模式：匹配段品牌色高亮；其余模式/无匹配 = 整段不高亮）
function stemSegments(stem: string): { text: string; hit: boolean }[] {
  const f = findText.value;
  if (mode.value !== "replace" || !f) return [{ text: stem, hit: false }];
  const out: { text: string; hit: boolean }[] = [];
  let rest = stem;
  for (;;) {
    const i = rest.indexOf(f);
    if (i < 0) {
      if (rest) out.push({ text: rest, hit: false });
      break;
    }
    if (i > 0) out.push({ text: rest.slice(0, i), hit: false });
    out.push({ text: f, hit: true });
    rest = rest.slice(i + f.length);
  }
  return out;
}

// 可执行项（有变化才提交）；全不变更确认钮禁用
const pendingItems = computed<RenameItemInput[]>(() =>
  rows.value
    .map((row, i) => ({ row, i }))
    .filter(({ row, i }) => newStem(i, splitName(row.name).stem) !== null)
    .map(({ row, i }) => ({
      assetId: row.ref.assetId,
      folderId: row.ref.folderId,
      origin: row.ref.origin,
      newName: previewName(row, i),
    })),
);

const busy = ref(false);
async function onConfirm(): Promise<void> {
  if (pendingItems.value.length === 0 || busy.value) return;
  busy.value = true;
  const ok = await renamePlacementsBatch(pendingItems.value);
  busy.value = false;
  if (ok) selection.renameDraft = null; // 部分失败也关（toast 已汇总，撤销入口在 toast）
}
function onClose(): void {
  selection.renameDraft = null;
}

const listRef = ref<HTMLElement>();
</script>

<template>
  <AppModal v-if="selection.renameDraft" title="批量重命名" class="rename-dialog" @close="onClose">
    <!-- 表头（37 稿：双列标题档 16/22 标签色 / 下分隔线 / 列 gap 24） -->
    <header class="rd-head">
      <span class="col">原始文件名</span>
      <span class="col">新的文件名</span>
    </header>
    <!-- 行列表（滚轮 = ScrollThumb 悬浮件） -->
    <div class="rd-list-wrap">
      <div ref="listRef" class="rd-list kit-scroll">
        <div v-for="(row, i) in rows" :key="`${row.ref.assetId}|${row.ref.folderId}|${row.ref.origin}`" class="rd-row">
          <div class="rd-card">
            <span class="rd-thumb">
              <img v-if="row.thumb" :src="row.thumb" :class="{ icon: row.contain }" alt="" />
              <span v-else class="ph">{{ row.ext || "—" }}</span>
            </span>
            <span class="rd-name" :title="row.name">
            <template v-for="(seg, j) in stemSegments(splitName(row.name).stem)" :key="j"
              ><span v-if="seg.hit" class="hit">{{ seg.text }}</span
              ><template v-else>{{ seg.text }}</template></template
            ><span class="ext">{{ splitName(row.name).ext }}</span>
            </span>
          </div>
          <div class="rd-card">
            <span class="rd-thumb">
              <img v-if="row.thumb" :src="row.thumb" :class="{ icon: row.contain }" alt="" />
              <span v-else class="ph">{{ row.ext || "—" }}</span>
            </span>
            <span class="rd-name" :title="previewName(row, i)">{{ previewName(row, i) }}</span>
          </div>
        </div>
      </div>
      <ScrollThumb :body="listRef" />
    </div>
    <!-- 输入区（同一时刻只显示当前模式一行；PRD 02-14；TAB 两框互跳——2026-09-13 实机走查） -->
    <div class="rd-fields">
      <template v-if="mode === 'sequence'">
        <span class="lbl">名称:</span>
        <AppInput ref="seqNameRef" v-model="seqName" size="medium" clearable class="grow" placeholder="输入名称" @keydown.tab.prevent="seqStartRef?.focus()" />
        <span class="lbl">起始编号:</span>
        <AppInput ref="seqStartRef" v-model="seqStart" size="medium" clearable class="narrow" placeholder="1" @keydown.tab.prevent="seqNameRef?.focus()" />
      </template>
      <template v-else>
        <span class="lbl">查找文本:</span>
        <AppInput ref="findRef" v-model="findText" size="medium" clearable class="grow" placeholder="输入查找文本" @keydown.tab.prevent="replaceRef?.focus()" />
        <span class="lbl">替换为:</span>
        <AppInput ref="replaceRef" v-model="replaceText" size="medium" clearable class="grow" placeholder="输入替换文本" @keydown.tab.prevent="findRef?.focus()" />
      </template>
    </div>
    <!-- 底部栏（37 稿：62 高 / padding 12 12 12 24 / 顶部分隔线；左 segmented 右按钮组主左次右） -->
    <footer class="rd-foot">
      <AppSegmented v-model="mode" :options="MODES" />
      <span class="spacer" />
      <AppButton variant="primary" label="确认" :disabled="pendingItems.length === 0 || busy" @click="onConfirm" />
      <AppButton variant="panel" label="取消" @click="onClose" />
    </footer>
  </AppModal>
</template>

<style scoped>
/* 37 稿壳：970 内容宽 + 真 border 3 补偿 = 973（速查表 §1）；高 750 稿值 + 窗口封顶；
   壳 padding 0（各区自负边距）gap 0（稿相邻区紧贴：表头线 / 行线 / 按钮栏线各自承担） */
.rename-dialog :deep(.app-modal) {
  width: 973px;
  height: min(753px, calc(100vh - 96px));
  padding: 0;
  gap: 0;
}
.rename-dialog :deep(.titlebar) {
  height: 54px;
  padding: var(--space-12) var(--space-12) var(--space-12) var(--space-24);
  border-bottom: 1px solid var(--line-hover);
  flex: none;
}
.rename-dialog :deep(.content) {
  flex: 1;
  gap: 0;
  min-height: 0;
  /* 壳 padding 0：归零 AppModal .content 的负 margin 对（2026-09-13 实机走查贴右缘口径）——
     各区自承边距（.rd-head margin 0 24 / .rd-list padding 0 24 等），.rd-list-wrap 通栏，
     归零后 thumb 自然贴弹窗缘 */
  margin-right: 0;
  padding-right: 0;
}
/* 表头（37 稿：46 高 / 0 24 容器内 / 下分隔线暗-悬停；双列 flex 1 对齐行内双卡） */
.rd-head {
  display: flex;
  align-items: center;
  gap: var(--space-24);
  height: 46px;
  margin: 0 var(--space-24);
  border-bottom: 1px solid var(--line-hover);
  flex: none;
}
.rd-head .col {
  flex: 1;
  min-width: 0;
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--text-label);
}
/* 行列表：0 24 容器（稿 922 内容宽）；行 = padding 24 0 + C3 底 + 下边框（稿 item 117 高） */
.rd-list-wrap {
  position: relative;
  flex: 1;
  min-height: 0;
}
.rd-list {
  height: 100%;
  overflow-y: auto;
  padding: 0 var(--space-24);
}
.rd-row {
  display: flex;
  gap: var(--space-24);
  padding: var(--space-24) 0;
  background: var(--c2); /* 2026-08-19 用户定：item 底色 C3 → C2 */
  border-bottom: 1px solid var(--line-hover);
}
/* 名称卡（37 稿：449×69 / C4 底 / 1px 暗-悬停 border / 圆角 8 / padding 12 / gap 8） */
.rd-card {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  flex: 1;
  min-width: 0;
  height: 69px;
  padding: var(--space-12);
  background: var(--c4);
  border: 1px solid var(--line-hover);
  border-radius: var(--radius-8);
}
.rd-thumb {
  width: 80px;
  height: 45px;
  border-radius: var(--radius-6);
  overflow: hidden;
  flex: none;
}
.rd-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.rd-thumb img.icon {
  object-fit: contain;
}
/* 无图占位（速查表 §6）：浅色块 #C6C7CC + 扩展名文字 */
.rd-thumb .ph {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #c6c7cc;
  font-size: var(--text-label-size);
  font-weight: var(--text-title-weight);
  color: var(--c3);
}
.rd-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-highlight);
}
/* 替换模式匹配段：品牌色高亮（Eagle 交互参考——原名称被替换部分一眼可辨） */
.rd-name .hit {
  color: var(--brand);
}
/* 输入区（稿：label 14/20 标签色；输入框 C1 底 36 高 padding 8 12——medium 提权覆盖） */
.rd-fields {
  display: flex;
  align-items: center;
  gap: var(--space-12);
  padding: var(--space-12) var(--space-24);
  flex: none;
}
.rd-fields .lbl {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-label);
  flex: none;
  white-space: nowrap;
}
.rd-fields .grow {
  flex: 1;
  min-width: 0;
}
.rd-fields .narrow {
  width: 82px;
  flex: none;
}
.rd-fields :deep(.app-input-shell.medium) {
  padding: var(--space-8) var(--space-12);
}
/* 底部栏（37 稿：62 高 / padding 12 12 12 24 / 顶部分隔线 / 右组主左次右 88×38） */
.rd-foot {
  display: flex;
  align-items: center;
  gap: var(--space-12);
  height: 62px;
  padding: var(--space-12) var(--space-12) var(--space-12) var(--space-24);
  border-top: 1px solid var(--line-hover);
  flex: none;
}
.rd-foot .spacer {
  flex: 1;
}
.rd-foot :deep(.app-btn) {
  min-width: 88px;
}
</style>
