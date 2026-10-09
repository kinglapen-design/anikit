<script setup lang="ts">
// 筛选弹窗 · 文件夹（稿 = 中间面板-多功能悬浮栏-筛选/文件夹 350×468）：头行（搜索 + 清除，
// 同属性弹窗两段式）+ 树区（max-h 424 滚轮）——首项「未分类」（root 直属数）+ 库文件夹树。
// 树行 26 高：展开箭头 18（有子级才出，expand 旋转口径）+ 文件夹 icon 18（folder-open/-closed
// 双色，TwoToneIcon 通道）+ 名称 + 数量（Fira Mono 12 三级，FolderNode.count 递归总数）
// + 对勾 18；选中 = 半透明-2 底 + 立体感-通用 + 一级文字 + 对勾显（稿 prose），hover 半透明-3。
// 勾选 = folderIds（含后代口径 = 后端预展开，2026-10-09 三轮定：勾父圈整棵子树）；
// 清除 = 清空全部选中（稿定：不清搜索输入）。搜索 = 名称子串过滤，命中节点连同祖先保留并强制展开。
// 树渲染 = 扁平化数组 + TransitionGroup + expandAnim（150ms 展开口径，DestTreeDialog 同径）。
import { computed, ref } from "vue";
import AppIcon from "../AppIcon.vue";
import TagButton from "../buttons/TagButton.vue";
import TextInput from "../inputs/TextInput.vue";
import TwoToneIcon from "../items/TwoToneIcon.vue";
import type { FolderNode } from "../../stores/content";
import { expandEnter, expandLeave } from "../../lib/expandAnim";
import "./bars.css";

const props = withDefaults(
  defineProps<{
    nodes?: readonly FolderNode[]; // 库文件夹树（content.folders）
    rootCount?: number; // 未分类直属数
    selected?: readonly string[]; // 后端 folderIds 原样
    forceHover?: string; // 预览页静态验收：某行 hover
  }>(),
  { nodes: () => [], rootCount: 0, selected: () => [], forceHover: undefined },
);
const emit = defineEmits<{ toggle: [id: string]; clear: [] }>();

const query = ref("");
const collapsed = ref(new Set<string>()); // 默认全展开（稿图示多层展开态）

// 搜索过滤：命中名称的节点连同其祖先保留（搜索态强制全展）
const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return props.nodes;
  const keep = (list: readonly FolderNode[]): FolderNode[] =>
    list.flatMap((n) => {
      const children = keep(n.children);
      return n.name.toLowerCase().includes(q) || children.length > 0 ? [{ ...n, children }] : [];
    });
  return keep(props.nodes);
});

interface FlatRow {
  node: FolderNode;
  depth: number;
  hasChildren: boolean;
  expanded: boolean;
}
const flatRows = computed<FlatRow[]>(() => {
  const searching = !!query.value.trim();
  const out: FlatRow[] = [];
  const walk = (list: readonly FolderNode[], depth: number): void => {
    for (const node of list) {
      const hasChildren = node.children.length > 0;
      const expanded = searching || !collapsed.value.has(node.id);
      out.push({ node, depth, hasChildren, expanded });
      if (hasChildren && expanded) walk(node.children, depth + 1);
    }
  };
  walk(filtered.value, 0);
  return out;
});

function toggleCollapse(id: string): void {
  const s = new Set(collapsed.value);
  if (s.has(id)) s.delete(id);
  else s.add(id);
  collapsed.value = s;
}
</script>

<template>
  <div class="fb-pop fp-root fp-folders">
    <div class="fp-head">
      <TextInput v-model="query" size="sm" leading-icon="search" placeholder="搜索..." />
      <TagButton label="清除" size="md" @click="emit('clear')" />
    </div>
    <div class="fp-tree-wrap">
      <TransitionGroup tag="div" class="fp-tree fp-list" :css="false" @enter="expandEnter" @leave="expandLeave">
        <!-- 未分类（root 直属；无子级不出展开箭头，图标 = uncategorized 既有口径） -->
        <button
          key="__root__"
          class="fp-item"
          :class="[{ on: selected.includes('root'), 'force-hover': forceHover === 'root' }]"
          @click="emit('toggle', 'root')"
        >
          <span class="fp-arrow"></span>
          <AppIcon name="uncategorized" :size="18" class="fp-folder" />
          <span class="fp-name">未分类</span>
          <span class="fp-count">{{ rootCount }}</span>
          <AppIcon name="check" :size="18" class="fp-check" />
        </button>
        <button
          v-for="row in flatRows"
          :key="row.node.id"
          class="fp-item"
          :class="[{ on: selected.includes(row.node.id), 'force-hover': forceHover === row.node.id }]"
          :style="{ paddingLeft: `${8 + row.depth * 18}px` }"
          @click="emit('toggle', row.node.id)"
        >
          <AppIcon
            v-if="row.hasChildren"
            name="expand"
            :size="18"
            class="fp-arrow"
            :class="{ open: row.expanded }"
            @click.stop="toggleCollapse(row.node.id)"
          />
          <span v-else class="fp-arrow"></span>
          <TwoToneIcon :name="row.node.icon ?? (row.expanded && row.hasChildren ? 'folder-open' : 'folder-closed')" :size="18" class="fp-folder" />
          <span class="fp-name">{{ row.node.name }}</span>
          <span class="fp-count">{{ row.node.count }}</span>
          <AppIcon name="check" :size="18" class="fp-check" />
        </button>
      </TransitionGroup>
    </div>
  </div>
</template>

<style scoped>
.fp-folders {
  display: flex;
  flex-direction: column;
  width: 350px;
}
.fp-head {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  height: 44px;
  padding: var(--n-space-8) var(--n-space-8) var(--n-space-8) var(--n-space-12);
}
/* 树区（稿 424 高滚轮；与头行 1px 分隔线） */
.fp-tree-wrap {
  max-height: 424px;
  overflow-y: auto;
  border-top: 1px solid var(--n-l-a3);
}
.fp-tree {
  gap: var(--n-space-2);
}
/* 行件：展开箭头（expand 旋转 90° 朝下 = 展开态，树容器族口径）/ 文件夹 icon / 名称 / 数量 */
.fp-arrow {
  flex: none;
  width: 18px;
  height: 18px;
  color: var(--n-l3);
  transition: transform var(--n-dur-150) var(--n-ease-in-out);
}
.fp-arrow.open {
  transform: rotate(90deg);
}
.fp-folder {
  flex: none;
}
/* 文件夹双色随选中升亮（稿：未选 #666/#CCC → 选中 #CCC/#F2F2F2；近似 token = l3·l2 → l2·l1，
   TwoToneIcon 变量通道覆盖，双类特异性压组件默认档） */
.fp-item .fp-folder {
  --tone-1: var(--n-l2);
  --tone-2: var(--n-l3);
}
.fp-item.on .fp-folder {
  --tone-1: var(--n-l1);
  --tone-2: var(--n-l2);
}
.fp-name {
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 数量（数字档 Fira Mono 12 三级，稿值） */
.fp-count {
  margin-left: auto;
  flex: none;
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  color: var(--n-l3);
}
/* 数量与对勾并存：对勾不再 auto 左距（数量已顶到右） */
.fp-count + .fp-check {
  margin-left: var(--n-space-6);
}
</style>
