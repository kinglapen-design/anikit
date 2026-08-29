<script setup lang="ts">
// 文件夹树（递归，7.1 十八轮迁移 TreeRowItem，稿 = 布局/主窗口/左侧边栏 + 组件/item/树容器 item）：
// item = TreeRowItem（26 高 / 圆角 6 / hover 半透明-3 / 选中 半透明-2+立体感+提亮一级）；
// 层级与缩进线（沿用 2026-08-08 简化定案，数值按新稿重算）：item 不携带层级类名——内容缩进 =
// TreeRowItem padding-left 6 + depth×20；缩进竖线 = 子层容器级连续线（left = depth×20+15 =
// item 左 padding 6 + 展开钮半宽 9，正对父级展开钮尖角），纵贯首子项顶部至末子项底部；
// 线色 = 亮色/半透明-3，节点设自定义色后 = 该色-1 的 6% 档（2026-08-30 用户定，color-mix 派生）。
// 新建文件夹"先建后命名"（2026-07-29 复验三轮）：入口直接以默认名建库（LibraryView），成功返回的
// 新 id 写入 selection.namingFolderId，本组件对应节点进 naming 输入态——Enter / Esc / 失焦一律保存：
// 名非空且变了 → renameFolder，空或不变保留默认名；命名态节点不参与拖拽 / 右键（不挂 data-folder-id）。
// 树 item 右键：全局菜单宿主（lib/contextMenus → App.vue <ContextMenu>），05 稿 320 宽 + 搜索框。
// 业务态视觉（2026-08-30 用户定案）：多选 = 半透明-2 底 + 1.5px 二级描边；拖拽插入线/并入 =
// 品牌常态描边 + 品牌半透明填充；命名 = 品牌常态描边；剪切/拖拽源 = 半透明（TreeRowItem 内实现）。
import { computed, nextTick } from "vue";
import TreeRowItem from "./items/TreeRowItem.vue";
import { dragView, visibleTreeFolderIds } from "../lib/dragOperation";
import { renameFolder } from "../lib/placementOps";
import { openContextMenu } from "../lib/contextMenus";
import { treeItemSpec } from "../lib/menuSpecs/folder";
import { isTreeOpen, toggleOpen } from "../lib/treeExpansion";
import { toNewTint } from "../lib/customColors";
import { expandEnter, expandLeave } from "../lib/expandAnim";
import { useContentStore, type FolderNode } from "../stores/content";
import { usePickerStore } from "../stores/picker";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";
import { useUiStore } from "../stores/ui";

const props = withDefaults(
  defineProps<{
    nodes: FolderNode[];
    depth?: number; // 本层层级（内容缩进与缩进线定位用；item 底色块满宽不缩进）
    parentId?: string | null; // 本层父级 id（顶层 = null）：树排序"当前层级"与右键菜单判定用
    forceOpenIds?: Set<string>; // 命名节点的祖先链强制展开（新建子文件夹时父级可能折叠）
    inheritTint?: string; // 祖先链最近自定义色（2026-08-18，3.3）：子级视觉跟随（图标双色 tint）
  }>(),
  { depth: 0, parentId: null, forceOpenIds: undefined, inheritTint: undefined },
);
const content = useContentStore();
const selection = useSelectionStore();
const tabs = useTabsStore();
const ui = useUiStore();

// 树排序（6.3 右键菜单"查看-排序"）：custom = folders.json order 原序；
// level* 仅锚定层级（ui.treeSortAnchor = 右键时记录的本层父级）、all* 全部层级按标题
const sortedNodes = computed(() => {
  const mode = ui.treeSort;
  if (mode === "custom") return props.nodes;
  const all = mode.startsWith("all");
  if (!all && (props.parentId ?? null) !== ui.treeSortAnchor) return props.nodes;
  const desc = mode.endsWith("Desc");
  return [...props.nodes].sort((a, b) =>
    desc ? b.name.localeCompare(a.name, "zh") : a.name.localeCompare(b.name, "zh"),
  );
});

// 展开态：模块级共享（lib/treeExpansion，2026-07-29——原每层自管，右键"展开/折叠所有"无从着力）
function isOpen(id: string): boolean {
  return isTreeOpen(id) || (props.forceOpenIds?.has(id) ?? false);
}

// 自定义色（2026-08-18，3.3）：本节点色 ?? 祖先链最近色（子级视觉跟随）；子孙层经 inheritTint 续传。
// 存量色名为旧九色（red/orange/...），展示层经 toNewTint 映射新九色名（TreeRowItem tint /
// 缩进线色）；数据迁移归独立待办（lib/customColors 注）
function nodeTint(node: FolderNode): string | undefined {
  return node.color ?? props.inheritTint;
}

// 缩进线色：默认 亮色/半透明-2（2026-08-30 用户实机：半透明-3 太淡，变色后更看不清）；
// 节点设自定义色后 = 该色-1 的 14% 档（同半透明-2 透明度）
function guideColor(node: FolderNode): string | undefined {
  const t = toNewTint(nodeTint(node));
  return t ? `color-mix(in srgb, var(--n-custom-${t}-1) 14%, transparent)` : undefined;
}

// 拖拽落点（C4c）：引擎按 data-folder-id 匹配（dragOperation），zone 三分（上/下插入线、中间并入）；
// 命名态节点不参与拖拽（无 data-folder-id，引擎命不中）
function dropZoneFor(node: FolderNode): "before" | "after" | "into" | null {
  if (selection.namingFolderId === node.id) return null;
  return dragView.tree?.folderId === node.id ? dragView.tree.zone : null;
}

// 选中：有效拖拽结束后的紧随 click 不触发（C4 引擎置位）；
// 多选（2026-07-28 复验反馈 5）：Ctrl = 加选切换 / Shift = 按可见展开态扁平化顺序锚点范围选
//（文件夹选择域在 selection store，与素材互斥由 clickFolder 内部做）——多选不切换当前文件夹视图
function onSelect(id: string, e: MouseEvent): void {
  if (dragView.suppressClick) return;
  // 选择模式（6.2 一轮，PRD 03_数据链接 04）：左侧树仅导航——plain click 只切换视口，
  // 不入选择域、不多选（选择域本模式无意义）
  if (usePickerStore().active) {
    if (!e.ctrlKey && !e.metaKey && !e.shiftKey) tabs.openView(`folder:${id}`);
    return;
  }
  if (e.ctrlKey || e.metaKey || e.shiftKey) {
    // 范围选顺序按所在区块容器收窄（2026-08-14 多区块：跨区块范围选无意义）
    const scope = (e.target as HTMLElement | null)?.closest(".block-tree") ?? null;
    selection.clickFolder(id, { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey }, visibleTreeFolderIds(scope));
    return;
  }
  tabs.openView(`folder:${id}`);
  // 2026-07-29 复验二轮 2 根因：树 plain click 原先只导航、不入文件夹选择域——
  // sidebarSelected 高亮冒充选中态，用户拖"看着选中实则不在域内"的项时引擎按
  // "拖拽未选中项 = 单选再拖"把已有多选清成单选（树多选排序只动被拖一项）。
  // 修为 plain click 同步单选入域；App.vue 的 sidebarSelected watch 会 selection.clear()
  //（pre flush 先于渲染执行），nextTick 让过它再入域；
  // 守卫（2026-08-13）：连点"树 item → 快速访问"同 tick 内切走时 sidebarSelected 已非本项，
  // 不入域——否则已切走的文件夹 domain 描边残留（双选中观感的次要路径）
  void nextTick(() => {
    if (content.sidebarSelected === `folder:${id}`) selection.clickFolder(id);
  });
}

// 树 item 右键（6.3）：全局菜单宿主（lib/contextMenus）；命名态节点不出菜单（2026-07-29 复验三轮）；
// 右键未选中项先单选（文件夹选择域，与素材互斥由 clickFolder 内部做）；
// 菜单取消（点外 / Esc 未执行动作）时恢复原选择（2026-08-10 用户反馈：取消后品牌色选中态残留）
function onItemMenu(e: MouseEvent, node: FolderNode): void {
  if (selection.namingFolderId === node.id) return;
  const opts = { width: 320, searchable: true }; // 1.3：树节点右键复用 05 文件夹稿（320 宽 + 搜索框）
  // 选择模式（6.2）：不入选择域直接出菜单（spec 内已分流"仅导航"精简集）
  if (usePickerStore().active) {
    openContextMenu(e, treeItemSpec(node, props.parentId ?? null), undefined, opts);
    return;
  }
  if (!selection.isFolderSelected(node.id)) {
    const snapshot = new Set(selection.selectedFolders);
    const prevAnchor = selection.folderAnchor;
    selection.clickFolder(node.id);
    openContextMenu(e, treeItemSpec(node, props.parentId ?? null), (executed) => {
      if (executed) return;
      selection.selectedFolders.clear();
      for (const id of snapshot) selection.selectedFolders.add(id);
      selection.folderAnchor = prevAnchor;
    }, opts);
    return;
  }
  openContextMenu(e, treeItemSpec(node, props.parentId ?? null), undefined, opts);
}

// 命名保存（先建后命名）：非空且与当前名（默认名）不同 → renameFolder；
// 空或不变 = 保留默认名直接退出，不再有"取消新建"
async function onNameCommit(node: FolderNode, name: string): Promise<void> {
  if (selection.namingFolderId !== node.id) return; // Enter 提交后 blur 晚到的双触发护栏
  selection.namingFolderId = null;
  if (!name || name === node.name) return;
  await renameFolder(node.id, name);
}
</script>

<template>
  <template v-for="node in sortedNodes" :key="node.id">
    <div class="ft-node" @contextmenu.prevent.stop="onItemMenu($event, node)">
      <!-- 当前目录（sidebarSelected）= selected 实心高亮（你在这里）；选择域成员 =
           domain-selected 半透明-2 底 + 二级描边（文件夹卡片选中联动 / 树多选）——两态互斥，
           当前目录优先（2026-08-10 用户定：区分双高亮，防"停在哪个目录"混淆） -->
      <TreeRowItem
        :kind="node.kind === 'archive' ? 'archive' : 'folder'"
        :icon="node.kind === 'archive' ? '' : (node.icon ?? '')"
        :name="node.name"
        :count="node.count"
        :direct-count="content.directCounts.get(node.id) ?? 0"
        :expanded="isOpen(node.id)"
        :expandable="node.children.length > 0"
        :selected="content.sidebarSelected === `folder:${node.id}`"
        :domain-selected="selection.isFolderSelected(node.id) && content.sidebarSelected !== `folder:${node.id}`"
        :tint="toNewTint(nodeTint(node))"
        :depth="depth"
        :dimmed="dragView.folderIds?.has(node.id) ?? false"
        :cut="selection.isFolderCut(node.id)"
        :drop-zone="dropZoneFor(node)"
        :naming="selection.namingFolderId === node.id"
        :data-folder-id="selection.namingFolderId === node.id ? undefined : node.id"
        :data-drag-name="node.name"
        @toggle="toggleOpen(node.id)"
        @click="onSelect(node.id, $event)"
        @name-commit="onNameCommit(node, $event)"
      />
      <Transition :css="false" @enter="expandEnter" @leave="expandLeave">
        <div
          v-if="isOpen(node.id) && node.children.length > 0"
          class="ft-children"
          :style="{ '--guide-x': `${depth * 20 + 15}px`, '--guide-color': guideColor(node) }"
        >
          <FolderTree
            :nodes="node.children"
            :depth="depth + 1"
            :parent-id="node.id"
            :force-open-ids="forceOpenIds"
            :inherit-tint="nodeTint(node)"
          />
        </div>
      </Transition>
    </div>
  </template>
</template>

<style scoped>
/* 节点：TreeRowItem 与展开子层间距 2（左侧边栏稿 树形容器 gap 2） */
.ft-node {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
}
/* 子层容器：不缩进（item 满宽，内容缩进由 TreeRowItem padding-left 承担，2026-08-08 用户定） */
.ft-children {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
}
/* 缩进竖线：横坐标 = 父级展开钮尖角（item 左 padding 6+depth×20 + 展开钮半宽 9，即
   --guide-x = depth×20+15）；纵坐标首子项顶部 → 末子项底部（top 0 bottom 0 贯穿）。
   线色 = 亮色/半透明-2（半透明-3 实机太淡，变色后更看不清，2026-08-30 用户改定），
   节点设自定义色后 = 该色-1 的 14% 档（--guide-color 传入）。
   z-index 1：item 罩层满宽后会盖住同层先绘制的线（2026-08-08 走查修复），线在 item 空白
   padding 区（内容从 depth×20+24 起），不压内容 */
.ft-children::before {
  content: "";
  position: absolute;
  left: var(--guide-x);
  top: 0;
  bottom: 0;
  width: 1px;
  background: var(--guide-color, var(--n-l-a2));
  z-index: 1;
  pointer-events: none;
}
</style>
