<script setup lang="ts">
// 素材卡片（figma-css 01_组件/14_卡片，2026-08-10 重做定稿）：
// 整卡 1px 暗-悬停边框 / 圆角 6 / padding 0；缩略图区 C3 圆角上沿，简介区 C3 + 上边框圆角下沿
// （padding 8 8 8 12 / 间距 4）；名称单行左对齐截断（14/20 高亮，旧两行居中作废）；
// 格式徽标左上 6,6（padding 2 4 / 徽标档 10/14 700 常态色）；原始文件徽章右下 8,8（15×15）。
// 显示简介（2026-08-19 接入）：关 = 隐藏简介行（.meta）且布局层简介区高 57→36 缩短卡片
// （useGridLayout 同步重排）；徽章为整卡 absolute 定位不随简介隐藏，自动落名称行右侧；
// 简介内容按 ui.briefMode 分流（尺寸/文件大小/创建日期/修改日期，标签档待后端数据通道）。
// 状态边框 ::after 1.5px（同文件夹卡片机制）：hover 亮-选中 / 选中·重命名 品牌色-边框线
// （重命名 = 整卡高亮提示，输入框透明底不单独描边，2026-08-10 用户定）。
// 悬停控件（2026-08-10 用户定）：放大镜 / 默认应用打开 / 发送到按钮全删——放大预览改为
// 空格触发（访达 Quick Look 逻辑，AssetGrid 接线）；图片类无悬停控件；
// 视频 / 音频 / 动图 / 序列帧悬停播放保留（播放类无按钮，5.3 定案），
// 查看方式"自动播放动图（含序列帧）/ 自动播放视频"开关 = 网格内免悬停自动播放。
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppIcon from "./AppIcon.vue";
import HoverVideoLayer from "./HoverVideoLayer.vue";
import HoverAudioLayer from "./HoverAudioLayer.vue";
import HoverSequenceLayer from "./HoverSequenceLayer.vue";
import AnimCanvasLayer from "./AnimCanvasLayer.vue";
import { CARD_CAPTION_H, CARD_CAPTION_COMPACT_H, type Box } from "../lib/layouts";
import { dragView } from "../lib/dragOperation";
import {
  displayName as assetDisplayName,
  formatBadgeColor,
  isAudioExt,
  isSequence,
  isVideoExt,
} from "../lib/formatBadge";
import { formatDuration } from "../lib/formatDuration";
import { formatSize } from "../lib/formatSize";
import { formatDetailDate } from "../lib/detailSections";
import { renamePlacement } from "../lib/placementOps";
import { acquireAutoplay, releaseAutoplay, type AutoplayPool } from "../lib/autoplayGate";
import { useInlineRename } from "../lib/useInlineRename";
import { useUiStore } from "../stores/ui";
import { usePickerStore } from "../stores/picker";
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
// 简介区高度跟随「显示简介」开关（2026-08-19 接入：关 = 紧凑 36，隐藏简介行并缩短卡片）
const captionH = computed(() => (ui.showBrief ? CARD_CAPTION_H : CARD_CAPTION_COMPACT_H));
const imgH = computed(() => Math.max(20, props.box.height - captionH.value));
const thumbUrl = computed(() =>
  // ?v=modifiedAt = 缩略图 URL 版本戳：设封面后 modifiedAt 变 → URL 变 → img 重新加载
  //（asset 协议同 URL 有启发式缓存，2026-08-20 用户实机"静帧不热刷新"修复；query 不影响协议路径解析）
  props.asset.thumbPath ? `${convertFileSrc(props.asset.thumbPath)}?v=${encodeURIComponent(props.asset.modifiedAt ?? "")}` : null,
);
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
const picker = usePickerStore();
const cardKey = `${props.asset.assetId}|${props.asset.folderId}`;
// 选择模式（6.2 一轮，PRD 03_数据链接 04 四重标识之四）：已选卡片压暗（30% 档）+ 品牌绿大勾
const picked = computed(() => picker.active && picker.isPicked("asset", props.asset.assetId));
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

// 动图播放（5.3）：悬停或"自动播放动图"开关（序列帧同归此开关，2026-07-27 用户定）；
// 阶段四起由 AnimCanvasLayer 帧流渲染（进入延迟分档收编 useHoverPlayer：悬停 300 / 自动播放 100）
// 自动播放数量闸（2.3）：开关驱动的自动播放申请名额（满员排队 FIFO 唤醒），悬停播放直通——
// 小卡片高密度时窗口内 200+ 卡片同时解码致全局卡顿（hover 目录树滞后）；
// 动图 / 视频 / 序列帧三处自动播放共用本卡 gateRequested/autoGranted（一卡同时至多一种 kind）。
// 可见性联动（2026-08-12 用户实机：纯 FIFO 名额被缓冲区不可见卡占住、头尾可见卡排队等死）——
// IntersectionObserver 判定真实可见，仅可见才申请、滚出视口立即释放（视口外暂停的落地）
const autoGranted = ref(false);
let gateRequested = false;
const cardVisible = ref(false);
const rootEl = ref<HTMLElement>();
let visibilityObs: IntersectionObserver | undefined;
const gatePool = computed<AutoplayPool>(() => (kind.value === "video" ? "video" : "anim"));
const wantAutoplay = computed(() => {
  if (!cardVisible.value) return false;
  if (kind.value === "anim" || kind.value === "sequence") return ui.autoPlayGif;
  if (kind.value === "video") return ui.autoPlayVideo;
  return false;
});
watch(
  wantAutoplay,
  (want) => {
    if (want && !gateRequested) {
      gateRequested = true;
      autoGranted.value = acquireAutoplay(cardKey, gatePool.value, () => {
        autoGranted.value = true;
      });
    } else if (!want && gateRequested) {
      gateRequested = false;
      releaseAutoplay(cardKey, gatePool.value);
      autoGranted.value = false;
    }
  },
  { immediate: true },
);
const animPlaying = computed(() => hoverActive.value || autoGranted.value);
// 动图播放（阶段四）：animPlaying 且 kind==='anim' 时模板挂载 AnimCanvasLayer——
// WS 帧流 canvas 渲染（自研解码器，替代 2.3 小动画缓存 img 换源）；序列帧归 HoverSequenceLayer。
onMounted(() => {
  // root null = webview 视口：网格 BUFFER 缓冲区内但视口外的卡判不可见不占名额
  visibilityObs = new IntersectionObserver((entries) => {
    cardVisible.value = entries[0]?.isIntersecting ?? false;
  });
  if (rootEl.value) visibilityObs.observe(rootEl.value);
});
onBeforeUnmount(() => {
  visibilityObs?.disconnect();
  if (gateRequested) {
    gateRequested = false;
    releaseAutoplay(cardKey, gatePool.value);
  }
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
// 简介行内容按查看方式「显示简介」下拉分流（2026-08-19 接入；标签档占位禁用，待后端数据通道）：
// 尺寸 = 音视频时长（mm:ss，5.1）/ 动图与其他按 w×h；
// 空值兜底（2026-08-19 用户定）：档内无数据一律回退文件大小（恒有）——存量创建日期 null、
// 尺寸档无尺寸资源（fla 等；旧"回退格式角标文案"废除：角标/名称/简介三处重复同一格式词，信息浪费）
const meta = computed(() => {
  if (ui.briefMode === "fileSize") return formatSize(props.asset.size);
  if (ui.briefMode === "createdAt")
    return props.asset.createdAt ? formatDetailDate(props.asset.createdAt) : formatSize(props.asset.size);
  if (ui.briefMode === "modifiedAt") return formatDetailDate(props.asset.modifiedAt);
  if (props.asset.duration != null && !isAnim.value) return formatDuration(props.asset.duration);
  return props.asset.w != null && props.asset.h != null
    ? `${props.asset.w} x ${props.asset.h}`
    : formatSize(props.asset.size);
});
</script>

<template>
  <div
    ref="rootEl"
    class="asset-card"
    :class="{ selected, renaming, picked, 'fade-source': faded === 'source', 'fade-cut': faded === 'cut' }"
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
        :src="thumbUrl"
        :alt="asset.name"
        decoding="async"
        draggable="false"
      />
      <div v-else class="img" />
      <HoverVideoLayer
        v-if="animPlaying && kind === 'video'"
        :asset="asset"
        :auto="!hoverActive"
      />
      <AnimCanvasLayer v-else-if="animPlaying && kind === 'anim'" :asset="asset" :auto="!hoverActive" />
      <HoverAudioLayer v-else-if="hoverActive && kind === 'audio'" :asset="asset" />
      <HoverSequenceLayer v-else-if="animPlaying && kind === 'sequence'" :asset="asset" :auto="!hoverActive" />
      <span v-if="badgeColor && ui.showBadge" class="badge" :style="{ background: badgeColor }">{{ badge }}</span>
      <span v-if="asset.noteCount" class="note-badge">{{ asset.noteCount }}</span>
    </div>
    <!-- 简介区（figma 14）：C3 + 上边框 / padding 8 8 8 12 / 间距 4；
         标题单行左对齐（双击就地重命名，stop 防触发卡片双击预览）；
         整卡（含 meta 行）点击 = 选中、按下拖动 = 起拖拽（2026-08-18 用户定，card-info 空白口径退役） -->
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
      <!-- 名称悬停 tip（2026-08-10 用户定）：完整名称（限宽换行）+ 当前简介档内容（尺寸/文件大小
           等，简介开关关闭时无简介行）；序列帧显示名（.序列帧 后缀）与卡片一致 -->
      <span
        v-else
        class="title"
        v-tip="{ text: displayName, keys: ui.showBrief ? meta : undefined, column: true }"
        @dblclick.stop="startRename"
      >{{ displayName }}</span>
      <span v-show="ui.showBrief" class="meta">{{ meta }}</span>
    </div>
    <!-- 原始文件徽章（figma 14：15×15 品牌色图标，卡片右下各 8） -->
    <AppIcon v-if="asset.origin" name="original-file-badge" :size="15" class="origin-badge" />
    <!-- 选择模式已选大勾（6.2 一轮）：绿圆白勾居中于缩略图区；不随压暗（在根层级、opacity 作用域外） -->
    <span v-if="picked" class="pick-check" :style="{ top: `${imgH / 2}px` }">
      <AppIcon name="check" :size="22" />
    </span>
  </div>
</template>

<style scoped>
/* 卡片（figma 14）：圆角 6 / padding 0 / 无间距（缩略图与简介无缝拼接）。
   边框双伪元素分层（2026-08-19 残留修复——踩坑 036）：::after = 常态 1px 暗-悬停（无过渡）；
   ::before = 状态层 1.5px（hover 亮-选中 / 选中·重命名 品牌色-边框线），**opacity 过渡**——
   border-color / box-shadow 的颜色过渡是重绘型动画，圆角 + 1.5px 分数像素下 GPU 合成损伤区
   偶发漏覆盖 → 取消状态后品牌色像素残留（用户实机多次复现）；opacity 走合成器无光栅损伤。
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
}
/* 状态层：常态 opacity 0 不占渲染；hover / 选中换色瞬时 + 透明度淡入淡出 */
.asset-card::before {
  content: "";
  position: absolute;
  inset: 0;
  z-index: 1; /* 盖常态层与缩略图 / 简介底（::after 同层叠序上的最上） */
  border: var(--border-strong) solid transparent;
  border-radius: var(--radius-6);
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.12s ease;
}
.asset-card:hover::before {
  opacity: 1;
  border-color: var(--line-card-hover); /* 实色 #414743（2026-08-10 用户黑底吸色定）：
    复杂缩略图（纯黑区域）上半透明线显淡、边框断开感；实色不透明在任何内容上连贯 */
}
.asset-card.selected::before,
.asset-card.renaming::before {
  opacity: 1;
  border-color: var(--brand-border);
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
/* 选择模式已选（6.2 一轮）：缩略图区 / 简介区 / 徽章压暗 30% 档（速查表 §5），大勾不压 */
.asset-card.picked .thumb,
.asset-card.picked .texts,
.asset-card.picked .origin-badge {
  opacity: 0.3;
}
/* 已选大勾（2026-08-23 用户贴稿：绿圆白勾居中于缩略图区）：成功色圆底 44 + check 22 白 */
.pick-check {
  position: absolute;
  left: 50%;
  transform: translate(-50%, -50%);
  z-index: 3;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  border-radius: 50%;
  background: var(--success);
  color: var(--text-highlight);
  pointer-events: none;
}
</style>
