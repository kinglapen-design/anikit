<script lang="ts">
// 行决策模型（DuplicateDialog 共用）：radio 三选一 + 跳过开关
export type DupAction = "link" | "copy" | "useExisting";
export interface DupDecision {
  action: DupAction;
  skipped: boolean;
}
</script>

<script setup lang="ts">
// 重复文件提示行（36_重复文件提示稿 item，922×149）：上 = 左右对照卡片（已存在 C4 / 新的文件 C2，
// 选"作为新文件导入"时新文件卡变 C4），下 = radio 三选一 + 行尾"跳过此文件"开关。
// 跳过 = 整行 30% 透明度（36 稿注明）且行内控件禁用（跳过开关除外——可逆切回）。
import { computed, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppRadio from "./AppRadio.vue";
import AppSwitch from "./AppSwitch.vue";
import type { ConflictItem } from "../stores/content";

const props = defineProps<{
  item: ConflictItem;
  decision: DupDecision;
  newName: string | null; // 批量重命名预览（仅 copy / link 会给；null = 用源文件名）
  waitingArea?: boolean; // 候车区（落点 = 未分类 root）：radio 收窄只留"作为新文件导入"
  findText?: string; // 替换命名模式的查找文本（非空时右卡显示源名 + 命中段高亮）
}>();
const emit = defineEmits<{ change: [patch: Partial<DupDecision>] }>();

const imgError = ref(false);

const IMAGE_EXTS = new Set(["png", "jpg", "jpeg", "webp", "gif", "bmp"]);
function extOf(name: string): string {
  const i = name.lastIndexOf(".");
  return i >= 0 ? name.slice(i + 1).toLowerCase() : "";
}

// 新文件预览：后端预览缓存（PSD/PSB/SVG 等）优先；图片格式直读源文件（asset 协议）；其余显示格式占位
const sourceUrl = computed(() => {
  const c = props.item;
  if (imgError.value) return null;
  if (c.sourceThumbPath) return convertFileSrc(c.sourceThumbPath);
  if (!IMAGE_EXTS.has(extOf(c.sourceName))) return null;
  return convertFileSrc(c.sourcePath);
});
const existingUrl = computed(() =>
  props.item.existingThumbPath ? convertFileSrc(props.item.existingThumbPath) : null,
);

// 右卡显示名：改名预览优先，否则源文件名
const displayName = computed(() => props.newName ?? props.item.sourceName);

// 查找命中高亮（2026-08-13 用户定，参考 Eagle：查找时原始名称命中片段品牌色高亮）——
// 替换模式且查找文本非空时，右卡显示源名并高亮命中段（新名预览让位：此刻要确认的是命中对不对）；
// 规则只作用词干，扩展名段恒不高亮
const hlParts = computed(() => {
  const find = props.findText ?? "";
  if (!find) return null;
  const name = props.item.sourceName;
  const dot = name.lastIndexOf(".");
  const stem = dot > 0 ? name.slice(0, dot) : name;
  const ext = dot > 0 ? name.slice(dot) : "";
  if (!stem.includes(find)) return null;
  const parts: { text: string; hit: boolean }[] = [];
  let i = 0;
  while (i < stem.length) {
    const j = stem.indexOf(find, i);
    if (j < 0) {
      parts.push({ text: stem.slice(i), hit: false });
      break;
    }
    if (j > i) parts.push({ text: stem.slice(i, j), hit: false });
    parts.push({ text: find, hit: true });
    i = j + find.length;
  }
  if (ext) parts.push({ text: ext, hit: false });
  return parts;
});

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}

function setAction(action: DupAction): void {
  emit("change", { action });
}
</script>

<template>
  <div class="dup-row" :class="{ skipped: decision.skipped }">
    <!-- 左右对照卡片（36 稿：449×69，padding 12，缩略图 80×45 + 名称 + 大小 + 所在位置标签） -->
    <div class="cards">
      <div class="file-card">
        <div class="thumb">
          <img v-if="existingUrl" :src="existingUrl" :alt="item.existingName" draggable="false" />
          <span v-else class="ext">{{ item.existingExt.toUpperCase() }}</span>
        </div>
        <span class="fname" :title="item.existingName">{{ item.existingName }}</span>
        <div class="meta">
          <span class="size">{{ formatSize(item.existingSize) }}</span>
          <span class="chip" :title="item.existingFolderName">{{ item.existingFolderName }}</span>
        </div>
      </div>
      <div class="file-card new" :class="{ solid: decision.action === 'copy' }">
        <div class="thumb">
          <img
            v-if="sourceUrl"
            :src="sourceUrl"
            :alt="item.sourceName"
            draggable="false"
            @error="imgError = true"
          />
          <span v-else class="ext">{{ extOf(item.sourceName).toUpperCase() }}</span>
        </div>
        <span class="fname" :title="displayName">
          <template v-if="hlParts">
            <span v-for="(p, k) in hlParts" :key="k" :class="{ hl: p.hit }">{{ p.text }}</span>
          </template>
          <template v-else>{{ displayName }}</template>
        </span>
        <div class="meta">
          <span class="size">{{ formatSize(item.sourceSize) }}</span>
          <span class="chip" :title="item.targetFolderName">{{ item.targetFolderName }}</span>
        </div>
      </div>
    </div>
    <!-- 决策行：radio 三选一（36 稿序：新链接 / 新文件 / 已存在）+ 行尾跳过开关；
         候车区收窄（2026-08-13 用户定）：落点 = 未分类时只留"作为新文件导入"——
         候车区不是分类位置，建链接 / 移入既有资产对候车区无意义（后端 resolve 同口径拒 root） -->
    <div class="controls">
      <div class="radios">
        <template v-if="!waitingArea">
          <AppRadio :model-value="decision.action" value="link" label="作为新链接导入" @update:model-value="setAction('link')" />
        </template>
        <AppRadio :model-value="decision.action" value="copy" label="作为新文件导入" @update:model-value="setAction('copy')" />
        <template v-if="!waitingArea">
          <AppRadio :model-value="decision.action" value="useExisting" label="用已存在文件导入" @update:model-value="setAction('useExisting')" />
        </template>
      </div>
      <div class="skip">
        <AppSwitch :model-value="decision.skipped" @update:model-value="emit('change', { skipped: $event })" />
        <span class="skip-label">跳过此文件</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* item（36 稿）：C3 底 + 下分隔线，padding 24 0，gap 12 */
.dup-row {
  display: flex;
  flex-direction: column;
  gap: var(--space-12);
  padding: var(--space-24) 0;
  background: var(--c3);
  box-shadow: inset 0 -1px 0 0 var(--line-hover);
  transition: opacity 0.2s ease;
}
/* 跳过：整行 30% 透明（36 稿注明）；卡片与 radio 禁用，跳过开关保持可点（可逆） */
.dup-row.skipped {
  opacity: 0.3;
}
.dup-row.skipped .cards,
.dup-row.skipped .radios {
  pointer-events: none;
}
/* 左右卡片行：gap 24 */
.cards {
  display: flex;
  gap: var(--space-24);
}
/* 文件卡片：449×69 / padding 12 / 圆角 8 / 1px 暗-悬停边框；
   已存在 = C4 底；新的文件 = C2 底，选"作为新文件导入"变 C4（36 稿注明） */
.file-card {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  padding: var(--space-12);
  border-radius: var(--radius-8);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
}
.file-card.new {
  background: var(--c2);
  transition: background 0.2s ease;
}
.file-card.new.solid {
  background: var(--c4);
}
/* 缩略图：80×45 / 圆角 6；无图占位 = 36 稿浅色块 + 扩展名 */
.thumb {
  width: 80px;
  height: 45px;
  border-radius: var(--radius-6);
  background: #c6c7cc; /* 浅色（文件夹卡片），36 稿缩略图占位色 */
  overflow: hidden;
  flex: none;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-right: var(--space-8);
}
.thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.ext {
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  font-weight: var(--text-title-weight);
  color: var(--c3);
}
/* 名称：正文 / 高亮，超出省略 */
.fname {
  flex: 1;
  min-width: 0;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-highlight);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 查找命中段：品牌色高亮（Eagle 参考，2026-08-13 用户定） */
.fname .hl {
  color: var(--brand-border);
}
/* 右侧：大小 + 所在位置标签，gap 12 */
.meta {
  display: flex;
  align-items: center;
  gap: var(--space-12);
  flex: none;
  margin-left: var(--space-8);
}
.size {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-label);
  white-space: nowrap;
}
/* 所在位置标签：C4 底 + 1px 暗-悬停边框 / 圆角 6 / padding 4 8 / 标签档 */
.chip {
  max-width: 120px;
  padding: var(--space-4) var(--space-8);
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  color: var(--text-label);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 决策行：radio 组与跳过开关两端分布 */
.controls {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
}
.radios {
  display: flex;
  gap: var(--space-12);
}
.skip {
  display: flex;
  align-items: center;
  gap: var(--space-6);
}
.skip-label {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-label);
}
</style>
