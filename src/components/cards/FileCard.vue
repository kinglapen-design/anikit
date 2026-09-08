<script setup lang="ts">
// 文件卡片——docs/06_UI重构/Design Token/组件/卡片/ 文件卡片-通用 / 视频 / 音频 / 字体 / txt-md 五稿合一
// （同 ActionButton 五稿合一先例）：
// kind：general = 缩略图充满（cover，沿用旧网格口径；square = shell 缩略图 / 自动导入未导入 →
//        正方形卡 + 缩略图占 50% 居中，稿注明确）；
//       video  = 缩略图 + 悬停胶囊（帧数 + 播放/暂停 + 音量/静音）+ 底部 4px 播放条（轨道 亮色/半透明-2、
//        填充 亮色/一级，贴底 4、左右 12）；
//       audio  = 波形（竖线 4px 圆头 gap 4 通用/五级，根数恒定由业务保证、美观优先）+ 悬停胶囊
//        （时长 + 播放/暂停）；进度 = 品牌色波形层 clip-path 丝滑扫过（与根数解耦，稿 prose + 2026-08-29 用户定色）；
//       font   = 示例字块居中（标题档 亮色/四级，日/韩环境示例字业务侧替换）+ 右上 FontInstallButton；
//       text   = 标题（标题档 二级）+ 正文（正文档 四级）滚动预览，内容溢出出 6px 滚动胶囊
//        （通用/三级，胶囊自身 hover 升五级——稿「hover-滚动条」节）；
// 悬停控件（胶囊 / 播放条 / 波形扫过）一律 hover 才出现（2026-08-29 用户定），过渡 150ms；
// 壳 / hover 位移 -4 + 小阴影 / 选中 2px 品牌描边 见 cards.css（kc-media / kc-sel）。
// 业务态（7.1 内容容器轮接线，模式同 cards/FolderCard）：naming = 行内重命名（名称位换输入框；
// 文件口径 Esc = 取消——nameCancel 上抛，TreeRowItem/文件夹卡的 escSaves 不同）；picked = 选择模式
// 已选压暗 30%（大勾在壳层）；thumbFit = 缩略图适配三档（cover 默认 / contain 网格固定格完整显示
// ——旧 AssetCard 口径 / icon 格式图标 50% 居中——Eagle 口径，不与内容缩略图抢视觉噪音）；
// 默认插槽 = 内容区扩展（壳层注入悬停播放层等，叠在缩略图上、渐变罩下）。
import { computed, nextTick, onMounted, reactive, ref, watch } from "vue";
import AppIcon from "../AppIcon.vue";
import FontInstallButton from "../buttons/FontInstallButton.vue";
import FormatBadge from "./FormatBadge.vue";
import { useInlineRename } from "../../lib/useInlineRename";
import "./cards.css";

const props = withDefaults(
  defineProps<{
    kind?: "general" | "video" | "audio" | "font" | "text";
    name: string;
    info?: string; // 简介行（尺寸 / 时长 / 字重…，内容业务侧定）
    badge?: string; // 格式角标文案（不传不出）
    thumbUrl?: string; // general / video 缩略图
    thumbFit?: "cover" | "contain" | "icon"; // 缩略图适配（icon = 格式图标 50% 居中）
    square?: boolean; // general：shell 缩略图 / 未导入卡 = 正方形 + 50% 居中
    capsuleText?: string; // 悬停胶囊文本（video = 帧数 / audio = 时长）
    playing?: boolean; // 胶囊 播放 ↔ 暂停
    muted?: boolean; // video 胶囊 音量 ↔ 静音
    progress?: number; // 0..1：video 播放条 / audio 波形扫过
    peaks?: number[]; // audio 波形（0..1 归一化）
    fontLines?: string[]; // font 示例字
    installed?: boolean; // font 安装钮选中
    textTitle?: string; // text 卡内标题
    textBody?: string; // text 卡正文
    originBadge?: boolean; // 原始文件-徽章（右下 12/12）
    selected?: boolean;
    naming?: boolean; // 行内重命名输入态
    picked?: boolean; // 选择模式已选（内容压暗 30%，大勾在壳层）
    forceState?: "" | "hover" | "selected";
  }>(),
  {
    kind: "general",
    info: "",
    badge: "",
    thumbUrl: "",
    thumbFit: "cover",
    square: false,
    capsuleText: "",
    playing: false,
    muted: false,
    progress: 0,
    peaks: () => [],
    fontLines: () => ["永和九年", "岁在癸丑", "AaBaCcDd", "01234567"],
    installed: false,
    textTitle: "",
    textBody: "",
    originBadge: false,
    selected: false,
    naming: false,
    picked: false,
    forceState: "",
  },
);

const emit = defineEmits<{
  nameCommit: [name: string]; // Enter / 失焦（父级判空/不变/落命令）
  nameCancel: []; // Esc（文件重命名口径 = 取消，区别于文件夹 escSaves）
  nameEdit: []; // 名称双击 = 进命名态（父级置 naming）
}>();

// 行内重命名（useInlineRename 封装，速查 §4）：selectStem = 词干选区不包后缀（文件口径）；
// Esc = 取消上抛（onEsc），Enter / 失焦 = 提交上抛（空/不变判定在父级壳层）
const { draft: nameDraft, bindInput, commit: commitName, onEsc: cancelName } = useInlineRename({
  active: () => props.naming,
  initial: () => props.name,
  selectStem: true,
  onCommit: (name) => emit("nameCommit", name),
  onCancel: () => emit("nameCancel"),
});

const hasCapsule = computed(() => props.kind === "video" || props.kind === "audio");
const hasProgress = computed(() => props.progress > 0);

// 波形竖线高度：peaks 归一化 → 百分比（最低 8% ≈ 稿最小 13px / 容器 176）
function barH(p: number): string {
  return `${Math.min(100, Math.max(8, p * 100))}%`;
}

// text 卡滚动胶囊：内容溢出才显，轨道 = 内容区（卡 padding 12 内），跟随滚动
const textEl = ref<HTMLElement>();
const thumb = reactive({ show: false, top: 12, height: 0 });
function updateThumb(): void {
  const el = textEl.value;
  if (!el) return;
  const { scrollHeight, clientHeight, scrollTop } = el;
  if (scrollHeight <= clientHeight || clientHeight === 0) {
    thumb.show = false;
    return;
  }
  const h = Math.max(24, (clientHeight / scrollHeight) * clientHeight);
  thumb.height = h;
  thumb.top = 12 + (scrollTop / (scrollHeight - clientHeight)) * (clientHeight - h);
  thumb.show = true;
}
onMounted(updateThumb);
watch(
  () => [props.textTitle, props.textBody],
  () => nextTick(updateThumb),
);
</script>

<template>
  <div
    class="kc-media kc-bordered kc-sel fc-root"
    :class="[kind, { square, selected, playing, naming, picked }, forceState ? `force-${forceState}` : '']"
  >
    <!-- 内容区（圆角裁剪层；选中环在层外防 overflow 切边） -->
    <div class="kc-clip">
      <img
        v-if="(kind === 'general' || kind === 'video') && thumbUrl"
        :src="thumbUrl"
        class="fc-thumb"
        :class="square ? 'shell' : thumbFit"
        alt=""
        decoding="async"
        draggable="false"
      />

      <div v-else-if="kind === 'audio'" class="fc-wave">
        <div class="fc-wave-row">
          <span v-for="(p, i) in peaks" :key="i" class="fc-wave-bar" :style="{ height: barH(p) }" />
        </div>
        <div
          v-if="hasProgress"
          class="fc-wave-row played"
          :style="{ clipPath: `inset(0 ${(1 - progress) * 100}% 0 0)` }"
        >
          <span v-for="(p, i) in peaks" :key="i" class="fc-wave-bar" :style="{ height: barH(p) }" />
        </div>
      </div>

      <div v-else-if="kind === 'font'" class="fc-font">
        <div class="fc-font-lines">
          <span v-for="l in fontLines" :key="l" class="fc-font-line">{{ l }}</span>
        </div>
      </div>

      <div v-else-if="kind === 'text'" ref="textEl" class="fc-text" @scroll="updateThumb">
        <div v-if="textTitle" class="fc-text-title">{{ textTitle }}</div>
        <div class="fc-text-body">{{ textBody }}</div>
      </div>

      <!-- 内容区扩展槽（壳层悬停播放层等注入）：叠在缩略图上、渐变罩下 -->
      <slot />

      <!-- 名称+简介渐变罩（pointer-events 穿透：罩区不再吃 pointermove，scrub 整卡跟手；
           仅名称文本行 / 命名输入恢复交互——v-tip、双击重命名保留，见 scoped 样式注） -->
      <div class="kc-overlay">
        <!-- 命名态：input 替换名称位（整卡品牌环提示，输入框透明底，速查 §4 统一口径）；
             命名中 click / dblclick / pointerdown 自停冒泡（防触发选中 / 预览 / 起拖拽） -->
        <input
          v-if="naming"
          :ref="bindInput"
          v-model="nameDraft"
          class="rename-input"
          @keydown.enter.prevent="commitName"
          @keydown.esc.prevent="cancelName"
          @blur="commitName"
          @click.stop
          @dblclick.stop
          @pointerdown.stop
        />
        <span v-else class="kc-name" v-tip="{ text: name, keys: info || undefined, column: true }" @dblclick.stop="emit('nameEdit')">{{ name }}</span>
        <span v-if="info" class="kc-info">{{ info }}</span>
      </div>
    </div>

    <!-- 格式角标（左上 12/12） -->
    <FormatBadge v-if="badge" class="fc-badge" :label="badge" />

    <!-- 悬停胶囊（video / audio，hover 才现） -->
    <div v-if="hasCapsule" class="kc-float fc-capsule">
      <span v-if="capsuleText" class="kc-info fc-capsule-text">{{ capsuleText }}</span>
      <AppIcon
        :name="playing ? 'card-hover-pause' : 'card-hover-play'"
        :size="14"
        class="fc-capsule-icon"
      />
      <AppIcon
        v-if="kind === 'video'"
        :name="muted ? 'card-hover-mute' : 'card-hover-volume'"
        :size="14"
        class="fc-capsule-icon"
      />
    </div>

    <!-- 视频播放条（hover 才现） -->
    <div v-if="kind === 'video'" class="fc-vbar">
      <span class="fc-vbar-fill" :style="{ width: `${progress * 100}%` }" />
    </div>

    <!-- 字体安装钮（右上 12/12，恒显） -->
    <FontInstallButton v-if="kind === 'font'" class="fc-install" :selected="installed" />

    <!-- text 卡滚动胶囊（贴右缘，hover 升五级） -->
    <span
      v-if="kind === 'text'"
      v-show="thumb.show"
      class="fc-text-thumb"
      :style="{ top: `${thumb.top}px`, height: `${thumb.height}px` }"
    />

    <!-- 原始文件-徽章（右下 12/12，品牌色原值） -->
    <AppIcon v-if="originBadge" name="original-file-badge" :size="14" class="kc-corner-badge" />
  </div>
</template>

<style scoped>
.fc-root {
  width: 100%;
  height: 100%;
}
/* square 变体：shell 缩略图 / 自动导入未导入 = 正方形比例（稿注） */
.fc-root.square {
  height: auto;
  aspect-ratio: 1 / 1;
}
/* 圆角裁剪层 .kc-clip 见 cards.css（与档案卡共享） */

/* 渐变罩 pointer-events 穿透（2026-09-08 用户实机报：悬停视频 scrub 划过罩区丢失跟随——
   罩贴底 ≈74px 高、冒泡不经播放层，pointermove 被吃）；名称文本行 / 命名输入恢复 auto
   保 v-tip / 双击重命名 / 输入交互（"底部名称行不 scrub"旧口径精确化到文本行；
   info 行无交互一并穿透，也可 scrub） */
.kc-overlay {
  pointer-events: none;
}
.kc-name,
.rename-input {
  pointer-events: auto;
}

/* 缩略图：cover 充满（旧网格口径）；contain = 网格固定格完整显示不裁切（2026-07-24 用户定）；
   icon / shell = 50% 居中 contain（格式图标 Eagle 口径 / 稿 square 注）。
   铺满档自带圆角 8：图片走图片绘制路径画角（比容器蒙版裁剪平滑），与描边弧同心同半径——
   2026-08-30 用户实机亮图四角锯齿（.kc-clip 蒙版弧 + 描边弧双线错位），图片自圆角后裁剪弧合一 */
.fc-thumb.cover,
.fc-thumb.contain {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  border-radius: var(--n-radius-8);
}
.fc-thumb.cover {
  object-fit: cover;
}
.fc-thumb.contain {
  object-fit: contain;
}
.fc-thumb.icon,
.fc-thumb.shell {
  position: absolute;
  left: 25%;
  top: 25%;
  width: 50%;
  height: 50%;
  object-fit: contain;
}

/* 音频波形：竖线 4px 圆头 gap 4 通用/五级，垂直居中；扫过层 = 品牌色同构层 clip-path 按进度裁。
   扫过显隐同悬停控件口径（hover 才现，2026-08-29 用户定），播放中恒显 */
.fc-wave {
  position: absolute;
  inset: var(--n-space-12);
  display: flex;
  align-items: center;
}
.fc-wave-row {
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: center;
  gap: var(--n-space-4);
  width: 100%;
  height: 100%;
}
.fc-wave-row.played {
  position: absolute;
  inset: 0;
  opacity: 0;
  transition: opacity var(--n-dur-150) var(--n-ease-in-out);
}
.fc-root:hover .fc-wave-row.played,
.fc-root.force-hover .fc-wave-row.played,
.fc-root.playing .fc-wave-row.played {
  opacity: 1;
}
.fc-wave-bar {
  flex: none;
  width: 4px;
  border-radius: 99px; /* 稿值 99 = 胶囊圆头（Token.md 圆角无此档，非新造 token） */
  background: var(--n-g5);
}
.fc-wave-row.played .fc-wave-bar {
  background: var(--n-brand);
}

/* 字体示例字：块居中、行左对齐，标题档 亮色/四级 */
.fc-font {
  position: absolute;
  inset: var(--n-space-12);
  display: flex;
  align-items: center;
  justify-content: center;
}
.fc-font-lines {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--n-space-4);
}
.fc-font-line {
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l4);
  white-space: nowrap;
}

/* 文本预览：卡 padding 12 内滚动（滚动内容可入渐变罩下），标题 二级 / 正文 四级 gap 8 */
.fc-text {
  position: absolute;
  inset: var(--n-space-12);
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
  scrollbar-width: none;
}
.fc-text::-webkit-scrollbar {
  display: none;
}
.fc-text-title {
  flex: none;
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
.fc-text-body {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l4);
  white-space: pre-wrap;
}
/* 滚动胶囊：6px 通用/三级贴右缘，自身 hover 升五级（稿「hover-滚动条」节） */
.fc-text-thumb {
  position: absolute;
  right: 0;
  z-index: 3;
  width: 6px;
  border-radius: 99px; /* 稿值 99 */
  background: var(--n-g3);
  transition: background var(--n-dur-150) var(--n-ease-in-out);
}
.fc-text-thumb:hover {
  background: var(--n-g5);
}

/* 格式角标定位（左上 12/12） */
.fc-badge {
  position: absolute;
  left: var(--n-space-12);
  top: var(--n-space-12);
  z-index: 3;
}

/* 悬停胶囊（右上 12/12，gap 6）：常态隐，hover 现（150ms） */
.fc-capsule {
  position: absolute;
  right: var(--n-space-12);
  top: var(--n-space-12);
  z-index: 4;
  gap: var(--n-space-6);
  opacity: 0;
  pointer-events: none;
  transition: opacity var(--n-dur-150) var(--n-ease-in-out);
}
.fc-root:hover .fc-capsule,
.fc-root.force-hover .fc-capsule {
  opacity: 1;
  pointer-events: auto;
}
.fc-capsule-text {
  overflow: visible;
}
/* 胶囊图标：常态 亮色/二级，自身 hover 提亮 亮色/一级（稿「hover-播放/音量按钮」节） */
.fc-capsule-icon {
  color: var(--n-l2);
  cursor: pointer;
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.fc-capsule-icon:hover {
  color: var(--n-l1);
}

/* 视频播放条：4px 高、左右 12、贴底 4；轨道 亮色/半透明-2、填充 亮色/一级；hover 才现 */
.fc-vbar {
  position: absolute;
  left: var(--n-space-12);
  right: var(--n-space-12);
  bottom: var(--n-space-4);
  z-index: 5;
  height: 4px;
  border-radius: 99px; /* 稿值 99 */
  background: var(--n-l-a2);
  opacity: 0;
  transition: opacity var(--n-dur-150) var(--n-ease-in-out);
}
.fc-root:hover .fc-vbar,
.fc-root.force-hover .fc-vbar {
  opacity: 1;
}
.fc-vbar-fill {
  display: block;
  height: 100%;
  border-radius: 99px; /* 稿值 99 */
  background: var(--n-l1);
}

/* 字体安装钮定位（右上 12/12） */
.fc-install {
  position: absolute;
  right: var(--n-space-12);
  top: var(--n-space-12);
  z-index: 4;
}

/* 命名输入：透明底不单独描边（整卡品牌环提示）；覆盖共享基座 = 正文-加粗 13/18 与名称视觉连续 */
.rename-input {
  width: 100%;
  min-width: 0;
  height: 18px;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
  text-align: left;
}
/* 命名态整卡品牌描边（复用 .kc-sel 选中环层，opacity 恒亮）；常态环让位透明（cards.css 选中同款） */
.fc-root.naming::before {
  opacity: 1;
}
.fc-root.naming::after {
  border-color: transparent;
}
/* 选择模式已选（§12）：内容压暗 30% 档（大勾在壳层不随压） */
.fc-root.picked .kc-clip,
.fc-root.picked .fc-badge {
  opacity: 0.3;
}
</style>
