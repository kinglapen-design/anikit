<script setup lang="ts">
// 档案卡片——docs/UI 重构/Design Token/组件/卡片/档案卡片.md：
// 正方形比例（稿注明确，缩略图网格填充 = 封面 cover 充满）；渐变罩三行 = 名称（标题档 一级）/
// 属性标签行（数字档 二级，" | " 分隔）/ 文件数量行（" / " 分隔）；
// 左上 = 格式角标统一样式（档案 = 属性文字，统一铬不带自定义色——旧 badge.color 底色芯片口径作废）；
// 右下 = 链接文件-徽章 14 亮色/二级。
// 壳 / hover 位移 -4 + 小阴影 / 选中 2px 品牌描边 见 cards.css（kc-media / kc-sel）。
// 业务态（7.1 内容容器轮接线，模式同 items/TreeRowItem / cards/FolderCard）：naming = 行内重命名
//（useInlineRename，名称位换输入框）；dimmed / cut = 整卡半透明 0.4 / 0.5（速查 §5）；
// dropInto = 品牌/半透明填充罩 + 常态描边层换 2px 品牌/常态（速查 §7 / §20）；
// picked = 选择模式已选压暗 30% + 绿圆大勾（§12 四重标识之四）；pickerSelf = 发起档案自身
// 剪切档半透明 + 禁选（2026-08-24 用户定）。
import { computed } from "vue";
import AppIcon from "../AppIcon.vue";
import FormatBadge from "./FormatBadge.vue";
import { useInlineRename } from "../../lib/useInlineRename";
import "./cards.css";

const props = withDefaults(
  defineProps<{
    name: string;
    coverUrl?: string; // 封面图（网格填充合成，业务侧给）
    badge?: string; // 左上角标（档案 = 属性文字，如「主角」）
    attrs?: string[]; // 属性标签行（段间 " | "）
    stats?: string[]; // 文件数量行（段间 " / "）
    linkedBadge?: boolean; // 链接文件-徽章（右下 12/12）
    selected?: boolean;
    dimmed?: boolean; // 拖拽源半透明（0.4）
    cut?: boolean; // 剪切半透明（0.5）
    naming?: boolean; // 行内重命名输入态
    dropInto?: boolean; // 拖拽并入落点（父级按引擎判定算好传入）
    picked?: boolean; // 选择模式已选（压暗 + 大勾）
    pickerSelf?: boolean; // 选择模式发起档案自身（半透明 + 禁选）
    forceState?: "" | "hover" | "selected";
  }>(),
  {
    coverUrl: "",
    badge: "",
    attrs: () => [],
    stats: () => [],
    linkedBadge: false,
    selected: false,
    dimmed: false,
    cut: false,
    naming: false,
    dropInto: false,
    picked: false,
    pickerSelf: false,
    forceState: "",
  },
);

const emit = defineEmits<{
  nameCommit: [name: string];
  nameEdit: []; // 名称双击 = 进命名态（父级置 naming）
}>();

// 行内重命名（useInlineRename，速查 §4；空/不变判定在父级，nameCommit 上抛）
const { draft: nameDraft, bindInput, commit: commitName } = useInlineRename({
  active: () => props.naming,
  initial: () => props.name,
  escSaves: true,
  onCommit: (name) => emit("nameCommit", name),
});

// 名称悬停 tip（2026-08-10 用户定口径沿用）：完整名称 + 简介档（属性行 / 数量行拼一行，column 竖排）
const nameTip = computed(() => {
  const meta = [...(props.attrs.length ? [props.attrs.join(" | ")] : []), props.stats.join(" / ")]
    .filter(Boolean)
    .join(" / ");
  return { text: props.name, keys: meta || undefined, column: true };
});
</script>

<template>
  <div
    class="kc-media kc-bordered kc-sel ar-card"
    :class="[
      { selected, dimmed, cut, naming, 'drop-into': dropInto, picked, 'picker-self': pickerSelf },
      forceState ? `force-${forceState}` : '',
    ]"
  >
    <div class="kc-clip">
      <img v-if="coverUrl" :src="coverUrl" class="ar-cover" alt="" draggable="false" />
      <div class="kc-overlay">
        <!-- 命名态：input 替换名称位（整卡品牌环提示，输入框透明底，速查 §4 统一口径） -->
        <input
          v-if="naming"
          :ref="bindInput"
          v-model="nameDraft"
          class="rename-input"
          @keydown.enter.prevent="commitName"
          @keydown.esc.prevent="commitName"
          @blur="commitName"
          @click.stop
          @dblclick.stop
          @pointerdown.stop
        />
        <span v-else class="ar-name" v-tip="nameTip" @dblclick.stop="emit('nameEdit')">{{ name }}</span>
        <span v-if="attrs.length" class="ar-row">
          <template v-for="(a, i) in attrs" :key="i">
            <span v-if="i > 0" class="ar-sep">|</span>
            <span>{{ a }}</span>
          </template>
        </span>
        <span v-if="stats.length" class="ar-row">
          <template v-for="(s, i) in stats" :key="i">
            <span v-if="i > 0" class="ar-sep">/</span>
            <span>{{ s }}</span>
          </template>
        </span>
      </div>
    </div>

    <FormatBadge v-if="badge" class="ar-badge" :label="badge" />
    <AppIcon
      v-if="linkedBadge"
      name="linked-file-badge"
      :size="14"
      class="kc-corner-badge ar-linked"
    />
    <!-- 拖拽并入填充罩：品牌/半透明盖全部内容（描边层在其上换 2px 品牌） -->
    <span v-if="dropInto" class="ar-drop-fill" aria-hidden="true" />
    <!-- 选择模式已选大勾（§12）：绿圆 44 + check 22 居中整卡；不随压暗（在压暗作用域外） -->
    <span v-if="picked" class="ar-pick-check" aria-hidden="true">
      <AppIcon name="check" :size="22" />
    </span>
  </div>
</template>

<style scoped>
.ar-card {
  width: 100%;
  height: auto;
  aspect-ratio: 1 / 1; /* 稿注：保持正方形比例 */
}
.ar-cover {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  border-radius: var(--n-radius-8); /* 图片自圆角（FileCard 缩略图同款，2026-08-30 角部锯齿治理） */
}
.ar-badge {
  position: absolute;
  left: var(--n-space-12);
  top: var(--n-space-12);
  z-index: 3;
}
.ar-linked {
  color: var(--n-l2);
}

/* 名称 = 标题档 一级；属性 / 数量行 = 数字档 二级，分隔符同行 */
.ar-name {
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 名称区悬停文本光标（双击可改名） */
.ar-name:hover {
  cursor: text;
}
/* 命名输入：透明底不单独描边；覆盖共享基座 = 标题档 18/26 与名称视觉连续 */
.rename-input {
  width: 100%;
  min-width: 0;
  height: 26px;
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
  text-align: left;
}
/* 命名态整卡品牌描边（复用 .kc-sel 选中环层，opacity 恒亮）；常态环让位透明（cards.css 选中同款） */
.ar-card.naming::before {
  opacity: 1;
}
.ar-card.naming::after {
  border-color: transparent;
}
.ar-row {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--n-space-4);
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  font-weight: var(--n-fw-medium);
  color: var(--n-l2);
  white-space: nowrap;
  overflow: hidden;
}

/* 拖拽源 / 剪切：整卡半透明（速查 §5 既有档 0.4 / 0.5） */
.ar-card.dimmed {
  opacity: 0.4;
}
.ar-card.cut {
  opacity: 0.5;
}
/* 拖拽并入：品牌/半透明填充罩（z 6 盖内容）+ 常态描边层换 2px 品牌/常态（同层换档瞬时） */
.ar-drop-fill {
  position: absolute;
  inset: 0;
  z-index: 6;
  border-radius: var(--n-radius-8);
  background: var(--n-brand-50);
  pointer-events: none;
}
.ar-card.drop-into::after {
  border: 2px solid var(--n-brand);
}
/* 选择模式已选（§12）：内容压暗 30% 档（大勾在压暗作用域外不随压） */
.ar-card.picked .kc-clip,
.ar-card.picked .ar-badge {
  opacity: 0.3;
}
/* 选择模式发起档案自身（2026-08-24 用户定）：剪切档半透明 + 禁选光标 */
.ar-card.picker-self {
  opacity: 0.5;
  cursor: not-allowed;
}
/* 已选大勾：成功色圆底 44 + check 22（烘焙 亮色/一级 原值），居中整卡（正方形封面区） */
.ar-pick-check {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  z-index: 9;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  border-radius: 50%;
  background: var(--n-success);
  pointer-events: none;
}
</style>
