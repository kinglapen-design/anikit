<script setup lang="ts">
// 树容器 item 族（文件夹 / 快速访问 / 档案）——docs/UI 重构/Design Token/组件/item/树容器 item/
// 壳：宽 100% 随父容器（稿值 230 = 254 侧栏 − 12×2 的画布语境，卡片族尺寸模型口径）× 26 高 /
// padding 4 6 / gap 6 / 圆角 6；hover = 亮色/半透明-3；选中 = 半透明-2 + 立体感-通用 +
// 展开钮/提示装饰/图标/名称/数量提亮 亮色/一级。
// 业务态（2026-08-30 左栏轮用户定案，稿未画——状态描边一律 ::after 伪元素 border，速查 §7）：
//   多选联动 domain-selected = 半透明-2 底 + 1.5px 亮色/二级描边（与当前目录选中互斥由父保证）；
//   拖拽排序插入线 = 2px 品牌/常态（item 上/下沿）；并入 drop-into = 品牌/半透明填充 + 1.5px 品牌/常态描边；
//   重命名输入态 naming = 1.5px 品牌/常态描边（校验违规换 --n-danger），输入框透明底（base.css
//   .rename-input 共享基座，本地覆盖 18 高 + 新字档）；
//   拖拽源 dimmed / 剪切 cut = 整项半透明 0.4 / 0.5（沿用速查 §5 既有档）；
//   层级缩进 = padding-left 6 + depth×20（稿未画层级，沿用旧 20 档）。
// 数量双口径（旧 TreeItem 2026-08-20 定）：folder 展开 = 直接子项数 / 折叠 = 递归总数；0 不显示。
import { computed } from "vue";
import AppIcon from "../AppIcon.vue";
import TwoToneIcon from "./TwoToneIcon.vue";
import { useInlineRename } from "../../lib/useInlineRename";

const props = withDefaults(
  defineProps<{
    kind: "folder" | "archive" | "quick"; // 文件夹 / 档案 / 快速访问
    name: string;
    count?: number | string; // 数量（数字档；不传或 0 不渲染）
    directCount?: number; // folder 直接子项文件数（展开态显示此值，折叠态显示 count 递归总数）
    icon?: string; // quick 型图标名（01_常用 线条图标）；folder 型自定义图标名（覆盖 folder-closed/open 默认）
    expanded?: boolean; // folder 型：展开钮朝向（∨ / >）与文件夹-打开图标联动
    expandable?: boolean; // folder 型：有子项（false = 藏箭头保占位，列对齐）
    selected?: boolean;
    domainSelected?: boolean; // 选择域联动（文件夹卡片选中 / 树多选）：半透明-2 底 + 1.5px 二级描边
    tint?: string; // folder/archive 型图标自定义九色（red/blue/...；空 = 常态双色）
    depth?: number; // 层级（内容缩进：padding-left = 6 + depth×20，稿未画层级沿用旧档）
    dimmed?: boolean; // 拖拽源半透明（0.4）
    cut?: boolean; // 剪切中半透明（0.5）
    dropZone?: "before" | "after" | "into" | null; // 拖拽落点（父级按引擎/本地判定算好传入）
    naming?: boolean; // 行内重命名输入态（useInlineRename，速查 §4）
    namingError?: boolean; // 名称校验违规：描边 --n-danger
    forceState?: "" | "hover" | "selected"; // 预览页强制态
  }>(),
  {
    count: undefined,
    directCount: undefined,
    icon: "",
    expanded: false,
    expandable: true,
    selected: false,
    domainSelected: false,
    tint: "",
    depth: 0,
    dimmed: false,
    cut: false,
    dropZone: null,
    naming: false,
    namingError: false,
    forceState: "",
  },
);

const emit = defineEmits<{
  toggle: [];
  click: [e: MouseEvent]; // 携带修饰键（Ctrl 加选 / Shift 范围选判定在父级）
  nameCommit: [name: string];
}>();

// 数量显示规则：折叠 = 递归总数（提醒"哪个文件夹里有文件"）、展开 = 直接子项数
//（子文件夹已各自成行）；quick 恒折叠态（expanded 不传）；0 不显示（去视觉噪音）；数字千分位（稿 3,134）
const displayCount = computed(() => (props.expanded ? (props.directCount ?? 0) : (props.count ?? 0)));
const showCount = computed(() => {
  const c = displayCount.value;
  if (c === undefined || c === "") return false;
  return !(typeof c === "number" && c <= 0);
});
const countText = computed(() => {
  const c = displayCount.value;
  return typeof c === "number" ? c.toLocaleString() : String(c ?? "");
});

// 行内重命名（useInlineRename 封装，词干选区不包后缀 / blur 恒提交 / Enter/Esc 语义）：
// Enter / Esc / 失焦一律保存退出，空或不变的判定在父级 FolderTree
const { draft: nameDraft, bindInput, commit: commitName } = useInlineRename({
  active: () => props.naming,
  initial: () => props.name,
  escSaves: true,
  onCommit: (name) => emit("nameCommit", name),
});

// 单击选中 + 双击展开/折叠（e.detail 连击计数自判，不用 dblclick——Chromium 只在连击序列
// 第 2 次点击派发，第 4/6… 次不再派发，连续双击展开/折叠会失效，旧 TreeItem 2026-08-08 走查）；
// archive 无展开钮（提示装饰圆点）但保留双击展开（旧 TreeItem 行为一致）
function onClick(e: MouseEvent): void {
  emit("click", e);
  if (props.kind !== "quick" && props.expandable && e.detail > 1 && e.detail % 2 === 0) emit("toggle");
}
</script>

<template>
  <div
    class="tree-item"
    :class="[
      kind,
      {
        selected,
        'domain-selected': domainSelected,
        naming,
        'naming-error': namingError,
        'drop-before': dropZone === 'before',
        'drop-after': dropZone === 'after',
        'drop-into': dropZone === 'into',
        dimmed,
        cut,
      },
      forceState ? `force-${forceState}` : '',
    ]"
    :style="{ paddingLeft: `${6 + depth * 20}px` }"
    @click="onClick"
  >
    <span class="lead">
      <button
        v-if="kind === 'folder'"
        class="exp"
        :class="{ open: expanded, placeholder: !expandable }"
        @click.stop="emit('toggle')"
        @dblclick.stop
      >
        <AppIcon name="expand" :size="14" />
      </button>
      <span v-else-if="kind === 'archive'" class="dot" aria-hidden="true" />
      <template v-if="kind === 'folder'">
        <AppIcon v-if="icon" :name="icon" :size="18" class="fico" />
        <TwoToneIcon v-else :name="expanded ? 'folder-open' : 'folder-closed'" :size="18" :tint="tint" />
      </template>
      <TwoToneIcon v-else-if="kind === 'archive'" name="archive" :size="18" :tint="tint" />
      <span v-else-if="kind === 'quick' && icon" class="qico"><AppIcon :name="icon" :size="18" /></span>
      <!-- 命名态：input 替换名称位（整 item 品牌描边提示，输入框透明底，速查 §4 统一口径） -->
      <input
        v-if="naming"
        :ref="bindInput"
        v-model="nameDraft"
        class="rename-input"
        @keydown.enter.prevent="commitName"
        @keydown.esc.prevent="commitName"
        @blur="commitName"
        @click.stop
        @pointerdown.stop
      />
      <span v-else class="nm">{{ name }}</span>
    </span>
    <span v-if="showCount && !naming" class="cnt">{{ countText }}</span>
  </div>
</template>

<style scoped>
.tree-item {
  position: relative; /* 状态描边 / 插入线伪元素锚点 */
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-6);
  width: 100%;
  height: 26px;
  padding: var(--n-space-4) var(--n-space-6);
  border-radius: var(--n-radius-6);
  cursor: pointer;
  color: var(--n-l2);
  transition: background var(--n-dur-100) var(--n-ease-in-out);
}
/* hover = 亮色/半透明-3；选中 / 多选联动 / 并入高亮行 hover 不再叠罩（选中态不叠 hover 口径） */
.tree-item:hover:not(.selected):not(.domain-selected):not(.drop-into),
.tree-item.force-hover {
  background: var(--n-l-a3);
}
/* 选中 = 半透明-2 + 立体感-通用 + 提亮一级（稿）；force-selected 供预览页静态验收 */
.tree-item.selected,
.tree-item.force-selected {
  background: var(--n-l-a2);
  box-shadow: var(--n-effect-raised);
  color: var(--n-l1);
}
.lead {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  min-width: 0;
  flex: 1;
}
.exp {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  flex: none;
  padding: 0;
  color: var(--n-l4);
  transition:
    transform var(--n-dur-100) var(--n-ease-in-out),
    color var(--n-dur-100) var(--n-ease-in-out);
}
/* 点击热区外扩（2026-08-28 走查通用问题，旧 TreeItem 同款机制）：
   上/下 -4 吃满 item padding、左 -6 吃到 item 左缘、右 -3 吃箭头与图标间距一半 */
.exp::after {
  content: "";
  position: absolute;
  inset: -4px -3px -4px -6px;
}
.exp:not(.open) {
  transform: rotate(-90deg); /* 收起 = >，展开 = ∨（expand.svg 源图形朝下） */
}
/* 无子项：藏箭头保占位（列对齐），不响应点击 */
.exp.placeholder {
  pointer-events: none;
  opacity: 0;
}
.exp:hover {
  color: var(--n-l2);
}
.selected .exp,
.force-selected .exp {
  color: var(--n-l1);
}
.dot {
  width: 18px;
  height: 18px;
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.dot::before {
  content: "";
  width: 8px;
  height: 8px;
  border-radius: 50%; /* 提示装饰：稿 CSS 为 8×8 方块，图实为圆点（以图为准） */
  background: var(--n-l4);
}
.selected .dot::before,
.force-selected .dot::before {
  background: var(--n-l1);
}
.qico,
.fico {
  display: inline-flex;
  width: 18px;
  height: 18px;
  flex: none;
  color: var(--n-l3);
}
/* folder 自定义图标与名称同档（二级）；quick 图标稿 = 三级 */
.fico {
  color: var(--n-l2);
}
.selected .qico,
.force-selected .qico,
.selected .fico,
.force-selected .fico {
  color: var(--n-l1);
}
.nm {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}
/* 命名输入：透明底不单独描边（整 item 1.5px 品牌描边提示）；覆盖共享基座 20 高 → 18（item 内容行高），
   字档 = 正文档 13/18（基座为旧字档） */
.rename-input {
  flex: 1;
  min-width: 0;
  height: 18px;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
  text-align: left;
}
.cnt {
  flex: none;
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  font-weight: var(--n-fw-medium);
  color: var(--n-l3);
}
.selected .cnt,
.force-selected .cnt {
  color: var(--n-l1);
}
/* —— 业务态（2026-08-30 用户定案；描边一律 ::after 伪元素 border，速查 §7）—— */
/* 多选联动：半透明-2 底 + 1.5px 亮色/二级描边（与 selected 互斥由父保证） */
.tree-item.domain-selected {
  background: var(--n-l-a2);
}
.tree-item.domain-selected::after {
  content: "";
  position: absolute;
  inset: 0;
  border: 1.5px solid var(--n-l2);
  border-radius: var(--n-radius-6);
  pointer-events: none;
}
/* 拖拽排序插入线：2px 品牌/常态于 item 上/下沿（仅文件夹拖拽上/下 1/3 区） */
.tree-item.drop-before::before,
.tree-item.drop-after::before {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  height: 2px;
  background: var(--n-brand);
  z-index: 1;
  pointer-events: none;
}
.tree-item.drop-before::before {
  top: 0;
}
.tree-item.drop-after::before {
  bottom: 0;
}
/* 拖拽并入：品牌/半透明填充 + 1.5px 品牌/常态描边 */
.tree-item.drop-into {
  background: var(--n-brand-50);
}
.tree-item.drop-into::after {
  content: "";
  position: absolute;
  inset: 0;
  border: 1.5px solid var(--n-brand);
  border-radius: var(--n-radius-6);
  pointer-events: none;
}
/* 重命名输入态：1.5px 品牌/常态描边；校验违规（重名等）换禁止色 */
.tree-item.naming::after {
  content: "";
  position: absolute;
  inset: 0;
  border: 1.5px solid var(--n-brand);
  border-radius: var(--n-radius-6);
  pointer-events: none;
}
.tree-item.naming-error::after {
  border-color: var(--n-danger);
}
/* 拖拽源 / 剪切：整项半透明（速查 §5 既有档 0.4 / 0.5） */
.tree-item.dimmed {
  opacity: 0.4;
}
.tree-item.cut {
  opacity: 0.5;
}
</style>
