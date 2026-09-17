<script setup lang="ts">
// ws 子目录行列表（WorkspaceTree 根层 / 递归子层共用，避免行模板两处重复）：
// 本层 = rel 的子目录行（点击导航 ws 视口，展开钮挂 treeExpansion，展开递归 WorkspaceTree）+
// 行包装 div 右键 wsTreeRowSpec（打开 / 固定·取消固定 / 资源管理器 / 刷新）+
// 拖拽落点（data-folder-id="ws:"+rel，落点视觉 dropZone）。
import { computed } from "vue";
import TreeRowItem from "./items/TreeRowItem.vue";
import WorkspaceTree from "./WorkspaceTree.vue";
import { openContextMenu } from "../lib/contextMenus";
import { wsTreeRowSpec } from "../lib/menuSpecs/workspace";
import { isTreeOpen, toggleOpen } from "../lib/treeExpansion";
import { expandEnter, expandLeave } from "../lib/expandAnim";
import { dragView } from "../lib/dragOperation";
import { useContentStore } from "../stores/content";
import { useTabsStore } from "../stores/tabs";
import { useWsStore } from "../stores/ws";

// rel = 本层目录（"" = 根）；depth = 本层行缩进档（TreeRowItem padding-left 6+depth×20）
const props = withDefaults(defineProps<{ rel?: string; depth?: number }>(), { rel: "", depth: 0 });

const ws = useWsStore();
const content = useContentStore();
const tabs = useTabsStore();
const childDepth = computed(() => props.depth + 1);

const subDirs = computed(() => ws.subDirs(props.rel));
// 展开钮形态：未加载先乐观给箭头（点开即知有没有子目录）；已加载按实际子目录数（无则占位隐藏）
const expandable = (rel: string): boolean =>
  !ws.loadedOk(rel) || ws.subDirs(rel).length > 0;

function openDir(rel: string): void {
  tabs.openView(`ws:${rel}`);
}

// 行右键菜单（固定·取消固定按清单状态自动切换，spec 内分流）
function onRowMenu(e: MouseEvent, rel: string): void {
  openContextMenu(e, wsTreeRowSpec(rel), undefined);
}

// 拖拽落点视觉（PRD 02 步骤 2：树行 = ws 素材移动落点，引擎 dragView.tree 按 "ws:"+rel 命中；
// ws 只有并入（into）语义，before/after 排序不出现）
function dropZone(rel: string): "before" | "after" | "into" | undefined {
  return dragView.tree?.folderId === `ws:${rel}` ? dragView.tree.zone : undefined;
}
</script>

<template>
  <div class="ws-rows">
    <template v-for="d in subDirs" :key="d.relPath">
      <div class="ws-node" @contextmenu.prevent.stop="onRowMenu($event, d.relPath)">
        <TreeRowItem
          kind="folder"
          :name="d.name"
          :data-folder-id="`ws:${d.relPath}`"
          :drop-zone="dropZone(d.relPath)"
          :count="ws.loadedOk(d.relPath) ? ws.fileCount(d.relPath) : undefined"
          :expanded="isTreeOpen(d.relPath)"
          :expandable="expandable(d.relPath)"
          :selected="content.sidebarSelected === `ws:${d.relPath}`"
          :depth="props.depth"
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
  </div>
</template>

<style scoped>
/* 行列表（items gap 2，同库 FolderTree 口径） */
.ws-rows {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
}
.ws-node {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
}
</style>
