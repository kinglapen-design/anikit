<script setup lang="ts">
// ws 子目录行列表（WorkspaceTree 根层 / 递归子层共用，避免行模板两处重复）：
// 本层 = rel 的子目录行（plain click = 导航 ws 视口 + 单选入素材选择域——库 FolderTree 同口径，
// 2026-09-26 接通；同日 _18 补 Ctrl/Shift 多选：Ctrl = 加选/减选不导航，Shift = 同级范围选，
// 均对齐库 FolderTree 三态；展开钮挂 treeExpansion，显隐按条目 hasSubDir，展开递归 WorkspaceTree）+
// 行包装 div 右键 wsTreeRowSpec（打开 / 固定·取消固定 / 复制·剪切·删除批量分流 / 资源管理器 / 刷新）+
// 拖拽落点（data-folder-id="ws:"+rel，落点视觉 dropZone）。
import { computed, nextTick } from "vue";
import TreeRowItem from "./items/TreeRowItem.vue";
import WorkspaceTree from "./WorkspaceTree.vue";
import { openContextMenu } from "../lib/contextMenus";
import { wsTreeRowSpec } from "../lib/menuSpecs/workspace";
import { isTreeOpen, toggleOpen } from "../lib/treeExpansion";
import { expandEnter, expandLeave } from "../lib/expandAnim";
import { dragView } from "../lib/dragOperation";
import { useContentStore } from "../stores/content";
import { usePickerStore } from "../stores/picker";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";
import { useWsStore } from "../stores/ws";
import type { WsEntry } from "../lib/wsOps";

// rel = 本层目录（"" = 根）；depth = 本层行缩进档（TreeRowItem padding-left 6+depth×20）
const props = withDefaults(defineProps<{ rel?: string; depth?: number }>(), { rel: "", depth: 0 });

const ws = useWsStore();
const content = useContentStore();
const selection = useSelectionStore();
const tabs = useTabsStore();
const childDepth = computed(() => props.depth + 1);

const subDirs = computed(() => ws.subDirs(props.rel));
// 展开钮显隐按条目自带 hasSubDir（2026-09-26 真实化：列目录时后端已扫直接子级，.anikit 不计——
// 原「未加载乐观给箭头，展开后才知没有子级才消」废除）；旧缓存缺字段回退旧判定
const expandable = (d: WsEntry): boolean =>
  d.hasSubDir ?? (!ws.loadedOk(d.relPath) || ws.subDirs(d.relPath).length > 0);

// 行选择键（ws 文件夹与文件同走素材选择域，2026-09-26 对齐库侧选中链）：assetId|folderId 键形同 ws 文件卡
const rowKey = (rel: string): string => `ws:${rel}|ws:${props.rel}`;

function openDir(rel: string): void {
  tabs.openView(`ws:${rel}`);
  // plain click 同步单选入域（库 FolderTree 同口径）：详情面板 / 键盘操作链随之接通；
  // sidebarSelected watch 清选择（pre flush 先行），nextTick 让过再入域；
  // 守卫同 FolderTree（2026-08-13）：同 tick 内已切走（锁定页新开等）不入域
  void nextTick(() => {
    if (content.sidebarSelected === `ws:${rel}`) selection.clickItem(rowKey(rel));
  });
}

// 行点击三态（2026-09-26_18 对齐库 FolderTree）：plain = 导航 + 单选入域（导航与选中并存）；
// Ctrl / Shift = 多选不导航——Ctrl 加选/减选；Shift 同级范围选（范围序 = 本层已加载同级行序）。
// 懒加载边界：order 只含本层已加载行，两锚点须同父级且均已加载才成段——跨父级 / 锚点间未加载
// 的段落按已加载行处理（锚点不在本层序内时退化单选，与素材范围选同规则）
function onRowClick(rel: string, e: MouseEvent): void {
  if (dragView.suppressClick) return;
  // 选择模式（6.2）：树仅导航（库 FolderTree 同口径——修饰键点击不多选不导航）
  if (usePickerStore().active) {
    if (!e.ctrlKey && !e.metaKey && !e.shiftKey) openDir(rel);
    return;
  }
  if (e.ctrlKey || e.metaKey || e.shiftKey) {
    selection.clickItem(
      rowKey(rel),
      { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey },
      subDirs.value.map((d) => rowKey(d.relPath)),
    );
    return;
  }
  openDir(rel);
}

// 行右键菜单（固定·取消固定按清单状态自动切换，spec 内分流）；
// 未选中项先单选（FolderStrip ws 卡片同口径），批量复制 / 剪切 / 删除随选择域分流；
// 选择模式（6.2）不入选择域，直接出菜单
function onRowMenu(e: MouseEvent, rel: string): void {
  if (!usePickerStore().active) {
    const key = rowKey(rel);
    if (!selection.isSelected(key)) selection.clickItem(key);
  }
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
          :expandable="expandable(d)"
          :selected="content.sidebarSelected === `ws:${d.relPath}`"
          :domain-selected="selection.isSelected(rowKey(d.relPath)) && content.sidebarSelected !== `ws:${d.relPath}`"
          :depth="props.depth"
          @toggle="toggleOpen(d.relPath)"
          @click="onRowClick(d.relPath, $event)"
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
