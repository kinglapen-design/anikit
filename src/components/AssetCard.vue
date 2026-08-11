<script setup lang="ts">
// 素材卡片（figma-css 01_组件/14_卡片，2026-08-10 重做定稿）：
// 整卡 1px 暗-悬停边框 / 圆角 6 / padding 0；缩略图区 C3 圆角上沿，简介区 C3 + 上边框圆角下沿
// （padding 8 8 8 12 / 间距 4）；名称单行左对齐截断（14/20 高亮，旧两行居中作废）；
// 格式徽标左上 6,6（padding 2 4 / 徽标档 10/14 700 常态色）；原始文件徽章右下 8,8（15×15）。
// 状态边框 ::after 1.5px（同文件夹卡片机制）：hover 亮-选中 / 选中·重命名 品牌色-边框线
// （重命名 = 整卡高亮提示，输入框透明底不单独描边，2026-08-10 用户定）。
// 悬停控件（2026-08-10 用户定）：放大镜 / 默认应用打开 / 发送到按钮全删——放大预览改为
// 空格触发（访达 Quick Look 逻辑，AssetGrid 接线）；图片类无悬停控件；
// 视频 / 音频 / 动图 / 序列帧悬停播放保留（播放类无按钮，5.3 定案），
// 查看方式"自动播放动图（含序列帧）/ 自动播放视频"开关 = 网格内免悬停自动播放。
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppIcon from "./AppIcon.vue";
import HoverVideoLayer from "./HoverVideoLayer.vue";
import HoverAudioLayer from "./HoverAudioLayer.vue";
import HoverSequenceLayer from "./HoverSequenceLayer.vue";
import { CARD_CAPTION_H, type Box } from "../lib/layouts";
import { dragView } from "../lib/dragOperation";
import {
  displayName as assetDisplayName,
  formatBadgeColor,
  isAudioExt,
  isSequence,
  isVideoExt,
} from "../lib/formatBadge";
import { formatDuration } from "../lib/formatDuration";
import { renamePlacement } from "../lib/placementOps";
import { getAssetFilePath } from "../lib/mediaOps";
import { useInlineRename } from "../lib/useInlineRename";
import { useUiStore } from "../stores/ui";
import { useSelectionStore } from "../stores/selection";
import { useContentStore } from "../stores/content";
import type { PlacementView } from "../stores/content";

const props = withDefaults(
  defineProps<{
    asset: PlacementView;
    box: Box;
    selected?: boolean;
    renaming?: boolean;
    faded?: "source" | "cut" | null; // 拖拽源半透明 / 被剪切半透明（2026-07-28 复验反馈 6①/8）
  }>(),
  { selected: false, renaming: false, faded: null },
);
const ui = useUiStore();
const imgH = computed(() => Math.max(20, props.box.height - CARD_CAPTION_H));
const thumbUrl = computed(() => (props.asset.thumbPath ? convertFileSrc(props.asset.thumbPath) : null));
// 动图（5.1）：GIF / WEBP 动图；与序列帧区分（序列帧同样带 frameCount）
const isAnim = computed(() => props.asset.animated);
// 序列帧（2026-07-22 用户定）：格式角标显示"序列帧"，名称后缀显示为 .序列帧（仅显示层，数据不动）
const isSeq = computed(() => isSequence(props.asset));

// 悬停播放分发（序列帧归动图范围，2026-07-27 用户定：打包就是为了看动画效果）；
// 图片 / 工程 / 格式图标无悬停控件（2026-08-10 用户定：图片不用悬停）
type HoverKind = "anim" | "sequence" | "video" | "audio" | "other";
const kind = computed<HoverKind>(() => {
  const ext = props.asset.ext.toLowerCase();
  if (isAnim.value) return "anim";
  if (isSeq.value) return "sequence";
  if (isVideoExt(ext)) return "video";
  if (isAudioExt(ext)) return "audio";
  return "other";
});

// 悬停态：驱动播放层挂载（视频/音频）与动图播放
const hovered = ref(false);
// 拖拽期间全局禁用悬停层（2026-07-29 复验二轮 5，JS 闸）：拖拽激活时视同未悬停——
// 播放层 v-if 条件即刻转假，走既有退出路径（卸载停播 / 动图还原封面 / 序列帧停轮播）
const hoverActive = computed(() => hovered.value && !dragView.active);

// —— 就地重命名（6.1 C2，2026-07-28 定案）：标题 ↔ 输入框；
// Enter 确认 / Esc 取消 / 失焦确认；滚出虚拟滚动销毁 = 取消（AssetGrid 先清 renamingKey，
// blur 晚于清理到达时按 key 失配判取消）；确认调 rename_placement 后 content.refresh()（在 ops 内）
const selection = useSelectionStore();
const cardKey = `${props.asset.assetId}|${props.asset.folderId}`;
const { draft: renameDraft, bindInput, commit: commitRename, onEsc: cancelRename } = useInlineRename({
  active: () => selection.renamingKey === cardKey,
  initial: () => props.asset.name,
  selectStem: true,
  onCommit: async (name) => {
    selection.renamingKey = null;
    if (!name || name === props.asset.name) return;
    await renamePlacement(
      { assetId: props.asset.assetId, folderId: props.asset.folderId, origin: props.asset.origin },
      name,
    );
  },
  onCancel: () => {
    selection.renamingKey = null;
  },
});
// 双击名称 = 就地重命名（2026-07-29 复验三轮，renamingKey 既有机制——
// 双击第一下 click 已把卡片单选入域，此处仅置重命名目标）；
// 回收站视口只读不可重命名（2026-07-29 二轮，与 F2 / 菜单同口径闸）
function startRename(): void {
  if (useContentStore().isRecycleView) return;
  selection.renamingKey = cardKey;
}

// 动图播放（5.3）：悬停或"自动播放动图"开关（序列帧同归此开关，2026-07-27 用户定）——
// 300ms 延迟后把卡片图换成源文件（webview 原生循环，零等待），停止后还原封面
const animSrc = ref<string | null>(null);
let animTimer: ReturnType<typeof setTimeout> | undefined;
const animPlaying = computed(() => hoverActive.value || ui.autoPlayGif);
// immediate：虚拟滚动销毁重建 / 开关已开时挂载，animPlaying 初始即 true 无"变化"不触发——
// 挂载即评估一次（修复：自动播放滚出视口再回来 / 视口外滚入不播，2026-07-27 用户实测）
watch(
  animPlaying,
  (p) => {
    clearTimeout(animTimer);
    if (p && kind.value === "anim") {
      animTimer = setTimeout(() => void loadAnim(), 300);
    } else {
      animSrc.value = null;
    }
  },
  { immediate: true },
);
async function loadAnim(): Promise<void> {
  try {
    const r = await getAssetFilePath(props.asset.assetId);
    if (r.path && animPlaying.value) animSrc.value = convertFileSrc(r.path);
  } catch {
    // 解析失败保持静态封面
  }
}
onBeforeUnmount(() => {
  clearTimeout(animTimer);
});

// 角标文案：音频带 BPM 显示"格式 / BPM: 值"（Eagle 同格式，2026-07-26 用户截图定稿）
const badge = computed(() => {
  if (isSeq.value) return "序列帧";
  const ext = props.asset.ext.toUpperCase();
  return props.asset.bpm != null ? `${ext} / BPM: ${Math.round(props.asset.bpm)}` : ext;
});
// 角标徽章底色：格式实色（01_样式 token 01-6，7 组）
const badgeColor = computed(() => formatBadgeColor(props.asset.ext, isSeq.value, isAnim.value));
const displayName = computed(() => assetDisplayName(props.asset, ui.showExt));
// 标签行：音视频改时长属性（mm:ss，5.1）；动图与其他按尺寸；fla 等无尺寸资源显示格式角标文案
const meta = computed(() => {
  if (props.asset.duration != null && !isAnim.value) return formatDuration(props.asset.duration);
  return props.asset.w != null && props.asset.h != null ? `${props.asset.w} x ${props.asset.h}` : badge.value;
});
</script>

<template>
  <div
    class="asset-card"
    :class="{ selected, renaming, 'fade-source': faded === 'source', 'fade-cut': faded === 'cut' }"
    :style="{
      left: `${box.left}px`,
      top: `${box.top}px`,
      width: `${box.width}px`,
      height: `${box.height}px`,
    }"
  >
    <div
      class="thumb"
      :style="{ height: `${imgH}px` }"
      @mouseenter="hovered = true"
      @mouseleave="hovered = false"
    >
      <img
        v-if="thumbUrl"
        class="img"
        :class="{ icon: asset.thumbKind === 'formatIcon', contain: ui.layoutMode === 'grid' }"
        :src="animSrc ?? thumbUrl"
        :alt="asset.name"
        decoding="async"
        draggable="false"
      />
      <div v-else class="img" />
      <HoverVideoLayer
        v-if="(hoverActive || ui.autoPlayVideo) && kind === 'video'"
        :asset="asset"
        :auto="!hoverActive"
      />
      <HoverAudioLayer v-else-if="hoverActive && kind === 'audio'" :asset="asset" />
      <HoverSequenceLayer v-else-if="(hoverActive || ui.autoPlayGif) && kind === 'sequence'" :asset="asset" />
      <span v-if="badgeColor && ui.showBadge" class="badge" :style="{ background: badgeColor }">{{ badge }}</span>
      <span v-if="asset.noteCount" class="note-badge">{{ asset.noteCount }}</span>
    </div>
    <!-- 简介区（figma 14）：C3 + 上边框 / padding 8 8 8 12 / 间距 4；
         标题单行左对齐（双击就地重命名，stop 防触发卡片双击预览）；
         meta 尺寸行仍是 card-info 框选空白（按下拖动起框选 / 单击清空选择） -->
    <div class="texts">
      <input
        v-if="renaming"
        :ref="bindInput"
        v-model="renameDraft"
        class="rename-input"
        @keydown.enter.prevent="commitRename"
        @keydown.esc.prevent="cancelRename"
        @blur="commitRename"
        @click.stop
        @dblclick.stop
        @pointerdown.stop
      />
      <!-- 名称悬停 tip（2026-08-10 用户定）：完整名称（限宽换行）+ 当前简介档（尺寸/文件大小等，
           简介关闭或 briefMode none 时无简介行）；序列帧显示名（.序列帧 后缀）与卡片一致 -->
      <span
        v-else
        class="title"
        v-tip="{ text: displayName, keys: ui.showBrief && ui.briefMode !== 'none' ? meta : undefined, column: true }"
        @dblclick.stop="startRename"
      >{{ displayName }}</span>
      <span v-show="ui.showBrief && ui.briefMode !== 'none'" class="meta card-info">{{ meta }}</span>
    </div>
    <!-- 原始文件徽章（figma 14：15×15 品牌色图标，卡片右下各 8） -->
    <AppIcon v-if="asset.origin" name="original-file-badge" :size="15" class="origin-badge" />
  </div>
</template>

<style scoped>
/* 卡片（figma 14）：圆角 6 / padding 0 / 无间距（缩略图与简介无缝拼接）。
   边框全部走 ::after 伪元素（最上层——缩略图 C2 / 简介 C3 整片不透明底会盖住父级
   inset 阴影，2026-08-10 走查：外边框全消失的根因）：常态 1px 暗-悬停；
   hover 1.5px 亮-选中；选中 / 重命名 1.5px 品牌色-边框线（重命名 = 整卡高亮提示）。
   伪元素 border 宽变 1→1.5 向内画不占布局（同文件夹卡片机制） */
.asset-card {
  position: absolute;
  display: flex;
  flex-direction: column;
  border-radius: var(--radius-6);
  cursor: default;
}
.asset-card::after {
  content: "";
  position: absolute;
  inset: 0;
  border: 1px solid var(--line-hover);
  border-radius: var(--radius-6);
  pointer-events: none;
  transition: border-color 0.12s ease;
}
.asset-card:hover::after {
  border: 1.5px solid var(--line-card-hover); /* 实色 #414743（2026-08-10 用户黑底吸色定）：
    复杂缩略图（纯黑区域）上半透明线显淡、边框断开感；实色不透明在任何内容上连贯 */
}
.asset-card.selected::after,
.asset-card.renaming::after {
  border: 1.5px solid var(--brand-border);
}
/* 拖拽源 / 被剪切整体半透明（2026-07-28 复验反馈 6①/8，token 透明度档） */
.asset-card.fade-source {
  opacity: var(--opacity-drag-source);
}
.asset-card.fade-cut {
  opacity: var(--opacity-cut);
}
/* 缩略图区（figma 14）：C2 底（2026-08-10 用户纠正：缩略图 C2、简介区才是 C3）/ 圆角上沿 */
.thumb {
  position: relative;
  flex: none;
  display: flex; /* 格式图标居中缩到 50%（2026-07-25 用户定）；内容缩略图仍 width/height 100% */
  align-items: center;
  justify-content: center;
  background: var(--c2);
  border-radius: var(--radius-6) var(--radius-6) 0 0;
  overflow: hidden;
}
/* 网格模式固定格：contain 完整显示不裁切（cover 必裁，2026-07-24 用户定） */
.img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.img.contain {
  object-fit: contain;
}
/* shell 格式图标：最大边 = 容器 50%（左右各留 1/4，Eagle 口径——小一点，
   不与真正的内容缩略图抢视觉噪音，2026-07-25 用户定） */
.img.icon {
  width: 50%;
  height: 50%;
  object-fit: contain;
}
/* 格式徽标（figma 14）：左上各 6 / padding 2 4 / 圆角 4 / 徽标档 10/14 700 常态色 */
.badge {
  position: absolute;
  top: var(--space-6);
  left: var(--space-6);
  padding: var(--space-2) var(--space-4);
  border-radius: var(--radius-4);
  font-size: var(--text-badge-size);
  line-height: var(--text-badge-line);
  font-weight: var(--text-badge-weight);
  color: var(--text-normal);
  pointer-events: none;
}
/* 笔记角标（5.2，Eagle 式数字圆标）：图片右上各 4px / 直径 16 / 品牌紫实底 /
   数字 font-icon-1（角标文字 10/16 粗体；5.4 红线修正：#fff 硬编码改 token，2026-07-28） */
.note-badge {
  position: absolute;
  top: var(--space-4);
  right: var(--space-4);
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--brand-1);
  font-size: var(--text-badge-size);
  line-height: 16px;
  font-weight: var(--text-badge-weight);
  color: var(--font-icon-1);
  text-align: center;
  pointer-events: none;
}
/* 简介区（figma 14）：C3 / 上边框 暗-悬停（inset 模拟分割线不占位，简介区恒 57 高含线）/
   圆角下沿 / padding 8 8 8 12 / 间距 4 */
.texts {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding: var(--space-8) var(--space-8) var(--space-8) var(--space-12);
  background: var(--c3);
  box-shadow: inset 0 1px 0 0 var(--line-hover);
  border-radius: 0 0 var(--radius-6) var(--radius-6);
  flex: none;
}
/* 名称：正文 14/20 高亮，单行左对齐截断（2026-08-10 用户定，旧两行居中作废） */
.title {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-highlight);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  /* 名称区悬停文本光标（2026-07-29 复验三轮：退出 card-info，双击可改名） */
  cursor: text;
}
.meta {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--text-label);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 重命名输入：透明底不单独描边（整卡 1.5px 品牌色高亮提示，同文件夹卡片 2026-08-10 用户定）；
   共享基座在 base.css（高度恒 20 = 字体行高不撑高简介区），本地差异 = 满宽 + 高亮字色 */
.rename-input {
  width: 100%;
  color: var(--text-highlight);
}
/* 原始文件徽章（figma 14）：15×15，卡片右下各 8（简介区右下） */
.origin-badge {
  position: absolute;
  right: var(--space-8);
  bottom: var(--space-8);
  pointer-events: none;
}
</style>
