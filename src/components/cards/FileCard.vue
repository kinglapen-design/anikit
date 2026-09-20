<script setup lang="ts">
// 文件卡片——docs/06_UI重构/Design Token/组件/卡片/ 文件卡片-通用 / 视频 / 音频 / 字体 / txt-md 五稿合一
// （同 ActionButton 五稿合一先例）。内容区 kind-switch 已注册表化（2026-09-21 R5 扩展点轮，
// 范式 = barSpecs）：kind → src/lib/cardSpecs/ CardContentSpec 查表 = 内容件（缩略图 / 波形 /
// 字形 / 文本）+ kind 专属附加件（悬停胶囊 / 视频播放条 / 字体安装钮）+ 可选滚动胶囊接线；
// 未注册 kind 回退 general；新内容形态 = 加 spec 文件 + 一行注册，本组件零改动。五稿口径：
//        general = 缩略图充满（cover，沿用旧网格口径；square = shell 缩略图 / 自动导入未导入 →
//        正方形卡 + 缩略图占 50% 居中，稿注明确）；
//        video  = 缩略图 + 悬停胶囊（帧数 + 播放/暂停 + 音量/静音）+ 底部 4px 播放条（轨道 亮色/半透明-2、
//        填充 亮色/一级，贴底 4、左右 12）；
//        audio  = 波形（竖线 4px 圆头 gap 4 通用/五级，根数恒定由业务保证、美观优先）+ 悬停胶囊
//        （时长 + 播放/暂停）；进度 = 品牌色波形层 clip-path 丝滑扫过（与根数解耦，稿 prose + 2026-08-29 用户定色）；
//        font   = 示例字块居中（标题档 亮色/四级，日/韩环境示例字业务侧替换）+ 右上 FontInstallButton；
//        text   = 标题（标题档 二级）+ 正文（正文档 四级）滚动预览，**内容随卡宽 cqw 等比缩放**
//        （2026-09-13 实机走查：文档卡片跟随尺寸缩放，稿基准 300 宽比例换算），内容溢出出 6px
//        滚动胶囊（通用/四级，胶囊自身 hover 升六级——2026-09-13 全局滚动条色档定案）；
// 悬停控件（胶囊 / 播放条 / 波形扫过）一律 hover 才出现（2026-08-29 用户定），过渡 150ms；
// 壳 / hover 位移 -4 + 小阴影 / 选中 2px 品牌描边 见 cards.css（kc-media / kc-sel）。
// 业务态（7.1 内容容器轮接线，模式同 cards/FolderCard）：naming = 行内重命名（名称位换输入框；
// 文件口径 Esc = 取消——nameCancel 上抛，TreeRowItem/文件夹卡的 escSaves 不同）；picked = 选择模式
// 已选压暗 30%（大勾在壳层）；thumbFit = 缩略图适配三档（cover 默认 / contain 网格固定格完整显示
// ——旧 AssetCard 口径 / icon 格式图标 50% 居中——Eagle 口径，不与内容缩略图抢视觉噪音）；
// 默认插槽 = 内容区扩展（壳层注入悬停播放层等，叠在缩略图上、渐变罩下）。
// props 契约 = cardSpecs/types.ts FileCardProps（注册表接线单一事实源，注释口径在那边）。
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import type { ComponentPublicInstance } from "vue";
import AppIcon from "../AppIcon.vue";
import FormatBadge from "./FormatBadge.vue";
import { cardContentSpecFor, type FileCardProps } from "../../lib/cardSpecs";
import { generalContentSpec } from "../../lib/cardSpecs/general";
import { useInlineRename } from "../../lib/useInlineRename";
import "./cards.css";

const props = withDefaults(defineProps<FileCardProps>(), {
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
});

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

// 内容形态查表（R5 注册表化）：kind → CardContentSpec；未注册 kind 回退 general
const spec = computed(() => cardContentSpecFor(props.kind) ?? generalContentSpec);

// 滚动胶囊（scrollThumb 形态 = text 卡）：内容溢出才显，轨道 = 内容区（卡 padding 12 内），跟随滚动；
// 拇指在根层渲染（kc-clip 外——picked 压暗不波及）；滚动容器 = 内容件根元素（spec 契约：单根、
// 根即滚动容器），经组件 ref.$el 接线（scroll 监听 + ResizeObserver）；
// 卡片尺寸变化（尺寸滑块）同步重算（2026-09-13 实机走查：文档卡内容随卡宽等比缩放，滚动量随动）
const contentRef = ref<ComponentPublicInstance>();
const thumb = reactive({ show: false, top: 12, height: 0 });
function contentScrollEl(): HTMLElement | undefined {
  const el = contentRef.value?.$el;
  return el instanceof HTMLElement ? el : undefined;
}
function updateThumb(): void {
  if (!spec.value.scrollThumb) return;
  const el = contentScrollEl();
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
let textRo: ResizeObserver | undefined;
onMounted(() => {
  updateThumb();
  if (!spec.value.scrollThumb) return;
  const el = contentScrollEl();
  if (!el) return;
  el.addEventListener("scroll", updateThumb);
  textRo = new ResizeObserver(updateThumb);
  textRo.observe(el);
});
onBeforeUnmount(() => {
  contentScrollEl()?.removeEventListener("scroll", updateThumb);
  textRo?.disconnect();
});
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
      <!-- 内容形态 = cardSpecs 查表分发（缩略图 / 波形 / 字形 / 文本…），props 映射在 spec -->
      <component :is="spec.content" ref="contentRef" v-bind="spec.contentProps(props)" />

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

    <!-- kind 专属附加件（悬停胶囊 / 视频播放条 / 字体安装钮…）：注册表 overlays，DOM 序 = 数组序 -->
    <component
      v-for="(ov, i) in spec.overlays ?? []"
      :key="i"
      :is="ov.comp"
      v-bind="ov.props(props)"
    />

    <!-- 滚动胶囊（scrollThumb 形态 = text 卡：贴右缘，hover 升五级） -->
    <span
      v-if="spec.scrollThumb"
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
  container-type: inline-size; /* text 卡内容按卡宽 cqw 等比缩放（2026-09-13 实机走查） */
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
/* 名称区悬停文本光标（双击可改名，FolderCard / ArchiveCard / ListRowCard 同款先例） */
.kc-name:hover {
  cursor: text;
}

/* kind 内容件 / 附加件样式各随其组件（cards/contents/）—— scoped 只给末段选择器加 data-v，
   件内祖先态选择器（.fc-root:hover …）照常命中；本节宿主只留壳层统一定位与卡级态 */

/* 格式角标定位（左上 12/12） */
.fc-badge {
  position: absolute;
  left: var(--n-space-12);
  top: var(--n-space-12);
  z-index: 3;
}

/* 字体安装钮定位（右上 12/12；类经 spec 映射挂 FontInstallButton 根——fc-badge 同款先例） */
.fc-install {
  position: absolute;
  right: var(--n-space-12);
  top: var(--n-space-12);
  z-index: 4;
}

/* 滚动胶囊：6px 通用/四级贴右缘，自身 hover 升六级（2026-09-13 实机走查全局滚动条色档
   定案应用至卡片——原稿「hover-滚动条」节 g3/g5 档被全局定案覆盖） */
.fc-text-thumb {
  position: absolute;
  right: 0;
  z-index: 3;
  width: 6px;
  border-radius: 99px; /* 稿值 99 */
  background: var(--n-g4);
  transition: background var(--n-dur-150) var(--n-ease-in-out);
}
.fc-text-thumb:hover {
  background: var(--n-g6);
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
