<script setup lang="ts">
// 工作空间树（PRD 02 + 步骤 3）：常用固定区块 + 懒加载子目录树，区块标题（items/SectionTitle
// block，「常用 / N」「文件夹 / N」）分割——2026-09-17 用户截图定：固定行与目录树挤在一起，
// 要区块分割；交互沿用 FolderStrip / ArchiveStrip 先例（hover 出展开钮，点击折叠区块）。
// 子目录树：展开才挂载子层（v-if），子层挂载即 list_workspace_dir 取子目录（只展示目录，
// 文件进网格视口）；节点点击 = tabs.openView("ws:"+rel)；复用 TreeRowItem 与 treeExpansion
// 会话级展开态（openSet 以 relPath 为 id，与库文件夹 F id 不撞）；展开动画复用 expandAnim
// （FolderTree 同口径）。步骤 3 废除根行「工作空间」（磁盘上无此目录，合成父级与子级同
// 缩进视觉撞兄弟）；根视口经切 workspace 分段自动 openView("ws:") 与历史后退可达。
// 树行右键（步骤 3 最小集）：打开 / 固定到常用·取消固定 / 在资源管理器中显示 / 刷新。
// workspace/ 目录不存在（exists=false）= 空态文案（正文档 + 亮色/四级，SidebarLeft 同风格；
// root=null 是拉取瞬态不渲染误标）；步骤 2 起树行兼拖拽落点（data-folder-id="ws:"+rel，
// 素材移入该目录，落点视觉 dropZone）。
import { computed, onMounted, ref } from "vue";
import TreeRowItem from "./items/TreeRowItem.vue";
import SectionTitle from "./items/SectionTitle.vue";
import { openContextMenu } from "../lib/contextMenus";
import { wsTreeRowSpec } from "../lib/menuSpecs/workspace";
import { isTreeOpen, toggleOpen } from "../lib/treeExpansion";
import { expandEnter, expandLeave } from "../lib/expandAnim";
import { dragView } from "../lib/dragOperation";
import { useContentStore } from "../stores/content";
import { useTabsStore } from "../stores/tabs";
import { useWsStore } from "../stores/ws";

// rel = 本层目录（"" = 根）；depth = 本层内容缩进档（TreeRowItem padding-left 6+depth×20）
const props = withDefaults(defineProps<{ rel?: string; depth?: number }>(), { rel: "", depth: 0 });

const ws = useWsStore();
const content = useContentStore();
const tabs = useTabsStore();
const isRoot = computed(() => props.rel === "");
// 本层子目录行缩进档 / 递归子层缩进档
const rowDepth = computed(() => (isRoot.value ? 0 : props.depth));
const childDepth = computed(() => rowDepth.value + 1);

// 区块折叠（FolderStrip / ArchiveStrip 同先例：区块标题点击折叠，组件级 ref 默认开）；
// 空态不随折叠隐藏（workspace 目录缺失是重要状态）
const pinsOpen = ref(true);
const treeOpen = ref(true);

// 懒加载：层级挂载即拉本目录 listing（根层随分段切换挂载；子层随父级展开挂载）；
// 根层另取固定清单（ensureRoot 已拉一次，此处兜底会话内 pin 变化）+ 拉各 pin 目录计数
onMounted(() => {
  void ws.loadDir(props.rel);
  if (isRoot.value) {
    void ws.loadPinned();
    for (const rel of ws.pinned) void ws.loadDir(rel);
  }
});

const subDirs = computed(() => ws.subDirs(props.rel));
// 展开钮形态：未加载先乐观给箭头（点开即知有没有子目录）；已加载按实际子目录数（无则占位隐藏）
const expandable = (rel: string): boolean =>
  !ws.loadedOk(rel) || ws.subDirs(rel).length > 0;

// pin 行显示名 = rel 最后一段
const pinName = (rel: string): string => rel.slice(rel.lastIndexOf("/") + 1);

function openDir(rel: string): void {
  tabs.openView(`ws:${rel}`);
}

// 树行 / pin 行右键菜单（固定·取消固定按清单状态自动切换，spec 内分流）
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
  <div class="ws-tree" :class="{ root: isRoot }">
    <!-- 常用固定区块（步骤 3：区块标题「常用 / N」+ item 列表；行 = 目录快捷入口，
         右键可取消固定；数量 = 该目录直接文件数，未加载不显示） -->
    <template v-if="isRoot && ws.pinned.length">
      <SectionTitle
        class="ws-head"
        variant="block"
        title="常用"
        :count="ws.pinned.length"
        :expanded="pinsOpen"
        :actions="false"
        @toggle="pinsOpen = !pinsOpen"
      />
      <template v-if="pinsOpen">
        <div v-for="rel in ws.pinned" :key="`pin:${rel}`" class="ws-node" @contextmenu.prevent.stop="onRowMenu($event, rel)">
          <TreeRowItem
            kind="folder"
            :name="pinName(rel)"
            :count="ws.loadedOk(rel) ? ws.fileCount(rel) : undefined"
            :expandable="false"
            :selected="content.sidebarSelected === `ws:${rel}`"
            :depth="0"
            @click="openDir(rel)"
          />
        </div>
      </template>
    </template>
    <!-- 文件夹区块（根层区块标题「文件夹 / N」；子层递归无标题） -->
    <SectionTitle
      v-if="isRoot"
      class="ws-head"
      variant="block"
      title="文件夹"
      :count="subDirs.length"
      :expanded="treeOpen"
      :actions="false"
      @toggle="treeOpen = !treeOpen"
    />
    <!-- 子目录行（本层 = props.rel 的子目录；点击导航，展开钮挂 treeExpansion） -->
    <template v-if="!isRoot || treeOpen" v-for="d in subDirs" :key="d.relPath">
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
        （ensureRoot null 不缓存、下次调用即重试），不渲染「未开启」误标（2026-09-17 实机报）；
        不随区块折叠隐藏 -->
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
/* 区块标题与上一区块的间隔（顶 8）+ 与本职首行的间隔（底 6 + 容器 gap 2 = 8 视觉，
   参照截图 2 标题—首行留白大于行间距）；首个标题不吃双份顶距——容器已有 padding-top 8 */
.ws-head {
  padding-top: var(--n-space-8);
  padding-bottom: var(--n-space-6);
}
.ws-tree.root > .ws-head:first-child {
  padding-top: 0;
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
