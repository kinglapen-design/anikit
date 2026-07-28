<script setup lang="ts">
// 文件树 item（02_通用组件/02_item 02）：高 28；展开按钮容器宽 8（= 图标宽，高随 item），
// 无子项时保留占位保证装饰图标列对齐。
// 层级缩进 20px：子项展开箭头尖角（缩进 + 4）对齐父级装饰图标中心（缩进 + 24），
// 即缩进 = 8/2 + gap 2 + padding 6 + 图标 16/2 = 20（8×8 图标无 0.5 偏差）。
// 命名输入态（6.1 C3）：受控 naming prop——只服务"新建文件夹后的命名输入"（后端无
// rename_folder，已存在文件夹重命名留待后续，2026-07-28 定案）；Enter 提交 / Esc·失焦取消。
// 拖拽落点（6.1 C4c）：item 高度三等分——上/下 1/3 出 2px 插入线 + hover 底（仅文件夹拖拽），
// 中间 1/3（素材任何位置）= 并入高亮（--brand-1-12 + inset 1px --brand-1）。
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
  nameCancel: [];
}>();

// 数量标签显示规则（2026-07-25 用户定）：0 不显示（去视觉噪音）；父级展开后隐藏——
// 数量的作用是提醒"哪个文件夹里有文件"，展开后用户已在观察父级内容，折叠后重新显示
const showCount = computed(() => props.count !== undefined && props.count > 0 && !props.open);

// 命名输入（C3）：挂载聚焦全选；Enter 提交（校验在父级，违规保持输入态）/ Esc·失焦取消
const nameDraft = ref("新建文件夹");
const nameInput = ref<HTMLInputElement>();
watch(
  () => props.naming,
  (n) => {
    if (!n) return;
    nameDraft.value = "新建文件夹";
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
</script>

<template>
  <div
    class="tree-item"
    :class="{
      leaf: guide && !expandable && depth > 0,
      last,
      'drop-before': dropZone === 'before',
      'drop-after': dropZone === 'after',
      dimmed,
    }"
    :style="{ paddingLeft: `${depth * 20}px` }"
    :data-folder-id="folderId"
  >
    <button class="expand" :class="{ placeholder: !expandable }" @click.stop="emit('toggle')">
      <AppIcon name="tree-expand-arrow" :size="8" class="chev" :class="{ open }" />
    </button>
    <!-- 命名态用 div（input 套 button 是非法嵌套，Chromium 虽可用但行为无保障） -->
    <div v-if="naming" class="body">
      <AppIcon :name="icon" class="deco" />
      <input
        ref="nameInput"
        v-model="nameDraft"
        class="name-input"
        :class="{ error: namingError }"
        @keydown.enter.prevent="emit('nameCommit', nameDraft.trim())"
        @keydown.esc.prevent="emit('nameCancel')"
        @blur="emit('nameCancel')"
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
      <AppIcon :name="icon" class="deco" />
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
  height: 28px;
}
/* 拖拽源整项半透明（2026-07-28 复验反馈 6①，拖拽结束引擎清态即恢复） */
.tree-item.dimmed {
  opacity: var(--opacity-drag-source);
}
/* 展开按钮容器：宽 8（= 图标宽）/ 高 28（判定高度），图标 8×8 实心三角，展开时旋转 90°；
   有箭头的容器铺侧栏底色——遮住穿过的缩进竖线（视觉不穿过箭头，同 Eagle） */
.expand {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 8px;
  height: 28px;
  /* 展开箭头常态 font-icon-3（02_item：容器内图标），hover 提亮 font-icon-2 */
  color: var(--font-icon-3);
  flex: none;
}
.expand:not(.placeholder) {
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
.chev.open {
  transform: rotate(90deg);
}
.expand:hover .chev {
  color: var(--font-icon-2);
}
/* L 形横线：仅最后一个叶子 item，x4 → x8（竖线由 ft-children 容器级绘制、
   止于本行中心，与此横线组成 L；竖线 1px 位于 x4-5 正中）；中间子项纯竖线，不画横线 */
.tree-item.leaf.last .expand::after {
  content: "";
  position: absolute;
  left: 4px;
  top: 50%;
  width: 4px;
  height: 1px;
  background: var(--font-icon-6);
}
/* 拖拽排序插入线（C4c，仅文件夹拖拽上/下 1/3）：2px --brand-1 于 item 上/下沿，
   item 本身带 hover 底（--font-icon-6） */
.tree-item.drop-before::before,
.tree-item.drop-after::after {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  height: 2px;
  background: var(--brand-1);
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
  background: var(--font-icon-6);
}
/* item 容器：内边距 4 6 / 圆角 6 / 间距 6 */
.body {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  flex: 1;
  min-width: 0;
  padding: var(--space-4) var(--space-6);
  border-radius: var(--radius-6);
  color: var(--font-icon-1);
  transition: background 0.12s ease;
}
.body:hover {
  background: var(--font-icon-6);
}
.body.selected {
  background: var(--font-icon-5);
}
/* 拖拽并入高亮（C4c 中间 1/3 / 素材任何位置）：--brand-1-12 底 + inset 1px --brand-1 */
.body.drop-into {
  background: var(--brand-1-12);
  box-shadow: inset 0 0 0 1px var(--brand-1);
}
/* 装饰图标线条 font-icon-2（80%），选中提亮 font-icon-1（100%）；
   填充层 SVG 直接引用 var(--font-icon-4)，恒 12% 不随 color、选中不变（2026-07-23 用户定） */
.deco {
  color: var(--font-icon-2);
}
.body.selected .deco {
  color: var(--font-icon-1);
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
/* 命名输入（C3）：C4 底 / 1px border-1 / 圆角 6（输入框 token 口径）；违规描边 --error */
.name-input {
  flex: 1;
  min-width: 0;
  box-sizing: border-box;
  padding: 0 var(--space-6);
  border: none;
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
  color: var(--font-icon-1);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  text-align: left;
  outline: none;
}
.name-input.error {
  box-shadow: inset 0 0 0 1px var(--error);
}
.count {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
  flex: none;
}
.body.selected .count {
  color: var(--font-icon-1);
}
</style>
