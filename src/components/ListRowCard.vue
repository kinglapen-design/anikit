<script setup lang="ts">
// 列表行（02_通用组件/02_item 10）：缩略图（可随滑块缩放）+ 名称 + 尺寸 / 格式 / 文件大小 / 修改日期。
// C3 / C1 交替行底色；1px border-2（inset）/ 圆角 8 / 内边距 6 8。
// 文件大小 / 修改日期仍是假占位：阶段 4 排序功能接真实值（后端 PlacementView 待补 size / modifiedAt）。
import { computed, nextTick, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Box } from "../lib/layouts";
import { isAudioExt } from "../lib/formatBadge";
import { formatDuration } from "../lib/formatDuration";
import { renamePlacement } from "../lib/placementOps";
import { useSelectionStore } from "../stores/selection";
import type { PlacementView } from "../stores/content";
import { useUiStore } from "../stores/ui";

const props = withDefaults(
  defineProps<{
    asset: PlacementView;
    box: Box;
    odd?: boolean;
    selected?: boolean;
    renaming?: boolean;
    faded?: "source" | "cut" | null; // 拖拽源半透明 / 被剪切半透明（2026-07-28 复验反馈 6①/8）
  }>(),
  { odd: false, selected: false, renaming: false, faded: null },
);
const ui = useUiStore();

// 缩略图随滑块缩放（80×45 基准 × 行高 / 220），等高不等宽限定容器内不裁剪
const thumbW = computed(() => Math.max(30, Math.round((80 * ui.targetRowHeight) / 220)));
const thumbH = computed(() => Math.max(17, Math.round((45 * ui.targetRowHeight) / 220)));
const thumbUrl = computed(() => (props.asset.thumbPath ? convertFileSrc(props.asset.thumbPath) : null));
// 尺寸列：音频显示时长（metadata 宽高是波形图尺寸非内容尺寸，5.1）；其余 w×h，无尺寸显示格式
const size = computed(() => {
  if (props.asset.duration != null && isAudioExt(props.asset.ext)) return formatDuration(props.asset.duration);
  return props.asset.w != null && props.asset.h != null
    ? `${props.asset.w} x ${props.asset.h}`
    : props.asset.ext.toUpperCase();
});
// 序列帧（2026-07-22 用户定）：格式列显示"序列帧"，名称后缀显示为 .序列帧（仅显示层，数据不动）；
// 动图（5.1）同带 frameCount，按 animated 区分——格式列显示扩展名大写
const isSeq = computed(() => props.asset.frameCount != null && !props.asset.animated);
const format = computed(() => (isSeq.value ? "序列帧" : props.asset.ext.toUpperCase()));
const displayName = computed(() =>
  isSeq.value ? `${props.asset.name.replace(/\.[^.]+$/, "")}.序列帧` : props.asset.name,
);
const emit = defineEmits<{ select: [e: MouseEvent] }>();

// —— 就地重命名（6.1 C2，与 AssetCard 同口径）：Enter 确认 / Esc 取消 / 失焦确认；
// 滚出虚拟滚动销毁 = 取消（AssetGrid 先清 renamingKey，blur 晚到按 key 失配判取消）
const selection = useSelectionStore();
const rowKey = `${props.asset.assetId}|${props.asset.folderId}`;
const renameDraft = ref("");
const renameInput = ref<HTMLInputElement>();
watch(
  () => props.renaming,
  (r) => {
    if (!r) return;
    renameDraft.value = props.asset.name;
    void nextTick(() => {
      const el = renameInput.value;
      if (!el) return;
      el.focus();
      const dot = props.asset.name.lastIndexOf(".");
      el.setSelectionRange(0, dot > 0 ? dot : props.asset.name.length); // 默认选中词干
    });
  },
  { immediate: true },
);
function cancelRename(): void {
  if (selection.renamingKey === rowKey) selection.renamingKey = null;
}
async function commitRename(): Promise<void> {
  if (selection.renamingKey !== rowKey) return;
  const name = renameDraft.value.trim();
  selection.renamingKey = null;
  if (!name || name === props.asset.name) return;
  await renamePlacement(
    { assetId: props.asset.assetId, folderId: props.asset.folderId, origin: props.asset.origin },
    name,
  );
}
// 双击名称 = 就地重命名（2026-07-29 复验三轮，renamingKey 既有机制——
// 双击第一下 click 已把本行单选入域，此处仅置重命名目标）
function startRename(): void {
  selection.renamingKey = rowKey;
}
</script>

<template>
  <div
    class="list-row"
    :class="{ odd, selected, 'fade-source': faded === 'source', 'fade-cut': faded === 'cut' }"
    :style="{ left: `${box.left}px`, top: `${box.top}px`, width: `${box.width}px`, height: `${box.height}px` }"
    @click="emit('select', $event)"
  >
    <div class="main">
      <img
        v-if="thumbUrl"
        class="thumb"
        :class="{ icon: asset.thumbKind === 'formatIcon' }"
        :style="{ width: `${thumbW}px`, height: `${thumbH}px` }"
        :src="thumbUrl"
        :alt="asset.name"
        decoding="async"
        draggable="false"
      />
      <div v-else class="thumb" :style="{ width: `${thumbW}px`, height: `${thumbH}px` }" />
      <input
        v-if="renaming"
        ref="renameInput"
        v-model="renameDraft"
        class="rename-input"
        @keydown.enter.prevent="commitRename"
        @keydown.esc.prevent="cancelRename"
        @blur="commitRename"
        @click.stop
        @dblclick.stop
        @pointerdown.stop
      />
      <!-- 名称（2026-07-29 复验三轮）：退出 card-info——悬停文本光标、可起拖拽 /
           单击正常选中、双击就地重命名（stop 防触发行双击预览） -->
      <span v-else class="name" @dblclick.stop="startRename">{{ displayName }}</span>
    </div>
    <div class="details card-info">
      <span class="col size">{{ size }}</span>
      <span class="col format">{{ format }}</span>
      <span class="col filesize">123 MB</span>
      <span class="col date">2026/07/12 14:26</span>
    </div>
  </div>
</template>

<style scoped>
.list-row {
  position: absolute;
  display: flex;
  align-items: center;
  justify-content: space-between;
  /* 上下 padding 8（2026-07-24 用户定，原 6） */
  padding: var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c1);
  box-shadow: inset 0 0 0 1px var(--border-2);
  cursor: default;
}
/* C3 / C1 交替行底色 */
.list-row.odd {
  background: var(--c3);
}
/* hover / 选中与素材卡片一致：2px 内阴影描边不用 fill；悬停 border-1，选中品牌色-1 */
.list-row:hover {
  box-shadow: inset 0 0 0 2px var(--border-1);
}
.list-row.selected {
  box-shadow: inset 0 0 0 2px var(--brand-1);
}
/* 拖拽源 / 被剪切整体半透明（2026-07-28 复验反馈 6①/8，token 透明度档） */
.list-row.fade-source {
  opacity: var(--opacity-drag-source);
}
.list-row.fade-cut {
  opacity: var(--opacity-cut);
}
/* 标题容器：缩略图 + 名称，间距 8 */
.main {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  flex: 1;
  min-width: 0;
}
.thumb {
  flex: none;
  display: block;
  border-radius: var(--radius-4);
  box-shadow: inset 0 0 0 1px var(--border-2);
  /* contain 完整显示不裁切（行高固定 cover 必裁，2026-07-24 用户定；格式图标同规则） */
  object-fit: contain;
  /* 无缩略图占位：中间面板底色 C1（阶段3修改——不要 font-icon-4 灰块） */
  background: var(--c1);
}
.name {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  /* 名称区悬停文本光标（2026-07-29 复验三轮：退出 card-info，双击可改名） */
  cursor: text;
}
/* 就地重命名输入（6.1 C2）：C4 底 / 1px border-1 / 圆角 6（输入框 token 口径）；
   高度恒 20 = 字体行高（2026-07-29 用户定：不撑高行）；聚焦描边品牌色-1（用户定） */
.rename-input {
  flex: 1;
  min-width: 0;
  height: 20px;
  box-sizing: border-box;
  padding: 0 var(--space-6);
  border: none;
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
  color: var(--font-icon-1);
  font-size: var(--text-body-size);
  line-height: 20px;
  outline: none;
}
.rename-input:focus {
  box-shadow: inset 0 0 0 1px var(--brand-1);
}
/* 详情容器：尺寸 / 格式 / 文件大小 / 修改日期，正文 font-icon-3，固定列宽与列头对齐 */
.details {
  display: flex;
  align-items: center;
  gap: var(--space-16);
  flex: none;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-3);
}
.col {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.size {
  width: 90px;
}
.format {
  width: 50px;
}
.filesize {
  width: 70px;
}
.date {
  width: 130px;
}
</style>
