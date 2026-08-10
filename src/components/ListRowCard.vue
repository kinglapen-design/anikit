<script setup lang="ts">
// 列表行（figma-css 列表显示稿，2026-08-10 对齐）：padding 12 / 高 69（定稿，滑块联动作废）/
// C2、C3 交替行底色 / 1px 暗-悬停边框（inset）/ 圆角 8；
// 缩略图 80×45 圆角 6 + 名称（14/20 高亮）；基础信息列 = 尺寸 82 / 格式 64 / 文件大小 64 /
// 修改日期 119，间距 24（14/20 标签色）——列区不再挂 card-info（2026-08-10 用户反馈：
// 基础信息点击无法选中，整行任意处点击都应选中本行）。
// 文件大小 / 修改日期仍是假占位：阶段 4 排序功能接真实值（后端 PlacementView 待补 size / modifiedAt）。
import { computed, nextTick, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Box } from "../lib/layouts";
import { isAudioExt } from "../lib/formatBadge";
import { formatDuration } from "../lib/formatDuration";
import { renamePlacement } from "../lib/placementOps";
import { useSelectionStore } from "../stores/selection";
import { useContentStore } from "../stores/content";
import type { PlacementView } from "../stores/content";

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
// 双击第一下 click 已把本行单选入域，此处仅置重命名目标）；
// 回收站视口只读不可重命名（2026-07-29 二轮，与 AssetCard 同口径）
function startRename(): void {
  if (useContentStore().isRecycleView) return;
  selection.renamingKey = rowKey;
}
</script>

<template>
  <div
    class="list-row"
    :class="{ odd, selected, renaming, 'fade-source': faded === 'source', 'fade-cut': faded === 'cut' }"
    :style="{ left: `${box.left}px`, top: `${box.top}px`, width: `${box.width}px`, height: `${box.height}px` }"
    @click="emit('select', $event)"
  >
    <div class="main">
      <img
        v-if="thumbUrl"
        class="thumb"
        :class="{ icon: asset.thumbKind === 'formatIcon' }"
        :src="thumbUrl"
        :alt="asset.name"
        decoding="async"
        draggable="false"
      />
      <div v-else class="thumb" />
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
    <!-- 基础信息列（figma 列表显示稿）：整行任意处点击都选中本行——不再挂 card-info
         （2026-08-10 用户反馈：列区点击无法选中） -->
    <div class="details">
      <span class="col size">{{ size }}</span>
      <span class="col format">{{ format }}</span>
      <span class="col filesize">123 MB</span>
      <span class="col date">2026/07/12 14:26</span>
    </div>
  </div>
</template>

<style scoped>
/* 列表行（figma 列表显示稿）：padding 12 / C2 底（奇行 C3 交替）/ 1px 暗-悬停边框（inset）/ 圆角 8 */
.list-row {
  position: absolute;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-12);
  border-radius: var(--radius-8);
  background: var(--c2);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  cursor: default;
}
/* C2 / C3 交替行底色（figma：首行 C2、次行 C3） */
.list-row.odd {
  background: var(--c3);
}
/* hover / 选中 / 重命名与文件夹卡片同口径（2026-08-10 用户定，列表不复杂无需实色专档）：
   1.5px 内阴影描边——hover 亮-选中（border-1），选中品牌色-边框线，重命名整行高亮 */
.list-row:hover {
  box-shadow: inset 0 0 0 1.5px var(--border-1);
}
.list-row.selected {
  box-shadow: inset 0 0 0 1.5px var(--brand-border);
}
.list-row.renaming {
  box-shadow: inset 0 0 0 1.5px var(--brand-border);
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
/* 缩略图（figma：80×45 圆角 6，定稿不随滑块）；contain 完整显示不裁切（行高固定 cover 必裁） */
.thumb {
  flex: none;
  display: block;
  width: 80px;
  height: 45px;
  border-radius: var(--radius-6);
  box-shadow: inset 0 0 0 1px var(--border-2);
  object-fit: contain;
  /* 无缩略图占位：中间面板底色 C1（阶段3修改——不要 font-icon-4 灰块） */
  background: var(--c1);
}
.name {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-highlight);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  /* 名称区悬停文本光标（2026-07-29 复验三轮：退出 card-info，双击可改名） */
  cursor: text;
}
/* 重命名输入（2026-08-10 统一口径：整行 1.5px 品牌色边框高亮提示，输入框透明底不再单独
   描边——同文件夹卡片 / 素材卡片）；高度恒 20 = 字体行高（不撑高行） */
.rename-input {
  flex: 1;
  min-width: 0;
  height: 20px;
  box-sizing: border-box;
  padding: 0;
  border: none;
  background: transparent;
  color: var(--text-highlight);
  font-size: var(--text-body-size);
  line-height: 20px;
  outline: none;
}
/* 基础信息列（figma）：间距 24 / 正文 14/20 标签色；列宽 82 / 64 / 64 / 119 与列头对齐 */
.details {
  display: flex;
  align-items: center;
  gap: var(--space-24);
  flex: none;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-label);
}
.col {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.size {
  width: 82px;
}
.format {
  width: 64px;
}
.filesize {
  width: 64px;
}
.date {
  width: 119px;
}
</style>
