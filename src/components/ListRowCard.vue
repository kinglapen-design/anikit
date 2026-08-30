<script setup lang="ts">
// 列表行业务壳（7.1 内容容器轮改造）：视觉芯 = cards/ListRowCard（57 高 / padding 6/12/6/6 / 列 123 对齐表头，
// 斑马行废除——新稿统一 通用/二级底），本壳承担业务装配——
// box 绝对定位（虚拟滚动）/ 就地重命名（selection.renamingKey 驱动，芯 naming 位）/
// 拖拽半透明 / 位置徽章（origin ★ / 链接 icon，活位置 > 1 才上，§6 口径；行级悬停链接列表
// 稿未画不接，09 待办）/ 文件大小·修改日期接真值（PlacementView.size / modifiedAt——
// 旧假占位（"123 MB" 写死）本轮废除，阶段 4 欠账清）。
// 尺寸列：音频显示时长（metadata 宽高是波形图尺寸非内容尺寸，5.1）；其余 w×h，无尺寸显示格式。
import { computed } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import RowCard from "./cards/ListRowCard.vue";
import type { Box } from "../lib/layouts";
import { displayName as assetDisplayName, isAudioExt, isSequence } from "../lib/formatBadge";
import { formatDuration } from "../lib/formatDuration";
import { formatSize } from "../lib/formatSize";
import { formatDetailDate } from "../lib/detailSections";
import { renamePlacement } from "../lib/placementOps";
import { useSelectionStore } from "../stores/selection";
import { useContentStore } from "../stores/content";
import { useUiStore } from "../stores/ui";
import type { PlacementView } from "../stores/content";

const props = withDefaults(
  defineProps<{
    asset: PlacementView;
    box: Box;
    odd?: boolean; // 旧斑马行参数（7.1 起废除——新稿统一底色；接口保留兼容 AssetGrid 传参）
    selected?: boolean;
    renaming?: boolean;
    faded?: "source" | "cut" | null; // 拖拽源半透明 / 被剪切半透明（2026-07-28 复验反馈 6①/8）
  }>(),
  { odd: false, selected: false, renaming: false, faded: null },
);

const ui = useUiStore();
const selection = useSelectionStore();
const content = useContentStore();
const emit = defineEmits<{ select: [e: MouseEvent] }>();

// ?v=modifiedAt = 缩略图 URL 版本戳（同 AssetCard：设封面后 URL 变强制 img 重载，asset 协议缓存修复）
const thumbUrl = computed(() =>
  props.asset.thumbPath ? `${convertFileSrc(props.asset.thumbPath)}?v=${encodeURIComponent(props.asset.modifiedAt ?? "")}` : null,
);
// 尺寸列：音频显示时长（5.1）；其余 w×h，无尺寸显示格式
const size = computed(() => {
  if (props.asset.duration != null && isAudioExt(props.asset.ext)) return formatDuration(props.asset.duration);
  return props.asset.w != null && props.asset.h != null
    ? `${props.asset.w} x ${props.asset.h}`
    : props.asset.ext.toUpperCase();
});
// 序列帧（2026-07-22 用户定）：格式列显示"序列帧"，名称后缀显示为 .序列帧（仅显示层，数据不动）；
// 动图（5.1）同带 frameCount，按 animated 区分——格式列显示扩展名大写
const isSeq = computed(() => isSequence(props.asset));
const format = computed(() => (isSeq.value ? "序列帧" : props.asset.ext.toUpperCase()));
const displayName = computed(() => assetDisplayName(props.asset, ui.showExt));
// 文件大小 / 修改日期真值（7.1 接）：日期时间两段 = formatDetailDate 拆分（YYYY/MM/DD + HH:mm）
const fileSize = computed(() => formatSize(props.asset.size));
const modParts = computed(() => {
  const s = formatDetailDate(props.asset.modifiedAt);
  const i = s.indexOf(" ");
  return i > 0 ? { date: s.slice(0, i), time: s.slice(i + 1) } : { date: s, time: "" };
});
// 位置徽章（§6 口径）：活位置 > 1 才上——origin ★ / 链接 icon
const linkCount = computed(() => content.placementCountByAsset.get(props.asset.assetId) ?? 0);
const originBadge = computed(() => props.asset.origin && linkCount.value > 1);
const linkedBadge = computed(() => !props.asset.origin && linkCount.value > 1);

// —— 就地重命名（6.1 C2，与 AssetCard 同口径）：Enter / 失焦确认（空或不变不动）/ Esc 取消；
// 滚出虚拟滚动销毁 = 取消（AssetGrid 先清 renamingKey，blur 晚到按 key 失配判取消）
const rowKey = `${props.asset.assetId}|${props.asset.folderId}`;
async function onRenameCommit(name: string): Promise<void> {
  selection.renamingKey = null;
  if (!name || name === props.asset.name) return;
  await renamePlacement(
    { assetId: props.asset.assetId, folderId: props.asset.folderId, origin: props.asset.origin },
    name,
  );
}
function onRenameCancel(): void {
  selection.renamingKey = null;
}
// 双击名称 = 就地重命名（2026-07-29 复验三轮，renamingKey 既有机制——
// 双击第一下 click 已把本行单选入域，此处仅置重命名目标）；
// 回收站视口只读不可重命名（2026-07-29 二轮，与 AssetCard 同口径闸）
function startRename(): void {
  if (content.isRecycleView) return;
  selection.renamingKey = rowKey;
}
</script>

<template>
  <div
    class="list-row-shell"
    :class="{ 'fade-source': faded === 'source', 'fade-cut': faded === 'cut' }"
    :style="{ left: `${box.left}px`, top: `${box.top}px`, width: `${box.width}px`, height: `${box.height}px` }"
    @click="emit('select', $event)"
  >
    <RowCard
      :name="displayName"
      :thumb-url="thumbUrl ?? ''"
      :badge="format"
      :dims="size"
      :file-size="fileSize"
      :mod-date="modParts.date"
      :mod-time="modParts.time"
      :origin-badge="originBadge"
      :linked-badge="linkedBadge"
      :selected="selected"
      :naming="renaming"
      @name-edit="startRename"
      @name-commit="onRenameCommit"
      @name-cancel="onRenameCancel"
    />
  </div>
</template>

<style scoped>
/* 壳：虚拟滚动 box 绝对定位；选中 / 重命名环与 hover 全归 cards/ListRowCard（kc 族） */
.list-row-shell {
  position: absolute;
  cursor: default;
}
/* 拖拽源 / 被剪切整体半透明（2026-07-28 复验反馈 6①/8，token 透明度档） */
.list-row-shell.fade-source {
  opacity: var(--opacity-drag-source);
}
.list-row-shell.fade-cut {
  opacity: var(--opacity-cut);
}
</style>
