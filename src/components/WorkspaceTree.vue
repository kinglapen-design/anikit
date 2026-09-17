<script setup lang="ts">
// 工作空间树（PRD 02）：根节点「工作空间」+ 懒加载子目录树——
// 展开才挂载子层（v-if），子层挂载即 list_workspace_dir 取子目录（只展示目录，文件进网格视口）；
// 节点点击 = tabs.openView("ws:"+rel)（根 = "ws:"）；复用 TreeRowItem 与 treeExpansion 会话级展开态
// （openSet 以 relPath 为 id，与库文件夹 F id 不撞）；展开动画复用 expandAnim（FolderTree 同口径）。
// workspace/ 目录不存在（exists=false）= 空态文案（正文档 + 亮色/四级，
// 沿用 SidebarLeft empty-state 风格；root=null 是拉取瞬态不渲染误标）；
// 步骤 2 起树行兼拖拽落点（data-folder-id="ws:"+rel，素材移入该目录，落点视觉 dropZone）。
import { computed, onMounted } from "vue";
import TreeRowItem from "./items/TreeRowItem.vue";
import { isTreeOpen, toggleOpen } from "../lib/treeExpansion";
import { expandEnter, expandLeave } from "../lib/expandAnim";
import { dragView } from "../lib/dragOperation";
import { useContentStore } from "../stores/content";
import { useTabsStore } from "../stores/tabs";
import { useWsStore } from "../stores/ws";

// rel = 本层目录（"" = 根）；depth = 本层内容缩进档（TreeRowItem padding-left 6+depth×20）；
// 根层（rel=""）的孩子行缩进 0（根行 = 段首位，同 LibraryView 区块头→树层级关系）
const props = withDefaults(defineProps<{ rel?: string; depth?: number }>(), { rel: "", depth: 0 });

const ws = useWsStore();
const content = useContentStore();
const tabs = useTabsStore();
const isRoot = computed(() => props.rel === "");
// 本层子目录行缩进档 / 递归子层缩进档
const rowDepth = computed(() => (isRoot.value ? 0 : props.depth));
const childDepth = computed(() => rowDepth.value + 1);

// 懒加载：层级挂载即拉本目录 listing（根层随分段切换挂载；子层随父级展开挂载）
onMounted(() => {
  void ws.loadDir(props.rel);
});

const subDirs = computed(() => ws.subDirs(props.rel));
// 展开钮形态：未加载先乐观给箭头（点开即知有没有子目录）；已加载按实际子目录数（无则占位隐藏）
const expandable = (rel: string): boolean =>
  !ws.loadedOk(rel) || ws.subDirs(rel).length > 0;

function openDir(rel: string): void {
  tabs.openView(`ws:${rel}`);
}

// 拖拽落点视觉（PRD 02 步骤 2：树行 = ws 素材移动落点，引擎 dragView.tree 按 "ws:"+rel 命中；
// ws 只有并入（into）语义，before/after 排序不出现）
function dropZone(rel: string): "before" | "after" | "into" | undefined {
  return dragView.tree?.folderId === `ws:${rel}` ? dragView.tree.zone : undefined;
}
</script>

<template>
  <div class="ws-tree" :class="{ root: isRoot }">
    <!-- 根节点「工作空间」（常驻首行；数量 = 根目录直接文件数，未加载不显示） -->
    <TreeRowItem
      v-if="isRoot"
      kind="folder"
      name="工作空间"
      data-folder-id="ws:"
      :drop-zone="dropZone('')"
      :count="ws.loadedOk('') ? ws.fileCount('') : undefined"
      :expanded="isTreeOpen('')"
      :expandable="true"
      :selected="content.sidebarSelected === 'ws:'"
      :depth="0"
      @toggle="toggleOpen('')"
      @click="openDir('')"
    />
    <!-- 子目录行（本层 = props.rel 的子目录；点击导航，展开钮挂 treeExpansion） -->
    <template v-for="d in subDirs" :key="d.relPath">
      <div class="ws-node">
        <TreeRowItem
          kind="folder"
          :name="d.name"
          :data-folder-id="`ws:${d.relPath}`"
          :drop-zone="dropZone(d.relPath)"
          :count="ws.loadedOk(d.relPath) ? ws.fileCount(d.relPath) : undefined"
          :expanded="isTreeOpen(d.relPath)"
          :expandable="expandable(d.relPath)"
          :selected="content.sidebarSelected === `ws:${d.relPath}`"
          :depth="rowDepth"
          @toggle="toggleOpen(d.relPath)"
          @click="openDir(d.relPath)"
        />
        <Transition :css="false" @enter="expandEnter" @leave="expandLeave">
          <div v-if="isTreeOpen(d.relPath)" class="ws-children">
            <!-- 递归子层：挂载即懒加载其 listing -->
            <WorkspaceTree :rel="d.relPath" :depth="childDepth" />
          </div>
        </Transition>
      </div>
    </template>
    <!-- 空态：workspace 目录不存在（exists=false）。root=null 是拉取中 / 未开空间的瞬态
        （ensureRoot null 不缓存、下次调用即重试），不渲染「未开启」误标（2026-09-17 实机报） -->
    <div v-if="isRoot && ws.root !== null && !ws.root.exists" class="ws-empty">
      工作空间目录不存在
    </div>
  </div>
</template>

<style scoped>
/* 树容器（段 padding 沿用 LibraryView .block 口径 8 12；items gap 2） */
.ws-tree {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
}
.ws-tree.root {
  padding: var(--n-space-8) var(--n-space-12);
}
.ws-node {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
}
/* 空态（未开空间 / 目录不存在）：正文档 + 亮色/四级居中（SidebarLeft .empty-state 同档） */
.ws-empty {
  padding: var(--n-space-12) 0;
  text-align: center;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l4);
  user-select: none;
}
</style>
