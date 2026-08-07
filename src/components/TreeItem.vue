<script setup lang="ts">
// 文件树 item（figma-css 02_目录 item）：高 32、padding 6/8、radius 6；
// variant 四形态：root = 根父级（C5 底 + 1px 亮-选中边框）/ child = 子级（C5-底色底，默认）/
// archive = 档案（C5-底色底 + 品牌色提示圆点替代展开箭头）/ quick = 快速访问（无底）；
// tint = 自定义色名（red/orange/...）：root 底 = custom-{tint}，child/archive 底 = custom-{tint}-bg；
// hover 叠 暗-悬停；选中叠 亮-选中 + 1.5px 文字常态内阴影边框（不掺白，2026-08-07 用户定）。
// 层级缩进 20px（1.3 重构 FolderTree 时按新箭头容器 12 再校准）。
// 命名输入态（6.1 C3）：受控 naming prop——Enter / Esc / 失焦一律保存退出，空或不变 = 保留默认名。
// 拖拽落点（6.1 C4c）：item 高度三等分——上/下 1/3 出 2px 插入线 + hover 底（仅文件夹拖拽），
// 中间 1/3（素材任何位置）= 并入高亮（品牌 12% 底 + inset 1px 品牌色）。
import { computed, nextTick, ref, watch } from "vue";
import AppIcon from "./AppIcon.vue";
import { dragView } from "../lib/dragOperation";

const props = withDefaults(
  defineProps<{
    icon: string;
    label: string;
    count?: number;
    depth?: number;
    expandable?: boolean;
    open?: boolean;
    selected?: boolean;
    variant?: "root" | "child" | "archive" | "quick";
    tint?: string; // 自定义色名（custom-{tint} / custom-{tint}-bg）
    dimmed?: boolean; // 拖拽源半透明（2026-07-28 复验反馈 6①）
    last?: boolean;
    guide?: boolean;
    folderId?: string; // 拖拽导入落点判定（App.vue elementFromPoint → closest）
    naming?: boolean; // 命名输入态（C3 新建文件夹临时节点）
    namingError?: boolean; // 名称校验违规：输入框描边 --error
  }>(),
  {
    count: undefined,
    depth: 0,
    expandable: false,
    open: false,
    selected: false,
    variant: "child",
    tint: undefined,
    dimmed: false,
    last: false,
    guide: false,
    folderId: undefined,
    naming: false,
    namingError: false,
  },
);
const emit = defineEmits<{
  toggle: [];
  select: [e: MouseEvent]; // 携带修饰键（2026-07-28 复验反馈 5：Ctrl 加选 / Shift 范围选）
  nameCommit: [name: string];
}>();

// 数量标签显示规则（2026-07-25 用户定）：0 不显示（去视觉噪音）；父级展开后隐藏——
// 数量的作用是提醒"哪个文件夹里有文件"，展开后用户已在观察父级内容，折叠后重新显示
const showCount = computed(() => props.count !== undefined && props.count > 0 && !props.open);

// 命名输入（2026-07-29 复验三轮"先建后命名"）：初值 = 节点当前名（默认名），挂载聚焦全选；
// Enter / Esc / 失焦一律保存退出（空或不变的判定在父级 FolderTree）
const nameDraft = ref("");
const nameInput = ref<HTMLInputElement>();
watch(
  () => props.naming,
  (n) => {
    if (!n) return;
    nameDraft.value = props.label;
    void nextTick(() => {
      nameInput.value?.focus();
      nameInput.value?.select();
    });
  },
  { immediate: true },
);

// 拖拽落点视觉（C4c）：引擎按 data-folder-id 匹配，zone 见 dragOperation
const dropZone = computed(() =>
  props.folderId && dragView.tree?.folderId === props.folderId ? dragView.tree.zone : null,
);

// 自定义色（3.3 预留）：root 底 = custom-{tint}，child/archive = custom-{tint}-bg；
// 经 --tint-bg 注入，hover/选中罩层照常叠加（不掺白，2026-08-07 用户定）
const tintBg = computed(() => {
  if (!props.tint) return undefined;
  return `var(--custom-${props.tint}${props.variant === "root" ? "" : "-bg"})`;
});
</script>

<template>
  <div
    class="tree-item"
    :class="[
      variant,
      {
        leaf: guide && !expandable && depth > 0,
        last,
        'drop-before': dropZone === 'before',
        'drop-after': dropZone === 'after',
        dimmed,
      },
      tint ? `tint-${tint}` : '',
    ]"
    :style="{ paddingLeft: `${depth * 20}px`, '--tint-bg': tintBg }"
    :data-folder-id="folderId"
  >
    <!-- 档案 item：品牌色提示圆点替代展开箭头（figma 02_目录 item 3） -->
    <span v-if="variant === 'archive'" class="expand archive-dot-slot">
      <AppIcon name="archive-dot" :size="12" />
    </span>
    <button v-else class="expand" :class="{ placeholder: !expandable }" @click.stop="emit('toggle')">
      <AppIcon name="tree-expand-arrow" :size="12" class="chev" :class="{ closed: !open }" />
    </button>
    <!-- 命名态用 div（input 套 button 是非法嵌套，Chromium 虽可用但行为无保障）；
         Enter / Esc / 失焦一律保存（2026-07-29 复验三轮，不再有"取消新建"） -->
    <div v-if="naming" class="body">
      <AppIcon :name="icon" :size="18" class="deco" />
      <input
        ref="nameInput"
        v-model="nameDraft"
        class="name-input"
        :class="{ error: namingError }"
        @keydown.enter.prevent="emit('nameCommit', nameDraft.trim())"
        @keydown.esc.prevent="emit('nameCommit', nameDraft.trim())"
        @blur="emit('nameCommit', nameDraft.trim())"
        @click.stop
        @pointerdown.stop
      />
    </div>
    <button
      v-else
      class="body"
      :class="{ selected, 'drop-into': dropZone === 'into' }"
      @click="emit('select', $event)"
      @dblclick="expandable && emit('toggle')"
    >
      <AppIcon :name="icon" :size="18" class="deco" />
      <span class="label">{{ label }}</span>
      <span v-if="showCount" class="count">{{ (count ?? 0).toLocaleString() }}</span>
    </button>
  </div>
</template>

<style scoped>
.tree-item {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--space-2);
  height: 32px;
}
/* 拖拽源整项半透明（2026-07-28 复验反馈 6①，拖拽结束引擎清态即恢复） */
.tree-item.dimmed {
  opacity: var(--opacity-drag-source);
}
/* 展开按钮容器：12×12（figma 02_目录 item），高 32（判定高度）；
   箭头源图形向下：收起 = rotate(-90) 朝右、展开 = 无旋转朝下（与 figma 导出态一致）；
   有箭头的容器铺侧栏底色——遮住穿过的缩进竖线（视觉不穿过箭头，同 Eagle） */
.expand {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 12px;
  height: 32px;
  color: var(--text-label);
  flex: none;
}
.expand:not(.placeholder):not(.archive-dot-slot) {
  background: var(--c3);
}
/* 无子项时占位保留（藏图标不藏容器：叶子行的竖线要照常穿过） */
.expand.placeholder {
  pointer-events: none;
}
.expand.placeholder .chev {
  opacity: 0;
}
.chev {
  transition: transform 0.15s ease;
}
.chev.closed {
  transform: rotate(-90deg);
}
.expand:hover .chev {
  color: var(--text-normal);
}
/* 档案提示圆点：品牌色（SVG 自带结构色），替代展开箭头 */
.archive-dot-slot {
  color: var(--brand);
}
/* L 形横线：仅最后一个叶子 item（竖线由 ft-children 容器级绘制，止于本行中心） */
.tree-item.leaf.last .expand::after {
  content: "";
  position: absolute;
  left: 6px;
  top: 50%;
  width: 4px;
  height: 1px;
  background: var(--line-hover);
}
/* 拖拽排序插入线（C4c，仅文件夹拖拽上/下 1/3）：2px 品牌色于 item 上/下沿 */
.tree-item.drop-before::before,
.tree-item.drop-after::after {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  height: 2px;
  background: var(--brand);
  z-index: 1;
  pointer-events: none;
}
.tree-item.drop-before::before {
  top: 0;
}
.tree-item.drop-after::after {
  bottom: 0;
}
.tree-item.drop-before .body,
.tree-item.drop-after .body {
  background: linear-gradient(var(--line-hover), var(--line-hover)), var(--tint-bg, transparent);
}
/* item 容器：padding 6/8 / 圆角 6 / 间距 6（figma 02_目录 item，32 高）；
   --tint-bg 由 tint prop 注入（root = custom-{tint}，child/archive = custom-{tint}-bg） */
.body {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  flex: 1;
  min-width: 0;
  padding: var(--space-6) var(--space-8);
  border-radius: var(--radius-6);
  color: var(--text-normal);
  background: var(--tint-bg, transparent);
  transition: background 0.15s ease;
}
/* 形态底色（tint 时由 --tint-bg 覆盖） */
.tree-item.root .body {
  background: var(--tint-bg, var(--c5));
  box-shadow: inset 0 0 0 1px var(--line-active);
}
.tree-item.child .body,
.tree-item.archive .body {
  background: var(--tint-bg, var(--c5-base));
}
/* hover 叠 暗-悬停 罩层（不掺白，2026-08-07 用户定） */
.tree-item.quick .body:hover {
  background: linear-gradient(var(--line-hover), var(--line-hover)), var(--tint-bg, transparent);
}
.tree-item.root .body:hover {
  background: linear-gradient(var(--line-hover), var(--line-hover)), var(--tint-bg, var(--c5));
}
.tree-item.child .body:hover,
.tree-item.archive .body:hover {
  background: linear-gradient(var(--line-hover), var(--line-hover)), var(--tint-bg, var(--c5-base));
}
/* 选中：叠 亮-选中 罩层 + 1.5px 文字常态内阴影边框（figma 02_目录 item 动画效果） */
.tree-item .body.selected {
  background: linear-gradient(var(--line-active), var(--line-active)), var(--tint-bg, transparent);
  box-shadow: inset 0 0 0 var(--border-strong) var(--text-normal);
}
.tree-item.root .body.selected {
  background: linear-gradient(var(--line-active), var(--line-active)), var(--tint-bg, var(--c5));
}
.tree-item.child .body.selected,
.tree-item.archive .body.selected {
  background: linear-gradient(var(--line-active), var(--line-active)), var(--tint-bg, var(--c5-base));
}
/* 拖拽并入高亮（C4c 中间 1/3 / 素材任何位置）：品牌 12% 底 + inset 1px 品牌色 */
.body.drop-into {
  background: var(--brand-1-12);
  box-shadow: inset 0 0 0 1px var(--brand);
}
/* 装饰图标 18px 常态档（figma：图标 svg = 文字/常态） */
.deco {
  color: var(--text-normal);
  flex: none;
}
.label {
  flex: 1;
  min-width: 0;
  text-align: left;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 命名输入（C3）：高度恒 20 = 字体行高（2026-07-29 用户定：不撑高 item）；
   C1 底 / 圆角 8（输入框 token）；聚焦品牌色 1.5px；违规描边 --error */
.name-input {
  flex: 1;
  min-width: 0;
  height: 20px;
  box-sizing: border-box;
  padding: 0 var(--space-6);
  border: none;
  border-radius: var(--radius-8);
  background: var(--c1);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  color: var(--text-normal);
  font-size: var(--text-body-size);
  line-height: 20px;
  text-align: left;
  outline: none;
}
.name-input:focus {
  box-shadow:
    inset 0 0 0 1px var(--line-hover),
    inset 0 0 0 var(--border-strong) var(--brand);
}
.name-input.error,
.name-input.error:focus {
  box-shadow: inset 0 0 0 1px var(--error);
}
/* 数量标签：标签档，右侧（figma 02_目录 item） */
.count {
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  color: var(--text-label);
  flex: none;
}
</style>
